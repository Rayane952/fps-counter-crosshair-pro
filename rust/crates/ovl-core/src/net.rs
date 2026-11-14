//! Réseau : ping, perte et gigue (ICMP), type de connexion et signal (WinRT), trafic par processus (ETW).

use std::collections::VecDeque;
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use windows::Networking::Connectivity::NetworkInformation;
use windows::Win32::NetworkManagement::IpHelper::{
    IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY,
};

/// Mesures réseau publiées par le thread de fond.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Reading {
    pub host: String,
    /// Dernier aller-retour réussi ; `None` si le dernier ping a échoué.
    pub ping_ms: Option<f32>,
    pub quality: Option<Quality>,
    pub connection: Option<Connection>,
    /// L'hôte n'a pas pu être résolu.
    pub host_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Quality {
    pub loss_percent: f32,
    pub jitter_ms: Option<f32>,
    /// Perte récente, perte ≥ 2 % ou gigue ≥ 30 ms.
    pub trouble: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Offline,
    Ethernet,
    WiFi,
    Mobile,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Connection {
    pub kind: Kind,
    /// 0..5 pour le Wi-Fi et le mobile.
    pub bars: Option<u8>,
    pub link_mbps: Option<f32>,
}

impl Connection {
    pub fn weak(&self) -> bool {
        self.kind == Kind::Offline || self.bars.is_some_and(|b| b <= 1)
    }
}

/// Une minute d'historique à un ping par seconde.
const HISTORY: usize = 60;
const RECENT: usize = 10;
const MIN_SAMPLES: usize = 5;
const TROUBLE_LOSS: f32 = 2.0;
const TROUBLE_JITTER: f32 = 30.0;
const CONNECTION_EVERY: Duration = Duration::from_secs(5);

/// Perte et gigue à partir de l'historique (None = pas de réponse).
pub fn quality(history: &[Option<f32>]) -> Option<Quality> {
    if history.len() < MIN_SAMPLES {
        return None;
    }
    let lost = history.iter().filter(|h| h.is_none()).count();
    let loss_percent = 100.0 * lost as f32 / history.len() as f32;
    let answered: Vec<f32> = history.iter().flatten().copied().collect();
    let jitter_ms = (answered.len() >= 2).then(|| {
        answered
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .sum::<f32>()
            / (answered.len() - 1) as f32
    });
    let recent_loss = history.iter().rev().take(RECENT).any(|h| h.is_none());
    let trouble = recent_loss
        || loss_percent >= TROUBLE_LOSS
        || jitter_ms.is_some_and(|j| j >= TROUBLE_JITTER);
    Some(Quality {
        loss_percent,
        jitter_ms,
        trouble,
    })
}

#[derive(Default)]
struct Want {
    ping: bool,
    connection: bool,
    host: String,
}

struct Shared {
    want: Want,
    reading: Reading,
    running: bool,
}

static SHARED: LazyLock<Mutex<Shared>> = LazyLock::new(|| {
    Mutex::new(Shared {
        want: Want::default(),
        reading: Reading::default(),
        running: false,
    })
});

/// Déclare ce dont l'appelant a besoin ; démarre le thread de fond à la première demande.
pub fn want(ping: bool, connection: bool, host: &str) {
    let mut shared = SHARED.lock().unwrap();
    if shared.want.host != host {
        shared.reading = Reading {
            host: host.to_owned(),
            ..Default::default()
        };
    }
    shared.want = Want {
        ping,
        connection,
        host: host.to_owned(),
    };
    if (ping || connection) && !shared.running {
        shared.running = true;
        std::thread::Builder::new()
            .name("net".into())
            .spawn(worker)
            .ok();
    }
}

pub fn reading() -> Reading {
    SHARED.lock().unwrap().reading.clone()
}

fn resolve(host: &str) -> Option<u32> {
    (host, 0)
        .to_socket_addrs()
        .ok()?
        .find_map(|a| match a.ip() {
            IpAddr::V4(v4) => Some(u32::from_ne_bytes(v4.octets())),
            IpAddr::V6(_) => None,
        })
}

/// Un ping ICMP (IPv4), délai 1 s. `None` si pas de réponse.
fn ping_once(address: u32) -> Option<f32> {
    unsafe {
        let handle = IcmpCreateFile().ok()?;
        let payload = [0x61u8; 32];
        let mut reply = vec![0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + payload.len() + 8 + 32];
        let n = IcmpSendEcho(
            handle,
            address,
            payload.as_ptr() as *const _,
            payload.len() as u16,
            None,
            reply.as_mut_ptr() as *mut _,
            reply.len() as u32,
            1000,
        );
        let _ = IcmpCloseHandle(handle);
        if n == 0 {
            return None;
        }
        let echo = &*(reply.as_ptr() as *const ICMP_ECHO_REPLY);
        (echo.Status == 0).then_some(echo.RoundTripTime as f32)
    }
}

fn read_connection() -> Option<Connection> {
    let profile = match NetworkInformation::GetInternetConnectionProfile() {
        Ok(p) => p,
        // pas de profil Internet : Windows renvoie un pointeur nul
        Err(e) if e.code().is_ok() || e.code() == windows::Win32::Foundation::E_POINTER => {
            return Some(Connection {
                kind: Kind::Offline,
                bars: None,
                link_mbps: None,
            })
        }
        // autre erreur (accès refusé…) : état inconnu, surtout pas « hors ligne »
        Err(_) => return None,
    };
    let adapter = profile.NetworkAdapter().ok();
    let kind = if profile.IsWlanConnectionProfile().unwrap_or(false) {
        Kind::WiFi
    } else if profile.IsWwanConnectionProfile().unwrap_or(false) {
        Kind::Mobile
    } else if adapter.as_ref().and_then(|a| a.IanaInterfaceType().ok()) == Some(6) {
        Kind::Ethernet
    } else {
        Kind::Other
    };
    let bars = matches!(kind, Kind::WiFi | Kind::Mobile)
        .then(|| profile.GetSignalBars().ok().and_then(|r| r.Value().ok()))
        .flatten();
    let link_mbps = adapter
        .and_then(|a| a.InboundMaxBitsPerSecond().ok())
        .filter(|b| *b > 0)
        .map(|b| (b as f64 / 1_000_000.0) as f32);
    Some(Connection {
        kind,
        bars,
        link_mbps,
    })
}

fn worker() {
    let mut history: VecDeque<Option<f32>> = VecDeque::new();
    let mut resolved: Option<(String, Option<u32>)> = None;
    let mut last_connection = Instant::now() - CONNECTION_EVERY;
    loop {
        let started = Instant::now();
        let (ping, connection, host) = {
            let shared = SHARED.lock().unwrap();
            (
                shared.want.ping,
                shared.want.connection,
                shared.want.host.clone(),
            )
        };
        if !ping && !connection {
            let mut shared = SHARED.lock().unwrap();
            shared.running = false;
            shared.reading = Reading::default();
            return;
        }

        if ping {
            if resolved.as_ref().map(|(h, _)| h != &host).unwrap_or(true) {
                resolved = Some((host.clone(), resolve(&host)));
                history.clear();
            }
            let address = resolved.as_ref().and_then(|(_, a)| *a);
            let ms = address.and_then(ping_once);
            if address.is_some() {
                history.push_back(ms);
                while history.len() > HISTORY {
                    history.pop_front();
                }
            }
            let mut shared = SHARED.lock().unwrap();
            if shared.want.host == host {
                shared.reading.host = host.clone();
                shared.reading.host_error = address.is_none();
                shared.reading.ping_ms = ms;
                shared.reading.quality = quality(history.make_contiguous());
            }
        }
        if connection && last_connection.elapsed() >= CONNECTION_EVERY {
            last_connection = Instant::now();
            let c = read_connection();
            SHARED.lock().unwrap().reading.connection = c;
        }
        std::thread::sleep(Duration::from_secs(1).saturating_sub(started.elapsed()));
    }
}

/// Trafic réseau par processus, alimenté par les événements Winsock-AFD de la session ETW.
///
/// Charge utile commune des événements de transfert (manifeste du fournisseur) :
/// EnterExit u32 @0, Location u32 @4, Process ptr @8, Endpoint ptr @16, BufferCount u32 @24,
/// Buffer ptr @28, BufferLength u32 @36, Status u32 @40.
/// L'entrée (EnterExit = 0) est émise dans le contexte du processus appelant ; la sortie (1),
/// qui porte la longueur transférée, peut arriver depuis un autre contexte : le pointeur
/// `Process` sert à la rattacher.
pub mod traffic {
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};
    use std::time::Instant;

    use crate::games::process_name as image_name;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct Reading {
        /// Tous les processus sauf le jeu, l'overlay et le système, en Mb/s.
        pub total_mbps: f32,
        /// Processus le plus gourmand (nom de l'exécutable) et son débit.
        pub top: Option<(String, f32)>,
    }

    struct Book {
        bytes: HashMap<u64, u64>,
        owner: HashMap<u64, u32>,
        since: Instant,
        names: HashMap<u32, String>,
    }

    static BOOK: LazyLock<Mutex<Book>> = LazyLock::new(|| {
        Mutex::new(Book {
            bytes: HashMap::new(),
            owner: HashMap::new(),
            since: Instant::now(),
            names: HashMap::new(),
        })
    });

    fn system(pid: u32) -> bool {
        pid == 0 || pid == 4
    }

    pub(crate) fn on_afd_event(pid: u32, data: &[u8]) {
        if data.len() < 40 {
            return;
        }
        let u32_at = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        let enter_exit = u32_at(0);
        let process = u64::from_le_bytes(data[8..16].try_into().unwrap());
        let Ok(mut book) = BOOK.lock() else { return };
        if book.owner.len() > 4096 {
            book.owner.clear();
        }
        match enter_exit {
            0 if !system(pid) => {
                book.owner.insert(process, pid);
            }
            1 => {
                let length = u64::from(u32_at(36));
                if length > 0 && book.bytes.len() < 4096 {
                    *book.bytes.entry(process).or_default() += length;
                    if !system(pid) {
                        book.owner.entry(process).or_insert(pid);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn clear() {
        if let Ok(mut book) = BOOK.lock() {
            book.bytes.clear();
            book.owner.clear();
            book.since = Instant::now();
        }
    }

    /// Débits depuis l'appel précédent (remet les compteurs à zéro).
    pub fn take(game_pid: u32) -> Reading {
        let own = std::process::id();
        let Ok(mut book) = BOOK.lock() else {
            return Reading {
                total_mbps: 0.0,
                top: None,
            };
        };
        let seconds = book.since.elapsed().as_secs_f32().max(0.2);
        book.since = Instant::now();
        let bytes = std::mem::take(&mut book.bytes);
        let mut per_pid: HashMap<u32, u64> = HashMap::new();
        for (process, n) in bytes {
            let pid = book.owner.get(&process).copied().unwrap_or(0);
            *per_pid.entry(pid).or_default() += n;
        }
        let mbps = |b: u64| (b as f64 * 8.0 / 1_000_000.0) as f32 / seconds;
        let mut total = 0u64;
        let mut top: Option<(u32, u64)> = None;
        for (pid, n) in per_pid {
            if pid == game_pid || pid == own || system(pid) {
                continue;
            }
            total += n;
            if top.map_or(true, |(_, b)| n > b) {
                top = Some((pid, n));
            }
        }
        if book.names.len() > 256 {
            book.names.clear();
        }
        let top = top.map(|(pid, n)| {
            let name = book
                .names
                .entry(pid)
                .or_insert_with(|| image_name(pid))
                .clone();
            (name, mbps(n))
        });
        Reading {
            total_mbps: mbps(total),
            top,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_needs_samples() {
        assert!(quality(&[Some(10.0); 4]).is_none());
    }

    #[test]
    fn quality_loss_and_jitter() {
        let mut h = vec![Some(20.0), Some(30.0), Some(20.0), Some(30.0)];
        h.extend(std::iter::repeat(Some(25.0)).take(46));
        let q = quality(&h).unwrap();
        assert_eq!(q.loss_percent, 0.0);
        assert!(!q.trouble);

        h.push(None);
        let q = quality(&h).unwrap();
        assert!(q.loss_percent > 1.9 && q.loss_percent < 2.0);
        assert!(q.trouble, "une perte dans les 10 dernières secondes");
    }
}
