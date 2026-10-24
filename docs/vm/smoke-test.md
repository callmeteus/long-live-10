# Disposable VM smoke test

This procedure validates the agent in the QEMU VM without changing the host.

## Prerequisites

- Start the VM with `yarn vm:start`.
- Ensure WinRM works or use RDP to run commands in the guest.
- Build the agent: `cargo build -p agent`.
- Copy `target/debug/ll10-agent.exe` and the catalog directory into the guest.

## Steps

1. Read diagnostics:

   ```powershell
   ll10-agent diagnostics
   ```

2. Validate the catalog:

   ```powershell
   ll10-agent plan --help
   ```

   Or run the repository validation command from a checkout inside the guest.

3. Create a harmless target file and a matching artifact directory.
4. Run a dry-run plan and review conflicts.
5. Apply with explicit confirmation:

   ```powershell
   ll10-agent apply --target-dir C:\ll10-test --journal-dir C:\ll10-journal --confirm
   ```

6. Verify the intended file change.
7. Revert:

   ```powershell
   ll10-agent revert --journal-dir C:\ll10-journal --confirm
   ```

8. Verify the original file is restored.
9. Revert the VM:

   ```powershell
   yarn vm:revert
   ```

## Evidence

Record the VM build, UBR, agent version, plan JSON, journal, command output, and
logs in the pull request. A completed installer exit code is not sufficient
evidence.
