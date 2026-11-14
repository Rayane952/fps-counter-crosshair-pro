//! Enveloppe minimale autour de PDH (compteurs de performance Windows).

use windows::core::PCWSTR;
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhOpenQueryW, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_MORE_DATA,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Une requête PDH : ouverte une fois, collectée à chaque échantillon.
pub struct Query {
    handle: isize,
}

impl Query {
    pub fn new() -> Option<Self> {
        let mut handle: isize = 0;
        let status = unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut handle) };
        (status == 0).then_some(Self { handle })
    }

    pub fn add(&self, path: &str) -> Option<Counter> {
        let w = wide(path);
        let mut handle: isize = 0;
        let status = unsafe { PdhAddEnglishCounterW(self.handle, PCWSTR(w.as_ptr()), 0, &mut handle) };
        (status == 0).then_some(Counter { handle })
    }

    pub fn collect(&self) -> bool {
        unsafe { PdhCollectQueryData(self.handle) == 0 }
    }
}

impl Drop for Query {
    fn drop(&mut self) {
        unsafe {
            let _ = PdhCloseQuery(self.handle);
        }
    }
}

/// Un compteur rattaché à une requête PDH.
#[derive(Clone, Copy)]
pub struct Counter {
    handle: isize,
}

impl Counter {
    /// Valeurs courantes : (nom d'instance, valeur). Instance vide = compteur global.
    pub fn values(&self) -> Vec<(String, f64)> {
        let mut size = 0u32;
        let mut count = 0u32;

        let status = unsafe {
            PdhGetFormattedCounterArrayW(self.handle, PDH_FMT_DOUBLE, &mut size, &mut count, None)
        };
        if status != PDH_MORE_DATA && status != 0 {
            return Vec::new();
        }
        if size == 0 || count == 0 {
            return Vec::new();
        }

        let mut buffer = vec![0u8; size as usize];
        let date_ptr = buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
        let status = unsafe {
            PdhGetFormattedCounterArrayW(
                self.handle,
                PDH_FMT_DOUBLE,
                &mut size,
                &mut count,
                Some(date_ptr),
            )
        };
        if status != 0 {
            return Vec::new();
        }

        let items = buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W;
        let mut out = Vec::with_capacity(count as usize);
        for index in 0..count as usize {
            let item = unsafe { &*items.add(index) };
            let name = unsafe { item.szName.to_string() }.unwrap_or_default();
            let value = unsafe { item.FmtValue.Anonymous.doubleValue };
            out.push((name, value));
        }
        out
    }

    /// Valeur unique (compteur sans instance).
    pub fn value(&self) -> Option<f64> {
        self.values().first().map(|(_, value)| *value)
    }
}
