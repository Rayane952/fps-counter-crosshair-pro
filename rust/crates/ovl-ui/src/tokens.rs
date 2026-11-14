//! Jetons de design : 4 neutres, 1 accent, 3 couleurs d'état. Aucune autre couleur dans l'interface.
//!
//! Thème sombre seul : l'outil s'ouvre à côté d'un jeu, souvent dans une pièce sombre,
//! et reprend l'apparence du HUD par défaut (Graphite).

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Rounding, Stroke, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(0x15, 0x17, 0x1B);
pub const SIDEBAR: Color32 = Color32::from_rgb(0x10, 0x12, 0x15);
pub const SURFACE: Color32 = Color32::from_rgb(0x1D, 0x20, 0x25);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(0x26, 0x2A, 0x31);
pub const BORDER: Color32 = Color32::from_rgb(0x2E, 0x33, 0x3B);

/// Texte principal : 15:1 sur SURFACE.
pub const TEXT: Color32 = Color32::from_rgb(0xEC, 0xEE, 0xF2);
/// Texte secondaire : 7:1 sur SURFACE.
pub const TEXT_DIM: Color32 = Color32::from_rgb(0xA4, 0xAA, 0xB5);
/// Entrées désactivées (exemptées de contraste WCAG, gardées lisibles : 4,6:1 sur SIDEBAR).
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x80, 0x86, 0x92);

/// Seul accent : sélection, interrupteurs actifs, anneau de focus.
pub const ACCENT: Color32 = Color32::from_rgb(0x5B, 0x9D, 0xFF);

pub const OK: Color32 = Color32::from_rgb(0x4C, 0xC3, 0x8A);
pub const WARN: Color32 = Color32::from_rgb(0xE5, 0xA2, 0x3B);
pub const ERROR: Color32 = Color32::from_rgb(0xF0, 0x60, 0x5D);

pub const RADIUS: f32 = 8.0;
pub const RADIUS_SMALL: f32 = 6.0;
pub const CONTENT_MAX_WIDTH: f32 = 760.0;

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn font_semibold(size: f32) -> FontId {
    FontId::new(size, semibold())
}

pub fn font_bold(size: f32) -> FontId {
    FontId::new(size, bold())
}

/// Fond teinté par l'accent pour les éléments sélectionnés (le blanc reste lisible dessus).
pub fn accent_tint() -> Color32 {
    Color32::from_rgb(0x24, 0x34, 0x4D)
}

pub fn focus_ring() -> Stroke {
    Stroke::new(2.0_f32, ACCENT)
}

fn load_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut add = |name: &str, file: &str| -> bool {
        match std::fs::read(format!(r"C:\Windows\Fonts\{file}")) {
            Ok(bytes) => {
                fonts
                    .font_data
                    .insert(name.to_owned(), FontData::from_owned(bytes));
                true
            }
            Err(_) => false,
        }
    };
    let regular = add("ui", "SegUIVar.ttf") || add("ui", "segoeui.ttf");
    let semi = add("ui-semibold", "seguisb.ttf");
    let heavy = add("ui-bold", "segoeuib.ttf");

    let fallback = fonts.families[&FontFamily::Proportional].clone();
    if regular {
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "ui".to_owned());
    }
    let family = |primary: Option<&str>| -> Vec<String> {
        let mut list: Vec<String> = primary.into_iter().map(str::to_owned).collect();
        if regular {
            list.push("ui".to_owned());
        }
        list.extend(fallback.iter().cloned());
        list
    };
    fonts
        .families
        .insert(semibold(), family(semi.then_some("ui-semibold")));
    fonts
        .families
        .insert(bold(), family(heavy.then_some("ui-bold")));
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    load_fonts(ctx);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = SIDEBAR;
    visuals.override_text_color = Some(TEXT);
    visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
    visuals.selection.bg_fill = accent_tint();
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.hyperlink_color = ACCENT;

    let w = &mut visuals.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_DIM);
    for state in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        state.rounding = Rounding::same(RADIUS_SMALL);
        state.fg_stroke = Stroke::new(1.0_f32, TEXT);
    }
    w.inactive.bg_fill = SURFACE_HOVER;
    w.inactive.weak_bg_fill = SURFACE_HOVER;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.hovered.bg_fill = Color32::from_rgb(0x30, 0x35, 0x3E);
    w.hovered.weak_bg_fill = Color32::from_rgb(0x30, 0x35, 0x3E);
    w.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x44, 0x4A, 0x55));
    w.hovered.expansion = 0.0;
    // egui affiche le focus clavier avec l'état « actif » : on en fait un anneau d'accent
    w.active.bg_fill = Color32::from_rgb(0x30, 0x35, 0x3E);
    w.active.weak_bg_fill = Color32::from_rgb(0x30, 0x35, 0x3E);
    w.active.bg_stroke = focus_ring();
    w.active.expansion = 0.0;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.interact_size.y = 30.0;
    style.spacing.slider_width = 200.0;
    style.text_styles = [
        (egui::TextStyle::Heading, font_semibold(24.0)),
        (egui::TextStyle::Body, font(14.0)),
        (egui::TextStyle::Button, font(13.5)),
        (egui::TextStyle::Small, font(12.0)),
        (egui::TextStyle::Monospace, FontId::monospace(13.0)),
    ]
    .into();
    ctx.set_style(style);
}
