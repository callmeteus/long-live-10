# Setup-Vm.ps1 (VirtualBox) - Creates and configures the Windows 10 VM
param(
    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" }),

    [string]$VmName = "ll10-22h2",

    [string]$BaseImage = ""
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found. Install with: scoop install nonportable/virtualbox-np" }

New-Item -ItemType Directory -Force (Join-Path $VmDir "vm"), (Join-Path $VmDir "logs") | Out-Null

$vmPath = Join-Path $VmDir "vm"

# ---------------------------------------------------------------------------
# Pre-built image path
# ---------------------------------------------------------------------------
if ($BaseImage -ne "") {
    if (-not (Test-Path $BaseImage)) { throw "Base image not found: $BaseImage" }

    Write-Host "Importing pre-built image: $BaseImage"

    # Remove existing VM if present (ignore errors)
    try { & $vb unregistervm $VmName --delete 2>$null } catch { }
$LASTEXITCODE = 0
    $LASTEXITCODE = 0

    $ext = [System.IO.Path]::GetExtension($BaseImage).ToLowerInvariant()
    if ($ext -eq ".ova" -or $ext -eq ".ovf") {
        & $vb import $BaseImage --vsys 0 --vmname $VmName
    } else {
        # VHD, VHDX, qcow2, raw -> create VM and attach disk
        & $vb createvm --name $VmName --ostype "Windows10_64" --register --basefolder $vmPath
        & $vb modifyvm $VmName --memory 4096 --cpus 2 --vram 128 --nic1 nat --nictype1 82540EM
        & $vb modifyvm $VmName --natpf1 "winrm,tcp,,55985,,5985" --natpf1 "rdp,tcp,,53389,,3389"

        # Convert image to VDI if needed
        $vdiPath = Join-Path $vmPath "$VmName.vdi"
        if ($ext -eq ".vdi") {
            Copy-Item $BaseImage $vdiPath -Force
        } else {
            & $vb clonemedium disk $BaseImage $vdiPath --format VDI
        }

        # Attach disk
        & $vb storagectl $VmName --name "SATA" --add sata --controller IntelAhci
        & $vb storageattach $VmName --storagectl "SATA" --port 0 --device 0 --type hdd --medium $vdiPath
    }

    # Enable RDP (VRDP)
    & $vb modifyvm $VmName --vrde on --vrdeport 53389 --vrdeauthtype null

    @"
VM: $VmName
Source: pre-built image ($BaseImage)
Username: ll10
Password: Ll10Test!2026
WinRM: http://127.0.0.1:55985
RDP: 127.0.0.1:53389
VBoxManage: $vb
"@ | Set-Content -Path (Join-Path $vmPath "credentials.txt") -Encoding UTF8

    Write-Host "Import complete. VM: $VmName"
    Write-Host "Next: yarn vm:start ; yarn vm:smoke"
    return
}

# ---------------------------------------------------------------------------
# Standard path: create VM from ISO with unattended install
# ---------------------------------------------------------------------------
$iso = Join-Path $VmDir "iso\Win10_22H2_English_x64.iso"
if (-not (Test-Path $iso)) {
    Write-Host "ISO not found at $iso. Download Windows 10 22H2 ISO first."
    Write-Host "Or use: yarn vm:import -- -Image <path-to-prebuilt.vhd>"
    exit 1
}

# Remove existing VM
try { & $vb unregistervm $VmName --delete 2>$null } catch { }
$LASTEXITCODE = 0

# Create VM
Write-Host "Creating VM: $VmName"
& $vb createvm --name $VmName --ostype "Windows10_64" --register --basefolder $vmPath
& $vb modifyvm $VmName --memory 4096 --cpus 2 --vram 128 --nic1 nat --nictype1 82540EM
& $vb modifyvm $VmName --natpf1 "winrm,tcp,,55985,,5985" --natpf1 "rdp,tcp,,53389,,3389"

# Storage
& $vb storagectl $VmName --name "SATA" --add sata --controller IntelAhci
$diskPath = Join-Path $vmPath "$VmName.vdi"
& $vb createmedium disk --filename $diskPath --size 51200 --format VDI
& $vb storageattach $VmName --storagectl "SATA" --port 0 --device 0 --type hdd --medium $diskPath

# Attach ISO
& $vb storageattach $VmName --storagectl "SATA" --port 1 --device 0 --type dvddrive --medium $iso

# Unattended install
$unattendIso = Join-Path $VmDir "unattend\autounattend.iso"
if (Test-Path $unattendIso) {
    & $vb storageattach $VmName --storagectl "SATA" --port 2 --device 0 --type dvddrive --medium $unattendIso
}

# Enable VRDP
& $vb modifyvm $VmName --vrde on --vrdeport 53389 --vrdeauthtype null

@"
VM: $VmName
Username: ll10
Password: Ll10Test!2026
WinRM: http://127.0.0.1:55985
RDP: 127.0.0.1:53389
VBoxManage: $vb
"@ | Set-Content -Path (Join-Path $vmPath "credentials.txt") -Encoding UTF8

Write-Host "VM created. Start with: yarn vm:start"