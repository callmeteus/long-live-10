# AGENTS.md - long-live-10

Operational guide for AI agents working in this repository. Read
[`docs/patches/field-notes.md`](docs/patches/field-notes.md) before changing
anything under `packages/patches`, `packages/patch-runtime` or the VM tooling:
it lists the approaches that failed and the exact symptoms they produce.

## Language

Answer in the user's language, regardless of the language of the code,
documentation, comments or this file. Code and docs are written in English.

## Toolchain

- `yarn` only. Never `npm`/`npx`.
- Rust workspace; the patch CLI lives in `packages/agent` (`ll10-agent`).
- Build the binary before testing on a guest:
  `cargo build -p agent --release`.
- Tests: `cargo test` (workspace) or a single suite, e.g.
  `cargo test -p patch-runtime --test drift`.
- Validate a manifest after editing it:
  `cargo run -q -p agent -- patches validate`.

## Patch invariants

1. **No write before every gate passes.** Preconditions and `build_variants`
   are enforced *before* the first operation. An earlier version checked them
   but wrote anyway; a DLL from the wrong build bricked the guest.
2. **A binary patch is build-specific.** One artifact per build plus a
   `build_variants` entry mapping stock hash -> artifact. Never write an artifact
   to a file whose hash matches no declared stock build.
3. **The journal is bookkeeping, not proof of state.** SFC and servicing restore
   files without touching it. `apply` decides from the file hash: patched ->
   "already applied"; known stock -> revert the recorded changes and re-apply;
   anything else -> `not_applicable`, no write.
4. **`remediate` must never undo a revert.** It only runs `apply` when the
   journal exists and is not reverted.
5. **Disarm auto-remediation before writing and re-arm afterwards.** The engine's
   own write is an `__InstanceModificationEvent`; leaving it armed starts a
   second apply against the one in progress.
6. **Revert restores through `restore_file`.** Do not reintroduce
   `schedule_rename` for reverts: it hides behind a no-op in the test backend and
   silently restores nothing.
7. **Every patch declares tests.** `packages/patch-runtime/tests/drift.rs` is
   the model: drift, unknown content, double apply, revert after drift, and the
   remediation policy.
8. **Patch layout** stays `packages/patches/<id>/` with `patch.yml`,
   `README.md`, `RECOVERY.md`, `artifacts/` and `notes/`. `notes/` is never
   executed by the engine.
9. **No registry-only mitigations.** Every mitigation must be implemented via
   reverse engineering (R) and a binary file patch (A) - i.e. a patched
   artifact under `artifacts/` produced by a script in `notes/`. Registry
   changes are only ever an accompaniment to a binary patch (as in
   CVE-2026-96951), never the mitigation itself. A patch with no binary
   artifact is not acceptable.

## Replacing protected system files

- The working technique is rename-then-copy (see field notes, section 2).
  `takeown /f` **without** `/a`, grant `*S-1-5-32-544:F` and `*S-1-5-18:F`,
  rename the mapped file to `.ll10-old` (unique name if the plain one exists),
  copy the artifact, `MoveFileEx(..., DELAY_UNTIL_REBOOT)`.
- `LocalSystem` is not an administrator; any code that may run from a WMI
  consumer needs the SYSTEM grant.
- The stock file cannot be deleted before a reboot. Always assume a leftover
  image exists.
- SFC will restore the stock binary. Do not promise "SFC-proof"; the engine
  detects and reapplies.

## VM testing

Guest `ll10` / `Ll10Test!2026`, WinRM `127.0.0.1:55985`, VM `ll10-22h2`.
`yarn vm:start` boots headless and waits for a real WinRM login;
`yarn vm:revert` returns to the `baseline` snapshot.

Known constraints of this image:

- **WinRM dies a few minutes after boot** while the desktop stays alive. Do the
  work in a short window right after boot, run long jobs as a scheduled task, or
  read the result offline.
- **`Start-Process -Verb RunAs` does not work from a WinRM session** (no UAC
  desktop; the process dies with the session). Use
  `Register-ScheduledTask -UserId SYSTEM -LogonType ServiceAccount -RunLevel Highest`.
- **Stop/unregister a task before starting it again.** Starting a task that is
  still `Running` fails (`0x800710E0`) and two instances clobber each other's
  log file.
- **Log to a file immediately.** Redirected stdout from `Start-Process` is not a
  reliable progress indicator.
- **Offline inspection:** after the `baseline` snapshot the live disk is the
  differencing VHD under `vm\<name>\Snapshots\`, not `vm\<name>.vhd`. Mount the
  disk from `VBoxManage showvminfo <vm> --machinereadable` (`SATA-0-0`), and
  power the VM off first.

Guest paths used by the harness: `C:\ll10\agent\ll10-agent.exe`,
`C:\ll10\patches`, logs under `C:\ll10`, engine journal and remediation log
under `C:\ProgramData\LongLive10`.

## Symptom -> cause quick reference

| Symptom | Look at |
|---|---|
| `Unparsable query` | WQL needs doubled backslashes in the path literal |
| Filters created but nothing fires | `__FilterToConsumerBinding` missing or its matcher uses `*:<name>` (must be `*<name>*`) |
| `Object or property already exists` | A binding from a previous arm was not removed |
| `Access is denied` on rename | Owner is Administrators (`takeown /a`) and/or `.ll10-old` already exists |
| `apply` says "already applied" but the file is stock | Journal was trusted; check `target_is_patched` / hash logic |
| Patch reverted itself after a test | The `remediate` policy saw a reverted journal - expected |
| Guest files "missing" on the mounted VHD | Mounted the parent disk instead of the snapshot differencing disk |

## Do not

- Do not edit `WinSxS` manifests or catalogs (`DCM\x01`, signed).
- Do not resurrect `poqexec`/`pending.xml`, testsigning + DISM, or self-signed
  CAB servicing: all were tried and failed (see field notes).
- Do not hard-code user paths; resolve them from the environment.
- Do not hand-edit `artifacts/`; regenerate with the script in `notes/`.
- Do not ship a registry-only or configuration-only mitigation. Every
  mitigation must include a binary artifact produced by reverse engineering
  (R&A). If a binary patch cannot be produced safely, do not add the patch.
- Do not weaken the fail-closed ordering to make a test pass.
