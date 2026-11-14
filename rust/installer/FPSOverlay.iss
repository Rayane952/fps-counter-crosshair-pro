; Installeur de FPS Overlay (application Rust).
; Compilation : ISCC.exe FPSOverlay.iss  (après cargo build --release)
; Sortie : ..\..\dist\FPS-Overlay-Setup-<version>.exe

#define AppName "FPS Overlay"
#define AppVersion "1.0.0"
#define Bin "..\target\release"
#define Assets "..\assets"
#define SensorTask "FPSOverlaySensor"

[Setup]
AppId={{B3F1A6D2-5C47-4E8B-9A1D-2F6C8E4B7A90}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Rayane
VersionInfoVersion={#AppVersion}
VersionInfoDescription=Installation de {#AppName}
DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
; administrateur : la tâche du capteur de température doit tourner avec élévation
PrivilegesRequired=admin
; la clé Run et les anciens fichiers de développement sont ceux de l'utilisateur qui installe
UsedUserAreasWarning=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist
OutputBaseFilename=FPS-Overlay-Setup-{#AppVersion}
SetupIconFile={#Assets}\fps-overlay.ico
WizardSmallImageFile={#Assets}\logo-256.png
UninstallDisplayIcon={app}\FPSOverlay.exe
UninstallDisplayName={#AppName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
CloseApplications=no

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "autostart"; Description: "Lancer {#AppName} au démarrage de Windows (recommandé)"
Name: "desktopicon"; Description: "Créer une icône sur le Bureau"; Flags: unchecked

[Files]
Source: "{#Bin}\FPSOverlay.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Bin}\FPSOverlaySettings.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Bin}\FPSOverlaySensor.exe"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
; anciennes versions de développement (dossier copié à la main, script de démarrage, raccourci)
Type: filesandordirs; Name: "{localappdata}\Programs\FPSOverlayRust"
Type: files; Name: "{userstartup}\FPSOverlayRust.vbs"
Type: files; Name: "{userprograms}\FPS Overlay.lnk"

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\FPSOverlaySettings.exe"; Comment: "Réglages de l'overlay FPS"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\FPSOverlaySettings.exe"; Comment: "Réglages de l'overlay FPS"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#AppName}"; ValueData: """{app}\FPSOverlay.exe"""; Tasks: autostart; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "{#AppName}"; Flags: deletevalue; Check: not WizardIsTaskSelected('autostart')

[Run]
; capteur de température CPU : tâche planifiée élevée, lancée à chaque ouverture de session
Filename: "{sys}\schtasks.exe"; Parameters: "/Create /F /TN ""{#SensorTask}"" /SC ONLOGON /RL HIGHEST /TR ""\""{app}\FPSOverlaySensor.exe\"""""; StatusMsg: "Installation du capteur de température..."; Flags: runhidden waituntilterminated
Filename: "{sys}\schtasks.exe"; Parameters: "/Run /TN ""{#SensorTask}"""; Flags: runhidden waituntilterminated
; l'overlay tourne en arrière-plan (zone de notification) avec les droits normaux de l'utilisateur
Filename: "{app}\FPSOverlay.exe"; Flags: nowait runasoriginaluser
Filename: "{app}\FPSOverlaySettings.exe"; Description: "Ouvrir les réglages de {#AppName}"; Flags: postinstall nowait skipifsilent runasoriginaluser

[UninstallRun]
Filename: "{sys}\schtasks.exe"; Parameters: "/End /TN ""{#SensorTask}"""; Flags: runhidden waituntilterminated; RunOnceId: "EndSensor"
Filename: "{sys}\schtasks.exe"; Parameters: "/Delete /F /TN ""{#SensorTask}"""; Flags: runhidden waituntilterminated; RunOnceId: "DeleteSensor"

[UninstallDelete]
Type: filesandordirs; Name: "{app}"

[Code]
{ Arrête l'overlay, les réglages et le capteur (nouveaux noms, et anciens noms du dossier de
  développement) pour pouvoir remplacer ou supprimer les fichiers. Filtré par chemin : aucun
  autre programme portant le même nom n'est touché. }
procedure StopRunning();
var
  Code: Integer;
  Script: String;
begin
  Script := 'Get-Process FPSOverlay,FPSOverlaySettings,FPSOverlaySensor,overlay,settings,sensor -ErrorAction SilentlyContinue | ' +
            'Where-Object { $_.Path -like ''*\FPSOverlayRust\*'' -or $_.Path -like ''*\FPS Overlay\*'' } | ' +
            'Stop-Process -Force -ErrorAction SilentlyContinue';
  Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
       '-NoProfile -ExecutionPolicy Bypass -Command "' + Script + '"',
       '', SW_HIDE, ewWaitUntilTerminated, Code);
  Sleep(500);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
    StopRunning();
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    StopRunning();
end;
