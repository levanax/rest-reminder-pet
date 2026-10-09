use std::thread;
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

const PET_LABEL: &str = "pet";
const SNOW_LABEL: &str = "snow";
const SETTINGS_LABEL: &str = "settings";

#[cfg(windows)]
fn cursor_pos() -> Option<(i32, i32)> {
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
fn cursor_pos() -> Option<(i32, i32)> {
    None
}

fn close_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.close();
    }
}

fn wait_webview_ready(newly_created: bool) {
    if newly_created {
        // 给前端脚本挂事件监听的时间；销毁重建后必等
        thread::sleep(Duration::from_millis(450));
    }
}

fn ensure_pet(app: &AppHandle) -> Result<(WebviewWindow, bool), String> {
    if let Some(w) = app.get_webview_window(PET_LABEL) {
        return Ok((w, false));
    }
    let w = WebviewWindowBuilder::new(app, PET_LABEL, WebviewUrl::App("index.html".into()))
        .title("Rest Reminder Pet")
        .inner_size(800.0, 600.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .shadow(false)
        .focused(false)
        .build()
        .map_err(|e| format!("创建 pet 窗口失败: {e}"))?;
    let _ = w.set_ignore_cursor_events(true);
    Ok((w, true))
}

fn ensure_snow(app: &AppHandle) -> Result<(WebviewWindow, bool), String> {
    if let Some(w) = app.get_webview_window(SNOW_LABEL) {
        return Ok((w, false));
    }
    let w = WebviewWindowBuilder::new(app, SNOW_LABEL, WebviewUrl::App("snow.html".into()))
        .title("Rest Reminder Pet — Snow")
        .inner_size(800.0, 600.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .shadow(false)
        .focused(false)
        .build()
        .map_err(|e| format!("创建 snow 窗口失败: {e}"))?;
    let _ = w.set_ignore_cursor_events(true);
    Ok((w, true))
}

fn ensure_settings(app: &AppHandle) -> Result<(WebviewWindow, bool), String> {
    if let Some(w) = app.get_webview_window(SETTINGS_LABEL) {
        return Ok((w, false));
    }
    let w = WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("settings.html".into()))
        .title("Rest Reminder Pet — Settings")
        .inner_size(440.0, 560.0)
        .resizable(false)
        .center()
        .visible(false)
        .build()
        .map_err(|e| format!("创建 settings 窗口失败: {e}"))?;
    Ok((w, true))
}

pub fn position_pet_on_mouse_monitor(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window(PET_LABEL)
        .ok_or_else(|| "找不到 pet 窗口".to_string())?;

    let (cx, cy) = cursor_pos().unwrap_or((0, 0));
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let mut target = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "无显示器".to_string())?;

    for m in monitors {
        let pos = m.position();
        let size = m.size();
        let left = pos.x;
        let top = pos.y;
        let right = left + size.width as i32;
        let bottom = top + size.height as i32;
        if cx >= left && cx < right && cy >= top && cy < bottom {
            target = m;
            break;
        }
    }

    let pos = target.position();
    let size = target.size();
    pet.set_position(PhysicalPosition::new(pos.x, pos.y))
        .map_err(|e| e.to_string())?;
    pet.set_size(PhysicalSize::new(size.width, size.height))
        .map_err(|e| e.to_string())?;
    pet.set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    pet.set_always_on_top(true).map_err(|e| e.to_string())?;
    Ok(())
}

/// Cover the full virtual desktop (all monitors) for the snowfall layer.
pub fn position_snow_all_monitors(app: &AppHandle) -> Result<(), String> {
    let snow = app
        .get_webview_window(SNOW_LABEL)
        .ok_or_else(|| "找不到 snow 窗口".to_string())?;
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    if monitors.is_empty() {
        return Err("无显示器".into());
    }

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for m in &monitors {
        let pos = m.position();
        let size = m.size();
        min_x = min_x.min(pos.x);
        min_y = min_y.min(pos.y);
        max_x = max_x.max(pos.x + size.width as i32);
        max_y = max_y.max(pos.y + size.height as i32);
    }

    let width = (max_x - min_x).max(1) as u32;
    let height = (max_y - min_y).max(1) as u32;
    snow.set_position(PhysicalPosition::new(min_x, min_y))
        .map_err(|e| e.to_string())?;
    snow.set_size(PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    snow.set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    snow.set_always_on_top(true).map_err(|e| e.to_string())?;
    Ok(())
}

/// 创建（如需）并显示 pet；返回是否新建（调用方可据此等待前端就绪）。
pub fn show_pet(app: &AppHandle) -> Result<bool, String> {
    let (pet, newly) = ensure_pet(app)?;
    position_pet_on_mouse_monitor(app)?;
    pet.show().map_err(|e| e.to_string())?;
    wait_webview_ready(newly);
    Ok(newly)
}

/// 销毁 pet 窗口以释放 WebView2 内存。
pub fn destroy_pet(app: &AppHandle) {
    close_window(app, PET_LABEL);
}

/// 创建（如需）并显示雪景；雪页面加载后会自动开始飘雪。
pub fn show_snow(app: &AppHandle) -> Result<bool, String> {
    let (snow, newly) = ensure_snow(app)?;
    position_snow_all_monitors(app)?;
    snow.show().map_err(|e| e.to_string())?;
    wait_webview_ready(newly);
    // 兼容：若页面已在监听，再补发一次；新建页会在 onload 自启
    let _ = app.emit("snow-event", serde_json::json!({ "active": true }));
    Ok(newly)
}

/// 销毁 snow 窗口以释放内存并停止飘雪。
pub fn destroy_snow(app: &AppHandle) {
    let _ = app.emit("snow-event", serde_json::json!({ "active": false }));
    close_window(app, SNOW_LABEL);
}

pub fn show_settings(app: &AppHandle) -> Result<(), String> {
    let (w, newly) = ensure_settings(app)?;
    wait_webview_ready(newly);
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// 兼容旧名：关闭即销毁。
pub fn hide_pet(app: &AppHandle) {
    destroy_pet(app);
}

pub fn hide_snow(app: &AppHandle) {
    destroy_snow(app);
}
