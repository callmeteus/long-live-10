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
3. The Ed25519 signature matches the canonical manifest bytes.
4. The schema version is supported.
5. The manifest is inside its validity window.
6. The offered version is newer than the installed version.
7. The installed version meets `minimum_version`.
8. Artifacts have HTTPS URLs, valid SHA-256 values, file names without separators, and non-zero sizes.

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

`ManifestError` distinguishes `MissingSignature`, `InvalidSignature`, `UnknownKey`, `Expired`, `NotYetValid`, `Downgrade`, `UpgradeRequired`, and artifact validation failures. Match on the variants instead of parsing message text.

## Key management

`KeyRing` accepts multiple keys so a rotation window can trust the old and new key at the same time. Revocation and rotation policy are implemented with the release signing work, not in this package.

## Run

```powershell
cargo test -p updater
```
