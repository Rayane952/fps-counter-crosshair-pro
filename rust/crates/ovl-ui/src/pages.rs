//! Outils communs aux pages, et page Paramètres.

use eframe::egui::{RichText, Ui};
use ovl_platform::hotkeys::TOGGLE_HUD_LABEL;
use ovl_platform::Config;

use crate::live::{self, Snapshot};
use crate::tokens::{self, *};
use crate::widgets::{self, divider, row, section, Tone};
use crate::App;

pub(crate) fn megabytes(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1_048_576.0).replace('.', ",")
}

pub(crate) fn page_title(ui: &mut Ui, title: &str) {
    ui.label(RichText::new(title).font(font_semibold(24.0)));
    ui.add_space(4.0);
}

/// Paragraphe d'introduction sous un titre de section.
pub(crate) fn intro(ui: &mut Ui, text: &str) {
    ui.add(
        eframe::egui::Label::new(RichText::new(text).font(tokens::font(12.5)).color(TEXT_DIM))
            .wrap(),
    );
    ui.add_space(6.0);
}

/// Ligne d'état d'une mesure ou d'un composant. Renvoie vrai si l'action est cliquée.
pub(crate) fn metric(
    ui: &mut Ui,
    title: &str,
    tone: Tone,
    text: &str,
    action: Option<&str>,
) -> bool {
    let mut clicked = false;
    let width = if action.is_some() { 170.0 } else { 0.0 };
    widgets::status_row(ui, title, tone, text, width, |ui| {
        if let Some(action) = action {
            clicked = ui.button(action).clicked();
        }
    });
    clicked
}

impl App {
    pub(crate) fn report(&mut self, result: Result<(), String>, success: &str) {
        match result {
            Ok(()) => self.flash(Tone::Ok, success.to_owned()),
            Err(err) => self.flash(Tone::Error, format!("Échec : {err}.")),
        }
    }

    pub(crate) fn settings_page(&mut self, ui: &mut Ui, snap: &Snapshot) {
        page_title(ui, "Paramètres");
        ui.add_space(16.0);

        let mut autostart = self.autostart;
        let mut autostart_changed = false;
        section(ui, "Démarrage", |ui| {
            row(
                ui,
                "Lancer au démarrage de Windows",
                Some(RichText::new(
                    "Lance l'overlay à l'ouverture de session, sans fenêtre.",
                )),
                48.0,
                |ui| {
                    autostart_changed =
                        widgets::toggle(ui, &mut autostart, "Lancer au démarrage de Windows")
                            .changed();
                },
            );
        });
        if autostart_changed {
            let message = if autostart {
                "Lancement au démarrage activé."
            } else {
                "Lancement au démarrage désactivé."
            };
            self.report(live::set_autostart(autostart), message);
            self.refresh_autostart();
        }

        let mut start = false;
        section(ui, "Overlay", |ui| {
            let (tone, text) = match &snap.overlay {
                Some(s) => (
                    Tone::Ok,
                    format!(
                        "Actif. Mémoire utilisée : {} Mo (mesurée par overlay.exe).",
                        megabytes(s.working_set_bytes)
                    ),
                ),
                None => (Tone::Error, "Arrêté.".to_owned()),
            };
            let action = snap.overlay.is_none().then_some("Démarrer l'overlay");
            start = metric(ui, "État", tone, &text, action);
            divider(ui);
            let (tone, text) = match &snap.overlay {
                Some(s) if s.hotkey_ok => (
                    Tone::Ok,
                    format!("{TOGGLE_HUD_LABEL} affiche ou masque le HUD."),
                ),
                Some(_) => (
                    Tone::Warn,
                    format!("{TOGGLE_HUD_LABEL} est déjà utilisé par une autre application."),
                ),
                None => (
                    Tone::Neutral,
                    format!("{TOGGLE_HUD_LABEL}, actif quand l'overlay tourne."),
                ),
            };
            metric(ui, "Raccourci", tone, &text, None);
        });
        if start {
            self.report(live::start_overlay(), "Overlay démarré.");
        }

        let mut open = false;
        section(ui, "Fichiers", |ui| {
            let dir = Config::dir();
            row(
                ui,
                "Dossier de configuration",
                Some(RichText::new(format!(
                    "{} (config.json, status.json, overlay.log)",
                    dir.display()
                ))),
                150.0,
                |ui| open = ui.button("Ouvrir le dossier").clicked(),
            );
        });
        if open {
            self.report(live::open_folder(&Config::dir()), "Dossier ouvert.");
        }

        section(ui, "À propos", |ui| {
            row(
                ui,
                "Version",
                Some(RichText::new(format!(
                    "{} · installée dans {}",
                    env!("CARGO_PKG_VERSION"),
                    live::install_dir().display()
                ))),
                0.0,
                |_| {},
            );
        });
    }
}
