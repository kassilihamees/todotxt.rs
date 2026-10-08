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
        let path = fs::canonicalize(path)?;
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
        self.ensure_unchanged()?;
        let resolved = if target.exists() {
            fs::canonicalize(target)?
        } else {
            target.to_path_buf()
        };
        if resolved == self.path {
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
        let mut archived = match fs::read(target) {
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
        atomic_write(target, &archived)?;
        self.replace(&completed).map_err(|e| {
            invalid(format!(
                "Tasks were copied to the archive but could not be removed from todo.txt: {e}"
            ))
        })?;
        Ok(completed.len())
    }
}
