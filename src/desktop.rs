use chrono::{Duration, Local, NaiveDate};
use eframe::egui::{self, Color32, FontId, Key, RichText, Stroke, Vec2};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::PathBuf,
    time::{Duration as StdDuration, Instant},
};
use todotxt_rs::settings::Settings;
use todotxt_rs::{
    document::{Document, atomic_write},
    task::{Task, parse_date, resolve_date},
    view::{self, Filter, Row, Sort},
};

#[derive(Clone)]
enum Dialog {
    Delete,
    Append(String),
    Priority(String),
    Date(&'static str, String),
    Postpone(&'static str, String),
    Filters(Box<Settings>),
    Options(Box<Settings>),
    Help,
}
#[derive(Clone, Copy, Debug)]
enum Action {
    Open,
    NewFile,
    NewTask,
    Edit,
    Duplicate,
    Delete,
    Append,
    Toggle,
    Archive,
    Reload,
    Options,
    Filters,
    Help,
    Calendar,
    Print,
    Log,
    Copy,
    Paste,
    Cut,
    Priority,
    ShiftPriority(i8),
    Date(&'static str),
    ShiftDate(&'static str, i64),
    RemoveDate(&'static str),
    Postpone(&'static str),
    Sort(Sort),
    Preset(usize),
    HideFuture,
    ShowHidden,
    Exit,
}
pub struct Desktop {
    settings: Settings,
    config_dir: PathBuf,
    document: Option<Document>,
    tasks: Vec<(usize, Task)>,
    rows: Vec<Row>,
    selection: BTreeSet<usize>,
    anchor: Option<usize>,
    editor: String,
    editing: Option<usize>,
    focus_editor: bool,
    list_focus: bool,
    scroll_to: Option<usize>,
    dialog: Option<Dialog>,
    dialog_focus: bool,
    error: Option<String>,
    last_poll: Instant,
    date: NaiveDate,
    calendar: bool,
    completion_index: usize,
    completion_dismissed: bool,
}
fn today() -> NaiveDate {
    Local::now().date_naive()
}
impl Desktop {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        path: Option<PathBuf>,
        config: Option<PathBuf>,
        demo: bool,
    ) -> Self {
        configure_style(&cc.egui_ctx);
        let config_dir = config.unwrap_or_else(|| {
            directories::ProjectDirs::from("", "", "todotxt.rs")
                .map(|p| p.config_dir().to_owned())
                .unwrap_or_else(|| PathBuf::from(".local"))
        });
        let loaded = fs::read(config_dir.join("settings.json"));
        let (mut settings, startup_error) = match loaded {
            Ok(bytes) => match serde_json::from_slice::<Settings>(&bytes) {
                Ok(s) => (s, None),
                Err(e) => (
                    Settings::default(),
                    Some(format!("Could not read saved settings: {e}")),
                ),
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => (Settings::default(), None),
            Err(e) => (
                Settings::default(),
                Some(format!("Could not read saved settings: {e}")),
            ),
        };
        let first_run = settings.file.is_none() && path.is_none();
        let chosen = if demo || first_run {
            let dest = config_dir.join("demo/todo.txt");
            let result = fs::create_dir_all(dest.parent().unwrap()).and_then(|()| {
                if dest.exists() {
                    Ok(())
                } else {
                    fs::write(&dest, include_bytes!("../fixtures/demo.todo"))
                }
            });
            if let Err(e) = result {
                eprintln!("Could not create demo file: {e}");
            }
            if first_run {
                settings.sort = Sort::Priority;
                settings.word_wrap = true;
            }
            Some(dest)
        } else {
            path.or_else(|| settings.file.clone())
        };
        let mut app = Self {
            settings,
            config_dir,
            document: None,
            tasks: Vec::new(),
            rows: Vec::new(),
            selection: BTreeSet::new(),
            anchor: None,
            editor: String::new(),
            editing: None,
            focus_editor: false,
            list_focus: true,
            scroll_to: None,
            dialog: None,
            dialog_focus: false,
            error: startup_error,
            last_poll: Instant::now(),
            date: today(),
            calendar: false,
            completion_index: 0,
            completion_dismissed: false,
        };
        app.settings.debug_event(
            &app.config_dir,
            concat!(
                "startup version ",
                env!("CARGO_PKG_VERSION"),
                " frontend portable"
            ),
        );
        if let Some(path) = chosen {
            app.load(path);
        }
        app
    }
    fn fail(&mut self, error: impl std::fmt::Display) {
        let message = error.to_string();
        if fs::create_dir_all(&self.config_dir).is_ok() {
            use std::io::Write;
            if let Ok(mut file) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.config_dir.join("error.log"))
            {
                let _ = writeln!(file, "{} {message}", Local::now());
            }
        }
        self.error = Some(message);
    }
    fn save_settings(&mut self) {
        let result = fs::create_dir_all(&self.config_dir).and_then(|()| {
            let json = serde_json::to_vec_pretty(&self.settings).map_err(io::Error::other)?;
            atomic_write(&self.config_dir.join("settings.json"), &json)
        });
        if let Err(e) = result {
            self.fail(format!("Could not save preferences: {e}"));
        }
    }
    fn refresh(&mut self) {
        self.date = today();
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
        let ids: BTreeSet<_> = self
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
        self.selection.retain(|id| ids.contains(id));
        if self.selection.is_empty()
            && let Some(id) = self.rows.iter().find_map(|r| {
                if let Row::Task(id) = r {
                    Some(*id)
                } else {
                    None
                }
            })
        {
            self.selection.insert(id);
        }
    }
    fn load(&mut self, path: PathBuf) {
        match Document::open(path) {
            Ok(doc) => {
                self.settings.file = Some(doc.path.clone());
                self.document = Some(doc);
                self.selection.clear();
                self.editing = None;
                self.editor.clear();
                self.refresh();
                self.save_settings();
            }
            Err(e) => self.fail(e),
        }
    }
    fn selected(&self) -> Vec<(usize, Task)> {
        self.tasks
            .iter()
            .filter(|(id, _)| self.selection.contains(id))
            .cloned()
            .collect()
    }
    fn apply(&mut self, operation: impl Fn(&Task) -> Option<String>) {
        let changes = self
            .selected()
            .into_iter()
            .map(|(id, t)| (id, operation(&t)))
            .collect();
        if let Some(doc) = &mut self.document {
            match doc.replace(&changes) {
                Ok(()) => self.refresh(),
                Err(e) => self.fail(e),
            }
        }
    }
    fn submit(&mut self) {
        if self.document.is_none() {
            self.fail("Open or create a todo.txt file first.");
            return;
        }
        let raw = if self.settings.preserve_blank {
            &self.editor
        } else {
            self.editor.trim()
        };
        if raw.is_empty() {
            return;
        }
        let task = Task::parse(raw, self.date);
        let raw = if self.editing.is_none() && self.settings.add_creation {
            task.with_creation_date(self.date)
        } else {
            task.raw
        };
        let doc = self.document.as_mut().unwrap();
        let result = if let Some(id) = self.editing {
            doc.replace(&BTreeMap::from([(id, Some(raw))])).map(|()| id)
        } else {
            doc.add(&raw)
        };
        match result {
            Ok(id) => {
                self.editor.clear();
                self.editing = None;
                self.selection = BTreeSet::from([id]);
                self.scroll_to = Some(id);
                self.refresh();
                self.list_focus = self.settings.focus_list;
                if !self.list_focus {
                    self.focus_editor = true;
                }
            }
            Err(e) => self.fail(e),
        }
    }
    fn archive(&mut self) {
        if self.document.is_none() {
            return;
        }
        let default = self
            .document
            .as_ref()
            .unwrap()
            .path
            .with_file_name("done.txt");
        let path = if self.settings.auto_archive_path {
            Some(default)
        } else {
            self.settings.archive.clone().or_else(|| {
                rfd::FileDialog::new()
                    .set_title("Archive completed tasks")
                    .set_file_name("done.txt")
                    .add_filter("Text documents", &["txt"])
                    .save_file()
            })
        };
        if let Some(path) = path {
            match self.document.as_mut().unwrap().archive(&path, self.date) {
                Ok(_) => {
                    self.settings.archive = Some(path);
                    self.editing = None;
                    self.refresh();
                    self.save_settings();
                }
                Err(e) => self.fail(e),
            }
        }
    }
    fn action(&mut self, action: Action, ctx: &egui::Context) {
        self.settings
            .debug_event(&self.config_dir, &format!("action {action:?}"));
        match action {
            Action::Open => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Text documents", &["txt"])
                    .pick_file()
                {
                    self.load(path);
                }
            }
            Action::NewFile => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("todo.txt")
                    .add_filter("Text documents", &["txt"])
                    .save_file()
                {
                    match fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                    {
                        Ok(_) => self.load(path),
                        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => self.load(path),
                        Err(e) => self.fail(e),
                    }
                }
            }
            Action::NewTask => {
                self.editing = None;
                self.editor = self
                    .settings
                    .filter
                    .lines()
                    .filter(|s| !s.starts_with('-') && *s != "DONE")
                    .map(|s| s.replace("due:active", "due:today"))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.focus_editor = true;
                self.list_focus = false;
            }
            Action::Edit | Action::Duplicate => {
                if let [(id, task)] = self.selected().as_slice() {
                    self.editor = task.raw.clone();
                    self.editing = if matches!(action, Action::Edit) {
                        Some(*id)
                    } else {
                        None
                    };
                    self.focus_editor = true;
                    self.list_focus = false;
                }
            }
            Action::Delete if !self.selection.is_empty() => self.dialog = Some(Dialog::Delete),
            Action::Append if !self.selection.is_empty() => {
                self.dialog = Some(Dialog::Append(String::new()))
            }
            Action::Toggle => {
                let date = self.date;
                self.apply(|t| Some(t.toggle(date)));
                if self.settings.auto_archive {
                    self.archive();
                }
            }
            Action::Archive => self.archive(),
            Action::Reload => {
                if let Some(doc) = &mut self.document {
                    let before = doc.byte_len();
                    let result = doc.reload();
                    self.settings.debug_event(
                        &self.config_dir,
                        &format!(
                            "reload loaded_bytes_before={before} accepted={} loaded_bytes_after={}",
                            result.is_ok(),
                            doc.byte_len()
                        ),
                    );
                    match result {
                        Ok(()) => {
                            self.editing = None;
                            self.refresh();
                        }
                        Err(e) => self.fail(e),
                    }
                }
            }
            Action::Priority if !self.selection.is_empty() => {
                self.dialog = Some(Dialog::Priority(String::new()))
            }
            Action::ShiftPriority(delta) => self.apply(|t| Some(t.shifted_priority(delta))),
            Action::Date(key) if !self.selection.is_empty() => {
                let task = self.selected().first().map(|(_, t)| t.clone()).unwrap();
                let date = if key == "due" {
                    task.due_date
                } else {
                    task.threshold_date
                };
                self.dialog = Some(Dialog::Date(
                    key,
                    if date.is_empty() {
                        self.date.to_string()
                    } else {
                        date
                    },
                ));
            }
            Action::ShiftDate(key, days) => {
                let date = self.date;
                self.apply(|t| Some(t.shift_date(key, days, date)));
            }
            Action::RemoveDate(key) => self.apply(|t| Some(t.with_date(key, None))),
            Action::Postpone(key) if !self.selection.is_empty() => {
                self.dialog = Some(Dialog::Postpone(key, "1".to_owned()))
            }
            Action::Sort(sort) => {
                self.settings.sort = sort;
                self.refresh();
                self.save_settings();
            }
            Action::Filters => self.dialog = Some(Dialog::Filters(Box::new(self.settings.clone()))),
            Action::Options => self.dialog = Some(Dialog::Options(Box::new(self.settings.clone()))),
            Action::Preset(n) => {
                self.settings.filter = if n == 0 {
                    String::new()
                } else {
                    self.settings.presets[n - 1].clone()
                };
                self.settings.active_preset = n;
                self.refresh();
                self.save_settings();
            }
            Action::HideFuture => {
                self.settings.hide_future = !self.settings.hide_future;
                self.refresh();
                self.save_settings();
            }
            Action::ShowHidden => {
                self.settings.show_hidden = !self.settings.show_hidden;
                self.refresh();
                self.save_settings();
            }
            Action::Help => self.dialog = Some(Dialog::Help),
            Action::Calendar => {
                self.calendar = !self.calendar;
                let title = if self.calendar {
                    let days = (0..7)
                        .map(|n| {
                            let date = today() + Duration::days(n);
                            format!(
                                "  {}:{}",
                                &date.format("%A").to_string()[..2],
                                date.format("%m-%d")
                            )
                        })
                        .collect::<String>();
                    format!("todotxt.rs       Calendar:  {days}")
                } else {
                    "todotxt.rs".into()
                };
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
            }
            Action::Print => self.print(),
            Action::Log => {
                let path = self.config_dir.join("error.log");
                if !path.exists() {
                    let _ = fs::write(&path, "No errors recorded.\n");
                }
                if let Err(e) = open::that(path) {
                    self.fail(e);
                }
            }
            Action::Copy | Action::Cut => {
                if !self.list_focus {
                    ctx.memory_mut(|m| m.request_focus(egui::Id::new("task_editor")));
                    ctx.input_mut(|i| {
                        i.events.push(if matches!(action, Action::Cut) {
                            egui::Event::Cut
                        } else {
                            egui::Event::Copy
                        })
                    });
                    return;
                }
                ctx.copy_text(
                    self.selected()
                        .iter()
                        .map(|(_, t)| t.raw.as_str())
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                if matches!(action, Action::Cut) && !self.selection.is_empty() {
                    self.dialog = Some(Dialog::Delete);
                }
            }
            Action::Paste => match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                Ok(text) => {
                    if self.list_focus {
                        self.paste_tasks(&text);
                    } else {
                        ctx.memory_mut(|m| m.request_focus(egui::Id::new("task_editor")));
                        ctx.input_mut(|i| i.events.push(egui::Event::Paste(text)));
                    }
                }
                Err(e) => self.fail(e),
            },
            Action::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            _ => {}
        }
        if self.dialog.is_some() {
            self.dialog_focus = true;
        }
    }
    fn paste_tasks(&mut self, text: &str) {
        if let Some(doc) = &mut self.document {
            for line in text.lines().filter(|line| !line.is_empty()) {
                let raw = Task::parse(line, self.date).raw;
                if let Err(e) = doc.add(&raw) {
                    self.fail(e);
                    break;
                }
            }
            self.refresh();
        }
    }
    fn print(&mut self) {
        let html = todotxt_rs::printing::html(&self.tasks, &self.rows);
        let path = self.config_dir.join("print-preview.html");
        match fs::write(&path, html).and_then(|()| open::that(path).map_err(io::Error::other)) {
            Ok(()) => {}
            Err(e) => self.fail(e),
        }
    }
    fn navigate(&mut self, delta: isize, extend: bool) {
        let mut ids = Vec::new();
        for row in &self.rows {
            if let Row::Task(id) = row
                && !ids.contains(id)
            {
                ids.push(*id);
            }
        }
        if ids.is_empty() {
            return;
        }
        let current = self.anchor.or_else(|| self.selection.first().copied());
        let index = current
            .and_then(|id| ids.iter().position(|n| *n == id))
            .unwrap_or(0);
        let next = (index as isize + delta).clamp(0, ids.len() as isize - 1) as usize;
        if !extend {
            self.selection.clear();
        }
        self.selection.insert(ids[next]);
        self.anchor = Some(ids[next]);
        self.scroll_to = Some(ids[next]);
    }
    fn keyboard(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || self.error.is_some() || egui::Popup::is_any_open(ctx) {
            return;
        }
        let editor_id = egui::Id::new("task_editor");
        let editing = ctx.memory(|m| m.has_focus(editor_id));
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            if !editing {
                match &event {
                    egui::Event::Copy => self.action(Action::Copy, ctx),
                    egui::Event::Paste(text) => self.paste_tasks(text),
                    egui::Event::Cut => self.action(Action::Cut, ctx),
                    _ => {}
                }
            }
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers: m,
                ..
            } = event
            else {
                continue;
            };
            let ctrl = m.ctrl || m.command;
            let action = if key == Key::F5 {
                Some(Action::Reload)
            } else if key == Key::F10 {
                Some(Action::Options)
            } else if ctrl && key == Key::O {
                Some(Action::Open)
            } else if ctrl && key == Key::N {
                Some(Action::NewFile)
            } else if ctrl && key == Key::P && !m.alt {
                Some(Action::Print)
            } else if !editing {
                if ctrl && !m.alt {
                    match key {
                        Key::Num0 => Some(Action::Sort(Sort::File)),
                        Key::Num1 => Some(Action::Sort(Sort::Alphabetical)),
                        Key::Num2 => Some(Action::Sort(Sort::Completed)),
                        Key::Num3 => Some(Action::Sort(Sort::Context)),
                        Key::Num4 => Some(Action::Sort(Sort::Due)),
                        Key::Num5 => Some(Action::Sort(Sort::Created)),
                        Key::Num6 => Some(Action::Sort(Sort::Priority)),
                        Key::Num7 => Some(Action::Sort(Sort::Project)),
                        Key::S => Some(Action::Date("t")),
                        Key::T => Some(Action::HideFuture),
                        Key::H => Some(Action::ShowHidden),
                        Key::C if m.shift => Some(Action::Duplicate),
                        Key::A => {
                            self.selection = self
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
                            None
                        }
                        _ => arrow_action(key, "t"),
                    }
                } else if ctrl && m.alt {
                    if key == Key::P {
                        Some(Action::Postpone("t"))
                    } else {
                        arrow_action(key, "due")
                    }
                } else if m.alt {
                    match key {
                        Key::ArrowUp => Some(Action::ShiftPriority(-1)),
                        Key::ArrowDown => Some(Action::ShiftPriority(1)),
                        _ => None,
                    }
                } else {
                    match key {
                        Key::N => Some(Action::NewTask),
                        Key::O => Some(Action::Open),
                        Key::C => Some(Action::NewFile),
                        Key::U | Key::F2 => Some(Action::Edit),
                        Key::X => Some(Action::Toggle),
                        Key::A => Some(Action::Archive),
                        Key::D | Key::Delete | Key::Backspace => Some(Action::Delete),
                        Key::T => Some(Action::Append),
                        Key::F => Some(Action::Filters),
                        Key::I => Some(Action::Priority),
                        Key::S => Some(Action::Date("due")),
                        Key::P => Some(Action::Postpone("due")),
                        Key::Period => Some(Action::Reload),
                        Key::Questionmark => Some(Action::Help),
                        Key::Num0 => Some(Action::Preset(0)),
                        Key::Num1 => Some(Action::Preset(1)),
                        Key::Num2 => Some(Action::Preset(2)),
                        Key::Num3 => Some(Action::Preset(3)),
                        Key::Num4 => Some(Action::Preset(4)),
                        Key::Num5 => Some(Action::Preset(5)),
                        Key::Num6 => Some(Action::Preset(6)),
                        Key::Num7 => Some(Action::Preset(7)),
                        Key::Num8 => Some(Action::Preset(8)),
                        Key::Num9 => Some(Action::Preset(9)),
                        Key::J | Key::ArrowDown => {
                            self.navigate(1, m.shift);
                            None
                        }
                        Key::K | Key::ArrowUp => {
                            self.navigate(-1, m.shift);
                            None
                        }
                        Key::Home => {
                            self.navigate(-(self.rows.len() as isize), m.shift);
                            None
                        }
                        Key::End => {
                            self.navigate(self.rows.len() as isize, m.shift);
                            None
                        }
                        Key::PageDown => {
                            self.navigate(10, m.shift);
                            None
                        }
                        Key::PageUp => {
                            self.navigate(-10, m.shift);
                            None
                        }
                        _ => None,
                    }
                }
            } else {
                None
            };
            if m.alt && !ctrl && matches!(key, Key::ArrowLeft | Key::ArrowRight) && !editing {
                self.apply(|t| Some(t.with_priority(None)));
            } else if let Some(action) = action {
                self.action(action, ctx);
            }
            if editing && key == Key::Escape {
                self.editor.clear();
                self.editing = None;
                self.list_focus = true;
                ctx.memory_mut(|m| m.surrender_focus(editor_id));
            }
        }
    }
    fn menus(&mut self, ui: &mut egui::Ui) {
        let mut chosen = None;
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                menu(ui, "New", "Ctrl+N", Action::NewFile, &mut chosen);
                menu(ui, "Open", "Ctrl+O", Action::Open, &mut chosen);
                menu(ui, "Print", "Ctrl+P", Action::Print, &mut chosen);
                menu(
                    ui,
                    "Print Preview",
                    "Ctrl+Shift+P",
                    Action::Print,
                    &mut chosen,
                );
                ui.separator();
                menu(
                    ui,
                    "Archive Completed Tasks",
                    "A",
                    Action::Archive,
                    &mut chosen,
                );
                menu(ui, "Reload File", ".", Action::Reload, &mut chosen);
                ui.separator();
                menu(ui, "Options...", "F10", Action::Options, &mut chosen);
                ui.separator();
                menu(ui, "Exit", "Alt+F4", Action::Exit, &mut chosen);
            });
            ui.menu_button("Edit", |ui| {
                menu(ui, "Cut", "Ctrl+X", Action::Cut, &mut chosen);
                menu(ui, "Copy", "Ctrl+C", Action::Copy, &mut chosen);
                menu(
                    ui,
                    "Copy Task to New Task",
                    "Ctrl+Shift+C",
                    Action::Duplicate,
                    &mut chosen,
                );
                menu(ui, "Paste", "Ctrl+V", Action::Paste, &mut chosen);
            });
            ui.menu_button("Task", |ui| {
                menu(ui, "Add New Task", "N", Action::NewTask, &mut chosen);
                menu(ui, "Update Task", "U", Action::Edit, &mut chosen);
                menu(ui, "Append Text", "T", Action::Append, &mut chosen);
                menu(ui, "Delete Task", "D", Action::Delete, &mut chosen);
                ui.separator();
                menu(ui, "Toggle Completion", "X", Action::Toggle, &mut chosen);
                ui.separator();
                menu(ui, "Set Priority", "I", Action::Priority, &mut chosen);
                menu(
                    ui,
                    "Increase Priority",
                    "Alt+Up",
                    Action::ShiftPriority(-1),
                    &mut chosen,
                );
                menu(
                    ui,
                    "Decrease Priority",
                    "Alt+Down",
                    Action::ShiftPriority(1),
                    &mut chosen,
                );
                // Empty priority is handled by the priority dialog or Alt+Left/Right.
                if ui
                    .button("Remove Priority                    Alt+Left/Right")
                    .clicked()
                {
                    self.apply(|t| Some(t.with_priority(None)));
                    ui.close();
                }
                for (key, name, prefix) in [("due", "Due", "Ctrl+Alt"), ("t", "Threshold", "Ctrl")]
                {
                    ui.separator();
                    menu(
                        ui,
                        &format!("Set {name} Date"),
                        if key == "due" { "S" } else { "Ctrl+S" },
                        Action::Date(key),
                        &mut chosen,
                    );
                    menu(
                        ui,
                        if key == "due" {
                            "Postpone"
                        } else {
                            "Threshold"
                        },
                        if key == "due" { "P" } else { "Ctrl+Alt+P" },
                        Action::Postpone(key),
                        &mut chosen,
                    );
                    menu(
                        ui,
                        &format!("Increase {name} Date By 1 Day"),
                        &format!("{prefix}+Up"),
                        Action::ShiftDate(key, 1),
                        &mut chosen,
                    );
                    menu(
                        ui,
                        &format!("Decrease {name} Date By 1 Day"),
                        &format!("{prefix}+Down"),
                        Action::ShiftDate(key, -1),
                        &mut chosen,
                    );
                    menu(
                        ui,
                        &format!("Remove {name} Date"),
                        &format!("{prefix}+Left/Right"),
                        Action::RemoveDate(key),
                        &mut chosen,
                    );
                }
            });
            ui.menu_button("Sort", |ui| {
                for (n, sort) in Sort::ALL.into_iter().enumerate() {
                    let label = format!(
                        "{} {}",
                        if self.settings.sort == sort {
                            "✓"
                        } else {
                            "  "
                        },
                        sort.label()
                    );
                    menu(
                        ui,
                        &label,
                        &format!("Ctrl+{n}"),
                        Action::Sort(sort),
                        &mut chosen,
                    );
                }
            });
            ui.menu_button(
                if self.settings.filter.is_empty() {
                    RichText::new("Filter")
                } else {
                    RichText::new("Filter").strong()
                },
                |ui| {
                    menu(
                        ui,
                        &format!(
                            "{} Hide future tasks",
                            if self.settings.hide_future {
                                "✓"
                            } else {
                                "  "
                            }
                        ),
                        "Ctrl+T",
                        Action::HideFuture,
                        &mut chosen,
                    );
                    menu(
                        ui,
                        &format!(
                            "{} Show hidden tasks",
                            if self.settings.show_hidden {
                                "✓"
                            } else {
                                "  "
                            }
                        ),
                        "Ctrl+H",
                        Action::ShowHidden,
                        &mut chosen,
                    );
                    ui.separator();
                    menu(ui, "Define Filters", "F", Action::Filters, &mut chosen);
                    ui.separator();
                    menu(ui, "Remove Filter", "0", Action::Preset(0), &mut chosen);
                    ui.separator();
                    for n in 1..=9 {
                        menu(
                            ui,
                            &format!("Apply Preset Filter {n}"),
                            &n.to_string(),
                            Action::Preset(n),
                            &mut chosen,
                        );
                    }
                },
            );
            ui.menu_button("Help", |ui| {
                menu(ui, "About / Help", "?", Action::Help, &mut chosen);
                menu(ui, "View Error Log", "", Action::Log, &mut chosen);
                menu(ui, "Show Calendar", "", Action::Calendar, &mut chosen);
            });
        });
        if let Some(action) = chosen {
            self.action(action, ui.ctx());
        }
    }
    fn editor(&mut self, ui: &mut egui::Ui) {
        let id = egui::Id::new("task_editor");
        let focused = ui.memory(|m| m.has_focus(id));
        let cursor = egui::TextEdit::load_state(ui.ctx(), id)
            .and_then(|s| s.cursor.char_range())
            .map(|r| r.primary.index)
            .unwrap_or_else(|| self.editor.chars().count());
        let before: String = self.editor.chars().take(cursor).collect();
        let start = before.rfind(char::is_whitespace).map_or(0, |p| p + 1);
        let prefix = &before[start..];
        let suggestions =
            if focused && !self.completion_dismissed && prefix.starts_with(['+', '@', '(']) {
                view::suggestions(&self.tasks, prefix, self.settings.intellisense_case)
            } else {
                Vec::new()
            };
        self.completion_index = self
            .completion_index
            .min(suggestions.len().saturating_sub(1));
        let mut accept = None;
        let mut consumed_enter = false;
        if !suggestions.is_empty() {
            ui.input_mut(|i| {
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowDown) {
                    self.completion_index = (self.completion_index + 1) % suggestions.len();
                }
                if i.consume_key(egui::Modifiers::NONE, Key::ArrowUp) {
                    self.completion_index = self.completion_index.saturating_sub(1);
                }
                if i.consume_key(egui::Modifiers::NONE, Key::Tab)
                    || i.consume_key(egui::Modifiers::NONE, Key::Enter)
                    || i.consume_key(egui::Modifiers::NONE, Key::Space)
                {
                    i.events
                        .retain(|event| !matches!(event, egui::Event::Text(text) if text == " "));
                    accept = Some(suggestions[self.completion_index].clone());
                    consumed_enter = true;
                }
            });
        }
        let output = egui::TextEdit::singleline(&mut self.editor)
            .id(id)
            .font(FontId::proportional(12.0))
            .desired_width(f32::INFINITY)
            .margin(Vec2::new(2.0, 3.0))
            .show(ui);
        if output.response.changed() {
            self.completion_dismissed = false;
            self.completion_index = 0;
        }
        if self.focus_editor {
            output.response.request_focus();
            self.focus_editor = false;
            let end = egui::text::CCursor::new(self.editor.chars().count());
            let mut state = output.state.clone();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(end)));
            state.store(ui.ctx(), id);
        }
        if output.response.has_focus() {
            self.list_focus = false;
        }
        if !suggestions.is_empty() {
            egui::Area::new(egui::Id::new("completions"))
                .order(egui::Order::Foreground)
                .fixed_pos(output.response.rect.left_bottom())
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.set_min_width(150.0);
                        egui::ScrollArea::vertical()
                            .max_height(160.0)
                            .show(ui, |ui| {
                                for (n, tag) in suggestions.iter().enumerate() {
                                    if ui
                                        .selectable_label(n == self.completion_index, tag)
                                        .clicked()
                                    {
                                        accept = Some(tag.clone());
                                    }
                                }
                            });
                    });
                });
        }
        if let Some(tag) = accept {
            let tail: String = self.editor.chars().skip(cursor).collect();
            self.editor = format!("{}{tag}{tail}", &before[..start]);
            let end =
                egui::text::CCursor::new(before[..start].chars().count() + tag.chars().count());
            let mut state = output.state.clone();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(end)));
            state.store(ui.ctx(), id);
            self.focus_editor = false;
            output.response.request_focus();
            self.completion_dismissed = true;
        }
        let enter = ui.input(|i| {
            i.key_pressed(Key::Enter) && (!self.settings.ctrl_enter || i.modifiers.ctrl)
        });
        if self.settings.ctrl_enter
            && output.response.lost_focus()
            && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.ctrl)
        {
            // A single-line TextEdit normally relinquishes focus on Enter. In
            // Ctrl-Enter mode a plain Enter must leave the draft editable.
            output.response.request_focus();
            self.list_focus = false;
        }
        if self.dialog.is_none()
            && self.error.is_none()
            && (output.response.has_focus() || output.response.lost_focus())
            && enter
            && !consumed_enter
        {
            self.submit();
            if self.list_focus {
                ui.memory_mut(|m| m.surrender_focus(id));
            }
        }
    }
    fn task_list(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        let rows = self.rows.clone();
        let grouped = self.settings.grouping
            && !matches!(self.settings.sort, Sort::File | Sort::Alphabetical);
        let scroll = if self.settings.word_wrap {
            egui::ScrollArea::vertical()
        } else {
            egui::ScrollArea::both()
        };
        scroll
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.add_space(10.0);
                let mut alternate = 0;
                for row in rows {
                    match row {
                        Row::Header(name) => {
                            ui.add_space(10.0);
                            ui.add(egui::Label::new(
                                RichText::new(name)
                                    .size(self.settings.font_size)
                                    .strong()
                                    .family(egui::FontFamily::Name("system-bold".into())),
                            ));
                            ui.add_space(3.0);
                            alternate = 0;
                        }
                        Row::Task(id) => {
                            let Some((_, task)) = self.tasks.iter().find(|(n, _)| *n == id) else {
                                continue;
                            };
                            let task = task.clone();
                            let selected = self.selection.contains(&id);
                            let color = if selected {
                                Color32::BLACK
                            } else if task.completed {
                                Color32::from_gray(190)
                            } else if parse_date(&task.due_date).is_some_and(|d| d < self.date) {
                                Color32::RED
                            } else if parse_date(&task.due_date) == Some(self.date) {
                                Color32::from_rgb(0, 128, 0)
                            } else {
                                Color32::from_rgb(
                                    self.settings.font_color as u8,
                                    (self.settings.font_color >> 8) as u8,
                                    (self.settings.font_color >> 16) as u8,
                                )
                            };
                            let indent = if grouped { 12.0 } else { 4.0 };
                            let max_width = if self.settings.word_wrap {
                                (ui.available_width() - indent - 4.0).max(20.0)
                            } else {
                                f32::INFINITY
                            };
                            let (job, links) = task_layout(&task, &self.settings, color, max_width);
                            let galley = ui.fonts_mut(|f| f.layout_job(job));
                            let width = ui.available_width().max(galley.size().x + indent + 4.0);
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(width, galley.size().y + 4.0),
                                egui::Sense::click(),
                            );
                            let bg = if selected {
                                Color32::from_rgb(229, 243, 251)
                            } else if alternate % 2 == 1 {
                                Color32::from_gray(248)
                            } else {
                                Color32::WHITE
                            };
                            ui.painter().rect_filled(rect, 0.0, bg);
                            if selected {
                                ui.painter().rect_stroke(
                                    rect,
                                    0.0,
                                    Stroke::new(1.0_f32, Color32::from_rgb(203, 203, 203)),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            let text_pos = rect.min + Vec2::new(indent, 2.0);
                            ui.painter().galley(text_pos, galley.clone(), color);
                            let mut link_clicked = false;
                            if let Some(pointer) = response.hover_pos() {
                                let cursor = galley.cursor_from_pos(pointer - text_pos).index;
                                if let Some((_, _, url)) = links
                                    .iter()
                                    .find(|(start, end, _)| cursor >= *start && cursor < *end)
                                {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    response.clone().on_hover_text(url);
                                    if response.clicked() {
                                        if let Err(e) = open::that(url) {
                                            self.fail(e);
                                        }
                                        link_clicked = true;
                                    }
                                }
                            }
                            if response.clicked() && !link_clicked {
                                ui.memory_mut(|m| m.surrender_focus(egui::Id::new("task_editor")));
                                self.list_focus = true;
                                let modifiers = ui.input(|i| i.modifiers);
                                if modifiers.shift {
                                    let visible: Vec<_> = self
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
                                    if let (Some(a), Some(b)) = (
                                        visible.iter().position(|n| Some(*n) == self.anchor),
                                        visible.iter().position(|n| *n == id),
                                    ) {
                                        if !modifiers.ctrl {
                                            self.selection.clear();
                                        }
                                        self.selection
                                            .extend(visible[a.min(b)..=a.max(b)].iter().copied());
                                    } else {
                                        self.selection.insert(id);
                                    }
                                } else if modifiers.ctrl {
                                    if !self.selection.remove(&id) {
                                        self.selection.insert(id);
                                    }
                                    self.anchor = Some(id);
                                } else {
                                    self.selection = BTreeSet::from([id]);
                                    self.anchor = Some(id);
                                }
                            }
                            if response.double_clicked() && !link_clicked {
                                self.selection = BTreeSet::from([id]);
                                self.action(Action::Edit, ui.ctx());
                            }
                            if self.scroll_to == Some(id) {
                                response.scroll_to_me(Some(egui::Align::Center));
                                self.scroll_to = None;
                            }
                            alternate += 1;
                        }
                    }
                }
            });
    }
    fn status(&self, ui: &mut egui::Ui) {
        ui.spacing_mut().interact_size.y = 14.0;
        let counts = view::counts(&self.tasks, &self.rows, self.date);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.label(if self.settings.active_preset > 0 {
                format!("Filter #: {}", self.settings.active_preset)
            } else if self.settings.filter.is_empty() {
                "Filter: None".to_owned()
            } else {
                "Filter: Custom".to_owned()
            });
            ui.separator();
            ui.label(format!("Sort: {}", self.settings.sort.label()));
            ui.separator();
            ui.label(format!(
                "Tasks: {} of {}",
                counts["visible"],
                self.tasks.len()
            ));
            ui.separator();
            ui.label(format!("Incomplete: {}", counts["incomplete"]));
            ui.separator();
            ui.label(format!("Due Today: {}", counts["today"]));
            ui.separator();
            ui.label(format!("Overdue: {}", counts["overdue"]));
        });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(error) = self.error.clone() {
            egui::Modal::new(egui::Id::new("error")).show(ctx, |ui| {
                ui.set_width(380.0);
                ui.heading("Error");
                ui.label(error);
                if ui.button("OK").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                    self.error = None;
                }
            });
            return;
        }
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut accepted = false;
        let mut cancelled = false;
        egui::Modal::new(egui::Id::new("dialog")).show(ctx, |ui| {
            ui.set_width(if matches!(dialog, Dialog::Options(_)) {
                465.0
            } else {
                380.0
            });
            match &mut dialog {
                Dialog::Delete => {
                    ui.heading("Delete Tasks");
                    ui.label("Are you sure you want to delete the selected tasks?");
                }
                Dialog::Append(text) => {
                    ui.heading("Append Text");
                    dialog_input(ui, text, f32::INFINITY, &mut self.dialog_focus);
                }
                Dialog::Priority(text) => {
                    ui.heading("Set Priority");
                    ui.label("Priority (A–Z), or leave blank to remove:");
                    dialog_input(ui, text, 80.0, &mut self.dialog_focus);
                }
                Dialog::Date(key, text) => {
                    ui.heading(if *key == "due" {
                        "Set Due Date"
                    } else {
                        "Set Threshold Date"
                    });
                    ui.label("Date (YYYY-MM-DD, today, tomorrow, or weekday):");
                    dialog_input(ui, text, f32::INFINITY, &mut self.dialog_focus);
                    if resolve_date(text, self.date).is_none() {
                        ui.colored_label(Color32::RED, "Enter a valid date.");
                    }
                }
                Dialog::Postpone(key, text) => {
                    ui.heading(if *key == "due" {
                        "Postpone"
                    } else {
                        "Threshold"
                    });
                    ui.label("Number of days to add to the date:");
                    dialog_input(ui, text, 80.0, &mut self.dialog_focus);
                }
                Dialog::Filters(settings) => {
                    ui.heading("Filter");
                    ui.label("Press 1–9 in the task list to apply a preset. 0 clears the filter.");
                    egui::ScrollArea::vertical()
                        .max_height(430.0)
                        .show(ui, |ui| {
                            ui.label("Currently active filter:");
                            filter_editor(
                                ui,
                                &mut settings.filter,
                                &self.tasks,
                                settings.intellisense_case,
                                egui::Id::new("filter_active"),
                            );
                            for (n, text) in settings.presets.iter_mut().enumerate() {
                                ui.add_space(8.0);
                                ui.label(format!("Preset filter #{}:", n + 1));
                                filter_editor(
                                    ui,
                                    text,
                                    &self.tasks,
                                    settings.intellisense_case,
                                    egui::Id::new(("filter_preset", n)),
                                );
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui.button("Clear Active").clicked() {
                            settings.filter.clear();
                        }
                        if ui.button("Clear All").clicked() {
                            settings.filter.clear();
                            settings.presets = Default::default();
                        }
                    });
                }
                Dialog::Options(s) => {
                    ui.heading("Options");
                    ui.horizontal(|ui| {
                        ui.label("Archive File");
                        let mut path = s
                            .archive
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default();
                        if ui
                            .add(egui::TextEdit::singleline(&mut path).desired_width(260.0))
                            .changed()
                        {
                            s.archive = if path.is_empty() {
                                None
                            } else {
                                Some(path.into())
                            };
                        }
                        if ui.button("Select...").clicked()
                            && let Some(path) =
                                rfd::FileDialog::new().set_file_name("done.txt").save_file()
                        {
                            s.archive = Some(path);
                        }
                    });
                    ui.checkbox(&mut s.auto_archive, "Automatically archive completed tasks");
                    ui.checkbox(
                        &mut s.auto_archive_path,
                        "Automatically select archive path (done.txt)",
                    );
                    ui.horizontal(|ui| {
                        ui.label("Task font size");
                        ui.add(egui::DragValue::new(&mut s.font_size).range(8.0..=96.0));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut s.font_italic, "Italic");
                        ui.checkbox(&mut s.font_underline, "Underline");
                        ui.checkbox(&mut s.font_strike, "Strikeout");
                        let mut color = [
                            s.font_color as u8,
                            (s.font_color >> 8) as u8,
                            (s.font_color >> 16) as u8,
                        ];
                        if ui.color_edit_button_srgb(&mut color).changed() {
                            s.font_color =
                                color[0] as u32 | (color[1] as u32) << 8 | (color[2] as u32) << 16;
                        }
                    });
                    ui.add_space(12.0);
                    for (value, label) in [
                        (&mut s.add_creation, "Add created date to new tasks"),
                        (
                            &mut s.focus_list,
                            "Move focus to task list after creating new task",
                        ),
                        (
                            &mut s.auto_refresh,
                            "Automatically refresh task list from file",
                        ),
                        (&mut s.case_sensitive, "Filter text is case-sensitive"),
                        (
                            &mut s.intellisense_case,
                            "Intellisense project and context suggestions are case-sensitive",
                        ),
                        (&mut s.ctrl_enter, "Use Ctrl-Enter to create new task"),
                        (&mut s.grouping, "Allow grouping of tasks"),
                        (
                            &mut s.preserve_blank,
                            "Display blank lines and preserve whitespace in edits",
                        ),
                        (&mut s.word_wrap, "Apply word wrap to task list"),
                        (&mut s.status_bar, "Display status bar"),
                        (&mut s.debug_logging, "Enable debug logging"),
                    ] {
                        ui.checkbox(value, label);
                    }
                }
                Dialog::Help => {
                    ui.heading("todotxt.rs");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.label("A Rust port of todotxt.net by Ben Hughes. BSD licensed.");
                    egui::ScrollArea::vertical()
                        .max_height(410.0)
                        .show(ui, |ui| {
                            ui.label(include_str!("../docs/HELP.md"));
                        });
                }
            }
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(if matches!(dialog, Dialog::Delete) {
                        "Delete"
                    } else {
                        "OK"
                    })
                    .clicked()
                {
                    accepted = true;
                }
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
            });
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                cancelled = true;
            }
            if !matches!(dialog, Dialog::Filters(_) | Dialog::Options(_))
                && ui.input(|i| i.key_pressed(Key::Enter))
            {
                accepted = true;
            }
        });
        if cancelled {
            return;
        }
        if accepted {
            match &dialog {
                Dialog::Delete => {
                    self.apply(|_| None);
                    self.editing = None;
                    self.selection.clear();
                    self.refresh();
                }
                Dialog::Append(text) if !text.trim().is_empty() => {
                    let date = self.date;
                    self.apply(|t| {
                        Some(Task::parse(&format!("{} {}", t.raw, text.trim()), date).raw)
                    });
                }
                Dialog::Priority(text) => {
                    let value = text.trim().chars().next().map(|p| p.to_ascii_uppercase());
                    if text.trim().len() > 1 || value.is_some_and(|p| !p.is_ascii_uppercase()) {
                        self.fail("Priority must be a single letter A–Z, or blank.");
                        self.dialog = Some(dialog);
                        return;
                    }
                    self.apply(|t| Some(t.with_priority(value)));
                }
                Dialog::Date(key, text) => {
                    let Some(date) = resolve_date(text.trim(), self.date) else {
                        self.dialog = Some(dialog);
                        return;
                    };
                    self.apply(|t| Some(t.with_date(key, Some(date))));
                }
                Dialog::Postpone(key, text) => {
                    let Ok(days) = text.trim().parse::<i64>() else {
                        self.fail("Enter a whole number of days.");
                        self.dialog = Some(dialog);
                        return;
                    };
                    if !(-365000..=365000).contains(&days) {
                        self.fail("Number of days is out of range.");
                        self.dialog = Some(dialog);
                        return;
                    }
                    let date = self.date;
                    self.apply(|t| Some(t.shift_date(key, days, date)));
                }
                Dialog::Filters(settings) => {
                    self.settings = *settings.clone();
                    self.settings.active_preset = 0;
                    self.refresh();
                    self.save_settings();
                }
                Dialog::Options(settings) => {
                    self.settings = *settings.clone();
                    self.refresh();
                    self.save_settings();
                }
                _ => {}
            }
        } else {
            self.dialog = Some(dialog);
        }
    }
}

impl Desktop {
    fn render(&mut self, ctx: &egui::Context) {
        self.keyboard(ctx);
        if self.last_poll.elapsed() > StdDuration::from_secs(1) {
            self.last_poll = Instant::now();
            if self.date != today() {
                self.refresh();
            }
            if self.settings.auto_refresh
                && self.editing.is_none()
                && self.editor.is_empty()
                && self.dialog.is_none()
                && self.error.is_none()
                && let Some(doc) = &self.document
                && doc.can_auto_reload()
            {
                match doc.changed() {
                    Ok(true) => self.action(Action::Reload, ctx),
                    Ok(false) => {}
                    Err(e) => self.fail(e),
                }
            }
        }
        ctx.request_repaint_after(StdDuration::from_secs(1));
        egui::TopBottomPanel::top("menu")
            .exact_height(25.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_gray(245))
                    .inner_margin(egui::Margin::symmetric(2, 2)),
            )
            .show(ctx, |ui| self.menus(ui));
        egui::TopBottomPanel::top("editor")
            .exact_height(25.0)
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| self.editor(ui));
        if self.settings.status_bar {
            egui::TopBottomPanel::bottom("status")
                .exact_height(23.0)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_gray(240))
                        .inner_margin(egui::Margin::symmetric(2, 2)),
                )
                .show(ctx, |ui| self.status(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::WHITE))
            .show(ctx, |ui| self.task_list(ui));
        self.dialogs(ctx);
    }
}
impl eframe::App for Desktop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(ctx);
    }
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.save_settings();
    }
    fn persist_egui_memory(&self) -> bool {
        true
    }
}

fn arrow_action(key: Key, date_key: &'static str) -> Option<Action> {
    match key {
        Key::ArrowUp => Some(Action::ShiftDate(date_key, 1)),
        Key::ArrowDown => Some(Action::ShiftDate(date_key, -1)),
        Key::ArrowLeft | Key::ArrowRight => Some(Action::RemoveDate(date_key)),
        _ => None,
    }
}
fn dialog_input(ui: &mut egui::Ui, text: &mut String, width: f32, focus: &mut bool) {
    let id = egui::Id::new("dialog_input");
    let output = egui::TextEdit::singleline(text)
        .id(id)
        .desired_width(width)
        .show(ui);
    if *focus {
        output.response.request_focus();
        let mut state = output.state;
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(text.chars().count()),
            )));
        state.store(ui.ctx(), id);
        *focus = false;
    }
}
fn menu(
    ui: &mut egui::Ui,
    label: &str,
    shortcut: &str,
    action: Action,
    chosen: &mut Option<Action>,
) {
    if ui
        .add(egui::Button::new(label).shortcut_text(shortcut))
        .clicked()
    {
        *chosen = Some(action);
        ui.close();
    }
}
type Links = Vec<(usize, usize, String)>;
fn task_layout(
    task: &Task,
    settings: &Settings,
    color: Color32,
    width: f32,
) -> (egui::text::LayoutJob, Links) {
    use egui::text::{LayoutJob, TextFormat};
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    let mut links = Vec::new();
    let mut pos = 0;
    for word in task.raw.split_inclusive(char::is_whitespace) {
        let token = word.trim_end();
        let link = token.starts_with("https://")
            || token.starts_with("http://")
            || token.starts_with("ftp://")
            || token.starts_with("www.");
        let mut format = TextFormat {
            font_id: FontId::proportional(settings.font_size),
            color,
            italics: settings.font_italic,
            underline: if settings.font_underline {
                Stroke::new(1.0_f32, color)
            } else {
                Stroke::NONE
            },
            ..Default::default()
        };
        if task.completed || settings.font_strike {
            format.strikethrough = Stroke::new(1.0_f32, color);
        }
        if link {
            format.color = if task.completed {
                Color32::from_rgb(190, 190, 255)
            } else {
                Color32::BLUE
            };
            format.underline = Stroke::new(1.0_f32, format.color);
            let url = if token.starts_with("www.") {
                format!("http://{token}")
            } else {
                token.to_owned()
            };
            links.push((pos, pos + token.chars().count(), url));
        }
        job.append(token, 0.0, format);
        job.append(
            &word[token.len()..],
            0.0,
            TextFormat {
                font_id: FontId::proportional(settings.font_size),
                color,
                ..Default::default()
            },
        );
        pos += word.chars().count();
    }
    (job, links)
}
fn configure_style(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::light());
    let mut fonts = egui::FontDefinitions::default();
    let candidates = if cfg!(windows) {
        vec!["C:/Windows/Fonts/segoeui.ttf"]
    } else {
        vec![
            "/usr/share/fonts/truetype/msttcorefonts/Segoe_UI.ttf",
            "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ]
    };
    for path in candidates {
        if let Ok(bytes) = fs::read(path) {
            fonts.font_data.insert(
                "system-ui".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "system-ui".to_owned());
            break;
        }
    }
    let bold_candidates = if cfg!(windows) {
        vec!["C:/Windows/Fonts/segoeuib.ttf"]
    } else {
        vec![
            "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        ]
    };
    let mut bold_family = fonts.families[&egui::FontFamily::Proportional].clone();
    for path in bold_candidates {
        if let Ok(bytes) = fs::read(path) {
            fonts.font_data.insert(
                "system-bold".to_owned(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            bold_family.insert(0, "system-bold".to_owned());
            break;
        }
    }
    fonts
        .families
        .insert(egui::FontFamily::Name("system-bold".into()), bold_family);
    ctx.set_fonts(fonts);
    ctx.style_mut(|s| {
        for text_style in [
            egui::TextStyle::Body,
            egui::TextStyle::Button,
            egui::TextStyle::Small,
        ] {
            s.text_styles.insert(text_style, FontId::proportional(12.0));
        }
        s.spacing.item_spacing = Vec2::new(6.0, 4.0);
        s.spacing.button_padding = Vec2::new(6.0, 3.0);
        s.spacing.interact_size.y = 20.0;
        s.spacing.scroll = egui::style::ScrollStyle {
            bar_width: 17.0,
            bar_inner_margin: 0.0,
            ..egui::style::ScrollStyle::solid()
        };
        s.visuals.text_edit_bg_color = Some(Color32::WHITE);
        s.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_gray(171));
        s.visuals.widgets.inactive.corner_radius = 0.into();
        s.visuals.widgets.hovered.corner_radius = 0.into();
        s.visuals.widgets.active.corner_radius = 0.into();
        s.visuals.window_corner_radius = 0.into();
        s.visuals.override_text_color = Some(Color32::BLACK);
        s.visuals.selection.bg_fill = Color32::from_rgb(229, 243, 251);
    });
}

#[cfg(test)]
mod gui_tests {
    use super::*;

    struct Harness {
        app: Desktop,
        ctx: egui::Context,
        _dir: tempfile::TempDir,
    }
    impl Harness {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("todo.txt");
            fs::write(&path, "(A) first +test\nsecond @home\n").unwrap();
            let ctx = egui::Context::default();
            configure_style(&ctx);
            let mut app = Desktop {
                settings: Settings::default(),
                config_dir: dir.path().to_owned(),
                document: Some(Document::open(path).unwrap()),
                tasks: Vec::new(),
                rows: Vec::new(),
                selection: BTreeSet::new(),
                anchor: None,
                editor: String::new(),
                editing: None,
                focus_editor: false,
                list_focus: true,
                scroll_to: None,
                dialog: None,
                dialog_focus: false,
                error: None,
                last_poll: Instant::now(),
                date: today(),
                calendar: false,
                completion_index: 0,
                completion_dismissed: false,
            };
            app.refresh();
            let mut harness = Self {
                app,
                ctx,
                _dir: dir,
            };
            harness.frame(Vec::new());
            harness.frame(Vec::new());
            harness
        }
        fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
            self.ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(506.0, 1006.0),
                    )),
                    events,
                    focused: true,
                    ..Default::default()
                },
                |ctx| self.app.render(ctx),
            )
        }
        fn key(&mut self, key: Key) {
            self.frame(vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            self.frame(vec![egui::Event::Key {
                key,
                physical_key: Some(key),
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }]);
        }
        fn text(&mut self, text: &str) {
            self.frame(vec![egui::Event::Text(text.to_owned())]);
        }
        fn file(&self) -> String {
            fs::read_to_string(&self.app.document.as_ref().unwrap().path).unwrap()
        }
    }

    #[test]
    fn keyboard_add_edit_complete_and_confirmed_delete() {
        let mut h = Harness::new();
        h.key(Key::N);
        h.text("native sample Unicode +test ");
        h.key(Key::Enter);
        assert!(h.file().contains("native sample Unicode +test"));
        assert_eq!(h.app.tasks.len(), 3);
        h.key(Key::U);
        h.text(" updated");
        h.key(Key::Enter);
        assert!(h.file().contains("native sample Unicode +test updated"));
        h.key(Key::X);
        assert!(h.file().contains(&format!("x {} native", today())));
        let before = h.file();
        h.key(Key::D);
        h.key(Key::Escape);
        assert_eq!(h.file(), before);
        h.key(Key::D);
        h.key(Key::Enter);
        assert!(!h.file().contains("native"));
    }

    #[test]
    fn failed_submit_keeps_draft_and_error_then_reload_cancels_old_target() {
        let mut h = Harness::new();
        h.key(Key::U);
        h.text(" draft");
        let path = h.app.document.as_ref().unwrap().path.clone();
        fs::write(&path, "external writer\n").unwrap();
        h.key(Key::Enter);
        assert_eq!(h.file(), "external writer\n");
        assert!(h.app.error.as_deref().unwrap().contains("changed outside"));
        assert!(h.app.editor.ends_with(" draft"));
        assert_eq!(h.app.editing, Some(0));
        h.key(Key::Escape);
        h.key(Key::F5);
        assert_eq!(h.app.editing, None);
        assert!(h.app.editor.ends_with(" draft"));
        // The kept draft is now a new task; it cannot replace the external writer's line.
        h.key(Key::Enter);
        assert!(h.file().starts_with("external writer\n"));
    }

    #[test]
    fn completion_popup_and_keyboard_navigation_do_not_submit_early() {
        let mut h = Harness::new();
        h.key(Key::N);
        h.text("new +t");
        h.frame(Vec::new());
        h.key(Key::Tab);
        assert_eq!(h.app.editor, "new +test");
        assert_eq!(h.app.tasks.len(), 2);
        h.key(Key::Enter);
        assert_eq!(h.app.tasks.len(), 3);
        h.key(Key::J);
        assert!(!h.app.selection.is_empty());
        h.key(Key::F);
        assert!(matches!(h.app.dialog, Some(Dialog::Filters(_))));
        h.key(Key::Escape);
        assert!(h.app.dialog.is_none());
    }

    #[test]
    fn space_accepts_task_and_filter_suggestions_without_saving_or_closing() {
        let mut h = Harness::new();
        h.key(Key::N);
        h.text("new +t");
        h.frame(Vec::new());
        h.key(Key::Space);
        assert_eq!(h.app.editor, "new +test");
        assert_eq!(h.app.tasks.len(), 2);
        h.key(Key::Escape);
        h.key(Key::F);
        let id = egui::Id::new("filter_active");
        h.ctx.memory_mut(|m| m.request_focus(id));
        h.frame(Vec::new());
        h.text("+t");
        h.frame(Vec::new());
        h.key(Key::Space);
        let Some(Dialog::Filters(settings)) = &h.app.dialog else {
            panic!("Filter dialog closed during completion");
        };
        assert_eq!(settings.filter, "+test");
        assert!(h.app.settings.filter.is_empty());
        assert_eq!(h.file(), "(A) first +test\nsecond @home\n");
    }

    #[test]
    fn status_and_task_rows_paint_inside_the_window() {
        let mut h = Harness::new();
        let output = h.frame(Vec::new());
        fn text_shapes(shape: &egui::epaint::Shape, found: &mut Vec<(String, egui::Rect)>) {
            match shape {
                egui::epaint::Shape::Text(t) => {
                    found.push((t.galley.job.text.clone(), t.visual_bounding_rect()))
                }
                egui::epaint::Shape::Vec(shapes) => {
                    for s in shapes {
                        text_shapes(s, found);
                    }
                }
                _ => {}
            }
        }
        let mut texts = Vec::new();
        for shape in output.shapes {
            text_shapes(&shape.shape, &mut texts);
        }
        let status = texts
            .iter()
            .find(|(s, _)| s == "Tasks: 2 of 2")
            .expect("status is rendered");
        assert!(status.1.bottom() <= 1006.0, "{:?}", status.1);
        assert!(texts.iter().any(|(s, _)| s == "(A) first +test"));
    }

    #[test]
    fn keyboard_dialogs_focus_fields_and_replace_default_values() {
        let mut h = Harness::new();
        h.key(Key::I);
        h.text("C");
        h.key(Key::Enter);
        assert!(h.file().starts_with("(C) first +test"));
        h.key(Key::S);
        h.text("tomorrow");
        h.key(Key::Enter);
        assert!(
            h.file()
                .contains(&format!("due:{}", today() + Duration::days(1)))
        );
        h.key(Key::P);
        h.text("3");
        h.key(Key::Enter);
        assert!(
            h.file()
                .contains(&format!("due:{}", today() + Duration::days(4)))
        );
        h.key(Key::T);
        h.text(" appended");
        h.key(Key::Enter);
        assert!(h.file().contains(" appended"));
    }

    #[test]
    fn ctrl_enter_preference_blocks_plain_enter() {
        let mut h = Harness::new();
        h.app.settings.ctrl_enter = true;
        h.key(Key::N);
        h.text("draft");
        h.key(Key::Enter);
        assert!(!h.file().contains("draft"));
        // Modifiers are also part of the frame's current state, like native input.
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(506.0, 1006.0),
            )),
            modifiers: egui::Modifiers::CTRL,
            events: vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: Some(Key::Enter),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::CTRL,
            }],
            ..Default::default()
        };
        let _ = h.ctx.run(input, |ctx| h.app.render(ctx));
        assert!(h.file().contains("draft"));
    }
}

#[derive(Clone, Default)]
struct FilterCompletion {
    prefix: String,
    index: usize,
    dismissed: bool,
}
fn filter_editor(
    ui: &mut egui::Ui,
    text: &mut String,
    tasks: &[(usize, Task)],
    case_sensitive: bool,
    id: egui::Id,
) {
    let state_id = id.with("completion");
    let mut completion = ui.data_mut(|data| {
        data.get_temp::<FilterCompletion>(state_id)
            .unwrap_or_default()
    });
    let cursor = egui::TextEdit::load_state(ui.ctx(), id)
        .and_then(|s| s.cursor.char_range())
        .map_or(text.chars().count(), |range| range.primary.index);
    let before: String = text.chars().take(cursor).collect();
    let start = before
        .rfind(char::is_whitespace)
        .map_or(0, |n| n + before[n..].chars().next().unwrap().len_utf8());
    let prefix = &before[start..];
    if completion.prefix != prefix {
        completion.prefix = prefix.into();
        completion.index = 0;
        completion.dismissed = false;
    }
    let focused = ui.memory(|m| m.has_focus(id));
    let suggestions = if focused && !completion.dismissed && prefix.starts_with(['+', '@', '(']) {
        view::suggestions(tasks, prefix, case_sensitive)
    } else {
        Vec::new()
    };
    completion.index = completion.index.min(suggestions.len().saturating_sub(1));
    let mut accept = None;
    if !suggestions.is_empty() {
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, Key::ArrowDown) {
                completion.index = (completion.index + 1).min(suggestions.len() - 1);
            }
            if i.consume_key(egui::Modifiers::NONE, Key::ArrowUp) {
                completion.index = completion.index.saturating_sub(1);
            }
            if i.consume_key(egui::Modifiers::NONE, Key::Tab)
                || i.consume_key(egui::Modifiers::NONE, Key::Enter)
                || i.consume_key(egui::Modifiers::NONE, Key::Space)
            {
                i.events
                    .retain(|event| !matches!(event, egui::Event::Text(text) if text == " "));
                accept = Some(suggestions[completion.index].clone());
            }
            if i.consume_key(egui::Modifiers::NONE, Key::Escape) {
                completion.dismissed = true;
            }
        });
    }
    let output = egui::TextEdit::multiline(text)
        .id(id)
        .desired_width(f32::INFINITY)
        .desired_rows(3)
        .show(ui);
    if !suggestions.is_empty() && !completion.dismissed {
        egui::Area::new(id.with("suggestions"))
            .order(egui::Order::Foreground)
            .fixed_pos(output.response.rect.left_bottom())
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    for (n, suggestion) in suggestions.iter().take(8).enumerate() {
                        if ui
                            .selectable_label(n == completion.index, suggestion)
                            .clicked()
                        {
                            accept = Some(suggestion.clone());
                        }
                    }
                });
            });
    }
    if let Some(tag) = accept {
        let tail: String = text.chars().skip(cursor).collect();
        *text = format!("{}{tag}{tail}", &before[..start]);
        let mut state = output.state;
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(before[..start].chars().count() + tag.chars().count()),
            )));
        state.store(ui.ctx(), id);
        output.response.request_focus();
        completion.dismissed = true;
        completion.prefix = tag;
    }
    ui.data_mut(|data| data.insert_temp(state_id, completion));
}
