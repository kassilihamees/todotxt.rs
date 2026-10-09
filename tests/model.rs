use std::fs;
use todotxt_rs::{model::Model, settings::Settings};

#[test]
fn delayed_empty_reload_preserves_completed_tasks_selection_and_draft() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.txt");
    fs::write(&path, "First fictional task\nSecond fictional task\n").unwrap();
    let mut model = Model::new(Some(path.clone()), Some(dir.path().join("config")), false);
    model.settings.auto_refresh = true;
    let date = model.date;
    model.modify(|task| Some(task.toggle(date))).unwrap();
    let saved = fs::read(&path).unwrap();
    let rows = model.rows.clone();
    let selection = model.selected.clone();
    model.draft = "Retained fictional draft".into();
    fs::write(&path, []).unwrap();
    assert!(model.reload().is_err());
    assert_eq!(model.tasks.len(), 2);
    assert!(model.tasks.iter().any(|(_, task)| task.completed));
    assert_eq!(model.rows, rows);
    assert_eq!(model.selected, selection);
    assert_eq!(model.draft, "Retained fictional draft");
    assert!(!model.document.as_ref().unwrap().can_auto_reload());
    fs::write(&path, saved).unwrap();
    model.reload().unwrap();
    assert!(model.document.as_ref().unwrap().can_auto_reload());
}

#[test]
fn conflicting_edit_retains_draft_and_reload_allows_explicit_retry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.txt");
    fs::write(&path, "original\r\n").unwrap();
    let mut model = Model::new(Some(path.clone()), Some(dir.path().join("config")), false);
    assert!(model.begin_edit(false));
    model.draft = "my edited task".into();
    fs::write(&path, "externally edited\r\n").unwrap();
    assert!(model.submit().is_err());
    assert_eq!(model.draft, "my edited task");
    assert_eq!(model.editing, Some(0));
    assert_eq!(fs::read_to_string(&path).unwrap(), "externally edited\r\n");
    model.reload().unwrap();
    assert_eq!(model.draft, "my edited task");
    assert_eq!(model.editing, None);
    model.submit().unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "externally edited\r\nmy edited task\r\n"
    );
}

#[test]
fn duplicate_selection_updates_one_physical_task() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.txt");
    fs::write(&path, "same\nsame\n").unwrap();
    let mut model = Model::new(Some(path.clone()), Some(dir.path().join("config")), false);
    model.selected = [1].into();
    let date = model.date;
    model.modify(|t| Some(t.toggle(date))).unwrap();
    let actual = fs::read_to_string(path).unwrap();
    assert_eq!(actual.lines().next(), Some("same"));
    assert!(actual.lines().nth(1).unwrap().starts_with("x "));
}

#[test]
fn native_and_portable_preferences_use_the_same_schema() {
    let dir = tempfile::tempdir().unwrap();
    let mut model = Model::new(None, Some(dir.path().into()), true);
    model.settings.ctrl_enter = true;
    model.settings.word_wrap = false;
    model.settings.presets[2] = "+work\n-DONE".into();
    model.save().unwrap();
    let restored = Settings::read(dir.path()).unwrap();
    assert!(restored.ctrl_enter);
    assert!(!restored.word_wrap);
    assert_eq!(restored.presets[2], "+work\n-DONE");
    model.preset(3).unwrap();
    assert_eq!(model.settings.filter, "+work\n-DONE");
}

#[test]
fn old_preferences_keep_safe_defaults_and_new_preferences_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("settings.json"),
        r#"{"font_size":12,"word_wrap":true}"#,
    )
    .unwrap();
    let mut settings = Settings::read(dir.path()).unwrap();
    assert!(!settings.minimize_to_tray);
    assert!(!settings.minimize_on_close);
    assert!(!settings.debug_logging);
    assert_eq!(settings.font_family, "Segoe UI");
    settings.font_family = "Consolas".into();
    settings.font_size = 18.0;
    settings.font_weight = 700;
    settings.font_italic = true;
    settings.font_underline = true;
    settings.font_strike = true;
    settings.font_color = 0x123456;
    settings.minimize_to_tray = true;
    settings.minimize_on_close = true;
    settings.debug_logging = true;
    settings.save(dir.path()).unwrap();
    let restored = Settings::read(dir.path()).unwrap();
    assert_eq!(
        serde_json::to_value(&settings).unwrap(),
        serde_json::to_value(&restored).unwrap()
    );
}

#[test]
fn debug_logging_is_optional_and_does_not_include_task_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut model = Model::new(None, Some(dir.path().into()), true);
    model.draft = "Fictional private draft marker".into();
    model.settings.filter = "Fictional private filter marker".into();
    model.debug_event("command 102");
    assert!(!dir.path().join("error.log").exists());
    model.settings.debug_logging = true;
    model.debug_event("command 102");
    let log = fs::read_to_string(dir.path().join("error.log")).unwrap();
    assert!(log.contains("DEBUG command 102"));
    assert!(!log.contains(&model.draft));
    assert!(!log.contains(&model.settings.filter));
    assert!(!log.contains("todo.txt"));
}

#[test]
fn printable_table_has_dates_groups_escaped_details_and_one_copy_of_each_tag() {
    let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 8).unwrap();
    let task = todotxt_rs::task::Task::parse(
        "(A) 2026-01-01 Review <fictional> & sample +demo @desk due:2026-10-08",
        date,
    );
    let tasks = vec![(0, task)];
    let rows = vec![
        todotxt_rs::view::Row::Header("<group>".into()),
        todotxt_rs::view::Row::Task(0),
    ];
    let html = todotxt_rs::printing::html(&tasks, &rows);
    assert!(html.contains("<th>Done</th><th>Created</th><th>Due</th><th>Details</th>"));
    assert!(html.contains("&lt;group&gt;"));
    assert!(html.contains("&lt;fictional&gt; &amp; sample"));
    assert!(html.contains("class='created'>2026-01-01"));
    assert!(html.contains("class='due'>2026-10-08"));
    assert_eq!(html.matches("+demo").count(), 1);
    assert_eq!(html.matches("@desk").count(), 1);
    assert!(!html.contains("<fictional>"));
}
