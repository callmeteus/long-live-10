# Development

## Quick start

```powershell
npm run build       # cargo build
npm run test        # cargo test
npm run lint        # cargo clippy -- -D warnings
npm run format      # cargo fmt
npm run validate:catalog  # validate patch and compatibility catalogs
```

Build or test a single package:

```powershell
cargo build -p catalog-schema
cargo test -p vm-harness
cargo test -p agent -p agent-ffi
```

## Flutter interface

The desktop app lives in `packages/ui` and is not part of the Cargo workspace.
It requires:

- Flutter stable with Windows desktop support.
- Visual Studio 2022 with the "Desktop development with C++" workload.
- The `agent_ffi.dll` library built from `packages/agent-ffi`.

Flutter 3.47.7 is installed at `C:\Users\Gio\dev\flutter` and added to the user
`PATH`. Visual Studio 2022 Build Tools 17.14.42 provides the C++ workload and
Windows SDK 10.0.26100, so the Windows runner builds and runs.

```powershell
cd packages/ui
flutter pub get
flutter analyze
flutter test
```

Build the release runner. The build installs `agent_ffi.dll` next to the
executable:

```powershell
cargo build -p agent-ffi --release
flutter build windows --release
```

Run the integration test against the real Rust library:

```powershell
$env:LL10_AGENT_FFI = "<repo>\target\release\agent_ffi.dll"
flutter test test/ffi_integration_test.dart
```

Build and test the Rust side from the repository root:

```powershell
cargo build -p agent-ffi --release
cargo test -p agent -p agent-ffi
```

The DLL is written to `target/release/agent_ffi.dll`.

## CI

Pull requests run only the checks affected by the changed files. The mapping lives in `tools/ci/affected.mjs`:

| Change | Checks |
|--------|--------|
| A Rust package | That package plus every package that depends on it |
| `packages/ui` | Flutter analyze and tests, including the real FFI integration test |
| `catalog-schema`, `compatibility-catalog`, or `catalogs/` | Catalog validation |
| `Cargo.toml` or `Cargo.lock` | Every Rust package and the Flutter interface |
| Workflow or CI tooling | The full pipeline |

Run the mapping locally:

```powershell
npm run test:ci
node tools/ci/affected.mjs HEAD^ HEAD
```

VM tests are not part of normal PR runs. They run only on a self-hosted Hyper-V runner when the PR carries the `vm-tests` label and the change touches `vm-harness`, `patch-core`, or `agent`, or when the workflow is started manually.

## Release workflow

Tagging a commit with `v<version>` runs `.github/workflows/release.yml`. The workflow:

1. runs the Rust and Flutter test suites;
2. builds the release DLL and Windows runner;
3. archives the runner as `long-live-10-<version>-windows-x64.zip`;
4. generates a release spec with notes, limitations, and rollback steps;
5. signs and stages the manifest with `release-tools`;
6. publishes the manifest, public key, and archive to the GitHub release.

Repository configuration:

| Name | Kind | Purpose |
|------|------|---------|
| `LL10_SIGNING_KEY` | Secret | Base64 Ed25519 seed used to sign the manifest |
| `LL10_KEY_ID` | Variable | Key identifier written into the envelope |
| `LL10_BASE_URL` | Variable | HTTPS base URL where artifacts will be served |

The release can also be started manually with `workflow_dispatch`; provide the version and base URL, and optionally the release notes, limitations, and rollback steps. Without manual notes, the workflow writes a single `Release <version>.` entry.

## VM base image

Hyper-V tests need a prepared Windows 10 22H2 base image with a `baseline` checkpoint, no network adapter, and a DPAPI credential file for PowerShell Direct. Follow [Windows 10 22H2 base image](../vm/windows-10-22h2-base-image.md) before running `cargo test -p vm-harness --test smoke_test -- --ignored`.

## Safety guidelines

- Never test destructive operations on the host.
- Use the VM harness for every Windows system modification.
- Prepare and document rollback before applying a change.
- A completed installer is not evidence that a fix works.
