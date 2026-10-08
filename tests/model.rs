use std::fs;
use todotxt_rs::{model::Model, settings::Settings};

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
