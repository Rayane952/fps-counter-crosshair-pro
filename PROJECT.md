# FPS Counter & Crosshair — projet complet

Document de référence : ce qui existe, où, et tout ce qui a été modifié.

```
┌──────────────────────────────────────────────────────────────────────┐
│  APPLICATION D'ORIGINE (base)          APPLICATION RUST (réécriture) │
│  paquet MSIX Store 1.0.7               FPSOverlay*.exe (installeur)  │
│  C:\Program Files\WindowsApps\…        C:\Users\Rayane\AppData\      │
│                                          Local\Programs\             │
│                                          C:\Program Files\FPS Overlay│
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
| Installation | `C:\Program Files\FPS Overlay\` (installeur `dist\FPS-Overlay-Setup-1.0.0.exe`) |
| — overlay | `FPSOverlay.exe` : HUD, détection des jeux, icône de notification |
| — réglages | `FPSOverlaySettings.exe` : interface (egui) ; réduire ou fermer = retour en arrière-plan |
| — capteur | `FPSOverlaySensor.exe` : température CPU (SDK AMD), tâche planifiée élevée |
| Sources | `C:\Users\Rayane\Documents\deepseek-harness\default-workspace\repo\rust\` |
| Configuration | `%LOCALAPPDATA%\FPSOverlayRust\config.json` (conservée à la désinstallation) |
| Démarrage | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, valeur « FPS Overlay » |
| Tâche | `FPSOverlaySensor` (à l'ouverture de session, privilèges élevés) |
| Raccourci | menu Démarrer « FPS Overlay » (+ Bureau en option) ; désinstallation dans Applications installées |
| Icône | `rust\assets\fps-overlay.ico` (16 à 256 px ; réticule seul jusqu'à 48 px) |

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

Binaires secondaires : `probe.exe` (diagnostic de toutes les sources), `FPSOverlaySensor.exe` (capteur élevé).

---

## 3. Ce que fait l'application Rust aujourd'hui

- **HUD** click-through, topmost (dès la création), sans vol de focus, sur le moniteur de la fenêtre
  active, dans le coin choisi. Taille × échelle Windows du moniteur appliquées à la fenêtre et aux polices.
- **Détection des jeux** (`ovl-core::detect`) : le HUD n'apparaît que lorsqu'un jeu est au premier plan
  (ou partout avec « Afficher hors jeu »). Ordre de décision :
  1. règle de l'utilisateur pour l'exécutable (« toujours » / « jamais ») ;
  2. jeu reconnu : liste des jeux de Windows (`HKCU\System\GameConfigStore\Children`,
     `MatchedExeFullPath`) ou dossier de bibliothèque (Steam via `libraryfolders.vdf`, manifestes Epic,
     GOG via le registre, `XboxGames`, `Riot Games`, EA, Ubisoft, Battle.net, `X:\Games`) **et** rendu actif
     (FPS ETW ≥ 15 ou moteur 3D ≥ 3 %) : un lanceur dessiné par un autre processus ne déclenche rien ;
  3. inconnu : plein écran (fenêtre couvrant le moniteur ou `SHQueryUserNotificationState`) **et** rendu
     actif, hors navigateurs, lecteurs vidéo, messageries, outils de capture.

  2 s de détection continue avant l'affichage ; un jeu confirmé le reste tant que son processus vit
  (écrans de chargement). Les réglages listent les 6 derniers programmes avec leur verdict et des
  boutons « Toujours / Ne jamais afficher ». Aucune injection : compatible anti-triche.
- **Rendu** : alpha par pixel (`UpdateLayeredWindow`) : fond à l'opacité choisie, texte opaque,
  thème Transparent avec ombre.
- **Dispositions** : Panneau, Ligne, Graphique (panneau + courbe des temps d'image).
  **10 thèmes** : Graphite, Clair, Contraste élevé, Transparent, Vert, Ambre, Cyan, Rose, Violet,
  Corail. Couleur du texte personnalisable. Coin, distances horizontale et verticale, taille, opacité.
- **19 mesures au choix** (une mesure sans valeur n'apparaît pas) :

  | Mesure | Source |
  |---|---|
  | FPS, temps d'image, lows 1 % / 0,1 % (10 s), courbe | ETW temps réel : DXGI `Present` (42, 55), DxgKrnl `PresentHistory` (171, 215) |
  | Charge CPU, chaque cœur, fréquence | PDH `Processor Information` (`% Processor Utility`, `% Processor Performance` × `Processor Frequency`) |
  | Température CPU | SDK AMD via `FPSOverlaySensor.exe` (ignorée si publiée depuis plus de 10 s) |
  | Charge GPU | PDH `GPU Engine(*)`, moteur le plus occupé |
  | Température GPU (+ ventilateur) | `D3DKMTQueryAdapterInfo(ADAPTERPERFDATA)` : valeur du pilote AMD Adrenalin |
  | Mémoire GPU, mémoire GPU du jeu | PDH `GPU Adapter Memory` / `GPU Process Memory` (PID) + DXGI |
  | RAM | `GlobalMemoryStatusEx` |
  | Activité et débit disque | PDH `PhysicalDisk` |
  | Ping, perte, gigue | ICMP (`IcmpSendEcho`) chaque seconde, fenêtre de 60 s, hôte réglable |
  | Connexion, barres Wi-Fi, débit du lien | WinRT `NetworkInformation` |
  | Trafic en arrière-plan | ETW `Microsoft-Windows-Winsock-AFD` (octets par processus, hors jeu) |
  | Horloge | `GetLocalTime` |

  ETW exige d'être administrateur ou membre de « Utilisateurs du journal de performances »
  (c'est le cas de ce compte) ; sinon les réglages l'expliquent.
- **Alertes** (page dédiée) : FPS sous un seuil, températures CPU/GPU, ping, perte/gigue, trafic en
  arrière-plan, Wi-Fi faible. Ligne rouge clignotante dans le HUD, pop-up en haut de l'écran, ou
  les deux. Les alertes soutenues attendent 3 s.
- **Raccourci global** Alt+Maj+O : affiche / masque le HUD.
- **Icône de notification** : clic → réglages ; clic droit → menu (Réglages, Afficher le HUD,
  Afficher hors jeu, Quitter). Réenregistrée si l'Explorateur redémarre.
- **Réglages** (egui) : pages Overlay (état, aperçu réel, affichage, thèmes avec vos valeurs,
  mesures, position et taille, état des capteurs), Alertes, et Paramètres (démarrage, état de l'overlay, dossier de config). Clavier et lecteur d'écran
  (AccessKit). Instance unique.
- **Fichiers partagés** dans `%LOCALAPPDATA%\FPSOverlayRust\` : `config.json` (écrit par
  remplacement atomique, champ par champ), `status.json` (publié par l'overlay chaque seconde),
  `overlay.log`.
- Audit UI/UX : `repo/anti-slop/audit-001-2026-11-14.md`.

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

Les exécutables portent l'icône et les informations de version (`build-support\resources.rs`, compilé
par `windres` de MinGW). Installeur (Inno Setup, dans `tools\inno`) :

```powershell
cd repo\rust\installer
..\..\..\tools\inno\ISCC.exe FPSOverlay.iss     # -> repo\dist\FPS-Overlay-Setup-1.0.0.exe
```

L'installeur demande l'élévation (tâche du capteur), installe dans Program Files, crée le raccourci,
la valeur de démarrage et l'entrée de désinstallation, arrête et supprime l'ancienne copie de
développement (`%LOCALAPPDATA%\Programs\FPSOverlayRust`). Lancer un .exe de `target\release` ou de
`dist` directement le fait tourner en intégrité faible (voir Dépannage) : passer par l'installeur.
---

## 5. Dépannage

| Symptôme | Cause | Solution |
|---|---|---|
| Fenêtre noire « cmd » au démarrage | binaire compilé en console | `#![windows_subsystem = "windows"]` (déjà appliqué) |
| HUD blanc / invisible | fenêtre jamais peinte, fond de classe par défaut | brosse noire + peinture directe (déjà corrigé) |
| Panneau effacé chaque seconde | `SetWindowPos` à chaque tick | repositionnement seulement si la cible change |
| Température CPU absente | capteur arrêté | `schtasks /run /tn FPSOverlaySensor` |
| Réglages ne s'ouvrent pas | `FPSOverlaySettings.exe` absent | réinstaller avec l'installeur |
| FPS absents, « accès refusé » (ETW) ou réseau « inconnu » en lançant depuis `target\release` | le dossier de build hérite d'une étiquette d'intégrité **faible** : le .exe tourne en intégrité basse | lancer la copie installée dans `%LOCALAPPDATA%\Programs\FPSOverlayRust` |
| FPS absents pour une appli Electron/navigateur | les images sont présentées par un processus GPU séparé, pas par celui de la fenêtre | normal ; les jeux présentent depuis leur propre processus |
| Diagnostic de toutes les sources | — | `probe.exe [secondes] [hôte]` (compilé avec `-p ovl-core`), copié hors de `target` |

---

## 6. Reste à faire

1. Pages **Réticule**, **Profils**, **Rapports** de l'interface (affichées « Bientôt »).
2. Ping en IPv6 (aujourd'hui IPv4 seulement).
3. Désinstallation de l'app d'origine (Store) quand la parité sera atteinte (réticule, profils, rapports).
