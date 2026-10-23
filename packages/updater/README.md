# updater

Release manifest fetching and Ed25519 signature verification for Long Live 10.

## Overview

The updater decides whether a release manifest is authentic and safe to offer. It owns:

- manifest and artifact contracts;
- signature verification against trusted keys;
- validity window, downgrade, and minimum-version checks;
- HTTPS transport of signed envelopes.

It does **not** install artifacts, mutate the host, or decide when to prompt the user. Those responsibilities belong to the agent and UI layers.

## Envelope

A manifest travels as a signed envelope:

```json
{
    "key_id": "release-2026-a",
    "manifest": {
        "schema_version": { "major": 1, "minor": 0, "patch": 0 },
        "version": "1.1.0",
        "channel": "stable",
        "published_at": 1800000000,
        "expires_at": 1800086400,
        "minimum_version": "1.0.0",
        "notes": [
            "Adds the status screen."
        ],
        "limitations": [
            "Validated on Windows 10 22H2 x64 only."
        ],
        "rollback_steps": [
            "Reinstall the previous release from the download archive."
        ],
        "artifacts": [
            {
                "id": "installer",
                "file_name": "long-live-10.msi",
                "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "size_bytes": 1048576,
                "url": "https://example.com/releases/long-live-10.msi"
            }
        ]
    },
    "signature": "base64-ed25519-signature"
}
```

The signature covers `serde_json::to_vec(&manifest)` using the field order in [`ReleaseManifest`](src/lib.rs). Producers must serialize the same struct rather than hand-written JSON to keep the bytes stable.

## Verification

`SignedManifest::verify` checks, in order:

1. A signature is present and valid base64.
2. `key_id` exists in the trusted key ring.
3. The key is active for the manifest publication time. Revoked keys are refused, and retired keys only accept manifests published before the retirement cutoff.
4. The Ed25519 signature matches the canonical manifest bytes.
5. The schema version is supported.
6. The manifest is inside its validity window.
7. The offered version is newer than the installed version.
8. The installed version meets `minimum_version`.
9. Artifacts have HTTPS URLs, valid SHA-256 values, file names without separators, and non-zero sizes.

Any failure returns a typed `ManifestError`. A tampered or unsigned manifest never reaches installation.

## Usage

```rust
use updater::{HttpTransport, KeyRing, ManifestFetcher, TrustedKey};

let mut keyring = KeyRing::default();
keyring.add(TrustedKey::from_base64("release-2026-a", public_key_base64)?);

let fetcher = ManifestFetcher::new(HttpTransport);
let manifest = fetcher.fetch_and_verify(
    "https://example.com/releases/manifest.json",
    &keyring,
    now_unix_seconds,
    env!("CARGO_PKG_VERSION"),
)?;
```

For deterministic tests, implement `ManifestTransport` instead of making network calls.

## Errors

`ManifestError` distinguishes `MissingSignature`, `InvalidSignature`, `UnknownKey`, `RevokedKey`, `RetiredKey`, `KeyNotYetValid`, `KeyExpired`, `Expired`, `NotYetValid`, `Downgrade`, `UpgradeRequired`, `UnknownArtifact`, and artifact validation failures. Match on the variants instead of parsing message text.

## Key management

`KeyRing` holds every key that may appear in a signed envelope. Each key has one lifecycle state:

| State | Behavior |
|-------|----------|
| `Active` | Accepts manifests published inside its optional validity window. |
| `Retired` | Accepts manifests published at or before its retirement cutoff, so older releases stay verifiable. |
| `Revoked` | Refuses every manifest and every artifact signed by the key. |

Rotation keeps both keys in the ring for a transition period:

```rust
let old_key = TrustedKey::from_base64("release-2025", old_public_key)?
    .retired(rotation_unix_seconds);
let new_key = TrustedKey::from_base64("release-2026", new_public_key)?;

let keyring = KeyRing::new(vec![old_key, new_key]);
```

Retirement and revocation use the manifest publication time, not the current clock, so a manifest keeps the same trust decision after it is published. A key can also carry an explicit validity window with `TrustedKey::with_window`; manifests outside that window return `KeyNotYetValid` or `KeyExpired`.

`SignedManifest::verify_artifact` verifies the manifest first, then returns the requested artifact. A revoked key therefore rejects the artifact before any download starts.

Key distribution, storage, and rotation scheduling happen outside this crate. This crate only enforces the trust decision for the keys it is given.

## Run

```powershell
cargo test -p updater
```
