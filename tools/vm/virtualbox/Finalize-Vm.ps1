# Finalize-Vm.ps1 (VirtualBox) - Creates baseline snapshot after OOBE completion
param(
    [string]$VmName = "ll10-22h2",

    [string]$Snapshot = "baseline"
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found." }

# Check VM is running
$running = & $vb list runningvms 2>&1
if ($running -notmatch $VmName) {
    throw "VM '$VmName' is not running. Start it first with yarn vm:start."
}

# Wait for WinRM to be ready
Write-Host "Waiting for WinRM..."
$deadline = (Get-Date).AddMinutes(10)
$ready = $false
while ((Get-Date) -lt $deadline) {
    try {
        $req = [System.Net.HttpWebRequest]::Create('http://127.0.0.1:55985/wsman')
        $req.Timeout = 5000
        $req.GetResponse() | Out-Null
        $ready = $true
        break
    } catch {
        if ($_.Exception.Response) { $ready = $true; break }
        Start-Sleep -Seconds 5
    }
}

if (-not $ready) {
    Write-Host "Warning: WinRM not responding. Creating snapshot anyway."
}

# Delete existing snapshot if present
& $vb snapshot $VmName delete $Snapshot 2>$null

# Create snapshot
Write-Host "Creating snapshot '$Snapshot'..."
& $vb snapshot $VmName take $Snapshot
if ($LASTEXITCODE -ne 0) { throw "Failed to create snapshot." }

Write-Host "Snapshot '$Snapshot' created successfully."
Write-Host "Use 'yarn vm:revert' to restore this state."