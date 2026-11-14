//! Télémétrie : CPU, mémoire, GPU, capteur AMD. Aucune dépendance UI.

pub mod amd;
pub mod cpu;
pub mod games;
pub mod gpu;
pub mod pdh;

use serde::Serialize;

/// Instantané affiché par l'overlay.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Stats {
    pub cpu_load: Option<f32>,   // 0..1
    pub cpu_temp_c: Option<f32>, // capteur AMD
    pub gpu_load: Option<f32>,   // 0..1
    pub gpu_temp_c: Option<f32>,
    pub vram_gb: Option<f32>,
    pub ram_used: u64,
    pub ram_total: u64,
}

impl Stats {
    pub fn ram_ratio(&self) -> Option<f32> {
        (self.ram_total > 0).then(|| self.ram_used as f32 / self.ram_total as f32)
    }
}

/// Échantillonneur : garde les requêtes PDH ouvertes entre deux mesures.
pub struct Sampler {
    cpu: Option<cpu::CpuCounter>,
    gpu: Option<gpu::GpuCounter>,
    vram: Option<f32>,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            cpu: cpu::CpuCounter::new(),
            gpu: gpu::GpuCounter::new(),
            vram: gpu::vram_gb(),
        }
    }

    pub fn sample(&mut self) -> Stats {
        let (ram_used, ram_total) = cpu::memory();
        Stats {
            cpu_load: self.cpu.as_mut().and_then(|c| c.load().ok()),
            cpu_temp_c: amd::cpu_temp_auto(),
            gpu_load: self.gpu.as_mut().and_then(|g| g.load().ok()),
            gpu_temp_c: gpu::temperature(),
            vram_gb: self.vram,
            ram_used,
            ram_total,
        }
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}