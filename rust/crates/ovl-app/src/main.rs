#![windows_subsystem = "windows"]
//! Point d'entrée : icône de notification, réglages, overlay affiché en jeu.

use std::thread;
use std::time::{Duration, Instant};

use ovl_core::games::GameProbe;
use ovl_core::{Sampler, Stats};
use ovl_overlay::{Overlay, OverlayConfig};
use ovl_platform::Config;

fn main() {
    env_logger::init();

    let mut overlay = match Overlay::create(OverlayConfig::default()) {
        Some(overlay) => overlay,
        None => {
            let _ = std::fs::write(r"C:ProgramDataOverlayTempust_overlay.log", "create failed
");
            return;
        }
    };
    let _ = std::fs::write(r"C:ProgramDataOverlayTempust_overlay.log", "create ok
");
    overlay.hide();

    let _tray = ovl_platform::tray::create();

    let mut sampler = Sampler::new();
    sampler.sample(); // amorce PDH
    let mut game_probe = GameProbe::new();

    let mut config = Config::load();
    overlay.set_scale(config.scale);

    let mut last = Instant::now() - Duration::from_secs(2);
    let mut visible = false;

    loop {
        if !Overlay::pump() {
            break;
        }

        if last.elapsed() < Duration::from_millis(1000) {
            thread::sleep(Duration::from_millis(30));
            continue;
        }
        last = Instant::now();

        // les réglages écrivent le fichier : on le relit à chaque tick (coût négligeable)
        config = Config::load();
        overlay.set_scale(config.scale);

        if !config.show_stats {
            if visible {
                overlay.hide();
                visible = false;
            }
            continue;
        }

        let stats = sampler.sample();
        let pid = ovl_core::games::foreground_pid();
        let usage = game_probe.as_mut().map(|p| p.usage(pid)).unwrap_or(0.0);
        let in_game = config.always_show || usage >= 3.0;

        if in_game {
            if !visible {
                overlay.show();
                visible = true;
            }
            overlay.follow_foreground_monitor();
            overlay.paint(&rows(&stats));
        } else if visible {
            overlay.hide();
            visible = false;
        }
    }

    ovl_platform::tray::destroy();
}

/// Lignes affichées (label, valeur).
fn rows(stats: &Stats) -> Vec<(String, String)> {
    let percent = |value: Option<f32>| {
        value.map(|v| format!("{:.0}%", v * 100.0)).unwrap_or_else(|| "—".to_string())
    };
    let celsius = |value: Option<f32>| {
        value.map(|v| format!("{v:.0}°C")).unwrap_or_else(|| "—".to_string())
    };

    vec![
        ("FPS".to_string(), "—".to_string()),
        ("CPU".to_string(), percent(stats.cpu_load)),
        ("TEMP".to_string(), celsius(stats.cpu_temp_c)),
        ("GPU".to_string(), percent(stats.gpu_load)),
        ("GTEMP".to_string(), celsius(stats.gpu_temp_c)),
        ("RAM".to_string(), format!("{:.1} Go", stats.ram_used as f32 / 1_073_741_824.0)),
    ]
}