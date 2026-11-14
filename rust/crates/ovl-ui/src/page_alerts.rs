//! Page Alertes : style d'affichage et seuils.

use eframe::egui::{self, RichText, Ui, WidgetType};
use ovl_core::alerts::Style;

use crate::live::Snapshot;
use crate::pages::{intro, page_title};
use crate::widgets::{self, divider, row, section, Tone};
use crate::App;

/// Seuil réglable + interrupteur, sur une ligne.
fn threshold(
    ui: &mut Ui,
    title: &str,
    description: String,
    on: &mut bool,
    value: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    unit: &str,
) -> bool {
    let mut changed = false;
    row(ui, title, Some(RichText::new(description)), 330.0, |ui| {
        changed |= widgets::toggle(ui, on, title).changed();
        ui.add_space(12.0);
        let slider = egui::Slider::new(value, range).suffix(format!(" {unit}"));
        let response = ui.add_enabled(*on, slider);
        changed |=
            widgets::named(response, WidgetType::Slider, &format!("Seuil : {title}")).changed();
    });
    changed
}

fn now_text(value: Option<String>) -> String {
    value
        .map(|v| format!(" Actuellement : {v}."))
        .unwrap_or_default()
}

impl App {
    pub(crate) fn alerts_page(&mut self, ui: &mut Ui, snap: &Snapshot) {
        page_title(ui, "Alertes");
        intro(
            ui,
            "L'overlay vous prévient pendant le jeu quand quelque chose ne va pas. FPS, températures, ping et \
             Wi-Fi attendent 3 secondes avant d'alerter : un pic isolé ne déclenche rien.",
        );
        ui.add_space(10.0);

        let mut config = self.config.clone();
        let mut changed = false;

        if config.alerts.style != Style::Popup && !config.show_stats {
            widgets::notice(
                ui,
                Tone::Warn,
                "Le HUD est masqué : une alerte « dans le panneau » ne sera pas visible. Choisissez Pop-up ou réaffichez le HUD.",
                None,
            );
        }

        section(ui, "Affichage des alertes", |ui| {
            let description = match config.alerts.style {
                Style::Flash => "Une ligne rouge s'ajoute au HUD, qui clignote pendant 10 s.",
                Style::Popup => "Un bandeau en haut de l'écran pendant 5 s, au plus toutes les 2 min par alerte.",
                Style::Both => "Ligne rouge clignotante dans le HUD et bandeau en haut de l'écran.",
            };
            row(ui, "Style", Some(RichText::new(description)), 300.0, |ui| {
                let options: Vec<_> = Style::ALL.iter().map(|s| (*s, s.name())).collect();
                changed |=
                    widgets::segmented(ui, &mut config.alerts.style, &options, "Style des alertes");
            });
        });

        let stats = snap.stats.as_ref();
        let overlay = snap.overlay.as_ref();
        let fps = overlay
            .and_then(|o| o.frames)
            .map(|f| format!("{:.0} FPS", f.fps));
        let cpu = stats
            .and_then(|s| s.cpu_temp_c)
            .map(|t| format!("{t:.0} °C"));
        let gpu = stats
            .and_then(|s| s.gpu_temp_c)
            .map(|t| format!("{t:.0} °C"));
        let net = overlay.and_then(|o| o.net.as_ref());
        let ping = net.and_then(|n| n.ping_ms).map(|p| format!("{p:.0} ms"));
        let traffic = overlay
            .and_then(|o| o.traffic.as_ref())
            .map(|t| format!("{:.1} Mb/s", t.total_mbps).replace('.', ","));
        let host = config.ping_host.clone();

        let a = &mut config.alerts;
        section(ui, "Seuils", |ui| {
            changed |= threshold(
                ui,
                "FPS sous",
                format!("Quand un jeu est au premier plan.{}", now_text(fps)),
                &mut a.fps,
                &mut a.fps_below,
                10..=240,
                "FPS",
            );
            divider(ui);
            changed |= threshold(
                ui,
                "CPU au-dessus de",
                format!("Température du capteur AMD.{}", now_text(cpu)),
                &mut a.cpu_temp,
                &mut a.cpu_temp_above,
                50..=105,
                "°C",
            );
            divider(ui);
            changed |= threshold(
                ui,
                "GPU au-dessus de",
                format!("Température du pilote graphique.{}", now_text(gpu)),
                &mut a.gpu_temp,
                &mut a.gpu_temp_above,
                50..=105,
                "°C",
            );
            divider(ui);
            changed |= threshold(
                ui,
                "Ping au-dessus de",
                format!(
                    "Vers {host} (modifiable dans Overlay › Statistiques avancées).{}",
                    now_text(ping)
                ),
                &mut a.ping,
                &mut a.ping_above,
                20..=500,
                "ms",
            );
            divider(ui);
            changed |= threshold(
                ui,
                "Trafic en arrière-plan au-dessus de",
                format!(
                    "Programmes autres que le jeu (téléchargements, synchro…).{}",
                    now_text(traffic)
                ),
                &mut a.traffic,
                &mut a.traffic_above,
                1..=200,
                "Mb/s",
            );
            divider(ui);
            row(
                ui,
                "Perte de paquets ou gigue",
                Some(RichText::new(
                    "Perte de 2 % ou plus, gigue de 30 ms ou plus sur la dernière minute, ou une perte dans les 10 dernières secondes.",
                )),
                48.0,
                |ui| changed |= widgets::toggle(ui, &mut a.packet_loss, "Perte de paquets ou gigue").changed(),
            );
            divider(ui);
            row(
                ui,
                "Wi-Fi faible ou hors ligne",
                Some(RichText::new(
                    "1 barre sur 5 ou moins, ou plus de connexion Internet.",
                )),
                48.0,
                |ui| {
                    changed |= widgets::toggle(ui, &mut a.weak_wifi, "Wi-Fi faible ou hors ligne")
                        .changed()
                },
            );
        });

        if changed {
            self.set_config(config);
        }
    }
}
