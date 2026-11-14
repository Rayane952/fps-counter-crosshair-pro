//! Fenêtre overlay : WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW, topmost,
//! positionnée sur le moniteur de la fenêtre au premier plan, exclue des captures.

/// TODO(phase 2) : `CreateWindowExW` + `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`.
pub struct OverlayWindow {
    pub hwnd: isize,
}

impl OverlayWindow {
    pub fn create() -> Option<Self> {
        None
    }
}
