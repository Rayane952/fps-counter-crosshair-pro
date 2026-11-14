//! Page Overlay : aperçu, affichage, thème, mesures, position et taille, état des capteurs.

use eframe::egui::{self, RichText, Ui, WidgetType};
use ovl_core::amd::SensorState;
use ovl_core::frames::Status as FrameStatus;
use ovl_core::hud::{self, Corner, Layout, Row, ThemeId};
use ovl_core::stat::Stat;
use ovl_platform::config::{LoadIssue, MARGIN_MAX, SCALE_MAX, SCALE_MIN};
use ovl_platform::hotkeys::TOGGLE_HUD_LABEL;
use ovl_platform::Config;

use crate::live::{self, Snapshot};
use crate::pages::{intro, metric, page_title};
use crate::tokens::*;
use crate::widgets::{self, divider, row, section, Tone};
use crate::{preview, App};

/// Phrase d'état de l'overlay, déduite de l'état publié par overlay.exe et de la configuration.
fn overlay_state(config: &Config, snap: &Snapshot) -> (Tone, String) {
    match &snap.overlay {
        None => (Tone::Error, "Arrêté : rien ne s'affichera en jeu.".into()),
        Some(status) if !config.show_stats => {
            let how = if status.hotkey_ok {
                format!(" ou avec {TOGGLE_HUD_LABEL}")
            } else {
                String::new()
            };
            (
                Tone::Neutral,
                format!("HUD masqué. Réactivez-le ci-dessous{how}."),
            )
        }
        Some(status) if status.in_game => (Tone::Ok, "Jeu détecté : le HUD est affiché.".into()),
        Some(_) if config.always_show => (
            Tone::Ok,
            "Le HUD est affiché en permanence, même hors jeu.".into(),
        ),
        Some(_) => (
            Tone::Ok,
            "Actif : le HUD apparaîtra dès qu'un jeu 3D passe au premier plan.".into(),
        ),
    }
}

/// Lignes du HUD avec les données disponibles ici : mesures système des réglages,
/// images et réseau publiés par l'overlay.
fn preview_rows(config: &Config, snap: &Snapshot) -> Option<Vec<Row>> {
    let stats = snap.stats.as_ref()?;
    let clock = hud::clock_now();
    let overlay = snap.overlay.as_ref();
    let view = hud::Snapshot {
        stats: Some(stats),
        frames: overlay.and_then(|o| o.frames),
        net: overlay.and_then(|o| o.net.as_ref()),
        traffic: overlay.and_then(|o| o.traffic.as_ref()),
        clock: Some(&clock),
    };
    Some(hud::rows(&config.stats, &view))
}

/// Ordre canonique : celui des cases à cocher.
fn sort_stats(stats: &mut Vec<Stat>) {
    let order: Vec<Stat> = Stat::BASIC.into_iter().chain(Stat::ADVANCED).collect();
    stats.sort_by_key(|s| order.iter().position(|o| o == s).unwrap_or(usize::MAX));
}

impl App {
    pub(crate) fn overlay_page(&mut self, ui: &mut Ui, snap: &Snapshot) {
        page_title(ui, "Overlay");
        let (tone, text) = overlay_state(&self.config, snap);
        ui.horizontal(|ui| {
            widgets::status_dot(ui, tone);
            ui.label(RichText::new(text).color(TEXT_DIM));
        });
        ui.add_space(16.0);

        if snap.overlay.is_none()
            && widgets::notice(
                ui,
                Tone::Error,
                "overlay.exe ne tourne pas. Les réglages sont enregistrés mais le HUD ne s'affichera pas.",
                Some("Démarrer l'overlay"),
            )
        {
            self.report(live::start_overlay(), "Overlay démarré.");
        }
        if let Some(LoadIssue::Invalid(err)) = &self.config_issue {
            let text = format!(
                "config.json est illisible ({err}). Valeurs par défaut affichées ; un changement réécrira le fichier."
            );
            if widgets::notice(ui, Tone::Warn, &text, Some("Ouvrir le dossier")) {
                self.report(live::open_folder(&Config::dir()), "Dossier ouvert.");
            }
        }

        let rows = preview_rows(&self.config, snap);
        ui.label(RichText::new("Aperçu").font(font_semibold(15.0)));
        intro(
            ui,
            "Valeurs mesurées sur ce PC, à taille réelle. FPS, ping et trafic viennent de l'overlay \
             (programme au premier plan). Le cadre représente le coin d'écran choisi.",
        );
        let graph = snap
            .overlay
            .as_ref()
            .map(|o| o.graph.as_slice())
            .unwrap_or(&[]);
        preview::show(ui, &self.config, rows.as_deref(), graph);
        ui.add_space(20.0);

        self.display_section(ui, snap);
        self.theme_section(ui, rows.as_deref().unwrap_or(&[]));
        self.stats_sections(ui);
        self.position_section(ui);
        self.sensors_section(ui, snap);
    }

    fn display_section(&mut self, ui: &mut Ui, snap: &Snapshot) {
        let mut config = self.config.clone();
        let mut changed = false;
        let hotkey = match &snap.overlay {
            Some(s) if !s.hotkey_ok => {
                format!("Raccourci {TOGGLE_HUD_LABEL} indisponible : déjà utilisé par une autre application.")
            }
            _ => format!("Affiché en jeu uniquement. Raccourci : {TOGGLE_HUD_LABEL}."),
        };
        section(ui, "Affichage", |ui| {
            row(
                ui,
                "Afficher le HUD",
                Some(RichText::new(hotkey)),
                48.0,
                |ui| {
                    changed |=
                        widgets::toggle(ui, &mut config.show_stats, "Afficher le HUD").changed();
                },
            );
            divider(ui);
            row(
                ui,
                "Afficher hors jeu",
                Some(RichText::new(
                    "Garde le HUD visible sur le bureau, pratique pour régler la position.",
                )),
                48.0,
                |ui| {
                    changed |=
                        widgets::toggle(ui, &mut config.always_show, "Afficher hors jeu").changed()
                },
            );
            divider(ui);
            row(
                ui,
                "Disposition",
                Some(RichText::new("Une mesure par ligne, tout sur une ligne, ou panneau + courbe des temps d'image.")),
                290.0,
                |ui| {
                    let options: Vec<_> = Layout::ALL.iter().map(|l| (*l, l.name())).collect();
                    changed |= widgets::segmented(ui, &mut config.layout, &options, "Disposition");
                },
            );
        });
        if changed {
            self.set_config(config);
        }
    }

    fn theme_section(&mut self, ui: &mut Ui, rows: &[Row]) {
        ui.label(RichText::new("Thème").font(font_semibold(15.0)));
        intro(
            ui,
            "Chaque vignette montre vos valeurs actuelles avec les couleurs du thème.",
        );
        let mut chosen = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
            for theme in ThemeId::ALL {
                let selected = self.config.theme == theme;
                if preview::theme_swatch(ui, theme, selected, rows, &self.config).clicked()
                    && !selected
                {
                    chosen = Some(theme);
                }
            }
        });
        ui.add_space(18.0);
        if let Some(theme) = chosen {
            let mut config = self.config.clone();
            config.theme = theme;
            self.set_config(config);
        }
    }

    fn stat_grid(ui: &mut Ui, id: &str, stats: &[Stat], config: &mut Config) -> bool {
        let mut changed = false;
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let column = (ui.available_width() / 3.0).floor();
                egui::Grid::new(id)
                    .num_columns(3)
                    .min_col_width(column - 8.0)
                    .spacing([8.0, 10.0])
                    .show(ui, |ui| {
                        for (i, stat) in stats.iter().enumerate() {
                            let mut on = config.shows(*stat);
                            if widgets::checkbox(ui, &mut on, stat.name())
                                .on_hover_text(stat.hint())
                                .changed()
                            {
                                if on {
                                    config.stats.push(*stat);
                                    sort_stats(&mut config.stats);
                                } else {
                                    config.stats.retain(|s| s != stat);
                                }
                                changed = true;
                            }
                            if i % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
            });
        changed
    }

    fn stats_sections(&mut self, ui: &mut Ui) {
        let mut config = self.config.clone();
        let mut changed = false;
        section(ui, "Ce qui est affiché", |ui| {
            changed |= Self::stat_grid(ui, "basic", &Stat::BASIC, &mut config);
        });
        section(ui, "Statistiques avancées", |ui| {
            changed |= Self::stat_grid(ui, "advanced", &Stat::ADVANCED, &mut config);
            let pinging = config.shows(Stat::Ping)
                || config.shows(Stat::PacketLoss)
                || config.alerts.ping
                || config.alerts.packet_loss;
            if pinging {
                divider(ui);
                row(
                    ui,
                    "Hôte du ping",
                    Some(RichText::new(
                        "Adresse ou nom d'hôte IPv4 mesuré chaque seconde (Entrée pour valider).",
                    )),
                    220.0,
                    |ui| {
                        let edit = ui.add(
                            egui::TextEdit::singleline(&mut self.ping_draft).desired_width(200.0),
                        );
                        let edit = widgets::named(edit, WidgetType::TextEdit, "Hôte du ping");
                        if edit.lost_focus() && self.ping_draft.trim() != config.ping_host {
                            config.ping_host = self.ping_draft.trim().to_owned();
                            changed = true;
                        }
                        if !edit.has_focus() && self.ping_draft.trim() == config.ping_host {
                            self.ping_draft = config.ping_host.clone();
                        }
                    },
                );
            }
        });
        if changed {
            self.set_config(config);
            self.ping_draft = self.config.ping_host.clone();
        }
    }

    fn position_section(&mut self, ui: &mut Ui) {
        let mut config = self.config.clone();
        let mut changed = false;
        section(ui, "Position et taille", |ui| {
            row(
                ui,
                "Coin de l'écran",
                Some(RichText::new("Sur le moniteur du jeu au premier plan.")),
                420.0,
                |ui| {
                    let options: Vec<_> = Corner::ALL.iter().map(|c| (*c, c.name())).collect();
                    changed |=
                        widgets::segmented(ui, &mut config.corner, &options, "Coin de l'écran");
                },
            );
            divider(ui);
            row(
                ui,
                "Distance horizontale",
                Some(RichText::new("Au bord gauche ou droit, en pixels à 100 %.")),
                280.0,
                |ui| {
                    let slider =
                        egui::Slider::new(&mut config.margin_x, 0..=MARGIN_MAX).suffix(" px");
                    changed |=
                        widgets::named(ui.add(slider), WidgetType::Slider, "Distance horizontale")
                            .changed();
                },
            );
            divider(ui);
            row(
                ui,
                "Distance verticale",
                Some(RichText::new("Au bord haut ou bas, en pixels à 100 %.")),
                280.0,
                |ui| {
                    let slider =
                        egui::Slider::new(&mut config.margin_y, 0..=MARGIN_MAX).suffix(" px");
                    changed |=
                        widgets::named(ui.add(slider), WidgetType::Slider, "Distance verticale")
                            .changed();
                },
            );
            divider(ui);
            row(
                ui,
                "Taille",
                Some(RichText::new("S'ajoute à la mise à l'échelle de Windows.")),
                280.0,
                |ui| {
                    let mut percent = (config.scale * 100.0).round();
                    let slider =
                        egui::Slider::new(&mut percent, SCALE_MIN * 100.0..=SCALE_MAX * 100.0)
                            .step_by(5.0)
                            .fixed_decimals(0)
                            .suffix(" %");
                    if widgets::named(ui.add(slider), WidgetType::Slider, "Taille du HUD").changed()
                    {
                        config.scale = percent / 100.0;
                        changed = true;
                    }
                },
            );
            divider(ui);
            let clear = config.theme.palette().clear;
            let desc = if clear {
                "Sans effet avec le thème Transparent, qui n'a pas de fond."
            } else {
                "Opacité du fond du panneau ; le texte reste opaque."
            };
            row(ui, "Arrière-plan", Some(RichText::new(desc)), 280.0, |ui| {
                let mut percent = (config.opacity * 100.0).round();
                let slider = egui::Slider::new(&mut percent, 0.0..=100.0)
                    .step_by(5.0)
                    .fixed_decimals(0)
                    .suffix(" %");
                let response = ui.add_enabled(!clear, slider);
                if widgets::named(response, WidgetType::Slider, "Opacité de l'arrière-plan")
                    .changed()
                {
                    config.opacity = percent / 100.0;
                    changed = true;
                }
            });
            divider(ui);
            let mut custom = config.text_color.is_some();
            row(
                ui,
                "Couleur du texte",
                Some(RichText::new(
                    "Valeurs et détails ; les libellés gardent la couleur du thème.",
                )),
                300.0,
                |ui| {
                    if custom {
                        let mut rgb = config.text_rgb().unwrap_or([255, 255, 255]);
                        let picker = egui::color_picker::color_edit_button_srgb(ui, &mut rgb);
                        if widgets::named(picker, WidgetType::ColorButton, "Couleur du texte")
                            .changed()
                        {
                            config.text_color = Some(hud::format_color(rgb));
                            changed = true;
                        }
                    }
                    let before = custom;
                    widgets::segmented(
                        ui,
                        &mut custom,
                        &[(false, "Couleur du thème"), (true, "Personnalisée")],
                        "Couleur du texte",
                    );
                    if custom != before {
                        let fallback = hud::format_color(config.theme.palette().value);
                        config.text_color = custom.then_some(fallback);
                        changed = true;
                    }
                },
            );
        });
        if changed {
            self.set_config(config);
        }
    }

    fn sensors_section(&mut self, ui: &mut Ui, snap: &Snapshot) {
        let stats = snap.stats.as_ref();
        let loading = stats.is_none();
        let overlay = snap.overlay.as_ref();
        let mut sensor_action = false;

        section(ui, "État des capteurs", |ui| {
            // images : session ETW de l'overlay
            let (tone, text) = match overlay {
                None => (Tone::Error, "Mesurées par l'overlay, qui est arrêté.".to_owned()),
                Some(o) => match (o.frames_status, o.frames) {
                    (None, _) => (Tone::Neutral, "Capture inactive : cochez FPS, temps d'image ou lows pour la lancer.".to_owned()),
                    (Some(FrameStatus::Running), Some(f)) => (
                        Tone::Ok,
                        format!("{:.0} FPS mesurés pour {}.", f.fps, o.foreground.as_deref().unwrap_or("le programme au premier plan")),
                    ),
                    (Some(FrameStatus::Running), None) => (
                        Tone::Ok,
                        format!(
                            "Capture active ; {} n'affiche pas d'images en ce moment.",
                            o.foreground.as_deref().unwrap_or("le programme au premier plan")
                        ),
                    ),
                    (Some(FrameStatus::NeedsPermission), _) => (
                        Tone::Error,
                        "Windows refuse la capture : le compte doit être membre du groupe « Utilisateurs du journal \
                         de performances » (puis fermer la session)."
                            .to_owned(),
                    ),
                    (Some(FrameStatus::Failed(code)), _) => (Tone::Error, format!("La capture a échoué (code Windows {code}).")),
                    (Some(FrameStatus::Stopped), _) => (Tone::Neutral, "Capture arrêtée.".to_owned()),
                },
            };
            metric(
                ui,
                "Images (FPS, temps d'image, lows, courbe)",
                tone,
                &text,
                None,
            );
            divider(ui);

            let temp = stats.and_then(|s| s.cpu_temp_c);
            let (tone, text, action) = match (temp, snap.sensor) {
                (Some(t), _) => (
                    Tone::Ok,
                    format!("{t:.0} °C, publiée par le capteur AMD (sensor.exe)."),
                    None,
                ),
                _ if loading => (Tone::Neutral, "Mesure en cours…".to_owned(), None),
                (None, Some(SensorState::Stopped)) => (
                    Tone::Error,
                    "Capteur arrêté : sa dernière valeur a plus de 10 s.".to_owned(),
                    Some("Relancer le capteur"),
                ),
                (None, Some(SensorState::NeverStarted)) => (
                    Tone::Error,
                    "Capteur jamais lancé (tâche planifiée FPSOverlaySensor).".to_owned(),
                    Some("Lancer le capteur"),
                ),
                (None, _) => (
                    Tone::Warn,
                    "Le capteur tourne mais ne publie aucune valeur valide.".to_owned(),
                    None,
                ),
            };
            sensor_action = metric(ui, "Température CPU", tone, &text, action);
            divider(ui);

            let (tone, text) = match stats {
                None => (Tone::Neutral, "Mesure en cours…".to_owned()),
                Some(s) => match (s.gpu_temp_c, s.gpu_fan_rpm) {
                    (Some(t), Some(fan)) => (
                        Tone::Ok,
                        format!("{t:.0} °C, ventilateur {fan} tr/min (pilote graphique)."),
                    ),
                    (Some(t), None) => (Tone::Ok, format!("{t:.0} °C (pilote graphique).")),
                    (None, _) => (
                        Tone::Error,
                        "Le pilote graphique ne publie pas de température.".to_owned(),
                    ),
                },
            };
            metric(ui, "Température GPU", tone, &text, None);
            divider(ui);

            let net = overlay.and_then(|o| o.net.as_ref());
            let pinging = self.config.wanted().any(&[Stat::Ping, Stat::PacketLoss]);
            let (tone, text) = match (overlay, net) {
                (None, _) => (
                    Tone::Error,
                    "Mesuré par l'overlay, qui est arrêté.".to_owned(),
                ),
                _ if !pinging => (
                    Tone::Neutral,
                    "Inactif : cochez Ping ou Perte de paquets.".to_owned(),
                ),
                (_, Some(n)) if n.host_error => {
                    (Tone::Error, format!("Hôte « {} » introuvable.", n.host))
                }
                (_, Some(n)) => match n.ping_ms {
                    Some(ms) => (Tone::Ok, format!("{ms:.0} ms vers {}.", n.host)),
                    None if n.quality.is_some() => {
                        (Tone::Warn, format!("Pas de réponse de {}.", n.host))
                    }
                    None => (Tone::Neutral, "Mesure en cours…".to_owned()),
                },
                (_, None) => (Tone::Neutral, "Mesure en cours…".to_owned()),
            };
            metric(ui, "Ping", tone, &text, None);
            divider(ui);

            let traffic_on = self.config.wanted().has(Stat::BgTraffic);
            let (tone, text) = match (overlay, overlay.and_then(|o| o.traffic.as_ref())) {
                (None, _) => (
                    Tone::Error,
                    "Mesuré par l'overlay, qui est arrêté.".to_owned(),
                ),
                _ if !traffic_on => (
                    Tone::Neutral,
                    "Inactif : cochez Trafic en arrière-plan.".to_owned(),
                ),
                (_, Some(t)) => (
                    Tone::Ok,
                    match &t.top {
                        Some((name, m)) if *m >= 0.1 => {
                            format!("{:.1} Mb/s, surtout {name}.", t.total_mbps).replace('.', ",")
                        }
                        _ => format!("{:.1} Mb/s.", t.total_mbps).replace('.', ","),
                    },
                ),
                (_, None) => (Tone::Neutral, "Mesure en cours…".to_owned()),
            };
            metric(ui, "Trafic en arrière-plan", tone, &text, None);
        });
        if sensor_action {
            self.report(
                live::start_sensor(),
                "Capteur lancé. La température apparaît sous 2 s.",
            );
        }
    }
}
