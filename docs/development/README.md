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

## VM base image

Hyper-V tests need a prepared Windows 10 22H2 base image with a `baseline` checkpoint, no network adapter, and a DPAPI credential file for PowerShell Direct. Follow [Windows 10 22H2 base image](../vm/windows-10-22h2-base-image.md) before running `cargo test -p vm-harness --test smoke_test -- --ignored`.

## Safety guidelines

- Never test destructive operations on the host.
- Use the VM harness for every Windows system modification.
- Prepare and document rollback before applying a change.
- A completed installer is not evidence that a fix works.
