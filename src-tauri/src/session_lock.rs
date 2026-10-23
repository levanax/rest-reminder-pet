use std::sync::Mutex;
use tauri::AppHandle;

use crate::timer::HoldReason;

static SESSION_APP: Mutex<Option<AppHandle>> = Mutex::new(None);

pub fn start_session_watch(app: AppHandle) {
    if let Ok(mut slot) = SESSION_APP.lock() {
        *slot = Some(app.clone());
    }
    #[cfg(windows)]
    {
        std::thread::spawn(move || {
            if let Err(err) = run_windows_watch() {
                eprintln!("session watch failed: {err}");
            }
        });
    }
    #[cfg(not(windows))]
    {
        let _ = app;
    }
}

fn apply(reason: HoldReason, active: bool) {
    let app = SESSION_APP.lock().ok().and_then(|guard| guard.clone());
    if let Some(app) = app {
        crate::apply_session_hold(&app, reason, active);
    }
}

#[cfg(windows)]
fn run_windows_watch() -> Result<(), String> {
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::RemoteDesktop::{
        WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
        TranslateMessage, MSG, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP,
    };

    const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
    const WTS_SESSION_LOCK: u32 = 0x7;
    const WTS_SESSION_UNLOCK: u32 = 0x8;
    const WM_POWERBROADCAST: u32 = 0x0218;
    const PBT_APMSUSPEND: u32 = 0x0004;
    const PBT_APMRESUMESUSPEND: u32 = 0x0007;
    const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let code = wparam.0 as u32;
        match msg {
            WM_WTSSESSION_CHANGE => {
                if code == WTS_SESSION_LOCK {
                    apply(HoldReason::Lock, true);
                } else if code == WTS_SESSION_UNLOCK {
                    apply(HoldReason::Lock, false);
                }
                return LRESULT(0);
            }
            WM_POWERBROADCAST => {
                if code == PBT_APMSUSPEND {
                    apply(HoldReason::Sleep, true);
                } else if code == PBT_APMRESUMESUSPEND || code == PBT_APMRESUMEAUTOMATIC {
                    apply(HoldReason::Sleep, false);
                }
                return LRESULT(1);
            }
            _ => {}
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    unsafe {
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let class_name = w!("RestReminderSessionWatch");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            // Class may already exist if the watch thread restarted; keep going.
        }
        // Top-level (not HWND_MESSAGE) so WM_POWERBROADCAST can arrive.
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name,
            w!("session-watch"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .map_err(|e| e.to_string())?;
        WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION).map_err(|e| e.to_string())?;

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == 0x0012 {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}
