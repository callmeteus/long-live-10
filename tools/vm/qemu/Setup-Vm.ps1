param(
    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" })
)

$ErrorActionPreference = "Stop"

$toolsDir = $PSScriptRoot
$qemu = if ($env:LL10_QEMU) { $env:LL10_QEMU } else { Join-Path $env:USERPROFILE "scoop\apps\qemu\current\qemu-system-x86_64.exe" }
$qemuImg = if ($env:LL10_QEMU_IMG) { $env:LL10_QEMU_IMG } else { Join-Path $env:USERPROFILE "scoop\apps\qemu\current\qemu-img.exe" }

if (-not (Test-Path $qemu)) { throw "QEMU not found at $qemu. Install with 'scoop install qemu'." }
if (-not (Test-Path $qemuImg)) { throw "qemu-img not found at $qemuImg." }

New-Item -ItemType Directory -Force (Join-Path $VmDir "iso"), (Join-Path $VmDir "vm"), (Join-Path $VmDir "unattend"), (Join-Path $VmDir "logs") | Out-Null

$iso = Join-Path $VmDir "iso\Win10_22H2_English_x64.iso"
if (-not (Test-Path $iso)) {
    Write-Host "Downloading official Windows 10 22H2 ISO..."
    $fido = Join-Path $env:TEMP "Fido.ps1"
    Invoke-WebRequest -Uri "https://raw.githubusercontent.com/pbatard/Fido/master/Fido.ps1" -OutFile $fido
    $url = & $fido -Win "Windows 10" -Rel "22H2" -Lang "English" -Arch "x64" -GetUrl
    Invoke-WebRequest -Uri $url -OutFile $iso
}

$unattendIso = Join-Path $VmDir "unattend\autounattend.iso"
if (-not (Test-Path $unattendIso)) {
    Write-Host "Building unattended install ISO..."
    python -c "import pycdlib" 2>$null
    if ($LASTEXITCODE -ne 0) {
        python -m pip install --user pycdlib | Out-Null
    }
    Copy-Item (Join-Path $toolsDir "autounattend.xml") (Join-Path $VmDir "unattend\autounattend.xml") -Force
    Copy-Item (Join-Path $toolsDir "make-unattend-iso.py") (Join-Path $VmDir "unattend\make-unattend-iso.py") -Force
    Push-Location (Join-Path $VmDir "unattend")
    python make-unattend-iso.py
    Pop-Location
}

$disk = Join-Path $VmDir "vm\ll10-22h2.qcow2"
if (-not (Test-Path $disk)) {
    Write-Host "Creating VM disk..."
    & $qemuImg create -f qcow2 $disk 50G
}

@"
VM: ll10-22h2
Username: ll10
Password: Ll10Test!2026
WinRM: http://127.0.0.1:55985
RDP: 127.0.0.1:53389
"@ | Set-Content -Path (Join-Path $VmDir "vm\credentials.txt") -Encoding UTF8

Write-Host "Setup complete. Starting unattended installation..."
& (Join-Path $toolsDir "Start-Vm.ps1") -VmDir $VmDir -BootMode install
