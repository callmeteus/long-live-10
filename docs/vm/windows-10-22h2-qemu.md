# Windows 10 22H2 QEMU VM (host without Hyper-V management)

This document describes a disposable Windows 10 22H2 x64 VM that runs under
QEMU with WHPX acceleration. It is intended for hosts where the Hyper-V
management cmdlets are unavailable (for example, Windows 10 Home).

## Host requirements

- Windows 10 with WHPX available (`qemu-system-x86_64 -accel whpx` starts)
- QEMU 11 or newer (installed via Scoop: `scoop install qemu`)
- At least 6 GB of free RAM and 20 GB of disk space
- `wimlib` and `pycdlib` for ISO inspection and unattended media (optional)

## Guest image

- Windows 10 Pro 22H2 (build 19045.2965), English (United States)
- Source: official Microsoft Windows 10 22H2 multi-edition ISO
- Disk: 50 GB qcow2 at `C:\Users\Gio\VMs\long-live-10\vm\ll10-22h2.qcow2`
- Baseline snapshot tag: `baseline`

## Reproducible commands

From the repository root:

```powershell
yarn vm:setup
yarn vm:start
yarn vm:revert
yarn vm:smoke
```

`vm:setup` downloads the official ISO, builds the unattended install media, creates
the disk, and starts the installation. The other commands assume that setup has
completed.

## Start the VM

```powershell
$q = "C:\Users\Gio\scoop\apps\qemu\current\qemu-system-x86_64.exe"
$dir = "C:\Users\Gio\VMs\long-live-10"
& $q `
  -name ll10-22h2 `
  -machine pc,accel=whpx `
  -cpu host `
  -smp 1 `
  -m 4096 `
  -rtc base=localtime,clock=host `
  -drive file="$dir\vm\ll10-22h2.qcow2",format=qcow2,if=ide,index=0 `
  -netdev user,id=net0,hostfwd=tcp:127.0.0.1:55985-:5985,hostfwd=tcp:127.0.0.1:53389-:3389 `
  -device e1000,netdev=net0 `
  -vga std `
  -display none `
  -vnc 127.0.0.1:1 `
  -qmp tcp:127.0.0.1:4444,server=on,wait=off `
  -boot order=c,menu=on
```

## Revert to the baseline

```powershell
# Stop the VM first (QMP quit or close the process)
& "C:\Users\Gio\scoop\apps\qemu\current\qemu-img.exe" snapshot -a baseline "C:\Users\Gio\VMs\long-live-10\vm\ll10-22h2.qcow2"
```

## Access

- WinRM: `http://127.0.0.1:55985`
- RDP: `127.0.0.1:53389`
- QMP: `127.0.0.1:4444`
- VNC: `127.0.0.1:5901`

## Manual guest setup

The unattended install creates the `ll10` administrator account, but WinRM may still
be blocked by the guest firewall on some images. If `yarn vm:smoke` cannot connect,
open an elevated PowerShell session in the guest through RDP or the QEMU console and run:

```powershell
netsh advfirewall set allprofiles state off
Enable-PSRemoting -Force -SkipNetworkProfileCheck
Set-Item -Path WSMan:\localhost\Service\Auth\Basic -Value $true
Set-Item -Path WSMan:\localhost\Service\AllowUnencrypted -Value $true
Set-Service WinRM -StartupType Automatic
Start-Service WinRM
```

Then rerun `yarn vm:smoke`.

Credentials are stored in `C:\Users\Gio\VMs\long-live-10\vm\credentials.txt` and are
not committed to this repository.

## Notes

- This VM is disposable. Revert to the baseline snapshot after every test run.
- WinRM configuration and guest tooling are part of the baseline image.
- Hyper-V remains the preferred backend for CI; this QEMU path is for local
  validation on hosts without Hyper-V management.
