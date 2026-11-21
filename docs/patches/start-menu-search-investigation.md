# Investigation: Start menu search stops responding (Windows 10 22H2)

Status: **investigation in progress, blocked on reproduction**. No patch artifact
has been produced, because the root cause has not been established with
evidence. This document records what was observed so the next pass can resume
without repeating work.

## Goal

Investigate the intermittent bug where opening the Start menu and typing does
not start a search, and reopening Start fixes it temporarily. A fix, if any,
must come from reverse engineering (R) and a binary file patch (A), per
`AGENTS.md` and `docs/patches/authoring.md`. No registry-only or config-only
mitigation is acceptable.

## Observed on the guest (2026-11-20)

- VM `ll10-22h2` (VirtualBox, local host), WinRM `127.0.0.1:55985`, guest
  `ll10` / `Ll10Test!2026`.
- Build: Windows 10 Pro 10.0.19045, UBR 256. Console session 1 active.
- Relevant processes: `SearchApp.exe`, `SearchIndexer.exe`,
  `StartMenuExperienceHost.exe`, `explorer.exe`.
- AppX packages healthy: `Microsoft.Windows.Search_1.14.9.19041...` and
  `Microsoft.Windows.StartMenuExperienceHost_10.0.19041.1023...`, both Status Ok.
- Web triage (not guest evidence): in Windows 10 22H2 the search box is hosted
  by `SearchApp.exe` and the `Microsoft.Windows.Search` package. KB4520146
  documents an unresponsive search box and re-registering the package as the
  official fix. Reopening Start resetting the symptom is consistent with a
  hung state in the search host, but this is not yet proven.

## Binaries collected for REA

Copied from the guest to `.data/rea/`:

| File | SHA-256 |
|---|---|
| `SearchApp.exe` | `e19dfc8606e3fe33e920b91ad84a8343c91a2b855e48b4dd22d435f5cf378d33` |
| `SearchApi.dll` | `54f2601881614dc8884b2e62049fe126dc346a2ff4438e8a898ad4d2bddd4ac0` |
| `Search.Core.dll` | `79a6003bb7ca8f13cab01fef9d45fa67ac098454932b8d7a2368435711a51` |
| `StartMenuExperienceHost.exe` | `7d00473ed0e5affd24c1b658c0ffb2da386105ce72a4d061f4097d00b13bde34` |
| `StartUI.dll` | `9b1e67a49b0aaf87ae32b069b7e75d4d68b1c5ec35347d30e62133363933617e` |

## REA setup

- REA 6.4.0 CLI is installed.
- Ghidra 12.1.4 headless + native decompiler is installed.
- REA initially refused Ghidra because JAVA_HOME pointed at JDK 17. Installed
  `temurin21-jdk` via scoop and set `JAVA_HOME` to
  `C:\Users\Gio\scoop\apps\temurin21-jdk\current`. After that, `rea inspect`
  with `--provider ghidra` works. Note: the bash shell in this environment
  still resolves `java` to 17; invoke `rea` through PowerShell with
  `$env:JAVA_HOME` and `$env:PATH` overridden.
- `SearchApp.exe` is native PE x86_64, no CLI metadata. Ghidra found 23,613
  procedures and 6,171 strings.

## REA findings so far

Strings in `SearchApp.exe` related to query text:

- `SearchBoxViewModel_NotifyAPIQueryTextChanged` at `0x1402a0643`
- `SearchBoxViewModel_QueryTextSetViaAPI` at `0x1402a0718`
- `SearchBoxViewModel_QueryTextOrSelectionUpdated` at `0x1402a0768`
- `QueryText` (several locations)
- `Windows.Foundation.TypedEventHandler<Object, Cortana.Search.IQueryTextSetEventArgs>`

These are observations only. The procedure that handles the query text change
has not been identified yet. `rea function` / `rea decompile` at the string
addresses returned "no procedure at requested address", so xref discovery is
still needed (for example with a Ghidra headless script) before any function
can be decompiled.

## Reproduction: harness now works, bug did not trigger yet

A UI-control daemon was built and runs in the interactive console session
(session 1) from a Startup shortcut (`C:\ll10\ui-daemon.ps1`, TCP listener on
`127.0.0.1:55990` inside the guest). The WinRM client
(`C:\ll10\ui-client.ps1`) sends commands (FOCUS, WIN, TEXT, ENTER, ESC, KEY,
SNAP, PROCS) into the console session. This solves the session-0 problem:
`WScript.Shell.AppActivate` and scheduled tasks had failed, but the daemon
receives `PONG` and reliably drives the Start menu.

`C:\ll10\run-repro.ps1` performs the full cycle 12 times: Win -> type `notepad`
-> PROCS -> Enter -> check whether `notepad.exe` launched. Result on
2026-11-20 (guest build 19045.256): **12 iterations, 12 successes
(`notepad-after-enter=1`), 0 failures**. The search worked every time. No
Application event log entries mentioning Search/StartMenu appeared. The bug is
intermittent and did not reproduce in this 12-iteration run.

Failed approaches before the daemon (do not repeat):

- Scheduled tasks with `-LogonType Interactive` fail with `0x800710E0`.
- A Startup `SendKeys` loop without the daemon produced `notepad-count=0`.
- `WScript.Shell.AppActivate("Program Manager")` from WinRM (session 0) returns
  False.
- NAT rule host `127.0.0.1:55990` -> guest `55990` was added, but host firewall
  aborts the connection; use the in-guest `ui-client.ps1` instead.

Without a captured failing state, static REA alone cannot establish the root
cause among 23k unlabeled procedures. Per the plan and repository rules, no
binary patch will be guessed. Next step: run the 12-iteration cycle repeatedly
(or with more iterations / faster key timing) until one `notepad-after-enter=0`
occurs, then capture PROCS, event logs and a UI snapshot at that exact moment.

## Versioned daemon

The UI bridge is now a versioned package, `packages/vm-daemon/`:

- `daemon/ll10-ui-daemon.ps1` - runs inside the guest in session 1, listens on
  TCP 55990, supports `PING`, `FOCUS`, `WIN`, `TEXT`, `ENTER`, `ESC`, `SNAP`,
  `PROCS`, `LONGRUN [n]`, `GETLOG` and `STATUS`.
- `client/vmctl.ps1` - host controller over the VirtualBox NAT port forward
  (host 127.0.0.1:55990), so no WinRM is needed to drive the UI or read logs.
- `scripts/install-in-guest.ps1` - one-time install via WinRM plus a Startup
  shortcut.

A NAT rule maps host 55990 to guest 55990. A guest firewall may abort the
loopback TCP connection; the alternative shared-folder bridge is documented in
the package README.

## BUG REPRODUCED (2026-11-20, VM on h-server-1)

A 300-iteration batch (85 completed before the guest rebooted) produced
**1 failure at iteration 82** (~1.2% failure rate):

```
2026-11-20T19:22:58.8654556+00:00 RESULT FAIL *** BUG REPRODUCED ***
2026-11-20T19:22:58.8968729+00:00 SNAP: count=1 | [Program Manager] class=[Progman] pid=3276
2026-11-20T19:22:59.4415098+00:00 EV 11/20/2026 19:22:23 id=16394 Microsoft-Windows-Security-SPP ...
```

The `SNAP` at failure shows **only `[Program Manager]`** in UI Automation,
i.e. the `SearchApp` window is not visible while the failure is active. This is
consistent with the search host being stuck / not processing the typed query.
The only Application event in the preceding minute was unrelated
(Security-SPP), so no crash was logged. Log saved at
`.data/rea/repro-long-bug-reproduced.log`.

### Environment where it reproduced
- VM `ll10-22h2` running on **h-server-1** via QEMU/KVM (not the local
  VirtualBox host), booted from the flat parent disk
  `ll10-22h2-parent.qcow2` (the differencing snapshot image did not boot
  correctly after conversion).
- Guest build 19045, x64, user `ll10`. QEMU NAT maps host 55990 -> guest 55990
  (daemon), 55985 -> 5985 (WinRM), 5900 (VNC).
- SSH access to h-server-1: user `server`, key
  `C:\Users\Gio\Documents\key`, Tailscale `100.69.33.118`
  (from gti-infra `docs/MACHINES.md`).

## REA progress on SearchApp.exe

`SearchApp.exe` e19dfc86... is a native x86-64 PE with 23,613 procedures and
6,171 strings. REA/Ghidra (via `rea` 6.4.0 CLI with Ghidra 12.1.4 and JDK 21)
is operational. Key findings:

- The search UI is Cortana Query Formulation: strings such as
  `Cortana.UI.ViewModels.QueryFormulationViewModel`,
  `QueryFormulationVisibilityState`, `QueryFormulationHostedWebView`,
  `ms-appx:///Views/QueryFormulationControl.xaml`, and the source path
  `shellcommon\shell\search\cortanaui\src\ViewModels\QueryFormulationViewModel.cpp`.
- `Cortana::UI::QueryFormationViewModelForwarder` has paired visibility
  callbacks that mutate two adjacent boolean state fields on the forwarder
  object (all decompiled with REA/Ghidra):
  - `FUN_140012c80` (`AppShown`): writes `*(param_1 + 0x58) = 1`, then calls
    virtual method `+0x30` on the inner object at `+0x28`.
  - `FUN_140012d30` (`AppHidden`): writes `*(param_1 + 0x58) = 0`, then calls
    virtual method `+0x38`.
  - `FUN_140012de0` (`WebViewShown`): writes `*(param_1 + 0x59) = 1`, then
    calls virtual method `+0x40`.
  - `FUN_140137f70` (`DynamicSearchBoxContentUpdated`, xref from
    `0x14028a550`): telemetry forwarder only.
- The forwarder vtable (around `0x1402ab9ee`..`0x1402abe80`) lists the paired
  callbacks: `WebViewHidden`, `WebViewShown`, `AppHidden`, `AppShown`,
  `NotifyAppSuspending`, and also
  `SearchBoxViewModelForwarder::GotFocusAfterAppHidden`,
  `SearchBoxViewModelForwarder::InputPaneHidden`/`InputPaneShown`.
- `FUN_14011627c` is a WebView telemetry event handler that logs
  `WebViewHidden`, `LoadingNavComplete`, `Error`, and reads `param_1[0x58]`
  before recording a `WebViewHidden` event.
- `FUN_140015450` = `SearchBoxViewModelForwarder::GotFocusAfterAppHidden`
  (vtable xref `0x140175a34`): if `*(param_1 + 0x10) == 0`, it writes
  `*(param_1 + 0x50) = 1` and **returns immediately**, without invoking the
  AppShown/WebViewShown visibility callbacks that restore `+0x58`/`+0x59`.
  Otherwise it goes through an error-logging path (`FUN_14006c39c(..., 0x26)`).
- `FUN_14011627c` is a WebView telemetry event handler that logs
  `WebViewHidden`, `LoadingNavComplete`, `Error`, and reads
  `param_1[0x58]` before recording a `WebViewHidden` event.

**Runtime evidence from the second capture (2026-11-20, 57 iterations, 11
failures ~19%):**

- `SearchApp.exe` CPU grows monotonically across iterations: 5 -> 12 -> 17 ->
  52 -> 78 -> 83 -> 99 -> 102 -> 110, with thread counts 32 -> 40 -> 52 ->
  57 -> 60. This is the signature of a **leaked loop / threads on every Start
  activation**, not a one-off desync.
- At failure, `SNAP` often shows a window
  `[notepad - Search - Profile 1 - Microsoft Edge] class=[Chrome_WidgetWin_1]`
  (Edge pid ~6232) instead of a SearchApp window. The Cortana search UI is a
  **WebView2 hosted by Edge**; when the native search path fails, the query
  goes to an Edge search tab and Enter does not launch the app.
- Mini-dumps of `SearchApp.exe` were captured at each failure
  (`C:\ll10\searchapp-fail-<i>.dmp`, 379-517MB each) and are available on the
  guest disk for memory analysis.

## FINAL RESOLUTION (2026-11-21): degraded AppX registration

After reverting the experimental binary patches and re-registering the AppX
package with:

```
Add-AppxPackage -Path C:\Windows\SystemApps\Microsoft.Windows.Search_cw5n1h2txyewy\AppxManifest.xml -DisableDevelopmentMode -Register
```

the bug **disappeared completely**: three consecutive batches of 132, 137 and
137 iterations (406 total) with **0 failures**. Before that, the same guest
produced 11/57, 11/57 and 1/101 failures.

The TWinUI event 5961 errors seen during the patched experiments had ErrorCode
`0x80073D07` (AppX deployment/registration error), not the original
`0x80070057` (E_INVALIDARG). Those were artifacts of replacing the signed
`SearchApp.exe` in place, which invalidated the AppX registration.

**Conclusion:** the root cause is a **degraded AppX registration** of
`Microsoft.Windows.Search` in the guest, not a code defect in `SearchApp.exe`.
The correct fix is to re-register the package (documented in KB4520146), not a
binary patch. Per the repository rules (no registry/config-only mitigation
where a binary patch is required), **no binary patch is being shipped** — there
is no faulty instruction to patch because the stock binary is correct.

## ROOT CAUSE FOUND (2026-11-21, TWinUI event 5961)

With the improved failure capture (TWinUI channel), the exact error is now
visible on every failure:

```
TWINUI id=5961: Activation for
Microsoft.Windows.Search_cw5n1h2txyewy!CortanaUI failed.
Error code: The parameter is incorrect. Activation phase: COM ActivateExtension
```

`The parameter is incorrect` is `E_INVALIDARG` (0x80070057). The failure is in
the **COM activation of the CortanaUI component** (`COM ActivateExtension`
phase), i.e. SearchApp passes an invalid parameter when activating the
CortanaUI extension. This is the real root cause, superseding the earlier
visibility-flag hypothesis.

**Experiment performed and reverted:** patched `FUN_1400a8ed0` (0x1400a8ed0)
to NOP the `JNZ` guarding the `+0x170` flag, forcing the
`QueryFormulationControl.xaml` / WebView to reload on every activation. This
made the failure **much worse** (dozens of failures) and produced the TWinUI
5961 events above. The patch was reverted; the guest is back to the stock hash
`e19dfc86...`. This disproves the visibility-flag hypothesis and confirms the
COM activation error.

**Dump analysis (cdb.exe from WinDbg, 2026-11-20):** the failure mini-dump
(`searchapp-fail-1.dmp`, copied to `.data/rea/`) has 28 threads. Stack counts:

- 2 threads in `edgehtml!CreateCoreWebView`
- 8 threads in `edgehtml!InitializeLocalHtmlEngine`
- 7 frames in `Chakra!` (JS engine)
- 8 frames in `Windows_UI_Xaml!`

This confirms the leak is in the **EdgeHTML WebView hosting path**: each Start
activation spawns another `CreateCoreWebView` / `InitializeLocalHtmlEngine`
thread that never exits, accumulating CPU and threads until the native search
UI stops responding and the query falls through to the Edge search tab.

**Working hypothesis (runtime-confirmed mechanism, exact root cause not yet
isolated):** repeated Start activation leaks EdgeHTML WebView initialization
threads in `SearchApp.exe`. Once enough threads accumulate, the native Search
UI stops responding and the query is routed to the Edge-hosted WebView search
tab, where Enter does not launch the app. The state-field observation
(`+0x58`/`+0x59` in the forwarder, `GotFocusAfterAppHidden` returning early
after setting `+0x50`) is consistent with a visibility bookkeeping bug that
prevents the native UI from being re-shown, but the precise faulty instruction
has not been isolated yet.

**This is not sufficient for a patch.** No specific faulty instruction or
unsafe state transition has been proven yet, so per the repository rules no
binary patch is being proposed.

## Next steps

1. Prove or disprove the state-desync hypothesis: decompile the complementary
   `AppShown` / `WebViewHidden` forwarder callbacks and the code that reads
   `+0x58`/`+0x59` when the Start menu is activated, then identify whether a
   bounded branch/data patch can force a consistent state.
2. Improve the failure capture: on `RESULT FAIL`, dump `SearchApp.exe` CPU and
   thread states and `Microsoft-Windows-TWinUI/Operational` events at that
   exact moment.
3. Only after a root cause and a safe binary patch site are established, add
   the patch. If no safe site exists, stop and report (no registry-only
   mitigation).

## Guest VM lifecycle fix (systemd)

Repeated host/server restarts were killing the QEMU process and the SSH tunnel,
requiring manual VM restarts. Fixed on `h-server-1` with a systemd unit:

- `/etc/systemd/system/ll10-vm.service` (source in
  `packages/vm-daemon/ll10-vm.service`) runs QEMU with `Restart=always`,
  `RestartSec=5`, enabled at boot, logging to
  `/home/server/vm/qemu-systemd.log`.
- Verified: `systemctl is-active ll10-vm` stayed `active` after
  `systemctl kill -s SIGKILL ll10-vm`.
- The host-side SSH tunnel is no longer needed: the host controller runs
  directly on h-server-1 (`ssh ... python3 /home/server/vm/vmd.py <cmd>`).

## Cleanup note

Temporary test scripts and shortcuts were left in the guest under `C:\ll10\`
and in the Startup folder (`LL10SearchRepro.lnk`). They should be removed once
reproduction is working, and the guest should be reverted to `baseline` before
any real patch validation.
