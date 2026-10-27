# Revert-Vm.ps1 (VirtualBox) - Restores the VM to the baseline snapshot
param(
    [string]$VmName = "ll10-22h2",

    [string]$Snapshot = "baseline"
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found." }

# Stop VM if running
$running = & $vb list runningvms 2>&1
if ($running -match [regex]::Escape($VmName)) {
    Write-Host "Stopping VM..."
    try { & $vb controlvm $VmName poweroff 2>&1 | Out-Null } catch { }

    $deadline = (Get-Date).AddMinutes(2)
    while ((Get-Date) -lt $deadline) {
        $info = & $vb showvminfo $VmName --machinereadable 2>&1
        if ($info -match '^VMState="powered off"') { break }
        Start-Sleep -Seconds 3
    }
}

# Restore snapshot
Write-Host "Restoring snapshot '$Snapshot'..."
$previousEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& $vb snapshot $VmName restore $Snapshot | Out-Null
$restoreExit = $LASTEXITCODE
$ErrorActionPreference = $previousEap
if ($restoreExit -ne 0) { throw "Failed to restore snapshot '$Snapshot'." }

Write-Host "Snapshot restored. Start with: yarn vm:start"
