use chrono::NaiveDate;
use std::{collections::BTreeMap, fs};
use todotxt_rs::{
    document::Document,
    task::{Task, resolve_date},
    view::{self, Filter, Row, Sort},
};

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 8).unwrap()
}
fn task(raw: &str) -> Task {
    Task::parse(raw, today())
}
fn unfiltered() -> Filter<'static> {
    Filter {
        text: "",
        case_sensitive: false,
        hide_future: false,
        show_hidden: true,
    }
}

#[test]
fn upstream_parser_examples() {
    for raw in [
        "(A) This is a test task +test @work",
        "(A) This is a test task @work +test",
        "(A) @work +test This is a test task",
    ] {
        let t = task(raw);
        assert_eq!(t.priority, Some('A'));
        assert_eq!(t.projects, ["+test"]);
        assert_eq!(t.contexts, ["@work"]);
        assert_eq!(t.body, "This is a test task");
        assert_eq!(t.raw, raw);
    }
    assert_eq!(task("Oh (A) task +work&home").priority, None);
    assert_eq!(task("Oh (A) task +work&home").projects, ["+work&home"]);
    assert!(!task("xyz task").completed);
    assert!(task("X @work +test This is a test task").completed);
    assert_eq!(
        task("X 2011-05-10 (A) @work +test task").completed_date,
        "2011-05-10"
    );
}

#[test]
fn metadata_is_sorted_but_primary_tag_is_first_in_text() {
    let t = task("(B) +z +a +z @z @a @z töö märkus");
    assert_eq!(t.projects, ["+a", "+z"]);
    assert_eq!(t.contexts, ["@a", "@z"]);
    assert_eq!(t.primary_project.as_deref(), Some("+z"));
    assert_eq!(t.primary_context.as_deref(), Some("@z"));
    assert_eq!(t.body, "töö märkus");
}

#[test]
fn dates_and_completion_preserve_placement() {
    let raw = "(A) 2011-05-07 +test töö due:2011-05-08 @work t:2011-05-06";
    let t = task(raw);
    assert_eq!(t.creation_date, "2011-05-07");
    assert_eq!(t.due_date, "2011-05-08");
    assert_eq!(t.threshold_date, "2011-05-06");
    let done = t.toggle(today());
    assert_eq!(
        done,
        "x 2026-10-08 2011-05-07 +test töö due:2011-05-08 @work t:2011-05-06"
    );
    assert_eq!(
        task(&done).toggle(today()),
        raw.strip_prefix("(A) ").unwrap()
    );
    assert_eq!(t.with_priority(Some('c')), raw.replacen("(A)", "(C)", 1));
    assert_eq!(
        t.shift_date("due", 1, today()),
        raw.replace("due:2011-05-08", "due:2011-05-09")
    );
}

#[test]
fn relative_dates_and_weekdays() {
    assert_eq!(
        task("work due:tOmORRow t:thu").raw,
        "work due:2026-10-09 t:2026-10-15"
    );
    assert_eq!(
        resolve_date("thursday", today()).unwrap().to_string(),
        "2026-10-15"
    );
    assert_eq!(
        resolve_date("fri", today()).unwrap().to_string(),
        "2026-10-09"
    );
    assert_eq!(resolve_date("2026-02-30", today()), None);
    assert_eq!(
        task("(A) task").with_creation_date(today()),
        "(A) 2026-10-08 task"
    );
    assert_eq!(
        task("task due:today").shift_date("t", 1, today()),
        "task due:2026-10-08 t:2026-10-09"
    );
}

#[test]
fn filter_conditions_and_date_boundaries() {
    let t = task("(A) töö +Home due:2026-10-08");
    for text in [
        "+home\n-DONE\ndue:active",
        "due:today",
        "-due:future",
        "-due:past",
        "-DONE",
    ] {
        assert!(
            Filter {
                text,
                ..unfiltered()
            }
            .matches(&t, today()),
            "{text}"
        );
    }
    for text in ["DONE", "due:past", "due:future", "-due:today", "+other"] {
        assert!(
            !Filter {
                text,
                ..unfiltered()
            }
            .matches(&t, today()),
            "{text}"
        );
    }
    assert!(
        !Filter {
            text: "+home",
            case_sensitive: true,
            ..unfiltered()
        }
        .matches(&t, today())
    );
    assert!(
        !Filter {
            hide_future: true,
            ..unfiltered()
        }
        .matches(&task("future t:2026-10-09"), today())
    );
    assert!(
        Filter {
            hide_future: true,
            ..unfiltered()
        }
        .matches(&task("today t:2026-10-08"), today())
    );
    assert!(
        !Filter {
            show_hidden: false,
            ..unfiltered()
        }
        .matches(&task("secret h:1"), today())
    );
}

#[test]
fn grouping_repeats_multitag_tasks_but_counts_them_once() {
    let tasks = vec![(0, task("(A) +b +a task")), (1, task("(B) +a second"))];
    let rows = view::rows(&tasks, Sort::Project, &unfiltered(), true, today());
    assert_eq!(
        rows,
        [
            Row::Header("+a".into()),
            Row::Task(1),
            Row::Task(0),
            Row::Header("+b".into()),
            Row::Task(0)
        ]
    );
    assert_eq!(view::counts(&tasks, &rows, today())["visible"], 2);
}

#[test]
fn every_sort_uses_upstream_order_and_tiebreaks() {
    let tasks = vec![
        (0, task("(B) 2020-01-02 Zebra +z @z due:2026-10-07")),
        (1, task("(A) 2020-01-01 Apple +a @a due:2026-10-08")),
        (2, task("x 2026-10-08 2020-01-03 done +z @z")),
        (3, task("bare")),
    ];
    for (sort, expected) in [
        (Sort::File, vec![0, 1, 2, 3]),
        (Sort::Alphabetical, vec![1, 0, 3, 2]),
        (Sort::Completed, vec![1, 0, 3, 2]),
        (Sort::Priority, vec![1, 0, 3, 2]),
        (Sort::Due, vec![0, 1, 3, 2]),
        (Sort::Created, vec![3, 1, 0, 2]),
        (Sort::Project, vec![1, 0, 2, 3]),
        (Sort::Context, vec![1, 0, 2, 3]),
    ] {
        let ids: Vec<_> = view::rows(&tasks, sort, &unfiltered(), false, today())
            .into_iter()
            .filter_map(|r| {
                if let Row::Task(id) = r {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(ids, expected, "{sort:?}");
    }
}

#[test]
fn fictional_demo_covers_groups_unicode_and_completion() {
    let text = include_str!("../fixtures/demo.todo");
    let tasks: Vec<_> = text
        .lines()
        .enumerate()
        .map(|(id, s)| (id, task(s)))
        .collect();
    assert_eq!(tasks.len(), 16);
    assert!(text.contains("café"));
    assert!(text.contains("日本語"));
    let rows = view::rows(&tasks, Sort::Priority, &unfiltered(), true, today());
    assert_eq!(rows.first(), Some(&Row::Header("(A)".into())));
    assert_eq!(rows.get(1), Some(&Row::Task(0)));
    assert_eq!(view::counts(&tasks, &rows, today())["incomplete"], 14);
}
#[test]
fn file_updates_preserve_utf8_bom_crlf_blank_lines_and_duplicate_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.txt");
    fs::write(&path, "\u{feff}  töö  \r\n\r\nsame\r\nsame\r\n").unwrap();
    let mut doc = Document::open(&path).unwrap();
    assert_eq!(doc.tasks(today(), false).len(), 3);
    doc.replace(&BTreeMap::from([(3, Some("changed".to_owned()))]))
        .unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "\u{feff}  töö  \r\n\r\nsame\r\nchanged\r\n"
    );
    doc.add("café +example1").unwrap();
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .ends_with("café +example1\r\n")
    );
}

#[test]
fn no_final_newline_and_empty_file_are_supported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.txt");
    fs::write(&path, "one\ntwo").unwrap();
    let mut doc = Document::open(&path).unwrap();
    doc.add("three").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "one\ntwo\nthree");
    doc.replace(&BTreeMap::from([(1, None)])).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "one\nthree");
    fs::write(&path, "").unwrap();
    let mut doc = Document::open(&path).unwrap();
    doc.add("first").unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), "first");
}

#[test]
fn external_change_refuses_all_writes_and_retains_in_memory_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.txt");
    fs::write(&path, "old\n").unwrap();
    let mut doc = Document::open(&path).unwrap();
    fs::write(&path, "external change\n").unwrap();
    assert!(
        doc.add("new")
            .unwrap_err()
            .to_string()
            .contains("changed outside")
    );
    assert!(doc.replace(&BTreeMap::from([(0, None)])).is_err());
    assert!(doc.archive(&dir.path().join("done.txt"), today()).is_err());
    assert_eq!(doc.lines, ["old"]);
    assert_eq!(fs::read_to_string(&path).unwrap(), "external change\n");
}

#[test]
fn invalid_input_does_not_change_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.txt");
    fs::write(&path, "valid").unwrap();
    let mut doc = Document::open(&path).unwrap();
    assert!(doc.add("bad\nline").is_err());
    assert!(doc.replace(&BTreeMap::from([(99, None)])).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"valid");
    fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(Document::open(path).is_err());
}

#[test]
fn archive_appends_without_losing_active_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("todo.txt");
    let archive = dir.path().join("done.txt");
    fs::write(&path, "active\nx 2026-10-08 töö +example1\n").unwrap();
    fs::write(&archive, "previous").unwrap();
    let mut doc = Document::open(&path).unwrap();
    assert!(doc.archive(&path, today()).is_err());
    assert_eq!(doc.archive(&archive, today()).unwrap(), 1);
    assert_eq!(fs::read_to_string(&path).unwrap(), "active\n");
    assert_eq!(
        fs::read_to_string(&archive).unwrap(),
        "previous\nx 2026-10-08 töö +example1\n"
    );
    assert_eq!(doc.archive(&archive, today()).unwrap(), 0);
}
