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
