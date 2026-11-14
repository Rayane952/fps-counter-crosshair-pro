//! Mémoire système.

use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

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
