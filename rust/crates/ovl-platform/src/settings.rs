//! Fenêtre de réglages de secours (Win32), utilisée seulement si settings.exe est absent.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetDlgItem, IsWindow, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, SetWindowTextW, ShowWindow, BS_PUSHBUTTON, HMENU, SW_HIDE, SW_SHOWNORMAL,
    WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_DESTROY, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_SYSMENU,
    WS_TABSTOP, WS_VISIBLE,
};

use crate::config::Config;

const ID_STATS: usize = 2001;
const ID_ALWAYS: usize = 2002;
const ID_SCALE: usize = 2003;
const ID_QUIT: usize = 2004;
const ID_CLOSE: usize = 2005;

const SCALE_STEPS: [f32; 4] = [0.75, 1.0, 1.25, 1.5];

static mut WINDOW: isize = 0;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn on_off(value: bool) -> &'static str {
    if value {
        "oui"
    } else {
        "non"
    }
}

unsafe fn set_text(hwnd: HWND, id: usize, text: &str) {
    if let Ok(child) = GetDlgItem(hwnd, id as i32) {
        let w = wide(text);
        let _ = SetWindowTextW(child, PCWSTR(w.as_ptr()));
    }
}

unsafe fn refresh(hwnd: HWND) {
    let config = Config::load();
    set_text(
        hwnd,
        ID_STATS,
        &format!("Afficher le HUD : {}", on_off(config.show_stats)),
    );
    set_text(
        hwnd,
        ID_ALWAYS,
        &format!("Afficher hors jeu : {}", on_off(config.always_show)),
    );
    set_text(
        hwnd,
        ID_SCALE,
        &format!("Taille : {:.0} %", config.scale * 100.0),
    );
}

unsafe fn button(parent: HWND, id: usize, text: &str, x: i32, y: i32, width: i32) {
    let label = wide(text);
    let _ = CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        PCWSTR(label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        x,
        y,
        width,
        30,
        parent,
        // pour un contrôle enfant, ce paramètre porte l'identifiant reçu dans WM_COMMAND
        HMENU(id as *mut _),
        None,
        None,
    );
}

/// Ouvre (ou ramène au premier plan) la fenêtre de réglages.
pub fn open() {
    unsafe {
        if WINDOW != 0 {
            let hwnd = HWND(WINDOW as *mut _);
            if IsWindow(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
                let _ = SetForegroundWindow(hwnd);
                refresh(hwnd);
                return;
            }
        }

        let class_name = w!("FPSOverlaySettingsFallback");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(settings_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = match CreateWindowExW(
            Default::default(),
            class_name,
            w!("FPS Overlay · Réglages (settings.exe introuvable)"),
            WS_CAPTION | WS_SYSMENU,
            200,
            200,
            360,
            250,
            None,
            None,
            None,
            None,
        ) {
            Ok(hwnd) => hwnd,
            Err(_) => return,
        };

        button(hwnd, ID_STATS, "", 16, 16, 310);
        button(hwnd, ID_ALWAYS, "", 16, 54, 310);
        button(hwnd, ID_SCALE, "", 16, 92, 310);
        button(hwnd, ID_CLOSE, "Fermer", 16, 150, 150);
        button(hwnd, ID_QUIT, "Quitter l'overlay", 176, 150, 150);

        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
        refresh(hwnd);
        WINDOW = hwnd.0 as isize;
    }
}

unsafe fn on_command(hwnd: HWND, id: usize) {
    let change: fn(&mut Config) = match id {
        ID_STATS => |c| c.show_stats = !c.show_stats,
        ID_ALWAYS => |c| c.always_show = !c.always_show,
        ID_SCALE => |c| {
            c.scale = SCALE_STEPS
                .iter()
                .position(|s| (s - c.scale).abs() < 0.01)
                .map(|i| SCALE_STEPS[(i + 1) % SCALE_STEPS.len()])
                .unwrap_or(1.0);
        },
        ID_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            return;
        }
        ID_QUIT => {
            crate::tray::destroy();
            PostQuitMessage(0);
            return;
        }
        _ => return,
    };
    let _ = Config::update(change);
    refresh(hwnd);
}

unsafe extern "system" fn settings_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            on_command(hwnd, wp.0 & 0xFFFF);
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            LRESULT(0)
        }
        WM_DESTROY => {
            WINDOW = 0;
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
