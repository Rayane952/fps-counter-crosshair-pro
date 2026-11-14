//! Charge GPU (PDH, moteurs 3D) et mémoire vidéo (DXGI).

use crate::pdh::{Counter, Query};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
};

pub struct GpuCounter {
    _query: Query,
    engines: Counter,
}

impl GpuCounter {
    pub fn new() -> Option<Self> {
        let query = Query::new()?;
        let engines = query.add(r"\GPU Engine(*)\Utilization Percentage")?;
        query.collect();
        Some(Self { _query: query, engines })
    }

    /// Somme des moteurs 3D (toutes instances confondues), en 0..1.
    pub fn load(&mut self) -> Result<f32, ()> {
        if !self._query.collect() {
            return Err(());
        }
        let mut total = 0.0f64;
        for (name, value) in self.engines.values() {
            if name.to_ascii_lowercase().contains("engtype_3d") {
                total += value;
            }
        }
        Ok((total as f32 / 100.0).clamp(0.0, 1.0))
    }
}

/// Mémoire vidéo dédiée de l'adaptateur principal (Go).
pub fn vram_gb() -> Option<f32> {
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().ok()?;
        let mut index = 0;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            index += 1;
            let desc = adapter.GetDesc1().ok()?;
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let bytes = desc.DedicatedVideoMemory as f32;
            if bytes > 0.0 {
                return Some(bytes / (1024.0 * 1024.0 * 1024.0));
            }
        }
        None
    }
}

/// Température GPU : D3DKMT (phase 2). En attendant, non disponible.
pub fn temperature() -> Option<f32> {
    None
}
