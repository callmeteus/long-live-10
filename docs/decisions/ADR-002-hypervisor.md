# ADR-002: VM Hypervisor Selection

## Status
Accepted

## Context
Need to select VM hypervisor for testing on Windows host. Options:
- Hyper-V (detected on host)
- VirtualBox (not installed)
- VMware (not installed)  
- QEMU (not installed)

## Decision
Use Hyper-V as primary hypervisor since it's already available and detected on the host.

## Rationale
- Already present and functional on this system
- Native to Windows
- Good PowerShell/management API support
- Meets requirement of having functional virtualization

## Consequences
- Implementation will be Windows/Hyper-V specific initially
- Need PowerShell integration for VM management
- Must ensure tests run in disposable VMs as required
