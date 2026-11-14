#![windows_subsystem = "windows"]

//! Réglages — clone de l'interface d'origine, réécrit en Rust (egui).

use std::path::PathBuf;
use std::process::Command;

use eframe::egui;
use egui::{
    Align, Color32, FontId, Layout, Margin, Mesh, Rect, RichText, Rounding, Sense, Stroke, Vec2,
};
use ovl_platform::Config;

const BG: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1F);
const SIDEBAR: Color32 = Color32::from_rgb(0x17, 0x17, 0x1B);
const CARD: Color32 = Color32::from_rgb(0x23, 0x23, 0x29);
const CARD_SOFT: Color32 = Color32::from_rgb(0x1F, 0x2A, 0x33);
const ACCENT: Color32 = Color32::from_rgb(0x4C, 0x9A, 0xFF);
const TEXT: Color32 = Color32::from_rgb(0xEC, 0xEC, 0xF1);
const DIM: Color32 = Color32::from_rgb(0x98, 0x98, 0xA4);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Overlay,
    Reticule,
    Profils,
    Rapports,
    Parametres,
}

struct App {
    page: Page,
    config: Config,
    dirty: bool,
    decorated: bool,
    layout: usize,
    theme: usize,
}

const LAYOUTS: [&str; 3] = ["Panneau", "Ligne", "Graphique"];

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        style(&cc.egui_ctx);
        Self {
            page: Page::Overlay,
            config: Config::load(),
            dirty: false,
            decorated: false,
            layout: 0,
            theme: 2,
        }
    }

    fn store(&mut self) {
        self.config.save();
        self.dirty = false;
    }
}

fn load_system_font(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\SegUIVar.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert("ui".to_owned(), egui::FontData::from_owned(bytes));
            fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "ui".to_owned());
            fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().insert(0, "ui".to_owned());
            ctx.set_fonts(fonts);
            return;
        }
    }
}

fn style(ctx: &egui::Context) {
    load_system_font(ctx);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = CARD;
    visuals.override_text_color = Some(TEXT);
    visuals.widgets.inactive.bg_fill = CARD;
    visuals.widgets.inactive.weak_bg_fill = CARD;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x2E, 0x2E, 0x36);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.5);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.text_styles = [
        (egui::TextStyle::Heading, FontId::proportional(26.0)),
        (egui::TextStyle::Body, FontId::proportional(14.0)),
        (egui::TextStyle::Button, FontId::proportional(14.0)),
        (egui::TextStyle::Small, FontId::proportional(11.5)),
    ]
    .into();
    ctx.set_style(style);
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.decorated {
            self.decorated = true;
            dark_title_bar();
        }

        egui::SidePanel::left("nav")
            .exact_width(224.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(SIDEBAR).inner_margin(Margin::symmetric(10.0, 14.0)))
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(22.0), Sense::hover());
                    ui.painter().circle_filled(rect.center(), 11.0, ACCENT.linear_multiply(0.35));
                    ui.painter().circle_stroke(rect.center(), 5.0, Stroke::new(2.0, ACCENT));
                    ui.add_space(2.0);
                    ui.label(RichText::new("FPS Counter & Crosshair").size(13.0).strong());
                    let badge = egui::Frame::none()
                        .fill(ACCENT.linear_multiply(0.25))
                        .rounding(Rounding::same(4.0))
                        .inner_margin(Margin::symmetric(5.0, 1.0));
                    badge.show(ui, |ui| ui.label(RichText::new("PRO").size(9.0).color(ACCENT)));
                });
                ui.add_space(16.0);

                nav(ui, &mut self.page, Page::Overlay, "\u{25C8}", "Overlay");
                nav(ui, &mut self.page, Page::Reticule, "\u{25CE}", "Réticule");
                nav(ui, &mut self.page, Page::Profils, "\u{2630}", "Profils");
                nav(ui, &mut self.page, Page::Rapports, "\u{25A4}", "Rapports");

                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    nav(ui, &mut self.page, Page::Parametres, "\u{2699}", "Paramètres");
                    nav_bottom(ui, "\u{2606}", "Pro");
                    nav_bottom(ui, "\u{003F}", "Aide");
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(Margin::symmetric(22.0, 16.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| match self.page {
                    Page::Overlay => self.overlay_page(ui),
                    Page::Reticule => self.reticule_page(ui),
                    Page::Profils => placeholder(ui, "Profils", "Profils de disposition par jeu — étape suivante."),
                    Page::Rapports => placeholder(ui, "Rapports", "Sessions enregistrées et exports — étape suivante."),
                    Page::Parametres => self.settings_page(ui),
                });
            });

        if self.dirty {
            self.store();
        }
    }
}

fn nav(ui: &mut egui::Ui, page: &mut Page, target: Page, icon: &str, label: &str) {
    let selected = *page == target;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(0x24, 0x24, 0x2A));
        let bar = Rect::from_min_size(rect.min + Vec2::new(1.0, 7.0), Vec2::new(3.0, rect.height() - 14.0));
        ui.painter().rect_filled(bar, Rounding::same(2.0), ACCENT);
    }
    let color = if selected { TEXT } else { DIM };
    ui.painter().text(
        rect.min + Vec2::new(18.0, rect.height() / 2.0),
        egui::Align2::LEFT_CENTER,
        icon,
        FontId::proportional(15.0),
        color,
    );
    ui.painter().text(
        rect.min + Vec2::new(44.0, rect.height() / 2.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        color,
    );
    if response.clicked() {
        *page = target;
    }
}

fn nav_bottom(ui: &mut egui::Ui, icon: &str, label: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
    ui.painter().text(
        rect.min + Vec2::new(18.0, rect.height() / 2.0),
        egui::Align2::LEFT_CENTER,
        icon,
        FontId::proportional(14.0),
        DIM,
    );
    ui.painter().text(
        rect.min + Vec2::new(44.0, rect.height() / 2.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.5),
        DIM,
    );
}

fn card<R>(ui: &mut egui::Ui, fill: Color32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::none()
        .fill(fill)
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(16.0, 13.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn toggle(ui: &mut egui::Ui, value: &mut bool) -> egui::Response {
    let size = Vec2::new(42.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool(response.id, *value);
    let bg = if *value { ACCENT } else { Color32::from_rgb(0x3A, 0x3A, 0x42) };
    ui.painter().rect_filled(rect, Rounding::same(11.0), bg);
    let cx = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
    ui.painter().circle_filled(egui::pos2(cx, rect.center().y), 8.0, Color32::WHITE);
    response
}

fn icon_badge(ui: &mut egui::Ui, glyph: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 12.0, color.linear_multiply(0.22));
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, glyph, FontId::proportional(13.0), color);
}

impl App {
    fn overlay_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Overlay").size(28.0).strong());
        ui.add_space(12.0);

        // bandeau d'information
        card(ui, CARD_SOFT, |ui| {
            ui.horizontal(|ui| {
                icon_badge(ui, "i", ACCENT);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Les fréquences d'images proviennent de Windows").size(13.5).strong());
                    ui.label(RichText::new("Aucun hook, aucune injection : rien ne touche à vos jeux.").size(12.0).color(DIM));
                });
            });
        });
        ui.add_space(8.0);

        // FPS
        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                icon_badge(ui, "\u{2713}", Color32::from_rgb(0x8A, 0xD6, 0x7A));
                ui.vertical(|ui| {
                    ui.label(RichText::new("Les FPS sont activés").size(13.5));
                    ui.label(RichText::new("Températures, charge et mémoire dans un coin de votre jeu.").size(12.0).color(DIM));
                });
            });
        });
        ui.add_space(8.0);

        // bascule
        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                icon_badge(ui, "\u{25A3}", ORANGE);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Afficher les statistiques").size(13.5));
                    ui.label(RichText::new("Raccourci : Alt+Shift+O.").size(12.0).color(DIM));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if toggle(ui, &mut self.config.show_stats).changed() {
                        self.dirty = true;
                    }
                });
            });
        });
        ui.add_space(12.0);

        // aperçu
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), Sense::hover());
        gradient(ui, rect);
        ui.painter().text(
            rect.min + Vec2::new(18.0, 22.0),
            egui::Align2::LEFT_TOP,
            "FPS    240   4.2 ms",
            FontId::proportional(13.0),
            Color32::WHITE,
        );
        for (i, line) in ["CPU    27%   51°C", "GPU    6%    37°C", "RAM    13.0 / 32 Go"].iter().enumerate() {
            ui.painter().text(
                rect.min + Vec2::new(18.0, 46.0 + i as f32 * 20.0),
                egui::Align2::LEFT_TOP,
                *line,
                FontId::proportional(13.0),
                Color32::from_rgba_unmultiplied(255, 255, 255, 200),
            );
        }
        ui.add_space(12.0);

        // disposition
        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                icon_badge(ui, "\u{25A6}", DIM);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Disposition").size(13.5));
                    ui.label(RichText::new("Panneau de lignes, ligne compacte ou graphique.").size(12.0).color(DIM));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("layout")
                        .selected_text(LAYOUTS[self.layout])
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for (i, name) in LAYOUTS.iter().enumerate() {
                                ui.selectable_value(&mut self.layout, i, *name);
                            }
                        });
                });
            });
        });
        ui.add_space(14.0);

        // thèmes
        ui.label(RichText::new("Thème").size(13.5).strong());
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for (i, (name, a, b)) in [
                ("Panneau", Color32::from_rgb(0x1F, 0x2A, 0x33), Color32::from_rgb(0x2E, 0x4A, 0x3E)),
                ("Clair", Color32::from_rgb(0xE8, 0xE8, 0xEE), Color32::from_rgb(0xD6, 0xD6, 0xE2)),
                ("Graphique", Color32::from_rgb(0x20, 0x24, 0x2E), Color32::from_rgb(0x3A, 0x2E, 0x4A)),
                ("Néon", Color32::from_rgb(0x14, 0x1A, 0x1C), Color32::from_rgb(0x1E, 0x3A, 0x2A)),
            ]
            .iter()
            .enumerate()
            {
                let selected = self.theme == i;
                let (rect, response) = ui.allocate_exact_size(Vec2::new(150.0, 92.0), Sense::click());
                let mut mesh = Mesh::default();
                mesh.colored_vertex(rect.left_top(), *a);
                mesh.colored_vertex(rect.right_top(), *b);
                mesh.colored_vertex(rect.right_bottom(), *b);
                mesh.colored_vertex(rect.left_bottom(), *a);
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                ui.painter().add(egui::Shape::mesh(mesh));
                ui.painter().rect_stroke(
                    rect,
                    Rounding::same(6.0),
                    Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { ACCENT } else { Color32::from_rgb(0x36, 0x36, 0x3E) }),
                );
                ui.painter().text(
                    rect.left_top() + Vec2::new(10.0, 10.0),
                    egui::Align2::LEFT_TOP,
                    format!("FPS  144"),
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );
                ui.painter().text(
                    rect.left_top() + Vec2::new(10.0, 32.0),
                    egui::Align2::LEFT_TOP,
                    "CPU  34%  67°C",
                    FontId::proportional(10.5),
                    Color32::from_rgba_unmultiplied(255, 255, 255, 190),
                );
                ui.painter().text(
                    rect.left_bottom() + Vec2::new(10.0, -14.0),
                    egui::Align2::LEFT_CENTER,
                    *name,
                    FontId::proportional(10.5),
                    DIM,
                );
                if response.clicked() {
                    self.theme = i;
                }
                ui.add_space(2.0);
            }
        });
    }

    fn reticule_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Réticule").size(28.0).strong());
        ui.add_space(12.0);
        card(ui, CARD, |ui| {
            ui.label(RichText::new("Dix réticules + éditeur").size(13.5));
            ui.label(RichText::new("Portage du dessin D2D en cours — étape suivante du plan.").size(12.0).color(DIM));
        });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Paramètres").size(28.0).strong());
        ui.add_space(12.0);

        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                icon_badge(ui, "\u{25B6}", ACCENT);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Lancer au démarrage").size(13.5));
                    ui.label(RichText::new("Démarre l'overlay à l'ouverture de session.").size(12.0).color(DIM));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut auto = autostart_enabled();
                    if toggle(ui, &mut auto).changed() {
                        set_autostart(auto);
                    }
                });
            });
        });
        ui.add_space(8.0);

        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                icon_badge(ui, "\u{1F5C1}", DIM);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Dossier de configuration").size(13.5));
                    ui.label(RichText::new("config.json, relu à chaud par l'overlay.").size(12.0).color(DIM));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Ouvrir").clicked() {
                        let _ = Command::new("explorer.exe")
                            .arg(Config::path().parent().unwrap_or(&PathBuf::from(".")))
                            .spawn();
                    }
                });
            });
        });
        ui.add_space(8.0);

        card(ui, CARD, |ui| {
            ui.horizontal(|ui| {
                icon_badge(ui, "\u{2139}", DIM);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Overlay Rust").size(13.5));
                    ui.label(RichText::new("21 Mo de RAM, sans .NET ni WebView2.").size(12.0).color(DIM));
                });
            });
        });
    }
}

const ORANGE: Color32 = Color32::from_rgb(0xE8, 0xA0, 0x5C);

fn gradient(ui: &mut egui::Ui, rect: Rect) {
    let mut mesh = Mesh::default();
    let left = Color32::from_rgb(0x1E, 0x33, 0x2B);
    let right = Color32::from_rgb(0x6E, 0x64, 0x38);
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(egui::Shape::mesh(mesh));
    ui.painter().rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0, Color32::from_rgb(0x36, 0x36, 0x3E)));
}

fn placeholder(ui: &mut egui::Ui, title: &str, text: &str) {
    ui.label(RichText::new(title).size(28.0).strong());
    ui.add_space(12.0);
    card(ui, CARD, |ui| {
        ui.label(RichText::new(text).size(12.5).color(DIM));
    });
}

fn autostart_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu\\Programs\\Startup\\FPSOverlayRust.vbs")
}

fn autostart_enabled() -> bool {
    autostart_path().exists()
}

fn set_autostart(enabled: bool) {
    let path = autostart_path();
    if enabled {
        let exe = std::env::var("LOCALAPPDATA").unwrap_or_default() + "\\Programs\\FPSOverlayRust\\overlay.exe";
        let script = format!("CreateObject(\"WScript.Shell\").Run \"\"\"{exe}\"\" 0, False");
        let _ = std::fs::write(path, script);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

fn dark_title_bar() {
    use windows::core::w;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE};
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    unsafe {
        if let Ok(hwnd) = FindWindowW(None, w!("FPS Overlay — Réglages")) {
            let dark: i32 = 1;
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as *const _, 4);
            let round: i32 = 2;
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &round as *const _ as *const _, 4);
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1060.0, 720.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("FPS Overlay — Réglages"),
        ..Default::default()
    };
    eframe::run_native("FPS Overlay", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}