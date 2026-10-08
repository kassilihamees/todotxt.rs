use super::*;
use todotxt_rs::settings::Settings;
use windows_sys::Win32::UI::Controls::Dialogs::*;

pub(super) unsafe fn file_if_needed(
    parent: HWND,
    needed: bool,
    name: &str,
) -> io::Result<Option<PathBuf>> {
    if needed {
        file(parent, true, name)
    } else {
        Ok(None)
    }
}
pub(super) unsafe fn file(parent: HWND, save: bool, name: &str) -> io::Result<Option<PathBuf>> {
    let mut buffer = vec![0u16; 32768];
    if save {
        let name = wide(name);
        buffer[..name.len()].copy_from_slice(&name);
    }
    let filter: Vec<u16> = "Text documents (*.txt)\0*.txt\0All files (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title = wide(if save {
        "Choose text file"
    } else {
        "Open todo.txt file"
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
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
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
        selected: usize,
    },
    Options(Box<Settings>),
}
struct Dialog {
    hwnd: HWND,
    parent: HWND,
    font: HFONT,
    dpi: u32,
    edit: HWND,
    tab: HWND,
    controls: Vec<HWND>,
    kind: Kind,
    result: Option<bool>,
}
impl Dialog {
    unsafe fn build(&mut self) {
        let (width, height) = match &self.kind {
            Kind::Input { readonly: true, .. } => (500, 450),
            Kind::Input { .. } => (390, 160),
            Kind::Filters { .. } => (450, 300),
            Kind::Options(_) => (510, 485),
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
                self.tab = child(self.hwnd, WC_TABCONTROLW, "", WS_TABSTOP, 102, self.font);
                for n in 0..10 {
                    let mut name = wide(&if n == 0 {
                        "Active".to_owned()
                    } else {
                        n.to_string()
                    });
                    let mut item = TCITEMW {
                        mask: TCIF_TEXT,
                        pszText: name.as_mut_ptr(),
                        ..zeroed()
                    };
                    SendMessageW(
                        self.tab,
                        TCM_INSERTITEMW,
                        n,
                        &mut item as *mut TCITEMW as isize,
                    );
                }
                MoveWindow(
                    self.tab,
                    scale(self.dpi, 12),
                    scale(self.dpi, 38),
                    scale(self.dpi, width - 40),
                    scale(self.dpi, 185),
                    1,
                );
                self.edit = child(
                    self.hwnd,
                    w!("EDIT"),
                    &settings.filter.replace('\n', "\r\n"),
                    WS_BORDER
                        | WS_TABSTOP
                        | WS_VSCROLL
                        | ES_MULTILINE as u32
                        | ES_AUTOVSCROLL as u32
                        | ES_WANTRETURN as u32,
                    101,
                    self.font,
                );
                MoveWindow(
                    self.edit,
                    scale(self.dpi, 20),
                    scale(self.dpi, 70),
                    scale(self.dpi, width - 56),
                    scale(self.dpi, 144),
                    1,
                );
                let clear = child(
                    self.hwnd,
                    w!("BUTTON"),
                    "Clear Active",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    103,
                    self.font,
                );
                self.place(clear, 12, height - 70, 95, 25);
                let clear_all = child(
                    self.hwnd,
                    w!("BUTTON"),
                    "Clear All",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    104,
                    self.font,
                );
                self.place(clear_all, 114, height - 70, 80, 25);
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
                    scale(self.dpi, 365),
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
                    scale(self.dpi, 365),
                    scale(self.dpi, 70),
                    scale(self.dpi, 23),
                    1,
                );
                self.controls.push(size);
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
        if let Kind::Filters { settings, selected } = &mut self.kind {
            let text = window_text(self.edit).replace("\r\n", "\n");
            if *selected == 0 {
                settings.filter = text;
            } else {
                settings.presets[*selected - 1] = text;
            }
        }
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
                let values: Vec<_> = self.controls[..12]
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
                let Ok(size) = window_text(self.controls[12]).parse::<f32>() else {
                    error_box(self.hwnd, "Font size must be a number between 8 and 30.");
                    return;
                };
                if !size.is_finite() || !(8.0..=30.0).contains(&size) {
                    error_box(self.hwnd, "Font size must be a number between 8 and 30.");
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
        WM_COMMAND => match (wp & 0xffff) as i32 {
            IDOK => dialog.accept(),
            IDCANCEL => dialog.result = Some(false),
            103 => {
                if let Kind::Filters { settings, selected } = &mut dialog.kind {
                    settings.filter.clear();
                    if *selected == 0 {
                        set_text(dialog.edit, "");
                    }
                }
            }
            104 => {
                if let Kind::Filters { settings, .. } = &mut dialog.kind {
                    settings.filter.clear();
                    settings.presets = Default::default();
                }
                set_text(dialog.edit, "");
            }
            105 => match file(dialog.hwnd, true, "done.txt") {
                Ok(Some(path)) => set_text(dialog.edit, &path.to_string_lossy()),
                Ok(None) => {}
                Err(e) => error_box(dialog.hwnd, &e.to_string()),
            },
            _ => {}
        },
        WM_NOTIFY => {
            if (*(lp as *const NMHDR)).code == TCN_SELCHANGE {
                dialog.store_filter();
                let n = SendMessageW(dialog.tab, TCM_GETCURSEL, 0, 0).max(0) as usize;
                if let Kind::Filters { settings, selected } = &mut dialog.kind {
                    *selected = n;
                    let text = if n == 0 {
                        settings.filter.clone()
                    } else {
                        settings.presets[n - 1].clone()
                    };
                    set_text(dialog.edit, &text.replace('\n', "\r\n"));
                }
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
    let state = RefCell::new(Dialog {
        hwnd: null_mut(),
        parent,
        font,
        dpi,
        edit: null_mut(),
        tab: null_mut(),
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
) -> io::Result<Option<Settings>> {
    match show(
        parent,
        font,
        dpi,
        Kind::Filters {
            settings: Box::new(settings.clone()),
            selected: 0,
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
