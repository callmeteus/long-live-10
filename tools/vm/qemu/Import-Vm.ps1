# Imports a pre-built disk image (VHD, VHDX, qcow2, raw) into the VM directory,
# skipping the unattended Windows installation entirely.
#
# Usage:
#   yarn vm:import -- -Image "C:\images\Win10_22H2.qcow2"
#   powershell -ExecutionPolicy Bypass -File tools/vm/qemu/Import-Vm.ps1 -Image "C:\images\Win10_22H2.vhdx"
#
# After import: yarn vm:start ; yarn vm:smoke

param(
    [Parameter(Mandatory = $true)]
    [string]$Image,

    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" })
)

$ErrorActionPreference = "Stop"

$toolsDir = $PSScriptRoot
$qemuImg = if ($env:LL10_QEMU_IMG) { $env:LL10_QEMU_IMG } else { Join-Path $env:USERPROFILE "scoop\apps\qemu\current\qemu-img.exe" }

if (-not (Test-Path $qemuImg)) { throw "qemu-img not found at $qemuImg." }
if (-not (Test-Path $Image)) { throw "Image not found: $Image" }

New-Item -ItemType Directory -Force (Join-Path $VmDir "vm"), (Join-Path $VmDir "logs") | Out-Null

$disk = Join-Path $VmDir "vm\ll10-22h2.qcow2"

Write-Host "Importing pre-built image: $Image"
Remove-Item $disk -Force -ErrorAction SilentlyContinue

$ext = [System.IO.Path]::GetExtension($Image).ToLowerInvariant()
if ($ext -eq ".qcow2") {
    Copy-Item $Image $disk -Force
} else {
    & $qemuImg convert -f auto -O qcow2 $Image $disk
    if ($LASTEXITCODE -ne 0) { throw "qemu-img convert failed (exit $LASTEXITCODE)." }
}

& $qemuImg check $disk | Out-Null

@"
VM: ll10-22h2
Source: pre-built image ($Image)
Username: ll10
Password: Ll10Test!2026
WinRM: http://127.0.0.1:55985
RDP: 127.0.0.1:53389
"@ | Set-Content -Path (Join-Path $VmDir "vm\credentials.txt") -Encoding UTF8

Write-Host "Import complete. Disk: $disk"
Write-Host "Next: yarn vm:start ; yarn vm:smoke"