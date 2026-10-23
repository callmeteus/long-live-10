# ui-contract

Versioned JSON payloads exchanged between the Rust core and the Flutter interface.

## Overview

The crate owns the wire contract, not the data itself. It depends on `catalog-schema` and `compatibility-catalog` so catalog payloads use the same validated entries as the rest of the platform.

It defines:

- `DiagnosticsPayload` for the `ll10_diagnostics_json` boundary;
- `CatalogPayload` with target builds, patches, and compatibility entries;
- `ContractError` for decoding and validation failures;
- shared JSON fixtures used by Rust and Dart tests.

It does **not** collect diagnostics, read the registry, or render anything. Those responsibilities belong to the agent and UI layers.

## Versioning

Every payload starts with `contract_version`:

```json
{
    "contract_version": {
        "major": 1,
        "minor": 0,
        "patch": 0
    }
}
```

- A **major** bump means a breaking shape change. Consumers must reject versions they do not understand.
- A **minor** bump adds optional fields.
- A **patch** bump corrects values without changing the shape.

The Dart models in `packages/ui` support major version 1 and throw a `FormatException` for anything else.

## Diagnostics payload

```json
{
    "contract_version": { "major": 1, "minor": 0, "patch": 0 },
    "agent_version": "0.1.0",
    "os": "windows",
    "architecture": "x86_64",
    "windows": {
        "product_name": "Windows 10 Pro",
        "display_version": "22H2",
        "build": "19045",
        "update_build_revision": 6466
    }
}
```

`windows` is `null` on non-Windows hosts.

## Catalog payload

```json
{
    "contract_version": { "major": 1, "minor": 0, "patch": 0 },
    "target_builds": [],
    "patches": [],
    "compatibility": []
}
```

Each item is the full validated catalog type: `TargetBuild`, `PatchEntry`, or `CompatibilityEntry`. Call `CatalogPayload::validate` after parsing to check the contract version and every nested entry.

## Fixtures

The files under `fixtures/` are the source of truth for both sides:

- `fixtures/diagnostics.json`
- `fixtures/catalog.json`

Rust tests parse and validate them in `tests/contract.rs`. Dart tests parse the same files in `packages/ui/test/contract_fixtures_test.dart`. If a field is renamed, both suites fail until the producers, consumers, and fixtures agree.

## Usage

```rust
use ui_contract::DiagnosticsPayload;

let payload = DiagnosticsPayload::new(
    env!("CARGO_PKG_VERSION"),
    std::env::consts::OS,
    std::env::consts::ARCH,
    None,
);

payload.validate()?;
let json = payload.to_json()?;
```

## Run

```powershell
cargo test -p ui-contract
```
