//! Platform-neutral state used by the Windows frontend.
use crate::{
    document::Document,
    settings::Settings,
    task::Task,
    view::{self, Filter, Row},
};
use chrono::{Local, NaiveDate};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

pub struct Model {
    pub settings: Settings,
    pub config_dir: PathBuf,
    pub document: Option<Document>,
    pub tasks: Vec<(usize, Task)>,
    pub rows: Vec<Row>,
    pub selected: BTreeSet<usize>,
    pub draft: String,
    pub editing: Option<usize>,
    pub date: NaiveDate,
    pub startup_error: Option<String>,
}
impl Model {
    pub fn new(path: Option<PathBuf>, config: Option<PathBuf>, demo: bool) -> Self {
        let config_dir = Settings::directory(config);
        let (mut settings, mut error) = match Settings::read(&config_dir) {
            Ok(s) => (s, None),
            Err(e) => (
                Settings::default(),
                Some(format!("Could not read settings: {e}")),
            ),
        };
        let first = path.is_none() && settings.file.is_none();
        let chosen = if demo || first {
            let path = config_dir.join("demo/todo.txt");
            let result = fs::create_dir_all(path.parent().unwrap()).and_then(|()| {
                if path.exists() {
                    Ok(())
                } else {
                    fs::write(&path, include_bytes!("../fixtures/demo.todo"))
                }
            });
            if let Err(e) = result {
                error = Some(e.to_string());
            }
            if first {
                settings.sort = view::Sort::Priority;
                settings.word_wrap = true;
            }
            Some(path)
        } else {
            path.or_else(|| settings.file.clone())
        };
        let mut model = Self {
            settings,
            config_dir,
            document: None,
            tasks: Vec::new(),
            rows: Vec::new(),
            selected: BTreeSet::new(),
            draft: String::new(),
            editing: None,
            date: Local::now().date_naive(),
            startup_error: error,
        };
        if let Some(path) = chosen
            && let Err(e) = model.load(&path)
        {
            model.startup_error = Some(e.to_string());
        }
        model
    }
    pub fn load(&mut self, path: &Path) -> io::Result<()> {
        let document = Document::open(path)?;
        self.settings.file = Some(document.path.clone());
        self.document = Some(document);
        self.draft.clear();
        self.editing = None;
        self.selected.clear();
        self.refresh();
        self.save()
    }
    pub fn save(&self) -> io::Result<()> {
        self.settings.save(&self.config_dir)
    }
    pub fn refresh(&mut self) {
        self.date = Local::now().date_naive();
        self.tasks = self
            .document
            .as_ref()
            .map(|d| d.tasks(self.date, self.settings.preserve_blank))
            .unwrap_or_default();
        self.rows = view::rows(
            &self.tasks,
            self.settings.sort,
            &Filter {
                text: &self.settings.filter,
                case_sensitive: self.settings.case_sensitive,
                hide_future: self.settings.hide_future,
                show_hidden: self.settings.show_hidden,
            },
            self.settings.grouping,
            self.date,
        );
        let visible: BTreeSet<_> = self
            .rows
            .iter()
            .filter_map(|r| {
                if let Row::Task(id) = r {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        self.selected.retain(|id| visible.contains(id));
        if self.selected.is_empty()
            && let Some(id) = self.rows.iter().find_map(|r| {
                if let Row::Task(id) = r {
                    Some(*id)
                } else {
                    None
                }
            })
        {
            self.selected.insert(id);
        }
    }
    pub fn selected_tasks(&self) -> Vec<(usize, Task)> {
        self.tasks
            .iter()
            .filter(|(id, _)| self.selected.contains(id))
            .cloned()
            .collect()
    }
    pub fn modify(&mut self, op: impl Fn(&Task) -> Option<String>) -> io::Result<()> {
        if self.selected.is_empty() {
            return Ok(());
        }
        let changes = self
            .selected_tasks()
            .into_iter()
            .map(|(id, t)| (id, op(&t)))
            .collect();
        if let Some(doc) = &mut self.document {
            doc.replace(&changes)?;
        }
        self.refresh();
        Ok(())
    }
    pub fn submit(&mut self) -> io::Result<Option<usize>> {
        let text = if self.settings.preserve_blank {
            self.draft.as_str()
        } else {
            self.draft.trim()
        };
        if text.is_empty() {
            return Ok(None);
        }
        let task = Task::parse(text, self.date);
        let raw = if self.editing.is_none() && self.settings.add_creation {
            task.with_creation_date(self.date)
        } else {
            task.raw
        };
        let doc = self
            .document
            .as_mut()
            .ok_or_else(|| io::Error::other("Open or create a todo.txt file first."))?;
        let id = if let Some(id) = self.editing {
            doc.replace(&BTreeMap::from([(id, Some(raw))]))?;
            id
        } else {
            doc.add(&raw)?
        };
        self.editing = None;
        self.draft.clear();
        self.selected = BTreeSet::from([id]);
        self.refresh();
        Ok(Some(id))
    }
    pub fn reload(&mut self) -> io::Result<()> {
        if let Some(doc) = &self.document {
            self.document = Some(Document::open(&doc.path)?);
        }
        self.editing = None;
        self.refresh();
        Ok(())
    }
    pub fn begin_new(&mut self) {
        self.editing = None;
        self.draft = self
            .settings
            .filter
            .lines()
            .filter(|s| !s.starts_with('-') && *s != "DONE")
            .map(|s| s.replace("due:active", "due:today"))
            .collect::<Vec<_>>()
            .join(" ");
    }
    pub fn begin_edit(&mut self, duplicate: bool) -> bool {
        if let [(id, task)] = self.selected_tasks().as_slice() {
            self.draft = task.raw.clone();
            self.editing = if duplicate { None } else { Some(*id) };
            true
        } else {
            false
        }
    }
    pub fn archive(&mut self, path: &Path) -> io::Result<()> {
        if let Some(doc) = &mut self.document {
            doc.archive(path, self.date)?;
        }
        self.settings.archive = Some(path.to_owned());
        self.editing = None;
        self.selected.clear();
        self.refresh();
        self.save()
    }
    pub fn preset(&mut self, n: usize) -> io::Result<()> {
        self.settings.filter = if n == 0 {
            String::new()
        } else {
            self.settings.presets[n - 1].clone()
        };
        self.settings.active_preset = n;
        self.refresh();
        self.save()
    }
    pub fn print_preview(&self) -> io::Result<PathBuf> {
        let escape = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        };
        let mut html = String::from(
            "<!doctype html><meta charset=utf-8><title>todotxt.rs — Print Preview</title><style>body{font:14px sans-serif}h3{margin:18px 0 4px}p{margin:0;padding:3px}p:nth-child(even){background:#f8f8f8}.done{text-decoration:line-through;color:#999}@media print{button{display:none}}</style><button onclick='window.print()'>Print</button>",
        );
        for row in &self.rows {
            match row {
                Row::Header(s) => html.push_str(&format!("<h3>{}</h3>", escape(s))),
                Row::Task(id) => {
                    if let Some((_, t)) = self.tasks.iter().find(|(n, _)| n == id) {
                        html.push_str(&format!(
                            "<p class='{}'>{}</p>",
                            if t.completed { "done" } else { "task" },
                            escape(&t.raw)
                        ));
                    }
                }
            }
        }
        fs::create_dir_all(&self.config_dir)?;
        let path = self.config_dir.join("print-preview.html");
        fs::write(&path, html)?;
        Ok(path)
    }
}
