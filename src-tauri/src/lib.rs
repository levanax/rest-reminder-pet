mod character_pack;
mod config;
mod daily_stats;
mod input_hook;
mod overlay;
mod session_lock;
mod timer;

use character_pack::{import_pack_from_folder, list_pack_names, load_pack, seed_default_pack};
use config::{load_config, save_config, AppConfig};
use daily_stats::{DailyRestStats, SummaryPayload};
use input_hook::start_input_hook;
use overlay::{
    hide_pet, hide_snow, hide_summary, refresh_snow_layout, refresh_snow_layout_if_changed,
    show_pet, show_settings, show_snow, show_summary,
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
    let was_sneak = {
        let timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.phase == Phase::SneakPeek
    };
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.pause();
    }
    if was_sneak {
        hide_pet(app)?;
        let timer = state.timer.lock().map_err(|e| e.to_string())?;
        emit_pet(app, &timer.to_event(None));
    }
    Ok(())
}

fn begin_boot_intro(app: &AppHandle, state: &AppState) -> Result<(), String> {
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        if timer.phase != Phase::BootIntro {
            return Ok(());
        }
        timer.start_boot_intro();
        let ev = timer.to_event(Some("jumpUp"));
        emit_pet(app, &ev);
    }
    show_pet(app)?;
    Ok(())
}

fn end_boot_intro(app: &AppHandle, state: &AppState) -> Result<(), String> {
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.end_boot_intro();
        let ev = timer.to_event(None);
        emit_pet(app, &ev);
    }
    hide_pet(app)?;
    Ok(())
}

fn begin_sneak_peek(app: &AppHandle, state: &AppState) -> Result<(), String> {
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        if timer.phase != Phase::Working {
            return Ok(());
        }
        timer.start_sneak_peek();
        let ev = timer.to_event(Some("sneakPeek"));
        emit_pet(app, &ev);
    }
    show_pet(app)?;
    Ok(())
}

fn end_sneak_peek(app: &AppHandle, state: &AppState) -> Result<(), String> {
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        timer.end_sneak_peek();
        let ev = timer.to_event(None);
        emit_pet(app, &ev);
    }
    hide_pet(app)?;
    Ok(())
}

fn begin_reminder(app: &AppHandle, state: &AppState) -> Result<(), String> {
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        if matches!(
            timer.phase,
            Phase::Reminding | Phase::Observing | Phase::HappyExit | Phase::FallExit
        ) {
            return Ok(());
        }
        *state
            .was_paused_before_reminder
            .lock()
            .map_err(|e| e.to_string())? = timer.phase == Phase::Paused;
        // 打断开机入场 / 偷瞧，直接进入正式提醒
        timer.start_reminding();
        let ev = timer.to_event(Some("crawl"));
        emit_pet(app, &ev);
    }
    // 先小猫、再全屏雪：积雪可盖住甚至淹没猫；雪一直下到休息成功或手动「知道了」
    show_pet(app)?;
    show_snow(app)?;
    Ok(())
}

/// 结束提醒：小猫退场并停雪（仅休息成功或托盘「知道了」）。
fn finish_exit(app: &AppHandle, state: &AppState) -> Result<(), String> {
    {
        let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
        if timer.claim_incomplete() {
            let mut stats = state.stats.lock().map_err(|e| e.to_string())?;
            stats.record_incomplete();
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
    hide_pet(app)?;
    hide_snow(app)?;
    Ok(())
}

fn present_summary(app: &AppHandle, payload: &SummaryPayload) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("summary") {
        let _ = window.emit("summary-event", payload);
    }
    show_summary(app)
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
    let existing = load_config();
    // Settings UI does not edit snow; keep whatever is already on disk
    cfg.snow_seconds = existing.snow_seconds.clamp(30, 600);
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
    let _ = sync_autostart(&app, cfg.autostart);
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
    app: AppHandle,
    state: State<AppState>,
    action: String,
) -> Result<(), String> {
    let mut timer = state.timer.lock().map_err(|e| e.to_string())?;
    match (timer.phase, action.as_str()) {
        (Phase::BootIntro, "jumpUp") => {
            drop(timer);
            end_boot_intro(&app, &state)?;
        }
        (Phase::SneakPeek, "sneakPeek") => {
            drop(timer);
            end_sneak_peek(&app, &state)?;
        }
        (Phase::Reminding, "crawl") => {
            let ev = timer.to_event(Some("anxiousHop"));
            drop(timer);
            emit_pet(&app, &ev);
        }
        (Phase::Reminding, "anxiousHop") => {
            timer.enter_observing();
            let ev = timer.to_event(Some("anxiousHop"));
            emit_pet(&app, &ev);
        }
        (Phase::HappyExit, _) => {
            let delay_secs = timer.claim_success_delay();
            drop(timer);
            let payload = if let Some(secs) = delay_secs {
                let mut stats = state.stats.lock().map_err(|e| e.to_string())?;
                Some(stats.record_success(secs))
            } else {
                None
            };
            // 连续空闲达标 → 休息成功，停雪并重置工作计时
            finish_exit(&app, &state)?;
            if let Some(payload) = payload {
                present_summary(&app, &payload)?;
            }
        }
        (Phase::FallExit, _) => {
            // 保留兼容：不再因活动掉落结束提醒
            drop(timer);
        }
        _ => {}
    }
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
    let show_settings_i = MenuItem::with_id(app, "settings", "打开设置", true, None::<&str>)?;
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
                    let _ = show_settings(app);
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
                let _ = show_settings(app);
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
                        let _ = begin_reminder(&app, state.inner());
                    }
                }
                Some(PhaseTransition::EnterSneakPeek) => {
                    if let Some(state) = app.try_state::<AppState>() {
                        let _ = begin_sneak_peek(&app, state.inner());
                    }
                }
                Some(PhaseTransition::EnterHappyExit) => {
                    let ev = {
                        let Ok(timer) = state_timer.lock() else {
                            continue;
                        };
                        timer.to_event(Some("happyClimb"))
                    };
                    emit_pet(&app, &ev);
                }
                None => {}
            }

            // ~1s: RDP / 分辨率变化时重铺雪层，避免积雪底边漂在半空
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
            refresh_snow_layout_cmd
        ])
        .setup(move |app| {
            let resource_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")));
            let _ = seed_default_pack(&resource_dir);

            setup_tray(app.handle())?;

            if let Some(pet) = app.get_webview_window("pet") {
                let _ = pet.set_ignore_cursor_events(true);
            }
            if let Some(snow) = app.get_webview_window("snow") {
                let _ = snow.set_ignore_cursor_events(true);
            }
            // 点关闭只隐藏，避免窗口被销毁后无法再打开设置
            if let Some(summary) = app.get_webview_window("summary") {
                let summary_hide = summary.clone();
                summary.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = summary_hide.hide();
                    }
                });
            }
            if let Some(settings) = app.get_webview_window("settings") {
                let settings_hide = settings.clone();
                settings.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = settings_hide.hide();
                    }
                });
            }

            let cfg = load_config();
            let _ = sync_autostart(app.handle(), cfg.autostart);

            start_input_hook(app.handle().clone(), Arc::clone(&timer));
            start_session_watch(app.handle().clone());
            start_tick_loop(app.handle().clone(), Arc::clone(&timer));

            // 等前端挂好事件监听后再播开机入场
            let boot_app = app.handle().clone();
            let demo = std::env::var("REST_REMINDER_DEMO")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(600));
                if let Some(state) = boot_app.try_state::<AppState>() {
                    let _ = begin_boot_intro(&boot_app, state.inner());
                }
                // 预览模式：入场结束后自动触发休息提醒（飘雪 + 小猫）
                if demo {
                    thread::sleep(Duration::from_millis(2800));
                    if let Some(state) = boot_app.try_state::<AppState>() {
                        let _ = begin_reminder(&boot_app, state.inner());
                    }
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Rest Reminder Pet");
}
