//! Rendu de l'overlay : fenêtre layered click-through + Direct2D/DirectWrite.

pub mod window;
pub mod painter;
pub mod crosshair;
pub mod theme;

/// Une entrée du panneau ("FPS", "240", "4.2 ms").
#[derive(Debug, Clone)]
pub struct Row {
    pub label: String,
    pub value: String,
    pub extra: String,
    pub warn: bool,
}
