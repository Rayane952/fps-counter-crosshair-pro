//! Configuration persistante (%LOCALAPPDATA%\FPSOverlayRust\config.json).
//!
//! Trois processus écrivent ce fichier (overlay, réglages, icône de notification) :
//! chaque écriture passe par `Config::update`, qui relit le fichier, ne change que le champ
//! demandé puis remplace le fichier d'un coup. Ni écrasement d'un réglage fait ailleurs,
//! ni lecture d'un fichier à moitié écrit.

use std::path::{Path, PathBuf};

use ovl_core::alerts;
use ovl_core::detect::Rule;
use ovl_core::hud::{parse_color, Corner, Layout, ThemeId};
use ovl_core::stat::{Stat, Wanted};
use serde::{Deserialize, Serialize};

pub const SCALE_MIN: f32 = 0.75;
pub const SCALE_MAX: f32 = 2.0;
pub const MARGIN_MAX: i32 = 200;
pub const DEFAULT_MARGIN: i32 = 12;
pub const DEFAULT_PING_HOST: &str = "1.1.1.1";

fn unset() -> i32 {
    -1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Afficher le panneau en jeu.
    pub show_stats: bool,
    /// Afficher aussi hors jeu (pour régler la position sur le bureau).
    pub always_show: bool,
    /// Mesures cochées, dans l'ordre de `Stat::BASIC` puis `Stat::ADVANCED`.
    pub stats: Vec<Stat>,
    /// Échelle du panneau, multipliée par l'échelle Windows du moniteur.
    pub scale: f32,
    /// Distances aux bords, en pixels logiques.
    #[serde(default = "unset")]
    pub margin_x: i32,
    #[serde(default = "unset")]
    pub margin_y: i32,
    /// Ancien réglage unique (avant marges séparées) : lu, jamais réécrit.
    #[serde(skip_serializing)]
    margin: Option<i32>,
    /// Opacité du fond du panneau, 0..1 (le texte reste opaque).
    pub opacity: f32,
    pub corner: Corner,
    pub layout: Layout,
    pub theme: ThemeId,
    /// « #RRGGBB » ; absent = couleur du thème.
    pub text_color: Option<String>,
    /// Hôte du ping et de la perte de paquets.
    pub ping_host: String,
    pub alerts: alerts::Settings,
    /// Règles par exécutable : afficher toujours / jamais, quelle que soit la détection.
    pub game_rules: Vec<Rule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            show_stats: true,
            always_show: false,
            stats: Stat::DEFAULT.to_vec(),
            scale: 1.0,
            margin_x: DEFAULT_MARGIN,
            margin_y: DEFAULT_MARGIN,
            margin: None,
            opacity: 0.8,
            corner: Corner::default(),
            layout: Layout::default(),
            theme: ThemeId::default(),
            text_color: None,
            ping_host: DEFAULT_PING_HOST.to_owned(),
            alerts: alerts::Settings::default(),
            game_rules: Vec::new(),
        }
    }
}

/// Résultat d'une lecture, pour signaler un fichier illisible au lieu de l'ignorer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadIssue {
    /// Pas encore de fichier : valeurs par défaut, situation normale au premier lancement.
    Missing,
    /// Fichier présent mais invalide : valeurs par défaut utilisées.
    Invalid(String),
}

impl Config {
    pub fn dir() -> PathBuf {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join("FPSOverlayRust")
    }

    pub fn path() -> PathBuf {
        Self::dir().join("config.json")
    }

    /// Bornes appliquées à toute valeur lue ou écrite (fichier édité à la main compris).
    fn sanitized(mut self) -> Self {
        if !self.scale.is_finite() {
            self.scale = 1.0;
        }
        self.scale = self.scale.clamp(SCALE_MIN, SCALE_MAX);
        let legacy = self.margin.take().unwrap_or(DEFAULT_MARGIN);
        if self.margin_x < 0 {
            self.margin_x = legacy;
        }
        if self.margin_y < 0 {
            self.margin_y = legacy;
        }
        self.margin_x = self.margin_x.clamp(0, MARGIN_MAX);
        self.margin_y = self.margin_y.clamp(0, MARGIN_MAX);
        if !self.opacity.is_finite() {
            self.opacity = 0.8;
        }
        self.opacity = self.opacity.clamp(0.0, 1.0);
        let mut seen = Vec::new();
        self.stats.retain(|s| {
            let fresh = !seen.contains(s);
            seen.push(*s);
            fresh
        });
        if self
            .text_color
            .as_deref()
            .is_some_and(|c| parse_color(c).is_none())
        {
            self.text_color = None;
        }
        let host = self.ping_host.trim();
        self.ping_host = if host.is_empty() {
            DEFAULT_PING_HOST.to_owned()
        } else {
            host.to_owned()
        };
        self
    }

    pub fn text_rgb(&self) -> Option<[u8; 3]> {
        self.text_color.as_deref().and_then(parse_color)
    }

    pub fn shows(&self, stat: Stat) -> bool {
        self.stats.contains(&stat)
    }

    /// Mesures à collecter : celles affichées et celles surveillées par une alerte.
    pub fn wanted(&self) -> Wanted {
        Wanted::new(self.stats.iter().copied().chain(self.alerts.needs()))
    }

    pub fn load_checked() -> (Self, Option<LoadIssue>) {
        match std::fs::read_to_string(Self::path()) {
            Ok(text) => match serde_json::from_str::<Config>(&text) {
                Ok(config) => (config.sanitized(), None),
                Err(err) => (Self::default(), Some(LoadIssue::Invalid(err.to_string()))),
            },
            Err(_) => (Self::default(), Some(LoadIssue::Missing)),
        }
    }

    pub fn load() -> Self {
        Self::load_checked().0
    }

    /// Relit, modifie, réécrit. À utiliser pour tout changement de réglage.
    pub fn update(change: impl FnOnce(&mut Config)) -> std::io::Result<Config> {
        let mut config = Self::load();
        change(&mut config);
        let config = config.sanitized();
        config.save()?;
        Ok(config)
    }

    pub fn save(&self) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        write_atomic(&Self::path(), text.as_bytes())
    }
}

/// Écrit dans un fichier temporaire puis le renomme : un lecteur voit l'ancien ou le nouveau contenu.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Config {
        serde_json::from_str::<Config>(text).unwrap().sanitized()
    }

    #[test]
    fn old_files_keep_working() {
        let config =
            parse(r#"{ "show_stats": false, "always_show": true, "scale": 1.25, "margin": 30 }"#);
        assert!(!config.show_stats);
        assert_eq!(config.layout, Layout::Panel);
        assert_eq!(config.corner, Corner::TopRight);
        assert_eq!(
            (config.margin_x, config.margin_y),
            (30, 30),
            "l'ancienne marge unique est reprise"
        );
        assert_eq!(config.stats, Stat::DEFAULT.to_vec());
        let saved = serde_json::to_string(&config).unwrap();
        assert!(
            !saved.contains("\"margin\""),
            "l'ancien champ n'est plus écrit"
        );
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let config = Config {
            scale: 9.0,
            margin_x: -4,
            margin_y: 999,
            opacity: 3.0,
            ..Default::default()
        }
        .sanitized();
        assert_eq!(config.scale, SCALE_MAX);
        assert_eq!(config.margin_x, DEFAULT_MARGIN, "négatif = non défini");
        assert_eq!(config.margin_y, MARGIN_MAX);
        assert_eq!(config.opacity, 1.0);
    }

    #[test]
    fn alerts_add_what_they_watch() {
        let mut config = Config {
            stats: vec![Stat::Fps],
            ..Default::default()
        };
        config.alerts.gpu_temp = true;
        assert!(config.wanted().has(Stat::GpuTemp));
        assert!(!config.shows(Stat::GpuTemp));
    }
}
