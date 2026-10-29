# Setup-Vm.ps1 (VirtualBox) - Creates the Windows 10 VM and provisions the guest offline.
#
# The guest disk is always stored as VHD so that Prepare-Guest.ps1 can mount it on
# the host and apply the "ready to use" configuration (WinRM auto-start, firewall,
# auto-logon) without any RDP session or keyboard automation.
param(
    [string]$VmDir = $(if ($env:LL10_VM_DIR) { $env:LL10_VM_DIR } else { Join-Path $env:USERPROFILE "VMs\long-live-10" }),

    [string]$VmName = "ll10-22h2",

    [string]$BaseImage = "",

    [switch]$SkipProvision
)

$ErrorActionPreference = "Stop"

$vb = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
if (-not (Test-Path $vb)) { throw "VirtualBox not found. Install with: scoop install nonportable/virtualbox-np" }

New-Item -ItemType Directory -Force (Join-Path $VmDir "vm"), (Join-Path $VmDir "logs") | Out-Null

$vmPath = Join-Path $VmDir "vm"
$diskPath = Join-Path $vmPath "$VmName.vhd"

function Get-AttachedDisk {
    param([string]$Name)
    $info = & $vb showvminfo $Name --machinereadable 2>&1
    $line = $info | Select-String '^"SATA-0-0"=' | Select-Object -First 1
    if (-not $line) { return $null }
    return ($line.Line -replace '^"SATA-0-0"="', '' -replace '"$', '')
}

function Convert-VmDiskToVhd {
    param([string]$Name, [string]$Target)

    $current = Get-AttachedDisk -Name $Name
    if (-not $current) { throw "VM '$Name' has no disk on SATA port 0." }
    if ([System.IO.Path]::GetExtension($current).ToLowerInvariant() -eq ".vhd") {
        if ($current -ne $Target) {
            Copy-Item $current $Target -Force
            & $vb storageattach $Name --storagectl "SATA" --port 0 --device 0 --medium none | Out-Null
            & $vb storageattach $Name --storagectl "SATA" --port 0 --device 0 --type hdd --medium $Target | Out-Null
        }
        return
    }

    Write-Host "Converting guest disk to VHD (required for offline provisioning)..."
    if (Test-Path $Target) { Remove-Item $Target -Force }
    & $vb clonemedium disk $current $Target --format VHD
    if ($LASTEXITCODE -ne 0) { throw "Failed to convert '$current' to VHD." }

    & $vb storageattach $Name --storagectl "SATA" --port 0 --device 0 --medium none | Out-Null
    & $vb storageattach $Name --storagectl "SATA" --port 0 --device 0 --type hdd --medium $Target | Out-Null
}

function Invoke-PrepareGuest {
    param([string]$Name, [string]$Disk)
    if ($SkipProvision) { return }
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "Prepare-Guest.ps1") -VmName $Name -DiskPath $Disk
    if ($LASTEXITCODE -ne 0) { throw "Guest provisioning failed." }
}

function Invoke-VBoxQuiet {
    # VBoxManage prints progress to stderr even on success; with
    # $ErrorActionPreference = "Stop" that becomes a terminating error.
    param([string[]]$Arguments)

    $previous = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $vb @Arguments 2>&1 | Out-String
        return [pscustomobject]@{ ExitCode = $LASTEXITCODE; Output = $output }
    }
    finally {
        $ErrorActionPreference = $previous
    }
}

# Records a powered-off "baseline" snapshot so 'yarn vm:revert' has a
# deterministic reset point. Safe to re-run: an existing baseline is replaced.
function Save-BaselineSnapshot {
    param([string]$Name, [string]$SnapshotName = "baseline")

    $existing = Invoke-VBoxQuiet -Arguments @("snapshot", $Name, "list")
    if ($existing.Output -match "Name:\s+$([regex]::Escape($SnapshotName))\b") {
        Write-Host "Replacing snapshot '$SnapshotName'..."
        $null = Invoke-VBoxQuiet -Arguments @("snapshot", $Name, "delete", $SnapshotName)
    }

    Write-Host "Taking snapshot '$SnapshotName'..."
    $result = Invoke-VBoxQuiet -Arguments @("snapshot", $Name, "take", $SnapshotName, "--description", "Provisioned guest, powered off")
    if ($result.ExitCode -ne 0) { throw "Failed to take snapshot '$SnapshotName': $($result.Output)" }
}

# ---------------------------------------------------------------------------
# Pre-built image path
# ---------------------------------------------------------------------------
if ($BaseImage -ne "") {
    if (-not (Test-Path $BaseImage)) { throw "Base image not found: $BaseImage" }

    # If the source image lives at the target path, stage it first: removing the
    # existing VM would delete it before it can be imported.
    $sourceImage = $BaseImage
    $stagedImage = $null
    if ([System.IO.Path]::GetFullPath($BaseImage) -eq [System.IO.Path]::GetFullPath($diskPath)) {
        $stagedImage = Join-Path $env:TEMP ("ll10-base-" + [guid]::NewGuid().ToString("N") + ".vhd")
        Write-Host "Staging source image (it is also the VM target)..."
        Copy-Item $BaseImage $stagedImage -Force
        $sourceImage = $stagedImage
    }

    Write-Host "Importing pre-built image: $BaseImage"

    # Remove existing VM if present (ignore errors)
    try { & $vb unregistervm $VmName --delete 2>&1 | Out-Null } catch { }
    if (Test-Path (Join-Path $vmPath $VmName)) {
        Remove-Item (Join-Path $vmPath $VmName) -Recurse -Force -ErrorAction SilentlyContinue
    }

    $ext = [System.IO.Path]::GetExtension($sourceImage).ToLowerInvariant()

    if ($ext -eq ".ova" -or $ext -eq ".ovf") {
        & $vb import $sourceImage --vsys 0 --vmname $VmName
        if ($LASTEXITCODE -ne 0) { throw "OVF/OVA import failed." }
        & $vb modifyvm $VmName --nic1 nat --nictype1 82540EM
        & $vb modifyvm $VmName --natpf1 "winrm,tcp,,55985,,5985" --natpf1 "rdp,tcp,,53389,,3389"
        & $vb modifyvm $VmName --boot1 disk --boot2 dvd --boot3 none --boot4 none
        Convert-VmDiskToVhd -Name $VmName -Target $diskPath
    } else {
        & $vb createvm --name $VmName --ostype "Windows10_64" --register --basefolder $vmPath
        & $vb modifyvm $VmName --memory 4096 --cpus 4 --vram 128 --nic1 nat --nictype1 82540EM
        & $vb modifyvm $VmName --natpf1 "winrm,tcp,,55985,,5985" --natpf1 "rdp,tcp,,53389,,3389"
        # Disk-first boot: skips floppy/DVD probing and makes boot deterministic.
        & $vb modifyvm $VmName --boot1 disk --boot2 dvd --boot3 none --boot4 none

        if ($ext -eq ".vhd") {
            Copy-Item $sourceImage $diskPath -Force
        } else {
            if (Test-Path $diskPath) { Remove-Item $diskPath -Force }
            & $vb clonemedium disk $sourceImage $diskPath --format VHD
            if ($LASTEXITCODE -ne 0) { throw "Failed to convert '$sourceImage' to VHD." }
        }

        & $vb storagectl $VmName --name "SATA" --add sata --controller IntelAhci
        & $vb storageattach $VmName --storagectl "SATA" --port 0 --device 0 --type hdd --medium $diskPath
    }

    if ($stagedImage -and (Test-Path $stagedImage)) { Remove-Item $stagedImage -Force -ErrorAction SilentlyContinue }

    # Enable RDP (VRDP)
    & $vb modifyvm $VmName --vrde on --vrdeport 53389 --vrdeauthtype null

    Invoke-PrepareGuest -Name $VmName -Disk $diskPath
    Save-BaselineSnapshot -Name $VmName

    @"
VM: $VmName
Source: pre-built image ($BaseImage)
Disk: $diskPath
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
    Write-Host "Or use: yarn vm:setup -- -BaseImage <path-to-prebuilt.vhd>"
    exit 1
}

# Remove existing VM
try { & $vb unregistervm $VmName --delete 2>&1 | Out-Null } catch { }

# Create VM
Write-Host "Creating VM: $VmName"
& $vb createvm --name $VmName --ostype "Windows10_64" --register --basefolder $vmPath
& $vb modifyvm $VmName --memory 4096 --cpus 4 --vram 128 --nic1 nat --nictype1 82540EM
& $vb modifyvm $VmName --natpf1 "winrm,tcp,,55985,,5985" --natpf1 "rdp,tcp,,53389,,3389"

# Storage (VHD so it can be provisioned offline later)
& $vb storagectl $VmName --name "SATA" --add sata --controller IntelAhci
if (Test-Path $diskPath) { Remove-Item $diskPath -Force }
& $vb createmedium disk --filename $diskPath --size 51200 --format VHD
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
Disk: $diskPath
Username: ll10
Password: Ll10Test!2026
WinRM: http://127.0.0.1:55985
RDP: 127.0.0.1:53389
VBoxManage: $vb
"@ | Set-Content -Path (Join-Path $vmPath "credentials.txt") -Encoding UTF8

Write-Host "VM created. Start with: yarn vm:start"
Write-Host "After Windows finishes installing, run: yarn vm:finalize"
