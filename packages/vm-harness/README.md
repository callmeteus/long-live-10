# VM Harness

Automation for VMs: creation, restoration, execution, log collection, validation and disposal of test environments.

## Safety
- All destructive tests must run in disposable VMs
- Network disabled by default
- No host credentials exposed to guests
- Hyper-V is the primary target on this Windows host
