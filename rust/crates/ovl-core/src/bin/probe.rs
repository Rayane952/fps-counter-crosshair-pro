//! Diagnostic : vérifie chaque source de mesure sur ce PC et affiche ce qu'elle renvoie.
//! `probe.exe [secondes] [hôte]`

use std::time::Duration;

use ovl_core::stat::{Stat, Wanted};
use ovl_core::{frames, hud, net, Sampler};

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let host = std::env::args().nth(2).unwrap_or_else(|| "1.1.1.1".into());

    let library = ovl_core::detect::Library::load();
    println!("bibliothèques: {:?}", library.summary());
    for exe in [
        r"D:\Riot Games\VALORANT\live\ShooterGame\Binaries\Win64\VALORANT-Win64-Shipping.exe",
        r"D:\SteamLibrary\steamapps\common\Deadlock\game\bin\win64\deadlock.exe",
        r"C:\Program Files\Epic Games\Fortnite\FortniteGame\Binaries\Win64\FortniteClient-Win64-Shipping.exe",
        r"D:\World of Warcraft\_forever_\Wow.exe",
        r"C:\Windows\System32\notepad.exe",
    ] {
        println!("  {:?} <- {exe}", library.classify(exe));
    }
    println!("etw étapes: {}", frames::diagnose());
    println!(
        "connexion WinRT: {:?}",
        windows::Networking::Connectivity::NetworkInformation::GetInternetConnectionProfile()
            .map(|p| p.ProfileName().map(|n| n.to_string()))
    );
    let status = frames::start();
    println!("etw: {status:?}");
    frames::set_network(true);
    println!("etw réseau: {}", frames::network_on());
    net::want(true, true, &host);

    let all = Wanted::new(Stat::BASIC.into_iter().chain(Stat::ADVANCED));
    let mut sampler = Sampler::new();
    sampler.sample(&all, 0);

    for _ in 0..seconds {
        std::thread::sleep(Duration::from_secs(1));
        let pid = ovl_core::games::foreground_pid();
        let stats = sampler.sample(&all, pid);
        let active = frames::active_pids();
        let busiest = active.first().map(|(p, _)| *p).unwrap_or(0);
        let reading = frames::read(busiest);
        let traffic = net::traffic::take(busiest);
        let n = net::reading();
        let clock = hud::clock_now();
        let snap = hud::Snapshot {
            stats: Some(&stats),
            frames: reading,
            net: Some(&n),
            traffic: Some(&traffic),
            clock: Some(&clock),
        };
        let rows = hud::rows(
            &Stat::BASIC
                .into_iter()
                .chain(Stat::ADVANCED)
                .collect::<Vec<_>>(),
            &snap,
        );
        println!("--- premier plan pid={pid}  pids avec images={active:?}");
        println!("    frames(pid {busiest}) = {reading:?}");
        println!(
            "    gpu temp={:?} fan={:?} vram={:?}/{:?} ghz={:?} disque={:?}/{:?} coeurs={}",
            stats.gpu_temp_c,
            stats.gpu_fan_rpm,
            stats.vram_used_gb,
            stats.vram_total_gb,
            stats.cpu_ghz,
            stats.disk_busy,
            stats.disk_mbps,
            stats.core_loads.as_ref().map_or(0, |c| c.len())
        );
        println!(
            "    net ping={:?} qualité={:?} connexion={:?} trafic={traffic:?}",
            n.ping_ms, n.quality, n.connection
        );
        for row in rows {
            println!("    | {:<9} {:?}", row.label, row.cell);
        }
    }
    frames::stop();
}
