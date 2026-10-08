use chrono::{Datelike, Duration, NaiveDate};
use regex::Regex;
use std::sync::OnceLock;

fn pattern(cell: &'static OnceLock<Regex>, expression: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(expression).expect("constant regex"))
}

fn completion() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    pattern(&RE, r"(?i)^x\s(?:\d{4}-\d{2}-\d{2})?")
}

fn priority() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    pattern(&RE, r"(?i)^\(([a-z])\)\s")
}

fn dates() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    pattern(&RE, r"\d{4}-\d{2}-\d{2}")
}

fn relative() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    pattern(
        &RE,
        r"(?i)\b(due|t):(today|tomorrow|mon(?:day)?|tue(?:sday)?|wed(?:nesday)?|thu(?:rsday)?|fri(?:day)?|sat(?:urday)?|sun(?:day)?)\b",
    )
}

pub fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

pub fn resolve_date(text: &str, today: NaiveDate) -> Option<NaiveDate> {
    if let Some(date) = parse_date(text) {
        return Some(date);
    }
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "today" => Some(today),
        "tomorrow" => today.checked_add_signed(Duration::days(1)),
        _ => {
            let day = match lower.as_str() {
                "mon" | "monday" => 0,
                "tue" | "tuesday" => 1,
                "wed" | "wednesday" => 2,
                "thu" | "thursday" => 3,
                "fri" | "friday" => 4,
                "sat" | "saturday" => 5,
                "sun" | "sunday" => 6,
                _ => return None,
            };
            let offset = (day + 7 - today.weekday().num_days_from_monday()) % 7;
            today.checked_add_signed(Duration::days(if offset == 0 { 7 } else { offset }.into()))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub raw: String,
    pub completed: bool,
    pub completed_date: String,
    pub priority: Option<char>,
    pub creation_date: String,
    pub due_date: String,
    pub threshold_date: String,
    pub projects: Vec<String>,
    pub contexts: Vec<String>,
    pub primary_project: Option<String>,
    pub primary_context: Option<String>,
    pub body: String,
}

impl Task {
    pub fn parse(raw: &str, today: NaiveDate) -> Self {
        let raw = raw.replace(['\r', '\n'], "");
        let raw = relative()
            .replace_all(&raw, |caps: &regex::Captures<'_>| {
                format!(
                    "{}:{}",
                    caps[1].to_ascii_lowercase(),
                    resolve_date(&caps[2], today).expect("relative date")
                )
            })
            .into_owned();
        let mut rest = raw.clone();
        let completed = completion().is_match(&rest);
        let completed_date = completion()
            .find(&rest)
            .and_then(|m| dates().find(m.as_str()))
            .map_or_else(String::new, |m| m.as_str().to_owned());
        rest = completion().replace(&rest, "").into_owned();
        let prio = priority().captures(&rest).and_then(|c| c[1].chars().next());
        rest = priority().replace(&rest, "").into_owned();
        let mut take_date = |key: &str| {
            let re = Regex::new(&format!(r"{key}:(\d{{4}}-\d{{2}}-\d{{2}})")).expect("date key");
            let date = re
                .captures(&rest)
                .map_or_else(String::new, |c| c[1].to_owned());
            rest = re.replace_all(&rest, "").into_owned();
            date
        };
        let due_date = take_date("due");
        let threshold_date = take_date("t");
        let creation_date = dates()
            .find(&rest)
            .map_or_else(String::new, |m| m.as_str().to_owned());
        rest = dates().replace_all(&rest, "").into_owned();
        let mut projects = Vec::new();
        let mut contexts = Vec::new();
        let mut primary_project = None;
        let mut primary_context = None;
        static TAGS: OnceLock<Regex> = OnceLock::new();
        let tags = pattern(&TAGS, r"(?:^|\s)([+@][^\s]+)");
        for tag in rest
            .split_whitespace()
            .filter(|s| s.starts_with(['+', '@']) && s.len() > 1)
        {
            let tag = tag.to_owned();
            if tag.starts_with('+') {
                primary_project.get_or_insert_with(|| tag.clone());
                projects.push(tag);
            } else {
                primary_context.get_or_insert_with(|| tag.clone());
                contexts.push(tag);
            }
        }
        projects.sort_by(|a, b| crate::view::compare_text(a, b));
        projects.dedup();
        contexts.sort_by(|a, b| crate::view::compare_text(a, b));
        contexts.dedup();
        if primary_project.is_none() {
            primary_project = projects.first().cloned();
        }
        if primary_context.is_none() {
            primary_context = contexts.first().cloned();
        }
        let mut body = rest;
        // Repeated passes catch adjacent tags without collapsing the body's whitespace.
        while tags.is_match(&body) {
            body = tags.replace_all(&body, "").into_owned();
        }
        Self {
            raw,
            completed,
            completed_date,
            priority: prio,
            creation_date,
            due_date,
            threshold_date,
            projects,
            contexts,
            primary_project,
            primary_context,
            body: body.trim().to_owned(),
        }
    }

    pub fn toggle(&self, today: NaiveDate) -> String {
        if self.completed {
            completion().replace(&self.raw, "").trim().to_owned()
        } else {
            format!("x {today} {}", priority().replace(&self.raw, ""))
        }
    }

    pub fn with_priority(&self, value: Option<char>) -> String {
        let raw = priority().replace(&self.raw, "");
        match value {
            Some(p) => format!("({}) {}", p.to_ascii_uppercase(), raw.trim()),
            None => raw.trim().to_owned(),
        }
    }

    pub fn shifted_priority(&self, delta: i8) -> String {
        let p = self
            .priority
            .map(|p| p.to_ascii_uppercase() as i16 + delta as i16)
            .unwrap_or('A' as i16);
        if ('A' as i16..='Z' as i16).contains(&p) {
            self.with_priority(Some(p as u8 as char))
        } else {
            self.raw.clone()
        }
    }

    pub fn with_date(&self, key: &str, date: Option<NaiveDate>) -> String {
        assert!(key == "due" || key == "t");
        let re = Regex::new(&format!(r"{key}:\d{{4}}-\d{{2}}-\d{{2}}")).expect("date key");
        match date {
            Some(date) if re.is_match(&self.raw) => re
                .replace_all(&self.raw, format!("{key}:{date}"))
                .into_owned(),
            Some(date) => format!("{} {key}:{date}", self.raw.trim_end()),
            None => re.replace_all(&self.raw, "").trim().to_owned(),
        }
    }

    pub fn shift_date(&self, key: &str, days: i64, today: NaiveDate) -> String {
        let text = if key == "due" {
            &self.due_date
        } else {
            &self.threshold_date
        };
        let date = parse_date(text)
            .unwrap_or(today)
            .checked_add_signed(Duration::days(days));
        self.with_date(key, date)
    }

    pub fn with_creation_date(&self, today: NaiveDate) -> String {
        if !self.creation_date.is_empty() {
            return self.raw.clone();
        }
        if let Some(prio) = self.priority {
            let raw = priority().replace(&self.raw, "");
            format!("({prio}) {today} {raw}")
        } else {
            format!("{today} {}", self.raw)
        }
    }
}
