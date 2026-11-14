//! Intégration système : configuration, état partagé, icône de notification, raccourci, instance unique.

pub mod config;
pub mod hotkeys;
pub mod settings;
pub mod single_instance;
pub mod status;
pub mod tray;

pub use config::Config;
pub use status::OverlayStatus;
