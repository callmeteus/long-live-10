# Start-Vm.ps1 (VirtualBox) - Starts the VM and waits for WinRM
param(
    [string]$VmName = "ll10-22h2",

    [switch]$Headless,

    # Do not return until WinRM answers, so callers can run commands right away.
    [bool]$WaitForWinRm = $true,

    [int]$TimeoutSeconds = 300,

    [int]$WinRmPort = 55985
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found." }

# Check if VM exists
$vmList = & $vb list vms 2>&1
if ($vmList -notmatch $VmName) {
    throw "VM '$VmName' not found. Run yarn vm:setup first."
}

function Test-WinRm {
    param([int]$Port)
    # A TCP connect is not enough: VirtualBox NAT accepts the connection even
    # while the guest is still booting. Test-WSMan performs a real
    # WS-Management handshake, so it only succeeds once the listener is up.
    try {
        $result = Test-WSMan -ComputerName "127.0.0.1" -Port $Port -ErrorAction Stop
        return [bool]$result
    } catch {
        return $false
    }
}

# Check if already running
$running = & $vb list runningvms 2>&1
if ($running -match $VmName) {
    Write-Host "VM '$VmName' is already running."
} else {
    Write-Host "Starting VM: $VmName"
    if ($Headless) {
        & $vb startvm $VmName --type headless
    } else {
        & $vb startvm $VmName --type gui
    }
    if ($LASTEXITCODE -ne 0) { throw "Failed to start VM '$VmName'." }
    Write-Host "VM started."
}

Write-Host "WinRM: http://127.0.0.1:$WinRmPort"
Write-Host "RDP:   127.0.0.1:53389"

if (-not $WaitForWinRm) { return }

Write-Host "Waiting for WinRM (up to $TimeoutSeconds s)..."
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
while ((Get-Date) -lt $deadline) {
    if (Test-WinRm -Port $WinRmPort) {
        # TCP is up; give the WS-Management listener a moment to accept auth.
        Start-Sleep -Seconds 3
        Write-Host "WinRM is ready." -ForegroundColor Green
        return
    }
    Start-Sleep -Seconds 5
}

throw "WinRM did not answer on 127.0.0.1:$WinRmPort within $TimeoutSeconds s. Run 'yarn vm:prepare' to provision the guest disk."
