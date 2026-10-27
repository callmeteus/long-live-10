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

# 3. Boot and wait until WinRM actually answers (default timeout: 300 s).
yarn vm:start

# 4. Run a harmless command inside the guest.
yarn vm:smoke

# 5. Shut down, re-provision and record the 'baseline' snapshot.
yarn vm:finalize

# 6. Discard all guest changes and return to the baseline.
yarn vm:revert
```

`yarn vm:start` performs a real WS-Management handshake (`Test-WSMan`) instead
of a TCP connect: VirtualBox NAT accepts TCP connections even while the guest
is still booting, so a TCP probe reports false positives.

Expected timing on a 4 vCPU / 4 GB guest: first WinRM answer ~2.5-3 minutes
after `vm:start`; `vm:smoke` then takes a few seconds.

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

**`WinRM did not answer ... within 300 s`**
The guest did not finish booting, or it was not provisioned. Run
`yarn vm:prepare` (the VM is powered off automatically), then `yarn vm:start`.

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
