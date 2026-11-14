#![windows_subsystem = "windows"]
//! Point d'entrée : icône de notification, raccourci global, overlay affiché en jeu.

use std::io::Write;
use std::thread;
use std::time::{Duration, Instant};

use ovl_core::alerts::{self, Engine};
use ovl_core::frames::{self, Status as FrameStatus};
use ovl_core::games::GameProbe;
use ovl_core::hud::{self, Layout};
use ovl_core::net;
use ovl_core::stat::{Stat, Wanted};
use ovl_core::{Sampler, Stats};
use ovl_overlay::{HudSettings, Overlay};
use ovl_platform::{hotkeys, single_instance, Config, OverlayStatus};
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::GetCurrentProcess;

/// Seuil d'utilisation du moteur 3D par le processus au premier plan pour le considérer comme un jeu.
const GAME_3D_THRESHOLD: f32 = 3.0;
/// Mesures, alertes, état publié.
const TICK: Duration = Duration::from_millis(1000);
/// Rendu : courbe des temps d'image et clignotement d'alerte.
const FRAME: Duration = Duration::from_millis(250);
/// Une session ETW en échec est réessayée à ce rythme.
const ETW_RETRY: Duration = Duration::from_secs(30);
/// Largeur maximale de la courbe, en images.
const GRAPH_SAMPLES: usize = 600;

/// Journal minimal (les binaires n'ont pas de console).
fn log_line(message: &str) {
    let path = Config::dir().join("overlay.log");
    let _ = std::fs::create_dir_all(Config::dir());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{stamp}] {message}");
    }
}

fn working_set_bytes() -> u64 {
    unsafe {
        let mut counters = PROCESS_MEMORY_COUNTERS {
            cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb).is_ok() {
            counters.WorkingSetSize as u64
        } else {
            0
        }
    }
}

fn hud_settings(config: &Config) -> HudSettings {
    HudSettings {
        scale: config.scale,
        margin_x: config.margin_x,
        margin_y: config.margin_y,
        corner: config.corner,
        layout: config.layout,
        palette: config.theme.palette_with(config.text_rgb()),
        opacity: config.opacity,
    }
}

/// Démarre ou arrête les sources lourdes (ETW, réseau) selon ce qui est demandé.
struct Sources {
    etw_tried: Option<Instant>,
}

impl Sources {
    fn update(&mut self, wanted: &Wanted, config: &Config) -> Option<FrameStatus> {
        let need_frames =
            wanted.any(&[Stat::Fps, Stat::FrameTime, Stat::Lows]) || config.layout == Layout::Graph;
        let need_traffic = wanted.has(Stat::BgTraffic);
        let mut status = None;
        if need_frames || need_traffic {
            let current = frames::status();
            let retry = self.etw_tried.map_or(true, |t| t.elapsed() >= ETW_RETRY);
            status = Some(if current != FrameStatus::Running && retry {
                self.etw_tried = Some(Instant::now());
                let s = frames::start();
                if s != FrameStatus::Running {
                    log_line(&format!("session ETW des images : {s:?}"));
                }
                s
            } else {
                current
            });
            frames::set_network(need_traffic);
        }
        net::want(
            wanted.any(&[Stat::Ping, Stat::PacketLoss]),
            wanted.has(Stat::Connection),
            &config.ping_host,
        );
        status
    }
}

fn main() {
    env_logger::init();

    let Some(_instance) = single_instance::acquire("overlay") else {
        // déjà lancé : un second lancement ouvre plutôt les réglages
        ovl_platform::tray::open_settings();
        return;
    };

    let Some(mut overlay) = Overlay::create() else {
        log_line("création des fenêtres du HUD impossible");
        return;
    };

    let _tray = ovl_platform::tray::create();
    let hotkey_ok = hotkeys::install();
    if !hotkey_ok {
        log_line("raccourci Alt+Maj+O déjà utilisé par une autre application");
    }

    let mut sampler = Sampler::new();
    let mut game_probe = GameProbe::new();
    let mut sources = Sources { etw_tried: None };
    let mut engine = Engine::default();

    let mut config = Config::load();
    let mut stats = Stats::default();
    let mut frames_now = None;
    let mut traffic = None;
    let mut net_now = net::Reading::default();
    let mut alert = alerts::Output::default();
    let mut in_game = false;
    let mut pid = 0;

    let mut last_tick = Instant::now() - TICK * 2;
    let mut last_frame = Instant::now() - FRAME;
    let mut flash_phase = false;

    loop {
        let pumped = Overlay::pump();
        if pumped.quit {
            break;
        }
        let mut force = false;
        if pumped.hotkey == Some(hotkeys::TOGGLE_HUD) {
            let _ = Config::update(|c| c.show_stats = !c.show_stats);
            force = true;
        }
        if !force && last_frame.elapsed() < FRAME {
            thread::sleep(Duration::from_millis(15));
            continue;
        }
        last_frame = Instant::now();
        flash_phase = !flash_phase;

        if force || last_tick.elapsed() >= TICK {
            last_tick = Instant::now();
            // les réglages écrivent le fichier : relu à chaque tick (quelques centaines d'octets)
            config = Config::load();
            let wanted = config.wanted();
            let frame_status = sources.update(&wanted, &config);

            pid = ovl_core::games::foreground_pid();
            let usage = game_probe.as_mut().map(|p| p.usage(pid)).unwrap_or(0.0);
            in_game = usage >= GAME_3D_THRESHOLD;

            let active = config.show_stats && (in_game || config.always_show);
            if active || !config.alerts.needs().is_empty() {
                stats = sampler.sample(&wanted, pid);
            }
            frames_now = frames::read(pid);
            traffic = wanted.has(Stat::BgTraffic).then(|| net::traffic::take(pid));
            net_now = net::reading();

            alert = engine.check(
                &config.alerts,
                &alerts::Inputs {
                    in_game,
                    frames: frames_now,
                    stats: Some(&stats),
                    ping_ms: net_now.ping_ms,
                    quality: net_now.quality,
                    traffic: traffic.as_ref(),
                    connection: net_now.connection,
                },
                Instant::now(),
            );

            OverlayStatus {
                working_set_bytes: working_set_bytes(),
                hotkey_ok,
                in_game,
                frames_status: frame_status,
                frames: frames_now,
                foreground: (pid != 0).then(|| ovl_core::games::process_name(pid)),
                net: Some(net_now.clone()),
                traffic: traffic.clone(),
                graph: if config.layout == Layout::Graph {
                    frames::frame_times(pid, 300)
                } else {
                    Vec::new()
                },
                updated: 0,
            }
            .publish();
        }

        let settings = hud_settings(&config);
        let shown = in_game || config.always_show;
        if config.show_stats && shown {
            let clock = hud::clock_now();
            let snap = hud::Snapshot {
                stats: Some(&stats),
                frames: frames_now,
                net: Some(&net_now),
                traffic: traffic.as_ref(),
                clock: Some(&clock),
            };
            let rows = hud::rows(&config.stats, &snap);
            let graph = if config.layout == Layout::Graph {
                frames::frame_times(pid, GRAPH_SAMPLES)
            } else {
                Vec::new()
            };
            overlay.render(
                &settings,
                &rows,
                &graph,
                alert.panel.as_deref(),
                alert.flashing && flash_phase,
            );
        } else {
            overlay.hide();
        }
        overlay.popup(&settings, alert.popup.as_deref().filter(|_| shown));
    }

    frames::stop();
    hotkeys::uninstall();
    ovl_platform::tray::destroy();
}
