//! Détection des jeux : le HUD ne s'affiche que lorsqu'un jeu est au premier plan.
//!
//! Signaux, du plus fort au plus faible :
//! 1. Règle de l'utilisateur pour cet exécutable (toujours / jamais).
//! 2. Jeu connu : liste des jeux reconnus par Windows (`GameConfigStore`, base de la Game Bar)
//!    ou exécutable situé dans une bibliothèque de lanceur (Steam, Epic, GOG, Riot, Xbox…).
//!    Il doit aussi rendre des images : un lanceur affiché par un processus séparé (client
//!    League of Legends, launchers web) ne déclenche rien.
//! 3. Jeu inconnu en plein écran (exclusif ou fenêtré sans bordure) qui rend en 3D, hors
//!    navigateurs, lecteurs vidéo et outils connus.
//!
//! Une détection doit tenir 2 échantillons (2 s) avant d'afficher le HUD ; un jeu confirmé le
//! reste tant que son processus vit, même pendant un écran de chargement sans images.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER,
    HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_SZ,
};
use windows::Win32::UI::Shell::{
    SHQueryUserNotificationState, QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
};

/// FPS minimum pour considérer qu'un programme rend activement.
pub const RENDER_MIN_FPS: f32 = 15.0;
/// Charge du moteur 3D (%) qui suffit à dire qu'il rend, si les images ne sont pas mesurables.
pub const RENDER_MIN_3D: f32 = 3.0;
/// Échantillons consécutifs exigés avant d'afficher.
const CONFIRM_TICKS: u8 = 2;
const LIBRARY_REFRESH: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Windows,
    Steam,
    Epic,
    Gog,
    Riot,
    Xbox,
    Ea,
    Ubisoft,
    Battlenet,
    GamesFolder,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Windows => "liste des jeux de Windows",
            Source::Steam => "bibliothèque Steam",
            Source::Epic => "Epic Games",
            Source::Gog => "GOG",
            Source::Riot => "Riot Games",
            Source::Xbox => "Xbox",
            Source::Ea => "EA",
            Source::Ubisoft => "Ubisoft",
            Source::Battlenet => "Battle.net",
            Source::GamesFolder => "dossier Games",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "source")]
pub enum Reason {
    UserAlways,
    UserNever,
    /// Overlay, réglages, bureau, menu Démarrer…
    Ignored,
    /// Navigateur, lecteur vidéo, messagerie, capture…
    Excluded,
    /// Jeu connu qui rend des images.
    Known(Source),
    /// Jeu connu, mais aucune image rendue par ce processus (lanceur, client web).
    KnownIdle(Source),
    /// Inconnu, plein écran, rendu 3D.
    Fullscreen3d,
    /// Inconnu, rend en 3D mais en fenêtre.
    Windowed3d,
    /// Ne rend pas d'images.
    NotRendering,
}

impl Reason {
    /// Phrase pour les réglages.
    pub fn describe(self) -> String {
        match self {
            Reason::UserAlways => "règle « toujours afficher »".into(),
            Reason::UserNever => "règle « ne jamais afficher »".into(),
            Reason::Ignored => "programme système ou de l'overlay".into(),
            Reason::Excluded => "navigateur, lecteur vidéo ou outil (pas un jeu)".into(),
            Reason::Known(s) => format!("jeu reconnu ({}), rend des images", s.name()),
            Reason::KnownIdle(s) => format!(
                "jeu reconnu ({}), mais n'affiche pas d'images (lanceur ?)",
                s.name()
            ),
            Reason::Fullscreen3d => "plein écran avec rendu 3D".into(),
            Reason::Windowed3d => "rendu 3D en fenêtre, sans être un jeu reconnu".into(),
            Reason::NotRendering => "ne rend pas d'images".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub game: bool,
    pub reason: Reason,
}

/// Règle de l'utilisateur pour un exécutable (chemin complet, comparé sans casse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub path: String,
    pub show: bool,
}

/// Faits sur le programme au premier plan, collectés à chaque échantillon.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub path: String,
    pub known: Option<Source>,
    pub rendering: bool,
    pub fullscreen: bool,
    pub excluded: bool,
    pub ignored: bool,
}

/// Décision pure, sans état (testée unitairement).
pub fn decide(facts: &Facts, rules: &[Rule]) -> Verdict {
    if let Some(rule) = rules
        .iter()
        .find(|r| r.path.eq_ignore_ascii_case(&facts.path))
    {
        return Verdict {
            game: rule.show,
            reason: if rule.show {
                Reason::UserAlways
            } else {
                Reason::UserNever
            },
        };
    }
    if facts.ignored {
        return Verdict {
            game: false,
            reason: Reason::Ignored,
        };
    }
    if let Some(source) = facts.known {
        return if facts.rendering {
            Verdict {
                game: true,
                reason: Reason::Known(source),
            }
        } else {
            Verdict {
                game: false,
                reason: Reason::KnownIdle(source),
            }
        };
    }
    if facts.excluded {
        return Verdict {
            game: false,
            reason: Reason::Excluded,
        };
    }
    match (facts.rendering, facts.fullscreen) {
        (true, true) => Verdict {
            game: true,
            reason: Reason::Fullscreen3d,
        },
        (true, false) => Verdict {
            game: false,
            reason: Reason::Windowed3d,
        },
        _ => Verdict {
            game: false,
            reason: Reason::NotRendering,
        },
    }
}

/// Exécutables jamais considérés comme des jeux par l'heuristique plein écran (noms sans .exe).
const EXCLUDED: &[&str] = &[
    "chrome",
    "msedge",
    "firefox",
    "opera",
    "opera_gx",
    "brave",
    "vivaldi",
    "arc",
    "iexplore",
    "librewolf",
    "vlc",
    "mpc-hc64",
    "mpc-hc",
    "mpc-be64",
    "mpv",
    "potplayermini64",
    "potplayer",
    "wmplayer",
    "video.ui",
    "microsoft.media.player",
    "netflix",
    "spotify",
    "discord",
    "teams",
    "ms-teams",
    "slack",
    "zoom",
    "obs64",
    "obs32",
    "streamlabs obs",
    "powerpnt",
    "mstsc",
    "vmware-vmx",
    "vmware",
    "virtualboxvm",
    "code",
    "devenv",
    "blender",
    "photoshop",
    "premiere pro",
    "afterfx",
    "resolve",
    "steam",
    "steamwebhelper",
    "epicgameslauncher",
    "battle.net",
    "riotclientux",
    "eadesktop",
    "upc",
    "galaxyclient",
    "claude",
];

/// Processus du système ou de l'overlay, jamais des jeux.
const IGNORED: &[&str] = &[
    "explorer",
    "shellexperiencehost",
    "startmenuexperiencehost",
    "searchhost",
    "searchapp",
    "lockapp",
    "textinputhost",
    "applicationframehost",
    "systemsettings",
    "dwm",
    "taskmgr",
];

/// Un exécutable du dossier d'installation de l'overlay (réglages, capteur…).
fn is_own_program(path: &str) -> bool {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.to_string_lossy().to_string()))
    else {
        return false;
    };
    !path.is_empty() && folder(path.rsplit_once('\\').map_or(path, |(d, _)| d)) == folder(&dir)
}

/// Programme au premier plan.
#[derive(Debug, Clone, Default)]
pub struct Foreground {
    pub hwnd: isize,
    pub pid: u32,
    /// Chemin complet de l'exécutable (vide si inaccessible).
    pub path: String,
    /// Nom sans « .exe ».
    pub name: String,
    pub desktop: bool,
}

pub fn foreground() -> Foreground {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Foreground {
                desktop: true,
                ..Default::default()
            };
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let mut class = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        let path = crate::games::process_path(pid).unwrap_or_default();
        let name = file_stem(&path);
        Foreground {
            hwnd: hwnd.0 as isize,
            pid,
            path,
            name,
            desktop: matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd"),
        }
    }
}

fn file_stem(path: &str) -> String {
    let file = path.rsplit('\\').next().unwrap_or(path);
    match file.len().checked_sub(4) {
        Some(i) if file[i..].eq_ignore_ascii_case(".exe") => file[..i].to_owned(),
        _ => file.to_owned(),
    }
}

/// La fenêtre couvre tout son moniteur (plein écran exclusif ou fenêtré sans bordure),
/// ou Windows signale une application Direct3D plein écran.
pub fn is_fullscreen(hwnd: isize) -> bool {
    unsafe {
        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_BUSY {
                return true;
            }
        }
        let hwnd = HWND(hwnd as *mut _);
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }
        let m = info.rcMonitor;
        rect.left <= m.left && rect.top <= m.top && rect.right >= m.right && rect.bottom >= m.bottom
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn reg_string(root: HKEY, key: &str, value: &str) -> Option<String> {
    let (k, v) = (wide(key), wide(value));
    let mut size = 0u32;
    unsafe {
        if RegGetValueW(
            root,
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
        .0 != 0
            || size < 2
        {
            return None;
        }
        let mut buffer = vec![0u16; size as usize / 2];
        if RegGetValueW(
            root,
            PCWSTR(k.as_ptr()),
            PCWSTR(v.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
        .0 != 0
        {
            return None;
        }
        let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..len]))
    }
}

fn reg_subkeys(root: HKEY, key: &str) -> Vec<String> {
    let k = wide(key);
    let mut out = Vec::new();
    unsafe {
        let mut handle = HKEY::default();
        if RegOpenKeyExW(root, PCWSTR(k.as_ptr()), 0, KEY_READ, &mut handle).0 != 0 {
            return out;
        }
        for index in 0.. {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            let rc = RegEnumKeyExW(
                handle,
                index,
                PWSTR(name.as_mut_ptr()),
                &mut len,
                None,
                PWSTR::null(),
                None,
                None,
            );
            if rc.0 != 0 {
                break;
            }
            out.push(String::from_utf16_lossy(&name[..len as usize]));
        }
        let _ = RegCloseKey(handle);
    }
    out
}

/// Normalise un dossier : minuscules, antislashs, terminé par « \ ».
fn folder(path: &str) -> String {
    let mut p = path.replace('/', "\\").to_lowercase();
    if !p.ends_with('\\') {
        p.push('\\');
    }
    p
}

/// Jeux et bibliothèques connus sur ce PC.
#[derive(Debug, Clone, Default)]
pub struct Library {
    /// Exécutables (minuscules) reconnus par Windows.
    exes: Vec<String>,
    /// Dossiers racines de bibliothèques.
    roots: Vec<(String, Source)>,
}

impl Library {
    pub fn load() -> Self {
        let mut lib = Library::default();

        let store = r"System\GameConfigStore\Children";
        for child in reg_subkeys(HKEY_CURRENT_USER, store) {
            if let Some(exe) = reg_string(
                HKEY_CURRENT_USER,
                &format!(r"{store}\{child}"),
                "MatchedExeFullPath",
            ) {
                if !exe.is_empty() {
                    lib.exes.push(exe.to_lowercase());
                }
            }
        }

        if let Some(steam) = reg_string(HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath") {
            let mut libraries = vec![steam.replace('/', "\\")];
            let vdf = format!(r"{}\steamapps\libraryfolders.vdf", libraries[0]);
            if let Ok(text) = std::fs::read_to_string(vdf) {
                for line in text.lines() {
                    let parts: Vec<&str> = line.split('"').collect();
                    // ligne « "path"		"D:\\SteamLibrary" »
                    if parts.len() >= 4 && parts[1] == "path" {
                        libraries.push(parts[3].replace(r"\\", r"\"));
                    }
                }
            }
            for l in libraries {
                lib.add_root(&format!(r"{l}\steamapps\common"), Source::Steam);
            }
        }

        let epic = r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests";
        if let Ok(entries) = std::fs::read_dir(epic) {
            for entry in entries.flatten() {
                let Ok(text) = std::fs::read_to_string(entry.path()) else {
                    continue;
                };
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(location) = json.get("InstallLocation").and_then(|v| v.as_str()) {
                        lib.add_root(location, Source::Epic);
                    }
                }
            }
        }

        let gog = r"SOFTWARE\WOW6432Node\GOG.com\Games";
        for id in reg_subkeys(HKEY_LOCAL_MACHINE, gog) {
            if let Some(path) = reg_string(HKEY_LOCAL_MACHINE, &format!(r"{gog}\{id}"), "path") {
                lib.add_root(&path, Source::Gog);
            }
        }

        // dossiers par défaut des lanceurs, sur chaque lecteur
        for drive in b'C'..=b'Z' {
            let d = format!("{}:\\", drive as char);
            if !std::path::Path::new(&d).exists() {
                continue;
            }
            for (rel, source) in [
                ("XboxGames", Source::Xbox),
                ("Riot Games", Source::Riot),
                ("Program Files\\Epic Games", Source::Epic),
                ("Program Files (x86)\\Epic Games", Source::Epic),
                ("Program Files (x86)\\GOG Galaxy\\Games", Source::Gog),
                ("GOG Games", Source::Gog),
                ("Program Files\\EA Games", Source::Ea),
                ("Program Files\\Electronic Arts", Source::Ea),
                ("Program Files (x86)\\Origin Games", Source::Ea),
                (
                    "Program Files (x86)\\Ubisoft\\Ubisoft Game Launcher\\games",
                    Source::Ubisoft,
                ),
                ("Program Files (x86)\\Battle.net", Source::Battlenet),
                ("Games", Source::GamesFolder),
            ] {
                let path = format!("{d}{rel}");
                if std::path::Path::new(&path).is_dir() {
                    lib.add_root(&path, source);
                }
            }
        }
        lib
    }

    fn add_root(&mut self, path: &str, source: Source) {
        let root = folder(path);
        if !self.roots.iter().any(|(r, _)| *r == root) {
            self.roots.push((root, source));
        }
    }

    /// Source qui reconnaît cet exécutable, s'il y en a une.
    pub fn classify(&self, exe: &str) -> Option<Source> {
        let lower = exe.to_lowercase();
        if lower.is_empty() {
            return None;
        }
        if self.exes.iter().any(|e| *e == lower) {
            return Some(Source::Windows);
        }
        self.roots
            .iter()
            .find(|(root, _)| lower.starts_with(root.as_str()))
            .map(|(_, s)| *s)
    }

    /// Résumé pour les réglages : (source, nombre de jeux ou de bibliothèques).
    pub fn summary(&self) -> Vec<(Source, usize)> {
        let mut out: Vec<(Source, usize)> = Vec::new();
        if !self.exes.is_empty() {
            out.push((Source::Windows, self.exes.len()));
        }
        for (_, s) in &self.roots {
            match out.iter_mut().find(|(o, _)| o == s) {
                Some((_, n)) => *n += 1,
                None => out.push((*s, 1)),
            }
        }
        out
    }

    #[cfg(test)]
    fn with(exes: &[&str], roots: &[(&str, Source)]) -> Self {
        let mut lib = Library {
            exes: exes.iter().map(|e| e.to_lowercase()).collect(),
            roots: Vec::new(),
        };
        for (r, s) in roots {
            lib.add_root(r, *s);
        }
        lib
    }
}

/// Détecteur avec état : bibliothèque, confirmation sur 2 échantillons, jeux confirmés.
pub struct Detector {
    library: Library,
    loaded: Instant,
    pending: Option<(u32, u8)>,
    /// pid → chemin des jeux confirmés (restent confirmés tant que le processus vit).
    confirmed: HashMap<u32, String>,
}

impl Default for Detector {
    fn default() -> Self {
        Self::new()
    }
}

impl Detector {
    pub fn new() -> Self {
        Self {
            library: Library::load(),
            loaded: Instant::now(),
            pending: None,
            confirmed: HashMap::new(),
        }
    }

    pub fn library(&self) -> &Library {
        &self.library
    }

    /// `rendering` : le programme au premier plan rend des images (FPS ou charge 3D).
    pub fn update(&mut self, fg: &Foreground, rendering: bool, rules: &[Rule]) -> Verdict {
        if self.loaded.elapsed() >= LIBRARY_REFRESH {
            self.library = Library::load();
            self.loaded = Instant::now();
        }
        let lower_name = fg.name.to_lowercase();
        let facts = Facts {
            path: fg.path.clone(),
            known: self.library.classify(&fg.path),
            rendering,
            fullscreen: !fg.desktop && fg.hwnd != 0 && is_fullscreen(fg.hwnd),
            excluded: EXCLUDED.contains(&lower_name.as_str()),
            ignored: fg.desktop
                || fg.pid == 0
                || fg.pid == std::process::id()
                || IGNORED.contains(&lower_name.as_str())
                || is_own_program(&fg.path),
        };
        let verdict = decide(&facts, rules);

        // une règle s'applique tout de suite, sans confirmation
        if matches!(verdict.reason, Reason::UserAlways | Reason::UserNever) {
            return verdict;
        }
        // jeu déjà confirmé : il le reste pendant ses chargements
        if self.confirmed.get(&fg.pid).is_some_and(|p| *p == fg.path) {
            let reason = match facts.known {
                Some(s) => Reason::Known(s),
                None => Reason::Fullscreen3d,
            };
            return Verdict { game: true, reason };
        }
        if !verdict.game {
            self.pending = None;
            return verdict;
        }
        let ticks = match self.pending {
            Some((pid, n)) if pid == fg.pid => n + 1,
            _ => 1,
        };
        self.pending = Some((fg.pid, ticks));
        if ticks >= CONFIRM_TICKS {
            self.confirmed
                .retain(|pid, _| crate::games::process_path(*pid).is_some());
            self.confirmed.insert(fg.pid, fg.path.clone());
            verdict
        } else {
            Verdict {
                game: false,
                reason: verdict.reason,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(path: &str, known: Option<Source>, rendering: bool, fullscreen: bool) -> Facts {
        Facts {
            path: path.into(),
            known,
            rendering,
            fullscreen,
            ..Default::default()
        }
    }

    #[test]
    fn known_game_needs_rendering() {
        let f = facts(
            r"D:\Riot Games\League of Legends\LeagueClientUx.exe",
            Some(Source::Windows),
            false,
            false,
        );
        assert_eq!(
            decide(&f, &[]),
            Verdict {
                game: false,
                reason: Reason::KnownIdle(Source::Windows)
            }
        );
        let f = facts(
            r"D:\Riot Games\League of Legends\Game\League of Legends.exe",
            Some(Source::Windows),
            true,
            false,
        );
        assert!(decide(&f, &[]).game, "un jeu reconnu en fenêtre compte");
    }

    #[test]
    fn unknown_needs_fullscreen() {
        assert_eq!(
            decide(&facts("x.exe", None, true, false), &[]).reason,
            Reason::Windowed3d
        );
        assert!(decide(&facts("x.exe", None, true, true), &[]).game);
        let browser = Facts {
            excluded: true,
            ..facts("chrome.exe", None, true, true)
        };
        assert_eq!(decide(&browser, &[]).reason, Reason::Excluded);
    }

    #[test]
    fn rules_win_and_ignore_case() {
        let rules = [Rule {
            path: r"C:\Jeux\Mon Jeu.exe".into(),
            show: true,
        }];
        assert_eq!(
            decide(&facts(r"c:\jeux\mon jeu.exe", None, false, false), &rules).reason,
            Reason::UserAlways
        );
        let never = [Rule {
            path: "x.exe".into(),
            show: false,
        }];
        assert!(!decide(&facts("x.exe", Some(Source::Steam), true, true), &never).game);
    }

    #[test]
    fn library_matches_roots_and_windows_list() {
        let lib = Library::with(
            &[r"D:\World of Warcraft\_forever_\Wow.exe"],
            &[(r"D:\SteamLibrary\steamapps\common", Source::Steam)],
        );
        assert_eq!(
            lib.classify(r"d:\world of warcraft\_forever_\WOW.EXE"),
            Some(Source::Windows)
        );
        assert_eq!(
            lib.classify(r"D:\SteamLibrary\steamapps\common\Rust\RustClient.exe"),
            Some(Source::Steam)
        );
        assert_eq!(
            lib.classify(r"D:\SteamLibrary\steamapps\commonx\a.exe"),
            None,
            "préfixe de dossier entier"
        );
        assert_eq!(lib.classify(r"C:\Windows\notepad.exe"), None);
    }

    #[test]
    fn stems() {
        assert_eq!(file_stem(r"C:\a\Game.EXE"), "Game");
        assert_eq!(file_stem(r"C:\a\noext"), "noext");
    }
}
