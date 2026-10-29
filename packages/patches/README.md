# Patches

All patches live here, one folder per patch id. Each patch is self-contained with its own docs, artifacts, and a declarative patch.yml manifest consumed by the engine.

## Available patches
| Patch | CVE | Severity | Status | Notes |
|---|---|---|---|---|
| [cve-2026-96951](cve-2026-96951/README.md) | CVE-2026-96951 | High | Active | Windows Shell MotW bypass mitigation (System32 + registry; SysWOW64 pending artifact) |
| [cve-2026-20816](cve-2026-20816/README.md) | CVE-2026-20816 | High | Experimental | Windows Installer elevation: disables AlwaysInstallElevated at its source in msi.dll |

## Rules
- Manifests must be valid YAML and pass ll10-agent patches validate.
- Patches must be reversible (journal + optional evert.after_journal).
- Patches targeting guest systems are refused on host unless explicitly allowed.
- No patch may execute arbitrary scripts; only declarative operations.
- Each patch should include tests/validation steps in its docs.
