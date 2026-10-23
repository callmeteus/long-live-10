# compatibility-engine

Requirement resolution for devices, drivers, and software on Long Live 10.

## Overview

The engine compares a compatibility catalog entry with the observable state of a guest. It answers one question: can this entry be used on this machine right now?

It owns:

- build and architecture matching;
- hardware identifier matching for devices and drivers;
- driver version comparison against the assessed version;
- software presence and version checks;
- dependency resolution;
- lifecycle handling for validated, experimental, and discontinued entries.

It does **not** install software, drivers, or updates. It also does not decide whether an entry should be recommended; that belongs to the agent and UI layers.

## Resolution outcomes

| Status | Meaning |
|--------|---------|
| `Compatible` | Every requirement is satisfied. |
| `Incompatible` | At least one requirement is definitively not satisfied. |
| `Refused` | No definitive mismatch was found, but a required value is unknown. |

Unknown values never count as compatible. This keeps the engine safe when a guest does not report its architecture, driver version, or another required field.

Each resolution carries individual `RequirementCheck` entries with a kind, the requirement being checked, a `Satisfied`, `Unsatisfied`, or `Undetermined` status, and a human-readable detail. Known issues and limitations from the catalog entry are copied into the resolution.

## Usage

```rust
use compatibility_engine::{CompatibilityEngine, GuestProfile};
use catalog_schema::Architecture;

let mut profile = GuestProfile::new("win10-22h2-x64");
profile.architecture = Some(Architecture::X64);
profile
    .software_versions
    .insert("software-dotnet-framework-4-8-1".to_string(), "4.8.1".to_string());

let engine = CompatibilityEngine::new();
let resolution = engine.resolve(&entry, &profile);

if resolution.status == compatibility_engine::ResolutionStatus::Compatible {
    println!("entry is usable");
}
```

`resolve_catalog` applies the same profile to every entry and returns resolutions in catalog order.

## Version matching

Versions are compared segment by segment with zero padding, so `1.0.0.0` matches `1.0.0`. Numeric values are compared as numbers, not strings. A tested version may use `x` or `*` as a wildcard segment:

- `14.x` matches `14.44.35211`
- `10.0.*` matches `10.0.19045`
- `13.x` does not match `14.0.0`

Non-numeric values fall back to a case-insensitive string comparison.

Driver versions must match the assessed version exactly. A newer driver is reported as a mismatch because it has not been tested by the catalog entry.

Hardware identifiers are normalized by trimming and uppercasing, so `PCI\VEN_1234&DEV_5678` matches `pci\ven_1234&dev_5678`.

## Lifecycle

- `Validated` entries resolve normally.
- `Experimental` entries can resolve as compatible, but the lifecycle check records that the entry has not passed the validation tier.
- `Discontinued` entries are always incompatible.

## Run

```powershell
cargo test -p compatibility-engine
```
