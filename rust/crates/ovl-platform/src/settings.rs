//! Fenêtre de réglages (Win32, sans dépendance externe).

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetDlgItem, IsWindow, PostQuitMessage, RegisterClassW,
    SetForegroundWindow, SetWindowTextW, ShowWindow, BS_PUSHBUTTON, SW_HIDE, SW_SHOWNORMAL,
    WINDOW_STYLE, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_SYSMENU, WS_VISIBLE,
};

use crate::config::Config;

const ID_STATS: usize = 2001;
const ID_ALWAYS: usize = 2002;
const ID_SCALE: usize = 2003;
const ID_QUIT: usize = 2004;
const ID_CLOSE: usize = 2005;

static mut WINDOW: isize = 0;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe fn set_text(hwnd: HWND, id: usize, text: &str) {
    if let Ok(child) = GetDlgItem(hwnd, id as i32) {
        let w = wide(text);
        let _ = SetWindowTextW(child, PCWSTR(w.as_ptr()));
    }
}

unsafe fn refresh(hwnd: HWND) {
    let config = Config::load();
    set_text(hwnd, ID_STATS, &format!("Statistiques : {}", if config.show_stats { "ON" } else { "OFF" }));
    set_text(hwnd, ID_ALWAYS, &format!("Toujours afficher : {}", if config.always_show { "ON" } else { "OFF" }));
    set_text(hwnd, ID_SCALE, &format!("Echelle : {:.2}x", config.scale));
}

unsafe fn button(parent: HWND, id: usize, text: &str, x: i32, y: i32, width: i32) {
    let label = wide(text);
    let _ = CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        PCWSTR(label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32),
        x, y, width, 30,
        parent, None, None, None,
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

        let class_name = w!("KoykSettingsWnd");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(settings_proc),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = match CreateWindowExW(
            Default::default(),
            class_name,
            w!("FPS Overlay - Reglages"),
            WS_CAPTION | WS_SYSMENU,
            200, 200, 340, 250,
            None, None, None, None,
        ) {
            Ok(hwnd) => hwnd,
            Err(_) => return,
        };

        button(hwnd, ID_STATS, "Statistiques : ON", 16, 16, 300);
        button(hwnd, ID_ALWAYS, "Toujours afficher : OFF", 16, 54, 300);
        button(hwnd, ID_SCALE, "Echelle : 1.00x", 16, 92, 300);
        button(hwnd, ID_CLOSE, "Fermer", 16, 150, 145);
        button(hwnd, ID_QUIT, "Quitter", 171, 150, 145);

        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
        refresh(hwnd);
        WINDOW = hwnd.0 as isize;
    }
}

unsafe fn on_command(hwnd: HWND, id: usize) {
    let mut config = Config::load();
    match id {
        ID_STATS => config.show_stats = !config.show_stats,
        ID_ALWAYS => config.always_show = !config.always_show,
        ID_SCALE => {
            const STEPS: [f32; 4] = [0.75, 1.0, 1.25, 1.5];
            let next = STEPS
                .iter()
                .position(|s| (s - config.scale).abs() < 0.01)
                .map(|i| STEPS[(i + 1) % STEPS.len()])
                .unwrap_or(1.0);
            config.scale = next;
        }
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
    }
    config.save();
    refresh(hwnd);
}

unsafe extern "system" fn settings_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{WM_CLOSE, WM_COMMAND, WM_DESTROY};
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
