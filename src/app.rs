use crate::autostart::set_autostart;
use crate::character_pack::seed_default_pack;
use crate::config::{load_config, AppConfig};
use crate::input_hook::start_input_hook;
use crate::overlay_win::{
    apply_overlay_style, find_overlay_hwnd, monitor_at_cursor, set_overlay_bounds, virtual_desktop,
    COLOR_KEY_B, COLOR_KEY_G, COLOR_KEY_R, OVERLAY_TITLE, RectI,
};
use crate::render::assets::PackCache;
use crate::render::pet_anim::PetAnimator;
use crate::render::snow::SnowScene;
use crate::settings_ui::open_settings_window;
use crate::timer::{Phase, PhaseTransition, PetEvent, TimerState};
use crate::tray::{poll_tray_events, setup_tray};
use macroquad::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum UiCommand {
    OpenSettings,
    Pause,
    Resume,
    RemindNow,
    Dismiss,
    Quit,
    ApplyConfig(AppConfig),
    AnimDone(String),
}

struct AppState {
    timer: Arc<Mutex<TimerState>>,
    was_paused_before_reminder: Mutex<bool>,
    snow_active: Mutex<bool>,
    pending_pet: Mutex<Option<PetEvent>>,
    settings_open: Arc<Mutex<bool>>,
    quit: Mutex<bool>,
}

fn push_pet(state: &AppState, ev: PetEvent) {
    if let Ok(mut p) = state.pending_pet.lock() {
        *p = Some(ev);
    }
}

fn do_pause(state: &AppState) {
    let was_sneak = {
        let Ok(timer) = state.timer.lock() else {
            return;
        };
        timer.phase == Phase::SneakPeek
    };
    if let Ok(mut timer) = state.timer.lock() {
        timer.pause();
        if was_sneak {
            push_pet(state, timer.to_event(None));
        }
    }
}

fn begin_boot_intro(state: &AppState) {
    let Ok(mut timer) = state.timer.lock() else {
        return;
    };
    if timer.phase != Phase::BootIntro {
        return;
    }
    timer.start_boot_intro();
    let ev = timer.to_event(Some("jumpUp"));
    drop(timer);
    push_pet(state, ev);
}

fn end_boot_intro(state: &AppState) {
    let Ok(mut timer) = state.timer.lock() else {
        return;
    };
    timer.end_boot_intro();
    let ev = timer.to_event(None);
    drop(timer);
    push_pet(state, ev);
}

fn begin_sneak_peek(state: &AppState) {
    let Ok(mut timer) = state.timer.lock() else {
        return;
    };
    if timer.phase != Phase::Working {
        return;
    }
    timer.start_sneak_peek();
    let ev = timer.to_event(Some("sneakPeek"));
    drop(timer);
    push_pet(state, ev);
}

fn end_sneak_peek(state: &AppState) {
    let Ok(mut timer) = state.timer.lock() else {
        return;
    };
    timer.end_sneak_peek();
    let ev = timer.to_event(None);
    drop(timer);
    push_pet(state, ev);
}

fn begin_reminder(state: &AppState) {
    {
        let Ok(mut timer) = state.timer.lock() else {
            return;
        };
        if matches!(
            timer.phase,
            Phase::Reminding | Phase::Observing | Phase::HappyExit
        ) {
            return;
        }
        if let Ok(mut w) = state.was_paused_before_reminder.lock() {
            *w = timer.phase == Phase::Paused;
        }
        timer.start_reminding();
        let ev = timer.to_event(Some("crawl"));
        drop(timer);
        push_pet(state, ev);
    }
    if let Ok(mut s) = state.snow_active.lock() {
        *s = true;
    }
}

fn finish_exit(state: &AppState) {
    let was_paused = state
        .was_paused_before_reminder
        .lock()
        .map(|g| *g)
        .unwrap_or(false);
    if let Ok(mut timer) = state.timer.lock() {
        timer.dismiss_to_working(was_paused);
        let ev = timer.to_event(None);
        drop(timer);
        push_pet(state, ev);
    }
    if let Ok(mut s) = state.snow_active.lock() {
        *s = false;
    }
}

fn notify_action_done(state: &AppState, action: &str) {
    let phase = {
        let Ok(timer) = state.timer.lock() else {
            return;
        };
        timer.phase
    };
    match (phase, action) {
        (Phase::BootIntro, "jumpUp") => end_boot_intro(state),
        (Phase::SneakPeek, "sneakPeek") => end_sneak_peek(state),
        (Phase::Reminding, "crawl") => {
            if let Ok(timer) = state.timer.lock() {
                let ev = timer.to_event(Some("lookDown"));
                drop(timer);
                push_pet(state, ev);
            }
        }
        (Phase::Reminding, "lookDown") => {
            if let Ok(mut timer) = state.timer.lock() {
                timer.enter_observing();
                let ev = timer.to_event(Some("lookDown"));
                drop(timer);
                push_pet(state, ev);
            }
        }
        (Phase::HappyExit, _) => finish_exit(state),
        _ => {}
    }
}

fn handle_cmd(state: &AppState, cmd: UiCommand, cmd_tx: &Sender<UiCommand>) {
    match cmd {
        UiCommand::OpenSettings => {
            open_settings_window(cmd_tx.clone(), Arc::clone(&state.settings_open));
        }
        UiCommand::Pause => do_pause(state),
        UiCommand::Resume => {
            if let Ok(mut t) = state.timer.lock() {
                t.resume();
            }
        }
        UiCommand::RemindNow => begin_reminder(state),
        UiCommand::Dismiss => finish_exit(state),
        UiCommand::Quit => {
            if let Ok(mut q) = state.quit.lock() {
                *q = true;
            }
        }
        UiCommand::ApplyConfig(cfg) => {
            if let Ok(mut timer) = state.timer.lock() {
                timer.apply_config(
                    cfg.work_minutes,
                    cfg.observation_seconds,
                    cfg.character_pack.clone(),
                );
            }
            let _ = set_autostart(cfg.autostart);
        }
        UiCommand::AnimDone(action) => notify_action_done(state, &action),
    }
}

fn start_tick_loop(state: Arc<AppState>, cmd_tx: Sender<UiCommand>) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(250));
        if state.quit.lock().map(|g| *g).unwrap_or(true) {
            break;
        }
        let transition = {
            let Ok(mut timer) = state.timer.lock() else {
                continue;
            };
            timer.tick()
        };
        match transition {
            Some(PhaseTransition::EnterReminding) => {
                begin_reminder(&state);
            }
            Some(PhaseTransition::EnterSneakPeek) => {
                begin_sneak_peek(&state);
            }
            Some(PhaseTransition::EnterHappyExit) => {
                if let Ok(timer) = state.timer.lock() {
                    let ev = timer.to_event(Some("happyClimb"));
                    drop(timer);
                    push_pet(&state, ev);
                }
            }
            None => {}
        }
        let _ = &cmd_tx;
    });
}

async fn render_loop(state: Arc<AppState>, cmd_rx: Receiver<UiCommand>, cmd_tx: Sender<UiCommand>) {
    let mut hwnd: Option<isize> = None;
    let mut styled = false;
    let mut packs = PackCache::new();
    let mut pet = PetAnimator::new();
    let mut snow = SnowScene::new();
    let mut last_snow = false;
    let mut last_bounds = RectI {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    let mut last_show = false;

    // Boot intro after short delay (handled via timestamp)
    let boot_at = get_time() + 0.6;
    let demo = std::env::var("REST_REMINDER_DEMO")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut boot_done = false;
    let mut demo_fired = false;
    let demo_at = boot_at + 2.8;

    loop {
        if state.quit.lock().map(|g| *g).unwrap_or(false) {
            break;
        }

        while let Ok(cmd) = cmd_rx.try_recv() {
            handle_cmd(&state, cmd, &cmd_tx);
        }
        poll_tray_events(&cmd_tx);

        if hwnd.is_none() {
            hwnd = find_overlay_hwnd(OVERLAY_TITLE);
        }
        if !styled {
            if let Some(h) = hwnd {
                apply_overlay_style(h);
                styled = true;
            }
        }

        let now = get_time();
        if !boot_done && now >= boot_at {
            begin_boot_intro(&state);
            boot_done = true;
        }
        if demo && !demo_fired && now >= demo_at {
            begin_reminder(&state);
            demo_fired = true;
        }

        let monitor = monitor_at_cursor();
        let desk = virtual_desktop();

        let snow_on = state.snow_active.lock().map(|g| *g).unwrap_or(false);
        if snow_on != last_snow {
            // Snow covers the full virtual desktop.
            snow.set_active(snow_on, desk.w as f32, desk.h as f32);
            last_snow = snow_on;
        }

        if let Ok(mut pending) = state.pending_pet.lock() {
            if let Some(ev) = pending.take() {
                let pack_name = ev.character_pack.clone();
                if let Ok(pack) = packs.ensure(&pack_name) {
                    pet.on_event(&ev, pack, monitor.h as f32);
                } else if ev.action.is_none() {
                    pet.clear();
                }
            }
        }

        pet.update(monitor.h as f32);
        if let Some(done) = pet.take_done() {
            notify_action_done(&state, &done);
        }

        let pet_on = pet.visible();
        let show = snow_on || pet_on;
        // Reminder/snow → virtual desktop; pet-only → cursor monitor (keeps GL backbuffer small).
        let bounds = if snow_on {
            desk
        } else if pet_on {
            monitor
        } else {
            RectI {
                x: -32000,
                y: -32000,
                w: 1,
                h: 1,
            }
        };

        if show != last_show
            || bounds.x != last_bounds.x
            || bounds.y != last_bounds.y
            || bounds.w != last_bounds.w
            || bounds.h != last_bounds.h
        {
            if let Some(h) = hwnd {
                set_overlay_bounds(h, bounds, show);
            }
            if show {
                request_new_screen_size(bounds.w as f32, bounds.h as f32);
            }
            last_bounds = bounds;
            last_show = show;
        }

        clear_background(Color::new(COLOR_KEY_R, COLOR_KEY_G, COLOR_KEY_B, 1.0));

        if show {
            let dt = get_frame_time().min(0.05);
            if snow_on {
                snow.update_and_draw(dt);
            }
            if let Some(pack) = packs.get() {
                if pet_on {
                    // Pet coords are monitor-local; window is virtual desktop (snow) or that monitor.
                    let draw_desktop = if snow_on { desk } else { monitor };
                    pet.draw(pack, monitor, draw_desktop);
                }
            }
        }

        next_frame().await;
    }
}

pub fn run() {
    let resource_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _ = seed_default_pack(&resource_dir);

    let cfg = load_config();
    let _ = set_autostart(cfg.autostart);

    let timer = Arc::new(Mutex::new(TimerState::new(
        cfg.work_minutes,
        cfg.observation_seconds,
        cfg.character_pack.clone(),
    )));

    let state = Arc::new(AppState {
        timer: Arc::clone(&timer),
        was_paused_before_reminder: Mutex::new(false),
        snow_active: Mutex::new(false),
        pending_pet: Mutex::new(None),
        settings_open: Arc::new(Mutex::new(false)),
        quit: Mutex::new(false),
    });

    let (cmd_tx, cmd_rx) = channel::<UiCommand>();

    let _tray = setup_tray(cmd_tx.clone()).expect("tray setup failed");

    start_input_hook(Arc::clone(&timer));
    start_tick_loop(Arc::clone(&state), cmd_tx.clone());

    let state_for_loop = Arc::clone(&state);
    let tx_for_loop = cmd_tx.clone();

    // Start tiny — expand only when pet/snow needs pixels (avoids huge idle GL backbuffers).
    macroquad::Window::from_config(
        Conf {
            window_title: OVERLAY_TITLE.to_owned(),
            window_width: 64,
            window_height: 64,
            high_dpi: true,
            fullscreen: false,
            sample_count: 1,
            window_resizable: true,
            ..Default::default()
        },
        async move {
            render_loop(state_for_loop, cmd_rx, tx_for_loop).await;
        },
    );
}
