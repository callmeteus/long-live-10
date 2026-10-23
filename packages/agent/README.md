# agent

User entry point for Long Live 10: read-only diagnostics and a bounded command line for planning, applying, and reverting file operations.

## Overview

The agent collects the information the interface and CLI need before any change is planned. Diagnostics never require administrator rights and never modify the system.

It owns:

- agent version, operating system, and architecture reporting;
- the Windows version block read from `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`;
- construction of the versioned diagnostics payload from `ui-contract`;
- the `ll10-agent` command line for diagnostics, plan, apply, and revert;
- a file-level executor with a recovery journal.

It does **not** install update packages, change registry values, or execute arbitrary system commands. Package operations are rejected before any file is written. Those operations belong to a later VM-validated execution tier.

## Commands

```powershell
ll10-agent diagnostics
ll10-agent diagnostics --json
ll10-agent plan --request request.json
ll10-agent apply --request request.json --artifact-dir artifacts --target-dir target --journal journal --confirm
ll10-agent revert --journal journal --confirm
```

Diagnostics are the default when no command is given. `--json` prints the versioned payload used by the Flutter interface.

`apply` and `revert` are dry runs unless `--confirm` is present. Without it they print the plan or journal summary and exit successfully without touching files.

## Request document

```json
{
    "patch": {
        "id": "KB-TEST",
        "schema_version": { "major": 1, "minor": 0, "patch": 0 },
        "cves": ["CVE-2026-0001"],
        "component": "example",
        "builds": ["19045"],
        "architectures": ["x64"],
        "title": "Test patch",
        "description": "Test patch.",
        "status": "experimental"
    },
    "artifacts": [
        {
            "id": "fix",
            "patch_id": "KB-TEST",
            "schema_version": { "major": 1, "minor": 0, "patch": 0 },
            "kind": "dll",
            "file_name": "fix.dll",
            "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "size_bytes": 8,
            "source_url": null
        }
    ],
    "state": {
        "build": 19045,
        "ubr": 6466,
        "architecture": "x64",
        "components": {},
        "applied_patches": []
    },
    "requirements": {
        "component_versions": [],
        "prerequisite_patches": []
    }
}
```

The artifact SHA-256 and size are checked against the file in `--artifact-dir` before anything is written.

## Execution model

| Operation | Behavior |
|-----------|----------|
| `backup` | Journal directory is created before changes. |
| `validate_precondition` | Planning and artifact hashes are verified again. |
| `replace_file` | Existing target is copied to `journal/backups/`, then the artifact replaces it. |
| `record_restore` | Journal records every changed target and backup. |
| Package, registry, rollback operations | Rejected before execution. |

`revert` processes journal changes in reverse order: it restores a backup when one exists, otherwise it removes the file that the plan created. A journal can be reverted once.

`--target-dir` is required so the CLI never guesses a system path. In a VM, point it at the directory under test.

## Verification

VM verification is pending a prepared Windows 10 22H2 image. CLI tests cover diagnostics, plan output, dry-run behavior, confirmed apply, revert, digest mismatch, journal reuse, and unsupported package operations.

## Run

```powershell
cargo test -p agent
```
