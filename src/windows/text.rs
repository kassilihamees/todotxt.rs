use super::*;
use todotxt_rs::task::parse_date;

pub(super) struct Run {
    pub text: Vec<u16>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub url: Option<String>,
}
pub(super) struct PaintRow {
    pub raw: String,
    pub runs: Vec<Run>,
    pub height: i32,
    pub width: i32,
    pub header: bool,
    pub completed: bool,
    pub color: u32,
    pub alternate: bool,
    pub line_height: i32,
}

unsafe fn extent(dc: HDC, s: &str) -> i32 {
    let text = wide(s);
    let mut size: SIZE = zeroed();
    GetTextExtentPoint32W(dc, text.as_ptr(), (text.len() - 1) as i32, &mut size);
    size.cx
}
pub(super) unsafe fn prepare(
    model: &Model,
    dc: HDC,
    width: i32,
    dpi: u32,
    fonts: [HFONT; 6],
) -> Paint {
    let mut metrics: TEXTMETRICW = zeroed();
    GetTextMetricsW(dc, &mut metrics);
    let line_height = metrics.tmHeight + metrics.tmExternalLeading;
    let grouped =
        model.settings.grouping && !matches!(model.settings.sort, Sort::File | Sort::Alphabetical);
    let mut alternate = 0;
    let mut rows = Vec::new();
    for row in &model.rows {
        if let Row::Header(label) = row {
            SelectObject(dc, fonts[2]);
            let measured = extent(dc, label);
            SelectObject(dc, fonts[1]);
            rows.push(PaintRow {
                raw: label.clone(),
                runs: vec![Run {
                    text: wide(label),
                    x: scale(dpi, 2),
                    y: scale(dpi, 10),
                    width: measured,
                    url: None,
                }],
                height: line_height + scale(dpi, 13),
                width: measured + scale(dpi, 6),
                header: true,
                completed: false,
                color: 0,
                alternate: false,
                line_height,
            });
            alternate = 0;
            continue;
        }
        let Row::Task(id) = row else {
            continue;
        };
        let Some((_, task)) = model.tasks.iter().find(|(n, _)| n == id) else {
            continue;
        };
        let indent = scale(dpi, if grouped { 12 } else { 4 });
        let limit = if model.settings.word_wrap {
            (width - scale(dpi, 4)).max(indent + scale(dpi, 20))
        } else {
            i32::MAX / 4
        };
        let (mut x, mut y) = (indent, scale(dpi, 2));
        let mut runs = Vec::new();
        let mut widest = indent;
        for word in task.raw.split_inclusive(char::is_whitespace) {
            let token = word.trim_end();
            let spaces = &word[token.len()..];
            let url = if ["http://", "https://", "ftp://", "www."]
                .iter()
                .any(|prefix| token.starts_with(prefix))
            {
                Some(if token.starts_with("www.") {
                    format!("http://{token}")
                } else {
                    token.to_owned()
                })
            } else {
                None
            };
            let measured = extent(dc, token);
            if x > indent && x + measured > limit {
                x = indent;
                y += line_height;
            }
            if measured > limit - indent {
                let mut segment = String::new();
                for c in token.chars() {
                    let mut next = segment.clone();
                    next.push(c);
                    if x + extent(dc, &next) > limit && !segment.is_empty() {
                        let width = extent(dc, &segment);
                        runs.push(Run {
                            text: wide(&segment),
                            x,
                            y,
                            width,
                            url: url.clone(),
                        });
                        widest = widest.max(x + width);
                        segment.clear();
                        x = indent;
                        y += line_height;
                    }
                    segment.push(c);
                }
                if !segment.is_empty() {
                    let width = extent(dc, &segment);
                    runs.push(Run {
                        text: wide(&segment),
                        x,
                        y,
                        width,
                        url,
                    });
                    x += width;
                }
            } else if !token.is_empty() {
                runs.push(Run {
                    text: wide(token),
                    x,
                    y,
                    width: measured,
                    url,
                });
                x += measured;
            }
            widest = widest.max(x);
            x += extent(dc, spaces);
        }
        let color = if task.completed {
            0x00bfbfbf
        } else if parse_date(&task.due_date).is_some_and(|d| d < model.date) {
            0x000000ff
        } else if parse_date(&task.due_date) == Some(model.date) {
            0x00008000
        } else {
            0
        };
        rows.push(PaintRow {
            raw: task.raw.clone(),
            runs,
            height: (y + line_height + scale(dpi, 2)).max(scale(dpi, 20)),
            width: widest + scale(dpi, 4),
            header: false,
            completed: task.completed,
            color,
            alternate: alternate % 2 == 1,
            line_height,
        });
        alternate += 1;
    }
    Paint {
        rows,
        font: fonts[1],
        bold: fonts[2],
        strike: fonts[3],
        link: fonts[4],
        strike_link: fonts[5],
        dpi,
    }
}

pub(super) unsafe fn draw(paint: &Paint, item: &DRAWITEMSTRUCT) {
    let Some(row) = paint.rows.get(item.itemID as usize) else {
        return;
    };
    let dc = item.hDC;
    let saved = SaveDC(dc);
    IntersectClipRect(
        dc,
        item.rcItem.left,
        item.rcItem.top,
        item.rcItem.right,
        item.rcItem.bottom,
    );
    let selected = !row.header && item.itemState & ODS_SELECTED != 0;
    let focused = GetFocus() == item.hwndItem;
    let bg = if selected {
        GetSysColor(if focused {
            COLOR_HIGHLIGHT
        } else {
            COLOR_BTNFACE
        })
    } else if row.alternate {
        0x00f8f8f8
    } else {
        0x00ffffff
    };
    let brush = CreateSolidBrush(bg);
    FillRect(dc, &item.rcItem, brush);
    DeleteObject(brush);
    SetBkMode(dc, TRANSPARENT as i32);
    for run in &row.runs {
        let font = if row.header {
            paint.bold
        } else if run.url.is_some() {
            if row.completed {
                paint.strike_link
            } else {
                paint.link
            }
        } else if row.completed {
            paint.strike
        } else {
            paint.font
        };
        SelectObject(dc, font);
        let color = if selected && focused {
            GetSysColor(COLOR_HIGHLIGHTTEXT)
        } else if run.url.is_some() {
            if row.completed {
                0x00ffbfbf
            } else {
                0x00ff0000
            }
        } else {
            row.color
        };
        SetTextColor(dc, color);
        TextOutW(
            dc,
            item.rcItem.left + run.x,
            item.rcItem.top + run.y,
            run.text.as_ptr(),
            (run.text.len() - 1) as i32,
        );
    }
    if !row.header && item.itemState & ODS_FOCUS != 0 {
        DrawFocusRect(dc, &item.rcItem);
    }
    RestoreDC(dc, saved);
}

pub(super) unsafe fn hit_link(hwnd: HWND, paint: &Paint, point: POINT) -> Option<String> {
    let item = SendMessageW(
        hwnd,
        LB_ITEMFROMPOINT,
        0,
        ((point.y as u32) << 16 | (point.x as u32 & 0xffff)) as isize,
    ) as u32;
    if item >> 16 != 0 {
        return None;
    }
    let id = (item & 0xffff) as usize;
    let row = paint.rows.get(id)?;
    let mut rect: RECT = zeroed();
    if SendMessageW(hwnd, LB_GETITEMRECT, id, &mut rect as *mut RECT as isize) == LB_ERR as isize {
        return None;
    }
    for run in &row.runs {
        if point.x >= rect.left + run.x
            && point.x < rect.left + run.x + run.width
            && point.y >= rect.top + run.y
            && point.y < rect.top + run.y + row.line_height
        {
            return run.url.clone();
        }
    }
    None
}
