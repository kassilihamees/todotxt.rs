//! Windows UI boundary. Handles stay on the message-loop thread. Mutable application
//! state and immutable drawing snapshots have separate RefCells for Win32 reentry.
#![allow(unsafe_op_in_unsafe_fn)]
mod dialogs;
mod printing;
mod shell;
mod text;

use chrono::{Duration, Local};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    fs, io,
    mem::{size_of, zeroed},
    path::PathBuf,
    ptr::{null, null_mut},
};
use todotxt_rs::{
    model::Model,
    task::{Task, resolve_date},
    view::{self, Row, Sort},
};
use windows_sys::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{DataExchange::*, LibraryLoader::*, Memory::*},
        UI::{
            Controls::*,
            HiDpi::*,
            Input::KeyboardAndMouse::*,
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
    w,
};

const OPEN: u16 = 100;
const NEW_FILE: u16 = 101;
const NEW: u16 = 102;
const EDIT: u16 = 103;
const DUPLICATE: u16 = 104;
const APPEND: u16 = 105;
const DELETE: u16 = 106;
const TOGGLE: u16 = 107;
const ARCHIVE: u16 = 108;
const RELOAD: u16 = 109;
const OPTIONS: u16 = 110;
const FILTER: u16 = 111;
const HELP: u16 = 112;
const PRINT: u16 = 113;
const COPY: u16 = 114;
const PASTE: u16 = 115;
const CUT: u16 = 116;
const PRIORITY: u16 = 117;
const PRIORITY_UP: u16 = 118;
const PRIORITY_DOWN: u16 = 119;
const PRIORITY_CLEAR: u16 = 120;
const DUE: u16 = 121;
const THRESHOLD: u16 = 122;
const POSTPONE: u16 = 123;
const DEFER: u16 = 124;
const DUE_UP: u16 = 125;
const DUE_DOWN: u16 = 126;
const DUE_CLEAR: u16 = 127;
const THRESHOLD_UP: u16 = 128;
const THRESHOLD_DOWN: u16 = 129;
const THRESHOLD_CLEAR: u16 = 130;
const HIDE_FUTURE: u16 = 131;
const SHOW_HIDDEN: u16 = 132;
const EXIT: u16 = 133;
const CALENDAR: u16 = 134;
const LOG: u16 = 135;
const PRINT_PREVIEW: u16 = 136;
const EDIT_ID: usize = 10;
const LIST_ID: usize = 11;
const SUGGEST_ID: usize = 12;
const SORT_BASE: u16 = 300;
const PRESET_BASE: u16 = 400;
pub(super) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub(super) unsafe fn window_text(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buf = vec![0; len + 1];
    let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}
pub(super) unsafe fn set_text(hwnd: HWND, text: &str) {
    SetWindowTextW(hwnd, wide(text).as_ptr());
}
pub(super) unsafe fn child(
    parent: HWND,
    class: *const u16,
    title: &str,
    style: u32,
    id: usize,
    font: HFONT,
) -> HWND {
    let hwnd = CreateWindowExW(
        0,
        class,
        wide(title).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        0,
        0,
        10,
        10,
        parent,
        id as HMENU,
        GetModuleHandleW(null()),
        null(),
    );
    SendMessageW(hwnd, WM_SETFONT, font as usize, 1);
    hwnd
}
pub(super) fn scale(dpi: u32, logical: i32) -> i32 {
    (logical * dpi as i32 + 48) / 96
}
unsafe fn font(dpi: u32, pixels: f32, weight: i32, strike: bool, underline: bool) -> HFONT {
    let mut lf: LOGFONTW = zeroed();
    lf.lfHeight = -((pixels * dpi as f32 / 96.0).round() as i32);
    lf.lfWeight = weight;
    lf.lfStrikeOut = strike as u8;
    lf.lfUnderline = underline as u8;
    lf.lfCharSet = DEFAULT_CHARSET;
    lf.lfQuality = CLEARTYPE_QUALITY;
    let name = wide("Segoe UI");
    lf.lfFaceName[..name.len()].copy_from_slice(&name);
    CreateFontIndirectW(&lf)
}
unsafe fn task_font(
    dpi: u32,
    settings: &todotxt_rs::settings::Settings,
    bold: bool,
    strike: bool,
    link: bool,
) -> HFONT {
    let mut lf: LOGFONTW = zeroed();
    lf.lfHeight = -(settings.font_size * dpi as f32 / 96.0).round() as i32;
    lf.lfWeight = if bold {
        settings.font_weight.max(700)
    } else {
        settings.font_weight
    };
    lf.lfItalic = settings.font_italic as u8;
    lf.lfStrikeOut = (strike || settings.font_strike) as u8;
    lf.lfUnderline = (link || settings.font_underline) as u8;
    lf.lfCharSet = DEFAULT_CHARSET;
    lf.lfQuality = CLEARTYPE_QUALITY;
    let name: Vec<u16> = settings.font_family.encode_utf16().take(31).collect();
    lf.lfFaceName[..name.len()].copy_from_slice(&name);
    CreateFontIndirectW(&lf)
}
#[derive(Default)]
struct Paint {
    rows: Vec<text::PaintRow>,
    font: HFONT,
    bold: HFONT,
    strike: HFONT,
    link: HFONT,
    strike_link: HFONT,
    dpi: u32,
}
struct Native {
    model: Model,
    hwnd: HWND,
    editor: HWND,
    list: HWND,
    status: HWND,
    suggestions: HWND,
    dpi: u32,
    fonts: [HFONT; 6],
    suggestion_tags: Vec<String>,
    suggestion_range: (usize, usize),
    suggestion_index: usize,
    suggestions_dismissed: bool,
    last_width: i32,
    suppress_edit: bool,
    shell: shell::Integration,
    exiting: bool,
    calendar: bool,
}
struct WindowData {
    app: RefCell<Native>,
    paint: RefCell<Paint>,
}

pub fn run(path: Option<PathBuf>, config: Option<PathBuf>, demo: bool) -> io::Result<()> {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let data = create(Model::new(path, config, demo))?;
        let hwnd = data.app.borrow().hwnd;
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        if let Some(error) = data.app.borrow_mut().model.startup_error.take() {
            error_box(hwnd, &error);
        }
        let mut msg: MSG = zeroed();
        loop {
            let result = GetMessageW(&mut msg, null_mut(), 0, 0);
            if result == 0 {
                break;
            }
            if result == -1 {
                return Err(io::Error::last_os_error());
            }
            let key = if msg.wParam == VK_SHIFT as usize && (msg.lParam >> 16) & 0xff == 0x36 {
                VK_RSHIFT
            } else {
                msg.wParam as u16
            };
            let handled = if msg.message == WM_KEYDOWN {
                data.app.borrow_mut().key(msg.hwnd, key, &data)
            } else {
                false
            };
            if !handled {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        Ok(())
    }
}
unsafe fn create(model: Model) -> io::Result<Box<WindowData>> {
    let instance = GetModuleHandleW(null());
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hIcon: LoadIconW(instance, std::ptr::without_provenance::<u16>(1)),
        hIconSm: LoadIconW(instance, std::ptr::without_provenance::<u16>(1)),
        hbrBackground: (COLOR_WINDOW + 1) as HBRUSH,
        lpszClassName: w!("TodoTxtRustNative"),
        ..zeroed()
    };
    if RegisterClassExW(&class) == 0 && GetLastError() != ERROR_CLASS_ALREADY_EXISTS {
        return Err(io::Error::last_os_error());
    }
    InitCommonControlsEx(&INITCOMMONCONTROLSEX {
        dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_BAR_CLASSES | ICC_TAB_CLASSES | ICC_STANDARD_CLASSES,
    });
    let data = Box::new(WindowData {
        app: RefCell::new(Native {
            model,
            hwnd: null_mut(),
            editor: null_mut(),
            list: null_mut(),
            status: null_mut(),
            suggestions: null_mut(),
            dpi: 96,
            fonts: [null_mut(); 6],
            suggestion_tags: Vec::new(),
            suggestion_range: (0, 0),
            suggestion_index: 0,
            suggestions_dismissed: false,
            last_width: 0,
            suppress_edit: false,
            shell: shell::Integration::default(),
            exiting: false,
            calendar: false,
        }),
        paint: RefCell::new(Paint::default()),
    });
    let saved: Option<[i32; 4]> = fs::read(
        data.app
            .borrow()
            .model
            .config_dir
            .join("native-window.json"),
    )
    .ok()
    .and_then(|b| serde_json::from_slice(&b).ok());
    let (x, y, width, height) = saved
        .filter(|r| r[2] >= 350 && r[3] >= 200)
        .map(|r| (r[0], r[1], r[2], r[3]))
        .unwrap_or((CW_USEDEFAULT, CW_USEDEFAULT, 521, 1047));
    let hwnd = CreateWindowExW(
        0,
        class.lpszClassName,
        w!("todotxt.rs"),
        WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
        x,
        y,
        width,
        height,
        null_mut(),
        make_menu(),
        instance,
        (&*data as *const WindowData).cast(),
    );
    if hwnd.is_null() {
        return Err(io::Error::last_os_error());
    }
    {
        let mut app = data.app.borrow_mut();
        app.hwnd = hwnd;
        app.dpi = GetDpiForWindow(hwnd).max(96);
        app.create_fonts();
        app.editor = child(
            hwnd,
            w!("EDIT"),
            "",
            WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            EDIT_ID,
            app.fonts[0],
        );
        app.list = child(
            hwnd,
            w!("LISTBOX"),
            "",
            WS_TABSTOP
                | WS_VSCROLL
                | WS_HSCROLL
                | LBS_NOTIFY as u32
                | LBS_EXTENDEDSEL as u32
                | LBS_OWNERDRAWVARIABLE as u32
                | LBS_HASSTRINGS as u32
                | LBS_NOINTEGRALHEIGHT as u32,
            LIST_ID,
            app.fonts[1],
        );
        SetWindowSubclass(
            app.list,
            Some(list_proc),
            1,
            (&*data as *const WindowData) as usize,
        );
        app.status = child(hwnd, STATUSCLASSNAMEW, "", SBARS_SIZEGRIP, 13, app.fonts[0]);
        app.suggestions = child(
            hwnd,
            w!("LISTBOX"),
            "",
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32 | LBS_NOINTEGRALHEIGHT as u32,
            SUGGEST_ID,
            app.fonts[0],
        );
        ShowWindow(app.suggestions, SW_HIDE);
        app.layout(&data);
        app.rebuild(&data);
        SetFocus(app.list);
        SetTimer(hwnd, 1, 1000, None);
        let tray_enabled = app.model.settings.minimize_to_tray;
        if let Err(error) = app.shell.configure(hwnd, tray_enabled) {
            app.model.startup_error = Some(error.to_string());
        }
    }
    Ok(data)
}
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == WM_NCCREATE {
        let create = &*(lparam as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowData;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    let data = &*ptr;
    match message {
        WM_DESTROY => {
            PostQuitMessage(0);
            return 0;
        }
        WM_MEASUREITEM => {
            let item = &mut *(lparam as *mut MEASUREITEMSTRUCT);
            item.itemHeight = scale(data.paint.borrow().dpi.max(96), 20) as u32;
            return 1;
        }
        WM_DRAWITEM => {
            if let Ok(paint) = data.paint.try_borrow() {
                text::draw(&paint, &*(lparam as *const DRAWITEMSTRUCT));
            }
            return 1;
        }
        WM_CTLCOLORLISTBOX | WM_CTLCOLOREDIT => {
            SetBkColor(wparam as HDC, 0x00ffffff);
            SetTextColor(wparam as HDC, 0);
            return GetStockObject(WHITE_BRUSH) as isize;
        }
        WM_GETMINMAXINFO => {
            let info = &mut *(lparam as *mut MINMAXINFO);
            info.ptMinTrackSize = POINT { x: 350, y: 200 };
            return 0;
        }
        _ => {}
    }
    let Ok(mut app) = data.app.try_borrow_mut() else {
        if message == WM_SIZE {
            PostMessageW(hwnd, shell::RELAYOUT, 0, 0);
        }
        return DefWindowProcW(hwnd, message, wparam, lparam);
    };
    if message == app.shell.explorer_restart && message != 0 {
        app.shell.recreate_icon();
        return 0;
    }
    match message {
        shell::RELAYOUT => {
            if IsIconic(hwnd) == 0 {
                app.layout(data);
                app.rebuild(data);
            }
            return 0;
        }
        shell::TRAY_MESSAGE => {
            // Version 4 packs the icon ID into the upper half; older callbacks
            // carry just the event. Decode both, including keyboard context menus.
            match lparam as u32 & 0xffff {
                WM_LBUTTONDBLCLK => app.shell.toggle(),
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    // TrackPopupMenu runs a nested message loop. Release application
                    // state so menu notifications and queued commands can be handled.
                    drop(app);
                    shell::popup(hwnd);
                }
                _ => {}
            }
            return 0;
        }
        WM_HOTKEY if wparam == shell::HOTKEY_ID as usize => {
            app.shell.toggle();
            return 0;
        }
        WM_SIZE => {
            if wparam == SIZE_MINIMIZED as usize {
                if app.shell.active {
                    ShowWindow(hwnd, SW_HIDE);
                }
                return 0;
            }
            if !app.editor.is_null() {
                app.layout(data);
                let mut rect: RECT = zeroed();
                GetClientRect(app.list, &mut rect);
                if rect.right != app.last_width {
                    app.rebuild(data);
                } else {
                    app.status_text();
                }
            }
            return 0;
        }
        WM_DPICHANGED => {
            app.dpi = (wparam as u32 & 0xffff).max(96);
            app.create_fonts();
            let rect = &*(lparam as *const RECT);
            SetWindowPos(
                hwnd,
                null_mut(),
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            app.layout(data);
            app.rebuild(data);
            return 0;
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as u16;
            let notify = ((wparam >> 16) & 0xffff) as u32;
            if lparam != 0 {
                if id as usize == EDIT_ID && notify == EN_CHANGE && !app.suppress_edit {
                    app.model.draft = window_text(app.editor);
                    app.suggestions_dismissed = false;
                    app.complete();
                }
                if id as usize == LIST_ID && notify == LBN_SELCHANGE {
                    app.sync_selection();
                }
                if id as usize == LIST_ID && notify == LBN_DBLCLK {
                    app.execute(EDIT, data);
                }
                if id as usize == SUGGEST_ID && (notify == LBN_SELCHANGE || notify == LBN_DBLCLK) {
                    app.suggestion_index =
                        SendMessageW(app.suggestions, LB_GETCURSEL, 0, 0).max(0) as usize;
                    app.accept_completion();
                }
            } else {
                app.execute(id, data);
            }
            return 0;
        }
        WM_INITMENUPOPUP => {
            app.update_menu();
            return 0;
        }
        WM_TIMER => {
            if app.model.date != Local::now().date_naive() {
                app.model.refresh();
                app.rebuild(data);
            }
            if app.model.settings.auto_refresh
                && app.model.editing.is_none()
                && window_text(app.editor).is_empty()
                && app
                    .model
                    .document
                    .as_ref()
                    .is_some_and(|doc| doc.can_auto_reload())
            {
                let changed = app.model.document.as_ref().map(|d| d.changed()).transpose();
                match changed {
                    Ok(Some(true)) => app.execute(RELOAD, data),
                    Err(e) => app.report(e),
                    _ => {}
                }
            }
            return 0;
        }
        WM_CLOSE => {
            if !app.exiting && app.shell.active && app.model.settings.minimize_on_close {
                ShowWindow(hwnd, SW_MINIMIZE);
                ShowWindow(hwnd, SW_HIDE);
                return 0;
            }
            app.save_geometry();
            if let Err(e) = app.model.save() {
                app.report(e);
            }
            DestroyWindow(hwnd);
            return 0;
        }
        _ => {}
    }
    // Default processing of WM_WINDOWPOSCHANGED/WM_SYSCOMMAND synchronously
    // sends WM_SIZE. Release state so that nested layout can run.
    drop(app);
    DefWindowProcW(hwnd, message, wparam, lparam)
}

unsafe extern "system" fn list_proc(
    hwnd: HWND,
    message: u32,
    wp: usize,
    lp: isize,
    id: usize,
    reference: usize,
) -> isize {
    let data = &*(reference as *const WindowData);
    if message == WM_SETCURSOR || message == WM_LBUTTONUP {
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        ScreenToClient(hwnd, &mut point);
        let link = data
            .paint
            .try_borrow()
            .ok()
            .and_then(|paint| text::hit_link(hwnd, &paint, point));
        if let Some(link) = link {
            if message == WM_SETCURSOR {
                SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                return 1;
            }
            if let Err(error) = open::that(link) {
                error_box(GetParent(hwnd), &error.to_string());
            }
        }
    }
    if message == WM_NCDESTROY {
        RemoveWindowSubclass(hwnd, Some(list_proc), id);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}

impl Native {
    unsafe fn create_fonts(&mut self) {
        let old = self.fonts;
        let settings = &self.model.settings;
        self.fonts = [
            font(self.dpi, 12.0, FW_NORMAL as i32, false, false),
            task_font(self.dpi, settings, false, false, false),
            task_font(self.dpi, settings, true, false, false),
            task_font(self.dpi, settings, false, true, false),
            task_font(self.dpi, settings, false, false, true),
            task_font(self.dpi, settings, false, true, true),
        ];
        for control in [self.editor, self.status, self.suggestions] {
            if !control.is_null() {
                SendMessageW(control, WM_SETFONT, self.fonts[0] as usize, 1);
            }
        }
        for font in old {
            if !font.is_null() {
                DeleteObject(font);
            }
        }
    }
    unsafe fn layout(&mut self, _data: &WindowData) {
        let mut rect: RECT = zeroed();
        GetClientRect(self.hwnd, &mut rect);
        let editor_height = scale(self.dpi, 23);
        let status_height = if self.model.settings.status_bar {
            scale(self.dpi, 23)
        } else {
            0
        };
        MoveWindow(self.editor, 0, 0, rect.right, editor_height, 1);
        MoveWindow(
            self.list,
            0,
            editor_height + scale(self.dpi, 10),
            rect.right,
            (rect.bottom - editor_height - status_height - scale(self.dpi, 10)).max(1),
            1,
        );
        ShowWindow(
            self.status,
            if status_height > 0 { SW_SHOW } else { SW_HIDE },
        );
        MoveWindow(
            self.status,
            0,
            rect.bottom - status_height,
            rect.right,
            status_height,
            1,
        );
        ShowWindow(self.suggestions, SW_HIDE);
    }
    unsafe fn rebuild(&mut self, data: &WindowData) {
        if self.list.is_null() {
            return;
        }
        let mut rect: RECT = zeroed();
        GetClientRect(self.list, &mut rect);
        self.last_width = rect.right;
        let dc = GetDC(self.list);
        SelectObject(dc, self.fonts[1]);
        let paint = text::prepare(&self.model, dc, rect.right, self.dpi, self.fonts);
        ReleaseDC(self.list, dc);
        *data.paint.borrow_mut() = paint;
        let top = SendMessageW(self.list, LB_GETTOPINDEX, 0, 0).max(0) as usize;
        SendMessageW(self.list, WM_SETREDRAW, 0, 0);
        SendMessageW(self.list, LB_RESETCONTENT, 0, 0);
        let mut extent = 0;
        for (n, row) in data.paint.borrow().rows.iter().enumerate() {
            SendMessageW(self.list, LB_ADDSTRING, 0, wide(&row.raw).as_ptr() as isize);
            SendMessageW(self.list, LB_SETITEMHEIGHT, n, row.height as isize);
            extent = extent.max(row.width);
            if let Row::Task(id) = self.model.rows[n]
                && self.model.selected.contains(&id)
            {
                SendMessageW(self.list, LB_SETSEL, 1, n as isize);
                SendMessageW(self.list, LB_SETCARETINDEX, n, 0);
            }
        }
        SendMessageW(
            self.list,
            LB_SETHORIZONTALEXTENT,
            if self.model.settings.word_wrap {
                0
            } else {
                extent as usize
            },
            0,
        );
        SendMessageW(
            self.list,
            LB_SETTOPINDEX,
            top.min(self.model.rows.len().saturating_sub(1)),
            0,
        );
        SendMessageW(self.list, WM_SETREDRAW, 1, 0);
        InvalidateRect(self.list, null(), 1);
        self.status_text();
        self.update_menu();
    }
    unsafe fn sync_selection(&mut self) {
        let mut selected = BTreeSet::new();
        for (n, row) in self.model.rows.iter().enumerate() {
            if SendMessageW(self.list, LB_GETSEL, n, 0) > 0 {
                match row {
                    Row::Task(id) => {
                        selected.insert(*id);
                    }
                    Row::Header(_) => {
                        SendMessageW(self.list, LB_SETSEL, 0, n as isize);
                    }
                }
            }
        }
        self.model.selected = selected;
    }
    unsafe fn status_text(&self) {
        let count = view::counts(&self.model.tasks, &self.model.rows, self.model.date);
        let labels = [
            if self.model.settings.active_preset > 0 {
                format!("Filter #: {}", self.model.settings.active_preset)
            } else if self.model.settings.filter.is_empty() {
                "Filter: None".into()
            } else {
                "Filter: Custom".into()
            },
            format!("Sort: {}", self.model.settings.sort.label()),
            format!("Tasks: {} of {}", count["visible"], self.model.tasks.len()),
            format!("Incomplete: {}", count["incomplete"]),
            format!("Due Today: {}", count["today"]),
            format!("Overdue: {}", count["overdue"]),
        ];
        let dc = GetDC(self.status);
        SelectObject(dc, self.fonts[0]);
        let mut right = 0;
        let mut parts = Vec::new();
        for text in &labels {
            let mut size: SIZE = zeroed();
            let s = wide(text);
            GetTextExtentPoint32W(dc, s.as_ptr(), (s.len() - 1) as i32, &mut size);
            right += size.cx + scale(self.dpi, 12);
            parts.push(right);
        }
        ReleaseDC(self.status, dc);
        parts[5] = -1;
        SendMessageW(
            self.status,
            SB_SETPARTS,
            parts.len(),
            parts.as_ptr() as isize,
        );
        for (n, text) in labels.iter().enumerate() {
            SendMessageW(self.status, SB_SETTEXTW, n, wide(text).as_ptr() as isize);
        }
    }
    unsafe fn update_menu(&self) {
        let menu = GetMenu(self.hwnd);
        for (n, sort) in Sort::ALL.iter().enumerate() {
            CheckMenuItem(
                menu,
                SORT_BASE as u32 + n as u32,
                MF_BYCOMMAND
                    | if *sort == self.model.settings.sort {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    },
            );
        }
        for (id, checked) in [
            (HIDE_FUTURE, self.model.settings.hide_future),
            (SHOW_HIDDEN, self.model.settings.show_hidden),
        ] {
            CheckMenuItem(
                menu,
                id as u32,
                MF_BYCOMMAND | if checked { MF_CHECKED } else { MF_UNCHECKED },
            );
        }
        for n in 0..=9 {
            CheckMenuItem(
                menu,
                PRESET_BASE as u32 + n,
                MF_BYCOMMAND
                    | if self.model.settings.active_preset == n as usize {
                        MF_CHECKED
                    } else {
                        MF_UNCHECKED
                    },
            );
        }
        for id in [EDIT, DUPLICATE] {
            EnableMenuItem(
                menu,
                id as u32,
                MF_BYCOMMAND
                    | if self.model.selected.len() == 1 {
                        MF_ENABLED
                    } else {
                        MF_GRAYED
                    },
            );
        }
        for id in [
            APPEND,
            DELETE,
            TOGGLE,
            PRIORITY,
            PRIORITY_UP,
            PRIORITY_DOWN,
            PRIORITY_CLEAR,
            DUE,
            THRESHOLD,
            POSTPONE,
            DEFER,
        ] {
            EnableMenuItem(
                menu,
                id as u32,
                MF_BYCOMMAND
                    | if self.model.selected.is_empty() {
                        MF_GRAYED
                    } else {
                        MF_ENABLED
                    },
            );
        }
    }
    unsafe fn editor_draft(&mut self, select_all: bool) {
        self.suppress_edit = true;
        set_text(self.editor, &self.model.draft);
        self.suppress_edit = false;
        SetFocus(self.editor);
        SendMessageW(
            self.editor,
            EM_SETSEL,
            if select_all { 0 } else { usize::MAX },
            -1,
        );
        self.suggestions_dismissed = true;
        ShowWindow(self.suggestions, SW_HIDE);
    }
    unsafe fn report(&self, e: impl std::fmt::Display) {
        use std::io::Write;
        if fs::create_dir_all(&self.model.config_dir).is_ok()
            && let Ok(mut log) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.model.config_dir.join("error.log"))
        {
            let _ = writeln!(log, "{} {e}", Local::now());
        }
        error_box(self.hwnd, &e.to_string());
    }
    unsafe fn execute(&mut self, id: u16, data: &WindowData) {
        self.model.debug_event(&format!("command {id}"));
        if let Err(e) = self.command(id, data) {
            self.report(e);
        }
    }
    unsafe fn command(&mut self, id: u16, data: &WindowData) -> io::Result<()> {
        match id {
            OPEN | NEW_FILE => {
                if let Some(path) = dialogs::file(self.hwnd, id == NEW_FILE, "todo.txt")? {
                    if id == NEW_FILE {
                        match fs::OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&path)
                        {
                            Ok(_) => {}
                            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                            Err(e) => return Err(e),
                        }
                    }
                    self.model.load(&path)?;
                    set_text(self.editor, "");
                    self.rebuild(data);
                    SetFocus(self.list);
                }
            }
            NEW => {
                self.model.begin_new();
                self.editor_draft(false);
            }
            EDIT | DUPLICATE => {
                if self.model.begin_edit(id == DUPLICATE) {
                    self.editor_draft(false);
                }
            }
            DELETE => {
                if !self.model.selected.is_empty()
                    && MessageBoxW(
                        self.hwnd,
                        w!("Are you sure you want to delete the selected tasks?"),
                        w!("Delete Tasks"),
                        MB_OKCANCEL | MB_ICONWARNING | MB_DEFBUTTON2,
                    ) == IDOK
                {
                    self.model.modify(|_| None)?;
                    self.model.editing = None;
                    self.model.selected.clear();
                    self.model.refresh();
                    self.rebuild(data);
                }
            }
            APPEND => {
                if let Some(text) = dialogs::input(
                    self.hwnd,
                    self.fonts[0],
                    self.dpi,
                    "Append Text",
                    "Text to append:",
                    "",
                    false,
                )? && !text.trim().is_empty()
                {
                    let date = self.model.date;
                    self.model.modify(|t| {
                        Some(Task::parse(&format!("{} {}", t.raw, text.trim()), date).raw)
                    })?;
                    self.rebuild(data);
                }
            }
            TOGGLE => {
                let date = self.model.date;
                self.model.modify(|t| Some(t.toggle(date)))?;
                if self.model.settings.auto_archive {
                    self.archive()?;
                }
                self.rebuild(data);
            }
            ARCHIVE => {
                self.archive()?;
                self.rebuild(data);
            }
            RELOAD => {
                self.model.reload()?;
                self.rebuild(data);
            }
            PRIORITY => {
                if let Some(text) = dialogs::input(
                    self.hwnd,
                    self.fonts[0],
                    self.dpi,
                    "Set Priority",
                    "Priority A–Z (leave blank to remove):",
                    "",
                    false,
                )? {
                    let value = text.trim().chars().next().map(|p| p.to_ascii_uppercase());
                    if text.trim().len() > 1 || value.is_some_and(|p| !p.is_ascii_uppercase()) {
                        return Err(io::Error::other(
                            "Priority must be one letter A–Z, or blank.",
                        ));
                    }
                    self.model.modify(|t| Some(t.with_priority(value)))?;
                    self.rebuild(data);
                }
            }
            PRIORITY_UP | PRIORITY_DOWN | PRIORITY_CLEAR => {
                self.model.modify(|t| {
                    Some(if id == PRIORITY_CLEAR {
                        t.with_priority(None)
                    } else {
                        t.shifted_priority(if id == PRIORITY_UP { -1 } else { 1 })
                    })
                })?;
                self.rebuild(data);
            }
            DUE | THRESHOLD => {
                let key = if id == DUE { "due" } else { "t" };
                let current = self
                    .model
                    .selected_tasks()
                    .first()
                    .map(|(_, t)| {
                        if id == DUE {
                            t.due_date.clone()
                        } else {
                            t.threshold_date.clone()
                        }
                    })
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| self.model.date.to_string());
                if let Some(text) = dialogs::input(
                    self.hwnd,
                    self.fonts[0],
                    self.dpi,
                    if id == DUE {
                        "Set Due Date"
                    } else {
                        "Set Threshold Date"
                    },
                    "YYYY-MM-DD, today, tomorrow, or weekday:",
                    &current,
                    false,
                )? {
                    let date = resolve_date(text.trim(), self.model.date)
                        .ok_or_else(|| io::Error::other("Enter a valid date."))?;
                    self.model.modify(|t| Some(t.with_date(key, Some(date))))?;
                    self.rebuild(data);
                }
            }
            POSTPONE | DEFER => {
                if let Some(text) = dialogs::input(
                    self.hwnd,
                    self.fonts[0],
                    self.dpi,
                    if id == POSTPONE {
                        "Postpone"
                    } else {
                        "Threshold"
                    },
                    "Number of days to add:",
                    "1",
                    false,
                )? {
                    let days = text
                        .trim()
                        .parse::<i64>()
                        .ok()
                        .filter(|n| (-365000..=365000).contains(n))
                        .ok_or_else(|| {
                            io::Error::other(
                                "Enter a whole number of days between -365000 and 365000.",
                            )
                        })?;
                    let date = self.model.date;
                    self.model.modify(|t| {
                        Some(t.shift_date(if id == POSTPONE { "due" } else { "t" }, days, date))
                    })?;
                    self.rebuild(data);
                }
            }
            DUE_UP | DUE_DOWN | DUE_CLEAR | THRESHOLD_UP | THRESHOLD_DOWN | THRESHOLD_CLEAR => {
                let key = if id <= DUE_CLEAR { "due" } else { "t" };
                let date = self.model.date;
                self.model.modify(|t| {
                    Some(if id == DUE_CLEAR || id == THRESHOLD_CLEAR {
                        t.with_date(key, None)
                    } else {
                        t.shift_date(
                            key,
                            if id == DUE_UP || id == THRESHOLD_UP {
                                1
                            } else {
                                -1
                            },
                            date,
                        )
                    })
                })?;
                self.rebuild(data);
            }
            FILTER => {
                if let Some(settings) = dialogs::filters(
                    self.hwnd,
                    self.fonts[0],
                    self.dpi,
                    &self.model.settings,
                    &self.model.tasks,
                )? {
                    self.model.settings = settings;
                    self.model.settings.active_preset = 0;
                    self.model.refresh();
                    self.model.save()?;
                    self.rebuild(data);
                }
            }
            OPTIONS => {
                if let Some(settings) =
                    dialogs::options(self.hwnd, self.fonts[0], self.dpi, &self.model.settings)?
                {
                    self.model.settings = settings;
                    self.model.refresh();
                    self.model.save()?;
                    self.create_fonts();
                    self.layout(data);
                    self.rebuild(data);
                    self.shell
                        .configure(self.hwnd, self.model.settings.minimize_to_tray)?;
                }
            }
            HIDE_FUTURE | SHOW_HIDDEN => {
                if id == HIDE_FUTURE {
                    self.model.settings.hide_future = !self.model.settings.hide_future;
                } else {
                    self.model.settings.show_hidden = !self.model.settings.show_hidden;
                }
                self.model.refresh();
                self.model.save()?;
                self.rebuild(data);
            }
            COPY | CUT | PASTE => {
                if GetFocus() == self.editor {
                    SendMessageW(
                        self.editor,
                        if id == COPY {
                            WM_COPY
                        } else if id == CUT {
                            WM_CUT
                        } else {
                            WM_PASTE
                        },
                        0,
                        0,
                    );
                } else if id == PASTE {
                    let text = clipboard_text(self.hwnd)?;
                    for line in text.lines().filter(|s| !s.is_empty()) {
                        if let Some(doc) = &mut self.model.document {
                            doc.add(&Task::parse(line, self.model.date).raw)?;
                        }
                    }
                    self.model.refresh();
                    self.rebuild(data);
                } else {
                    set_clipboard(
                        self.hwnd,
                        &self
                            .model
                            .selected_tasks()
                            .iter()
                            .map(|(_, t)| t.raw.as_str())
                            .collect::<Vec<_>>()
                            .join("\r\n"),
                    )?;
                    if id == CUT {
                        self.execute(DELETE, data);
                    }
                }
            }
            HELP => {
                dialogs::input(
                    self.hwnd,
                    self.fonts[0],
                    self.dpi,
                    "About / Help — todotxt.rs",
                    "Rust port of todotxt.net by Ben Hughes. BSD licensed.",
                    include_str!("../../docs/HELP.md"),
                    true,
                )?;
            }
            CALENDAR => {
                self.calendar = !self.calendar;
                let title = if self.calendar {
                    let days = (0..7)
                        .map(|n| {
                            let date = Local::now().date_naive() + Duration::days(n);
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
                set_text(self.hwnd, &title);
            }
            PRINT => printing::print(self.hwnd, &self.model)?,
            PRINT_PREVIEW => {
                open::that(self.model.print_preview()?).map_err(io::Error::other)?;
            }
            LOG => {
                let path = self.model.config_dir.join("error.log");
                if !path.exists() {
                    fs::create_dir_all(&self.model.config_dir)?;
                    fs::write(&path, "No errors recorded.\n")?;
                }
                open::that(path).map_err(io::Error::other)?;
            }
            EXIT => {
                self.exiting = true;
                PostMessageW(self.hwnd, WM_CLOSE, 0, 0);
            }
            n if (SORT_BASE..SORT_BASE + 8).contains(&n) => {
                self.model.settings.sort = Sort::ALL[(n - SORT_BASE) as usize];
                self.model.refresh();
                self.model.save()?;
                self.rebuild(data);
            }
            n if (PRESET_BASE..=PRESET_BASE + 9).contains(&n) => {
                self.model.preset((n - PRESET_BASE) as usize)?;
                self.rebuild(data);
            }
            _ => {}
        }
        Ok(())
    }
    unsafe fn archive(&mut self) -> io::Result<()> {
        let path = if self.model.settings.auto_archive_path {
            self.model
                .document
                .as_ref()
                .map(|d| d.path.with_file_name("done.txt"))
        } else {
            self.model.settings.archive.clone()
        };
        if let Some(path) = path.or(dialogs::file_if_needed(
            self.hwnd,
            self.model.settings.archive.is_none() && !self.model.settings.auto_archive_path,
            "done.txt",
        )?) {
            self.model.archive(&path)?;
        }
        Ok(())
    }
    unsafe fn save_geometry(&self) {
        let mut placement: WINDOWPLACEMENT = zeroed();
        placement.length = size_of::<WINDOWPLACEMENT>() as u32;
        if GetWindowPlacement(self.hwnd, &mut placement) != 0 {
            let r = placement.rcNormalPosition;
            let rect = [r.left, r.top, r.right - r.left, r.bottom - r.top];
            if let Ok(bytes) = serde_json::to_vec(&rect) {
                let _ = todotxt_rs::document::atomic_write(
                    &self.model.config_dir.join("native-window.json"),
                    &bytes,
                );
            }
        }
    }
    unsafe fn complete(&mut self) {
        if GetFocus() != self.editor || self.suggestions_dismissed {
            ShowWindow(self.suggestions, SW_HIDE);
            return;
        }
        let mut end: u32 = 0;
        SendMessageW(self.editor, EM_GETSEL, 0, &mut end as *mut u32 as isize);
        let utf16: Vec<_> = self.model.draft.encode_utf16().collect();
        let end = (end as usize).min(utf16.len());
        let start = utf16[..end]
            .iter()
            .rposition(|c| char::from_u32(*c as u32).is_some_and(char::is_whitespace))
            .map_or(0, |n| n + 1);
        let prefix = String::from_utf16_lossy(&utf16[start..end]);
        self.suggestion_tags = if prefix.starts_with(['+', '@', '(']) {
            view::suggestions(
                &self.model.tasks,
                &prefix,
                self.model.settings.intellisense_case,
            )
        } else {
            Vec::new()
        };
        self.suggestion_range = (start, end);
        self.suggestion_index = 0;
        SendMessageW(self.suggestions, LB_RESETCONTENT, 0, 0);
        for tag in &self.suggestion_tags {
            SendMessageW(
                self.suggestions,
                LB_ADDSTRING,
                0,
                wide(tag).as_ptr() as isize,
            );
        }
        if self.suggestion_tags.is_empty() {
            ShowWindow(self.suggestions, SW_HIDE);
        } else {
            SendMessageW(self.suggestions, LB_SETCURSEL, 0, 0);
            let cursor = SendMessageW(self.editor, EM_POSFROMCHAR, start, 0) as u32;
            let x = (cursor & 0xffff) as i16 as i32;
            let mut rect: RECT = zeroed();
            GetClientRect(self.hwnd, &mut rect);
            let width = scale(self.dpi, 180).min(rect.right);
            SetWindowPos(
                self.suggestions,
                HWND_TOP,
                x.max(0).min((rect.right - width).max(0)),
                scale(self.dpi, 23),
                width,
                scale(self.dpi, 20) * self.suggestion_tags.len().min(8) as i32,
                SWP_SHOWWINDOW | SWP_NOACTIVATE,
            );
        }
    }
    unsafe fn accept_completion(&mut self) {
        if let Some(tag) = self.suggestion_tags.get(self.suggestion_index) {
            self.suppress_edit = true;
            SendMessageW(
                self.editor,
                EM_SETSEL,
                self.suggestion_range.0,
                self.suggestion_range.1 as isize,
            );
            SendMessageW(self.editor, EM_REPLACESEL, 1, wide(tag).as_ptr() as isize);
            self.model.draft = window_text(self.editor);
            self.suppress_edit = false;
        }
        self.suggestions_dismissed = true;
        ShowWindow(self.suggestions, SW_HIDE);
        SetFocus(self.editor);
    }
    unsafe fn key(&mut self, source: HWND, key: u16, data: &WindowData) -> bool {
        let ctrl = GetKeyState(VK_CONTROL as i32) < 0;
        let alt = GetKeyState(VK_MENU as i32) < 0;
        let shift = GetKeyState(VK_SHIFT as i32) < 0;
        // Route queued keys by their destination, even if focus changed before dispatch.
        let editor = source == self.editor;
        let command;
        if key == VK_F5 {
            command = RELOAD;
        } else if key == VK_F10 {
            command = OPTIONS;
        } else if ctrl && !alt && key == b'O' as u16 {
            command = OPEN;
        } else if ctrl && !alt && key == b'N' as u16 {
            command = NEW_FILE;
        } else if ctrl && !alt && key == b'P' as u16 {
            command = if shift { PRINT_PREVIEW } else { PRINT };
        } else if editor {
            if IsWindowVisible(self.suggestions) != 0
                && matches!(
                    key,
                    VK_DOWN | VK_UP | VK_TAB | VK_RETURN | VK_SPACE | VK_ESCAPE
                )
            {
                match key {
                    VK_DOWN => {
                        self.suggestion_index = (self.suggestion_index + 1)
                            .min(self.suggestion_tags.len().saturating_sub(1))
                    }
                    VK_UP => self.suggestion_index = self.suggestion_index.saturating_sub(1),
                    VK_ESCAPE => {
                        self.suggestions_dismissed = true;
                        ShowWindow(self.suggestions, SW_HIDE);
                    }
                    VK_TAB | VK_RETURN | VK_SPACE => self.accept_completion(),
                    _ => {}
                }
                SendMessageW(self.suggestions, LB_SETCURSEL, self.suggestion_index, 0);
                return true;
            }
            if key == VK_RETURN {
                if !self.model.settings.ctrl_enter || ctrl {
                    self.model.draft = window_text(self.editor);
                    match self.model.submit() {
                        Ok(Some(id)) => {
                            self.suppress_edit = true;
                            set_text(self.editor, "");
                            self.suppress_edit = false;
                            self.rebuild(data);
                            if self.model.settings.focus_list {
                                SetFocus(self.list);
                            }
                            if let Some(n) =
                                self.model.rows.iter().position(|r| *r == Row::Task(id))
                            {
                                SendMessageW(self.list, LB_SETCARETINDEX, n, 0);
                            }
                        }
                        Ok(None) => {}
                        Err(e) => self.report(e),
                    }
                }
                return true;
            }
            if key == VK_ESCAPE {
                self.model.editing = None;
                self.model.draft.clear();
                set_text(self.editor, "");
                SetFocus(self.list);
                return true;
            }
            if key == VK_TAB {
                SetFocus(self.list);
                return true;
            }
            return false;
        } else if ctrl && alt {
            command = match key {
                VK_UP => DUE_UP,
                VK_DOWN => DUE_DOWN,
                VK_LEFT | VK_RIGHT => DUE_CLEAR,
                n if n == b'P' as u16 => DEFER,
                _ => 0,
            };
        } else if alt {
            command = match key {
                VK_UP => PRIORITY_UP,
                VK_DOWN => PRIORITY_DOWN,
                VK_LEFT | VK_RIGHT => PRIORITY_CLEAR,
                _ => 0,
            };
        } else if ctrl {
            command = match key {
                VK_UP => THRESHOLD_UP,
                VK_DOWN => THRESHOLD_DOWN,
                VK_LEFT | VK_RIGHT => THRESHOLD_CLEAR,
                n if n == b'S' as u16 => THRESHOLD,
                n if n == b'T' as u16 => HIDE_FUTURE,
                n if n == b'H' as u16 => SHOW_HIDDEN,
                n if n == b'C' as u16 && shift => DUPLICATE,
                n if n == b'C' as u16 => COPY,
                n if n == b'V' as u16 => PASTE,
                n if n == b'X' as u16 => CUT,
                n if (b'0' as u16..=b'7' as u16).contains(&n) => SORT_BASE + n - b'0' as u16,
                _ => 0,
            };
            if key == b'A' as u16 {
                for (n, row) in self.model.rows.iter().enumerate() {
                    SendMessageW(
                        self.list,
                        LB_SETSEL,
                        matches!(row, Row::Task(_)) as usize,
                        n as isize,
                    );
                }
                self.sync_selection();
                return true;
            }
        } else {
            command = match key {
                n if n == b'N' as u16 => NEW,
                n if n == b'O' as u16 => OPEN,
                n if n == b'C' as u16 => NEW_FILE,
                n if n == b'U' as u16 || n == VK_F2 => EDIT,
                n if n == b'T' as u16 => APPEND,
                n if n == b'X' as u16 => TOGGLE,
                n if n == b'A' as u16 => ARCHIVE,
                n if n == b'D' as u16 || n == VK_DELETE || n == VK_BACK => DELETE,
                n if n == b'F' as u16 => FILTER,
                n if n == b'I' as u16 => PRIORITY,
                n if n == b'S' as u16 => DUE,
                n if n == b'P' as u16 => POSTPONE,
                VK_RSHIFT => CALENDAR,
                VK_OEM_PERIOD => RELOAD,
                VK_OEM_2 if shift => HELP,
                n if (b'0' as u16..=b'9' as u16).contains(&n) => PRESET_BASE + n - b'0' as u16,
                _ => 0,
            };
            if key == b'J' as u16 || key == b'K' as u16 || key == VK_UP || key == VK_DOWN {
                self.navigate(
                    if key == b'J' as u16 || key == VK_DOWN {
                        1
                    } else {
                        -1
                    },
                    shift,
                );
                return true;
            }
            if key == VK_TAB {
                SetFocus(self.editor);
                return true;
            }
        }
        if command != 0 {
            self.execute(command, data);
            true
        } else {
            false
        }
    }
    unsafe fn navigate(&mut self, direction: isize, extend: bool) {
        let index = SendMessageW(self.list, LB_GETCARETINDEX, 0, 0).max(0);
        let mut next = index + direction;
        while next >= 0 && (next as usize) < self.model.rows.len() {
            if matches!(self.model.rows[next as usize], Row::Task(_)) {
                break;
            }
            next += direction;
        }
        if next < 0 || next as usize >= self.model.rows.len() {
            return;
        }
        if !extend {
            SendMessageW(self.list, LB_SETSEL, 0, -1);
        }
        SendMessageW(self.list, LB_SETSEL, 1, next);
        SendMessageW(self.list, LB_SETCARETINDEX, next as usize, 0);
        self.sync_selection();
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        unsafe {
            for font in self.fonts {
                if !font.is_null() {
                    DeleteObject(font);
                }
            }
        }
    }
}
unsafe fn error_box(parent: HWND, text: &str) {
    MessageBoxW(
        parent,
        wide(text).as_ptr(),
        w!("todotxt.rs — Error"),
        MB_OK | MB_ICONERROR,
    );
}
unsafe fn make_menu() -> HMENU {
    let bar = CreateMenu();
    let groups: [(&str, Vec<(u16, String)>); 6] = [
        (
            "&File",
            vec![
                (NEW_FILE, "&New\tCtrl+N".into()),
                (OPEN, "&Open...\tCtrl+O".into()),
                (PRINT, "&Print\tCtrl+P".into()),
                (PRINT_PREVIEW, "Print Pre&view\tCtrl+Shift+P".into()),
                (0, "".into()),
                (ARCHIVE, "&Archive Completed Tasks\tA".into()),
                (RELOAD, "&Reload File\tF5".into()),
                (0, "".into()),
                (OPTIONS, "Op&tions...\tF10".into()),
                (EXIT, "E&xit\tAlt+F4".into()),
            ],
        ),
        (
            "&Edit",
            vec![
                (CUT, "Cu&t\tCtrl+X".into()),
                (COPY, "&Copy\tCtrl+C".into()),
                (DUPLICATE, "Copy Task to &New Task\tCtrl+Shift+C".into()),
                (PASTE, "&Paste\tCtrl+V".into()),
            ],
        ),
        (
            "&Task",
            vec![
                (NEW, "Add &New Task\tN".into()),
                (EDIT, "&Update Task\tU".into()),
                (APPEND, "Append &Text...\tT".into()),
                (DELETE, "&Delete Task\tD".into()),
                (0, "".into()),
                (TOGGLE, "Toggle &Completion\tX".into()),
                (0, "".into()),
                (PRIORITY, "Set Pr&iority...\tI".into()),
                (PRIORITY_UP, "Increase Priority\tAlt+Up".into()),
                (PRIORITY_DOWN, "Decrease Priority\tAlt+Down".into()),
                (PRIORITY_CLEAR, "Remove Priority\tAlt+Left/Right".into()),
                (0, "".into()),
                (DUE, "&Set Due Date...\tS".into()),
                (POSTPONE, "&Postpone...\tP".into()),
                (DUE_UP, "Increase Due Date By 1 Day\tCtrl+Alt+Up".into()),
                (DUE_DOWN, "Decrease Due Date By 1 Day\tCtrl+Alt+Down".into()),
                (DUE_CLEAR, "Remove Due Date\tCtrl+Alt+Left/Right".into()),
                (0, "".into()),
                (THRESHOLD, "Set Threshold Date...\tCtrl+S".into()),
                (DEFER, "Threshold...\tCtrl+Alt+P".into()),
                (
                    THRESHOLD_UP,
                    "Increase Threshold Date By 1 Day\tCtrl+Up".into(),
                ),
                (
                    THRESHOLD_DOWN,
                    "Decrease Threshold Date By 1 Day\tCtrl+Down".into(),
                ),
                (
                    THRESHOLD_CLEAR,
                    "Remove Threshold Date\tCtrl+Left/Right".into(),
                ),
            ],
        ),
        (
            "&Sort",
            Sort::ALL
                .iter()
                .enumerate()
                .map(|(n, s)| (SORT_BASE + n as u16, format!("{}\tCtrl+{n}", s.label())))
                .collect(),
        ),
        (
            "Fi&lter",
            [
                (HIDE_FUTURE, "Hide &future tasks\tCtrl+T".into()),
                (SHOW_HIDDEN, "Show &hidden tasks\tCtrl+H".into()),
                (0, "".into()),
                (FILTER, "Define &Filters...\tF".into()),
                (PRESET_BASE, "&Remove Filter\t0".into()),
                (0, "".into()),
            ]
            .into_iter()
            .chain((1..=9).map(|n| (PRESET_BASE + n, format!("Apply Preset Filter &{n}\t{n}"))))
            .collect(),
        ),
        (
            "&Help",
            vec![
                (HELP, "&About / Help\t?".into()),
                (LOG, "View Error &Log".into()),
                (CALENDAR, "Show &Calendar\tRight Shift".into()),
            ],
        ),
    ];
    for (name, items) in groups {
        let menu = CreatePopupMenu();
        for (id, label) in items {
            AppendMenuW(
                menu,
                if id == 0 { MF_SEPARATOR } else { MF_STRING },
                id as usize,
                wide(&label).as_ptr(),
            );
        }
        AppendMenuW(bar, MF_POPUP, menu as usize, wide(name).as_ptr());
    }
    bar
}
unsafe fn set_clipboard(hwnd: HWND, text: &str) -> io::Result<()> {
    if OpenClipboard(hwnd) == 0 {
        return Err(io::Error::last_os_error());
    }
    let result = (|| {
        let s = wide(text);
        let mem = GlobalAlloc(GMEM_MOVEABLE, s.len() * 2);
        if mem.is_null() {
            return Err(io::Error::last_os_error());
        }
        let ptr = GlobalLock(mem) as *mut u16;
        if ptr.is_null() {
            GlobalFree(mem);
            return Err(io::Error::last_os_error());
        }
        std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, s.len());
        GlobalUnlock(mem);
        EmptyClipboard();
        if SetClipboardData(13, mem).is_null() {
            GlobalFree(mem);
            return Err(io::Error::last_os_error());
        }
        Ok(())
    })();
    CloseClipboard();
    result
}
unsafe fn clipboard_text(hwnd: HWND) -> io::Result<String> {
    if OpenClipboard(hwnd) == 0 {
        return Err(io::Error::last_os_error());
    }
    let result = (|| {
        let mem = GetClipboardData(13);
        if mem.is_null() {
            return Ok(String::new());
        }
        let ptr = GlobalLock(mem) as *const u16;
        if ptr.is_null() {
            return Err(io::Error::last_os_error());
        }
        let len = GlobalSize(mem) / 2;
        let slice = std::slice::from_raw_parts(ptr, len);
        let end = slice.iter().position(|c| *c == 0).unwrap_or(len);
        let text = String::from_utf16_lossy(&slice[..end]);
        GlobalUnlock(mem);
        Ok(text)
    })();
    CloseClipboard();
    result
}
