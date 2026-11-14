//! Détection du jeu au premier plan : processus qui rend réellement en 3D.

use crate::pdh::{Counter, Query};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// PID de la fenêtre au premier plan (0 si aucune).
pub fn foreground_pid() -> u32 {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return 0;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid
    }
}

/// Nom de l'exécutable d'un processus, sans « .exe » (« PID n » s'il est inaccessible).
pub fn process_name(pid: u32) -> String {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return format!("PID {pid}");
        };
        let mut buffer = [0u16; 260];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(handle);
        if !ok {
            return format!("PID {pid}");
        }
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        let file = path.rsplit('\\').next().unwrap_or(&path);
        let lower = file.to_ascii_lowercase();
        if lower.ends_with(".exe") {
            file[..file.len() - 4].to_owned()
        } else {
            file.to_owned()
        }
    }
}

/// Vérifie qu'un processus consomme réellement le moteur 3D du GPU.
pub struct GameProbe {
    query: Query,
    engines: Counter,
}

impl GameProbe {
    pub fn new() -> Option<Self> {
        let query = Query::new()?;
        let engines = query.add(r"\GPU Engine(*)\Utilization Percentage")?;
        query.collect();
        Some(Self { query, engines })
    }

    /// Somme des moteurs 3D du processus donné, en pourcentage (0..100).
    pub fn usage(&mut self, pid: u32) -> f32 {
        if pid == 0 || !self.query.collect() {
            return 0.0;
        }
        let needle = format!("pid_{pid}_");
        let mut total = 0.0f64;
        for (name, value) in self.engines.values() {
            let lower = name.to_ascii_lowercase();
            if lower.contains("engtype_3d") && lower.contains(&needle) {
                total += value;
            }
        }
        total as f32
    }
}
