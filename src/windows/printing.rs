//! Native printer dialog and paginated GDI output; cancellation never starts a job.
use super::*;
use windows_sys::Win32::{Storage::Xps::*, UI::Controls::Dialogs::*};

struct Printer(PRINTDLGW);
impl Drop for Printer {
    fn drop(&mut self) {
        unsafe {
            if !self.0.hDC.is_null() {
                DeleteDC(self.0.hDC);
            }
            if !self.0.hDevMode.is_null() {
                GlobalFree(self.0.hDevMode);
            }
            if !self.0.hDevNames.is_null() {
                GlobalFree(self.0.hDevNames);
            }
        }
    }
}

pub(super) unsafe fn print(parent: HWND, model: &Model) -> io::Result<()> {
    let mut printer = Printer(PRINTDLGW {
        lStructSize: size_of::<PRINTDLGW>() as u32,
        hwndOwner: parent,
        Flags: PD_RETURNDC | PD_NOSELECTION | PD_NOPAGENUMS | PD_USEDEVMODECOPIESANDCOLLATE,
        nCopies: 1,
        ..zeroed()
    });
    if PrintDlgW(&mut printer.0) == 0 {
        let code = CommDlgExtendedError();
        return if code == 0 {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "Windows print dialog failed: {code:#x}"
            )))
        };
    }
    let dc = printer.0.hDC;
    if dc.is_null() {
        return Err(io::Error::other(
            "Printer did not provide a drawing context.",
        ));
    }
    let saved = SaveDC(dc);
    let dpi = GetDeviceCaps(dc, LOGPIXELSY as i32).max(96) as u32;
    let regular = font(dpi, 12.0, FW_NORMAL as i32, false, false);
    let bold = font(dpi, 12.0, FW_BOLD as i32, false, false);
    let document = DOCINFOW {
        cbSize: size_of::<DOCINFOW>() as i32,
        lpszDocName: w!("todotxt.rs"),
        ..zeroed()
    };
    let result = (|| {
        if StartDocW(dc, &document) <= 0 {
            return Err(io::Error::other("Could not start the printer job."));
        }
        let margin = scale(dpi, 36);
        let width = GetDeviceCaps(dc, HORZRES as i32) - margin * 2;
        let height = GetDeviceCaps(dc, VERTRES as i32) - margin * 2;
        if width < scale(dpi, 100) || height < scale(dpi, 100) {
            return Err(io::Error::other("Printer page is too small."));
        }
        let line = scale(dpi, 22);
        let columns = [
            0,
            width * 7 / 100,
            width * 24 / 100,
            width * 41 / 100,
            width * 58 / 100,
            width,
        ];
        let mut y = height;
        let mut page_started = false;
        for row in &model.rows {
            let (cells, heading) = match row {
                Row::Header(name) => (vec![name.clone()], true),
                Row::Task(id) => {
                    let Some((_, task)) = model.tasks.iter().find(|(n, _)| n == id) else {
                        continue;
                    };
                    let details = std::iter::once(task.body.as_str())
                        .chain(
                            task.projects
                                .iter()
                                .chain(task.contexts.iter())
                                .map(String::as_str),
                        )
                        .collect::<Vec<_>>()
                        .join(" ");
                    (
                        vec![
                            if task.completed {
                                "x".into()
                            } else {
                                task.priority.map(|p| format!("({p})")).unwrap_or_default()
                            },
                            task.completed_date.clone(),
                            task.creation_date.clone(),
                            task.due_date.clone(),
                            details,
                        ],
                        false,
                    )
                }
            };
            SelectObject(dc, if heading { bold } else { regular });
            let mut row_height = line;
            for (n, cell) in cells.iter().enumerate() {
                let mut rect = RECT {
                    left: 0,
                    top: 0,
                    right: if heading {
                        width
                    } else {
                        columns[n + 1] - columns[n] - scale(dpi, 6)
                    },
                    bottom: 0,
                };
                let text = wide(cell);
                DrawTextW(
                    dc,
                    text.as_ptr(),
                    (text.len() - 1) as i32,
                    &mut rect,
                    DT_CALCRECT | DT_WORDBREAK | DT_NOPREFIX,
                );
                row_height = row_height.max(rect.bottom + scale(dpi, 6));
            }
            let mut offset = 0;
            while offset < row_height {
                if y + (row_height - offset).min(line) > height {
                    if page_started && EndPage(dc) <= 0 {
                        return Err(io::Error::other("Printer could not finish a page."));
                    }
                    if StartPage(dc) <= 0 {
                        return Err(io::Error::other("Printer could not start a page."));
                    }
                    page_started = true;
                    SetBkMode(dc, TRANSPARENT as i32);
                    SelectObject(dc, bold);
                    SetTextColor(dc, 0);
                    TextOutW(dc, margin, margin, w!("todotxt.rs"), 10);
                    for (n, title) in ["", "Done", "Created", "Due", "Details"].iter().enumerate() {
                        let text = wide(title);
                        TextOutW(
                            dc,
                            margin + columns[n],
                            margin + line,
                            text.as_ptr(),
                            (text.len() - 1) as i32,
                        );
                    }
                    y = line * 2;
                }
                let remaining = height - y;
                let portion = (row_height - offset).min(remaining);
                let clip = SaveDC(dc);
                IntersectClipRect(dc, margin, margin + y, margin + width, margin + y + portion);
                SelectObject(dc, if heading { bold } else { regular });
                for (n, cell) in cells.iter().enumerate() {
                    SetTextColor(
                        dc,
                        if heading {
                            0
                        } else {
                            [0, 0x008000, 0x0000ff, 0xff0000, 0][n]
                        },
                    );
                    let mut rect = RECT {
                        left: margin + if heading { 0 } else { columns[n] },
                        top: margin + y - offset,
                        right: margin
                            + if heading {
                                width
                            } else {
                                columns[n + 1] - scale(dpi, 6)
                            },
                        bottom: margin + y - offset + row_height,
                    };
                    let text = wide(cell);
                    DrawTextW(
                        dc,
                        text.as_ptr(),
                        (text.len() - 1) as i32,
                        &mut rect,
                        DT_WORDBREAK | DT_NOPREFIX,
                    );
                }
                RestoreDC(dc, clip);
                y += portion;
                offset += portion;
            }
        }
        if page_started && EndPage(dc) <= 0 {
            return Err(io::Error::other("Printer could not finish the last page."));
        }
        if EndDoc(dc) <= 0 {
            return Err(io::Error::other("Printer could not complete the job."));
        }
        Ok(())
    })();
    if result.is_err() {
        AbortDoc(dc);
    }
    RestoreDC(dc, saved);
    DeleteObject(regular);
    DeleteObject(bold);
    result
}
