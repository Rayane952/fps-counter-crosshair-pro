//! Overlay : fenêtre layered click-through + rendu Direct2D/DirectWrite.

mod painter;
mod window;

pub use painter::Theme;
pub use window::{Overlay, OverlayConfig};
