$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$manifest = Join-Path $here 'AppxManifest.xml'

$k = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock'
if (-not (Test-Path $k)) { New-Item -Path $k -Force | Out-Null }
New-ItemProperty -Path $k -Name 'AllowDevelopmentWithoutDevLicense' -Value 1 -PropertyType DWord -Force | Out-Null
New-ItemProperty -Path $k -Name 'AllowAllTrustedApps' -Value 1 -PropertyType DWord -Force | Out-Null

$existing = Get-AppxPackage -Name 'KoykStudio.GameOverlayPro' -ErrorAction SilentlyContinue
if ($existing) { Remove-AppxPackage -Package $existing.PackageFullName -ErrorAction SilentlyContinue }
Add-AppxPackage -Register $manifest

$data = 'C:\ProgramData\OverlayTemp'
New-Item -ItemType Directory -Force -Path $data | Out-Null
Copy-Item (Join-Path $here 'GameWatch.exe') $data -Force -ErrorAction SilentlyContinue
& icacls.exe $data /grant '*S-1-5-32-545:(OI)(CI)M' 2>&1 | Out-Null

& schtasks.exe /delete /tn 'OverlayTempBridge' /f 2>&1 | Out-Null
& schtasks.exe /create /tn 'OverlayTempBridge' /tr (Join-Path $here 'Bridge2.exe') /sc onlogon /rl HIGHEST /f 2>&1 | Out-Null
& schtasks.exe /delete /tn 'OverlayGameWatch' /f 2>&1 | Out-Null
& schtasks.exe /create /tn 'OverlayGameWatch' /tr ('"' + (Join-Path $data 'GameWatch.exe') + '" "' + (Join-Path $data 'gpu_pids.txt') + '" 2000') /sc onlogon /f 2>&1 | Out-Null

& schtasks.exe /run /tn 'OverlayTempBridge' 2>&1 | Out-Null
& schtasks.exe /run /tn 'OverlayGameWatch' 2>&1 | Out-Null

Start-Process 'explorer.exe' 'shell:AppsFolder\KoykStudio.GameOverlayPro_516232dwgyvgy!App'
