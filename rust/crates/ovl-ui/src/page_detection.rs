//! Section « Détection des jeux » de la page Overlay : verdict sur les programmes récents,
//! règles de l'utilisateur, bibliothèques trouvées.

use eframe::egui::{Align, Layout, RichText, Ui, Vec2, WidgetType};
use ovl_core::detect::{Rule, Source};
use ovl_platform::status::DetectionReport;

use crate::live::Snapshot;
use crate::pages::intro;
use crate::tokens::*;
use crate::widgets::{self, divider, row, section, Tone};
use crate::App;

/// Boutons alignés à droite mais dans l'ordre de lecture (ordre visuel = ordre de tabulation).
/// `target` complète le nom accessible : plusieurs lignes portent les mêmes libellés.
fn buttons(ui: &mut Ui, labels: &[&str], target: &str) -> Option<usize> {
    // largeur exacte : texte mesuré dans la police des boutons + marges internes + espacement
    let font = eframe::egui::TextStyle::Button.resolve(ui.style());
    let padding = ui.spacing().button_padding.x * 2.0;
    let gap = ui.spacing().item_spacing.x;
    let width = labels
        .iter()
        .map(|l| {
            ui.painter()
                .layout_no_wrap((*l).to_owned(), font.clone(), TEXT)
                .size()
                .x
                + padding
        })
        .sum::<f32>()
        + gap * labels.len().saturating_sub(1) as f32;
    let mut clicked = None;
    ui.allocate_ui_with_layout(
        Vec2::new(width, 32.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            for (i, label) in labels.iter().enumerate() {
                let response = ui.button(*label);
                if widgets::named(response, WidgetType::Button, &format!("{label} : {target}"))
                    .clicked()
                {
                    clicked = Some(i);
                }
            }
        },
    );
    clicked
}

fn file_stem(path: &str) -> &str {
    let file = path.rsplit('\\').next().unwrap_or(path);
    file.strip_suffix(".exe")
        .or_else(|| file.strip_suffix(".EXE"))
        .unwrap_or(file)
}

fn library_summary(library: &[(Source, usize)]) -> String {
    if library.is_empty() {
        return "Aucune bibliothèque trouvée : seuls les jeux en plein écran seront détectés."
            .to_owned();
    }
    library
        .iter()
        .map(|(source, n)| {
            let unit = match (*source == Source::Windows, *n > 1) {
                (true, true) => "jeux",
                (true, false) => "jeu",
                (false, true) => "dossiers",
                (false, false) => "dossier",
            };
            format!("{} ({n} {unit})", source.name())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

impl App {
    fn set_rule(&mut self, path: &str, show: Option<bool>) {
        let mut config = self.config.clone();
        config
            .game_rules
            .retain(|r| !r.path.eq_ignore_ascii_case(path));
        if let Some(show) = show {
            config.game_rules.push(Rule {
                path: path.to_owned(),
                show,
            });
        }
        self.set_config(config);
        let name = file_stem(path).to_owned();
        let text = match show {
            Some(true) => format!("Le HUD s'affichera toujours pour {name}."),
            Some(false) => format!("Le HUD ne s'affichera jamais pour {name}."),
            None => format!("Règle retirée : {name} est de nouveau détecté automatiquement."),
        };
        self.flash(Tone::Ok, text);
    }

    /// Une ligne par programme récent : verdict, raison, boutons de règle.
    fn recent_row(
        &self,
        ui: &mut Ui,
        report: &DetectionReport,
        action: &mut Option<(String, Option<bool>)>,
    ) {
        let ruled = self
            .config
            .game_rules
            .iter()
            .any(|x| x.path.eq_ignore_ascii_case(&report.path));
        let (tone, shown) = if report.verdict.game {
            (Tone::Ok, "HUD affiché")
        } else {
            (Tone::Neutral, "HUD masqué")
        };
        let fps = report
            .fps
            .filter(|f| *f >= 1.0)
            .map(|f| format!(", {f:.0} FPS"))
            .unwrap_or_default();
        let text = format!("{shown} : {}{fps}.", report.verdict.reason.describe());
        let width = if ruled { 150.0 } else { 300.0 };
        widgets::status_row(ui, &report.name, tone, &text, width, |ui| {
            if ruled {
                if buttons(ui, &["Retirer la règle"], &report.name).is_some() {
                    *action = Some((report.path.clone(), None));
                }
            } else {
                match buttons(
                    ui,
                    &["Toujours afficher", "Ne jamais afficher"],
                    &report.name,
                ) {
                    Some(0) => *action = Some((report.path.clone(), Some(true))),
                    Some(_) => *action = Some((report.path.clone(), Some(false))),
                    None => {}
                }
            }
        });
    }

    pub(crate) fn detection_section(&mut self, ui: &mut Ui, snap: &Snapshot) {
        ui.label(RichText::new("Détection des jeux").font(font_semibold(15.0)));
        intro(
            ui,
            "Le HUD s'affiche quand un jeu au premier plan rend des images : jeu reconnu par Windows ou \
             installé dans une bibliothèque (Steam, Epic, Riot…), ou programme inconnu en plein écran avec \
             rendu 3D. Il faut 2 secondes de détection avant l'affichage. Un programme mal classé se corrige \
             ci-dessous.",
        );

        let overlay = snap.overlay.as_ref();
        let recent = overlay.map(|o| o.recent.clone()).unwrap_or_default();
        let mut action: Option<(String, Option<bool>)> = None;

        ui.label(
            RichText::new("Programmes récents")
                .font(font_semibold(13.5))
                .color(TEXT_DIM),
        );
        ui.add_space(4.0);
        section(ui, "", |ui| match overlay {
            None => widgets::status_row(
                ui,
                "Détection arrêtée",
                Tone::Error,
                "La détection est assurée par l'overlay, qui est arrêté.",
                0.0,
                |_| {},
            ),
            Some(_) if recent.is_empty() => widgets::status_row(
                ui,
                "Aucun programme observé",
                Tone::Neutral,
                "Depuis le démarrage de l'overlay. Ouvrez un jeu puis revenez ici.",
                0.0,
                |_| {},
            ),
            Some(_) => {
                for (i, report) in recent.iter().enumerate() {
                    if i > 0 {
                        divider(ui);
                    }
                    self.recent_row(ui, report, &mut action);
                }
            }
        });

        ui.label(
            RichText::new("Vos règles et bibliothèques")
                .font(font_semibold(13.5))
                .color(TEXT_DIM),
        );
        ui.add_space(4.0);
        section(ui, "", |ui| {
            if self.config.game_rules.is_empty() {
                row(
                    ui,
                    "Aucune règle",
                    Some(RichText::new("Utilisez « Toujours afficher » ou « Ne jamais afficher » sur un programme récent.")),
                    0.0,
                    |_| {},
                );
            }
            let rules = self.config.game_rules.clone();
            for (i, rule) in rules.iter().enumerate() {
                if i > 0 {
                    divider(ui);
                }
                let mode = if rule.show {
                    "Toujours afficher"
                } else {
                    "Ne jamais afficher"
                };
                let tone = if rule.show { Tone::Ok } else { Tone::Neutral };
                let name = file_stem(&rule.path).to_owned();
                widgets::status_row(
                    ui,
                    &name,
                    tone,
                    &format!("{mode} · {}", rule.path),
                    110.0,
                    |ui| {
                        if buttons(ui, &["Retirer"], &name).is_some() {
                            action = Some((rule.path.clone(), None));
                        }
                    },
                );
            }
            divider(ui);
            let summary = match overlay {
                Some(o) => library_summary(&o.library),
                None => "Lues par l'overlay, qui est arrêté.".to_owned(),
            };
            row(
                ui,
                "Jeux et bibliothèques trouvés",
                Some(RichText::new(summary)),
                0.0,
                |_| {},
            );
        });

        if let Some((path, show)) = action {
            self.set_rule(&path, show);
        }
    }
}
