# vm-daemon

Stable command bridge for the disposable Windows 10 guest (`ll10-22h2`).

## Why

WinRM dies a few minutes after boot in this image (see
`docs/patches/field-notes.md`), and it runs in session 0, which cannot inject
keys into the interactive desktop. Task Scheduler with
`-LogonType Interactive` fails with `0x800710E0`, and
`WScript.Shell.AppActivate` from WinRM returns `False`.

This package replaces those approaches with a long-lived PowerShell daemon
that runs **inside the guest, in the interactive console session (session 1)**,
started from the user's Startup folder. The host talks to it over a VirtualBox
NAT port forward, with no WinRM dependency for driving the UI or reading logs.

## Layout

```
packages/vm-daemon/
  README.md
  daemon/ll10-ui-daemon.ps1      # runs inside the guest (session 1)
  client/vmctl.ps1               # host controller (TCP + status)
  client.ps1                     # shared-folder client (alternative bridge)
  scripts/install-in-guest.ps1   # one-time install via WinRM
```

## Setup (one time)

1. Add a NAT rule that maps host `127.0.0.1:55990` to guest `55990`:

   ```powershell
   VBoxManage modifyvm ll10-22h2 --nat-pf1 "ui,tcp,127.0.0.1,55990,,55990"
   ```

2. Install the daemon in the guest. This is the last step that needs WinRM:

   ```powershell
   # from a WinRM session into the guest
   .\scripts\install-in-guest.ps1
   ```

3. Reboot the guest once. The daemon starts from the Startup shortcut and
   listens on TCP 55990.

## Use (no WinRM)

From the host:

```powershell
# check the daemon
.\client\vmctl.ps1 -Command PING

# drive the Start menu search
.\client\vmctl.ps1 -Command FOCUS
.\client\vmctl.ps1 -Command WIN
.\client\vmctl.ps1 -Command "TEXT notepad"
.\client\vmctl.ps1 -Command ENTER
.\client\vmctl.ps1 -Command PROCS

# run 200 search iterations and capture state on failure
.\client\vmctl.ps1 -Command "LONGRUN 200"

# read the reproduction log (works while the batch runs)
.\client\vmctl.ps1 -Command GETLOG
```

## Commands

| Command | Effect |
|---|---|
| `PING` | `PONG` plus foreground window info |
| `FOCUS` | focus taskbar / Program Manager |
| `WIN` | press Windows key (opens Start) |
| `TEXT <value>` | type text via SendKeys |
| `ENTER`, `ESC`, `KEY <vk>` | send keys |
| `SNAP` | dump top-level windows via UI Automation |
| `PROCS` | list Search/StartMenu/explorer/notepad processes and CPU |
| `LONGRUN [n]` | run n search iterations; log `RESULT FAIL` with a snapshot and event log on failure |
| `GETLOG` | return `C:\ll10\repro-long.log` |
| `STATUS` | daemon status |

## Logs

- `C:\ll10\ui-daemon.log` - daemon activity
- `C:\ll10\repro-long.log` - reproduction cycles

## Known limitations

- Host firewall may abort loopback TCP connections to the NAT port. If
  `vmctl.ps1` reports "connection aborted by software on the host", add an
  inbound allow rule for `127.0.0.1:55990` or use the shared-folder bridge
  (`client.ps1`) instead.
- `LONGRUN` runs inside the daemon, so a WinRM death cannot interrupt it. Read
  results with `GETLOG` or the shared folder.
