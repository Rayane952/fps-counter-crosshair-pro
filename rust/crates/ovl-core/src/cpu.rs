//! Charge CPU et mémoire.

use crate::pdh::{Counter, Query};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

/// Requête PDH du processeur (utilitaire, pas le temps brut : même métrique que le Gestionnaire des tâches).
pub struct CpuCounter {
    _query: Query,
    counter: Counter,
}

impl CpuCounter {
    pub fn new() -> Option<Self> {
        let query = Query::new()?;
        let counter = query.add(r"\Processor Information(_Total)\% Processor Utility")?;
        // première collecte : PDH a besoin de deux points de mesure
        query.collect();
        Some(Self { _query: query, counter })
    }

    pub fn load(&mut self) -> Result<f32, ()> {
        if !self._query.collect() {
            return Err(());
        }
        self.counter
            .value()
            .map(|v| (v as f32 / 100.0).clamp(0.0, 1.0))
            .ok_or(())
    }
}

/// Mémoire physique utilisée / totale (octets).
pub fn memory() -> (u64, u64) {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) }.is_ok();
    if ok {
        let total = status.ullTotalPhys;
        let used = total.saturating_sub(status.ullAvailPhys);
        (used, total)
    } else {
        (0, 0)
    }
}
