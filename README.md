# Long Live 10

Community-driven project to extend Windows 10 support after conventional support ends, initially focused on Windows 10 22H2 x64.

## Status

Versioned catalog contracts, the compatibility catalog, applicability and operation planning, the agent command line, signed release packaging, and the Flutter diagnostics and status screens are implemented. Unit and contract tests pass. The VirtualBox harness creates and provisions a disposable Windows 10 22H2 VM that boots auto-logged-in and reachable over WinRM without manual steps; see [VM testing](#vm-testing-virtualbox). Binary `shell32.dll` patch validation in that VM is still in progress. The CI and release workflows exist but have not run on GitHub Actions. See the [support matrix](docs/support-matrix.md) for supported, experimental, and out-of-scope capabilities, [ROADMAP.md](ROADMAP.md) for planned work, and `.stillmark/plans/` for the private execution plan.

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

Setup details live in [docs/development/](docs/development/). The Hyper-V test image is documented in [docs/vm/windows-10-22h2-base-image.md](docs/vm/windows-10-22h2-base-image.md) and the VirtualBox harness in [docs/vm/windows-10-22h2-virtualbox.md](docs/vm/windows-10-22h2-virtualbox.md).

## VM testing (VirtualBox)

Patch and mitigation work is validated in a disposable Windows 10 22H2 x64 VM.
The guest is provisioned **offline**: its virtual disk (VHD) is mounted on the
host and the configuration is written directly into the offline registry hives.
The VM therefore boots already logged in and reachable over WinRM, with no RDP
session, no keyboard automation and no operator interaction.

### Why offline provisioning

Configuring the guest from inside a running session required an interactive
login, was lost on every snapshot restore, and differed from machine to machine.
`Prepare-Guest.ps1` makes the configuration part of the image instead. It is
idempotent, detects the active control set, and shuts the VM down gracefully
first so dirty registry hive logs cannot replay and discard the edits.

### Commands

| Command | What it does |
| --- | --- |
| `yarn vm:setup -- -BaseImage <image.vhd>` | Creates or re-creates the VM, stores the disk as VHD and provisions the guest |
| `yarn vm:prepare` | Re-applies provisioning to an existing powered-off VM |
| `yarn vm:start` | Boots the VM and waits until WinRM actually answers |
| `yarn vm:smoke` | Runs a harmless command inside the guest |
| `yarn vm:finalize` | Shuts down, re-provisions and records the `baseline` snapshot |
| `yarn vm:revert` | Discards guest changes and returns to `baseline` |

`yarn vm:start` waits for a real WS-Management handshake (`Test-WSMan`), not a
TCP connection: VirtualBox NAT accepts TCP connections while the guest is still
booting, which produces false positives.

### What provisioning guarantees

| Setting | Value | Effect |
| --- | --- | --- |
| `Services\WinRM` → `Start` | `2` (automatic) | WinRM starts at boot, no login required |
| `Control\Lsa` → `LocalAccountTokenFilterPolicy` | `1` | Local admin account can authenticate remotely |
| Firewall `DomainProfile` / `StandardProfile` / `PublicProfile` | `EnableFirewall = 0` | Forwarded ports are never filtered |
| `Winlogon` → `AutoAdminLogon`, `DefaultUserName`, `DefaultPassword`, `ForceAutoLogon` | enabled | Console session logs in automatically |
| `Winlogon` → `AutoLogonCount` | deleted | Auto-logon does not expire |
| Startup folder and stale `RunOnce` entries | removed | No console windows flash at boot |

Provisioning also writes `C:\ll10\provision.txt` inside the guest as a
diagnostic marker.

### Verified behaviour

After `yarn vm:revert && yarn vm:start && yarn vm:smoke` on a clean baseline:

```text
user=LL10-TEST\ll10
winrm=Running/Automatic
startupItems=0
consoleWindows=0
console  ll10  1  Active
ll10-smoke
Microsoft Windows 10 Pro
```

First WinRM answer takes about 2.5-3 minutes on a 4 GB / 2 vCPU guest; the smoke
test then completes in a few seconds. `vm:start` allows up to 300 seconds.

### Credentials and ports

| Item | Value |
| --- | --- |
| Account | `ll10` / `Ll10Test!2026` (local administrator) |
| WinRM | `http://127.0.0.1:55985` (NAT → guest 5985) |
| RDP | `127.0.0.1:53389` (NAT → guest 3389, only if needed) |
| VM name | `ll10-22h2` |

> **Known gap.** `yarn patch:apply` currently targets the local machine's
> `C:\Windows\System32\shell32.dll`. Patches must be executed inside the VM over
> WinRM before this command is considered safe to run. Until then, do not run
> `patch:*` on a development host.

Full details, troubleshooting and recovery notes are in
[docs/vm/windows-10-22h2-virtualbox.md](docs/vm/windows-10-22h2-virtualbox.md).

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
