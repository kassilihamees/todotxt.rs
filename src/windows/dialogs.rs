use super::*;
use todotxt_rs::settings::Settings;
use windows_sys::Win32::UI::Controls::Dialogs::*;

pub(super) unsafe fn file_if_needed(
    parent: HWND,
    needed: bool,
    name: &str,
) -> io::Result<Option<PathBuf>> {
    if needed {
        archive_file(parent, name)
    } else {
        Ok(None)
    }
}
pub(super) unsafe fn file(parent: HWND, save: bool, name: &str) -> io::Result<Option<PathBuf>> {
    select_file(
        parent,
        if save {
            FilePurpose::NewTodo
        } else {
            FilePurpose::OpenTodo
        },
        name,
    )
}

unsafe fn archive_file(parent: HWND, name: &str) -> io::Result<Option<PathBuf>> {
    select_file(parent, FilePurpose::Archive, name)
}

enum FilePurpose {
    OpenTodo,
    NewTodo,
    Archive,
}

unsafe fn select_file(
    parent: HWND,
    purpose: FilePurpose,
    name: &str,
) -> io::Result<Option<PathBuf>> {
    let save = !matches!(purpose, FilePurpose::OpenTodo);
    let mut buffer = vec![0u16; 32768];
    if save {
        let name = wide(name);
        buffer[..name.len()].copy_from_slice(&name);
    }
    let filter: Vec<u16> = "Text documents (*.txt)\0*.txt\0All files (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title = wide(match purpose {
        FilePurpose::OpenTodo => "Open todo.txt file",
        FilePurpose::NewTodo => "Choose text file",
        FilePurpose::Archive => "Select archive file (tasks will be appended)",
    });
    let mut dialog = OPENFILENAMEW {
        lStructSize: size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: parent,
        lpstrFilter: filter.as_ptr(),
        nFilterIndex: 1,
        lpstrFile: buffer.as_mut_ptr(),
        nMaxFile: buffer.len() as u32,
        lpstrTitle: title.as_ptr(),
        lpstrDefExt: w!("txt"),
        Flags: OFN_EXPLORER
            | OFN_NOCHANGEDIR
            | OFN_PATHMUSTEXIST
            | match purpose {
                FilePurpose::OpenTodo => OFN_FILEMUSTEXIST,
                FilePurpose::NewTodo => OFN_OVERWRITEPROMPT,
                FilePurpose::Archive => 0,
            },
        ..zeroed()
    };
    let accepted = if save {
        GetSaveFileNameW(&mut dialog)
    } else {
        GetOpenFileNameW(&mut dialog)
    };
    if accepted == 0 {
        let error = CommDlgExtendedError();
        return if error == 0 {
            Ok(None)
        } else {
            Err(io::Error::other(format!(
                "Windows file dialog failed: {error:#x}"
            )))
        };
    }
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    Ok(Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..end],
    ))))
}

enum Kind {
    Input {
        title: String,
        label: String,
        text: String,
        readonly: bool,
    },
    Filters {
        settings: Box<Settings>,
        tasks: Vec<(usize, Task)>,
    },
    Options(Box<Settings>),
}
struct Dialog {
    hwnd: HWND,
    parent: HWND,
    font: HFONT,
    dpi: u32,
    edit: HWND,
    panel: HWND,
    filter_edits: Vec<HWND>,
    filter_labels: Vec<HWND>,
    scroll: i32,
    suggestions: HWND,
    tags: Vec<String>,
    range: (usize, usize),
    suggestion_index: usize,
    controls: Vec<HWND>,
    kind: Kind,
    result: Option<bool>,
}
impl Dialog {
    unsafe fn build(&mut self) {
        let (width, height) = match &self.kind {
            Kind::Input { readonly: true, .. } => (500, 450),
            Kind::Input { .. } => (390, 160),
            Kind::Filters { .. } => (405, 600),
            Kind::Options(_) => (510, 590),
        };
        let mut owner: RECT = zeroed();
        GetWindowRect(self.parent, &mut owner);
        SetWindowPos(
            self.hwnd,
            null_mut(),
            owner.left + ((owner.right - owner.left - scale(self.dpi, width)) / 2).max(0),
            owner.top + ((owner.bottom - owner.top - scale(self.dpi, height)) / 2).max(0),
            scale(self.dpi, width),
            scale(self.dpi, height),
            SWP_NOZORDER,
        );
        let label = match &self.kind {
            Kind::Input { label, .. } => label.clone(),
            Kind::Filters { .. } => {
                "One AND condition per line; 0 clears, 1–9 apply presets.".into()
            }
            Kind::Options(_) => "Archive File".into(),
        };
        let title = match &self.kind {
            Kind::Input { title, .. } => title.clone(),
            Kind::Filters { .. } => "Filter".into(),
            Kind::Options(_) => "Options".into(),
        };
        set_text(self.hwnd, &title);
        let label_hwnd = child(self.hwnd, w!("STATIC"), &label, 0, 100, self.font);
        self.place(label_hwnd, 12, 12, width - 40, 22);
        match &mut self.kind {
            Kind::Input { text, readonly, .. } => {
                self.edit = child(
                    self.hwnd,
                    w!("EDIT"),
                    &text.replace('\n', "\r\n").replace("\r\r\n", "\r\n"),
                    WS_BORDER
                        | WS_TABSTOP
                        | if *readonly {
                            ES_MULTILINE as u32
                                | ES_READONLY as u32
                                | ES_AUTOVSCROLL as u32
                                | WS_VSCROLL
                        } else {
                            ES_AUTOHSCROLL as u32
                        },
                    101,
                    self.font,
                );
                MoveWindow(
                    self.edit,
                    scale(self.dpi, 12),
                    scale(self.dpi, 38),
                    scale(self.dpi, width - 40),
                    scale(self.dpi, if *readonly { height - 115 } else { 24 }),
                    1,
                );
            }
            Kind::Filters { settings, .. } => {
                self.panel = CreateWindowExW(
                    WS_EX_CONTROLPARENT,
                    w!("TodoTxtRustFilterPanel"),
                    w!(""),
                    WS_CHILD | WS_VISIBLE | WS_VSCROLL,
                    scale(self.dpi, 12),
                    scale(self.dpi, 38),
                    scale(self.dpi, width - 40),
                    scale(self.dpi, 430),
                    self.hwnd,
                    102usize as HMENU,
                    GetModuleHandleW(null()),
                    GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *const _,
                );
                for n in 0..10 {
                    let heading = if n == 0 {
                        "Active Filter".into()
                    } else {
                        format!("Preset {n}")
                    };
                    let label = child(self.panel, w!("STATIC"), &heading, 0, 700 + n, self.font);
                    let text = if n == 0 {
                        &settings.filter
                    } else {
                        &settings.presets[n - 1]
                    };
                    let edit = child(
                        self.panel,
                        w!("EDIT"),
                        &text.replace('\n', "\r\n"),
                        WS_BORDER
                            | WS_TABSTOP
                            | WS_VSCROLL
                            | ES_MULTILINE as u32
                            | ES_AUTOVSCROLL as u32
                            | ES_WANTRETURN as u32,
                        600 + n,
                        self.font,
                    );
                    MoveWindow(
                        label,
                        0,
                        scale(self.dpi, n as i32 * 100),
                        scale(self.dpi, width - 66),
                        scale(self.dpi, 20),
                        1,
                    );
                    MoveWindow(
                        edit,
                        0,
                        scale(self.dpi, n as i32 * 100 + 22),
                        scale(self.dpi, width - 66),
                        scale(self.dpi, 70),
                        1,
                    );
                    self.filter_labels.push(label);
                    self.filter_edits.push(edit);
                }
                self.edit = self.filter_edits[0];
                let info = SCROLLINFO {
                    cbSize: size_of::<SCROLLINFO>() as u32,
                    fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
                    nMin: 0,
                    nMax: scale(self.dpi, 1000) - 1,
                    nPage: scale(self.dpi, 430) as u32,
                    nPos: 0,
                    ..zeroed()
                };
                SetScrollInfo(self.panel, SB_VERT, &info, 1);
                self.suggestions = child(
                    self.hwnd,
                    w!("LISTBOX"),
                    "",
                    WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
                    109,
                    self.font,
                );
                ShowWindow(self.suggestions, SW_HIDE);
                let clear = child(
                    self.hwnd,
                    w!("BUTTON"),
                    "Clear Active",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    103,
                    self.font,
                );
                self.place(clear, 12, height - 70, 88, 25);
                let clear_all = child(
                    self.hwnd,
                    w!("BUTTON"),
                    "Clear All",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    104,
                    self.font,
                );
                self.place(clear_all, 104, height - 70, 78, 25);
            }
            Kind::Options(s) => {
                self.edit = child(
                    self.hwnd,
                    w!("EDIT"),
                    &s.archive
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                    101,
                    self.font,
                );
                MoveWindow(
                    self.edit,
                    scale(self.dpi, 95),
                    scale(self.dpi, 12),
                    scale(self.dpi, 290),
                    scale(self.dpi, 23),
                    1,
                );
                let browse = child(
                    self.hwnd,
                    w!("BUTTON"),
                    "Select...",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    105,
                    self.font,
                );
                MoveWindow(
                    browse,
                    scale(self.dpi, 390),
                    scale(self.dpi, 12),
                    scale(self.dpi, 80),
                    scale(self.dpi, 23),
                    1,
                );
                let checks = [
                    (s.auto_archive, "Automatically archive completed tasks"),
                    (
                        s.auto_archive_path,
                        "Automatically select archive path (done.txt)",
                    ),
                    (s.add_creation, "Add created date to new tasks"),
                    (
                        s.focus_list,
                        "Move focus to task list after creating new task",
                    ),
                    (s.auto_refresh, "Automatically refresh task list from file"),
                    (s.case_sensitive, "Filter text is case-sensitive"),
                    (
                        s.intellisense_case,
                        "Intellisense suggestions are case-sensitive",
                    ),
                    (s.ctrl_enter, "Use Ctrl-Enter to create new task"),
                    (s.grouping, "Allow grouping of tasks"),
                    (
                        s.preserve_blank,
                        "Display blank lines and preserve whitespace in edits",
                    ),
                    (s.word_wrap, "Apply word wrap to task list"),
                    (s.status_bar, "Display status bar"),
                    (s.minimize_to_tray, "Minimize to system tray (Ctrl+Alt+M)"),
                    (
                        s.minimize_on_close,
                        "Minimize on close when tray mode is enabled",
                    ),
                    (s.debug_logging, "Enable debug logging"),
                ];
                for (n, (checked, label)) in checks.iter().enumerate() {
                    let control = child(
                        self.hwnd,
                        w!("BUTTON"),
                        label,
                        WS_TABSTOP | BS_AUTOCHECKBOX as u32,
                        200 + n,
                        self.font,
                    );
                    SendMessageW(
                        control,
                        BM_SETCHECK,
                        if *checked {
                            BST_CHECKED as usize
                        } else {
                            BST_UNCHECKED as usize
                        },
                        0,
                    );
                    MoveWindow(
                        control,
                        scale(self.dpi, 12),
                        scale(self.dpi, 46 + n as i32 * 26),
                        scale(self.dpi, 460),
                        scale(self.dpi, 23),
                        1,
                    );
                    self.controls.push(control);
                }
                let label = child(self.hwnd, w!("STATIC"), "Task font size", 0, 106, self.font);
                MoveWindow(
                    label,
                    scale(self.dpi, 12),
                    scale(self.dpi, 445),
                    scale(self.dpi, 90),
                    scale(self.dpi, 23),
                    1,
                );
                let size = child(
                    self.hwnd,
                    w!("EDIT"),
                    &s.font_size.to_string(),
                    WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                    107,
                    self.font,
                );
                MoveWindow(
                    size,
                    scale(self.dpi, 108),
                    scale(self.dpi, 445),
                    scale(self.dpi, 70),
                    scale(self.dpi, 23),
                    1,
                );
                self.controls.push(size);
                let chooser = child(
                    self.hwnd,
                    w!("BUTTON"),
                    "Select Font...",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    108,
                    self.font,
                );
                MoveWindow(
                    chooser,
                    scale(self.dpi, 200),
                    scale(self.dpi, 445),
                    scale(self.dpi, 120),
                    scale(self.dpi, 25),
                    1,
                );
            }
        }
        let ok = child(
            self.hwnd,
            w!("BUTTON"),
            "OK",
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
            IDOK as usize,
            self.font,
        );
        self.place(ok, width - 202, height - 70, 75, 25);
        let cancel = child(
            self.hwnd,
            w!("BUTTON"),
            "Cancel",
            WS_TABSTOP | BS_PUSHBUTTON as u32,
            IDCANCEL as usize,
            self.font,
        );
        self.place(cancel, width - 120, height - 70, 75, 25);
        SetFocus(self.edit);
        SendMessageW(self.edit, EM_SETSEL, 0, -1);
    }
    unsafe fn place(&self, hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
        MoveWindow(
            hwnd,
            scale(self.dpi, x),
            scale(self.dpi, y),
            scale(self.dpi, w),
            scale(self.dpi, h),
            1,
        );
    }
    unsafe fn store_filter(&mut self) {
        if let Kind::Filters { settings, .. } = &mut self.kind {
            settings.filter = window_text(self.filter_edits[0]).replace("\r\n", "\n");
            for n in 1..10 {
                settings.presets[n - 1] = window_text(self.filter_edits[n]).replace("\r\n", "\n");
            }
        }
    }
    unsafe fn scroll_filters(&mut self, position: i32) {
        self.scroll = position.clamp(0, scale(self.dpi, 570));
        SetScrollPos(self.panel, SB_VERT, self.scroll, 1);
        for n in 0..10 {
            let y = scale(self.dpi, n as i32 * 100) - self.scroll;
            SetWindowPos(
                self.filter_labels[n],
                null_mut(),
                0,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER,
            );
            SetWindowPos(
                self.filter_edits[n],
                null_mut(),
                0,
                y + scale(self.dpi, 22),
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER,
            );
        }
        ShowWindow(self.suggestions, SW_HIDE);
        InvalidateRect(self.panel, null(), 1);
    }
    unsafe fn complete(&mut self, edit: HWND) {
        self.edit = edit;
        let text: Vec<u16> = window_text(edit).encode_utf16().collect();
        let mut end = 0u32;
        SendMessageW(edit, EM_GETSEL, &mut end as *mut u32 as usize, 0);
        let end = (end as usize).min(text.len());
        let start = text[..end]
            .iter()
            .rposition(|c| char::from_u32(*c as u32).is_some_and(char::is_whitespace))
            .map_or(0, |n| n + 1);
        let prefix = String::from_utf16_lossy(&text[start..end]);
        self.tags = if prefix.starts_with(['+', '@', '(']) {
            if let Kind::Filters { tasks, settings } = &self.kind {
                view::suggestions(tasks, &prefix, settings.intellisense_case)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        self.range = (start, end);
        self.suggestion_index = 0;
        SendMessageW(self.suggestions, LB_RESETCONTENT, 0, 0);
        for tag in &self.tags {
            SendMessageW(
                self.suggestions,
                LB_ADDSTRING,
                0,
                wide(tag).as_ptr() as isize,
            );
        }
        if self.tags.is_empty() {
            ShowWindow(self.suggestions, SW_HIDE);
            return;
        }
        let mut point: POINT = zeroed();
        GetCaretPos(&mut point);
        MapWindowPoints(edit, self.hwnd, &mut point, 1);
        SetWindowPos(
            self.suggestions,
            HWND_TOP,
            point.x,
            point.y + scale(self.dpi, 20),
            scale(self.dpi, 220),
            scale(self.dpi, 18) * self.tags.len().min(6) as i32,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        SendMessageW(self.suggestions, LB_SETCURSEL, 0, 0);
    }
    unsafe fn accept_completion(&mut self) {
        if let Some(tag) = self.tags.get(self.suggestion_index) {
            SendMessageW(self.edit, EM_SETSEL, self.range.0, self.range.1 as isize);
            SendMessageW(self.edit, EM_REPLACESEL, 1, wide(tag).as_ptr() as isize);
            SetFocus(self.edit);
        }
        ShowWindow(self.suggestions, SW_HIDE);
    }
    unsafe fn key(&mut self, msg: &MSG) -> bool {
        if !self.filter_edits.contains(&msg.hwnd) {
            return false;
        }
        // Keep the focused preset visible while navigating with Tab.
        if IsWindowVisible(self.suggestions) != 0 {
            match msg.wParam as u16 {
                VK_RETURN | VK_TAB | VK_SPACE => self.accept_completion(),
                VK_DOWN => {
                    self.suggestion_index =
                        (self.suggestion_index + 1).min(self.tags.len().saturating_sub(1))
                }
                VK_UP => self.suggestion_index = self.suggestion_index.saturating_sub(1),
                VK_ESCAPE => {
                    ShowWindow(self.suggestions, SW_HIDE);
                }
                _ => return false,
            }
            SendMessageW(self.suggestions, LB_SETCURSEL, self.suggestion_index, 0);
            return true;
        }
        false
    }
    unsafe fn accept(&mut self) {
        self.store_filter();
        match &mut self.kind {
            Kind::Input { text, .. } => *text = window_text(self.edit),
            Kind::Options(settings) => {
                let path = window_text(self.edit);
                settings.archive = if path.is_empty() {
                    None
                } else {
                    Some(path.into())
                };
                let values: Vec<_> = self.controls[..15]
                    .iter()
                    .map(|h| SendMessageW(*h, BM_GETCHECK, 0, 0) == BST_CHECKED as isize)
                    .collect();
                settings.auto_archive = values[0];
                settings.auto_archive_path = values[1];
                settings.add_creation = values[2];
                settings.focus_list = values[3];
                settings.auto_refresh = values[4];
                settings.case_sensitive = values[5];
                settings.intellisense_case = values[6];
                settings.ctrl_enter = values[7];
                settings.grouping = values[8];
                settings.preserve_blank = values[9];
                settings.word_wrap = values[10];
                settings.status_bar = values[11];
                settings.minimize_to_tray = values[12];
                settings.minimize_on_close = values[13];
                settings.debug_logging = values[14];
                let Ok(size) = window_text(self.controls[15]).parse::<f32>() else {
                    error_box(self.hwnd, "Font size must be a number between 8 and 96.");
                    return;
                };
                if !size.is_finite() || !(8.0..=96.0).contains(&size) {
                    error_box(self.hwnd, "Font size must be a number between 8 and 96.");
                    return;
                }
                settings.font_size = size;
            }
            _ => {}
        }
        self.result = Some(true);
    }
}
unsafe extern "system" fn dialog_proc(hwnd: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Dialog>;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let Ok(mut dialog) = (&*ptr).try_borrow_mut() else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    match msg {
        WM_COMMAND => {
            let id = (wp & 0xffff) as i32;
            let notification = (wp >> 16) as u32;
            if (600..610).contains(&id) {
                if notification == EN_SETFOCUS {
                    let n = id - 600;
                    let y = scale(dialog.dpi, n * 100);
                    if y < dialog.scroll {
                        dialog.scroll_filters(y);
                    } else if y + scale(dialog.dpi, 92) > dialog.scroll + scale(dialog.dpi, 430) {
                        let bottom = y + scale(dialog.dpi, 92 - 430);
                        dialog.scroll_filters(bottom);
                    }
                    dialog.edit = lp as HWND;
                } else if notification == EN_CHANGE && GetFocus() == lp as HWND {
                    dialog.complete(lp as HWND);
                }
                return 0;
            }
            match id {
                IDOK => dialog.accept(),
                IDCANCEL => dialog.result = Some(false),
                103 => {
                    if !dialog.filter_edits.is_empty() {
                        set_text(dialog.filter_edits[0], "");
                    }
                }
                104 => {
                    for edit in &dialog.filter_edits {
                        set_text(*edit, "");
                    }
                }
                105 => match archive_file(dialog.hwnd, "done.txt") {
                    Ok(Some(path)) => set_text(dialog.edit, &path.to_string_lossy()),
                    Ok(None) => {}
                    Err(e) => error_box(dialog.hwnd, &e.to_string()),
                },
                108 => {
                    let owner = dialog.hwnd;
                    let dpi = dialog.dpi;
                    if let Kind::Options(settings) = &mut dialog.kind {
                        if let Err(error) = choose_font(owner, dpi, settings) {
                            error_box(owner, &error.to_string());
                        }
                        let size = settings.font_size.to_string();
                        set_text(dialog.controls[15], &size);
                    }
                }
                109 if notification == LBN_SELCHANGE || notification == LBN_DBLCLK => {
                    dialog.suggestion_index =
                        SendMessageW(dialog.suggestions, LB_GETCURSEL, 0, 0).max(0) as usize;
                    dialog.accept_completion();
                }
                _ => {}
            }
        }
        WM_CLOSE => dialog.result = Some(false),
        _ => return DefWindowProcW(hwnd, msg, wp, lp),
    }
    0
}
unsafe fn show(parent: HWND, font: HFONT, dpi: u32, kind: Kind) -> io::Result<Option<Kind>> {
    let instance = GetModuleHandleW(null());
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(dialog_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: (COLOR_BTNFACE + 1) as HBRUSH,
        lpszClassName: w!("TodoTxtRustDialog"),
        ..zeroed()
    };
    RegisterClassExW(&class);
    let panel_class = WNDCLASSEXW {
        lpfnWndProc: Some(panel_proc),
        lpszClassName: w!("TodoTxtRustFilterPanel"),
        ..class
    };
    RegisterClassExW(&panel_class);
    let state = RefCell::new(Dialog {
        hwnd: null_mut(),
        parent,
        font,
        dpi,
        edit: null_mut(),
        panel: null_mut(),
        filter_edits: Vec::new(),
        filter_labels: Vec::new(),
        scroll: 0,
        suggestions: null_mut(),
        tags: Vec::new(),
        range: (0, 0),
        suggestion_index: 0,
        controls: Vec::new(),
        kind,
        result: None,
    });
    let hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT,
        class.lpszClassName,
        w!(""),
        WS_POPUP | WS_CAPTION | WS_SYSMENU,
        0,
        0,
        400,
        160,
        parent,
        null_mut(),
        instance,
        (&state as *const RefCell<Dialog>).cast(),
    );
    if hwnd.is_null() {
        return Err(io::Error::last_os_error());
    }
    {
        let mut dialog = state.borrow_mut();
        dialog.hwnd = hwnd;
        dialog.build();
    }
    EnableWindow(parent, 0);
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
    SetFocus(state.borrow().edit);
    let mut msg: MSG = zeroed();
    while state.borrow().result.is_none() {
        let result = GetMessageW(&mut msg, null_mut(), 0, 0);
        if result <= 0 {
            if result == 0 {
                PostQuitMessage(msg.wParam as i32);
            }
            break;
        }
        if msg.message == WM_KEYDOWN && state.borrow_mut().key(&msg) {
            continue;
        }
        if IsDialogMessageW(hwnd, &msg) == 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    EnableWindow(parent, 1);
    DestroyWindow(hwnd);
    SetActiveWindow(parent);
    InvalidateRect(parent, null(), 0);
    let dialog = state.into_inner();
    Ok(if dialog.result == Some(true) {
        Some(dialog.kind)
    } else {
        None
    })
}
pub(super) unsafe fn input(
    parent: HWND,
    font: HFONT,
    dpi: u32,
    title: &str,
    label: &str,
    text: &str,
    readonly: bool,
) -> io::Result<Option<String>> {
    match show(
        parent,
        font,
        dpi,
        Kind::Input {
            title: title.into(),
            label: label.into(),
            text: text.into(),
            readonly,
        },
    )? {
        Some(Kind::Input { text, .. }) => Ok(Some(text)),
        _ => Ok(None),
    }
}
pub(super) unsafe fn filters(
    parent: HWND,
    font: HFONT,
    dpi: u32,
    settings: &Settings,
    tasks: &[(usize, Task)],
) -> io::Result<Option<Settings>> {
    match show(
        parent,
        font,
        dpi,
        Kind::Filters {
            settings: Box::new(settings.clone()),
            tasks: tasks.to_vec(),
        },
    )? {
        Some(Kind::Filters { settings, .. }) => Ok(Some(*settings)),
        _ => Ok(None),
    }
}
pub(super) unsafe fn options(
    parent: HWND,
    font: HFONT,
    dpi: u32,
    settings: &Settings,
) -> io::Result<Option<Settings>> {
    match show(parent, font, dpi, Kind::Options(Box::new(settings.clone())))? {
        Some(Kind::Options(settings)) => Ok(Some(*settings)),
        _ => Ok(None),
    }
}

unsafe fn choose_font(owner: HWND, dpi: u32, settings: &mut Settings) -> io::Result<()> {
    let mut lf: LOGFONTW = zeroed();
    lf.lfHeight = -(settings.font_size * dpi as f32 / 96.0).round() as i32;
    lf.lfWeight = settings.font_weight;
    lf.lfItalic = settings.font_italic as u8;
    lf.lfUnderline = settings.font_underline as u8;
    lf.lfStrikeOut = settings.font_strike as u8;
    lf.lfCharSet = DEFAULT_CHARSET;
    let name: Vec<u16> = settings.font_family.encode_utf16().take(31).collect();
    lf.lfFaceName[..name.len()].copy_from_slice(&name);
    let mut chooser = CHOOSEFONTW {
        lStructSize: size_of::<CHOOSEFONTW>() as u32,
        hwndOwner: owner,
        lpLogFont: &mut lf,
        Flags: CF_SCREENFONTS | CF_EFFECTS | CF_INITTOLOGFONTSTRUCT,
        rgbColors: settings.font_color,
        ..zeroed()
    };
    if ChooseFontW(&mut chooser) == 0 {
        let code = CommDlgExtendedError();
        if code != 0 {
            return Err(io::Error::other(format!(
                "Windows font dialog failed: {code:#x}"
            )));
        }
        return Ok(());
    }
    // ChooseFont sizes are tenths of a point; settings use device-independent pixels.
    settings.font_size = (chooser.iPointSize as f32 / 10.0 * 96.0 / 72.0).clamp(8.0, 96.0);
    let end = lf
        .lfFaceName
        .iter()
        .position(|c| *c == 0)
        .unwrap_or(lf.lfFaceName.len());
    settings.font_family = String::from_utf16_lossy(&lf.lfFaceName[..end]);
    settings.font_weight = lf.lfWeight;
    settings.font_italic = lf.lfItalic != 0;
    settings.font_underline = lf.lfUnderline != 0;
    settings.font_strike = lf.lfStrikeOut != 0;
    settings.font_color = chooser.rgbColors;
    Ok(())
}

unsafe extern "system" fn panel_proc(hwnd: HWND, msg: u32, wp: usize, lp: isize) -> isize {
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    if msg == WM_COMMAND {
        return SendMessageW(GetParent(hwnd), msg, wp, lp);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Dialog>;
    if !ptr.is_null()
        && let Ok(mut dialog) = (&*ptr).try_borrow_mut()
    {
        if msg == WM_VSCROLL {
            let position = match wp as u16 as i32 {
                SB_LINEUP => dialog.scroll - scale(dialog.dpi, 26),
                SB_LINEDOWN => dialog.scroll + scale(dialog.dpi, 26),
                SB_PAGEUP => dialog.scroll - scale(dialog.dpi, 400),
                SB_PAGEDOWN => dialog.scroll + scale(dialog.dpi, 400),
                SB_THUMBPOSITION | SB_THUMBTRACK => {
                    let mut info = SCROLLINFO {
                        cbSize: size_of::<SCROLLINFO>() as u32,
                        fMask: SIF_TRACKPOS,
                        ..zeroed()
                    };
                    GetScrollInfo(hwnd, SB_VERT, &mut info);
                    info.nTrackPos
                }
                SB_TOP => 0,
                SB_BOTTOM => scale(dialog.dpi, 570),
                _ => dialog.scroll,
            };
            dialog.scroll_filters(position);
            return 0;
        }
        if msg == WM_MOUSEWHEEL {
            let position = dialog.scroll - ((wp >> 16) as i16 as i32 / 120) * scale(dialog.dpi, 78);
            dialog.scroll_filters(position);
            return 0;
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
