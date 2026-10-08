//! Native tray integration. Never hide the only window without a working icon.
use super::*;
use windows_sys::Win32::UI::Shell::*;

pub(super) const TRAY_MESSAGE: u32 = WM_APP + 2;
pub(super) const RELAYOUT: u32 = WM_APP + 3;
pub(super) const HOTKEY_ID: i32 = 1;

pub(super) struct Integration {
    hwnd: HWND,
    pub active: bool,
    hotkey: bool,
    pub explorer_restart: u32,
}
impl Default for Integration {
    fn default() -> Self {
        Self {
            hwnd: null_mut(),
            active: false,
            hotkey: false,
            explorer_restart: 0,
        }
    }
}
impl Integration {
    unsafe fn icon(&self) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = zeroed();
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = self.hwnd;
        data.uID = 1;
        data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP;
        data.uCallbackMessage = TRAY_MESSAGE;
        data.hIcon = LoadIconW(
            GetModuleHandleW(null()),
            std::ptr::without_provenance::<u16>(1),
        );
        let tip = wide("todotxt.rs");
        data.szTip[..tip.len()].copy_from_slice(&tip);
        data
    }
    pub unsafe fn configure(&mut self, hwnd: HWND, enabled: bool) -> io::Result<()> {
        self.hwnd = hwnd;
        self.explorer_restart = RegisterWindowMessageW(w!("TaskbarCreated"));
        if !enabled {
            self.remove();
            return Ok(());
        }
        if !self.active {
            if Shell_NotifyIconW(NIM_ADD, &self.icon()) == 0 {
                return Err(io::Error::other(
                    "Could not create the tray icon. The window will remain accessible on the taskbar.",
                ));
            }
            self.active = true;
            self.set_callback_version();
        }
        if !self.hotkey {
            if RegisterHotKey(
                hwnd,
                HOTKEY_ID,
                MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                b'M' as u32,
            ) == 0
            {
                return Err(io::Error::other(
                    "Ctrl+Alt+M is already in use or could not be registered. The tray icon still works. Close todotxt.net or the other hotkey owner before enabling this shortcut.",
                ));
            }
            self.hotkey = true;
        }
        Ok(())
    }
    unsafe fn set_callback_version(&self) {
        let mut icon = self.icon();
        icon.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        // A failure leaves legacy callbacks usable; the window handles both layouts.
        Shell_NotifyIconW(NIM_SETVERSION, &icon);
    }
    pub unsafe fn recreate_icon(&mut self) {
        if self.active {
            if Shell_NotifyIconW(NIM_ADD, &self.icon()) != 0 {
                self.set_callback_version();
            } else {
                self.active = false;
                ShowWindow(self.hwnd, SW_SHOWNORMAL);
                PostMessageW(self.hwnd, RELAYOUT, 0, 0);
            }
        }
    }
    pub unsafe fn toggle(&self) {
        if IsIconic(self.hwnd) != 0 || IsWindowVisible(self.hwnd) == 0 {
            ShowWindow(self.hwnd, SW_SHOWNORMAL);
            SetForegroundWindow(self.hwnd);
            PostMessageW(self.hwnd, RELAYOUT, 0, 0);
        } else if self.active {
            ShowWindow(self.hwnd, SW_MINIMIZE);
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }
    pub unsafe fn remove(&mut self) {
        if self.hotkey {
            UnregisterHotKey(self.hwnd, HOTKEY_ID);
            self.hotkey = false;
        }
        if self.active {
            Shell_NotifyIconW(NIM_DELETE, &self.icon());
            self.active = false;
        }
        if IsWindow(self.hwnd) != 0 && IsWindowVisible(self.hwnd) == 0 {
            ShowWindow(self.hwnd, SW_SHOWNORMAL);
            PostMessageW(self.hwnd, RELAYOUT, 0, 0);
        }
    }
}
impl Drop for Integration {
    fn drop(&mut self) {
        unsafe {
            self.remove();
        }
    }
}

pub(super) unsafe fn popup(hwnd: HWND) {
    let menu = CreatePopupMenu();
    AppendMenuW(menu, MF_STRING, EXIT as usize, w!("E&xit"));
    let mut point: POINT = zeroed();
    GetCursorPos(&mut point);
    SetForegroundWindow(hwnd);
    let command = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_RIGHTBUTTON,
        point.x,
        point.y,
        0,
        hwnd,
        null(),
    );
    if command != 0 {
        PostMessageW(hwnd, WM_COMMAND, command as usize, 0);
    }
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
}
