//! Contenu et apparence du HUD, partagés par l'overlay (GDI) et l'aperçu des réglages (egui).
//! Un seul formatage : ce que montre l'aperçu est exactement ce que montre le jeu.

use serde::{Deserialize, Serialize};

use crate::frames::Reading as Frames;
use crate::net::{self, traffic, Kind};
use crate::stat::Stat;
use crate::Stats;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// Une mesure par ligne, libellés alignés à gauche.
    #[default]
    Panel,
    /// Toutes les mesures sur une seule ligne.
    Line,
    /// Panneau + courbe des temps d'image.
    Graph,
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Panel, Layout::Line, Layout::Graph];

    pub fn name(self) -> &'static str {
        match self {
            Layout::Panel => "Panneau",
            Layout::Line => "Ligne",
            Layout::Graph => "Graphique",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub const ALL: [Corner; 4] = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Corner::TopLeft => "Haut gauche",
            Corner::TopRight => "Haut droite",
            Corner::BottomLeft => "Bas gauche",
            Corner::BottomRight => "Bas droite",
        }
    }

    pub fn is_left(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::BottomLeft)
    }

    pub fn is_top(self) -> bool {
        matches!(self, Corner::TopLeft | Corner::TopRight)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeId {
    #[default]
    Graphite,
    Light,
    Contrast,
    Clear,
    Green,
    Amber,
    Cyan,
    Rose,
    Violet,
    Coral,
}

/// Couleurs RGB du panneau.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub background: [u8; 3],
    pub label: [u8; 3],
    pub value: [u8; 3],
    /// Valeur secondaire (temps d'image, température à côté de la charge).
    pub detail: [u8; 3],
    pub alert: [u8; 3],
    /// Fond entièrement transparent, texte détouré d'une ombre.
    pub clear: bool,
}

const fn rgb(v: u32) -> [u8; 3] {
    [(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

const fn dark(background: u32, label: u32, value: u32, detail: u32) -> Palette {
    Palette {
        background: rgb(background),
        label: rgb(label),
        value: rgb(value),
        detail: rgb(detail),
        alert: rgb(0xFF5A55),
        clear: false,
    }
}

impl ThemeId {
    pub const ALL: [ThemeId; 10] = [
        ThemeId::Graphite,
        ThemeId::Light,
        ThemeId::Contrast,
        ThemeId::Clear,
        ThemeId::Green,
        ThemeId::Amber,
        ThemeId::Cyan,
        ThemeId::Rose,
        ThemeId::Violet,
        ThemeId::Coral,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ThemeId::Graphite => "Graphite",
            ThemeId::Light => "Clair",
            ThemeId::Contrast => "Contraste élevé",
            ThemeId::Clear => "Transparent",
            ThemeId::Green => "Vert",
            ThemeId::Amber => "Ambre",
            ThemeId::Cyan => "Cyan",
            ThemeId::Rose => "Rose",
            ThemeId::Violet => "Violet",
            ThemeId::Coral => "Corail",
        }
    }

    /// Contrastes vérifiés sur le fond opaque : valeurs > 10:1, libellés > 5:1.
    pub fn palette(self) -> Palette {
        match self {
            ThemeId::Graphite => dark(0x12141A, 0x9EA4AF, 0xF2F3F5, 0xBEC3CB),
            ThemeId::Light => Palette {
                background: rgb(0xF4F5F7),
                label: rgb(0x4F5560),
                value: rgb(0x14161A),
                detail: rgb(0x3A3F48),
                alert: rgb(0xC42B2B),
                clear: false,
            },
            ThemeId::Contrast => dark(0x000000, 0xFFD54A, 0xFFFFFF, 0xE6E6E6),
            ThemeId::Clear => Palette {
                clear: true,
                ..dark(0x000000, 0xE3E6EB, 0xFFFFFF, 0xE3E6EB)
            },
            ThemeId::Green => dark(0x08140D, 0x5CC77D, 0x8BF5A8, 0x6FD98F),
            ThemeId::Amber => dark(0x181107, 0xD9A245, 0xFFC766, 0xE8B254),
            ThemeId::Cyan => dark(0x07131A, 0x5DB9D6, 0x9AE8FF, 0x7DD0EA),
            ThemeId::Rose => dark(0x1A0B13, 0xD97AA4, 0xFFA6CC, 0xE88DB5),
            ThemeId::Violet => dark(0x120D1F, 0xA88EDB, 0xD6BEFF, 0xBFA6F0),
            ThemeId::Coral => dark(0x1A0E09, 0xD97C5A, 0xFFA286, 0xEB8E6E),
        }
    }

    /// Palette avec la couleur de texte choisie par l'utilisateur (valeurs et détails).
    pub fn palette_with(self, text: Option<[u8; 3]>) -> Palette {
        let mut p = self.palette();
        if let Some(c) = text {
            p.value = c;
            p.detail = c;
        }
        p
    }
}

/// Parse « #RRGGBB ».
pub fn parse_color(s: &str) -> Option<[u8; 3]> {
    let hex = s.strip_prefix('#')?;
    (hex.len() == 6).then_some(())?;
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some(rgb(v))
}

pub fn format_color(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text {
        value: String,
        detail: Option<String>,
    },
    /// Une barre 0..1 par cœur.
    Bars(Vec<f32>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub label: &'static str,
    pub cell: Cell,
}

impl Row {
    fn text(label: &'static str, value: String, detail: Option<String>) -> Self {
        Self {
            label,
            cell: Cell::Text { value, detail },
        }
    }

    pub fn value(&self) -> Option<&str> {
        match &self.cell {
            Cell::Text { value, .. } => Some(value),
            Cell::Bars(_) => None,
        }
    }
}

/// Tout ce que le HUD peut afficher à un instant donné.
#[derive(Debug, Default, Clone, Copy)]
pub struct Snapshot<'a> {
    pub stats: Option<&'a Stats>,
    pub frames: Option<Frames>,
    pub net: Option<&'a net::Reading>,
    pub traffic: Option<&'a traffic::Reading>,
    /// « HH:MM ».
    pub clock: Option<&'a str>,
}

fn comma(s: String) -> String {
    s.replace('.', ",")
}

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

fn celsius(v: f32) -> String {
    format!("{v:.0}°C")
}

fn gb(bytes: u64) -> f32 {
    bytes as f32 / 1_073_741_824.0
}

fn mbps(v: f32) -> String {
    if v < 10.0 {
        comma(format!("{v:.1} Mb/s"))
    } else {
        format!("{v:.0} Mb/s")
    }
}

/// Lignes affichées, dans l'ordre canonique. Une mesure sans valeur n'apparaît pas :
/// le HUD ne montre jamais de « — ».
pub fn rows(wanted: &[Stat], snap: &Snapshot) -> Vec<Row> {
    let on = |s: Stat| wanted.contains(&s);
    let mut out = Vec::new();
    let stats = snap.stats;

    if let Some(f) = snap.frames {
        let frame_ms = comma(format!("{:.1} ms", f.frame_ms));
        match (on(Stat::Fps), on(Stat::FrameTime)) {
            (true, true) => out.push(Row::text("FPS", format!("{:.0}", f.fps), Some(frame_ms))),
            (true, false) => out.push(Row::text("FPS", format!("{:.0}", f.fps), None)),
            (false, true) => out.push(Row::text("Image", frame_ms, None)),
            _ => {}
        }
        if on(Stat::Lows) {
            if let Some(l) = f.low_1pct {
                out.push(Row::text("1 %", format!("{l:.0}"), None));
            }
            if let Some(l) = f.low_01pct {
                out.push(Row::text("0,1 %", format!("{l:.0}"), None));
            }
        }
    }

    if let Some(s) = stats {
        let mut cpu_detail: Vec<String> = Vec::new();
        if on(Stat::CpuTemp) {
            cpu_detail.extend(s.cpu_temp_c.map(celsius));
        }
        if on(Stat::CpuClock) {
            cpu_detail.extend(s.cpu_ghz.map(|g| comma(format!("{g:.1} GHz"))));
        }
        let cpu_detail = (!cpu_detail.is_empty()).then(|| cpu_detail.join("  "));
        match (on(Stat::Cpu).then_some(s.cpu_load).flatten(), cpu_detail) {
            (Some(l), d) => out.push(Row::text("CPU", pct(l), d)),
            (None, Some(d)) => out.push(Row::text("CPU", d, None)),
            _ => {}
        }
        if let (true, Some(cores)) = (on(Stat::Cores), &s.core_loads) {
            out.push(Row {
                label: "Cœurs",
                cell: Cell::Bars(cores.clone()),
            });
        }

        let gpu_temp = on(Stat::GpuTemp)
            .then_some(s.gpu_temp_c)
            .flatten()
            .map(celsius);
        match (on(Stat::Gpu).then_some(s.gpu_load).flatten(), gpu_temp) {
            (Some(l), t) => out.push(Row::text("GPU", pct(l), t)),
            (None, Some(t)) => out.push(Row::text("GPU", t, None)),
            _ => {}
        }
        if let (true, Some(used)) = (on(Stat::Vram), s.vram_used_gb) {
            let value = match s.vram_total_gb {
                Some(total) => comma(format!("{used:.1} / {total:.0} Go")),
                None => comma(format!("{used:.1} Go")),
            };
            out.push(Row::text("VRAM", value, None));
        }
        if let (true, Some(v)) = (on(Stat::GameVram), s.game_vram_gb) {
            out.push(Row::text("VRAM jeu", comma(format!("{v:.1} Go")), None));
        }
        if on(Stat::Ram) && s.ram_total > 0 {
            out.push(Row::text(
                "RAM",
                comma(format!("{:.1} / {:.0} Go", gb(s.ram_used), gb(s.ram_total))),
                None,
            ));
        }

        let disk_speed = on(Stat::DiskSpeed)
            .then_some(s.disk_mbps)
            .flatten()
            .map(|m| format!("{m:.0} Mo/s"));
        match (
            on(Stat::DiskLoad).then_some(s.disk_busy).flatten(),
            disk_speed,
        ) {
            (Some(b), d) => out.push(Row::text("Disque", pct(b), d)),
            (None, Some(d)) => out.push(Row::text("Disque", d, None)),
            _ => {}
        }
    }

    if let Some(n) = snap.net {
        let quality = on(Stat::PacketLoss).then_some(n.quality).flatten();
        let loss_text = quality.map(|q| comma(format!("{:.1} %", q.loss_percent)));
        match (on(Stat::Ping).then_some(n.ping_ms).flatten(), loss_text) {
            (Some(p), loss) => out.push(Row::text(
                "Ping",
                format!("{p:.0} ms"),
                loss.map(|l| format!("perte {l}")),
            )),
            (None, Some(loss)) => {
                let jitter = quality
                    .and_then(|q| q.jitter_ms)
                    .map(|j| format!("gigue {j:.0} ms"));
                out.push(Row::text("Perte", loss, jitter));
            }
            _ => {}
        }
        if let (true, Some(c)) = (on(Stat::Connection), n.connection) {
            let value = match (c.kind, c.bars) {
                (Kind::Offline, _) => "Hors ligne".to_owned(),
                (Kind::WiFi, Some(b)) => format!("Wi-Fi {b}/5"),
                (Kind::WiFi, None) => "Wi-Fi".to_owned(),
                (Kind::Mobile, Some(b)) => format!("Mobile {b}/5"),
                (Kind::Mobile, None) => "Mobile".to_owned(),
                (Kind::Ethernet, _) => "Ethernet".to_owned(),
                (Kind::Other, _) => "Connecté".to_owned(),
            };
            let link = c.link_mbps.map(|m| {
                if m >= 1000.0 {
                    comma(format!("{:.1} Gb/s", m / 1000.0)).replace(",0 ", " ")
                } else {
                    format!("{m:.0} Mb/s")
                }
            });
            out.push(Row::text("Réseau", value, link));
        }
    }

    if let (true, Some(t)) = (on(Stat::BgTraffic), snap.traffic) {
        let detail = t
            .top
            .as_ref()
            .filter(|(_, m)| *m >= 0.1)
            .map(|(name, _)| name.clone());
        out.push(Row::text("Arr.-plan", mbps(t.total_mbps), detail));
    }

    if let (true, Some(clock)) = (on(Stat::Clock), snap.clock) {
        out.push(Row::text("Heure", clock.to_owned(), None));
    }
    out
}

/// Heure locale « HH:MM ».
pub fn clock_now() -> String {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format!("{:02}:{:02}", t.wHour, t.wMinute)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_values_are_hidden() {
        let stats = Stats {
            cpu_load: Some(0.27),
            ..Default::default()
        };
        let snap = Snapshot {
            stats: Some(&stats),
            ..Default::default()
        };
        let rows = rows(&Stat::DEFAULT, &snap);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], Row::text("CPU", "27%".into(), None));
    }

    #[test]
    fn details_follow_their_value() {
        let stats = Stats {
            cpu_load: Some(0.5),
            cpu_temp_c: Some(61.4),
            ram_used: 13 * 1_073_741_824,
            ram_total: 32 * 1_073_741_824,
            ..Default::default()
        };
        let frames = Frames {
            fps: 143.6,
            frame_ms: 6.96,
            low_1pct: Some(98.0),
            low_01pct: None,
        };
        let snap = Snapshot {
            stats: Some(&stats),
            frames: Some(frames),
            ..Default::default()
        };
        let rows = rows(
            &[
                Stat::Fps,
                Stat::FrameTime,
                Stat::Lows,
                Stat::Cpu,
                Stat::CpuTemp,
                Stat::Ram,
            ],
            &snap,
        );
        assert_eq!(
            rows[0],
            Row::text("FPS", "144".into(), Some("7,0 ms".into()))
        );
        assert_eq!(rows[1], Row::text("1 %", "98".into(), None));
        assert_eq!(rows[2], Row::text("CPU", "50%".into(), Some("61°C".into())));
        assert_eq!(rows[3], Row::text("RAM", "13,0 / 32 Go".into(), None));
    }

    #[test]
    fn unchecked_stats_stay_hidden() {
        let stats = Stats {
            cpu_load: Some(0.5),
            cpu_temp_c: Some(61.0),
            ..Default::default()
        };
        let snap = Snapshot {
            stats: Some(&stats),
            ..Default::default()
        };
        let rows = rows(&[Stat::CpuTemp], &snap);
        assert_eq!(rows, vec![Row::text("CPU", "61°C".into(), None)]);
    }

    #[test]
    fn colors_round_trip() {
        assert_eq!(parse_color("#1A2B3C"), Some([0x1A, 0x2B, 0x3C]));
        assert_eq!(format_color([0x1A, 0x2B, 0x3C]), "#1A2B3C");
        assert_eq!(parse_color("1A2B3C"), None);
    }
}
