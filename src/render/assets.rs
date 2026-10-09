use crate::character_pack::{load_pack, ActionDef, CharacterManifest, CharacterPackData};
use macroquad::prelude::*;
use std::collections::HashMap;

pub struct LoadedPack {
    pub name: String,
    pub manifest: CharacterManifest,
    pub textures: HashMap<String, Texture2D>,
}

impl LoadedPack {
    pub fn from_data(data: CharacterPackData) -> Self {
        let mut textures = HashMap::new();
        for (file, bytes) in &data.frames {
            if let Ok(img) = image::load_from_memory(bytes) {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                let tex = Texture2D::from_rgba8(w as u16, h as u16, &rgba);
                tex.set_filter(FilterMode::Linear);
                textures.insert(file.clone(), tex);
            }
        }
        Self {
            name: data.name,
            manifest: data.manifest,
            textures,
        }
    }

    pub fn action(&self, name: &str) -> Option<&ActionDef> {
        self.manifest.actions.get(name)
    }

    pub fn frame_tex(&self, file: &str) -> Option<&Texture2D> {
        self.textures.get(file)
    }
}

pub struct PackCache {
    current: Option<LoadedPack>,
    id: Option<String>,
}

impl PackCache {
    pub fn new() -> Self {
        Self {
            current: None,
            id: None,
        }
    }

    pub fn ensure(&mut self, name: &str) -> Result<&LoadedPack, String> {
        if self.id.as_deref() == Some(name) {
            return self
                .current
                .as_ref()
                .ok_or_else(|| "形象包未加载".to_string());
        }
        let data = load_pack(name)?;
        self.current = Some(LoadedPack::from_data(data));
        self.id = Some(name.to_string());
        self.current
            .as_ref()
            .ok_or_else(|| "形象包未加载".to_string())
    }

    pub fn get(&self) -> Option<&LoadedPack> {
        self.current.as_ref()
    }
}
