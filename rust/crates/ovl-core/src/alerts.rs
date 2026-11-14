//! Alertes pendant le jeu : seuils dépassés, affichés en clignotement dans le panneau,
//! en pop-up, ou les deux.
//!
//! Une alerte « soutenue » (FPS, températures, ping, Wi-Fi) attend 3 s avant de se déclencher :
//! un pic isolé ne doit pas déranger. Perte de paquets et trafic sont déjà lissés sur la durée.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::frames::Reading as Frames;
use crate::net::{traffic, Connection, Kind, Quality};
use crate::stat::Stat;
use crate::Stats;

const HOLD: Duration = Duration::from_secs(3);
const FLASH_FOR: Duration = Duration::from_secs(10);
const POPUP_FOR: Duration = Duration::from_secs(5);
/// Une même alerte ne réapparaît en pop-up qu'après ce délai.
const POPUP_AGAIN: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[default]
    Flash,
    Popup,
    Both,
}

impl Style {
    pub const ALL: [Style; 3] = [Style::Flash, Style::Popup, Style::Both];

    pub fn name(self) -> &'static str {
        match self {
            Style::Flash => "Dans le panneau",
            Style::Popup => "Pop-up",
            Style::Both => "Les deux",
        }
    }

    fn flash(self) -> bool {
        matches!(self, Style::Flash | Style::Both)
    }

    fn popup(self) -> bool {
        matches!(self, Style::Popup | Style::Both)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub style: Style,
    pub fps: bool,
    pub fps_below: u32,
    pub cpu_temp: bool,
    pub cpu_temp_above: u32,
    pub gpu_temp: bool,
    pub gpu_temp_above: u32,
    pub ping: bool,
    pub ping_above: u32,
    pub packet_loss: bool,
    pub traffic: bool,
    /// Mb/s.
    pub traffic_above: u32,
    pub weak_wifi: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            style: Style::Flash,
            fps: false,
            fps_below: 60,
            cpu_temp: false,
            cpu_temp_above: 90,
            gpu_temp: false,
            gpu_temp_above: 85,
            ping: false,
            ping_above: 100,
            packet_loss: false,
            traffic: false,
            traffic_above: 10,
            weak_wifi: false,
        }
    }
}

impl Settings {
    /// Mesures à collecter pour surveiller les alertes actives, même si elles ne sont pas affichées.
    pub fn needs(&self) -> Vec<Stat> {
        let mut out = Vec::new();
        for (on, stat) in [
            (self.fps, Stat::Fps),
            (self.cpu_temp, Stat::CpuTemp),
            (self.gpu_temp, Stat::GpuTemp),
            (self.ping, Stat::Ping),
            (self.packet_loss, Stat::PacketLoss),
            (self.traffic, Stat::BgTraffic),
            (self.weak_wifi, Stat::Connection),
        ] {
            if on {
                out.push(stat);
            }
        }
        out
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Inputs<'a> {
    pub in_game: bool,
    pub frames: Option<Frames>,
    pub stats: Option<&'a Stats>,
    pub ping_ms: Option<f32>,
    pub quality: Option<Quality>,
    pub traffic: Option<&'a traffic::Reading>,
    pub connection: Option<Connection>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Output {
    /// Ligne d'alerte à ajouter au panneau.
    pub panel: Option<String>,
    /// Le panneau doit clignoter (10 premières secondes d'une alerte).
    pub flashing: bool,
    /// Texte du pop-up à afficher.
    pub popup: Option<String>,
}

#[derive(Default)]
pub struct Engine {
    since: HashMap<&'static str, Instant>,
    started: HashMap<&'static str, Instant>,
    popped: HashMap<&'static str, Instant>,
    popup: Option<(String, Instant)>,
}

fn raised(s: &Settings, i: &Inputs) -> Vec<(&'static str, String, bool)> {
    let mut out = Vec::new();
    if let (true, true, Some(f)) = (s.fps, i.in_game, i.frames) {
        if f.fps < s.fps_below as f32 {
            out.push(("fps", format!("FPS {:.0}", f.fps), true));
        }
    }
    if let (true, Some(t)) = (s.cpu_temp, i.stats.and_then(|st| st.cpu_temp_c)) {
        if t >= s.cpu_temp_above as f32 {
            out.push(("cpu", format!("CPU {t:.0}°C"), true));
        }
    }
    if let (true, Some(t)) = (s.gpu_temp, i.stats.and_then(|st| st.gpu_temp_c)) {
        if t >= s.gpu_temp_above as f32 {
            out.push(("gpu", format!("GPU {t:.0}°C"), true));
        }
    }
    if let (true, Some(p)) = (s.ping, i.ping_ms) {
        if p >= s.ping_above as f32 {
            out.push(("ping", format!("Ping {p:.0} ms"), true));
        }
    }
    if let (true, Some(q)) = (s.packet_loss, i.quality) {
        if q.trouble {
            let text = match q.jitter_ms {
                None => "Serveur muet".to_owned(),
                Some(_) if q.loss_percent > 0.0 => {
                    format!("Perte {}", short_percent(q.loss_percent))
                }
                Some(j) => format!("Gigue {j:.0} ms"),
            };
            out.push(("loss", text, false));
        }
    }
    if let (true, Some(t)) = (s.traffic, i.traffic) {
        if t.total_mbps >= s.traffic_above as f32 {
            let text = match &t.top {
                Some((name, mbps)) => format!("{} {mbps:.0} Mb/s", short_name(name)),
                None => format!("Arrière-plan {:.0} Mb/s", t.total_mbps),
            };
            out.push(("traffic", text, false));
        }
    }
    if let (true, Some(c)) = (s.weak_wifi, i.connection) {
        if c.weak() {
            let text = match (c.kind, c.bars) {
                (Kind::Offline, _) => "Hors ligne".to_owned(),
                (Kind::Mobile, Some(b)) => format!("Signal faible {b}/5"),
                (_, Some(b)) => format!("Wi-Fi faible {b}/5"),
                _ => "Connexion faible".to_owned(),
            };
            out.push(("wifi", text, true));
        }
    }
    out
}

fn short_percent(p: f32) -> String {
    if p < 10.0 {
        format!("{p:.1} %").replace('.', ",")
    } else {
        format!("{p:.0} %")
    }
}

fn short_name(name: &str) -> String {
    let mut chars = name.chars();
    let head: String = chars.by_ref().take(16).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

impl Engine {
    pub fn check(&mut self, settings: &Settings, inputs: &Inputs, now: Instant) -> Output {
        let list = raised(settings, inputs);
        let keys: Vec<&str> = list.iter().map(|(k, _, _)| *k).collect();
        self.since.retain(|k, _| keys.contains(k));
        self.started.retain(|k, _| keys.contains(k));

        let mut active: Vec<(&'static str, String)> = Vec::new();
        let mut fresh = false;
        for (key, text, held) in list {
            let since = *self.since.entry(key).or_insert(now);
            if held && now.duration_since(since) < HOLD {
                continue;
            }
            self.started.entry(key).or_insert(now);
            if self
                .popped
                .get(key)
                .map_or(true, |t| now.duration_since(*t) >= POPUP_AGAIN)
            {
                self.popped.insert(key, now);
                fresh = true;
            }
            active.push((key, text));
        }

        let joined = |sep: &str| {
            active
                .iter()
                .map(|(_, t)| t.as_str())
                .collect::<Vec<_>>()
                .join(sep)
        };
        if settings.style.popup() && fresh {
            self.popup = Some((joined("   ·   "), now + POPUP_FOR));
        } else if let Some((_, until)) = &self.popup {
            if now >= *until || active.is_empty() {
                self.popup = None;
            } else {
                let until = *until;
                self.popup = Some((joined("   ·   "), until));
            }
        }

        let panel = (settings.style.flash() && !active.is_empty()).then(|| joined("  ·  "));
        let flashing = panel.is_some()
            && active.iter().any(|(k, _)| {
                self.started
                    .get(k)
                    .is_some_and(|t| now.duration_since(*t) < FLASH_FOR)
            });
        Output {
            panel,
            flashing,
            popup: self.popup.as_ref().map(|(t, _)| t.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hot_gpu(t: f32) -> Stats {
        Stats {
            gpu_temp_c: Some(t),
            ..Default::default()
        }
    }

    #[test]
    fn sustained_alert_waits_three_seconds() {
        let settings = Settings {
            gpu_temp: true,
            gpu_temp_above: 80,
            style: Style::Both,
            ..Default::default()
        };
        let mut engine = Engine::default();
        let t0 = Instant::now();
        let stats = hot_gpu(85.0);
        let inputs = Inputs {
            stats: Some(&stats),
            ..Default::default()
        };

        assert_eq!(engine.check(&settings, &inputs, t0).panel, None);
        let out = engine.check(&settings, &inputs, t0 + Duration::from_secs(3));
        assert_eq!(out.panel.as_deref(), Some("GPU 85°C"));
        assert!(out.flashing);
        assert_eq!(out.popup.as_deref(), Some("GPU 85°C"));

        let out = engine.check(&settings, &inputs, t0 + Duration::from_secs(14));
        assert!(!out.flashing, "le clignotement s'arrête après 10 s");
        assert!(out.popup.is_none(), "le pop-up disparaît après 5 s");
        assert_eq!(
            out.panel.as_deref(),
            Some("GPU 85°C"),
            "la ligne reste tant que la cause dure"
        );
    }

    #[test]
    fn short_spike_never_fires() {
        let settings = Settings {
            gpu_temp: true,
            gpu_temp_above: 80,
            ..Default::default()
        };
        let mut engine = Engine::default();
        let t0 = Instant::now();
        let hot = hot_gpu(90.0);
        let cool = hot_gpu(60.0);
        engine.check(
            &settings,
            &Inputs {
                stats: Some(&hot),
                ..Default::default()
            },
            t0,
        );
        engine.check(
            &settings,
            &Inputs {
                stats: Some(&cool),
                ..Default::default()
            },
            t0 + Duration::from_secs(2),
        );
        let out = engine.check(
            &settings,
            &Inputs {
                stats: Some(&hot),
                ..Default::default()
            },
            t0 + Duration::from_secs(4),
        );
        assert_eq!(
            out.panel, None,
            "le compteur repart de zéro après une baisse"
        );
    }
}
