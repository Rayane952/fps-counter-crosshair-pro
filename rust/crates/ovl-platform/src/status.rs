//! État publié par overlay.exe, lu par les réglages (status.json, à côté de config.json).
//!
//! Les images (ETW) et le réseau ne sont mesurés que par l'overlay : les réglages en affichent
//! les dernières valeurs publiées ici, pour l'aperçu et l'état de chaque mesure.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ovl_core::frames::{Reading as Frames, Status as FrameStatus};
use ovl_core::net::{traffic, Reading as Net};
use serde::{Deserialize, Serialize};

use crate::config::{write_atomic, Config};

/// L'overlay publie à chaque tick (1 s) ; au-delà, on le considère arrêté.
const STALE_AFTER: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayStatus {
    /// Mémoire de travail d'overlay.exe, mesurée par le processus lui-même.
    pub working_set_bytes: u64,
    /// Le raccourci Alt+Maj+O a pu être enregistré (faux s'il est pris par une autre application).
    pub hotkey_ok: bool,
    /// Un jeu est détecté au premier plan.
    pub in_game: bool,
    /// Session ETW des images : `None` tant qu'aucune mesure ne la demande.
    pub frames_status: Option<FrameStatus>,
    /// Images du processus au premier plan.
    pub frames: Option<Frames>,
    /// Nom de l'exécutable au premier plan (pour dire de qui sont les FPS).
    pub foreground: Option<String>,
    pub net: Option<Net>,
    pub traffic: Option<traffic::Reading>,
    /// Derniers temps d'image (ms), seulement en disposition Graphique, pour l'aperçu.
    pub graph: Vec<f32>,
    /// Secondes depuis l'époque Unix.
    pub updated: u64,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl OverlayStatus {
    fn path() -> std::path::PathBuf {
        Config::dir().join("status.json")
    }

    pub fn publish(mut self) {
        self.updated = now();
        if let Ok(text) = serde_json::to_string(&self) {
            let _ = write_atomic(&Self::path(), text.as_bytes());
        }
    }

    /// `None` si l'overlay ne tourne pas (pas de fichier, ou fichier trop ancien).
    pub fn read_live() -> Option<Self> {
        let text = std::fs::read_to_string(Self::path()).ok()?;
        let status: OverlayStatus = serde_json::from_str(&text).ok()?;
        (now().saturating_sub(status.updated) <= STALE_AFTER.as_secs()).then_some(status)
    }
}
