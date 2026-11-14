# FPS Counter & Crosshair — projet complet

Document de référence : ce qui existe, où, et tout ce qui a été modifié.

```
┌──────────────────────────────────────────────────────────────────────┐
│  APPLICATION D'ORIGINE (base)          APPLICATION RUST (réécriture) │
│  paquet MSIX Store 1.0.7               overlay.exe / sensor.exe      │
│  C:\Program Files\WindowsApps\…        C:\Users\Rayane\AppData\      │
│                                          Local\Programs\             │
│                                          FPSOverlayRust\             │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 1. Où se trouve quoi

### Application d'origine (la base, non modifiée)

| | |
|---|---|
| Paquet | `KoykStudio.GameOverlayFPSCounterCrosshairCPUGPUTem_1.0.7.0_x64__516232dwgyvgy` |
| Dossier | `C:\Program Files\WindowsApps\KoykStudio.GameOverlayFPSCounterCrosshairCPUGPUTem_1.0.7.0_x64__516232dwgyvgy` |
| Source | Microsoft Store (réinstallée par toi après nos essais) |
| Lancement | tuile « FPS Counter & Crosshair » du menu Démarrer |

> C'est le binaire de référence qu'on décompile et qu'on clone. On n'y touche plus.

### Application Rust (celle qu'on développe)

| | |
|---|---|
| Binaires installés | `C:\Users\Rayane\AppData\Local\Programs\FPSOverlayRust\` |
| — overlay | `overlay.exe` (4,3 Mo) — HUD + icône de notification |
| — capteur | `sensor.exe` (1,4 Mo) — température CPU (SDK AMD), tâche élevée |
| — réglages | `settings.exe` (12,9 Mo) — interface (egui) |
| Sources | `C:\Users\Rayane\Documents\deepseek-harness\default-workspace\repo\rust\` |
| Configuration | `%LOCALAPPDATA%\FPSOverlayRust\config.json` |
| Démarrage | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\FPSOverlayRust.vbs` |
| Tâche | `FPSOverlaySensor` (au logon, privilèges élevés) |
| Raccourci | `…\Start Menu\Programs\FPS Overlay.lnk` |

### Matériel de travail (hors installation)

| | |
|---|---|
| Décompilé C# de l'app d'origine | `…\default-workspace\src\GameOverlay.decompiled.cs` (15 000 lignes) |
| Payload patché + scripts de patch | `…\default-workspace\repo\installer\payload\` |
| Patchers IL (Mono.Cecil) | `…\default-workspace\probe\Patch*.cs` |
| Capteur C# historique | `…\default-workspace\probe\Bridge2.cs` |
| Toolchain Rust + MinGW | `…\default-workspace\tools\{cargo,rustup,mingw,inno}` |
| Dépôt GitHub | https://github.com/Rayane952/fps-counter-crosshair-pro (privé) |

---

## 2. Historique de ce qui a été fait

### Phase A — modification de l'application d'origine (C#/WinUI 3)

Patchs IL appliqués à `GameOverlay.dll` (via Mono.Cecil), dans l'ordre :

| # | Fichier source | Effet |
|---|---|---|
| 01 | `patch/01-pro.cs` | `AppState.get_ProLicensed` → true, `Pro.get_IsPro` → true, `get_InTrial`/`get_TrialEnded` → false : suppression de la période d'essai de 7 jours |
| 02 | `02-sensor.cs` | `Sensors.ReadThermalZoneC` lit `C:\ProgramData\OverlayTemp\cpu.txt` (le PC n'expose pas les compteurs `\Thermal Zone Information`) |
| 03 | `03-hud-fonts.cs` | températures rendues avec la police des valeurs (Segoe UI Variable Display, 600, 15·k) |
| 04 | `04-hud-column.cs` | suppression du placeholder de largeur `"888%"` |
| 05 | `05-hud-rows.cs` | sans pourcentage, la température passe dans la colonne valeur (alignée sous les FPS) |
| 06 | `06-single-window.cs` | garde de ré-entrance sur `App.OpenSettings` : plus de double fenêtre au clic |

Le paquet a été redéployé en paquet de développement `KoykStudio.GameOverlayPro` depuis
`%LOCALAPPDATA%\Programs\OverlayMod` (WindowsApps est verrouillé en écriture sur ce build, même
en SYSTEM/TrustedInstaller). **Cette version a depuis été désinstallée**, l'app Store d'origine est
revenue comme référence.

### Phase B — température CPU sans pilote tiers

Windows n'expose aucune sonde CPU sur AM5. La valeur vient du SDK installé par Adrenalin :

```
Adrenalin → RyzenMasterSDK → AMDRyzenMasterDriverV32.sys (ring0) → SMU (THM_TCON_CUR_TMP)

Platform.dll!GetRmCpuParameters(void*)   [export C, __cdecl, élévation requise]
    0x90 → double (°C)    0x14 → float (W)    0x20 → float (A)
```

Un démon élevé publie la valeur dans `C:\ProgramData\OverlayTemp\cpu.txt`, l'UI la lit sans privilèges.

### Phase C — réécriture Rust

| Crate | Rôle | État |
|---|---|---|
| `ovl-core` | PDH (CPU, GPU), mémoire, capteur AMD, détection de jeu | fonctionnel |
| `ovl-overlay` | fenêtre layered click-through + rendu GDI du panneau | fonctionnel |
| `ovl-platform` | config JSON, icône de notification, fenêtre de réglages Win32 | fonctionnel |
| `ovl-ui` | interface egui (clone de l'original) | page Overlay faite |
| `ovl-app` | assemblage, boucle 1 Hz, affichage en jeu | fonctionnel |

Binaires secondaires : `probe.exe` (dump JSON des stats), `sensor.exe` (capteur élevé).

---

## 3. Ce que fait l'application Rust aujourd'hui

- **HUD translucide**, positionné en haut à droite du moniteur de la fenêtre active, click-through,
  topmost, sans vol de focus. Affiché **uniquement si un jeu rend réellement en 3D** au premier plan
  (compteurs `\GPU Engine(*)\engtype_3d` du PID, seuil 3 %).
- **Mesures** : CPU %, température CPU (AMD), GPU %, RAM. FPS et température GPU encore à `—`.
- **Icône de notification** : clic simple → réglages ; clic droit → menu (Réglages, Afficher,
  Toujours afficher, Quitter).
- **Interface** : clone de l'original (barre latérale Overlay/Réticule/Profils/Rapports,
  Aide/Pro/Paramètres, cartes, toggle animé, aperçu, dispositions, thèmes, police Segoe UI Variable).
  Tout ce qui touche l'overlay est **fonctionnel** (écrit `config.json`, relu à chaud chaque seconde).

### Mesures

| | WinUI/.NET (base) | Rust |
|---|---|---|
| RAM | 276,8 Mo + ~383 Mo WebView2 | **23 Mo** (overlay) |
| Threads | ~40 | 3 |
| Binaire | 116 Mo installés | **4,3 Mo** |
| Ouverture des réglages | 300–1500 ms | < 100 ms |

---

## 4. Build

Prérequis installés hors dépôt :

1. `rustup` toolchain `stable-x86_64-pc-windows-gnu` (dans `tools\rustup`)
2. MinGW-w64 UCRT (dans `tools\mingw\mingw64\bin`) pour le link

```powershell
$env:CARGO_HOME = "…\tools\cargo"
$env:RUSTUP_HOME = "…\tools\rustup"
$env:PATH = "$env:CARGO_HOME\bin;…\tools\mingw\mingw64\bin;$env:PATH"
$env:TEMP = "…\tools\tmp"      # sinon gcc échoue : "Cannot create temporary file"
$env:TMP  = "…\tools\tmp"

cd repo\rust
cargo build --release -p ovl-app -p ovl-ui -p ovl-core
```

Copier ensuite `target\release\{overlay,sensor,settings}.exe` dans
`%LOCALAPPDATA%\Programs\FPSOverlayRust`.

---

## 5. Dépannage

| Symptôme | Cause | Solution |
|---|---|---|
| Fenêtre noire « cmd » au démarrage | binaire compilé en console | `#![windows_subsystem = "windows"]` (déjà appliqué) |
| HUD blanc / invisible | fenêtre jamais peinte, fond de classe par défaut | brosse noire + peinture directe (déjà corrigé) |
| Panneau effacé chaque seconde | `SetWindowPos` à chaque tick | repositionnement seulement si la cible change |
| Température CPU absente | `sensor.exe` arrêté | `schtasks /run /tn FPSOverlaySensor` |
| Réglages ne s'ouvrent pas | `settings.exe` absent du dossier | recopier le binaire |

---

## 6. Reste à faire

1. **FPS via ETW** (`Microsoft-Windows-DXGI` / `DxgKrnl` Present) dans `sensor.exe` — seule métrique encore vide.
2. **Température GPU** (`D3DKMT ADAPTER_PERFDATA`).
3. **Transparence par pixel** (aujourd'hui translucidité uniforme via `LWA_ALPHA`).
4. Pages **Réticule**, **Profils**, **Rapports** de l'interface.
5. Installeur Rust (.exe) + désinstallation propre de l'app d'origine quand la parité sera atteinte.
