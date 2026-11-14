//! Intégration système : tray, raccourcis globaux, configuration, instance unique.

pub mod tray;
pub mod hotkeys;
pub mod config;
pub mod single_instance;

/// État persistant (remplace state.json).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AppState {
    pub stats_on: bool,
    pub crosshair_on: bool,
    pub theme: String,
}
