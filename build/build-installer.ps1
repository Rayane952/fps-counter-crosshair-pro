# Reconstruit l'installeur depuis le paquet Store d'origine (admin requis)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$pkg  = Get-AppxPackage -Name 'KoykStudio.GameOverlayFPSCounterCrosshairCPUGPUTem'
if (-not $pkg) { throw 'Paquet Store introuvable.' }

$payload = Join-Path $root 'installer/payload'
New-Item -ItemType Directory -Force -Path $payload | Out-Null
robocopy $pkg.InstallLocation $payload /E /NFL /NDL /NJH /NJS | Out-Null

$dll = Join-Path $payload 'GameOverlay.dll'
& (Join-Path $root 'tools/patcher.exe') $dll (Join-Path $payload 'p1.dll')
& (Join-Path $root 'tools/patch3.exe') (Join-Path $payload 'p1.dll') (Join-Path $payload 'p2.dll')
& (Join-Path $root 'tools/patch4.exe') (Join-Path $payload 'p2.dll') (Join-Path $payload 'p3.dll')
& (Join-Path $root 'tools/patch5.exe') (Join-Path $payload 'p3.dll') (Join-Path $payload 'p4.dll')
& (Join-Path $root 'tools/patch6.exe') (Join-Path $payload 'p4.dll') (Join-Path $payload 'p5.dll')
& (Join-Path $root 'tools/patch7.exe') (Join-Path $payload 'p5.dll') (Join-Path $payload 'p6.dll')
Copy-Item (Join-Path $payload 'p6.dll') $dll -Force

$mf = Join-Path $payload 'AppxManifest.xml'
(Get-Content $mf -Raw) -replace 'KoykStudio\.GameOverlayFPSCounterCrosshairCPUGPUTem','KoykStudio.GameOverlayPro' -replace 'Version="1\.0\.7\.0"','Version="1.0.8.0"' | Set-Content $mf -Encoding UTF8

& (Join-Path $root 'tools/inno/ISCC.exe') (Join-Path $root 'installer/setup.iss')
