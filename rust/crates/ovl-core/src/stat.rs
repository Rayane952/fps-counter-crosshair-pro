//! Mesures que l'utilisateur peut afficher dans le HUD.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stat {
    Fps,
    FrameTime,
    Cpu,
    CpuTemp,
    Gpu,
    GpuTemp,
    Vram,
    Ram,
    Clock,
    Lows,
    Cores,
    CpuClock,
    GameVram,
    DiskLoad,
    DiskSpeed,
    Ping,
    PacketLoss,
    Connection,
    BgTraffic,
}

impl Stat {
    pub const BASIC: [Stat; 9] = [
        Stat::Fps,
        Stat::FrameTime,
        Stat::Cpu,
        Stat::CpuTemp,
        Stat::Gpu,
        Stat::GpuTemp,
        Stat::Vram,
        Stat::Ram,
        Stat::Clock,
    ];

    pub const ADVANCED: [Stat; 10] = [
        Stat::Lows,
        Stat::Cores,
        Stat::CpuClock,
        Stat::DiskLoad,
        Stat::DiskSpeed,
        Stat::GameVram,
        Stat::Ping,
        Stat::PacketLoss,
        Stat::Connection,
        Stat::BgTraffic,
    ];

    /// Sélection par défaut au premier lancement.
    pub const DEFAULT: [Stat; 7] = [
        Stat::Fps,
        Stat::FrameTime,
        Stat::Cpu,
        Stat::CpuTemp,
        Stat::Gpu,
        Stat::GpuTemp,
        Stat::Ram,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Stat::Fps => "FPS",
            Stat::FrameTime => "Temps d'image",
            Stat::Cpu => "Charge du CPU",
            Stat::CpuTemp => "Température du CPU",
            Stat::Gpu => "Charge du GPU",
            Stat::GpuTemp => "Température du GPU",
            Stat::Vram => "Mémoire du GPU",
            Stat::Ram => "RAM",
            Stat::Clock => "Horloge",
            Stat::Lows => "Lows 1 % et 0,1 %",
            Stat::Cores => "Chaque cœur du CPU",
            Stat::CpuClock => "Fréquence du CPU",
            Stat::GameVram => "Mémoire GPU du jeu",
            Stat::DiskLoad => "Activité du disque",
            Stat::DiskSpeed => "Débit du disque",
            Stat::Ping => "Ping",
            Stat::PacketLoss => "Perte de paquets",
            Stat::Connection => "Connexion et signal Wi-Fi",
            Stat::BgTraffic => "Trafic en arrière-plan",
        }
    }

    /// Ce que la case affiche concrètement, en une ligne.
    pub fn hint(self) -> &'static str {
        match self {
            Stat::Fps => "Images par seconde du jeu au premier plan.",
            Stat::FrameTime => "Durée moyenne d'une image, à côté des FPS.",
            Stat::Cpu => "Utilisation du processeur, comme le Gestionnaire des tâches.",
            Stat::CpuTemp => "Capteur AMD publié par sensor.exe.",
            Stat::Gpu => "Moteur graphique le plus occupé.",
            Stat::GpuTemp => "Température publiée par le pilote graphique.",
            Stat::Vram => "Mémoire vidéo dédiée utilisée / totale.",
            Stat::Ram => "Mémoire système utilisée / totale.",
            Stat::Clock => "Heure locale.",
            Stat::Lows => "FPS des 1 % et 0,1 % d'images les plus lentes (10 s).",
            Stat::Cores => "Une barre par cœur logique.",
            Stat::CpuClock => "Fréquence effective moyenne.",
            Stat::GameVram => "Mémoire vidéo utilisée par le jeu seul.",
            Stat::DiskLoad => "Disque le plus occupé.",
            Stat::DiskSpeed => "Lecture + écriture, tous disques.",
            Stat::Ping => "Aller-retour vers l'hôte choisi.",
            Stat::PacketLoss => "Pertes et gigue sur la dernière minute.",
            Stat::Connection => "Type de connexion, barres Wi-Fi, débit du lien.",
            Stat::BgTraffic => "Débit des autres programmes que le jeu.",
        }
    }
}

/// Mesures demandées : HUD + alertes actives.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Wanted(Vec<Stat>);

impl Wanted {
    pub fn new(stats: impl IntoIterator<Item = Stat>) -> Self {
        let mut list: Vec<Stat> = Vec::new();
        for stat in stats {
            if !list.contains(&stat) {
                list.push(stat);
            }
        }
        Self(list)
    }

    pub fn has(&self, stat: Stat) -> bool {
        self.0.contains(&stat)
    }

    pub fn any(&self, stats: &[Stat]) -> bool {
        stats.iter().any(|s| self.has(*s))
    }

    pub fn with(mut self, stat: Stat) -> Self {
        if !self.has(stat) {
            self.0.push(stat);
        }
        self
    }
}
