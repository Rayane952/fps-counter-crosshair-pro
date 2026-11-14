# Migration Rust — plan de conversion

## 1. Pourquoi

Mesures sur la version actuelle (WinUI 3 / .NET 10), au repos, overlay affiché :

| Processus | RAM (working set) |
|---|---|
| `GameOverlay.exe` | **276,8 Mo** (privé 234,6 Mo, 2 710 handles) |
| WebView2 (7 sous-processus) | **≈ 383 Mo** |
| `GameWatch.exe` | 35,1 Mo |
| `Bridge2.exe` | 10,1 Mo |
| **Total** | **≈ 705 Mo** |

Coût structurel, pas un bug : runtime .NET + WinUI 3 (Microsoft.WinUI 7,3 Mo, Microsoft.Windows.SDK.NET 24,9 Mo,
ressources .pri) + pile WebView2 pour l'interface. D'où les à-coups à l'ouverture : la fenêtre WinUI et
WebView2 se construisent à froid sur le thread UI (le patch « garde 5 s » n'y change rien, il ne fait
qu'éviter la double fenêtre).

Cible Rust : **< 60 Mo** au total, **< 0,3 % CPU** au repos, ouverture de la fenêtre **< 30 ms**,
coût du panneau **< 0,3 ms/frame**, binaire unique **< 10 Mo**.

## 2. Architecture cible

```
crates/
  ovl-core      télémetrie : frames, CPU, GPU, capteurs, sessions   (aucune dépendance UI)
  ovl-overlay   rendu Direct2D/DirectWrite : panneau, réticule, DPI, click-through
  ovl-ui        fenêtre de réglages (egui), i18n, preview live
  ovl-platform  tray, hotkeys, autostart, single-instance, config
  ovl-app       binaire : assemblage, boucle d'événements, état
```

Découpage volontairement identique au décompilé, pour porter module par module.

## 3. Correspondance C# → Rust

| Décompilé (.NET) | Crate | Bibliothèque / API Rust |
|---|---|---|
| `Overlay.FrameMonitor` | ovl-core :: frames | `windows` (ETW `Microsoft-Windows-DXGI` / `DxgKrnl` Present) ou `DwmGetCompositionFrameStats` |
| `Services.Sensors` (CPU/GPU/RAM) | ovl-core :: cpu/gpu | `windows` : `PdhOpenQuery`/`PdhGetFormattedCounterArray`, `D3DKMT` |
| `Sensors.ReadThermalZoneC` (SDK AMD) | ovl-core :: amd | `libloading` + `GetRmCpuParameters` (offset 0x90, déjà identifié) |
| `Overlay.OverlayHost` | ovl-overlay :: host | `windows` : fenêtre layered `WS_EX_LAYERED`/`TRANSPARENT`/`TOOLWINDOW`, `SetWindowPos` |
| `Overlay.PanelPainter` | ovl-overlay :: painter | `windows` : D2D1 + DWrite (Direct2D 1.1), `CreateTextFormat` |
| `Overlay.DrawCrosshair` | ovl-overlay :: crosshair | D2D1 (geometry) |
| `App` + `Views.*Page` (XAML) | ovl-ui | `egui`/`eframe` (immédiat, ~20 Mo, démarrage < 50 ms) |
| `TrayIcon` | ovl-platform :: tray | `tray-icon` |
| `Hotkey` | ovl-platform :: hotkeys | `global-hotkey` |
| `AppState` (state.json) | ovl-platform :: config | `serde` + `serde_json` |
| `Sessions` / `Reports` | ovl-core :: sessions | `serde`, `csv` |
| `Alerts` | ovl-core :: alerts | logique pure |
| `L.T()` / .pri (24 langues) | ovl-ui :: i18n | `fluent` (Fluent .ftl) — reprise des chaînes existantes |
| `StoreContext` (licence) | — | supprimé : paquet de développement, aucune dépendance Store |
| `Sentry` | ovl-app :: crash | `sentry` (optionnel, feature flag) |

## 4. Phasage

| Phase | Contenu | Sortie | Durée |
|---|---|---|---|
| 0 | Audit : figer la liste des fonctionnalités depuis le décompilé, baseline perf (les chiffres ci-dessus) | `BASELINE.md` | 2 j |
| 1 | `ovl-core` : frames, CPU, GPU, RAM, capteur AMD ; CLI `ovl-probe` qui imprime le JSON des stats | parité chiffrée vs app actuelle | 2–3 sem. |
| 2 | `ovl-overlay` : fenêtre click-through par moniteur, panneau D2D, 4 dispositions, thèmes, DPI | overlay autonome | 2 sem. |
| 3 | `ovl-platform` : tray, raccourcis, config, instance unique, autostart | app fonctionnelle sans UI de réglages | 1 sem. |
| 4 | `ovl-ui` : pages Overlay / Réticule / Profils / Rapports / Paramètres, preview live, i18n | parité UI | 3–4 sem. |
| 5 | Sessions, rapports, alertes | parité complète | 1 sem. |
| 6 | Packaging : binaire unique, installeur Inno, mise à jour, signature | `FPS-Counter-Setup.exe` | 3 j |
| 7 | Validation : profils de perf, non-régression capteurs, 24 langues | `PERF.md` | 1 sem. |

Total réaliste : **10 à 13 semaines** à temps plein, 4 à 6 semaines en réutilisant l'existant
(capteur AMD, géométrie du HUD, chaînes de traduction).

## 5. Choix techniques à trancher

1. **Frames** : ETW `Present` (fiable, toutes API, mais session ETW = droits élevés) vs
   `DwmGetCompositionFrameStats` (non documenté, sans admin, c'est ce que fait l'app actuelle).
   → garder DWM en principal, ETW en secours.
2. **UI** : `egui` (rapide, look différent) vs `slint` (proche du XAML, plus lourd) vs Tauri (web, RAM).
   → `egui`, avec thème proche de l'actuel.
3. **Rendu** : D2D1 (comme aujourd'hui, parité visuelle) vs `wgpu` (portable, plus de code).
   → D2D1, aucune raison de changer.

## 6. Risques

- **Parité visuelle** du HUD : D2D1 identique ⇒ risque faible, mais les formats de texte doivent être
  reproduits au point près (Segoe UI Variable Display, 600, 15·k).
- **ETW sans admin** : à valider tôt (phase 1), sinon on garde DWM.
- **24 langues** : volume de traduction à migrer vers `.ftl` (mécanique, mais long).
- **Régressions capteurs** : le CPU AMD dépend d'Adrenalin installé — déjà le cas aujourd'hui.
- **Effort** : c'est une réécriture, pas un portage ; l'UI XAML (≈ 60 % du code décompilé) est à refaire.

## 7. Gain attendu

| Métrique | Actuel | Cible Rust |
|---|---|---|
| RAM totale | ≈ 705 Mo | **< 60 Mo** |
| Démarrage (fenêtre visible) | 0,8–1,5 s | **< 30 ms** |
| Coût CPU au repos | ~1–2 % | **< 0,3 %** |
| Taille installée | 116 Mo (+ runtime) | **< 12 Mo** |
| Latence d'ouverture des réglages | 300–1000 ms | **< 50 ms** |
