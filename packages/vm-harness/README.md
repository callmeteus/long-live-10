# VM Harness

Automation for disposable Windows test VMs: creation, checkpoint restore, guest commands, log capture, and teardown.

## Overview

This package owns the Hyper-V lifecycle used by the release validation pipeline.

It does **not** prepare the Windows 10 base image. Image preparation is documented in [docs/vm/windows-10-22h2-base-image.md](../../docs/vm/windows-10-22h2-base-image.md) and must produce a VM with a clean baseline checkpoint and networking disabled by default.

## Provider model

`HyperVProvider` builds PowerShell requests and sends them through the `CommandRunner` trait.

| Type | Responsibility |
|------|----------------|
| `CommandRequest` | Program and arguments for one host command |
| `CommandOutput` | Exit status, stdout, and stderr |
| `CommandRunner` | Executes a request; injectable for tests |
| `SystemCommandRunner` | Production runner backed by `std::process::Command` |
| `HyperVProvider` | Hyper-V cmdlets for detect, create, restore, start, stop, execute, logs, and teardown |
| `VmHarness` | Config-driven lifecycle and reversible smoke test |

Production code uses `SystemCommandRunner`. Tests use a recording runner and assert the exact PowerShell that would run, so no Hyper-V installation is needed for unit tests.

## Lifecycle

```text
create_or_restore
    |-- VM exists -> Restore-VMSnapshot -> Start-VM
    |-- VM missing -> New-VM -> Set-VMProcessor -> Start-VM
execute_command -> Invoke-Command (PowerShell Direct)
capture_logs -> Get-WinEvent (Microsoft-Windows-Hyper-V-VMMS-Admin)
revert_or_destroy
    |-- checkpoint configured -> restore checkpoint
    |-- no checkpoint -> Stop-VM -> Remove-VM
```

## Usage

```rust
use vm_harness::{HyperVCreateOptions, HyperVProvider, SystemCommandRunner};

let provider = HyperVProvider::new(SystemCommandRunner, "ll10-22h2");

let options = HyperVCreateOptions {
    memory_mb: 4096,
    cpu_count: 2,
    vhd_path: "C:\\ll10\\base\\Windows10-22H2.vhdx".to_string(),
    switch_name: Some("Default Switch".to_string()),
    generation: 2,
};

provider.create(&options)?;
let output = provider.execute_command("Get-ComputerInfo | Select-Object OsVersion")?;
let logs = provider.capture_logs()?;
provider.teardown()?;
```

## Tests

Unit tests run without Hyper-V:

```powershell
cargo test -p vm-harness
```

The end-to-end smoke test is ignored until a prepared base image exists:

```powershell
$env:LL10_VM_NAME = "ll10-22h2-base"
$env:LL10_VM_VHD = "C:\ll10\base\Windows10-22H2.vhdx"
$env:LL10_VM_CHECKPOINT = "baseline"
cargo test -p vm-harness --test smoke_test -- --ignored
```

## Safety

- All destructive tests must run in disposable VMs.
- Networking stays disabled by default.
- No host credentials are exposed to guests.
- Guest commands use the credentials already available to the host process through PowerShell Direct.
- Hyper-V is the first backend; other variants remain reserved.
