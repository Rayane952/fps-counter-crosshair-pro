//! Overlay : fenêtres layered click-through (HUD, pop-up d'alerte) + rendu GDI à alpha par pixel.

mod painter;
mod window;

pub use window::{HudSettings, Overlay, Pumped};
