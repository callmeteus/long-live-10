# Field notes: replacing protected Windows files

This is the evidence behind `packages/patches/cve-2026-96951` and the patch
engine. Everything below was observed on a real Windows 10 22H2 guest
(`ll10-22h2`, 10.0.19045.2965) running under VirtualBox - not inferred from
documentation. It exists so the next person does not repeat the same dead ends.

Start with `docs/patches/authoring.md` for the manifest format. This document is
about why the engine looks the way it does.

## 1. Approaches that do not work

| Attempt | What happened |
|---|---|
| `poqexec.exe` + `pending.xml` (the "Windows Downdate" technique) | The servicing job never applied the payload and left the component store wedged. Removed. |
| `PendingFileRenameOperations` | The entry was ignored for this file; the stock binary stayed. |
| `MoveFileEx(..., MOVEFILE_DELAY_UNTIL_REBOOT)` for the replacement | The deferred rename still needs a write the protection filter allows. No effect. |
| `bcdedit /set testsigning on` + DISM to install a modified component | The component was not installed; servicing validates the catalog, not the boot policy. |
| DISM with a self-signed CAB | Fails with `0x80070032` (the operation is not supported). |
| Editing the SFC manifest | `WinSxS\Manifests\amd64_microsoft-windows-shell32_*.manifest` starts with `44 43 4d 01` (`DCM\x01`): a binary CBS database, not XML. The catalog that covers it is signed, so there is no supported way to make SFC accept different bytes. |

The consequence: **you cannot make SFC accept a patched system binary.** What a
patch can do is notice the repair and apply itself again (section 3).

## 2. Replacing a file that is in use

`C:\Windows\System32\shell32.dll` is loaded by Explorer and most desktop
processes, and it is owned by `TrustedInstaller` with an ACL that denies writes
even to administrators. A plain `CopyFile` over it returns access denied.

The working sequence, implemented in `packages/patch-runtime/src/windows.rs`:

```text
1. takeown /f <file>                                  owner = current principal
2. icacls <file> /grant *S-1-5-32-544:F *S-1-5-18:F   Administrators + SYSTEM
3. rename <file> -> <file>.ll10-old[.<millis>]        old image stays mapped
4. copy <artifact> -> <file>                          new image under the old name
5. MoveFileEx(<old>, NULL, DELAY_UNTIL_REBOOT)        delete the leftover at boot
```

Why the rename is the key step: Windows refuses to **overwrite** a mapped image
but allows **renaming** it. Processes that already loaded the DLL keep using the
old bytes; processes started afterwards load the new file. That is why the patch
takes effect without a reboot, and why the guest does not crash mid-session.

### 2.1 SYSTEM is not Administrators

An interactive admin apply worked from the beginning, but the same code run as
`LocalSystem` (which is what a WMI consumer is) failed with `Access is denied`
on the rename.

Cause: `takeown /f <file> /a` hands ownership to the **Administrators group**.
SYSTEM is not a member of that group, so after ownership moved it had no implicit
`WRITE_DAC` and the grants did not help. Granting `*S-1-5-18` (SYSTEM) explicitly
and using `takeown /f` **without `/a`** (owner becomes the caller) made both
identities work.

Keep the SID form (`*S-1-5-32-544`): the group name is localized
("Administradores", "Administratoren").

### 2.2 The leftover image cannot be deleted before the reboot

Step 3 renames the mapped image aside, and step 5 schedules it for deletion.
Until the machine reboots the file is still mapped and `DeleteFile` fails. A
second replacement that reuses the name `shell32.dll.ll10-old` therefore fails on
the rename with `Access is denied`, because the destination already exists.

Each replacement now uses a unique name (`shell32.dll.ll10-old.<millis>`) when
the plain name is still there. Without this, the **second** apply after a
remediation always failed - the first one after a boot succeeded, which made the
bug look intermittent.

### 2.3 What this survives and what it does not

| Event | Result |
|---|---|
| Reboot | Patch stays applied; the DLL loads normally. Verified. |
| `sfc /scannow` | File returns to the stock hash (`03b7a324...`). Verified. |
| Windows Update with a new build | Stock hash changes, no variant matches, `patches apply` reports `not_applicable` and writes nothing. |
| Auto-remediation armed | Repair is detected and undone within one debounce interval. Verified, including after a reboot. |

## 3. Keeping the patch applied

### 3.1 Why WMI

SFC repairs the file without warning, at an arbitrary time. A scheduled task
would only notice at boot. WMI permanent event subscriptions are the only
built-in mechanism that reacts to a file changing while the machine is running
and survives a reboot (the subscription is stored in the WMI repository).

### 3.2 The subscription

Everything lives in `root\subscription`:

```text
__EventFilter              LL10Remediate_<id>_filter
  SELECT * FROM __InstanceModificationEvent WITHIN <debounce>
  WHERE TargetInstance ISA 'CIM_DataFile'
    AND TargetInstance.Name = 'C:\\Windows\\System32\\shell32.dll'

CommandLineEventConsumer   LL10Remediate_<id>_consumer
  cmd.exe /c ""<agent>" patches remediate <id> --patches-dir "<dir>" --json >> "<log>" 2>&1"

__FilterToConsumerBinding  ties the two together
```

The patch never supplies the command; the engine always runs its own binary.

### 3.3 Policy the engine enforces

- **`remediate` only acts when the journal exists and is not reverted.** A user
  who ran `patches revert` must not have the patch resurrected by a late event.
- **Drift is decided by the file hash, not the journal.** If the target matches a
  declared stock variant, the engine rolls the recorded changes back and applies
  again. If it matches no known stock hash (a Windows update moved the machine
  to an unsupported build), it reports `not_applicable` and leaves the file
  untouched.
- **The subscription is disarmed before the engine writes and re-armed after.**
  The engine's own replacement is an `__InstanceModificationEvent`; leaving the
  subscription armed started a second `apply` while the first one was halfway
  through, and the two fought over the journal.
- **Arming is idempotent.** Disarm removes the binding by matching the filter
  *or* consumer path, then creates fresh objects; if a binding somehow survives,
  the arm script detects it instead of failing.
- **`revert` disarms.** Filter, consumer and binding are removed, and a later
  `remediate` is a no-op.

### 3.4 What "surviving SFC" means here

SFC still restores the stock file. The patch survives in the practical sense:
the repair is undone automatically, seconds later. Do not describe this as
"invisible to SFC" - it is not.

## 4. Engine semantics learned from failures

### 4.1 The journal is bookkeeping, not state

`apply` used to trust a non-reverted journal and return "already applied". After
SFC restored the stock file the journal still said applied, so `apply` and
`remediate` did nothing - the patch looked installed and was not.

Now `apply` hashes the target:

- target hash equals a declared artifact hash -> `already applied` (and re-arm);
- target hash equals a declared **stock** hash -> drift: revert the old journal,
  then apply from scratch so the backup still points at the original file;
- anything else -> refuse before writing.

### 4.2 Fail closed before writing

An early version evaluated preconditions but did not enforce them before the
replacement. A `shell32.dll` from a different build was written to the guest and
the machine stopped booting. Preconditions and variant selection now gate every
write; there is no path that writes first and checks later.

### 4.3 One artifact per build

A patched system binary is build-specific. The manifest declares
`build_variants` (stock hash -> artifact); the engine hashes the current file and
substitutes `$selected`. An unknown build is refused.

Two builds were validated:

| Build | Stock | Patched | Patch site |
|---|---|---|---|
| 10.0.19045.2965 | `03b7a324...` | `805c0ddc...` | `0x2A7BDD`, 6 bytes |
| the build from the CVE report | `936c33a8...` | `66049f12...` | `0x2C901D`, 6 bytes |

The instruction context (+/-32 bytes) is identical between them, which is why one
signature-driven generator (`notes/build-artifact.py`) produces both. The script
fails if the signature is missing or ambiguous; it reproduced the artifacts byte
for byte.

### 4.4 Revert must restore, not postpone

`revert` used to stage the backup and call `schedule_rename`. On Windows that
happens to work (the rename path handles in-use files), but the in-memory test
backend implemented `schedule_rename` as a no-op, so revert silently restored
nothing in tests. Revert now calls `restore_file`, which knows how to replace a
mapped file, and the test double behaves like the real backend.

### 4.5 Test doubles have to fail like the real thing

The no-op `schedule_rename` above is the cautionary example: a test double that
returns `Ok(())` for work it did not do hides whole classes of bugs. The drift
suite (`packages/patch-runtime/tests/drift.rs`) exercises the real decision paths
on `MemoryBackend`, including "unknown hash is never overwritten".

## 5. Trap catalogue

### PowerShell and WMI

| Symptom | Cause | Fix |
|---|---|---|
| `Set-WmiInstance : Unparsable query.` | WQL string literals need every backslash doubled. `'C:\Windows\...'` is invalid; use `'C:\\Windows\\...'`. | Escape the watch path before embedding it. |
| `armed` printed, but no filter exists | The disarm block inlined into the arm script set `$ErrorActionPreference='Continue'`, which leaked and turned creation errors into silent non-terminating errors. | Keep `SilentlyContinue` scoped to the disarm block and set `Stop` again before creating; capture stderr. |
| Filter/consumer exist, event never fires | `__FilterToConsumerBinding` was never created because creation failed with `Object or property already exists`. | Delete the binding first (match `*<name>*` on `Consumer` or `Filter`) and check before creating. |
| Binding survives every disarm | The matcher was `*:<name>`, but the association string is `CommandLineEventConsumer.Name="<name>"` - there is no colon before the name. | Match `*<name>*` without the colon. |
| Consumer runs but cannot replace the file | The consumer is `LocalSystem`, not an administrator. | Grant `*S-1-5-18` and take ownership without `/a`. |
| A single change triggers two applies | The engine's own write fires the subscription it just armed. | Disarm before writing, re-arm after. |
| Event fires, `remediate` says `none` | The journal was reverted or missing; that is the intended policy. | Check the journal before assuming a bug. |

### Windows filesystem

| Symptom | Cause | Fix |
|---|---|---|
| `Access is denied (os error 5)` renaming `shell32.dll` | `takeown /a` left the owner as Administrators, and/or a previous `.ll10-old` still exists. | `takeown /f` (no `/a`), grant SYSTEM, unique `.ll10-old.<millis>` names. |
| Second apply fails, first works | The first `.ll10-old` is still mapped and cannot be deleted before reboot. | Unique leftover names; schedule each for deletion at reboot. |
| Patch disappears after `sfc /scannow` | SFC compares against the component manifest. | Expected; rely on auto-remediation. |
| Manifest looks encrypted | It is `DCM\x01`, a binary CBS database. | Do not try to edit it. |

### VM harness

| Symptom | Cause | Fix |
|---|---|---|
| WinRM stops answering minutes after boot | This image (Defender/OneDrive/first-logon work) stalls the WinRM service; the desktop stays alive. | Work in short windows right after boot, run long jobs as a scheduled task, or read results offline. |
| `Start-Process -Verb RunAs` returns but the script never runs | There is no interactive UAC desktop in a WinRM session, and the elevated process dies with the session. | Register a scheduled task with `-UserId SYSTEM -LogonType ServiceAccount -RunLevel Highest` and start it. |
| `Start-ScheduledTask` fails with `0x800710E0` and no log appears | The task is still `Running` from a previous invocation (long wait loop). Two instances also clobber each other's log. | `Stop-ScheduledTask`/`Unregister-ScheduledTask` first; one instance at a time; log to a unique file. |
| Guest files appear to be missing when the VHD is mounted | After the `baseline` snapshot the live state is a differencing disk under `vm\<name>\Snapshots\`; `vm\<name>.vhd` is the parent. | Mount the disk from `VBoxManage showvminfo --machinereadable` (`SATA-0-0`), after powering off. |
| A test log is empty right after the hash changes | The WMI consumer writes the log a moment later. | Re-read the log; do not conclude the consumer did not run. |

## 6. Checklist for a new patch in this class

1. Patch the binary offline and record the exact site, the stock hash and the
   patched hash. Keep a generator in `notes/` that fails on ambiguity.
2. Declare one artifact per build and a `build_variants` entry for every stock
   hash you can verify. Never guess a hash.
3. Declare `target_path` on artifacts; `auto_remediate` uses it as the watch path
   when `watch_paths` is omitted.
4. Test on a real guest: apply without reboot, use the guest, reboot, use it
   again, revert, reboot, confirm the stock hash and that the subscription is
   gone.
5. Simulate a repair (restore the stock file by hand) and confirm remediation
   within the debounce interval; repeat after a reboot.
6. Confirm `patches remediate` is a no-op after `revert` and that no WMI objects
   remain.
7. Run `cargo test` and `patches validate`; keep the drift tests green.

## 7. Verified evidence

- [x] Apply on 19045.2965 with no reboot -> `805c0ddc...`, shell usable.
- [x] Reboot -> still patched, subscription intact (1 filter, 1 consumer, 1 binding).
- [x] Simulated SFC repair before and after the reboot -> back to `805c0ddc...`
      within one interval; `remediate.log` shows `state_before: Drifted`.
- [x] `revert` -> `03b7a324...`, `restored: 5`, zero WMI objects.
- [x] `remediate` after revert -> `action: none`, file untouched.
- [x] Multi-build selection: the engine picked the 2965 artifact from the stock
      hash without being told the build.
- [x] `sfc /scannow` -> stock hash (documented limitation).
- [x] Drift suite: 7 tests; full `cargo test` green.
