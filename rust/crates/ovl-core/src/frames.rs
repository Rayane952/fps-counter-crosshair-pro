//! Lecture des images présentées.
//!
//! Voie principale : `DwmGetCompositionFrameStats` (sans admin, ce que fait l'app actuelle).
//! Voie de secours : session ETW `Microsoft-Windows-DXGI` (nécessite l'élévation).

#[derive(Debug, Default, Clone, Copy)]
pub struct Reading {
    pub fps: f32,
    pub frame_ms: f32,
    pub low_1pct: f32,
    pub low_01pct: f32,
}

/// TODO(phase 1) : interroger DWM pour le PID au premier plan.
pub fn read(_pid: u32) -> Option<Reading> {
    None
}
