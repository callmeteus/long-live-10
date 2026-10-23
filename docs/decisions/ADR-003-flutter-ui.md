# ADR-003: Flutter Desktop UI over a Rust FFI Boundary

## Status

Accepted

## Context

The agent needs a graphical interface for users who will not use a terminal: diagnostics, catalog browsing, apply and revert with explicit confirmation, and update status. The core (applicability, planning, execution, validation) stays in Rust for memory safety and a single native binary.

Flutter was requested for the interface. Flutter supports Rust through two routes:

1. `dart:ffi` against a Rust `cdylib` with a hand-written C ABI.
2. `flutter_rust_bridge`, which generates the Dart and Rust glue from Rust signatures.

## Decision

- Interface: Flutter desktop app targeting Windows x64 first, located in `packages/ui`.
- Core: Rust crates under `packages/`, unchanged.
- Boundary: a dedicated `packages/agent-ffi` crate that exposes a small C ABI and owns all `unsafe` code. The Dart side loads the DLL with `dart:ffi`.
- Start with a hand-written C ABI. Move to `flutter_rust_bridge` only if the surface grows enough that manual bindings become a maintenance risk.
- The C ABI exchanges UTF-8 JSON strings with explicit allocation and free functions. No Rust structs cross the boundary by layout.

## Rationale

- The FFI crate can be built and tested without Flutter installed.
- A JSON contract is versionable and easy to assert in Rust and Dart tests.
- Keeping `unsafe` in one crate makes review and auditing practical.
- No code generation dependency in the first vertical slice.

## Consequences

- Every FFI call allocates a string that Dart must free through `ll10_free_string`. Leaks are a test target.
- The UI is not usable until the Flutter toolchain exists.
- The Rust toolchain is `x86_64-pc-windows-gnu`; producing a DLL that Flutter can load is supported, but Flutter itself still requires the Visual Studio C++ workload for its Windows runner.

## Toolchain requirements

| Tool | Purpose | Status |
|------|---------|--------|
| Flutter SDK (stable, Windows desktop enabled) | Build and run `packages/ui` | Not installed |
| Visual Studio 2022, Desktop development with C++ | Flutter Windows runner, CMake | Not installed |
| Rust `x86_64-pc-windows-gnu` | Build `agent-ffi` as `cdylib` | Installed |

## Alternatives considered

| Option | Why not now |
|--------|-------------|
| Tauri + web UI | Flutter was requested |
| Electron | Larger footprint and a second runtime |
| Native Win32/WinUI | Slower iteration for catalog and form-heavy screens |
| Full `flutter_rust_bridge` from the start | Adds codegen before the contract is stable |
