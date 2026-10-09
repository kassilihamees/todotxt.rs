use crate::task::Task;
use chrono::NaiveDate;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

fn preserve_failed_recovery(directory: &Path) -> PathBuf {
    let snapshot = (|| -> io::Result<PathBuf> {
        let folder = tempfile::Builder::new()
            .prefix("failed-")
            .tempdir_in(directory)?;
        fs::copy(
            directory.join("previous.txt"),
            folder.path().join("previous.txt"),
        )?;
        fs::copy(
            directory.join("intended.txt"),
            folder.path().join("intended.txt"),
        )?;
        Ok(folder.keep())
    })();
    snapshot.unwrap_or_else(|_| directory.to_owned())
}

// Virtual mounts can read files without supporting final-path/volume queries.
// Keep symlink resolution where available, but never require it for ordinary IO.
fn resolved_path(
    path: &Path,
    canonicalize: impl FnOnce(&Path) -> io::Result<PathBuf>,
) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    Ok(canonicalize(&absolute).unwrap_or(absolute))
}

fn same_path(a: &Path, b: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};
        fn key(path: &Path) -> Vec<u16> {
            let mut chars: Vec<_> = path
                .as_os_str()
                .encode_wide()
                .map(|c| if c == b'/' as u16 { b'\\' as u16 } else { c })
                .collect();
            if chars.starts_with(&[92, 92, 63, 92]) {
                chars.drain(..4);
                if chars.len() >= 4
                    && chars[..4]
                        .iter()
                        .copied()
                        .zip(*b"UNC\\")
                        .all(|(a, b)| a == b as u16 || a == b.to_ascii_lowercase() as u16)
                {
                    chars.splice(..4, [92, 92]);
                }
            }
            chars
        }
        let a = key(a);
        let b = key(b);
        unsafe {
            CompareStringOrdinal(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32, 1)
                == CSTR_EQUAL
        }
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

/// Stage, close, verify, replace, then verify again. Mounted filesystems may
/// mishandle renaming a file while its write handle is still open.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    atomic_write_checked(
        path,
        bytes,
        || Ok(()),
        |staged, destination| staged.persist(destination).map_err(|error| error.error),
    )
}

fn atomic_write_checked(
    path: &Path,
    bytes: &[u8],
    before_replace: impl FnOnce() -> io::Result<()>,
    replace: impl FnOnce(tempfile::TempPath, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    if let Ok(meta) = fs::metadata(path) {
        temp.as_file().set_permissions(meta.permissions())?;
    }
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    let (file, staged) = temp.into_parts();
    drop(file);
    if fs::read(&staged)? != bytes {
        return Err(invalid(
            "Save refused: the staged file did not retain its contents.",
        ));
    }
    before_replace()?;
    replace(staged, path)?;
    if fs::read(path)? != bytes {
        return Err(invalid(
            "Save verification failed: the filesystem did not retain the written contents.",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Document {
    pub path: PathBuf,
    pub lines: Vec<String>,
    original: Vec<u8>,
    newline: String,
    final_newline: bool,
    bom: bool,
    recovery_root: PathBuf,
    failed_save: bool,
}

impl Document {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::open_with(path.as_ref(), |path| fs::canonicalize(path))
    }

    fn open_with(
        path: &Path,
        canonicalize: impl FnOnce(&Path) -> io::Result<PathBuf>,
    ) -> io::Result<Self> {
        let path = resolved_path(path, canonicalize)?;
        let original = fs::read(&path)?;
        let bom = original.starts_with(&[0xef, 0xbb, 0xbf]);
        let text = std::str::from_utf8(if bom { &original[3..] } else { &original })
            .map_err(|_| invalid("The todo file must be UTF-8. It has not been modified."))?;
        let newline = if text.contains("\r\n") {
            "\r\n"
        } else if text.contains('\n') {
            "\n"
        } else if cfg!(windows) {
            "\r\n"
        } else {
            "\n"
        }
        .to_owned();
        let final_newline = text.ends_with('\n');
        let lines = text.lines().map(str::to_owned).collect();
        Ok(Self {
            path,
            lines,
            original,
            newline,
            final_newline,
            bom,
            recovery_root: directories::ProjectDirs::from("", "", "todotxt.rs")
                .map(|dirs| dirs.data_local_dir().join("recovery"))
                .unwrap_or_else(|| std::env::temp_dir().join("todotxt-rs-recovery")),
            failed_save: false,
        })
    }

    pub fn tasks(&self, today: NaiveDate, preserve_blank: bool) -> Vec<(usize, Task)> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, s)| preserve_blank || !s.is_empty())
            .map(|(id, raw)| (id, Task::parse(raw, today)))
            .collect()
    }

    pub fn changed(&self) -> io::Result<bool> {
        Ok(fs::read(&self.path)? != self.original)
    }

    pub fn can_auto_reload(&self) -> bool {
        !self.failed_save
    }

    pub fn byte_len(&self) -> usize {
        self.original.len()
    }

    /// Never discard a nonempty document merely because a later filesystem
    /// read returns empty. Explicit File > Open can open an intentionally empty file.
    pub fn reload(&mut self) -> io::Result<()> {
        let mut candidate = Self::open(&self.path)?;
        candidate.recovery_root = self.recovery_root.clone();
        if self.lines.iter().any(|line| !line.trim().is_empty())
            && candidate.lines.iter().all(|line| line.trim().is_empty())
        {
            self.failed_save = true;
            fs::create_dir_all(&self.recovery_root)?;
            let recovery = tempfile::Builder::new()
                .prefix("empty-read-")
                .tempdir_in(&self.recovery_root)?;
            atomic_write(&recovery.path().join("last-verified.txt"), &self.original)?;
            let recovery = recovery.keep();
            return Err(invalid(format!(
                "Reload refused: the filesystem returned an empty task file after a nonempty file was loaded. Tasks remain in memory and automatic refresh is paused. The last verified contents are in {}. Check or restore the disk file before reloading. To deliberately open an empty file, use File > Open.",
                recovery.join("last-verified.txt").display()
            )));
        }
        *self = candidate;
        Ok(())
    }

    fn ensure_unchanged(&self) -> io::Result<()> {
        if self.changed()? {
            return Err(invalid(
                "The todo file changed outside the application. Reload it before editing; your editor text has been kept.",
            ));
        }
        Ok(())
    }

    fn encoded(&self, lines: &[String]) -> Vec<u8> {
        let mut text = lines.join(&self.newline);
        if self.final_newline && !lines.is_empty() {
            text.push_str(&self.newline);
        }
        let mut bytes = if self.bom {
            vec![0xef, 0xbb, 0xbf]
        } else {
            Vec::new()
        };
        bytes.extend(text.as_bytes());
        bytes
    }

    fn commit(&mut self, lines: Vec<String>) -> io::Result<()> {
        self.commit_with(lines, |staged, destination| {
            staged.persist(destination).map_err(|error| error.error)
        })
    }

    fn commit_with(
        &mut self,
        lines: Vec<String>,
        replace: impl FnOnce(tempfile::TempPath, &Path) -> io::Result<()>,
    ) -> io::Result<()> {
        self.ensure_unchanged()?;
        let bytes = self.encoded(&lines);
        let recovery = self.recovery_copy(&self.path, &self.original, &bytes)?;
        if let Err(error) =
            atomic_write_checked(&self.path, &bytes, || self.ensure_unchanged(), replace)
        {
            self.failed_save = true;
            let recovery = preserve_failed_recovery(&recovery);
            return Err(invalid(format!(
                "{error}\nLocal recovery copies are in {}. previous.txt contains the file before this save; intended.txt contains the attempted save. Automatic refresh is paused. Check or restore the disk file before reloading.",
                recovery.display()
            )));
        }
        self.lines = lines;
        self.original = bytes;
        self.failed_save = false;
        Ok(())
    }

    fn recovery_copy(&self, path: &Path, previous: &[u8], intended: &[u8]) -> io::Result<PathBuf> {
        use std::hash::{Hash, Hasher};
        let mut key = std::collections::hash_map::DefaultHasher::new();
        path.hash(&mut key);
        let directory = self.recovery_root.join(format!("{:016x}", key.finish()));
        fs::create_dir_all(&directory)?;
        atomic_write(&directory.join("previous.txt"), previous)?;
        atomic_write(&directory.join("intended.txt"), intended)?;
        Ok(directory)
    }

    pub fn add(&mut self, raw: &str) -> io::Result<usize> {
        if raw.contains(['\r', '\n']) {
            return Err(invalid("A task must fit on one line."));
        }
        let id = self.lines.len();
        let mut lines = self.lines.clone();
        lines.push(raw.to_owned());
        self.commit(lines)?;
        Ok(id)
    }

    /// IDs are physical line offsets, which makes duplicate tasks independently editable.
    pub fn replace(&mut self, changes: &BTreeMap<usize, Option<String>>) -> io::Result<()> {
        if changes.keys().any(|id| *id >= self.lines.len()) {
            return Err(invalid("The selected task no longer exists."));
        }
        if changes.values().flatten().any(|s| s.contains(['\r', '\n'])) {
            return Err(invalid("A task must fit on one line."));
        }
        let lines = self
            .lines
            .iter()
            .enumerate()
            .filter_map(|(id, raw)| match changes.get(&id) {
                Some(value) => value.clone(),
                None => Some(raw.clone()),
            })
            .collect();
        self.commit(lines)
    }

    pub fn archive(&mut self, target: &Path, today: NaiveDate) -> io::Result<usize> {
        self.archive_with(target, today, |path| fs::canonicalize(path))
    }

    fn archive_with(
        &mut self,
        target: &Path,
        today: NaiveDate,
        canonicalize: impl FnOnce(&Path) -> io::Result<PathBuf>,
    ) -> io::Result<usize> {
        self.ensure_unchanged()?;
        let target = resolved_path(target, canonicalize)?;
        if same_path(&target, &self.path) {
            return Err(invalid(
                "The archive file must be different from the todo file.",
            ));
        }
        let completed: BTreeMap<_, _> = self
            .tasks(today, false)
            .into_iter()
            .filter(|(_, t)| t.completed)
            .map(|(id, _)| (id, None))
            .collect();
        if completed.is_empty() {
            return Ok(0);
        }
        let mut archived = match fs::read(&target) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let previous_archive = archived.clone();
        let archive_newline =
            if archived.contains(&b'\n') && !archived.windows(2).any(|w| w == b"\r\n") {
                "\n"
            } else {
                &self.newline
            };
        if !archived.is_empty() && !archived.ends_with(b"\n") {
            archived.extend(archive_newline.as_bytes());
        }
        for id in completed.keys() {
            archived.extend(self.lines[*id].as_bytes());
            archived.extend(archive_newline.as_bytes());
        }
        // Archive first: failure removing from source can duplicate tasks, but cannot lose them.
        let recovery = self.recovery_copy(&target, &previous_archive, &archived)?;
        atomic_write_checked(
            &target,
            &archived,
            || {
                self.ensure_unchanged()?;
                let current = match fs::read(&target) {
                    Ok(bytes) => bytes,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
                    Err(error) => return Err(error),
                };
                if current != previous_archive {
                    return Err(invalid(
                        "The archive file changed outside the application. Archiving was refused.",
                    ));
                }
                Ok(())
            },
            |staged, destination| staged.persist(destination).map_err(|error| error.error),
        )
        .map_err(|error| {
            let recovery = preserve_failed_recovery(&recovery);
            invalid(format!(
                "{error}\nArchive recovery copies are in {}. The todo file has not been changed.",
                recovery.display()
            ))
        })?;
        self.replace(&completed).map_err(|e| {
            invalid(format!(
                "Tasks were copied to the archive but could not be removed from todo.txt: {e}"
            ))
        })?;
        Ok(completed.len())
    }
}

#[cfg(test)]
mod mount_tests {
    use super::*;

    fn unsupported_volume(_: &Path) -> io::Result<PathBuf> {
        Err(io::Error::from_raw_os_error(1005))
    }

    #[test]
    fn delayed_empty_read_after_verified_completion_keeps_memory_and_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        fs::write(&path, "First fictional task\r\nSecond fictional task\r\n").unwrap();
        let mut doc = Document::open(&path).unwrap();
        doc.recovery_root = dir.path().join("local-recovery");
        doc.replace(&BTreeMap::from([(
            0,
            Some("x 2026-10-09 First fictional task".into()),
        )]))
        .unwrap();
        let verified = fs::read(&path).unwrap();
        // The save/read-back passed, then the filesystem exposed an empty file.
        fs::write(&path, []).unwrap();
        assert!(doc.changed().unwrap());
        let error = doc.reload().unwrap_err();
        assert!(error.to_string().contains("Reload refused"));
        assert_eq!(doc.original, verified);
        assert_eq!(
            doc.lines,
            ["x 2026-10-09 First fictional task", "Second fictional task"]
        );
        assert!(!doc.can_auto_reload());
        assert!(fs::read(&path).unwrap().is_empty());
        let snapshot = fs::read_dir(&doc.recovery_root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("empty-read-")
            })
            .unwrap();
        assert_eq!(
            fs::read(snapshot.join("last-verified.txt")).unwrap(),
            verified
        );
        fs::write(&path, &verified).unwrap();
        doc.reload().unwrap();
        assert!(doc.can_auto_reload());
    }

    #[test]
    fn reload_accepts_nonempty_changes_and_already_empty_documents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        fs::write(&path, "Original fictional task\n").unwrap();
        let mut doc = Document::open(&path).unwrap();
        fs::write(&path, "External fictional change\n").unwrap();
        doc.reload().unwrap();
        assert_eq!(doc.lines, ["External fictional change"]);
        fs::write(&path, []).unwrap();
        let mut explicitly_opened = Document::open(&path).unwrap();
        explicitly_opened.reload().unwrap();
        assert!(explicitly_opened.lines.is_empty());
    }

    #[test]
    fn reported_success_with_truncated_destination_is_an_error_with_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        let original = b"\xef\xbb\xbfFirst fictional task\r\nSecond fictional task\r\n";
        fs::write(&path, original).unwrap();
        let mut doc = Document::open(&path).unwrap();
        doc.recovery_root = dir.path().join("local-recovery");
        let lines = vec![
            "x 2026-10-09 First fictional task".into(),
            "Second fictional task".into(),
        ];
        let intended = doc.encoded(&lines);
        let error = doc
            .commit_with(lines, |_staged, destination| fs::write(destination, []))
            .unwrap_err();
        assert!(error.to_string().contains("verification failed"));
        assert!(fs::read(&path).unwrap().is_empty());
        assert_eq!(doc.original, original);
        assert_eq!(doc.lines, ["First fictional task", "Second fictional task"]);
        assert!(!doc.can_auto_reload());
        let root = fs::read_dir(&doc.recovery_root)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let failed = fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.is_dir())
            .unwrap();
        assert_eq!(fs::read(failed.join("previous.txt")).unwrap(), original);
        assert_eq!(fs::read(failed.join("intended.txt")).unwrap(), intended);
        // A later explicit reload/save must not overwrite the failed-save evidence.
        let mut reopened = Document::open(&path).unwrap();
        reopened.recovery_root = doc.recovery_root;
        reopened.add("Another fictional task").unwrap();
        assert_eq!(fs::read(failed.join("previous.txt")).unwrap(), original);
        assert_eq!(fs::read(failed.join("intended.txt")).unwrap(), intended);
    }

    #[test]
    fn successful_completion_keeps_a_local_previous_and_intended_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        let original = b"First fictional task\r\nSecond fictional task\r\n";
        fs::write(&path, original).unwrap();
        let mut doc = Document::open(&path).unwrap();
        doc.recovery_root = dir.path().join("local-recovery");
        let done = Task::parse(
            "First fictional task",
            NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
        )
        .toggle(NaiveDate::from_ymd_opt(2026, 10, 9).unwrap());
        doc.replace(&BTreeMap::from([(0, Some(done))])).unwrap();
        let saved = fs::read(&path).unwrap();
        assert_eq!(
            saved,
            b"x 2026-10-09 First fictional task\r\nSecond fictional task\r\n"
        );
        assert!(doc.can_auto_reload());
        let folder = fs::read_dir(&doc.recovery_root)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(fs::read(folder.join("previous.txt")).unwrap(), original);
        assert_eq!(fs::read(folder.join("intended.txt")).unwrap(), saved);
    }

    #[test]
    fn staging_rechecks_external_changes_before_replacement() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        fs::write(&path, "Original fictional task").unwrap();
        let error = atomic_write_checked(
            &path,
            b"Intended fictional edit",
            || {
                fs::write(&path, "External fictional edit")?;
                Err(invalid("changed outside"))
            },
            |_, _| panic!("Must not replace after the precondition failed"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("changed outside"));
        assert_eq!(fs::read(&path).unwrap(), b"External fictional edit");
    }

    #[cfg(windows)]
    #[test]
    fn staged_write_handle_is_closed_before_replacement() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        atomic_write_checked(
            &path,
            b"Fictional content",
            || Ok(()),
            |staged, destination| {
                let exclusive = fs::OpenOptions::new()
                    .read(true)
                    .share_mode(0)
                    .open(&staged)?;
                drop(exclusive);
                staged.persist(destination).map_err(|error| error.error)
            },
        )
        .unwrap();
        assert_eq!(fs::read(path).unwrap(), b"Fictional content");
    }

    #[test]
    fn unsupported_final_path_queries_allow_loading_and_preserve_write_protection() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        let original = b"\xef\xbb\xbfFirst fictional task\r\n\r\nSecond task\r\n";
        fs::write(&path, original).unwrap();
        let mut doc = Document::open_with(&path, unsupported_volume).unwrap();
        assert_eq!(doc.path, std::path::absolute(&path).unwrap());
        assert_eq!(doc.lines, ["First fictional task", "", "Second task"]);
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(!doc.changed().unwrap());
        doc.add("Third fictional task").unwrap();
        assert_eq!(
            fs::read(&path).unwrap(),
            b"\xef\xbb\xbfFirst fictional task\r\n\r\nSecond task\r\nThird fictional task\r\n"
        );
        fs::write(&path, "External fictional change\r\n").unwrap();
        assert!(doc.add("Refused draft").is_err());
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "External fictional change\r\n"
        );
        assert_eq!(doc.lines.last().unwrap(), "Third fictional task");
    }

    #[test]
    fn archive_handles_unsupported_final_paths_without_allowing_self_archive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        let archive = dir.path().join("done.txt");
        let today = NaiveDate::from_ymd_opt(2026, 10, 9).unwrap();
        let original = "Active fictional task\nx 2026-10-09 Completed fictional task\n";
        fs::write(&path, original).unwrap();
        fs::write(&archive, "Previously archived fictional task\n").unwrap();
        let mut doc = Document::open_with(&path, unsupported_volume).unwrap();
        assert!(doc.archive_with(&path, today, unsupported_volume).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(
            doc.archive_with(&archive, today, unsupported_volume)
                .unwrap(),
            1
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "Active fictional task\n"
        );
        assert_eq!(
            fs::read_to_string(&archive).unwrap(),
            "Previously archived fictional task\nx 2026-10-09 Completed fictional task\n"
        );
    }

    #[test]
    fn real_read_errors_are_not_hidden_by_path_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.todo");
        let error = Document::open_with(&path, unsupported_volume).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(!path.exists());
    }

    #[cfg(windows)]
    #[test]
    fn self_archive_is_rejected_across_case_and_verbatim_path_forms() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("todo.txt");
        let original = "x 2026-10-09 Completed fictional task\n";
        fs::write(&path, original).unwrap();
        let mut doc = Document::open(&path).unwrap();
        let target = PathBuf::from(
            std::path::absolute(&path)
                .unwrap()
                .to_string_lossy()
                .to_uppercase(),
        );
        assert!(
            doc.archive_with(
                &target,
                NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
                unsupported_volume
            )
            .is_err()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(doc.lines, [original.trim_end()]);
    }
}
