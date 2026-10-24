# Long Live 10

Community-driven project to extend Windows 10 support after conventional support ends, initially focused on Windows 10 22H2 x64.

## Status

Versioned catalog contracts, the compatibility catalog, applicability and operation planning, the agent command line, signed release packaging, and the Flutter diagnostics and status screens are implemented. Unit and contract tests pass. The Hyper-V provider and base image guide are ready, but real VM validation is pending a prepared image. The CI and release workflows exist but have not run on GitHub Actions. See the [support matrix](docs/support-matrix.md) for supported, experimental, and out-of-scope capabilities, [ROADMAP.md](ROADMAP.md) for planned work, and `.stillmark/plans/` for the private execution plan.

## Goals

- Security patch catalog for unsupported Windows versions
- Compatibility layer for drivers, devices, APIs, and software
- Local patch engine with apply and revert capabilities
- Automatic updater through GitHub Releases with signature and hash verification
- Community infrastructure with an open core

Essential security fixes stay free. Optional services and sponsorships fund the work.

## Safety

- **Never apply experimental patches to the host system.**
- All destructive testing happens in disposable VMs.
- The host machine is for development only.

## Structure

Monorepo with packages under `packages/`:

| Package | Responsibility |
|---------|----------------|
| `agent` | Diagnostics and the bounded command line for plan, file apply, and revert |
| `agent-ffi` | C ABI loaded by the Flutter app; owns `unsafe` and JSON marshalling |
| `ui-contract` | Versioned JSON payloads and shared fixtures for the interface |
| `ui` | Flutter desktop application (Windows first) |
| `patch-core` | Applicability, operation planning, and conflict detection |
| `updater` | Signed release manifest verification and key lifecycle |
| `catalog-schema` | Versioned contracts for patches, artifacts, builds, results |
| `compatibility-engine` | Requirement resolution for devices, drivers, and software |
| `compatibility-catalog` | Versioned catalog data and fixtures |
| `vm-harness` | Disposable VM automation for Windows testing |
| `release-tools` | Release specs, packaging, staging, and manifest signing |

The interface is Flutter and talks to the Rust core through `dart:ffi` and a small C ABI. The Dart side loads the `agent-ffi` dynamic library and exchanges UTF-8 JSON. See [ADR-003](docs/decisions/ADR-003-flutter-ui.md) for the boundary and the required toolchain.

## Development

```powershell
yarn build      # cargo build
yarn test       # cargo test
yarn lint       # cargo clippy
yarn format     # cargo fmt
yarn validate:catalog  # validate patch and compatibility catalogs
yarn test:ci    # test changed-file detection for CI
```

Run one package only:

```powershell
cargo test -p vm-harness
```

### Agent command line

`packages/agent` builds `ll10-agent`. Diagnostics are read-only. Plan, apply, and revert work from a request document, and apply and revert are dry runs unless `--confirm` is present.

```powershell
cargo run -p agent --bin ll10-agent -- diagnostics --json
cargo run -p agent --bin ll10-agent -- plan --request request.json
cargo run -p agent --bin ll10-agent -- apply --request request.json --artifact-dir artifacts --target-dir target --journal journal --confirm
cargo run -p agent --bin ll10-agent -- revert --journal journal --confirm
```

The current executor only replaces files inside the required target directory. Package and registry operations are rejected before anything is written. See [packages/agent/README.md](packages/agent/README.md).

### Flutter interface

`packages/ui` is a Flutter desktop app with a diagnostics screen, typed models,
a versioned catalog contract, and English and Portuguese strings. The Windows
runner builds and starts with the Rust library installed next to the executable.

```powershell
cargo build -p agent-ffi --release
cd packages/ui
flutter pub get
flutter analyze
flutter test
flutter build windows --release
```

The release build installs `agent_ffi.dll` next to `long_live_10.exe`.

Setup details live in [docs/development/](docs/development/). The Hyper-V test image is documented in [docs/vm/windows-10-22h2-base-image.md](docs/vm/windows-10-22h2-base-image.md).

## Planning

- Public roadmap: [ROADMAP.md](ROADMAP.md)
- Support matrix: [docs/support-matrix.md](docs/support-matrix.md)
- Contribution guide: [CONTRIBUTING.md](CONTRIBUTING.md)
- Governance and funding: [GOVERNANCE.md](GOVERNANCE.md)
- Architecture decisions: [docs/decisions/](docs/decisions/)
- Execution plan, progress, and findings: `.stillmark/plans/` (local, not versioned)

The planning workflow uses [Stillmark](https://stillmark.dev/). Install its agent skill with the instructions in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT License. See [LICENSE](LICENSE). This license does not grant rights over third-party proprietary binaries or components.
