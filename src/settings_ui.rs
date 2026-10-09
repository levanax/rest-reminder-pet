use crate::app::UiCommand;
use crate::autostart::set_autostart;
use crate::character_pack::{import_pack_from_folder, list_pack_names};
use crate::config::{load_config, save_config, AppConfig, WanxiangConfig};
use eframe::egui;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub struct SettingsApp {
    work_minutes: u32,
    observation_seconds: u32,
    character_pack: String,
    packs: Vec<String>,
    autostart: bool,
    api_key: String,
    status: String,
    status_ok: bool,
    cmd_tx: Sender<UiCommand>,
    /// Shared flag: when false, close requested from outside.
    open: Arc<Mutex<bool>>,
}

impl SettingsApp {
    pub fn new(cmd_tx: Sender<UiCommand>, open: Arc<Mutex<bool>>) -> Self {
        let cfg = load_config();
        let packs = list_pack_names().unwrap_or_default();
        Self {
            work_minutes: cfg.work_minutes,
            observation_seconds: cfg.observation_seconds,
            character_pack: cfg.character_pack,
            packs,
            autostart: cfg.autostart,
            api_key: cfg.wanxiang.api_key.unwrap_or_default(),
            status: String::new(),
            status_ok: true,
            cmd_tx,
            open,
        }
    }

    fn refresh_packs(&mut self, selected: Option<String>) {
        self.packs = list_pack_names().unwrap_or_default();
        if let Some(s) = selected {
            if self.packs.iter().any(|p| p == &s) {
                self.character_pack = s;
            }
        }
    }

    fn save(&mut self) {
        let mut cfg = AppConfig {
            work_minutes: self.work_minutes.clamp(1, 180),
            observation_seconds: self.observation_seconds.clamp(10, 120),
            snow_seconds: load_config().snow_seconds,
            character_pack: self.character_pack.clone(),
            autostart: self.autostart,
            wanxiang: WanxiangConfig {
                api_key: {
                    let t = self.api_key.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                },
                endpoint: load_config().wanxiang.endpoint.or_else(|| WanxiangConfig::default().endpoint),
            },
        };
        cfg.work_minutes = cfg.work_minutes.clamp(1, 180);
        cfg.observation_seconds = cfg.observation_seconds.clamp(10, 120);
        match save_config(&cfg) {
            Ok(()) => {
                let _ = set_autostart(cfg.autostart);
                let _ = self.cmd_tx.send(UiCommand::ApplyConfig(cfg));
                self.status = "已保存".into();
                self.status_ok = true;
            }
            Err(e) => {
                self.status = e;
                self.status_ok = false;
            }
        }
    }
}

impl eframe::App for SettingsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(flag) = self.open.lock() {
            if !*flag {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Rest Reminder Pet");
            ui.label("设置工作时长与桌宠形象。托盘图标可暂停、立即提醒或退出。");
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label("工作时长（分钟）");
                ui.add(egui::DragValue::new(&mut self.work_minutes).range(1..=180));
            });

            ui.horizontal(|ui| {
                ui.label("连续无键鼠才算休息成功（秒）");
                ui.add(egui::DragValue::new(&mut self.observation_seconds).range(10..=120));
            });
            ui.label(
                egui::RichText::new("有操作会重新计时；未达标前雪一直下")
                    .small()
                    .weak(),
            );

            ui.horizontal(|ui| {
                ui.label("形象包");
                egui::ComboBox::from_id_salt("pack")
                    .selected_text(&self.character_pack)
                    .show_ui(ui, |ui| {
                        for p in &self.packs {
                            ui.selectable_value(&mut self.character_pack, p.clone(), p);
                        }
                    });
                if ui.button("导入").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        match import_pack_from_folder(&folder) {
                            Ok(name) => {
                                self.refresh_packs(Some(name.clone()));
                                self.status = format!("已导入形象包：{name}");
                                self.status_ok = true;
                            }
                            Err(e) => {
                                self.status = e;
                                self.status_ok = false;
                            }
                        }
                    }
                }
            });

            ui.checkbox(&mut self.autostart, "登录 Windows 时启动（默认关闭）");

            ui.horizontal(|ui| {
                ui.label("阿里云万相 API Key");
                ui.add(
                    egui::TextEdit::singleline(&mut self.api_key)
                        .password(true)
                        .desired_width(220.0)
                        .hint_text("仅保存在本机"),
                );
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("保存").clicked() {
                    self.save();
                }
                if ui.button("立即提醒").clicked() {
                    let _ = self.cmd_tx.send(UiCommand::RemindNow);
                    self.status = "已触发提醒".into();
                    self.status_ok = true;
                }
            });

            if !self.status.is_empty() {
                let color = if self.status_ok {
                    egui::Color32::from_rgb(40, 140, 70)
                } else {
                    egui::Color32::from_rgb(180, 50, 50)
                };
                ui.colored_label(color, &self.status);
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Ok(mut flag) = self.open.lock() {
            *flag = false;
        }
    }
}

pub fn open_settings_window(cmd_tx: Sender<UiCommand>, open: Arc<Mutex<bool>>) {
    {
        let mut flag = open.lock().unwrap();
        if *flag {
            return;
        }
        *flag = true;
    }
    let open_flag = Arc::clone(&open);
    let tx = cmd_tx;
    std::thread::spawn(move || {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([420.0, 420.0])
                .with_title("Rest Reminder Pet — 设置"),
            ..Default::default()
        };
        let app = SettingsApp::new(tx, open_flag.clone());
        let _ = eframe::run_native(
            "Rest Reminder Pet — 设置",
            options,
            Box::new(|_cc| Ok(Box::new(app))),
        );
        if let Ok(mut flag) = open_flag.lock() {
            *flag = false;
        }
    });
}
