# Installs the LL10 UI daemon inside the guest so it starts on logon.
# Run this ONCE via WinRM (the short window after boot is enough).
# After installation, use packages/vm-daemon/client/vmctl.ps1 from the host;
# no further WinRM access is required.
param(
    [string]$GuestDaemonPath = "C:\ll10\ll10-ui-daemon.ps1"
)

$ErrorActionPreference = "Stop"
$source = Join-Path $PSScriptRoot "..\daemon\ll10-ui-daemon.ps1"
if (-not (Test-Path $source)) { throw "daemon script not found: $source" }

New-Item -ItemType Directory -Path "C:\ll10" -Force | Out-Null
Copy-Item -Path $source -Destination $GuestDaemonPath -Force

$startup = [Environment]::GetFolderPath("Startup")
$WshShell = New-Object -ComObject WScript.Shell
$shortcut = $WshShell.CreateShortcut((Join-Path $startup "LL10UiDaemon.lnk"))
$shortcut.TargetPath = "powershell.exe"
$shortcut.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$GuestDaemonPath`""
$shortcut.Save()

Write-Output "Installed daemon at $GuestDaemonPath"
Write-Output "Startup shortcut: $(Join-Path $startup 'LL10UiDaemon.lnk')"
Write-Output "Restart the guest once, then use vmctl.ps1 from the host."
