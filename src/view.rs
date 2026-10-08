use crate::task::{Task, parse_date};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sort {
    File,
    #[default]
    Alphabetical,
    Completed,
    Context,
    Due,
    Created,
    Priority,
    Project,
}

impl Sort {
    pub const ALL: [Self; 8] = [
        Self::File,
        Self::Alphabetical,
        Self::Completed,
        Self::Context,
        Self::Due,
        Self::Created,
        Self::Priority,
        Self::Project,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "Order in file",
            Self::Alphabetical => "Alphabetical",
            Self::Completed => "Completed",
            Self::Context => "Context",
            Self::Due => "Due Date",
            Self::Created => "Creation Date",
            Self::Priority => "Priority",
            Self::Project => "Project",
        }
    }
    fn compare(self, a: &Task, b: &Task) -> Ordering {
        let priority = || {
            a.priority
                .unwrap_or('\u{ffff}')
                .cmp(&b.priority.unwrap_or('\u{ffff}'))
        };
        let due = || empty_last(&a.due_date).cmp(empty_last(&b.due_date));
        let created = || a.creation_date.cmp(&b.creation_date);
        let completed = || a.completed.cmp(&b.completed);
        match self {
            Self::File => Ordering::Equal,
            Self::Alphabetical => compare_text(&a.raw, &b.raw),
            Self::Completed => completed()
                .then_with(priority)
                .then_with(due)
                .then_with(created),
            Self::Context => compare_text(
                a.primary_context.as_deref().unwrap_or("zzz"),
                b.primary_context.as_deref().unwrap_or("zzz"),
            )
            .then_with(completed)
            .then_with(priority)
            .then_with(due)
            .then_with(created),
            Self::Project => compare_text(
                a.primary_project.as_deref().unwrap_or("zzz"),
                b.primary_project.as_deref().unwrap_or("zzz"),
            )
            .then_with(completed)
            .then_with(priority)
            .then_with(due)
            .then_with(created),
            Self::Due => due()
                .then_with(completed)
                .then_with(priority)
                .then_with(created),
            Self::Created => created()
                .then_with(completed)
                .then_with(priority)
                .then_with(due),
            Self::Priority => priority()
                .then_with(completed)
                .then_with(due)
                .then_with(created),
        }
    }
    fn groups(self, task: &Task) -> Vec<String> {
        let value = match self {
            Self::Priority => task.priority.map(|p| format!("({p})")).unwrap_or_default(),
            Self::Due => task.due_date.clone(),
            Self::Created => task.creation_date.clone(),
            Self::Completed => task.completed_date.clone(),
            Self::Project if !task.projects.is_empty() => return task.projects.clone(),
            Self::Context if !task.contexts.is_empty() => return task.contexts.clone(),
            _ => String::new(),
        };
        vec![if value.is_empty() {
            "n/a".to_owned()
        } else {
            value
        }]
    }
}

/// Match .NET Framework's current-culture string ordering on Windows.
#[cfg(windows)]
pub(crate) fn compare_text(a: &str, b: &str) -> Ordering {
    use windows_sys::Win32::Globalization::{
        CSTR_EQUAL, CSTR_GREATER_THAN, CSTR_LESS_THAN, CompareStringEx,
    };
    let a16: Vec<u16> = a.encode_utf16().collect();
    let b16: Vec<u16> = b.encode_utf16().collect();
    let result = unsafe {
        CompareStringEx(
            std::ptr::null(),
            0,
            a16.as_ptr(),
            a16.len() as i32,
            b16.as_ptr(),
            b16.len() as i32,
            std::ptr::null(),
            std::ptr::null(),
            0,
        )
    };
    match result {
        CSTR_LESS_THAN => Ordering::Less,
        CSTR_EQUAL => Ordering::Equal,
        CSTR_GREATER_THAN => Ordering::Greater,
        _ => a.to_lowercase().cmp(&b.to_lowercase()),
    }
}
#[cfg(not(windows))]
pub(crate) fn compare_text(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase())
}

fn empty_last(s: &str) -> &str {
    if s.is_empty() { "9999-99-99" } else { s }
}

#[derive(Debug, Clone)]
pub struct Filter<'a> {
    pub text: &'a str,
    pub case_sensitive: bool,
    pub hide_future: bool,
    pub show_hidden: bool,
}

impl Filter<'_> {
    pub fn matches(&self, task: &Task, today: NaiveDate) -> bool {
        if !self.show_hidden && task.raw.contains("h:1") {
            return false;
        }
        if self.hide_future && parse_date(&task.threshold_date).is_some_and(|date| date > today) {
            return false;
        }
        self.text
            .lines()
            .filter(|s| !s.is_empty())
            .all(|condition| {
                let (negative, text) = condition
                    .strip_prefix('-')
                    .map_or((false, condition), |s| (true, s));
                let hit = if text == "DONE" {
                    task.completed
                } else {
                    let date = parse_date(&task.due_date);
                    match text.to_ascii_lowercase().as_str() {
                        "due:today" => date == Some(today),
                        "due:future" => date.is_some_and(|d| d > today),
                        "due:past" => date.is_some_and(|d| d < today),
                        "due:active" => date.is_some_and(|d| d <= today),
                        _ if self.case_sensitive => task.raw.contains(text),
                        _ => task.raw.to_lowercase().contains(&text.to_lowercase()),
                    }
                };
                if negative { !hit } else { hit }
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Header(String),
    Task(usize),
}

pub fn rows(
    tasks: &[(usize, Task)],
    sort: Sort,
    filter: &Filter<'_>,
    grouping: bool,
    today: NaiveDate,
) -> Vec<Row> {
    let mut visible: Vec<_> = tasks
        .iter()
        .filter(|(_, task)| filter.matches(task, today))
        .collect();
    visible.sort_by(|a, b| sort.compare(&a.1, &b.1));
    if !grouping || matches!(sort, Sort::File | Sort::Alphabetical) {
        return visible.iter().map(|(id, _)| Row::Task(*id)).collect();
    }
    // WPF creates groups in first-appearance order, and a multi-tag task occurs in each group.
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (id, task) in visible {
        for label in sort.groups(task) {
            if let Some((_, ids)) = groups.iter_mut().find(|(s, _)| s == &label) {
                ids.push(*id);
            } else {
                groups.push((label, vec![*id]));
            }
        }
    }
    groups
        .into_iter()
        .flat_map(|(label, ids)| {
            std::iter::once(Row::Header(label)).chain(ids.into_iter().map(Row::Task))
        })
        .collect()
}

pub fn suggestions(tasks: &[(usize, Task)], prefix: &str, case_sensitive: bool) -> Vec<String> {
    let mut tags = BTreeSet::new();
    if prefix.starts_with('(') {
        tags.extend(('A'..='Z').map(|p| format!("({p})")));
    }
    for (_, task) in tasks {
        tags.extend(task.projects.iter().chain(task.contexts.iter()).cloned());
    }
    tags.into_iter()
        .filter(|s| {
            if case_sensitive {
                s.starts_with(prefix)
            } else {
                s.to_lowercase().starts_with(&prefix.to_lowercase())
            }
        })
        .collect()
}

pub fn counts(
    tasks: &[(usize, Task)],
    visible: &[Row],
    today: NaiveDate,
) -> BTreeMap<&'static str, usize> {
    let ids: BTreeSet<_> = visible
        .iter()
        .filter_map(|r| {
            if let Row::Task(id) = r {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    let tasks: Vec<_> = tasks
        .iter()
        .filter(|(id, _)| ids.contains(id))
        .map(|(_, t)| t)
        .collect();
    BTreeMap::from([
        ("visible", tasks.len()),
        ("incomplete", tasks.iter().filter(|t| !t.completed).count()),
        (
            "today",
            tasks
                .iter()
                .filter(|t| !t.completed && parse_date(&t.due_date) == Some(today))
                .count(),
        ),
        (
            "overdue",
            tasks
                .iter()
                .filter(|t| !t.completed && parse_date(&t.due_date).is_some_and(|d| d < today))
                .count(),
        ),
    ])
}
