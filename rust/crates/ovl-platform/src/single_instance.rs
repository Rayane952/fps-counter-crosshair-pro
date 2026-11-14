//! Instance unique par mutex nommé : un double-clic ou un second lancement ne crée pas de doublon.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;

/// Garde le mutex pendant toute la vie du processus.
pub struct InstanceGuard(#[allow(dead_code)] HANDLE);

/// Une instance portant ce nom tourne-t-elle (sans en devenir une) ?
pub fn is_running(name: &str) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
    let wide: Vec<u16> = format!("Local\\FPSOverlayRust.{name}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        match OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, PCWSTR(wide.as_ptr())) {
            Ok(handle) => {
                let _ = CloseHandle(handle);
                true
            }
            Err(_) => false,
        }
    }
}

/// `None` si une autre instance portant ce nom tourne déjà.
pub fn acquire(name: &str) -> Option<InstanceGuard> {
    let wide: Vec<u16> = format!("Local\\FPSOverlayRust.{name}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let handle = CreateMutexW(None, false, PCWSTR(wide.as_ptr())).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return None;
        }
        Some(InstanceGuard(handle))
    }
}
