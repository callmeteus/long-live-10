# Compatibility Catalog

Versioned entries for devices, drivers, and software assessed against Windows 10 22H2 x64.

## Overview

The catalog records what was assessed, for which target build, and which external sources support the entry.

It does **not** resolve requirements, choose artifacts, or apply fixes. Requirement resolution belongs to `compatibility-engine`, and installation belongs to the patch and agent layers.

## Contract

`CompatibilityEntry` carries:

| Field | Purpose |
|-------|---------|
| `id` | Stable identifier used by dependencies |
| `schema_version` | Shared major version 1 contract |
| `kind` | `driver`, `device`, or `software` |
| `name`, `vendor`, `description` | Human readable identity |
| `tested_builds` | Target build identifiers from `catalogs/target-builds/` |
| `architectures` | `x86`, `x64`, or `arm64` |
| `hardware_ids` | Required for drivers and devices |
| `driver_version` | Required for drivers |
| `version` | Optional software version |
| `dependencies` | Entry identifiers that must be present first |
| `known_issues` | Known defects or regressions |
| `limitations` | Explicit limits of the assessment |
| `sources` | External evidence with HTTPS URLs and retrieval dates |
| `status` | `experimental`, `validated`, or `discontinued` |

Every source has a title, URL, publisher, and `YYYY-MM-DD` retrieval date. Entries without sources are rejected.

## Layout

```text
packages/compatibility-catalog/
  entries/
    software/
      dotnet-framework-4-8-1.json
      visual-cpp-redistributable-2015-2022.json
  src/
    lib.rs
    bin/compatibility-validate.rs
  tests/
    fixtures/valid/
    fixtures/invalid/
```

Driver and device entries belong in `entries/drivers/` and `entries/devices/` once a vendor source and hardware identifiers are available.

## Validation

From the repository root:

```powershell
npm run validate:catalog
```

This runs the patch catalog validator and the compatibility validator. Run only the compatibility validator with:

```powershell
cargo run -p compatibility-catalog --bin compatibility-validate -- packages/compatibility-catalog/entries
```

The command accepts files or directories, scans recursively, and exits non-zero when an entry is malformed or fails semantic validation.

```powershell
cargo test -p compatibility-catalog
```

## Rules

- Every entry needs at least one external source.
- Sources must use HTTPS and include a retrieval date.
- Drivers need hardware IDs and a driver version.
- Devices need hardware IDs.
- `tested_builds` should reference target builds defined in `catalogs/target-builds/`.
- Mark entries as `experimental` until they pass the VM validation tier.
- Do not record a validated status without a matching validation result.
