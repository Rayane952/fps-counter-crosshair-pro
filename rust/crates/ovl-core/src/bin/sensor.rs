#![windows_subsystem = "windows"]
//! Capteur élevé : publie la température CPU (SDK AMD) dans un fichier lu par l'overlay.
//!
//! GetRmCpuParameters exige un processus élevé ; l'overlay tourne volontairement en
//! utilisateur normal, d'où ce petit démon (tâche planifiée au logon, privilèges élevés).

use std::fs;
use std::thread;
use std::time::Duration;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| ovl_core::amd::TEMP_FILE.to_string());
    let interval: u64 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);

    if let Some(dir) = std::path::Path::new(&path).parent() {
        let _ = fs::create_dir_all(dir);
    }

    loop {
        if let Some(temp) = ovl_core::amd::cpu_temp() {
            let text = format!("{temp:.1}");
            let _ = fs::write(&path, text);
        }
        thread::sleep(Duration::from_millis(interval));
    }
}
