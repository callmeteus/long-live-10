# Patch Authoring

Why the engine works the way it does - including the approaches that failed and
the exact symptoms they leave behind - is in
[`field-notes.md`](field-notes.md). Read it before writing a patch that replaces
a system file.

## Layout
```npackages/patches/<id>/
  patch.yml      # manifest
  README.md      # what, why, risks, verify/revert
  RECOVERY.md    # manual recovery
  artifacts/     # binaries/files referenced by artifacts
  notes/         # research/scripts (NOT executed by engine)
```n
## Rules
- Never execute arbitrary scripts
- All changes must be journaled for revert
- Pending file replacements use on-reboot mechanism
- Registry changes journal previous values
- Validate with ll10-agent patches validate
- **No registry-only mitigations.** Every mitigation must be implemented via
  reverse engineering (R) and a binary file patch (A): a patched artifact under
  `artifacts/` produced by a script in `notes/`. Registry changes may only
  accompany a binary patch (see CVE-2026-96951), never replace it. A patch
  with no binary artifact is not acceptable.

## Build variants

A binary patch of a system file is build-specific. When a package supports more
than one Windows build, declare one artifact per build and a `build_variants`
list mapping each stock hash to its artifact:

```yaml
artifacts:
  - id: shell32-19045-2965
    file: artifacts/shell32-patched-19045.2965.dll
    sha256: 805c0ddc...
    target_path: C:\Windows\System32\shell32.dll
build_variants:
  - stock_sha256: 03b7a324...
    artifact: shell32-19045-2965
operations:
  - type: stage_file
    id: stage
    artifact: "$selected"
    to: C:\ProgramData\LongLive10\staging\shell32.dll
checks:
  - type: file_sha256
    id: check
    path: C:\Windows\System32\shell32.dll
    equals: "$selected"
```

The engine hashes the target file, picks the matching variant and substitutes
`$selected`. An unknown build is refused before anything is written. Do not
combine `build_variants` with `preconditions`: the variant list already decides
which builds are supported.

## Auto-remediation

A patch that has to keep working after a repair tool (SFC) or a servicing
operation rewrites its target can ask the engine to watch it:

```yaml
auto_remediate:
  enabled: true
  debounce_seconds: 10
  watch_paths:
    - C:\Windows\System32\shell32.dll   # optional when an artifact declares target_path
```

On `apply` the engine registers a permanent WMI event subscription in
`root\subscription` and removes it on `revert`. The subscription runs the agent
itself, never a command supplied by the patch:

```text
ll10-agent patches remediate <id> --patches-dir <dir> --json
```

Rules the engine enforces:

- `enabled: true` requires `watch_paths` or at least one artifact with
  `target_path`; `patches validate` rejects it otherwise.
- `remediate` re-applies only when the journal exists and says the patch is
  installed. After `revert` it is a no-op, so a late event cannot undo a revert.
- Drift is decided from the file hash, not the journal. A file that matches a
  declared stock build is re-applied; a file that matches no stock hash (a
  Windows update moved the machine to an unsupported build) is reported as
  `not_applicable` and left untouched.
- The subscription is disarmed before the engine writes and re-armed afterwards,
  so the engine's own replacement does not trigger a second apply.
- The consumer runs as `LocalSystem`. Replacement grants both the Administrators
  group and `SYSTEM` (`*S-1-5-18`) full control on the target; granting only
  Administrators fails with access denied under remediation.

Remediation output is appended to `C:\ProgramData\LongLive10\remediate.log`.
