# UI

Flutter desktop application for Long Live 10, targeting Windows 10 22H2 x64 first.

## Overview

This package owns the graphical interface: diagnostics, catalog browsing, update
status, and confirmed apply or revert actions. It does not implement patch logic.
Every privileged or system-touching operation belongs to the Rust core and is
reached through the `agent-ffi` C ABI.

The boundary is documented in [ADR-003](../../docs/decisions/ADR-003-flutter-ui.md).

## Status

The Flutter app builds, launches, and reads diagnostics from the Rust library.

- Flutter 3.47.7 stable, Dart 3.13.5.
- Visual Studio 2022 Build Tools 17.14.42 with the C++ workload and Windows SDK 10.0.26100.
- `flutter analyze`, `flutter test`, and `flutter build windows --release` pass.
- `agent_ffi.dll` is installed next to the executable by the CMake build.

## Toolchain

| Tool | Requirement |
|------|-------------|
| Flutter | Stable channel, Windows desktop enabled |
| Visual Studio 2022 | Build Tools with the Desktop development with C++ workload and a Windows SDK |
| Rust | `x86_64-pc-windows-gnu` or MSVC, building `agent-ffi` as `cdylib` |

Verify the installation:

```powershell
flutter --version
flutter doctor -v
```

`flutter doctor` must report the Visual Studio C++ workload as available for the
Windows runner to build.

## Layout

```text
packages/ui/
├── pubspec.yaml
├── lib/
│   ├── main.dart
│   ├── core/
│   │   ├── ffi/
│   │   │   ├── agent_client.dart
│   │   │   └── diagnostics_models.dart
│   │   └── i18n/
│   │       ├── app_localizations.dart
│   │       └── app_strings.dart
│   └── modules/
│       └── diagnostics/
│           ├── diagnostics_format.dart
│           └── diagnostics_screen.dart
├── test/
│   ├── diagnostics_format_test.dart
│   ├── diagnostics_models_test.dart
│   ├── diagnostics_screen_test.dart
│   └── ffi_integration_test.dart
└── windows/
```

## FFI contract

The Dart side loads `agent_ffi.dll` and calls:

| Function | Purpose |
|----------|---------|
| `ll10_abi_version` | Rejects an incompatible library |
| `ll10_diagnostics_json` | Returns diagnostics as UTF-8 JSON |
| `ll10_free_string` | Releases every string returned above |

Rules:

- Parse JSON into typed Dart models; widgets never read raw maps.
- Free every returned pointer, including on error paths.
- The library is resolved from `LL10_AGENT_FFI`, then from `agent_ffi.dll`.
- Load failures surface as an error screen with a retry action.
- No privileged action is triggered without an explicit confirmation step.

Build the library from the repository root:

```powershell
cargo build -p agent-ffi --release
```

The DLL is written to `target/release/agent_ffi.dll`.

## Run

```powershell
flutter pub get
flutter analyze
flutter test
```

Run the integration test against the real Rust library:

```powershell
$env:LL10_AGENT_FFI = "<repo>\target\release\agent_ffi.dll"
flutter test test/ffi_integration_test.dart
```

Without `LL10_AGENT_FFI` the integration test is skipped.

Build and launch the Windows app:

```powershell
cargo build -p agent-ffi --release
flutter build windows --release
build\windows\x64\runner\Release\long_live_10.exe
```

The release build installs `agent_ffi.dll` next to the executable. The window
opens centered in the primary monitor work area.

## Localization

User-facing text lives in `lib/core/i18n/app_strings.dart`, with `en-US` and
`pt-BR` string sets. The active language follows the system locale and falls
back to English.
