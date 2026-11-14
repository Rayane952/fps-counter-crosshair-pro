//! Détection du jeu au premier plan : processus qui rend réellement en 3D.

use crate::pdh::{Counter, Query};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// PID de la fenêtre au premier plan (0 si aucune).
pub fn foreground_pid() -> u32 {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return 0;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid
    }
}

/// Vérifie qu'un processus consomme réellement le moteur 3D du GPU.
pub struct GameProbe {
    query: Query,
    engines: Counter,
}

impl GameProbe {
    pub fn new() -> Option<Self> {
        let query = Query::new()?;
        let engines = query.add(r"\GPU Engine(*)\Utilization Percentage")?;
        query.collect();
        Some(Self { query, engines })
    }

    /// Somme des moteurs 3D du processus donné, en pourcentage (0..100).
    pub fn usage(&mut self, pid: u32) -> f32 {
        if pid == 0 || !self.query.collect() {
            return 0.0;
        }
        let needle = format!("pid_{pid}_");
        let mut total = 0.0f64;
        for (name, value) in self.engines.values() {
            let lower = name.to_ascii_lowercase();
            if lower.contains("engtype_3d") && lower.contains(&needle) {
                total += value;
            }
        }
        total as f32
    }
}