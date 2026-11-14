# Rust — application d'overlay

État : **cœur + overlay fonctionnels** (voir ../MIGRATION.md pour le reste du plan).

```
FPS    —          CPU  30%      TEMP 51°C
GPU    6%         GTEMP —        RAM  13,7 Go
```

Mesuré sur la même machine que la version WinUI :

| | WinUI/.NET (actuel) | Rust (ce binaire) |
|---|---|---|
| RAM | 276,8 Mo + 383 Mo WebView2 | **21 Mo** |
| Threads | ~40 | **4** |
| Binaire | 116 Mo installés | **4,1 Mo** |

## Crates

| Crate | Rôle | État |
|---|---|---|
| `ovl-core` | PDH (CPU, GPU), mémoire, capteur AMD, détection de jeu | fonctionnel |
| `ovl-overlay` | fenêtre layered click-through + rendu GDI | fonctionnel |
| `ovl-app` | boucle 1 Hz, affichage seulement en jeu | fonctionnel |
| `ovl-ui` | réglages egui | à faire |
| `ovl-platform` | tray, raccourcis, config | à faire |

## Build

Prérequis (non versionnés, installés hors dépôt) :

1. `rustup` toolchain `stable-x86_64-pc-windows-gnu`
2. MinGW-w64 (gcc) pour le link — winlibs UCRT

```powershell
$env:PATH = "<cargo>\bin;<mingw>\mingw64\bin;$env:PATH"
$env:TEMP = "<workspace>\tmp"   # sinon gcc échoue : "Cannot create temporary file in %TEMP%"
cargo build --release -p ovl-app
.`\target\release\overlay.exe
```

Binaires secondaires :

- `cargo run --release -p ovl-core --bin sensor` — publie la température CPU (élévation requise)
- `cargo run --release -p ovl-core --bin probe` — imprime les stats en JSON (vérification)

## Détection de jeu

`ovl-core::games::GameProbe` lit `\GPU Engine(*)\Utilization Percentage`, additionne les
instances `engtype_3d` du PID au premier plan ; seuil 3 %. `OVL_FORCE=1` force l'affichage
(pour les tests hors jeu).

## Reste à faire

1. FPS via ETW (`Microsoft-Windows-DXGI` Present) — actuellement `—`.
2. Température GPU (D3DKMT `ADAPTER_PERFDATA`).
3. Réglages egui + tray + raccourcis globaux + config JSON.
4. Installeur (binaire unique + tâche du capteur élevé).
