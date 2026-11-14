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
use ovl_platform::{names, OverlayStatus};

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
    install_dir().join(names::OVERLAY_EXE)
}

fn missing_overlay() -> String {
    format!(
        "{} introuvable dans {}",
        names::OVERLAY_EXE,
        install_dir().display()
    )
}

pub fn start_overlay() -> Result<(), String> {
    let exe = overlay_exe();
    if !exe.exists() {
        return Err(missing_overlay());
    }
    Command::new(exe)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
/// La tâche planifiée tourne avec élévation : on la déclenche, on ne lance pas le capteur soi-même.
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

/// Page Windows « Barre des tâches », où l'on choisit les icônes visibles.
pub fn open_taskbar_settings() -> Result<(), String> {
    Command::new("explorer.exe")
        .arg("ms-settings:taskbar")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn open_folder(path: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Commande de démarrage : l'overlay installé à côté de ces réglages.
fn autostart_command() -> String {
    format!("\"{}\"", overlay_exe().display())
}

fn read_run_value() -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    let (key, value) = (wide(RUN_KEY), wide(names::RUN_VALUE));
    let mut buffer = [0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
    };
    (rc.0 == 0).then(|| {
        let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..len])
    })
}

/// Démarrage automatique par la clé Run de l'utilisateur (comme l'installeur).
pub fn autostart_enabled() -> bool {
    read_run_value().is_some()
}

/// Ancien mécanisme (script .vbs dans le dossier Démarrage) des versions de développement.
fn legacy_script() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs\Startup\FPSOverlayRust.vbs")
}

/// Migre l'ancien script vers la clé Run, et corrige une valeur Run qui pointe ailleurs
/// (ancienne installation déplacée).
pub fn repair_autostart() {
    let legacy = legacy_script();
    let had_legacy = legacy.exists();
    if had_legacy {
        let _ = std::fs::remove_file(&legacy);
    }
    let points_elsewhere =
        read_run_value().is_some_and(|v| !v.eq_ignore_ascii_case(&autostart_command()));
    if (had_legacy || points_elsewhere) && overlay_exe().exists() {
        let _ = set_autostart(true);
    }
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ,
    };
    let (key, value) = (wide(RUN_KEY), wide(names::RUN_VALUE));
    if enabled {
        if !overlay_exe().exists() {
            return Err(missing_overlay());
        }
        let data = wide(&autostart_command());
        let rc = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(value.as_ptr()),
                REG_SZ.0,
                Some(data.as_ptr() as *const _),
                (data.len() * 2) as u32,
            )
        };
        if rc.0 == 0 {
            Ok(())
        } else {
            Err(format!("écriture du registre refusée (code {})", rc.0))
        }
    } else {
        let rc = unsafe {
            RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                PCWSTR(key.as_ptr()),
                PCWSTR(value.as_ptr()),
            )
        };
        // 2 = valeur déjà absente
        if rc.0 == 0 || rc.0 == 2 {
            Ok(())
        } else {
            Err(format!(
                "suppression dans le registre refusée (code {})",
                rc.0
            ))
        }
    }
}
