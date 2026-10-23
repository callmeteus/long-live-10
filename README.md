# Long Live 10

Community-driven project to extend Windows 10 support after conventional support ends, initially focused on Windows 10 22H2 x64.

## Status

Phase 1: building the Hyper-V test harness. See [ROADMAP.md](ROADMAP.md) for planned work and `.stillmark/plans/` for the private execution plan.

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
| `agent` | User entry point, CLI, diagnostics, coordination |
| `patch-core` | Applicability, planning, execution, validation, rollback |
| `updater` | Release queries, manifest verification, downloads, self-update |
| `catalog-schema` | Versioned contracts for patches, artifacts, builds, results |
| `compatibility-engine` | Requirement resolution for devices, drivers, and software |
| `compatibility-catalog` | Versioned catalog data and fixtures |
| `vm-harness` | Disposable VM automation for Windows testing |
| `release-tools` | Packaging, manifests, and pre-release validation |

## Development

```powershell
npm run build      # cargo build
npm run test       # cargo test
npm run lint       # cargo clippy
npm run format     # cargo fmt
```

Run one package only:

```powershell
cargo test -p vm-harness
```

Setup details live in [docs/development/](docs/development/).

## Planning

- Public roadmap: [ROADMAP.md](ROADMAP.md)
- Architecture decisions: [docs/decisions/](docs/decisions/)
- Execution plan, progress, and findings: `.stillmark/plans/` (local, not versioned)

The planning workflow uses [Stillmark](https://stillmark.dev/). Install its agent skill with the instructions in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT License. See [LICENSE](LICENSE). This license does not grant rights over third-party proprietary binaries or components.
