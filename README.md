# Long Live 10

Community-driven project to extend Windows 10 support after conventional support ends, initially focused on Windows 10 22H2 x64.

## Status

Phase 1: the Hyper-V provider and base image guide are in place, with real VM validation pending a prepared image. Phase 2: versioned catalog contracts, fixtures, and validators are implemented. The Flutter interface shell and the diagnostics screen are implemented. See [ROADMAP.md](ROADMAP.md) for planned work and `.stillmark/plans/` for the private execution plan.

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
| `agent` | User entry point, diagnostics, coordination |
| `agent-ffi` | C ABI loaded by the Flutter app; owns `unsafe` and JSON marshalling |
| `ui` | Flutter desktop application (Windows first) |
| `patch-core` | Applicability, planning, execution, validation, rollback |
| `updater` | Release queries, manifest verification, downloads, self-update |
| `catalog-schema` | Versioned contracts for patches, artifacts, builds, results |
| `compatibility-engine` | Requirement resolution for devices, drivers, and software |
| `compatibility-catalog` | Versioned catalog data and fixtures |
| `vm-harness` | Disposable VM automation for Windows testing |
| `release-tools` | Packaging, manifests, and pre-release validation |

The interface is Flutter and talks to the Rust core through `dart:ffi` and a small C ABI. The Dart side loads the `agent-ffi` dynamic library and exchanges UTF-8 JSON. See [ADR-003](docs/decisions/ADR-003-flutter-ui.md) for the boundary and the required toolchain.

## Development

```powershell
npm run build      # cargo build
npm run test       # cargo test
npm run lint       # cargo clippy
npm run format     # cargo fmt
npm run validate:catalog  # validate patch and compatibility catalogs
```

Run one package only:

```powershell
cargo test -p vm-harness
```

### Flutter interface

`packages/ui` is a Flutter desktop app with a diagnostics screen, typed models,
and English and Portuguese strings. The Windows runner builds and starts with
the Rust library installed next to the executable.

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
- Architecture decisions: [docs/decisions/](docs/decisions/)
- Execution plan, progress, and findings: `.stillmark/plans/` (local, not versioned)

The planning workflow uses [Stillmark](https://stillmark.dev/). Install its agent skill with the instructions in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT License. See [LICENSE](LICENSE). This license does not grant rights over third-party proprietary binaries or components.
