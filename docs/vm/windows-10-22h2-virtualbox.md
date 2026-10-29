# Windows 10 22H2 VirtualBox VM

Disposable Windows 10 22H2 x64 test machine used by the patch/mitigation work.
The VM is created, provisioned and reverted without any manual step: no RDP
session, no keyboard automation and no operator login.

## Why provisioning is done offline

An earlier harness configured the guest from inside a running session. That
required an interactive login, broke whenever the guest was restored to a
snapshot, and was not reproducible across machines.

`Prepare-Guest.ps1` instead mounts the guest disk (VHD) on the host and writes
the configuration into the offline registry hives. Booting the VM afterwards
produces a machine that is already logged in and reachable over WinRM.

Provisioning is idempotent: running it again on the same disk produces the same
state.

## Host requirements

- Windows 10/11 x64 with VirtualBox 7.2+
  (`scoop install nonportable/virtualbox-np`)
- Administrator rights **only for provisioning**: mounting a VHD and loading
  registry hives requires elevation. `Setup-Vm.ps1` and
  `Finalize-Vm.ps1` request it automatically.
- ~20 GB of free disk space for the guest disk

## Commands

All commands run from the repository root.

```powershell
# 1. Create (or re-create) the VM and provision the guest disk.
#    Accepts .vhd, .vdi, .ova/.ovf or an ISO-based unattended install.
yarn vm:setup -- -BaseImage "D:\images\Win10_22H2.vhd"

# 2. Re-apply provisioning only (disk must be attached and the VM powered off).
yarn vm:prepare

# 3. Boot headless and wait until a real WinRM login succeeds
#    (default budget: 900 s; override with -TimeoutSeconds or LL10_VM_TIMEOUT).
yarn vm:start

# 4. Run a harmless command inside the guest.
yarn vm:smoke

# 5. Shut down, re-provision and record the 'baseline' snapshot.
yarn vm:finalize

# 6. Discard all guest changes and return to the baseline.
#    If the snapshot is missing, this rebuilds from base-image.vhd instead.
yarn vm:revert
```

`yarn vm:start` performs a real WS-Management **login** (PSSession plus a
remote command), not a TCP connect. VirtualBox NAT accepts TCP connections
while the guest is still booting, so a TCP probe reports false positives, and
`Test-WSMan` alone can pass before authentication is ready.

The VM starts **headless**; no console window or RDP session is opened. Pass
`-Gui` to `Start-Vm.ps1` only when a human wants to watch the console.

Expected timing on the standard 4 GB / 2 vCPU guest: cold boot to a working
WinRM login is ~190-330 s depending on host load. The 900 s default exists
because the very first boot after a fresh import can be slower.

`vm:setup` records a `baseline` snapshot after provisioning. That makes
`vm:revert` an instant, exact reset. When the snapshot is absent, `vm:revert`
rebuilds the VM from `base-image.vhd` next to the guest disk.

## What provisioning guarantees

| Setting | Value | Effect |
| --- | --- | --- |
| `Services\WinRM` → `Start` | `2` (automatic) | WinRM starts at boot, no login needed |
| `Control\Lsa` → `LocalAccountTokenFilterPolicy` | `1` | Local admin account can authenticate remotely |
| Firewall `DomainProfile` / `StandardProfile` / `PublicProfile` | `EnableFirewall = 0` | Forwarded ports are never filtered |
| `Winlogon` → `AutoAdminLogon` / `DefaultUserName` / `DefaultPassword` / `ForceAutoLogon` | enabled | Console session logs in automatically |
| `Winlogon` → `AutoLogonCount` | deleted | Auto-logon does not expire |
| Startup folder / stale `RunOnce` entries | removed | No console windows flash at boot |

The provisioning also writes `C:\ll10\provision.txt` inside the guest as a
diagnostic marker (timestamp, account, expected state).

## Verifying a boot without any interaction

```powershell
yarn vm:revert
yarn vm:start        # waits for WinRM
yarn vm:smoke        # prints "ll10-smoke" and the OS caption
```

To confirm auto-logon and the absence of popup consoles, run inside the guest:

```powershell
query session
# >services  0  Disc
#  console   ll10  1  Active

(Get-CimInstance Win32_ComputerSystem).UserName   # LL10-TEST\ll10
(Get-ChildItem 'C:\ProgramData\Microsoft\Windows\Start Menu\Programs\StartUp').Count  # 0
```

## Credentials and ports

| Item | Value |
| --- | --- |
| Account | `ll10` / `Ll10Test!2026` (local administrator) |
| WinRM | `http://127.0.0.1:55985` (NAT → guest 5985) |
| RDP | `127.0.0.1:53389` (NAT → guest 3389, only if needed) |
| VM name | `ll10-22h2` |

## Troubleshooting

**`WinRM login did not succeed within ...`**
The guest did not finish booting, or it was not provisioned. Two usual causes:

- Not enough time. Cold boot is ~190-330 s; retry with a larger budget:
  `yarn vm:start -- -TimeoutSeconds 1200`.
- The guest disk lost its provisioning. Run `yarn vm:prepare` (it powers the VM
  off first), then `yarn vm:start`.

**`Guest disk is not a VHD`**
Offline provisioning needs a mountable format. Recreate the VM with
`yarn vm:setup` (it stores the disk as VHD) or convert the disk manually with
`VBoxManage clonemedium disk <src> <dst.vhd> --format VHD`.

**`Failed to load hive` / `Access denied`**
The shell is not elevated. `Prepare-Guest.ps1` self-elevates; approve the UAC
prompt if UAC is enabled.

**Provisioning changes seem to disappear after a boot**
The registry hive was dirty (the VM was killed instead of shut down) and
Windows replayed its transaction logs. `Prepare-Guest.ps1` always requests a
graceful ACPI shutdown before mounting; let it finish instead of powering the
VM off by force.

**Guest files look missing when the disk is inspected offline**
Once `Setup-Vm.ps1` takes the `baseline` snapshot, VirtualBox writes to a
differencing disk under `vm\<name>\Snapshots\`. The file named in the setup
(`vm\<name>.vhd`) is then only the parent image, so mounting it shows the guest
as it was before the snapshot: recent files, patches and logs are "gone" while
the running VM still has them. Mount the disk the VM is actually attached to:

```powershell
$disk = (& "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe" showvminfo ll10-22h2 --machinereadable |
  Select-String '^"SATA-0-0"') -replace '.*="(.*)"', '$1'
Start-Process powershell -Verb RunAs -ArgumentList '-NoProfile','-Command',"Mount-DiskImage -ImagePath '$disk'"
```

**The VM looks "hung": black screen, frozen clock, no WinRM**
Check whether the guest is actually spinning or merely slow:

```powershell
Get-Process VBoxHeadless | Select-Object Id, CPU, @{n='WS_MB';e={[math]::Round($_.WorkingSet64/1MB)}}
```

A high CPU number that keeps growing means Windows is busy (usually chkdsk after
an unclean shutdown, or first-boot work); wait it out. A frozen clock with no
CPU growth means a genuine hang; reset with `yarn vm:revert` (snapshot) or
`yarn vm:setup` (rebuild).
