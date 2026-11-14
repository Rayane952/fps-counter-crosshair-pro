//! Images présentées par processus (FPS, temps d'image, lows), via une session ETW temps réel.
//!
//! Fournisseurs :
//! - Microsoft-Windows-DXGI : `Present` (42) et `PresentMultiplaneOverlay` (55), début d'appel ;
//!   couvre Direct3D 10/11/12.
//! - Microsoft-Windows-DxgKrnl : `PresentHistory` (171) et `PresentHistoryDetailed` (215) ;
//!   couvre aussi Vulkan et OpenGL. Utilisé quand le processus n'émet rien côté DXGI.
//! - Microsoft-Windows-Winsock-AFD (optionnel) : envois et réceptions, pour le trafic par processus
//!   (voir `crate::net::traffic`).
//!
//! Droits : membre du groupe « Utilisateurs du journal de performances » (S-1-5-32-559) ou
//! administrateur. Sans eux, `start` renvoie `Status::NeedsPermission`.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use windows::core::{GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_INVALID_PARAMETER, WIN32_ERROR,
};
use windows::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW,
    CONTROLTRACE_HANDLE, ENABLE_TRACE_PARAMETERS, ENABLE_TRACE_PARAMETERS_VERSION_2,
    EVENT_CONTROL_CODE_DISABLE_PROVIDER, EVENT_CONTROL_CODE_ENABLE_PROVIDER,
    EVENT_FILTER_DESCRIPTOR, EVENT_FILTER_TYPE_EVENT_ID, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP,
    EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE, PROCESSTRACE_HANDLE,
    PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_RAW_TIMESTAMP,
    PROCESS_TRACE_MODE_REAL_TIME, WNODE_FLAG_TRACED_GUID,
};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

const SESSION_NAME: &str = "FPSOverlayRust";

const DXGI: GUID = GUID::from_u128(0xCA11C036_0102_4A2D_A6AD_F03CFED5D3C9);
const DXGKRNL: GUID = GUID::from_u128(0x802EC45A_1E99_4B83_9920_87C98277BA9D);
pub(crate) const AFD: GUID = GUID::from_u128(0xE53C6823_7BB8_44BB_90DC_3F86090D48A6);

const DXGI_EVENTS: [u16; 2] = [42, 55];
const DXGKRNL_EVENTS: [u16; 2] = [171, 215];
/// Mots-clés DxgKrnl « Base » et « Present ».
const DXGKRNL_KEYWORDS: u64 = 0x0000_0000_0800_0001;
/// Transferts de données AFD (send, recv, sendto, recvfrom, sendmsg, recvmsg).
pub(crate) const AFD_EVENTS: [u16; 10] =
    [1003, 1004, 1005, 1006, 1007, 1009, 1011, 1012, 1013, 1015];
/// Mot-clé AFD « source Winsock ».
const AFD_KEYWORDS: u64 = 0x4;
const TRACE_LEVEL_INFORMATION: u8 = 4;

/// 16 384 images : 10 s à 1 600 FPS.
const RING: usize = 1 << 14;
/// Fenêtre des FPS et du temps d'image.
const FPS_WINDOW_S: f64 = 1.0;
/// Fenêtre des lows : sur 1 s, 1 % de 144 images n'a pas de sens statistique.
const LOWS_WINDOW_S: f64 = 10.0;
/// Un processus sans image depuis ce délai n'est plus mesuré.
const STALE_S: f64 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Stopped,
    Running,
    /// Ni administrateur ni membre de « Utilisateurs du journal de performances ».
    NeedsPermission,
    Failed(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Reading {
    pub fps: f32,
    pub frame_ms: f32,
    /// FPS équivalent au 99e centile des temps d'image (10 s).
    pub low_1pct: Option<f32>,
    /// FPS équivalent au 99,9e centile (10 s).
    pub low_01pct: Option<f32>,
}

struct Track {
    times: Vec<i64>,
    next: usize,
    count: usize,
}

impl Track {
    fn new() -> Self {
        Self {
            times: vec![0; RING],
            next: 0,
            count: 0,
        }
    }

    fn push(&mut self, t: i64) {
        self.times[self.next] = t;
        self.next = (self.next + 1) % RING;
        self.count = (self.count + 1).min(RING);
    }

    /// i = 0 : plus récent.
    fn at(&self, i: usize) -> i64 {
        self.times[(self.next + RING - 1 - i) % RING]
    }

    fn last(&self) -> Option<i64> {
        (self.count > 0).then(|| self.at(0))
    }

    /// Intervalles (ms) entre images, du plus récent au plus ancien, sur `window_s` secondes.
    fn intervals(&self, qpf: f64, window_s: f64, max: usize) -> Vec<f64> {
        let mut out = Vec::new();
        let Some(newest) = self.last() else {
            return out;
        };
        let cutoff = newest - (window_s * qpf) as i64;
        for i in 1..self.count.min(max + 1) {
            let (later, earlier) = (self.at(i - 1), self.at(i));
            out.push((later - earlier) as f64 * 1000.0 / qpf);
            if earlier < cutoff {
                break;
            }
        }
        out
    }
}

struct State {
    status: Status,
    session: CONTROLTRACE_HANDLE,
    trace: PROCESSTRACE_HANDLE,
    network: bool,
    /// (pid, source noyau ?) → horodatages QPC.
    tracks: HashMap<(u32, bool), Track>,
    newest: i64,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        status: Status::Stopped,
        session: CONTROLTRACE_HANDLE { Value: 0 },
        trace: PROCESSTRACE_HANDLE { Value: 0 },
        network: false,
        tracks: HashMap::new(),
        newest: 0,
    })
});

fn qpf() -> f64 {
    let mut f = 0i64;
    unsafe {
        let _ = QueryPerformanceFrequency(&mut f);
    }
    f.max(1) as f64
}

fn qpc() -> i64 {
    let mut c = 0i64;
    unsafe {
        let _ = QueryPerformanceCounter(&mut c);
    }
    c
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Tampon EVENT_TRACE_PROPERTIES suivi de la place pour les noms (exigé par l'API).
fn properties_buffer(extra: usize) -> Vec<u64> {
    let bytes = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + extra;
    vec![0u64; bytes.div_ceil(8)]
}

fn stop_by_name(name: &str) {
    let mut buffer = properties_buffer(2048);
    let props = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
    let w = wide(name);
    unsafe {
        (*props).Wnode.BufferSize = (buffer.len() * 8) as u32;
        (*props).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
        (*props).LogFileNameOffset = (std::mem::size_of::<EVENT_TRACE_PROPERTIES>() + 1024) as u32;
        let _ = ControlTraceW(
            CONTROLTRACE_HANDLE { Value: 0 },
            PCWSTR(w.as_ptr()),
            props,
            EVENT_TRACE_CONTROL_STOP,
        );
    }
}

fn start_session(name: &str) -> Result<CONTROLTRACE_HANDLE, WIN32_ERROR> {
    let w = wide(name);
    let mut buffer = properties_buffer(w.len() * 2);
    let props = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
    let mut handle = CONTROLTRACE_HANDLE { Value: 0 };
    unsafe {
        (*props).Wnode.BufferSize = (buffer.len() * 8) as u32;
        (*props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        (*props).Wnode.ClientContext = 1; // horodatage QPC
        (*props).BufferSize = 64;
        (*props).MinimumBuffers = 4;
        (*props).MaximumBuffers = 32;
        (*props).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        (*props).FlushTimer = 1;
        (*props).LoggerNameOffset = std::mem::size_of::<EVENT_TRACE_PROPERTIES>() as u32;
        let rc = StartTraceW(&mut handle, PCWSTR(w.as_ptr()), props);
        if rc.0 != 0 {
            return Err(rc);
        }
    }
    Ok(handle)
}

/// Active un fournisseur en ne laissant passer que les identifiants donnés.
fn enable(
    session: CONTROLTRACE_HANDLE,
    provider: &GUID,
    keywords: u64,
    ids: &[u16],
) -> WIN32_ERROR {
    // EVENT_FILTER_EVENT_ID : FilterIn (u8), Reserved (u8), Count (u16), Events[Count] (u16)
    let mut filter: Vec<u16> = vec![1, ids.len() as u16];
    filter.extend_from_slice(ids);
    let mut descriptor = EVENT_FILTER_DESCRIPTOR {
        Ptr: filter.as_ptr() as u64,
        Size: (filter.len() * 2) as u32,
        Type: EVENT_FILTER_TYPE_EVENT_ID,
    };
    let params = ENABLE_TRACE_PARAMETERS {
        Version: ENABLE_TRACE_PARAMETERS_VERSION_2,
        EnableFilterDesc: &mut descriptor,
        FilterDescCount: 1,
        ..Default::default()
    };
    unsafe {
        let code = EVENT_CONTROL_CODE_ENABLE_PROVIDER.0;
        let rc = EnableTraceEx2(
            session,
            provider,
            code,
            TRACE_LEVEL_INFORMATION,
            keywords,
            0,
            0,
            Some(&params),
        );
        if rc == ERROR_INVALID_PARAMETER {
            // anciens Windows sans filtre par identifiant : le rappel filtre de toute façon
            return EnableTraceEx2(
                session,
                provider,
                code,
                TRACE_LEVEL_INFORMATION,
                keywords,
                0,
                0,
                None,
            );
        }
        rc
    }
}

fn status_from(rc: WIN32_ERROR) -> Status {
    if rc == ERROR_ACCESS_DENIED {
        Status::NeedsPermission
    } else {
        Status::Failed(rc.0)
    }
}

/// Détail de chaque étape, pour expliquer un échec (codes Win32).
pub fn diagnose() -> String {
    let name = format!("{SESSION_NAME}-diag-{}", std::process::id());
    let session = match start_session(&name) {
        Ok(s) => s,
        Err(rc) => return format!("StartTrace: {}", rc.0),
    };
    let dxgi = enable(session, &DXGI, u64::MAX, &DXGI_EVENTS).0;
    let krnl = enable(session, &DXGKRNL, DXGKRNL_KEYWORDS, &DXGKRNL_EVENTS).0;
    let afd = enable(session, &AFD, AFD_KEYWORDS, &AFD_EVENTS).0;
    stop_by_name(&name);
    format!("StartTrace: 0, DXGI: {dxgi}, DxgKrnl: {krnl}, AFD: {afd}")
}

/// Démarre la session (sans effet si elle tourne déjà).
pub fn start() -> Status {
    let mut state = STATE.lock().unwrap();
    if state.status == Status::Running {
        return state.status;
    }
    stop_by_name(SESSION_NAME); // session orpheline d'une instance précédente
    let session = match start_session(SESSION_NAME) {
        Ok(s) => s,
        Err(rc) if rc == ERROR_ALREADY_EXISTS => {
            let own = format!("{SESSION_NAME}-{}", std::process::id());
            stop_by_name(&own);
            match start_session(&own) {
                Ok(s) => s,
                Err(rc) => {
                    state.status = status_from(rc);
                    return state.status;
                }
            }
        }
        Err(rc) => {
            state.status = status_from(rc);
            return state.status;
        }
    };

    let mut rc = enable(session, &DXGI, u64::MAX, &DXGI_EVENTS);
    if rc.0 == 0 {
        rc = enable(session, &DXGKRNL, DXGKRNL_KEYWORDS, &DXGKRNL_EVENTS);
    }
    if rc.0 != 0 {
        stop_session(session);
        state.status = status_from(rc);
        return state.status;
    }

    let mut name = wide(SESSION_NAME);
    let mut logfile: EVENT_TRACE_LOGFILEW = unsafe { std::mem::zeroed() };
    logfile.LoggerName = PWSTR(name.as_mut_ptr());
    logfile.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME
        | PROCESS_TRACE_MODE_EVENT_RECORD
        | PROCESS_TRACE_MODE_RAW_TIMESTAMP;
    logfile.Anonymous2.EventRecordCallback = Some(on_event);
    let trace = unsafe { OpenTraceW(&mut logfile) };
    if trace.Value == u64::MAX {
        stop_session(session);
        state.status = Status::Failed(unsafe { windows::Win32::Foundation::GetLastError() }.0);
        return state.status;
    }

    state.session = session;
    state.trace = trace;
    state.status = Status::Running;
    state.network = false;
    std::thread::Builder::new()
        .name("etw-frames".into())
        .spawn(move || unsafe {
            // bloque jusqu'à l'arrêt de la session
            let _ = ProcessTrace(&[trace], None, None);
        })
        .ok();
    state.status
}

fn stop_session(_session: CONTROLTRACE_HANDLE) {
    stop_by_name(SESSION_NAME);
    stop_by_name(&format!("{SESSION_NAME}-{}", std::process::id()));
}

pub fn stop() {
    let mut state = STATE.lock().unwrap();
    if state.status != Status::Running {
        return;
    }
    state.status = Status::Stopped;
    stop_session(state.session);
    unsafe {
        let _ = CloseTrace(state.trace);
    }
    state.tracks.clear();
}

pub fn status() -> Status {
    STATE.lock().unwrap().status
}

/// Active ou coupe les événements réseau (trafic en arrière-plan).
pub fn set_network(on: bool) {
    let mut state = STATE.lock().unwrap();
    if state.status != Status::Running || state.network == on {
        return;
    }
    let rc = if on {
        enable(state.session, &AFD, AFD_KEYWORDS, &AFD_EVENTS)
    } else {
        unsafe {
            EnableTraceEx2(
                state.session,
                &AFD,
                EVENT_CONTROL_CODE_DISABLE_PROVIDER.0,
                0,
                0,
                0,
                0,
                None,
            )
        }
    };
    if rc.0 == 0 {
        state.network = on;
        if !on {
            crate::net::traffic::clear();
        }
    }
}

pub fn network_on() -> bool {
    let state = STATE.lock().unwrap();
    state.status == Status::Running && state.network
}

unsafe extern "system" fn on_event(record: *mut EVENT_RECORD) {
    let Some(record) = record.as_ref() else {
        return;
    };
    let header = &record.EventHeader;
    let id = header.EventDescriptor.Id;
    let kernel = if header.ProviderId == DXGI && DXGI_EVENTS.contains(&id) {
        false
    } else if header.ProviderId == DXGKRNL && DXGKRNL_EVENTS.contains(&id) {
        true
    } else {
        if header.ProviderId == AFD && AFD_EVENTS.contains(&id) && !record.UserData.is_null() {
            let data = std::slice::from_raw_parts(
                record.UserData as *const u8,
                record.UserDataLength as usize,
            );
            crate::net::traffic::on_afd_event(header.ProcessId, data);
        }
        return;
    };

    let Ok(mut state) = STATE.lock() else { return };
    let t = header.TimeStamp;
    state
        .tracks
        .entry((header.ProcessId, kernel))
        .or_insert_with(Track::new)
        .push(t);
    state.newest = state.newest.max(t);
    if state.tracks.len() > 64 {
        let cutoff = state.newest - (10.0 * qpf()) as i64;
        state
            .tracks
            .retain(|_, track| track.last().is_some_and(|l| l >= cutoff));
    }
}

/// Piste à utiliser pour un processus : DXGI si elle est vivante, sinon noyau.
fn with_track<R>(pid: u32, f: impl FnOnce(&Track, f64) -> R) -> Option<R> {
    let state = STATE.lock().ok()?;
    let qpf = qpf();
    let now = qpc();
    for kernel in [false, true] {
        if let Some(track) = state.tracks.get(&(pid, kernel)) {
            if track.count >= 2
                && track
                    .last()
                    .is_some_and(|l| (now - l) as f64 / qpf <= STALE_S)
            {
                return Some(f(track, qpf));
            }
        }
    }
    None
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let i = ((sorted.len() as f64 * p).floor() as usize).min(sorted.len() - 1);
    sorted[i]
}

pub fn read(pid: u32) -> Option<Reading> {
    if pid == 0 {
        return None;
    }
    with_track(pid, |track, qpf| {
        let recent = track.intervals(qpf, FPS_WINDOW_S, RING);
        if recent.is_empty() {
            return None;
        }
        let mean = recent.iter().sum::<f64>() / recent.len() as f64;
        let mut long = track.intervals(qpf, LOWS_WINDOW_S, RING);
        long.sort_by(|a, b| a.total_cmp(b));
        // en dessous de 100 images, le 1 % le plus lent n'est qu'une seule image
        let low_1pct = (long.len() >= 100).then(|| (1000.0 / percentile(&long, 0.99)) as f32);
        let low_01pct = (long.len() >= 1000).then(|| (1000.0 / percentile(&long, 0.999)) as f32);
        Some(Reading {
            fps: (1000.0 / mean) as f32,
            frame_ms: mean as f32,
            low_1pct,
            low_01pct,
        })
    })
    .flatten()
}

/// Processus qui présentent des images en ce moment, du plus actif au moins actif (diagnostic).
pub fn active_pids() -> Vec<(u32, usize)> {
    let Ok(state) = STATE.lock() else {
        return Vec::new();
    };
    let qpf = qpf();
    let now = qpc();
    let mut out: Vec<(u32, usize)> = state
        .tracks
        .iter()
        .filter(|(_, t)| t.last().is_some_and(|l| (now - l) as f64 / qpf <= STALE_S))
        .map(|((pid, _), t)| (*pid, t.intervals(qpf, 1.0, RING).len()))
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1));
    out.dedup_by_key(|(pid, _)| *pid);
    out
}

/// Derniers temps d'image (ms), du plus ancien au plus récent.
pub fn frame_times(pid: u32, max: usize) -> Vec<f32> {
    if pid == 0 {
        return Vec::new();
    }
    with_track(pid, |track, qpf| {
        let mut v: Vec<f32> = track
            .intervals(qpf, f64::MAX, max)
            .into_iter()
            .map(|x| x as f32)
            .collect();
        v.truncate(max);
        v.reverse();
        v
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_60_fps() {
        let mut track = Track::new();
        let qpf = 10_000_000.0;
        for i in 0..1200 {
            track.push((i as f64 * qpf / 60.0) as i64);
        }
        let v = track.intervals(qpf, 1.0, RING);
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        assert!((1000.0 / mean - 60.0).abs() < 0.1);
        assert!(v.len() >= 60 && v.len() <= 62);
    }
}
