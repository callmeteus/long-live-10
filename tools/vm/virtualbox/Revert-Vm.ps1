# Revert-Vm.ps1 (VirtualBox) - Restores VM to baseline snapshot
param(
    [string]$VmName = "ll10-22h2",

    [string]$Snapshot = "baseline"
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found." }

# Stop VM if running
$running = & $vb list runningvms 2>&1
if ($running -match $VmName) {
    Write-Host "Stopping VM..."
    & $vb controlvm $VmName poweroff 2>$null
    Start-Sleep -Seconds 3
}

# Restore snapshot
Write-Host "Restoring snapshot '$Snapshot'..."
& $vb snapshot $VmName restore $Snapshot
if ($LASTEXITCODE -ne 0) { throw "Failed to restore snapshot '$Snapshot'." }

Write-Host "Snapshot restored. Start with: yarn vm:start"