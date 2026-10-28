use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

/// Last snow virtual-desktop fingerprint — skip redundant re-emits.
static LAST_SNOW_FINGERPRINT: Mutex<String> = Mutex::new(String::new());
/// Active reminder scene for this session (`snow` | `rainGlass`).
static CURRENT_SCENE: Mutex<String> = Mutex::new(String::new());

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

/// Physical layout of the snow overlay relative to the virtual desktop origin.
struct SnowLayout {
    min_x: i32,
    min_y: i32,
    width: u32,
    height: u32,
    /// Monitor rects in physical pixels, relative to (min_x, min_y).
    monitors: Vec<serde_json::Value>,
}

fn snow_layout_fingerprint(layout: &SnowLayout) -> String {
    format!(
        "{}:{}:{}:{}:{:?}",
        layout.min_x, layout.min_y, layout.width, layout.height, layout.monitors
    )
}

fn snow_layout(app: &AppHandle) -> Result<SnowLayout, String> {
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
    let monitor_rects = monitors
        .iter()
        .map(|m| {
            let pos = m.position();
            let size = m.size();
            serde_json::json!({
                "x": pos.x - min_x,
                "y": pos.y - min_y,
                "width": size.width,
                "height": size.height,
            })
        })
        .collect();

    Ok(SnowLayout {
        min_x,
        min_y,
        width,
        height,
        monitors: monitor_rects,
    })
}

fn current_scene() -> String {
    CURRENT_SCENE
        .lock()
        .map(|s| s.clone())
        .unwrap_or_else(|_| "snow".into())
}

fn emit_snow_active(app: &AppHandle, layout: &SnowLayout) {
    let fp = snow_layout_fingerprint(layout);
    if let Ok(mut prev) = LAST_SNOW_FINGERPRINT.lock() {
        *prev = fp;
    }
    let scene = current_scene();
    let _ = app.emit(
        "snow-event",
        serde_json::json!({
            "active": true,
            "scene": scene,
            "window": { "width": layout.width, "height": layout.height },
            "monitors": layout.monitors,
        }),
    );
}

/// Cover the full virtual desktop (all monitors) for the snowfall layer.
fn position_snow_all_monitors(app: &AppHandle) -> Result<SnowLayout, String> {
    let snow = app
        .get_webview_window("snow")
        .ok_or_else(|| "找不到 snow 窗口".to_string())?;
    let layout = snow_layout(app)?;
    snow.set_position(PhysicalPosition::new(layout.min_x, layout.min_y))
        .map_err(|e| e.to_string())?;
    snow.set_size(PhysicalSize::new(layout.width, layout.height))
        .map_err(|e| e.to_string())?;
    snow.set_ignore_cursor_events(true)
        .map_err(|e| e.to_string())?;
    snow.set_always_on_top(true).map_err(|e| e.to_string())?;
    Ok(layout)
}

fn snow_window_visible(app: &AppHandle) -> bool {
    app.get_webview_window("snow")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

/// Re-query monitors, resize/reposition the snow window, and notify the WebView.
/// Used after RDP / display topology changes so pile bottoms stay on each screen edge.
pub fn refresh_snow_layout(app: &AppHandle) -> Result<(), String> {
    if !snow_window_visible(app) {
        return Ok(());
    }
    let layout = position_snow_all_monitors(app)?;
    emit_snow_active(app, &layout);
    Ok(())
}

/// Like refresh_snow_layout, but skips emit when geometry fingerprint is unchanged.
pub fn refresh_snow_layout_if_changed(app: &AppHandle) -> Result<(), String> {
    if !snow_window_visible(app) {
        return Ok(());
    }
    let layout = snow_layout(app)?;
    let fp = snow_layout_fingerprint(&layout);
    let unchanged = LAST_SNOW_FINGERPRINT
        .lock()
        .map(|prev| *prev == fp)
        .unwrap_or(false);
    if unchanged {
        return Ok(());
    }
    let layout = position_snow_all_monitors(app)?;
    emit_snow_active(app, &layout);
    Ok(())
}

pub fn show_snow(app: &AppHandle, scene: &str) -> Result<(), String> {
    let scene = match scene {
        "rainGlass" => "rainGlass",
        _ => "snow",
    };
    if let Ok(mut cur) = CURRENT_SCENE.lock() {
        *cur = scene.to_string();
    }
    if let Ok(mut prev) = LAST_SNOW_FINGERPRINT.lock() {
        prev.clear();
    }
    let layout = position_snow_all_monitors(app)?;
    let snow = app
        .get_webview_window("snow")
        .ok_or_else(|| "找不到 snow 窗口".to_string())?;
    snow.show().map_err(|e| e.to_string())?;
    emit_snow_active(app, &layout);

    // RDP / DPI: WebView often still reports the old innerWidth/Height when the first
    // event arrives. Re-apply layout a few times after the OS finishes resizing.
    let app_delayed = app.clone();
    thread::spawn(move || {
        let mut elapsed = 0u64;
        for at_ms in [80u64, 200, 500] {
            thread::sleep(Duration::from_millis(at_ms.saturating_sub(elapsed)));
            elapsed = at_ms;
            let _ = refresh_snow_layout(&app_delayed);
        }
    });
    Ok(())
}

pub fn hide_snow(app: &AppHandle) -> Result<(), String> {
    if let Ok(mut prev) = LAST_SNOW_FINGERPRINT.lock() {
        prev.clear();
    }
    if let Ok(mut cur) = CURRENT_SCENE.lock() {
        cur.clear();
    }
    let _ = app.emit("snow-event", serde_json::json!({ "active": false }));
    if let Some(snow) = app.get_webview_window("snow") {
        snow.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

const SUMMARY_W: f64 = 320.0;
const SUMMARY_H: f64 = 168.0;
const SUMMARY_MARGIN_DIP: f64 = 8.0;

fn place_summary_on_primary(summary: &WebviewWindow, monitor: &tauri::Monitor) -> Result<(), String> {
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let width = (SUMMARY_W * scale).round().max(1.0) as u32;
    let height = (SUMMARY_H * scale).round().max(1.0) as u32;
    let margin = (SUMMARY_MARGIN_DIP * scale).round() as i32;
    let x = area.position.x + area.size.width as i32 - width as i32 - margin;
    let y = area.position.y + area.size.height as i32 - height as i32 - margin;
    summary
        .set_size(PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    summary
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    summary
        .set_always_on_top(true)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn show_summary(app: &AppHandle) -> Result<(), String> {
    let summary = app
        .get_webview_window("summary")
        .ok_or_else(|| "找不到 summary 窗口".to_string())?;
    let monitor = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "无显示器".to_string())?;
    place_summary_on_primary(&summary, &monitor)?;
    summary.show().map_err(|e| e.to_string())?;
    place_summary_on_primary(&summary, &monitor)?;

    let app_delayed = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(80));
        let Some(summary) = app_delayed.get_webview_window("summary") else {
            return;
        };
        let Ok(Some(monitor)) = app_delayed.primary_monitor() else {
            return;
        };
        let _ = place_summary_on_primary(&summary, &monitor);
    });
    Ok(())
}

pub fn hide_summary(app: &AppHandle) -> Result<(), String> {
    if let Some(summary) = app.get_webview_window("summary") {
        summary.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn show_settings(app: &AppHandle, tab: &str) -> Result<(), String> {
    let tab = match tab {
        "settings" => "settings",
        _ => "today",
    };
    let w = app
        .get_webview_window("settings")
        .ok_or_else(|| "找不到 settings 窗口".to_string())?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    let _ = app.emit("main-ui-open", serde_json::json!({ "tab": tab }));
    Ok(())
}
