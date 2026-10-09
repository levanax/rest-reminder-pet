use muda::{Menu, MenuEvent, MenuItem};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use tray_icon::{
    MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use crate::app::UiCommand;

pub struct TrayHandle {
    _tray: TrayIcon,
}

pub fn setup_tray(cmd_tx: Sender<UiCommand>) -> Result<TrayHandle, String> {
    let icon = load_tray_icon()?;

    let settings_i = MenuItem::with_id("settings", "打开设置", true, None);
    let pause_i = MenuItem::with_id("pause", "暂停计时", true, None);
    let resume_i = MenuItem::with_id("resume", "继续计时", true, None);
    let remind_i = MenuItem::with_id("remind", "立即提醒", true, None);
    let dismiss_i = MenuItem::with_id("dismiss", "知道了", true, None);
    let quit_i = MenuItem::with_id("quit", "退出", true, None);

    let menu = Menu::new();
    menu.append(&settings_i).map_err(|e| e.to_string())?;
    menu.append(&pause_i).map_err(|e| e.to_string())?;
    menu.append(&resume_i).map_err(|e| e.to_string())?;
    menu.append(&remind_i).map_err(|e| e.to_string())?;
    menu.append(&dismiss_i).map_err(|e| e.to_string())?;
    menu.append(&quit_i).map_err(|e| e.to_string())?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Rest Reminder Pet")
        .with_icon(icon)
        .build()
        .map_err(|e| e.to_string())?;

    // Keep channel alive via clones in poll loop (caller polls events).
    let _ = cmd_tx;

    Ok(TrayHandle { _tray: tray })
}

pub fn poll_tray_events(cmd_tx: &Sender<UiCommand>) {
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        let id = event.id().0.as_str();
        let cmd = match id {
            "settings" => Some(UiCommand::OpenSettings),
            "pause" => Some(UiCommand::Pause),
            "resume" => Some(UiCommand::Resume),
            "remind" => Some(UiCommand::RemindNow),
            "dismiss" => Some(UiCommand::Dismiss),
            "quit" => Some(UiCommand::Quit),
            _ => None,
        };
        if let Some(c) = cmd {
            let _ = cmd_tx.send(c);
        }
    }

    while let Ok(event) = TrayIconEvent::receiver().try_recv() {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let _ = cmd_tx.send(UiCommand::OpenSettings);
        }
    }
}

fn load_tray_icon() -> Result<tray_icon::Icon, String> {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/32x32.png"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.png"),
        PathBuf::from("icons/32x32.png"),
    ];
    for path in candidates {
        if let Ok(img) = image::open(&path) {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            return tray_icon::Icon::from_rgba(rgba.into_raw(), w, h).map_err(|e| e.to_string());
        }
    }
    // 16x16 simple fallback
    let mut rgba = vec![0u8; 16 * 16 * 4];
    for px in rgba.chunks_mut(4) {
        px[0] = 240;
        px[1] = 200;
        px[2] = 160;
        px[3] = 255;
    }
    tray_icon::Icon::from_rgba(rgba, 16, 16).map_err(|e| e.to_string())
}
