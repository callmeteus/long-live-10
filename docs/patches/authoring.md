# Patch Authoring

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

Reserved, not implemented. `auto_remediate.enabled: true` is rejected by
`patches validate`; keep it disabled until the WMI subscription and its removal
exist and are validated.
