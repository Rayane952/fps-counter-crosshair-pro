//! Configuration persistante (%LOCALAPPDATA%\FPSOverlayRust\config.json).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Afficher le panneau (les statistiques restent collectées).
    pub show_stats: bool,
    /// Afficher même hors jeu (diagnostic).
    pub always_show: bool,
    /// Échelle du panneau.
    pub scale: f32,
    /// Marge au bord de l'écran, en pixels logiques.
    pub margin: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self { show_stats: true, always_show: false, scale: 1.0, margin: 12 }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join("FPSOverlayRust").join("config.json")
    }

    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}
