use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WanxiangConfig {
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
}

impl Default for WanxiangConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            endpoint: Some(
                "https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis"
                    .into(),
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub work_minutes: u32,
    pub observation_seconds: u32,
    /// 休息提醒时飘雪持续秒数（与小猫退场独立）
    #[serde(default = "default_snow_seconds")]
    pub snow_seconds: u32,
    pub character_pack: String,
    pub autostart: bool,
    pub wanxiang: WanxiangConfig,
}

fn default_snow_seconds() -> u32 {
    120
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            work_minutes: 20,
            observation_seconds: 30,
            snow_seconds: default_snow_seconds(),
            character_pack: "default".into(),
            autostart: false,
            wanxiang: WanxiangConfig::default(),
        }
    }
}

pub fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rest-reminder-pet")
}

pub fn config_path() -> PathBuf {
    app_data_dir().join("config.json")
}

pub fn characters_dir() -> PathBuf {
    app_data_dir().join("characters")
}

pub fn ensure_dirs() -> Result<(), String> {
    fs::create_dir_all(characters_dir()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_config() -> AppConfig {
    let path = config_path();
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&text) {
            cfg.work_minutes = cfg.work_minutes.clamp(1, 180);
            cfg.observation_seconds = cfg.observation_seconds.clamp(10, 120);
            cfg.snow_seconds = cfg.snow_seconds.clamp(30, 600);
            return cfg;
        }
    }
    let cfg = AppConfig::default();
    let _ = save_config(&cfg);
    cfg
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    ensure_dirs()?;
    let text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(), text).map_err(|e| e.to_string())
}
