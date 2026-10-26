# Start-Vm.ps1 (VirtualBox) - Starts the VM
param(
    [string]$VmName = "ll10-22h2",

    [switch]$Headless
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found." }

# Check if VM exists
$vmList = & $vb list vms 2>&1
if ($vmList -notmatch $VmName) {
    throw "VM '$VmName' not found. Run yarn vm:setup first."
}

# Check if already running
$running = & $vb list runningvms 2>&1
if ($running -match $VmName) {
    Write-Host "VM '$VmName' is already running."
    return
}

Write-Host "Starting VM: $VmName"
if ($Headless) {
    & $vb startvm $VmName --type headless
} else {
    & $vb startvm $VmName --type gui
}
Write-Host "VM started."
Write-Host "WinRM: http://127.0.0.1:55985"
Write-Host "RDP: 127.0.0.1:53389"