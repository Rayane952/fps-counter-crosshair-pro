$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$manifest = Join-Path $here 'AppxManifest.xml'

# 1. mode développeur (requis pour enregistrer un paquet non signé)
$k = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock'
if (-not (Test-Path $k)) { New-Item -Path $k -Force | Out-Null }
New-ItemProperty -Path $k -Name 'AllowDevelopmentWithoutDevLicense' -Value 1 -PropertyType DWord -Force | Out-Null
New-ItemProperty -Path $k -Name 'AllowAllTrustedApps' -Value 1 -PropertyType DWord -Force | Out-Null

# 2. enregistrement du paquet
$existing = Get-AppxPackage -Name 'KoykStudio.GameOverlayPro' -ErrorAction SilentlyContinue
if ($existing) { Remove-AppxPackage -Package $existing.PackageFullName -ErrorAction SilentlyContinue }
Add-AppxPackage -Register $manifest

# 3. pont température (tâche au logon, privilèges élevés)
$exe = Join-Path $here 'Bridge2.exe'
& schtasks.exe /delete /tn 'OverlayTempBridge' /f 2>&1 | Out-Null
& schtasks.exe /create /tn 'OverlayTempBridge' /tr $exe /sc onlogon /rl HIGHEST /f 2>&1 | Out-Null

# 4. démarrage
Start-Process 'explorer.exe' 'shell:AppsFolder\KoykStudio.GameOverlayPro_516232dwgyvgy!App'
