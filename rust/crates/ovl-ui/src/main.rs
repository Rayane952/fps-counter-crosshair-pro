#![windows_subsystem = "windows"]

//! Réglages de FPS Overlay (egui).

mod live;
mod page_alerts;
mod page_detection;
mod page_overlay;
mod pages;
mod preview;
mod tokens;
mod widgets;

use std::time::{Duration, Instant};

use eframe::egui::{self, Align, Layout, Margin, RichText};
use ovl_platform::config::LoadIssue;
use ovl_platform::{single_instance, Config};
use tokens::*;
use widgets::Tone;

const TITLE: &str = "FPS Overlay · Réglages";
/// Les autres processus (icône, raccourci) modifient aussi config.json.
const CONFIG_REFRESH: Duration = Duration::from_secs(1);
const AUTOSTART_REFRESH: Duration = Duration::from_secs(3);
const FEEDBACK_FOR: Duration = Duration::from_secs(6);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Overlay,
    Alerts,
    Settings,
}

struct Feedback {
    tone: Tone,
    text: String,
    at: Instant,
}

pub(crate) struct App {
    page: Page,
    config: Config,
    config_issue: Option<LoadIssue>,
    config_read: Instant,
    autostart: bool,
    autostart_read: Instant,
    live: live::Shared,
    feedback: Option<Feedback>,
    title_bar_done: bool,
    /// Saisie en cours de l'hôte du ping (enregistrée à la perte du focus).
    ping_draft: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        tokens::apply(&cc.egui_ctx);
        live::repair_autostart();
        let (config, config_issue) = Config::load_checked();
        Self {
            ping_draft: config.ping_host.clone(),
            page: Page::Overlay,
            config,
            config_issue,
            config_read: Instant::now(),
            autostart: live::autostart_enabled(),
            autostart_read: Instant::now(),
            live: live::start(cc.egui_ctx.clone()),
            feedback: None,
            title_bar_done: false,
        }
    }

    pub(crate) fn flash(&mut self, tone: Tone, text: String) {
        self.feedback = Some(Feedback {
            tone,
            text,
            at: Instant::now(),
        });
    }

    /// Enregistre un changement : relit le fichier, n'applique que la différence, réécrit.
    pub(crate) fn set_config(&mut self, wanted: Config) {
        let result = Config::update(|disk| {
            let before = self.config.clone();
            macro_rules! sync {
                ($($field:ident),*) => { $( if wanted.$field != before.$field { disk.$field = wanted.$field.clone(); } )* };
            }
            sync!(
                show_stats,
                always_show,
                stats,
                scale,
                margin_x,
                margin_y,
                opacity,
                corner,
                layout,
                theme,
                text_color,
                ping_host,
                alerts,
                game_rules
            );
        });
        match result {
            Ok(config) => {
                self.config = config;
                self.config_issue = None;
                self.config_read = Instant::now();
            }
            Err(err) => self.flash(Tone::Error, format!("Réglage non enregistré : {err}.")),
        }
    }

    pub(crate) fn refresh_autostart(&mut self) {
        self.autostart = live::autostart_enabled();
        self.autostart_read = Instant::now();
    }

    fn sidebar(&mut self, ui: &mut egui::Ui, overlay_running: bool) {
        ui.label(RichText::new("FPS Overlay").font(font_semibold(17.0)));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let (tone, text) = if overlay_running {
                (Tone::Ok, "Overlay actif")
            } else {
                (Tone::Error, "Overlay arrêté")
            };
            widgets::status_dot(ui, tone);
            ui.label(RichText::new(text).font(tokens::font(12.5)).color(TEXT_DIM));
        });
        ui.add_space(20.0);

        ui.spacing_mut().item_spacing.y = 2.0;
        for (page, label) in [
            (Page::Overlay, "Overlay"),
            (Page::Alerts, "Alertes"),
            (Page::Settings, "Paramètres"),
        ] {
            if widgets::nav_item(ui, label, self.page == page).clicked() {
                self.page = page;
            }
        }
        ui.add_space(20.0);
        for label in ["Réticule", "Profils", "Rapports"] {
            widgets::nav_upcoming(ui, label);
        }

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.label(
                RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                    .font(tokens::font(12.0))
                    .color(TEXT_DISABLED),
            );
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.title_bar_done {
            self.title_bar_done = platform::dark_title_bar(frame);
        }
        if self.config_read.elapsed() >= CONFIG_REFRESH {
            let (config, issue) = Config::load_checked();
            self.config = config;
            self.config_issue = issue;
            self.config_read = Instant::now();
        }
        if self.autostart_read.elapsed() >= AUTOSTART_REFRESH {
            self.refresh_autostart();
        }
        if self
            .feedback
            .as_ref()
            .is_some_and(|f| f.at.elapsed() > FEEDBACK_FOR)
        {
            self.feedback = None;
        }
        // garde la relecture de config.json et l'expiration des messages même sans entrée utilisateur
        ctx.request_repaint_after(CONFIG_REFRESH);

        let snap = self.live.lock().map(|s| s.clone()).unwrap_or_default();

        egui::SidePanel::left("nav")
            .exact_width(212.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::none()
                    .fill(SIDEBAR)
                    .inner_margin(Margin::symmetric(12.0, 18.0)),
            )
            .show(ctx, |ui| self.sidebar(ui, snap.overlay.is_some()));

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG).inner_margin(Margin {
                left: 32.0,
                right: 20.0,
                top: 24.0,
                bottom: 0.0,
            }))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(CONTENT_MAX_WIDTH);
                        if let Some(feedback) = &self.feedback {
                            let (tone, text) = (feedback.tone, feedback.text.clone());
                            widgets::notice(ui, tone, &text, None);
                        }
                        match self.page {
                            Page::Overlay => self.overlay_page(ui, &snap),
                            Page::Alerts => self.alerts_page(ui, &snap),
                            Page::Settings => self.settings_page(ui, &snap),
                        }
                        ui.add_space(24.0);
                    });
            });
    }
}

mod platform {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_WINDOW_CORNER_PREFERENCE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    /// Vrai une fois appliqué ; rappelé à chaque image tant que la fenêtre n'existe pas encore.
    pub fn dark_title_bar(frame: &eframe::Frame) -> bool {
        let Ok(handle) = frame.window_handle() else {
            return false;
        };
        let RawWindowHandle::Win32(win32) = handle.as_raw() else {
            return true;
        };
        let hwnd = HWND(win32.hwnd.get() as *mut _);
        unsafe {
            let dark: i32 = 1;
            let applied = DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &dark as *const _ as *const _,
                4,
            )
            .is_ok();
            let round: i32 = 2;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &round as *const _ as *const _,
                4,
            );
            applied
        }
    }

    /// Ramène au premier plan la fenêtre déjà ouverte.
    pub fn focus_existing(title: &str) {
        let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            if let Ok(hwnd) = FindWindowW(None, PCWSTR(wide.as_ptr())) {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    let Some(_instance) = single_instance::acquire("settings") else {
        platform::focus_existing(TITLE);
        return Ok(());
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 760.0])
            .with_min_inner_size([820.0, 560.0])
            .with_title(TITLE),
        ..Default::default()
    };
    eframe::run_native(
        "FPS Overlay",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
