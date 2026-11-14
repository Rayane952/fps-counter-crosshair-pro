#![windows_subsystem = "windows"]
//! Fenêtre de réglages (egui) — thème sombre, barre latérale, cartes.

use std::path::PathBuf;
use std::process::Command;

use eframe::egui;
use egui::{Color32, FontId, Margin, RichText, Rounding, Stroke, Vec2};
use ovl_platform::Config;

const BG: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1F);
const SIDEBAR: Color32 = Color32::from_rgb(0x14, 0x14, 0x18);
const CARD: Color32 = Color32::from_rgb(0x23, 0x23, 0x29);
const ACCENT: Color32 = Color32::from_rgb(0x7C, 0x5C, 0xFF);
const TEXT: Color32 = Color32::from_rgb(0xEC, 0xEC, 0xF1);
const DIM: Color32 = Color32::from_rgb(0x9A, 0x9A, 0xA6);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Overlay,
    Reticule,
    Settings,
}

struct App {
    page: Page,
    config: Config,
    dirty: bool,
    decorated: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        style(&cc.egui_ctx);
        Self { page: Page::Overlay, config: Config::load(), dirty: false, decorated: false }
    }

    fn store(&mut self) {
        self.config.save();
        self.dirty = false;
    }
}

fn style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.extreme_bg_color = CARD;
    visuals.override_text_color = Some(TEXT);
    visuals.widgets.inactive.bg_fill = CARD;
    visuals.widgets.inactive.weak_bg_fill = CARD;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x2E, 0x2E, 0x36);
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.selection.bg_fill = ACCENT.linear_multiply(0.6);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(12.0, 10.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.text_styles = [
        (egui::TextStyle::Heading, FontId::proportional(22.0)),
        (egui::TextStyle::Body, FontId::proportional(14.0)),
        (egui::TextStyle::Button, FontId::proportional(14.0)),
        (egui::TextStyle::Small, FontId::proportional(11.0)),
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
            .exact_width(190.0)
            .frame(egui::Frame::none().fill(SIDEBAR).inner_margin(Margin::symmetric(14.0, 18.0)))
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("FPS Overlay").size(17.0).strong());
                ui.label(RichText::new("version Rust").size(11.0).color(DIM));
                ui.add_space(18.0);

                nav(ui, &mut self.page, Page::Overlay, "Overlay");
                nav(ui, &mut self.page, Page::Reticule, "Réticule");
                nav(ui, &mut self.page, Page::Settings, "Paramètres");

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    if ui.button("Quitter").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new("Rust 4 Mo").size(11.0).color(DIM));
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(Margin::symmetric(26.0, 20.0)))
            .show(ctx, |ui| match self.page {
                Page::Overlay => self.overlay_page(ui),
                Page::Reticule => self.reticule_page(ui),
                Page::Settings => self.settings_page(ui),
            });

        if self.dirty {
            self.store();
        }
    }
}

fn nav(ui: &mut egui::Ui, page: &mut Page, target: Page, label: &str) {
    let selected = *page == target;
    let text = RichText::new(label).size(14.0).color(if selected { TEXT } else { DIM });
    let response = ui.add_sized(
        [162.0, 34.0],
        egui::Button::new(text)
            .fill(if selected { ACCENT.linear_multiply(0.25) } else { Color32::TRANSPARENT })
            .stroke(Stroke::NONE)
            .rounding(Rounding::same(8.0)),
    );
    if response.clicked() {
        *page = target;
    }
}

fn card(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(CARD)
        .rounding(Rounding::same(12.0))
        .inner_margin(Margin::symmetric(16.0, 14.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(13.0).color(DIM));
            ui.add_space(6.0);
            add(ui);
        });
    ui.add_space(10.0);
}

impl App {
    fn overlay_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Overlay").size(26.0).strong());
        ui.add_space(14.0);

        card(ui, "AFFICHAGE", |ui| {
            ui.checkbox(&mut self.config.show_stats, RichText::new("Afficher les statistiques").size(14.0))
                .changed()
                .then(|| self.dirty = true);
            ui.checkbox(&mut self.config.always_show, RichText::new("Toujours afficher (hors jeu)").size(14.0))
                .changed()
                .then(|| self.dirty = true);
        });

        card(ui, "TAILLE ET POSITION", |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Échelle").size(14.0));
                if ui
                    .add_sized([260.0, 20.0], egui::Slider::new(&mut self.config.scale, 0.75..=1.5).step_by(0.05).show_value(true))
                    .changed()
                {
                    self.dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Marge").size(14.0));
                if ui.add_sized([260.0, 20.0], egui::Slider::new(&mut self.config.margin, 0..=60).suffix(" px")).changed() {
                    self.dirty = true;
                }
            });
        });

        card(ui, "APERÇU", |ui| {
            let s = self.config.scale;
            ui.horizontal(|ui| {
            let panel = egui::Frame::none()
                .fill(Color32::from_rgb(0x0E, 0x0E, 0x12))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::symmetric(12.0, 10.0));
            panel.show(ui, |ui| {
                for (label, value) in [
                    ("FPS", "240"),
                    ("CPU", "27%"),
                    ("TEMP", "51°C"),
                    ("GPU", "6%"),
                ] {
                    ui.horizontal(|ui| {
                        ui.add_sized([70.0, 22.0 * s], egui::Label::new(RichText::new(label).size(11.0 * s).color(DIM)));
                        ui.label(RichText::new(value).size(15.0 * s).strong().color(TEXT));
                    });
                }
            });
            });
        });
    }

    fn reticule_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Réticule").size(26.0).strong());
        ui.add_space(14.0);
        card(ui, "BIENTÔT", |ui| {
            ui.label(RichText::new("Réticule et profils : portés à l'étape suivante du plan Rust.").color(DIM));
        });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Paramètres").size(26.0).strong());
        ui.add_space(14.0);

        card(ui, "DÉMARRAGE", |ui| {
            let mut auto = autostart_enabled();
            if ui.checkbox(&mut auto, RichText::new("Lancer au démarrage de session").size(14.0)).changed() {
                set_autostart(auto);
            }
        });

        card(ui, "FICHIERS", |ui| {
            if ui.button("Ouvrir le dossier de configuration").clicked() {
                let _ = Command::new("explorer.exe").arg(Config::path().parent().unwrap_or(&PathBuf::from("."))).spawn();
            }
        });

        card(ui, "À PROPOS", |ui| {
            ui.label(RichText::new("Overlay Rust — 4 Mo, sans .NET ni WebView2.").color(DIM));
            ui.label(RichText::new("Capteur AMD : sensor.exe (tâche élevée au logon).").color(DIM));
        });
    }
}

fn autostart_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata)
        .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup\\FPSOverlayRust.vbs")
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
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE};
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

    unsafe {
        if let Ok(hwnd) = FindWindowW(None, w!("FPS Overlay — Réglages")) {
            let dark: i32 = 1;
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as *const _, 4);
            let round: i32 = 2;
            let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &round as *const _ as *const _, 4);
            let _ = HWND(hwnd.0);
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 620.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title("FPS Overlay — Réglages"),
        ..Default::default()
    };
    eframe::run_native("FPS Overlay", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}