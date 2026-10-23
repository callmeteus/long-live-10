# Catalog Schema

Versioned contracts for patches, artifacts, target builds, and validation results.

This is a low-level dependency with no knowledge of the agent, the updater, or the UI.

## Overview

Every contract carries a `SchemaVersion` and implements the `Validate` trait.

| Contract | Purpose |
|----------|---------|
| `PatchEntry` | Patch metadata, CVEs, component, builds, architectures, and lifecycle status |
| `Artifact` | Downloadable file, format, SHA-256 digest, size, and HTTPS source |
| `TargetBuild` | Windows product, version, build, UBR, and architecture |
| `ValidationResult` | Run outcome, timestamps, checks, and optional log path |

`SchemaVersion::current()` returns major version 1. Documents with a different major version are rejected.

## Validation rules

- Required text fields must not be blank.
- Patch entries need at least one CVE and at least one build and architecture.
- CVE identifiers must match `CVE-YYYY-NNNN`.
- Artifact digests must be 64 hexadecimal characters.
- Artifact file names must not contain path separators.
- Artifact sources must use HTTPS when present.
- Artifact size must be greater than zero.
- Target builds must be Windows 10 build 10240 or newer.
- Validation results need at least one check.
- A passed result cannot contain failed or errored checks.
- A failed result needs at least one failed check.
- Timestamps are RFC 3339 UTC strings and must be ordered.

## Fixtures

Valid fixtures live in `tests/fixtures/valid/`. Invalid fixtures live in `tests/fixtures/invalid/` and cover missing fields, unknown enum values, unsupported versions, invalid digests, unsafe paths, insecure sources, empty checks, and inconsistent results.

```powershell
cargo test -p catalog-schema
```

## Usage

```rust
use catalog_schema::{PatchEntry, Validate};

let entry: PatchEntry = serde_json::from_str(json)?;
entry.validate()?;
```

Errors are typed through `SchemaError`, so callers can distinguish a malformed document from an unsupported schema version or an inconsistent validation result.
