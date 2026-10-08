//! Printable table translated from the upstream GetPrintContents layout.
use crate::{task::Task, view::Row};

pub fn html(tasks: &[(usize, Task)], rows: &[Row]) -> String {
    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>todotxt.rs</title><style>body{font:12px Arial,sans-serif}table{border-collapse:collapse;width:100%}td,th{padding:3px;text-align:left;vertical-align:top}th{background:#ddd}.group{font-weight:bold;background:#eee}.done{color:#888}.project,.created{color:red}.context,.due{color:blue}.completed{color:green}.created,.completed,.due{font-style:italic;white-space:nowrap}@media print{button{display:none}thead{display:table-header-group}tr{break-inside:avoid}}</style><button onclick='window.print()'>Print</button><h2>todotxt.rs</h2><table><thead><tr><th></th><th>Done</th><th>Created</th><th>Due</th><th>Details</th></tr></thead><tbody>",
    );
    for row in rows {
        match row {
            Row::Header(name) => html.push_str(&format!(
                "<tr class='group'><td colspan='5'>{}</td></tr>",
                escape(name)
            )),
            Row::Task(id) => {
                if let Some((_, task)) = tasks.iter().find(|(n, _)| n == id) {
                    let marker = if task.completed {
                        "x".to_owned()
                    } else {
                        task.priority.map(|p| format!("({p})")).unwrap_or_default()
                    };
                    html.push_str(&format!("<tr class='{}'><td>{}</td><td class='completed'>{}</td><td class='created'>{}</td><td class='due'>{}</td><td>{}", if task.completed { "done" } else { "task" }, escape(&marker), escape(&task.completed_date), escape(&task.creation_date), escape(&task.due_date), escape(&task.body)));
                    for tag in &task.projects {
                        html.push_str(&format!(" <span class='project'>{}</span>", escape(tag)));
                    }
                    for tag in &task.contexts {
                        html.push_str(&format!(" <span class='context'>{}</span>", escape(tag)));
                    }
                    html.push_str("</td></tr>");
                }
            }
        }
    }
    html.push_str("</tbody></table>");
    html
}
