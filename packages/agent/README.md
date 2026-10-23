# agent

User entry point and read-only diagnostics for Long Live 10.

## Overview

The agent collects the information the interface and CLI need before any change is planned. Diagnostics never require administrator rights and never modify the system.

It owns:

- agent version, operating system, and architecture reporting;
- the Windows version block read from `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion`;
- construction of the versioned diagnostics payload from `ui-contract`.

It does **not** inspect patches, plan operations, or modify the host. Those responsibilities belong to `patch-core` and the CLI.

## Usage

```rust
let info = agent::diagnostics();

println!("{} {} {}", info.agent_version, info.os, info.architecture);
```

Serialization is provided by `ui-contract`:

```rust
let json = info.to_json()?;
```

## Run

```powershell
cargo test -p agent
```
