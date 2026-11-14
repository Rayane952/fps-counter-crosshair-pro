//! Icône de notification + menu contextuel (Win32, sans dépendance).

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW, PostMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenu, TranslateMessage,
    IDI_APPLICATION, MF_CHECKED, MF_SEPARATOR, MF_STRING, MSG, TPM_RIGHTBUTTON, WM_COMMAND,
    WM_DESTROY, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
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
    /// Message diffusé par l'Explorateur quand la barre des tâches est recréée.
    taskbar_created: u32,
}

static mut STATE: Option<TrayState> = None;

#[allow(static_mut_refs)]
fn state() -> Option<&'static mut TrayState> {
    // un seul thread (celui de l'overlay) crée l'icône et reçoit ses messages
    unsafe { STATE.as_mut() }
}

fn icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    }
}

/// Icône de l'application à la taille de la zone de notification (icône générique en secours).
unsafe fn add_icon(hwnd: HWND) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};
    let icon = match crate::app_icon(GetSystemMetrics(SM_CXSMICON)) {
        Some(icon) => icon,
        None => match LoadIconW(None, IDI_APPLICATION) {
            Ok(icon) => icon,
            Err(_) => return false,
        },
    };
    let mut data = icon_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = icon;
    let tip: Vec<u16> = "FPS Overlay\0".encode_utf16().collect();
    data.szTip[..tip.len()].copy_from_slice(&tip);
    if Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
        return true;
    }
    // Shell_NotifyIcon peut échouer par délai dépassé tout en ayant ajouté l'icône (Explorateur
    // occupé, ouverture de session) : si une mise à jour réussit, l'icône est bien là
    Shell_NotifyIconW(NIM_MODIFY, &data).as_bool()
}

/// Réessaie d'ajouter l'icône tant qu'elle n'est pas en place. À appeler régulièrement :
/// au démarrage de Windows, la zone de notification peut ne pas être encore prête.
/// Renvoie vrai si l'icône est présente.
pub fn ensure_icon() -> bool {
    unsafe {
        match state() {
            Some(state) if !state.icon_added => {
                state.icon_added = add_icon(state.hwnd);
                state.icon_added
            }
            Some(_) => true,
            None => false,
        }
    }
}

/// Crée la fenêtre cachée qui reçoit les clics de l'icône.
pub fn create() -> Option<HWND> {
    unsafe {
        let class_name = w!("FPSOverlayTrayWnd");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(tray_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            Default::default(),
            class_name,
            w!("FPS Overlay tray"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            None,
        )
        .ok()?;

        let icon_added = add_icon(hwnd);
        let taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
        STATE = Some(TrayState {
            hwnd,
            icon_added,
            taskbar_created,
        });
        Some(hwnd)
    }
}

/// Retire l'icône et détruit la fenêtre.
pub fn destroy() {
    unsafe {
        if let Some(state) = state() {
            if state.icon_added {
                let _ = Shell_NotifyIconW(NIM_DELETE, &icon_data(state.hwnd));
            }
            let _ = DestroyWindow(state.hwnd);
        }
        STATE = None;
    }
}

unsafe fn show_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };
    let config = Config::load();
    let checked = |on: bool| {
        if on {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        }
    };

    let _ = AppendMenuW(menu, MF_STRING, ID_SETTINGS, w!("Réglages…"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(
        menu,
        checked(config.show_stats),
        ID_TOGGLE,
        w!("Afficher le HUD\tAlt+Maj+O"),
    );
    let _ = AppendMenuW(
        menu,
        checked(config.always_show),
        ID_ALWAYS,
        w!("Afficher hors jeu"),
    );
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(menu, MF_STRING, ID_QUIT, w!("Quitter l'overlay"));

    let mut point = POINT::default();
    let _ = GetCursorPos(&mut point);
    // sans premier plan, le menu ne se ferme pas quand on clique ailleurs
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, None);
    let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
    let _ = DestroyMenu(menu);
}

unsafe extern "system" fn tray_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if let Some(state) = state() {
        if msg == state.taskbar_created && state.taskbar_created != 0 {
            state.icon_added = add_icon(hwnd);
            return LRESULT(0);
        }
    }
    match msg {
        WM_TRAY => {
            // un seul déclencheur : le double-clic envoie aussi WM_LBUTTONUP
            match lp.0 as u32 {
                WM_RBUTTONUP => show_menu(hwnd),
                WM_LBUTTONUP => open_settings(),
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            match wp.0 & 0xFFFF {
                ID_SETTINGS => open_settings(),
                ID_TOGGLE => {
                    let _ = Config::update(|c| c.show_stats = !c.show_stats);
                }
                ID_ALWAYS => {
                    let _ = Config::update(|c| c.always_show = !c.always_show);
                }
                ID_QUIT => {
                    destroy();
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// Ouvre les réglages (egui) installés à côté, sinon la fenêtre Win32 de secours.
/// Les réglages garantissent eux-mêmes l'instance unique.
pub fn open_settings() {
    if let Some(settings) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(crate::names::SETTINGS_EXE)))
        .filter(|path| path.exists())
    {
        if std::process::Command::new(settings).spawn().is_ok() {
            return;
        }
    }
    crate::settings::open();
}

/// Boucle de messages (bloquante), pour un binaire sans overlay.
pub fn run_message_loop() {
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
