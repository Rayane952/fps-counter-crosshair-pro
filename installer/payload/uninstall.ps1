$ErrorActionPreference = 'SilentlyContinue'
& schtasks.exe /end /tn 'OverlayTempBridge' 2>&1 | Out-Null
& schtasks.exe /delete /tn 'OverlayTempBridge' /f 2>&1 | Out-Null
& schtasks.exe /end /tn 'OverlayGameWatch' 2>&1 | Out-Null
& schtasks.exe /delete /tn 'OverlayGameWatch' /f 2>&1 | Out-Null
Get-Process Bridge2,GameWatch,GameOverlay -ErrorAction SilentlyContinue | Stop-Process -Force
$p = Get-AppxPackage -Name 'KoykStudio.GameOverlayPro'
if ($p) { Remove-AppxPackage -Package $p.PackageFullName }
