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

/// Same-directory replace, so a interrupted write never leaves a truncated todo file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
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
    temp.persist(path).map_err(|error| error.error)?;
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
        self.ensure_unchanged()?;
        let bytes = self.encoded(&lines);
        atomic_write(&self.path, &bytes)?;
        self.lines = lines;
        self.original = bytes;
        Ok(())
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
        atomic_write(&target, &archived)?;
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
