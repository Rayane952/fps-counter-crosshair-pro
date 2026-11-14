## FPS Counter & Crosshair — build patché v1.0.8

Installeur complet du paquet patché (base : KoykStudio 1.0.7, réenregistré en paquet de développement).

### Contenu
- **Pro permanent** : `ProLicensed`/`IsPro` forcés à `true`, `InTrial`/`TrialEnded` à `false` (plus de compte à rebours ni de bandeau d'essai).
- **Température CPU** via le SDK AMD déjà installé par Adrenalin (`Platform.dll!GetRmCpuParameters`, offset `0x90`) — aucun driver tiers, plus de PawnIO / LibreHardwareMonitor. Pont `Bridge2.exe` + tâche planifiée au logon.
- **HUD** : températures au même format que les FPS (Segoe UI Variable Display, SemiBold, 15×k), alignées dans la colonne des valeurs.
- **Fenêtre unique** : garde de ré-entrance sur `App::OpenSettings` (plus de double fenêtre au clic).

### Installation
1. Lancer `FPS-Counter-Crosshair-Pro-Setup.exe` (admin requis).
2. L'installeur active le mode développeur, enregistre le paquet, crée la tâche du pont et les raccourcis.
3. Prérequis température CPU : AMD Software (Adrenalin) installé — le driver `AMDRyzenMasterDriverV32` et `Platform.dll` viennent de lui.

### Désinstallation
Panneau de configuration → FPS Counter & Crosshair Pro (retire la tâche, le paquet et les fichiers).

### SHA256
`BA1DDEF9804C2A9F22D9B8549F4F4A51A6645D7775A89C0CBF5C693DF1FA2AB9`
