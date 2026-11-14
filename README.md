# FPS Counter & Crosshair — Build patché (Pro permanent + capteur AMD)

Patchs appliqués au paquet MSIX **KoykStudio.GameOverlayFPSCounterCrosshairCPUGPUTem 1.0.7**,
redéployé en paquet de développement `KoykStudio.GameOverlayPro` (dossier inscriptible : WindowsApps
est verrouillé en écriture sur Windows 11 26xxx, même en SYSTEM/TrustedInstaller).

| # | Patch | Effet |
|---|---|---|
| 01 | `src/patchers/01-pro.cs` | ProLicensed/IsPro → true ; InTrial/TrialEnded → false (Pro permanent) |
| 02 | `src/patchers/02-sensor.cs` | Sensors.ReadThermalZoneC lit C:\ProgramData\OverlayTemp\cpu.txt |
| 03 | `src/patchers/03-hud-fonts.cs` | températures au format des FPS (Segoe UI Variable Display, SemiBold 15·k) |
| 04 | `src/patchers/04-hud-column.cs` | suppression du placeholder de largeur "888%" |
| 05 | `src/patchers/05-hud-rows.cs` | sans %, la température passe dans la colonne valeur (alignée sous les FPS) |
| 06 | `src/patchers/06-single-window.cs` | garde de ré-entrance sur App::OpenSettings (plus de double fenêtre) |

## Température CPU — source AMD

```
GetRmCpuParameters(void*)  →  rc=0 (nécessite l'élévation)
   offset 0x90 : double = température CPU (°C)
   offset 0x14 : float  = puissance package (W)
```

Appelé depuis Platform.dll (SDK Ryzen Master installé par Adrenalin), via `src/bridge/Bridge2.cs`.
Écrit C:\ProgramData\OverlayTemp\cpu.txt toutes les 2 s ; l'app patchée le lit.
Tâche planifiée OverlayTempBridge (au logon, privilèges élevés). Aucun driver tiers.

## Installeur

`installer/setup.iss` (Inno Setup 6) embarque le payload complet, active le mode développeur,
enregistre le paquet, crée la tâche et les raccourcis.

```powershell
build\build-installer.ps1     # → dist\FPS-Counter-Crosshair-Pro-Setup.exe
```

## Structure

```
src/patchers/    patchs IL (Mono.Cecil)
src/bridge/      pont température (SDK AMD)
installer/       script Inno Setup + install.ps1 / uninstall.ps1
docs/            captures
```

## Notes

- Paquet de développement : nécessite le mode développeur (activé par l'installeur).
- Dépend d'AMD Software (Platform.dll + AMDRyzenMasterDriverV32) pour la température CPU.
