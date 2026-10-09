//! Win32 virtual-desktop geometry and overlay HWND styling.

#[derive(Debug, Clone, Copy)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectI {
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[cfg(windows)]
pub fn virtual_desktop() -> RectI {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };
    unsafe {
        RectI {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
        }
    }
}

#[cfg(not(windows))]
pub fn virtual_desktop() -> RectI {
    RectI {
        x: 0,
        y: 0,
        w: 1920,
        h: 1080,
    }
}

#[cfg(windows)]
pub fn cursor_pos() -> Option<(i32, i32)> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut pt = POINT::default();
    unsafe {
        if GetCursorPos(&mut pt).is_ok() {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn cursor_pos() -> Option<(i32, i32)> {
    None
}

#[cfg(windows)]
pub fn monitor_at_cursor() -> RectI {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    let (cx, cy) = cursor_pos().unwrap_or((0, 0));
    unsafe {
        let mon = MonitorFromPoint(POINT { x: cx, y: cy }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(mon, &mut info).as_bool() {
            let r = info.rcMonitor;
            return RectI {
                x: r.left,
                y: r.top,
                w: (r.right - r.left).max(1),
                h: (r.bottom - r.top).max(1),
            };
        }
    }
    virtual_desktop()
}

#[cfg(not(windows))]
pub fn monitor_at_cursor() -> RectI {
    virtual_desktop()
}

#[cfg(windows)]
pub fn find_overlay_hwnd(title: &str) -> Option<isize> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    let wide: Vec<u16> = std::ffi::OsStr::new(title)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        match FindWindowW(PCWSTR::null(), PCWSTR(wide.as_ptr())) {
            Ok(hwnd) if !hwnd.0.is_null() => Some(hwnd.0 as isize),
            _ => None,
        }
    }
}

#[cfg(not(windows))]
pub fn find_overlay_hwnd(_title: &str) -> Option<isize> {
    None
}

/// Apply click-through / topmost / toolwindow + magenta color-key once.
#[cfg(windows)]
pub fn apply_overlay_style(hwnd_raw: isize) {
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE, LWA_COLORKEY,
        WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    };

    let hwnd = HWND(hwnd_raw as *mut _);
    unsafe {
        let mut ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        ex |= WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0 | WS_EX_TOOLWINDOW.0;
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex as i32);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0x00FF00FF), 0, LWA_COLORKEY);
    }
}

#[cfg(not(windows))]
pub fn apply_overlay_style(_hwnd_raw: isize) {}

/// Move/resize overlay. Pass `show=false` to park a 1×1 window (idle, low GPU memory).
#[cfg(windows)]
pub fn set_overlay_bounds(hwnd_raw: isize, rect: RectI, show: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_HIDEWINDOW, SWP_SHOWWINDOW,
    };

    let hwnd = HWND(hwnd_raw as *mut _);
    let flags = if show { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW };
    let (x, y, w, h) = if show {
        (rect.x, rect.y, rect.w.max(1), rect.h.max(1))
    } else {
        (-32000, -32000, 1, 1)
    };
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, w, h, flags);
    }
}

#[cfg(not(windows))]
pub fn set_overlay_bounds(_hwnd_raw: isize, _rect: RectI, _show: bool) {}

pub const OVERLAY_TITLE: &str = "RestReminderPetOverlay";
/// Clear color matching LWA_COLORKEY (BGR 0x00FF00FF → RGB magenta).
pub const COLOR_KEY_R: f32 = 1.0;
pub const COLOR_KEY_G: f32 = 0.0;
pub const COLOR_KEY_B: f32 = 1.0;
