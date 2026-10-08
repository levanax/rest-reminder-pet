use crate::config::{app_data_dir, characters_dir, ensure_dirs};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSize {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDef {
    pub fps: u32,
    pub frames: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterManifest {
    pub name: String,
    pub frame_size: FrameSize,
    pub actions: HashMap<String, ActionDef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterPackDto {
    pub name: String,
    pub path: String,
    pub manifest: CharacterManifest,
    pub frames: HashMap<String, String>,
}

const REQUIRED_ACTIONS: [&str; 4] = ["crawl", "lookDown", "happyClimb", "fall"];

pub fn validate_manifest(m: &CharacterManifest) -> Result<(), String> {
    for key in REQUIRED_ACTIONS {
        let action = m
            .actions
            .get(key)
            .ok_or_else(|| format!("形象包缺少动作: {key}"))?;
        if action.frames.is_empty() {
            return Err(format!("动作 {key} 没有帧"));
        }
    }
    Ok(())
}

pub fn bundled_characters_dir(resource_dir: &Path) -> PathBuf {
    resource_dir.join("characters")
}

fn resolve_bundled_default(resource_dir: &Path) -> Option<PathBuf> {
    let bundled = bundled_characters_dir(resource_dir).join("default");
    if bundled.exists() {
        return Some(bundled);
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("characters/default");
    if dev.exists() {
        return Some(dev);
    }
    None
}

fn default_pack_needs_refresh(dest: &Path) -> bool {
    let Ok(text) = fs::read_to_string(dest.join("manifest.json")) else {
        return true;
    };
    let Ok(manifest) = serde_json::from_str::<CharacterManifest>(&text) else {
        return true;
    };
    // 缺少关键动作、不是当前照片版、或帧数落后于当前内置包时刷新
    let frame_count = |key: &str| {
        manifest
            .actions
            .get(key)
            .map(|a| a.frames.len())
            .unwrap_or(0)
    };
    !manifest.actions.contains_key("sneakPeek")
        || !manifest.actions.contains_key("jumpUp")
        || !text.contains("cat-photo")
        || frame_count("jumpUp") < 6
        || frame_count("crawl") < 6
        || frame_count("lookDown") < 5
        || frame_count("sneakPeek") < 4
        || frame_count("happyClimb") < 5
        || frame_count("fall") < 6
}

pub fn seed_default_pack(resource_dir: &Path) -> Result<(), String> {
    ensure_dirs()?;
    let dest = characters_dir().join("default");
    let src = resolve_bundled_default(resource_dir).ok_or_else(|| "找不到内置 default 形象包".to_string())?;
    if dest.join("manifest.json").exists() && !default_pack_needs_refresh(&dest) {
        return Ok(());
    }
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    copy_dir(&src, &dest)
}

fn copy_dir(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let to = dest.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn list_pack_names() -> Result<Vec<String>, String> {
    ensure_dirs()?;
    let mut names = Vec::new();
    for entry in fs::read_dir(characters_dir()).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() && entry.path().join("manifest.json").exists() {
            names.push(entry.file_name().to_string_lossy().into());
        }
    }
    names.sort();
    Ok(names)
}

pub fn load_pack(name: &str) -> Result<CharacterPackDto, String> {
    let dir = characters_dir().join(name);
    let manifest_path = dir.join("manifest.json");
    let text = fs::read_to_string(&manifest_path).map_err(|e| format!("读取 manifest 失败: {e}"))?;
    let manifest: CharacterManifest =
        serde_json::from_str(&text).map_err(|e| format!("解析 manifest 失败: {e}"))?;
    validate_manifest(&manifest)?;

    let mut frames = HashMap::new();
    for action in manifest.actions.values() {
        for file in &action.frames {
            if frames.contains_key(file) {
                continue;
            }
            let bytes = fs::read(dir.join(file)).map_err(|e| format!("读取帧 {file} 失败: {e}"))?;
            let b64 = format!("data:image/png;base64,{}", B64.encode(bytes));
            frames.insert(file.clone(), b64);
        }
    }

    Ok(CharacterPackDto {
        name: manifest.name.clone(),
        path: dir.to_string_lossy().into(),
        manifest,
        frames,
    })
}

pub fn import_pack_from_folder(src: &Path) -> Result<String, String> {
    let manifest_path = src.join("manifest.json");
    if !manifest_path.exists() {
        return Err("所选文件夹缺少 manifest.json".into());
    }
    let text = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: CharacterManifest =
        serde_json::from_str(&text).map_err(|e| format!("manifest 无效: {e}"))?;
    validate_manifest(&manifest)?;
    let name = if manifest.name.trim().is_empty() {
        src.file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "imported".into())
    } else {
        manifest.name.clone()
    };
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    ensure_dirs()?;
    let dest = characters_dir().join(&safe);
    if dest.exists() {
        fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    copy_dir(src, &dest)?;
    Ok(safe)
}

#[allow(dead_code)]
pub fn app_root_hint() -> PathBuf {
    app_data_dir()
}
