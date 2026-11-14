//! Données réelles affichées par les réglages, et actions système.
//!
//! Les mesures sont prises dans un thread à part (PDH peut prendre quelques dizaines de ms) :
//! l'interface ne se bloque jamais et redessine quand une nouvelle mesure arrive.

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use ovl_core::amd::{sensor_state, SensorState};
use ovl_core::stat::{Stat, Wanted};
use ovl_core::{Sampler, Stats};
use ovl_platform::OverlayStatus;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const SENSOR_TASK: &str = "FPSOverlaySensor";

#[derive(Default, Clone)]
pub struct Snapshot {
    /// `None` tant que la première mesure n'est pas prête (amorçage PDH : ~1 s).
    pub stats: Option<Stats>,
    /// `None` si overlay.exe ne tourne pas.
    pub overlay: Option<OverlayStatus>,
    pub sensor: Option<SensorState>,
}

pub type Shared = Arc<Mutex<Snapshot>>;

pub fn start(ctx: egui::Context) -> Shared {
    let shared: Shared = Arc::default();
    let out = Arc::clone(&shared);
    std::thread::spawn(move || {
        // l'aperçu doit pouvoir montrer n'importe quelle case cochée : tout est mesuré ici
        let all = Wanted::new(Stat::BASIC.into_iter().chain(Stat::ADVANCED));
        let mut sampler = Sampler::new();
        sampler.sample(&all, 0); // la première mesure de charge est toujours vide
        loop {
            // l'état de l'overlay et du capteur d'abord : ils s'affichent sans attendre PDH
            let overlay = OverlayStatus::read_live();
            let sensor = Some(sensor_state());
            if let Ok(mut snap) = out.lock() {
                snap.overlay = overlay;
                snap.sensor = sensor;
            }
            ctx.request_repaint();
            std::thread::sleep(Duration::from_secs(1));
            let stats = sampler.sample(&all, 0);
            if let Ok(mut snap) = out.lock() {
                snap.stats = Some(stats);
            }
        }
    });
    shared
}

pub fn install_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn overlay_exe() -> PathBuf {
    install_dir().join("overlay.exe")
}

pub fn start_overlay() -> Result<(), String> {
    let exe = overlay_exe();
    if !exe.exists() {
        return Err(format!(
            "overlay.exe introuvable dans {}",
            install_dir().display()
        ));
    }
    Command::new(exe)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// La tâche planifiée tourne avec élévation : on la déclenche, on ne lance pas sensor.exe soi-même.
pub fn start_sensor() -> Result<(), String> {
    let output = Command::new("schtasks")
        .args(["/run", "/tn", SENSOR_TASK])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "la tâche planifiée {SENSOR_TASK} n'a pas pu être lancée (est-elle installée ?)"
        ))
    }
}

pub fn open_folder(path: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn autostart_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup\FPSOverlayRust.vbs")
}

pub fn autostart_enabled() -> bool {
    autostart_path().exists()
}

fn autostart_script() -> String {
    format!(
        "CreateObject(\"WScript.Shell\").Run \"\"\"{}\"\"\", 0, False\r\n",
        overlay_exe().display()
    )
}

/// Les versions précédentes écrivaient un script invalide (guillemet non fermé) :
/// s'il est présent et différent du script attendu, il est réécrit.
pub fn repair_autostart() {
    let path = autostart_path();
    if let Ok(current) = std::fs::read_to_string(&path) {
        if current != autostart_script() && overlay_exe().exists() {
            let _ = std::fs::write(&path, autostart_script());
        }
    }
}

/// Script de démarrage qui lance l'overlay installé à côté de ces réglages, sans fenêtre.
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let path = autostart_path();
    if enabled {
        if !overlay_exe().exists() {
            return Err(format!(
                "overlay.exe introuvable dans {}",
                install_dir().display()
            ));
        }
        std::fs::write(&path, autostart_script()).map_err(|e| e.to_string())
    } else {
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
