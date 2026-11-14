//! Raccourci global Alt+Maj+O : affiche ou masque le HUD.
//!
//! Enregistré sans fenêtre : `WM_HOTKEY` arrive dans la file du thread de l'overlay,
//! traitée par `Overlay::pump`.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_NOREPEAT, MOD_SHIFT,
};

pub const TOGGLE_HUD: i32 = 1;
pub const TOGGLE_HUD_LABEL: &str = "Alt+Maj+O";

/// Faux si une autre application a déjà pris la combinaison.
pub fn install() -> bool {
    unsafe {
        RegisterHotKey(
            None,
            TOGGLE_HUD,
            MOD_ALT | MOD_SHIFT | MOD_NOREPEAT,
            u32::from(b'O'),
        )
        .is_ok()
    }
}

pub fn uninstall() {
    unsafe {
        let _ = UnregisterHotKey(None, TOGGLE_HUD);
    }
}
