mod character_pack;
mod config;
mod daily_stats;
mod input_hook;
mod overlay;
mod session_lock;
mod timer;

use character_pack::{import_pack_from_folder, list_pack_names, load_pack, seed_default_pack};
use config::{load_config, normalize_reminder_scene, save_config, AppConfig};
use daily_stats::{DailyRestStats, SummaryPayload};
use input_hook::start_input_hook;
use overlay::{
    hide_desktop_pet, hide_snow, hide_summary, refresh_snow_layout, refresh_snow_layout_if_changed,
    set_desktop_pet_position, show_desktop_pet, show_settings, show_snow, show_summary,
    snap_desktop_pet,
};
use session_lock::start_session_watch;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use timer::{HoldReason, PetEvent, Phase, PhaseTransition, TimerState};

struct AppState {
    timer: Arc<Mutex<TimerState>>,
    stats: Mutex<DailyRestStats>,
    was_paused_before_reminder: Mutex<bool>,
    /// User enabled desktop pet, but memory guard forced it off until re-enabled.
    desktop_pet_forced_off: Mutex<bool>,
}

pub(crate) fn apply_session_hold(app: &AppHandle, reason: HoldReason, active: bool) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let changed = {
        let Ok(mut timer) = state.timer.lock() else {
            return;
        };
        timer.set_hold(reason, active)
    };
    if let Some(held) = changed {
        let _ = app.emit("session-hold", serde_json::json!({ "held": held }));
    }
}

fn emit_pet(app: &AppHandle, event: &PetEvent) {
    let _ = app.emit("pet-event", event);
}

fn do_pause(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let _ = app;
    let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
    timer.pause();
    Ok(())
}

fn resolve_reminder_scene(pref: &str) -> String {
    match normalize_reminder_scene(pref).as_str() {
        "snow" => "snow".into(),
        "rainGlass" => "rainGlass".into(),
        "rainStreak" => "rainStreak".into(),
        "blinds" => "blinds".into(),
        "waterFlow" => "waterFlow".into(),
        _ => {
            let n = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0);
            match n % 5 {
                0 => "snow".into(),
                1 => "rainGlass".into(),
                2 => "rainStreak".into(),
                3 => "blinds".into(),
                _ => "waterFlow".into(),
            }
        }
    }
}

fn begin_reminder(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let was_paused = {
        let timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.phase == Phase::Paused
    };
    *state
        .was_paused_before_reminder
        .lock()
        .map_err(|e| e.to_string())? = was_paused;

    // 允许在已提醒时再次调用（立即提醒）：先回到 Reminding 并重铺景致。
    // 景致成功前不进入 Observing，避免「状态已是观察中但全屏层失败」后永远不再提醒。
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.start_reminding();
        let ev = timer.to_event(None);
        emit_pet(app, &ev);
    }

    let scene = resolve_reminder_scene(&load_config().reminder_scene);
    if let Err(err) = show_snow(app, &scene) {
        eprintln!("show_snow failed: {err}");
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.dismiss_to_working(was_paused);
        let ev = timer.to_event(None);
        emit_pet(app, &ev);
        return Err(err);
    }

    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        if timer.phase == Phase::Reminding {
            timer.enter_observing();
            let ev = timer.to_event(None);
            emit_pet(app, &ev);
        }
    }
    Ok(())
}

fn finish_exit(app: &AppHandle, state: &AppState) -> Result<(), String> {
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        if let Some(delay_secs) = timer.claim_incomplete() {
            let mut stats = state.stats.lock().map_err(|e| e.to_string())?;
            stats.record_incomplete(delay_secs);
        }
    }
    let was_paused = *state
        .was_paused_before_reminder
        .lock()
        .map_err(|e| e.to_string())?;
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.dismiss_to_working(was_paused);
        let ev = timer.to_event(None);
        emit_pet(app, &ev);
    }
    hide_snow(app)?;
    Ok(())
}

fn complete_happy_exit(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let payload = {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        let delay_secs = timer.claim_success_delay();
        drop(timer);
        if let Some(secs) = delay_secs {
            let mut stats = state.stats.lock().map_err(|e| e.to_string())?;
            Some(stats.record_success(secs))
        } else {
            None
        }
    };
    let was_paused = *state
        .was_paused_before_reminder
        .lock()
        .map_err(|e| e.to_string())?;
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        // claim_success_delay already cleared reminder_active
        timer.dismiss_to_working(was_paused);
        let ev = timer.to_event(None);
        emit_pet(app, &ev);
    }
    hide_snow(app)?;
    if let Some(payload) = payload {
        present_summary(app, &payload)?;
    }
    Ok(())
}

fn present_summary(app: &AppHandle, payload: &SummaryPayload) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("summary") {
        let _ = window.emit("summary-event", payload);
    }
    show_summary(app)
}

fn apply_desktop_pet_visibility(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let cfg = load_config();
    let forced = *state
        .desktop_pet_forced_off
        .lock()
        .map_err(|e| e.to_string())?;
    if cfg.desktop_pet_enabled && !forced {
        show_desktop_pet(app)
    } else {
        hide_desktop_pet(app)
    }
}

fn system_memory_percent() -> Option<f32> {
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::{
            GlobalMemoryStatusEx, MEMORYSTATUSEX,
        };
        unsafe {
            let mut status = MEMORYSTATUSEX {
                dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
                ..Default::default()
            };
            if GlobalMemoryStatusEx(&mut status).is_ok() {
                return Some(status.dwMemoryLoad as f32);
            }
        }
        None
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn start_memory_guard(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(5));
        let cfg = load_config();
        if !cfg.desktop_pet_memory_guard || !cfg.desktop_pet_enabled {
            continue;
        }
        let Some(pct) = system_memory_percent() else {
            continue;
        };
        if pct < cfg.desktop_pet_memory_threshold as f32 {
            continue;
        }
        let Some(state) = app.try_state::<AppState>() else {
            continue;
        };
        {
            let Ok(mut forced) = state.desktop_pet_forced_off.lock() else {
                continue;
            };
            if *forced {
                continue;
            }
            *forced = true;
        }
        let _ = hide_desktop_pet(&app);
        let _ = app.emit(
            "desktop-pet-memory-closed",
            serde_json::json!({ "memoryPercent": pct }),
        );
        // Persist disabled so restart also respects forced-off until user re-enables
        let mut next = cfg;
        next.desktop_pet_enabled = false;
        let _ = save_config(&next);
        let _ = app.emit("config-changed", &next);
    });
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<AppConfig, String> {
    let timer = state.timer.lock().map_err(|e| e.to_string())?;
    let mut cfg = load_config();
    cfg.work_minutes = timer.work_minutes;
    cfg.observation_seconds = timer.observation_seconds;
    cfg.character_pack = timer.character_pack.clone();
    Ok(cfg)
}

#[tauri::command]
fn save_app_config(app: AppHandle, state: State<AppState>, cfg: AppConfig) -> Result<(), String> {
    let mut cfg = cfg;
    cfg.work_minutes = cfg.work_minutes.clamp(1, 180);
    cfg.observation_seconds = cfg.observation_seconds.clamp(10, 120);
    cfg.display_max_px = cfg.display_max_px.clamp(48, 192);
    cfg.desktop_pet_memory_threshold = cfg.desktop_pet_memory_threshold.clamp(50, 99);
    let existing = load_config();
    cfg.snow_seconds = existing.snow_seconds.clamp(30, 600);
    if cfg.reminder_scene.is_empty() {
        cfg.reminder_scene = existing.reminder_scene;
    }
    cfg.reminder_scene = normalize_reminder_scene(&cfg.reminder_scene);
    if cfg.character_pack.is_empty() {
        cfg.character_pack = existing.character_pack;
    }
    if cfg
        .wanxiang
        .endpoint
        .as_ref()
        .map(|s| s.is_empty())
        .unwrap_or(true)
    {
        cfg.wanxiang.endpoint = existing.wanxiang.endpoint;
        if cfg.wanxiang.endpoint.is_none() {
            cfg.wanxiang.endpoint = config::WanxiangConfig::default().endpoint;
        }
    }
    save_config(&cfg)?;
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.apply_config(
            cfg.work_minutes,
            cfg.observation_seconds,
            cfg.character_pack.clone(),
        );
    }
    if cfg.desktop_pet_enabled {
        *state
            .desktop_pet_forced_off
            .lock()
            .map_err(|e| e.to_string())? = false;
    }
    let _ = sync_autostart(&app, cfg.autostart);
    apply_desktop_pet_visibility(&app, &state)?;
    let _ = app.emit("config-changed", &cfg);
    Ok(())
}

#[tauri::command]
fn list_packs() -> Result<Vec<String>, String> {
    list_pack_names()
}

#[tauri::command]
fn get_pack(name: String) -> Result<character_pack::CharacterPackDto, String> {
    load_pack(&name)
}

#[tauri::command]
fn import_pack(path: String) -> Result<String, String> {
    import_pack_from_folder(std::path::Path::new(&path))
}

#[tauri::command]
fn pause_timer(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    do_pause(&app, &state)
}

#[tauri::command]
fn resume_timer(state: State<AppState>) -> Result<(), String> {
    let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
    timer.resume();
    Ok(())
}

#[tauri::command]
fn dismiss_reminder(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    finish_exit(&app, &state)
}

#[tauri::command]
fn hide_summary_cmd(app: AppHandle) -> Result<(), String> {
    hide_summary(&app)
}

#[tauri::command]
fn remind_now(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    begin_reminder(&app, &state)
}

#[tauri::command]
fn notify_action_done(
    _app: AppHandle,
    _state: State<AppState>,
    _action: String,
) -> Result<(), String> {
    // Character actions removed; kept for API compatibility
    Ok(())
}

#[tauri::command]
fn get_status(state: State<AppState>) -> Result<PetEvent, String> {
    let timer = state.timer.lock().map_err(|e| e.to_string())?;
    Ok(timer.to_event(None))
}

#[tauri::command]
fn refresh_snow_layout_cmd(app: AppHandle) -> Result<(), String> {
    refresh_snow_layout(&app)
}

#[tauri::command]
fn get_daily_stats(state: State<AppState>) -> Result<DailyRestStats, String> {
    let mut stats = state.stats.lock().map_err(|e| e.to_string())?;
    stats.ensure_today();
    Ok(stats.clone())
}

#[tauri::command]
fn open_main_ui(app: AppHandle, tab: Option<String>) -> Result<(), String> {
    let tab = tab.unwrap_or_else(|| "today".into());
    show_settings(&app, &tab)
}

#[tauri::command]
fn set_pet_position(app: AppHandle, x: i32, y: i32) -> Result<(), String> {
    set_desktop_pet_position(&app, x, y)
}

#[tauri::command]
fn snap_pet(app: AppHandle) -> Result<serde_json::Value, String> {
    snap_desktop_pet(&app)
}

#[tauri::command]
fn get_pet_screen_pos(app: AppHandle) -> Result<serde_json::Value, String> {
    let pet = app
        .get_webview_window("pet")
        .ok_or_else(|| "找不到 pet 窗口".to_string())?;
    let pos = pet.outer_position().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "x": pos.x, "y": pos.y }))
}

fn sync_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show_settings_i = MenuItem::with_id(app, "settings", "打开主界面", true, None::<&str>)?;
    let pause_i = MenuItem::with_id(app, "pause", "暂停计时", true, None::<&str>)?;
    let resume_i = MenuItem::with_id(app, "resume", "继续计时", true, None::<&str>)?;
    let remind_i = MenuItem::with_id(app, "remind", "立即提醒", true, None::<&str>)?;
    let dismiss_i = MenuItem::with_id(app, "dismiss", "知道了", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show_settings_i,
            &pause_i,
            &resume_i,
            &remind_i,
            &dismiss_i,
            &quit_i,
        ],
    )?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Rest Reminder Pet")
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            match event.id.as_ref() {
                "settings" => {
                    let _ = show_settings(app, "settings");
                }
                "pause" => {
                    let _ = do_pause(app, state.inner());
                }
                "resume" => {
                    if let Ok(mut t) = state.timer.lock() {
                        t.resume();
                    }
                }
                "remind" => {
                    let _ = begin_reminder(app, state.inner());
                }
                "dismiss" => {
                    let _ = finish_exit(app, state.inner());
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let _ = show_settings(app, "today");
            }
        })
        .build(app)?;
    Ok(())
}

fn start_tick_loop(app: AppHandle, state_timer: Arc<Mutex<TimerState>>) {
    thread::spawn(move || {
        let mut display_check_ticks: u8 = 0;
        loop {
            thread::sleep(Duration::from_millis(250));
            let transition = {
                let Ok(mut timer) = state_timer.lock() else {
                    continue;
                };
                timer.tick()
            };
            match transition {
                Some(PhaseTransition::EnterReminding) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Err(err) = begin_reminder(&app, state.inner()) {
                            eprintln!("begin_reminder failed: {err}");
                        }
                    }
                }
                Some(PhaseTransition::EnterObserving) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Ok(mut timer) = state.timer.lock() {
                            if timer.phase == Phase::Reminding {
                                timer.enter_observing();
                                let ev = timer.to_event(None);
                                drop(timer);
                                emit_pet(&app, &ev);
                            }
                        }
                    }
                }
                Some(PhaseTransition::EnterHappyExit) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        if let Err(err) = complete_happy_exit(&app, state.inner()) {
                            eprintln!("complete_happy_exit failed: {err}");
                        }
                    }
                }
                None => {}
            }

            display_check_ticks = display_check_ticks.wrapping_add(1);
            if display_check_ticks % 4 == 0 {
                let _ = refresh_snow_layout_if_changed(&app);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = load_config();
    let timer = Arc::new(Mutex::new(TimerState::new(
        cfg.work_minutes,
        cfg.observation_seconds,
        cfg.character_pack.clone(),
    )));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .manage(AppState {
            timer: Arc::clone(&timer),
            stats: Mutex::new(DailyRestStats::load()),
            was_paused_before_reminder: Mutex::new(false),
            desktop_pet_forced_off: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_app_config,
            list_packs,
            get_pack,
            import_pack,
            pause_timer,
            resume_timer,
            dismiss_reminder,
            hide_summary_cmd,
            remind_now,
            notify_action_done,
            get_status,
            refresh_snow_layout_cmd,
            get_daily_stats,
            open_main_ui,
            set_pet_position,
            snap_pet,
            get_pet_screen_pos
        ])
        .setup(move |app| {
            let resource_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
            let _ = seed_default_pack(&resource_dir);

            setup_tray(app.handle())?;

            if let Some(snow) = app.get_webview_window("snow") {
                let _ = snow.set_ignore_cursor_events(true);
            }
            if let Some(summary) = app.get_webview_window("summary") {
                let summary_hide = summary.clone();
                summary.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = summary_hide.hide();
                    }
                });
            }
            // settings: allow destroy on close; recreated by show_settings

            let cfg = load_config();
            let _ = sync_autostart(app.handle(), cfg.autostart);

            start_input_hook(app.handle().clone(), Arc::clone(&timer));
            start_session_watch(app.handle().clone());
            start_tick_loop(app.handle().clone(), Arc::clone(&timer));
            start_memory_guard(app.handle().clone());

            if let Some(state) = app.try_state::<AppState>() {
                let _ = apply_desktop_pet_visibility(app.handle(), state.inner());
            }

            let demo = std::env::var("REST_REMINDER_DEMO")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            if demo {
                let demo_app = app.handle().clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(800));
                    if let Some(state) = demo_app.try_state::<AppState>() {
                        let _ = begin_reminder(&demo_app, state.inner());
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Rest Reminder Pet");
}
