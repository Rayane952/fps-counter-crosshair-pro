# FPS Counter & Crosshair

Overlay de mesure de performance et réticule pour Windows 11, basé sur le paquet MSIX
**KoykStudio.GameOverlayFPSCounterCrosshairCPUGPUTem**, déployé en paquet de développement
`KoykStudio.GameOverlayPro` (dossier inscriptible dans `%LOCALAPPDATA%\Programs\OverlayMod`).

L'application lit les images via les événements de composition de Windows : aucun hook, aucune
injection dans les jeux.

## Fonctionnalités

**Overlay de statistiques**

| Mesure | Source |
|---|---|
| FPS, temps de frame, 1 % low, 0,1 % low | `DwmFlush` / présentation des images par Windows |
| Charge CPU, fréquence, RAM | compteurs de performance Windows (PDH) |
| Charge GPU, VRAM, température GPU | `D3DKMT` (données de performance de l'adaptateur, aucun pilote tiers) |
| **Température CPU** | **SDK AMD (`Platform.dll`) — voir plus bas** |
| Ping, perte de paquets, trafic | écho ICMP vers l'hôte choisi |

Quatre dispositions : panneau, panneau + graphique, ligne compacte, minimal.
Thèmes : clair, sombre, néon, minimal, dense — opacité, échelle, position et marges réglables.

**Réticule**

Dix réticules prêts à l'emploi plus un éditeur (bras, écart, épaisseur, point central,
anneau, contour, chevron, diagonales), couleur, taille et opacité. Affichage permanent ou
uniquement en jeu, masqué des captures d'écran si besoin.

**Profils et rapports**

Profils de disposition sauvegardables, application automatique par jeu, enregistrement de
sessions (FPS moyen, 1 % low, températures) et export.

**Alertes**

Seuils sur FPS, température CPU/GPU, ping, perte de paquets et trafic, avec retour visuel
dans l'overlay.

**Raccourcis**

`Alt+Shift+O` affiche/masque les statistiques, `Alt+Shift+X` le réticule (modifiables).

## Température CPU : d'où vient la valeur

Windows n'expose pas la sonde du CPU sur les cartes AM5 : les compteurs
`\Thermal Zone Information(*)` sont absents et `MSAcpi_ThermalZoneTemperature` répond
« non pris en charge ». La valeur est donc lue sur la chaîne officielle d'AMD :

```
AMD Software: Adrenalin
   └─ RyzenMasterSDK (installé par Adrenalin)
        └─ AMDRyzenMasterDriver.sys   (ring0, service AMDRyzenMasterDriverV32)
             └─ SMU → registre THM_TCON_CUR_TMP (SMN 0x00059800)

Platform.dll  →  GetRmCpuParameters(void* out)     [export C, __cdecl, élévation requise]
                 offset 0x90 : double = température CPU (°C)
                 offset 0x14 : float  = puissance package (W)
                 offset 0x20 : float  = courant (A)
```

`Bridge2.exe` appelle cet export toutes les 2 s et publie la valeur dans
`C:\ProgramData\OverlayTemp\cpu.txt`, que l'overlay relit. La tâche planifiée
`OverlayTempBridge` le lance à l'ouverture de session avec les privilèges nécessaires
à l'accès au pilote.

Aucun pilote supplémentaire n'est installé : tout vient d'AMD Software. Si Adrenalin est
désinstallé, la température CPU cesse simplement de s'afficher (l'overlay garde la dernière
valeur connue, sans erreur).

## Installation

1. Récupérer `FPS-Counter-Crosshair-Pro-Setup.exe` depuis les releases.
2. Lancer l'installeur (droits administrateur : activation du mode développeur, enregistrement
   du paquet, création de la tâche du pont).
3. Prérequis pour la température CPU : AMD Software installé.

Désinstallation via Applications → *FPS Counter & Crosshair* (retire la tâche, le paquet,
les raccourcis).

## Configuration

`%LOCALAPPDATA%\Packages\KoykStudio.GameOverlayPro_516232dwgyvgy\LocalState\state.json`
(écrit par l'application) : disposition, thème, métriques affichées, réticule, raccourcis,
seuils d'alerte, profils.

## Structure du dépôt

```
src/bridge/      pont température (SDK AMD) + outil de relevé des champs
src/tools/       utilitaires de post-traitement de l'assembly
installer/       script Inno Setup + install.ps1 / uninstall.ps1
build/           reconstruction de l'installeur
docs/            captures
```

```powershell
build\build-installer.ps1     # → dist\FPS-Counter-Crosshair-Pro-Setup.exe
```

## Notes

- Compatible Windows 11 x64 (le paquet d'origine cible `Windows.Desktop 10.0.17763`).
- Aucune collecte : les seuls accès réseau sont le Microsoft Store (mises à jour), Sentry
  (rapports de plantage, désactivables) et l'écho ICMP du ping.