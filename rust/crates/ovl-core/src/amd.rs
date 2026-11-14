//! Température CPU via le SDK AMD installé par Adrenalin.
//!
//! `Platform.dll!GetRmCpuParameters(void*)` — export C, __cdecl, élévation requise.
//! Structure relevée empiriquement : 0x90 = double (°C), 0x14 = float (W), 0x20 = float (A).

use libloading::{Library, Symbol};
use std::sync::OnceLock;

const DLL: &str = r"C:\Program Files\AMD\CNext\CNext\Platform.dll";
const OFFSET_TEMP: usize = 0x90;
const OFFSET_POWER: usize = 0x14;

type GetRmCpuParameters = unsafe extern "C" fn(*mut u8) -> i32;

struct Probe {
    _lib: Library,
    func: GetRmCpuParameters,
}

static PROBE: OnceLock<Option<Probe>> = OnceLock::new();

fn probe() -> Option<&'static Probe> {
    PROBE
        .get_or_init(|| unsafe {
            let lib = Library::new(DLL).ok()?;
            let func: Symbol<GetRmCpuParameters> = lib.get(b"GetRmCpuParameters\0").ok()?;
            let func = *func;
            Some(Probe { _lib: lib, func })
        })
        .as_ref()
}

/// Température CPU en °C, `None` si Adrenalin n'est pas installé ou si l'appel échoue.
pub fn cpu_temp() -> Option<f32> {
    let probe = probe()?;
    let mut buf = vec![0u8; 65536];
    let rc = unsafe { (probe.func)(buf.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    let value = f64::from_le_bytes(buf[OFFSET_TEMP..OFFSET_TEMP + 8].try_into().ok()?);
    if value > 0.0 && value < 120.0 {
        Some(value as f32)
    } else {
        None
    }
}

/// Puissance package en watts (offset 0x14), utile pour les rapports.
pub fn package_power() -> Option<f32> {
    let probe = probe()?;
    let mut buf = vec![0u8; 65536];
    let rc = unsafe { (probe.func)(buf.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    Some(f32::from_le_bytes(
        buf[OFFSET_POWER..OFFSET_POWER + 4].try_into().ok()?,
    ))
}
/// Chemin du fichier publié par ovl-sensor (exécuté avec élévation).
pub const TEMP_FILE: &str = r"C:\ProgramData\OverlayTemp\cpu.txt";

/// Au-delà, la valeur publiée est considérée comme périmée (capteur arrêté).
/// Le capteur écrit toutes les 2 s par défaut.
pub const SENSOR_STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

/// État du capteur élevé, pour l'expliquer dans les réglages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorState {
    /// Le fichier est récent.
    Running,
    /// Le fichier existe mais n'est plus mis à jour.
    Stopped,
    /// Le fichier n'a jamais été écrit.
    NeverStarted,
}

pub fn sensor_state() -> SensorState {
    match std::fs::metadata(TEMP_FILE).and_then(|m| m.modified()) {
        Ok(modified) => match modified.elapsed() {
            Ok(age) if age > SENSOR_STALE_AFTER => SensorState::Stopped,
            _ => SensorState::Running,
        },
        Err(_) => SensorState::NeverStarted,
    }
}

/// Température lue depuis le fichier publié par le capteur élevé.
/// Une valeur périmée n'est pas renvoyée : sinon le HUD figerait la dernière température connue.
pub fn cpu_temp_from_file() -> Option<f32> {
    if sensor_state() != SensorState::Running {
        return None;
    }
    let text = std::fs::read_to_string(TEMP_FILE).ok()?;
    let value: f32 = text.trim().parse().ok()?;
    (value > 0.0 && value < 120.0).then_some(value)
}

/// Température : appel direct si le processus est élevé, sinon fichier du capteur.
pub fn cpu_temp_auto() -> Option<f32> {
    cpu_temp().or_else(cpu_temp_from_file)
}
