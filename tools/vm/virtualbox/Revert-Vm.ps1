# Revert-Vm.ps1 (VirtualBox) - Returns the VM to a known-good state.
#
# Two paths, in order of preference:
#
#   1. Restore the 'baseline' snapshot (fast, exact).
#   2. If no snapshot exists, rebuild the VM from the base image next to the
#      guest disk (slow but still fully reproducible).
#
# The VM is hard-powered-off rather than shut down gracefully. Both paths above
# replace or discard the current disk contents, so there is nothing to preserve:
# a graceful ACPI shutdown would only add 30-90 s of waiting and then be
# discarded anyway. (Prepare-Guest.ps1 still shuts the guest down gracefully,
# because there the live disk - and its registry hives - are edited in place.)
#
# The script decides whether a reset is possible *before* touching the VM, so a
# failed reset never leaves the operator with a killed guest and nothing to
# restore.

[CmdletBinding()]
param(
    [string]$VmName = "ll10-22h2",

    [string]$Snapshot = "baseline",

    # Optional explicit base image used when there is no snapshot.
    [string]$BaseImage = ""
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found at $vb." }

function Invoke-VBox {
    param([string[]]$Arguments)

    $stdout = [System.IO.Path]::GetTempFileName()
    $stderr = [System.IO.Path]::GetTempFileName()
    try {
        $commandLine = ($Arguments | ForEach-Object {
            if ($_ -match '\s') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ }
        }) -join ' '

        $proc = Start-Process -FilePath $vb -ArgumentList $commandLine -Wait -PassThru `
            -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr

        $text = (Get-Content $stdout -Raw -ErrorAction SilentlyContinue) +
                (Get-Content $stderr -Raw -ErrorAction SilentlyContinue)
        return [pscustomobject]@{
            ExitCode = $proc.ExitCode
            Output   = ($text | Out-String).Trim()
        }
    }
    finally {
        Remove-Item $stdout, $stderr -Force -ErrorAction SilentlyContinue
    }
}

function Get-VmState {
    # During power transitions VBoxSVC briefly locks the machine and
    # 'showvminfo' fails. Treat that as "not yet known" and retry, otherwise a
    # successful power-off is misreported as a failure.
    for ($attempt = 0; $attempt -lt 4; $attempt++) {
        $result = Invoke-VBox -Arguments @("showvminfo", $VmName, "--machinereadable")
        if ($result.ExitCode -eq 0) {
            $line = ($result.Output -split "`r?`n") | Where-Object { $_ -match '^VMState=' } | Select-Object -First 1
            if ($line) { return ($line -replace '^VMState="', '' -replace '"$', '') }
        }
        Start-Sleep -Milliseconds 750
    }
    return $null
}

function Get-GuestDiskPath {
    $info = Invoke-VBox -Arguments @("showvminfo", $VmName, "--machinereadable")
    $line = ($info.Output -split "`r?`n") | Where-Object { $_ -match '^"SATA-0-0"=' } | Select-Object -First 1
    if (-not $line) { return $null }
    return ($line -replace '^"SATA-0-0"="', '' -replace '"$', '')
}

function Test-VmOff {
    # VirtualBox reports several spellings for "not running":
    # 'powered off' after a clean stop, 'poweroff'/'aborted' after controlvm
    # poweroff. All of them mean the disk is free to be restored or replaced.
    $state = Get-VmState
    return ($state -in @("powered off", "poweroff", "aborted", "saved"))
}

if ($null -eq (Get-VmState)) {
    throw "VM '$VmName' not found. Run 'yarn vm:setup' first."
}

# ---------------------------------------------------------------------------
# Decide how to reset BEFORE touching the VM
# ---------------------------------------------------------------------------
$snapshotList = Invoke-VBox -Arguments @("snapshot", $VmName, "list")
$hasSnapshot = $snapshotList.ExitCode -eq 0 -and
    $snapshotList.Output -match "Name:\s+$([regex]::Escape($Snapshot))\b"

$rebuildImage = $null
if (-not $hasSnapshot) {
    if (-not $BaseImage) {
        $diskPath = Get-GuestDiskPath
        if ($diskPath) {
            $candidate = Join-Path (Split-Path $diskPath -Parent) "base-image.vhd"
            if (Test-Path $candidate) { $BaseImage = $candidate }
        }
    }
    if ($BaseImage -and (Test-Path $BaseImage)) {
        $rebuildImage = $BaseImage
    }
}

if (-not $hasSnapshot -and -not $rebuildImage) {
    throw @"
No snapshot '$Snapshot' and no base image were found, so '$VmName' cannot be reset.

Create a snapshot by re-running setup (it records one):
    yarn vm:setup -BaseImage <path-to-base-image.vhd>

or pass the base image explicitly:
    powershell -ExecutionPolicy Bypass -File tools/vm/virtualbox/Revert-Vm.ps1 -BaseImage <path>
"@
}

# ---------------------------------------------------------------------------
# Power off (contents are discarded either way)
# ---------------------------------------------------------------------------
$state = Get-VmState
if ($state -eq "saved") {
    Write-Host "Discarding saved state..."
    Invoke-VBox -Arguments @("discardstate", $VmName) | Out-Null
}
elseif (-not (Test-VmOff)) {
    Write-Host "Stopping '$VmName'..."
    Invoke-VBox -Arguments @("controlvm", $VmName, "poweroff") | Out-Null

    $deadline = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $deadline -and -not (Test-VmOff)) {
        Start-Sleep -Seconds 2
    }
    if (-not (Test-VmOff)) {
        throw "VM '$VmName' did not power off (state: $(Get-VmState))."
    }
}

# ---------------------------------------------------------------------------
# Reset
# ---------------------------------------------------------------------------
if ($hasSnapshot) {
    Write-Host "Restoring snapshot '$Snapshot'..."
    $result = Invoke-VBox -Arguments @("snapshot", $VmName, "restore", $Snapshot)
    if ($result.ExitCode -ne 0) { throw "Failed to restore snapshot '$Snapshot': $($result.Output)" }
    Write-Host "Snapshot restored. Start with: yarn vm:start" -ForegroundColor Green
    return
}

Write-Host "No snapshot '$Snapshot'; rebuilding '$VmName' from $rebuildImage ..."
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "Setup-Vm.ps1") -BaseImage $rebuildImage
if ($LASTEXITCODE -ne 0) { throw "Rebuild from base image failed." }

Write-Host "VM rebuilt. Start with: yarn vm:start" -ForegroundColor Green
