use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

/// Last snow virtual-desktop fingerprint — skip redundant re-emits.
static LAST_SNOW_FINGERPRINT: Mutex<String> = Mutex::new(String::new());
/// Active reminder scene for this session.
static CURRENT_SCENE: Mutex<String> = Mutex::new(String::new());

pub const DESKTOP_PET_SIZE: u32 = 88;
const EDGE_SNAP_PX: i32 = 28;

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

fn pet_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("pet")
        .ok_or_else(|| "找不到 pet 窗口".to_string())
}

/// Place desktop pet near the cursor on its monitor (bottom-right of work area by default).
pub fn place_desktop_pet_default(app: &AppHandle) -> Result<(), String> {
    let pet = pet_window(app)?;
    let (cx, cy) = cursor_pos().unwrap_or((0, 0));
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;
    let mut target = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "无显示器".to_string())?;

    for m in &monitors {
        let pos = m.position();
        let size = m.size();
        let left = pos.x;
        let top = pos.y;
        let right = left + size.width as i32;
        let bottom = top + size.height as i32;
        if cx >= left && cx < right && cy >= top && cy < bottom {
            target = m.clone();
            break;
        }
    }

    let pos = target.position();
    let size = target.size();
    let x = pos.x + size.width as i32 - DESKTOP_PET_SIZE as i32 - 24;
    let y = pos.y + size.height as i32 - DESKTOP_PET_SIZE as i32 - 72;
    pet.set_size(PhysicalSize::new(DESKTOP_PET_SIZE, DESKTOP_PET_SIZE))
        .map_err(|e| e.to_string())?;
    pet.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    pet.set_ignore_cursor_events(false)
        .map_err(|e| e.to_string())?;
    pet.set_always_on_top(true).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn show_desktop_pet(app: &AppHandle) -> Result<(), String> {
    ensure_pet_window(app)?;
    place_desktop_pet_default(app)?;
    let pet = pet_window(app)?;
    pet.show().map_err(|e| e.to_string())?;
    let _ = app.emit("desktop-pet-event", serde_json::json!({ "visible": true }));
    Ok(())
}

pub fn hide_desktop_pet(app: &AppHandle) -> Result<(), String> {
    if let Some(pet) = app.get_webview_window("pet") {
        pet.hide().map_err(|e| e.to_string())?;
    }
    let _ = app.emit(
        "desktop-pet-event",
        serde_json::json!({ "visible": false }),
    );
    Ok(())
}

pub fn set_desktop_pet_position(app: &AppHandle, x: i32, y: i32) -> Result<(), String> {
    let pet = pet_window(app)?;
    pet.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Snap pet to nearest screen edge if within threshold; returns dock side for peeping pose.
pub fn snap_desktop_pet(app: &AppHandle) -> Result<serde_json::Value, String> {
    let pet = pet_window(app)?;
    let pos = pet.outer_position().map_err(|e| e.to_string())?;
    let size = pet.outer_size().map_err(|e| e.to_string())?;
    let monitors = app.available_monitors().map_err(|e| e.to_string())?;

    let cx = pos.x + size.width as i32 / 2;
    let cy = pos.y + size.height as i32 / 2;
    let mut target = app
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "无显示器".to_string())?;
    for m in &monitors {
        let mp = m.position();
        let ms = m.size();
        if cx >= mp.x
            && cx < mp.x + ms.width as i32
            && cy >= mp.y
            && cy < mp.y + ms.height as i32
        {
            target = m.clone();
            break;
        }
    }

    let mp = target.position();
    let ms = target.size();
    let left = mp.x;
    let top = mp.y;
    let right = left + ms.width as i32;
    let bottom = top + ms.height as i32;
    let pet_w = size.width as i32;
    let pet_h = size.height as i32;

    let dist_l = (pos.x - left).abs();
    let dist_r = ((pos.x + pet_w) - right).abs();
    let dist_t = (pos.y - top).abs();
    let dist_b = ((pos.y + pet_h) - bottom).abs();

    let mut dock = "none";
    let mut nx = pos.x;
    let mut ny = pos.y;
    let min_d = dist_l.min(dist_r).min(dist_t).min(dist_b);
    if min_d <= EDGE_SNAP_PX {
        if min_d == dist_l {
            dock = "left";
            nx = left - pet_w / 2;
            ny = pos.y.clamp(top, bottom - pet_h);
        } else if min_d == dist_r {
            dock = "right";
            nx = right - pet_w / 2;
            ny = pos.y.clamp(top, bottom - pet_h);
        } else if min_d == dist_t {
            dock = "top";
            ny = top - pet_h / 2;
            nx = pos.x.clamp(left, right - pet_w);
        } else {
            dock = "bottom";
            ny = bottom - pet_h / 2;
            nx = pos.x.clamp(left, right - pet_w);
        }
        pet.set_position(PhysicalPosition::new(nx, ny))
            .map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!({
        "dock": dock,
        "x": nx,
        "y": ny,
    }))
}

fn ensure_pet_window(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("pet").is_some() {
        return Ok(());
    }
    let url = WebviewUrl::App("index.html".into());
    WebviewWindowBuilder::new(app, "pet", url)
        .title("")
        .inner_size(DESKTOP_PET_SIZE as f64, DESKTOP_PET_SIZE as f64)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .shadow(false)
        .focused(false)
        .resizable(false)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Cover the full virtual desktop (all monitors) for the snowfall layer.
struct SnowLayout {
    min_x: i32,
    min_y: i32,
    width: u32,
    height: u32,
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

pub fn refresh_snow_layout(app: &AppHandle) -> Result<(), String> {
    if !snow_window_visible(app) {
        return Ok(());
    }
    let layout = position_snow_all_monitors(app)?;
    emit_snow_active(app, &layout);
    Ok(())
}

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
        "rainStreak" => "rainStreak",
        "blinds" => "blinds",
        "waterFlow" => "waterFlow",
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
    snow.set_always_on_top(true).map_err(|e| e.to_string())?;
    emit_snow_active(app, &layout);

    // 前端可能尚未挂上监听：多次补发，避免丢首帧事件导致「有状态无景致」
    let app_delayed = app.clone();
    thread::spawn(move || {
        let mut elapsed = 0u64;
        for at_ms in [80u64, 200, 500, 1200] {
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

fn attach_settings_close_destroy(window: &WebviewWindow) {
    let label = window.label().to_string();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            // allow destroy — window will be recreated on next open
            let _ = label.as_str();
        }
    });
}

pub fn show_settings(app: &AppHandle, tab: &str) -> Result<(), String> {
    let tab = match tab {
        "settings" => "settings",
        _ => "today",
    };

    if app.get_webview_window("settings").is_none() {
        let url = WebviewUrl::App("settings.html".into());
        let window = WebviewWindowBuilder::new(app, "settings", url)
            .title("设置")
            .inner_size(520.0, 720.0)
            .resizable(false)
            .visible(false)
            .center()
            .build()
            .map_err(|e| e.to_string())?;
        attach_settings_close_destroy(&window);
    }

    let w = app
        .get_webview_window("settings")
        .ok_or_else(|| "找不到 settings 窗口".to_string())?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    let _ = app.emit("main-ui-open", serde_json::json!({ "tab": tab }));
    Ok(())
}
