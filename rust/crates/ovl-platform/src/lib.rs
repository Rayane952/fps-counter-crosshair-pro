//! Intégration système : configuration, état partagé, icône de notification, raccourci, instance unique.

pub mod config;
pub mod hotkeys;
pub mod settings;
pub mod single_instance;
pub mod status;
pub mod tray;

pub use config::Config;

/// Noms des exécutables installés (voir les `[[bin]]` des Cargo.toml et l'installeur).
pub mod names {
    pub const OVERLAY_EXE: &str = "FPSOverlay.exe";
    pub const SETTINGS_EXE: &str = "FPSOverlaySettings.exe";
    pub const SENSOR_EXE: &str = "FPSOverlaySensor.exe";
    /// Valeur de démarrage automatique sous HKCU\…\Run (aussi écrite par l'installeur).
    pub const RUN_VALUE: &str = "FPS Overlay";
}

/// Icône de l'application, ressource n° 1 de chaque exécutable (voir `build-support/resources.rs`).
pub fn app_icon(size: i32) -> Option<windows::Win32::UI::WindowsAndMessaging::HICON> {
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{LoadImageW, HICON, IMAGE_ICON, LR_DEFAULTCOLOR};
    unsafe {
        let module = GetModuleHandleW(None).ok()?;
        let handle = LoadImageW(
            module,
            PCWSTR(1 as *const u16),
            IMAGE_ICON,
            size,
            size,
            LR_DEFAULTCOLOR,
        )
        .ok()?;
        Some(HICON(handle.0))
    }
}
pub use status::OverlayStatus;
