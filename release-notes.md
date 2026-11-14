## FPS Counter & Crosshair — v1.0.8

Overlay de performance et réticule pour Windows 11.

### Fonctionnalités
- FPS, temps de frame, 1 % low et 0,1 % low lus via les événements de composition Windows
- Charge CPU/GPU, fréquences, VRAM, RAM, températures
- **Température CPU via le SDK AMD** (`Platform.dll!GetRmCpuParameters`) — aucun pilote tiers
- Dix réticules + éditeur, profils, thèmes, alertes, ping
- **Détection de jeu** : l'overlay ne s'affiche que lorsqu'un jeu tourne au premier plan (compteurs GPU Engine 3D + liste noire + images DWM), plus rien sur le bureau
- Quatre dispositions, position/opacité/échelle réglables, raccourcis globaux

### Installation
1. Lancer `FPS-Counter-Crosshair-Pro-Setup.exe` (administrateur).
2. L'installeur enregistre le paquet, crée la tâche du pont de température et les raccourcis.
3. Prérequis température CPU : AMD Software (Adrenalin) installé.

### Désinstallation
Applications → FPS Counter & Crosshair.

### SHA256
`à recalculer`