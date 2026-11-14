//! Télémetrie : frames, CPU, GPU, RAM, capteurs. Aucune dépendance UI.

pub mod frames;
pub mod cpu;
pub mod gpu;
pub mod amd;
pub mod sessions;
pub mod alerts;

/// Instantané des statistiques affichées par l'overlay.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct Stats {
    pub cpu: Option<f32>,          // charge 0..1
    pub cpu_temp: Option<f32>,     // °C   (SDK AMD)
    pub cpu_clock_mhz: Option<f32>,
    pub gpu: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub vram_gb: Option<f32>,
    pub ram_used: u64,
    pub ram_total: u64,
}
