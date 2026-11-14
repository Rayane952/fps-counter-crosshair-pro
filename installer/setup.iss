; Installeur FPS Counter & Crosshair Pro (patché)
#define AppName "FPS Counter & Crosshair Pro"
#define AppVersion "1.0.8"

[Setup]
AppId={{7C4E2B1A-9F3D-4A55-9C1E-6B2D8A0F4E11}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=KoykStudio (build patché)
DefaultDirName={localappdata}\Programs\OverlayMod
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=admin
OutputDir=..\dist
OutputBaseFilename=FPS-Counter-Crosshair-Pro-Setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\GameOverlay.exe

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Files]
Source: "payload\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "explorer.exe"; Parameters: "shell:AppsFolder\KoykStudio.GameOverlayPro_516232dwgyvgy!App"
Name: "{userdesktop}\{#AppName}"; Filename: "explorer.exe"; Parameters: "shell:AppsFolder\KoykStudio.GameOverlayPro_516232dwgyvgy!App"

[Run]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\install.ps1"""; StatusMsg: "Enregistrement du paquet et installation du pont..."; Flags: runhidden waituntilterminated

[UninstallRun]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File ""{app}\uninstall.ps1"""; Flags: runhidden waituntilterminated
