//! Icône de notification + menu contextuel (Win32, sans dépendance).

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    GetCursorPos, GetMessageW, LoadIconW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
    TrackPopupMenu, TranslateMessage, DispatchMessageW, MF_CHECKED, MF_SEPARATOR, MF_STRING,
    MSG, TPM_RIGHTBUTTON, WNDCLASSW, WS_OVERLAPPED,
};

use crate::config::Config;

pub const WM_TRAY: u32 = 0x0400 + 1;

const ID_SETTINGS: usize = 1;
const ID_TOGGLE: usize = 2;
const ID_ALWAYS: usize = 3;
const ID_QUIT: usize = 4;

struct TrayState {
    hwnd: HWND,
    icon_added: bool,
}

static mut STATE: Option<TrayState> = None;

/// Crée la fenêtre cachée qui reçoit les clics de l'icône.
pub fn create() -> Option<HWND> {
    unsafe {
        let class_name = w!("KoykTrayWnd");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(tray_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            Default::default(),
            class_name,
            w!("Koyk tray"),
            WS_OVERLAPPED,
            0, 0, 0, 0,
            None, None, None, None,
        )
        .ok()?;

        let icon = LoadIconW(None, windows::Win32::UI::WindowsAndMessaging::IDI_APPLICATION).ok()?;

        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAY,
            hIcon: icon,
            ..Default::default()
        };
        let tip: Vec<u16> = "FPS Overlay (Rust)\0".encode_utf16().collect();
        data.szTip[..tip.len()].copy_from_slice(&tip);

        let added = Shell_NotifyIconW(NIM_ADD, &data).as_bool();
        STATE = Some(TrayState { hwnd, icon_added: added });
        Some(hwnd)
    }
}

/// Retire l'icône et détruit la fenêtre.
pub fn destroy() {
    unsafe {
        if let Some(state) = &STATE {
            if state.icon_added {
                let data = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: state.hwnd,
                    uID: 1,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            }
            let _ = DestroyWindow(state.hwnd);
        }
        STATE = None;
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };
    let config = Config::load();

    let _ = AppendMenuW(menu, MF_STRING, ID_SETTINGS, w!("Réglages…"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let toggle = if config.show_stats { MF_STRING | MF_CHECKED } else { MF_STRING };
    let _ = AppendMenuW(menu, toggle, ID_TOGGLE, w!("Afficher les statistiques"));
    let always = if config.always_show { MF_STRING | MF_CHECKED } else { MF_STRING };
    let _ = AppendMenuW(menu, always, ID_ALWAYS, w!("Toujours afficher (hors jeu)"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quitter"));

    let mut point = POINT::default();
    let _ = GetCursorPos(&mut point);
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, None);
    let _ = DestroyMenu(menu);
}

unsafe extern "system" fn tray_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            let event = lp.0 as u32;
            match event {
                windows::Win32::UI::WindowsAndMessaging::WM_RBUTTONUP => show_menu(hwnd),
                windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONUP
                | windows::Win32::UI::WindowsAndMessaging::WM_LBUTTONDBLCLK => open_settings(),
                _ => {}
            }
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_COMMAND => {
            let id = wp.0 & 0xFFFF;
            match id {
                ID_SETTINGS => crate::settings::open(),
                ID_TOGGLE => {
                    let mut config = Config::load();
                    config.show_stats = !config.show_stats;
                    config.save();
                }
                ID_ALWAYS => {
                    let mut config = Config::load();
                    config.always_show = !config.always_show;
                    config.save();
                }
                ID_QUIT => {
                    destroy();
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }
        windows::Win32::UI::WindowsAndMessaging::WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// Lance la fenêtre de réglages (binaire séparé, egui).
fn open_settings() {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let settings = dir.join("settings.exe");
            if settings.exists() {
                let _ = std::process::Command::new(settings).spawn();
                return;
            }
        }
    }
    crate::settings::open();
}

/// Boucle de messages (bloquante) — utile pour un binaire sans overlay.
pub fn run_message_loop() {
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}