# Windows 10 22H2 Base Image

Preparation guide for the disposable Hyper-V base image used by `vm-harness` and the release validation pipeline.

## Overview

The base image is a clean Windows 10 22H2 x64 installation with:

- Generation 2 virtual machine
- No network adapter
- No installation media attached after setup
- A checkpoint named `baseline`
- A dedicated local administrator account used only by PowerShell Direct

The harness restores the `baseline` checkpoint before and after every test. Nothing in this guide modifies the host operating system beyond creating the VM, the isolated switch, and the credential file.

## Prerequisites

| Requirement | Notes |
|-------------|-------|
| Windows host with Hyper-V | Windows 10/11 Pro, Enterprise, or a Server edition |
| Administrator PowerShell | Required for Hyper-V cmdlets |
| Free disk space | At least 80 GB for the VHDX and checkpoints |
| Windows 10 22H2 x64 ISO | Obtained from a legitimate source |
| VM name | `ll10-22h2-base` in the examples |

Enable Hyper-V from an elevated PowerShell session only when it is not already enabled:

```powershell
Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All
```

The host restarts after this command. Skip it when Hyper-V is already available.

## Legitimate ISO sources

Use one of these sources and record the exact file name and SHA-256 hash:

| Source | Use |
|--------|-----|
| [Microsoft software download](https://www.microsoft.com/software-download/windows10) | Retail Windows 10 ISO when the page still serves 22H2 |
| Volume Licensing Service Center | Business editions for licensed organizations |
| Visual Studio subscriber downloads | Development and test media |

Microsoft ended mainstream Windows 10 downloads after support ended on October 14, 2025. The public download page may redirect to Windows 11. In that case, use the Volume Licensing Service Center or Visual Studio subscriber downloads for archived 22H2 media.

Do not use modified ISOs, third-party mirrors, or media with preinstalled software. A base image is evidence only when its source and hash are documented.

## Verify the ISO

```powershell
$isoPath = "D:\iso\Win10_22H2_English_x64.iso"
Get-FileHash -Algorithm SHA256 -Path $isoPath
Get-Item $isoPath | Select-Object Name, Length, LastWriteTime
```

Record the hash in the VM inventory next to the VM name, build number, and checkpoint name. Compare it with the hash published by the source when one is available.

## Create the isolated switch

An internal switch has no uplink to the host network or the internet:

```powershell
New-VMSwitch -Name "LL10-Isolated" -SwitchType Internal
```

Do not use the `Default Switch` for the baseline. It provides NAT access to the host network, so tests would not be isolated.

## Create the virtual machine

```powershell
$vmName = "ll10-22h2-base"
$root = "D:\ll10"
$vhdPath = "$root\base\Windows10-22H2.vhdx"
$isoPath = "D:\iso\Win10_22H2_English_x64.iso"

New-Item -ItemType Directory -Force -Path "$root\base" | Out-Null
New-Item -ItemType Directory -Force -Path "$root\secrets" | Out-Null

New-VM -Name $vmName `
    -MemoryStartupBytes 4GB `
    -Generation 2 `
    -NewVHDPath $vhdPath `
    -NewVHDSizeBytes 80GB `
    -SwitchName "LL10-Isolated"

Set-VM -Name $vmName `
    -ProcessorCount 2 `
    -DynamicMemory `
    -MemoryMinimumBytes 2GB `
    -MemoryMaximumBytes 8GB

Add-VMDvdDrive -VMName $vmName -Path $isoPath

$dvd = Get-VMDvdDrive -VMName $vmName
Set-VMFirmware -VMName $vmName -FirstBootDevice $dvd

Start-VM -Name $vmName
vmconnect.exe localhost $vmName
```

## Install Windows 10 22H2

1. Start the VM console with `vmconnect.exe`.
2. Select the x64 Windows 10 22H2 edition. Pro or Enterprise is recommended for integration testing.
3. Choose a custom installation and install to the unallocated virtual disk.
4. Create a local account named `ll10`. Do not link a Microsoft account.
5. Decline optional services, advertising identifiers, and cloud sync prompts.
6. Install all Hyper-V integration components that ship with Windows 10. No separate ISO is required for a supported guest.
7. Restart the guest until Windows Update reports no pending critical updates for the chosen baseline.

Record the installed build inside the guest:

```powershell
Get-ComputerInfo | Select-Object WindowsProductName, WindowsVersion, OsBuildNumber
```

Windows 10 22H2 reports build `19045`. Store the exact build number with the base image inventory.

## Prepare the guest for testing

Run these commands inside the guest:

```powershell
# Keep the guest awake during long validation runs.
powercfg /change standby-timeout-ac 0
powercfg /change monitor-timeout-ac 0

# Confirm PowerShell Direct support and the guest identity.
$env:COMPUTERNAME
Get-LocalUser -Name "ll10" | Select-Object Name, Enabled
```

PowerShell Direct uses the VMBus, so it still works after the network adapter is removed. The guest account must be a member of the local Administrators group.

## Remove networking and installation media

Run these commands on the host after the guest shuts down cleanly:

```powershell
Stop-VM -Name $vmName

Remove-VMNetworkAdapter -VMName $vmName
Remove-VMDvdDrive -VMName $vmName
Set-VMFirmware -VMName $vmName -FirstBootDevice (Get-VMHardDiskDrive -VMName $vmName)
```

The baseline must have no network adapter and no ISO attached. If a test needs a network, attach `LL10-Isolated` explicitly and restore the checkpoint afterwards.

## Create the baseline checkpoint

With the VM stopped:

```powershell
Checkpoint-VM -Name $vmName -SnapshotName "baseline"
Get-VMSnapshot -VMName $vmName | Select-Object Name, CreationTime, ParentSnapshotName
```

The harness expects the checkpoint name to match `LL10_VM_CHECKPOINT` exactly.

## Create the guest credential file

PowerShell Direct authenticates with guest credentials. Export them to a DPAPI-protected file instead of storing a password in scripts or environment variables:

```powershell
$credential = Get-Credential -UserName "ll10"
$credential | Export-Clixml "D:\ll10\secrets\ll10-guest.xml"

icacls "D:\ll10\secrets" /inheritance:r /grant:r "$env:USERDOMAIN\$env:USERNAME:(OI)(CI)F"
```

DPAPI binds the file to the current host and Windows user. Re-export it when the harness runs under a different account or on a different host. The password never appears in the generated PowerShell command line.

## Verify against Hyper-V Manager

Open `virtmgmt.msc` and confirm each item before marking the image ready:

| Screen | Expected state |
|--------|----------------|
| VM summary | Generation 2, state Off after checkpoint |
| Settings, Hardware | 2 processors, 4 GB startup memory, one VHDX, no DVD, no network adapter |
| Settings, Management | Checkpoints enabled, `baseline` present |
| Settings, Firmware | Secure Boot on, disk first in boot order |
| Integration Services | Heartbeat and data exchange available when the VM runs |
| Guest | Windows 10 22H2 build 19045, local account `ll10` |

Run the same checks from PowerShell:

```powershell
Get-VM -Name $vmName | Select-Object Name, Generation, State, ProcessorCount, MemoryStartup
Get-VMFirmware -VMName $vmName | Select-Object SecureBoot, SecureBootTemplate
Get-VMNetworkAdapter -VMName $vmName
Get-VMDvdDrive -VMName $vmName
```

`Get-VMNetworkAdapter` must return no adapters once the baseline is finished.

## Run the smoke test

```powershell
$env:LL10_VM_NAME = "ll10-22h2-base"
$env:LL10_VM_VHD = "D:\ll10\base\Windows10-22H2.vhdx"
$env:LL10_VM_CHECKPOINT = "baseline"
$env:LL10_VM_CREDENTIAL = "D:\ll10\secrets\ll10-guest.xml"

cargo test -p vm-harness --test smoke_test -- --ignored
```

The smoke test restores the baseline, runs a harmless guest command, captures Hyper-V worker events, and restores the baseline again.

## Rebuild and refresh

Recreate the image when the guest no longer matches the documented build, the checkpoint is corrupted, or the ISO source changes. Delete the old VM only after the replacement passes the smoke test:

```powershell
Remove-VM -Name $vmName -Force
Remove-Item -LiteralPath $vhdPath -Force
```

Never run these commands against a VM that still contains evidence needed by an open investigation.
