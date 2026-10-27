# Finalize-Vm.ps1 (VirtualBox) - Provisions the guest, then records the baseline snapshot.
#
# Flow:
#   1. Ensure the VM is running and WinRM answers (auto-provisioned by Setup-Vm).
#   2. Shut the guest down gracefully so registry hives are consistent.
#   3. Run Prepare-Guest.ps1 on the powered-off disk (idempotent).
#   4. Delete any previous snapshot and take a fresh 'baseline'.
param(
    [string]$VmName = "ll10-22h2",

    [string]$Snapshot = "baseline",

    [string]$UserName = "ll10",

    [string]$Password = "Ll10Test!2026",

    [int]$WinRmPort = 55985
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found." }

$scriptRoot = $PSScriptRoot

# ---------------------------------------------------------------------------
# 1. Make sure the VM is up and reachable
# ---------------------------------------------------------------------------
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptRoot "Start-Vm.ps1") -VmName $VmName -WinRmPort $WinRmPort
if ($LASTEXITCODE -ne 0) { throw "VM did not become reachable over WinRM." }

# ---------------------------------------------------------------------------
# 2. Graceful shutdown
# ---------------------------------------------------------------------------
Write-Host "Shutting down the guest..."
$secure = ConvertTo-SecureString $Password -AsPlainText -Force
$credential = New-Object System.Management.Automation.PSCredential($UserName, $secure)

try {
    Invoke-Command -ComputerName "127.0.0.1" -Port $WinRmPort -Credential $credential `
        -Authentication Negotiate `
        -SessionOption (New-PSSessionOption -SkipCACheck -SkipCNCheck -SkipRevocationCheck) `
        -ScriptBlock { shutdown.exe /s /t 0 } -ErrorAction SilentlyContinue
} catch { }

$deadline = (Get-Date).AddMinutes(3)
while ((Get-Date) -lt $deadline) {
    $info = & $vb showvminfo $VmName --machinereadable 2>&1
    if ($info -match '^VMState="powered off"') { break }
    Start-Sleep -Seconds 5
}

# ---------------------------------------------------------------------------
# 3. Offline provisioning (authoritative; survives future boots)
# ---------------------------------------------------------------------------
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $scriptRoot "Prepare-Guest.ps1") -VmName $VmName -UserName $UserName -Password $Password
if ($LASTEXITCODE -ne 0) { throw "Guest provisioning failed." }

# ---------------------------------------------------------------------------
# 4. Baseline snapshot
# ---------------------------------------------------------------------------
$snapshots = ""
try { $snapshots = & $vb snapshot $VmName list 2>&1 | Out-String } catch { }
if ($snapshots -match [regex]::Escape($Snapshot)) {
    try { & $vb snapshot $VmName delete $Snapshot 2>&1 | Out-Null } catch { }
}

Write-Host "Creating snapshot '$Snapshot'..."
$previousEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& $vb snapshot $VmName take $Snapshot | Out-Null
$snapshotExit = $LASTEXITCODE
$ErrorActionPreference = $previousEap
if ($snapshotExit -ne 0) { throw "Failed to create snapshot '$Snapshot'." }

Write-Host "Snapshot '$Snapshot' created." -ForegroundColor Green
Write-Host "Use 'yarn vm:revert' to restore this state."
