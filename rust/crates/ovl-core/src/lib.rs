//! Télémétrie : CPU, mémoire, GPU, disque, images, réseau. Aucune dépendance UI.

pub mod alerts;
pub mod amd;
pub mod cpu;
pub mod frames;
pub mod games;
pub mod gpu;
pub mod hud;
pub mod net;
pub mod pdh;
pub mod stat;

use pdh::{Counter, Query};
use serde::Serialize;
use stat::{Stat, Wanted};

/// Instantané des mesures système (les images et le réseau ont leurs propres sources).
#[derive(Debug, Default, Clone, Serialize)]
pub struct Stats {
    pub cpu_load: Option<f32>,
    pub cpu_temp_c: Option<f32>,
    pub cpu_ghz: Option<f32>,
    /// Une valeur 0..1 par cœur logique.
    pub core_loads: Option<Vec<f32>>,
    pub gpu_load: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub gpu_fan_rpm: Option<u32>,
    pub vram_used_gb: Option<f32>,
    pub vram_total_gb: Option<f32>,
    pub game_vram_gb: Option<f32>,
    pub ram_used: u64,
    pub ram_total: u64,
    /// Disque le plus occupé, 0..1.
    pub disk_busy: Option<f32>,
    /// Lecture + écriture, tous disques, Mo/s.
    pub disk_mbps: Option<f32>,
}

impl Stats {
    pub fn ram_ratio(&self) -> Option<f32> {
        (self.ram_total > 0).then(|| self.ram_used as f32 / self.ram_total as f32)
    }
}

/// Un capteur absent n'est réinterrogé qu'après ce nombre d'échantillons (~1 min).
const RETRY_AFTER: u32 = 60;

#[derive(Default)]
struct Counters {
    cpu: Option<Counter>,
    cores: Option<Counter>,
    perf: Option<Counter>,
    freq: Option<Counter>,
    gpu_engines: Option<Counter>,
    gpu_memory: Option<Counter>,
    game_memory: Option<Counter>,
    disk_idle: Option<Counter>,
    disk_bytes: Option<Counter>,
}

/// Échantillonneur : une requête PDH, compteurs ajoutés à la première demande.
pub struct Sampler {
    query: Option<Query>,
    counters: Counters,
    vram_total: Option<f32>,
    gpu_sensor_wait: u32,
}

fn clamp01(v: f64) -> f32 {
    (v as f32).clamp(0.0, 1.0)
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            query: Query::new(),
            counters: Counters::default(),
            vram_total: gpu::vram_total_gb(),
            gpu_sensor_wait: 0,
        }
    }

    fn ensure(query: &Option<Query>, slot: &mut Option<Counter>, path: &str) {
        if slot.is_none() {
            if let Some(q) = query {
                *slot = q.add(path);
            }
        }
    }

    pub fn sample(&mut self, wanted: &Wanted, game_pid: u32) -> Stats {
        let q = &self.query;
        let c = &mut self.counters;
        Self::ensure(
            q,
            &mut c.cpu,
            r"\Processor Information(_Total)\% Processor Utility",
        );
        if wanted.has(Stat::Cores) {
            Self::ensure(
                q,
                &mut c.cores,
                r"\Processor Information(*)\% Processor Utility",
            );
        }
        if wanted.has(Stat::CpuClock) {
            Self::ensure(
                q,
                &mut c.perf,
                r"\Processor Information(_Total)\% Processor Performance",
            );
            Self::ensure(
                q,
                &mut c.freq,
                r"\Processor Information(_Total)\Processor Frequency",
            );
        }
        if wanted.has(Stat::Gpu) {
            Self::ensure(
                q,
                &mut c.gpu_engines,
                r"\GPU Engine(*)\Utilization Percentage",
            );
        }
        if wanted.has(Stat::Vram) {
            Self::ensure(
                q,
                &mut c.gpu_memory,
                r"\GPU Adapter Memory(*)\Dedicated Usage",
            );
        }
        if wanted.has(Stat::GameVram) {
            Self::ensure(
                q,
                &mut c.game_memory,
                r"\GPU Process Memory(*)\Dedicated Usage",
            );
        }
        if wanted.has(Stat::DiskLoad) {
            Self::ensure(q, &mut c.disk_idle, r"\PhysicalDisk(*)\% Idle Time");
        }
        if wanted.has(Stat::DiskSpeed) {
            Self::ensure(
                q,
                &mut c.disk_bytes,
                r"\PhysicalDisk(_Total)\Disk Bytes/sec",
            );
        }
        let collected = q.as_ref().is_some_and(|q| q.collect());

        let (ram_used, ram_total) = cpu::memory();
        let mut stats = Stats {
            ram_used,
            ram_total,
            vram_total_gb: self.vram_total,
            cpu_temp_c: wanted.has(Stat::CpuTemp).then(amd::cpu_temp_auto).flatten(),
            ..Default::default()
        };
        if !collected {
            return stats;
        }

        stats.cpu_load = c.cpu.and_then(|k| k.value()).map(|v| clamp01(v / 100.0));

        if let (true, Some(k)) = (wanted.has(Stat::Cores), c.cores) {
            let mut cores: Vec<(u32, u32, f32)> = k
                .values()
                .into_iter()
                .filter(|(name, _)| !name.contains("_Total"))
                .filter_map(|(name, v)| {
                    let (group, index) = name.split_once(',')?;
                    Some((group.parse().ok()?, index.parse().ok()?, clamp01(v / 100.0)))
                })
                .collect();
            cores.sort_by_key(|(g, i, _)| (*g, *i));
            stats.core_loads =
                (!cores.is_empty()).then(|| cores.into_iter().map(|(_, _, v)| v).collect());
        }

        if let (Some(perf), Some(freq)) = (
            c.perf.and_then(|k| k.value()),
            c.freq.and_then(|k| k.value()),
        ) {
            if freq > 0.0 {
                stats.cpu_ghz = Some((perf / 100.0 * freq / 1000.0) as f32);
            }
        }

        if let Some(k) = c.gpu_engines {
            let mut engines: std::collections::HashMap<String, f64> =
                std::collections::HashMap::new();
            for (name, value) in k.values() {
                *engines.entry(gpu::engine_key(&name)).or_default() += value;
            }
            stats.gpu_load = engines
                .values()
                .copied()
                .reduce(f64::max)
                .map(|v| clamp01(v / 100.0));
        }

        if let Some(k) = c.gpu_memory {
            stats.vram_used_gb = k
                .values()
                .into_iter()
                .map(|(_, v)| v)
                .reduce(f64::max)
                .map(|b| (b / 1_073_741_824.0) as f32);
        }

        if let (true, Some(k)) = (game_pid != 0, c.game_memory) {
            let tag = format!("pid_{game_pid}_");
            let bytes: f64 = k
                .values()
                .into_iter()
                .filter(|(n, _)| n.starts_with(&tag))
                .map(|(_, v)| v)
                .sum();
            stats.game_vram_gb = (bytes > 0.0).then(|| (bytes / 1_073_741_824.0) as f32);
        }

        if let Some(k) = c.disk_idle {
            let min_idle = k
                .values()
                .into_iter()
                .filter(|(n, _)| n != "_Total")
                .map(|(_, v)| v)
                .reduce(f64::min);
            stats.disk_busy = min_idle.map(|idle| clamp01(1.0 - idle / 100.0));
        }
        stats.disk_mbps = c
            .disk_bytes
            .and_then(|k| k.value())
            .map(|b| (b.max(0.0) / 1_048_576.0) as f32);

        if wanted.has(Stat::GpuTemp) {
            if self.gpu_sensor_wait > 0 {
                self.gpu_sensor_wait -= 1;
            } else {
                let sensors = gpu::sensors();
                stats.gpu_temp_c = sensors.temperature_c;
                stats.gpu_fan_rpm = sensors.fan_rpm;
                if sensors.temperature_c.is_none() {
                    self.gpu_sensor_wait = RETRY_AFTER;
                }
            }
        }
        stats
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}
