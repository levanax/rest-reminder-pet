use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

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

pub fn position_pet_on_mouse_monitor(app: &AppHandle) -> Result<(), String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "找不到 pet 窗口".to_string())?;

    let (cx, cy) = cursor_pos().unwrap_or((0, 0));
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let mut target = app.primary_monitor().map_err(|e| e.to_string())?.ok_or_else(|| "无显示器".to_string())?;

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

pub fn show_pet(app: &AppHandle) -> Result<(), String> {
    position_pet_on_mouse_monitor(app)?;
    let pet = pet_window(app)?;
    pet.show().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn hide_pet(app: &AppHandle) -> Result<(), String> {
    let pet = pet_window(app)?;
    pet.hide().map_err(|e| e.to_string())?;
    Ok(())
}

fn pet_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("pet")
        .ok_or_else(|| "找不到 pet 窗口".to_string())
}

/// Cover the full virtual desktop (all monitors) for the snowfall layer.
pub fn position_snow_all_monitors(app: &AppHandle) -> Result<(), String> {
    let snow = app
        .get_webview_window("snow")
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

pub fn show_snow(app: &AppHandle) -> Result<(), String> {
    position_snow_all_monitors(app)?;
    let snow = app
        .get_webview_window("snow")
        .ok_or_else(|| "找不到 snow 窗口".to_string())?;
    snow.show().map_err(|e| e.to_string())?;
    let _ = app.emit("snow-event", serde_json::json!({ "active": true }));
    Ok(())
}

pub fn hide_snow(app: &AppHandle) -> Result<(), String> {
    let _ = app.emit("snow-event", serde_json::json!({ "active": false }));
    if let Some(snow) = app.get_webview_window("snow") {
        snow.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_settings(app: &AppHandle) -> Result<(), String> {
    let w = app
        .get_webview_window("settings")
        .ok_or_else(|| "找不到 settings 窗口".to_string())?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
