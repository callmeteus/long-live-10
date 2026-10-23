# release-tools

Release packaging, manifest generation, and pre-release validation for Long Live 10.

## Overview

The packager turns a JSON release spec into a signed update manifest and a staged artifact directory. It computes every hash from the files on disk, so the manifest cannot drift from the published artifacts.

It owns:

- reading and validating release metadata;
- hashing artifacts and resolving download URLs;
- staging artifacts under `artifacts/`;
- generating the signed `release-manifest.json` envelope;
- writing the public key used to verify the release.

It does **not** upload files, manage keys, or choose which release should be promoted. Those steps belong to the release workflow.

## Spec

```json
{
    "version": "1.1.0",
    "channel": "stable",
    "published_at": 1800000000,
    "expires_at": 1800086400,
    "minimum_version": null,
    "base_url": "https://example.com/releases/1.1.0",
    "key_id": "release-2026-a",
    "notes": [
        "Adds the status screen."
    ],
    "limitations": [
        "Validated on Windows 10 22H2 x64 only."
    ],
    "rollback_steps": [
        "Uninstall Long Live 10.",
        "Reinstall the previous release from the download archive."
    ],
    "artifacts": [
        {
            "id": "installer",
            "path": "dist/long-live-10.msi"
        }
    ]
}
```

`notes`, `rollback_steps`, and at least one artifact are required. `limitations` may be empty. Every artifact must exist, be non-empty, and have a unique file name. `base_url` must use HTTPS.

## Build

```powershell
$env:LL10_SIGNING_KEY = "<base64-ed25519-seed>"
cargo run -p release-tools --bin release-build -- --spec release.json --out dist/release
```

The command writes:

```text
dist/release/
├── artifacts/
│   └── long-live-10.msi
├── release-manifest.json
└── release-public-key.txt
```

The signing key is a base64-encoded 32-byte Ed25519 seed. Prefer `LL10_SIGNING_KEY` in CI so the key does not appear in the process list. `--signing-key` exists for local runs.

## Verification

The generated envelope verifies with the `updater` crate and the public key written next to it:

```rust
let signed: SignedManifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
let keyring = KeyRing::new(vec![TrustedKey::from_base64(
    "release-2026-a",
    &public_key,
)?]);

signed.verify(&keyring, now, current_version)?;
signed.verify_artifact(&keyring, now, current_version, "installer")?;
```

## Library

```rust
use release_tools::{ReleaseCandidate, ReleaseSpec};

let spec: ReleaseSpec = serde_json::from_str(&fs::read_to_string(spec_path)?)?;
let candidate = ReleaseCandidate::build(spec)?;
let manifest_path = candidate.package(out_dir, &signing_key)?;
```

`ReleaseCandidate::build` performs all validation and hashing. `stage_artifacts` copies files without signing, `sign` produces an envelope, and `package` combines both.

## Run

```powershell
cargo test -p release-tools
```
