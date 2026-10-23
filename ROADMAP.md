# Roadmap

Community support for Windows 10 after conventional support ends. Items describe intended capabilities; only the Done section reports delivered work.

## Backlog

- [ ] **R-001** Unify monorepo commands and dependency rules
  priority: high · status: planned · added: 2026-10-23
  Root commands build, test, lint, and validate each package; package dependencies stay directional and documented.
- [ ] **R-003** Agent diagnostics command
  priority: medium · status: planned · added: 2026-10-23
  Report OS build, architecture, installed patches, and catalog state without administrator rights.
- [ ] **R-004** Hyper-V VM harness for disposable tests
  priority: critical · status: planned · added: 2026-10-23
  Base image, checkpoints, artifact transfer, guest command execution, log capture, and teardown.
- [ ] **R-005** VM smoke test tier
  priority: high · status: planned · added: 2026-10-23
  Install the agent, detect the guest build, validate the catalog, run a harmless operation, then revert.
- [ ] **R-006** Scheduled full test matrix in VMs
  priority: medium · status: planned · added: 2026-10-23
  Supported builds, clean install, upgrade, interrupted install, rollback, and offline scenarios.
- [ ] **R-007** Patch applicability evaluation
  priority: critical · status: planned · added: 2026-10-23
  Match build, architecture, component version, and prerequisites; refuse to apply when applicability is undetermined.
- [ ] **R-008** Operation planning and conflict detection
  priority: high · status: planned · added: 2026-10-23
  Ordered plan with conflicts, prerequisites, and explicitly permitted operation types.
- [ ] **R-009** Backup and recovery before privileged changes
  priority: critical · status: planned · added: 2026-10-23
  Restore point or file backup plus a documented recovery path for every supported operation.
- [ ] **R-010** Apply, validate, and roll back permitted operations
  priority: critical · status: planned · added: 2026-10-23
  Only allow-listed operation types; validation proves the intended effect instead of installer exit codes.
- [ ] **R-011** Narrow privileged interface
  priority: high · status: planned · added: 2026-10-23
  Unprivileged detection and catalog access separated from an allow-listed elevated helper.
- [ ] **R-012** Manifest-based updater with signature and hash verification
  priority: critical · status: planned · added: 2026-10-23
  Versioned metadata, trusted keys, and rejection of unsigned or tampered artifacts.
- [ ] **R-013** Downgrade, replay, and expiry protection
  priority: high · status: planned · added: 2026-10-23
  Monotonic versions, expiring metadata, and revocation of compromised artifacts.
- [ ] **R-014** Stable and experimental release channels
  priority: medium · status: planned · added: 2026-10-23
  Channel selection in the updater with separate metadata per channel.
- [ ] **R-015** Self-update with failure recovery
  priority: high · status: planned · added: 2026-10-23
  Safe update of the agent itself with rollback when installation or startup fails.
- [ ] **R-016** Offline and catalog-unavailable behavior
  priority: medium · status: planned · added: 2026-10-23
  Defined behavior when the network, catalog, or release host is unreachable.
- [ ] **R-017** Security patch catalog with provenance and evidence
  priority: critical · status: planned · added: 2026-10-23
  CVEs, affected builds, artifact origin, test results, licenses, and lifecycle status per fix.
- [ ] **R-018** Compatibility catalog for devices, drivers, and software
  priority: high · status: planned · added: 2026-10-23
  Hardware IDs, driver versions, tested builds, dependencies, known issues, and limitations.
- [ ] **R-019** Compatibility engine with requirement resolution
  priority: high · status: planned · added: 2026-10-23
  Resolve OS, software, device, driver, and requirement combinations against catalog entries.
- [ ] **R-020** First compatibility fix validated in a VM
  priority: medium · status: planned · added: 2026-10-23
  Small reproducible case with apply, verification, and revert evidence.
- [ ] **R-021** First experimental security patch with technical evidence
  priority: high · status: planned · added: 2026-10-23
  Documented vulnerability, bounded mitigation, and tests that prove the mitigated behavior.
- [ ] **R-022** Patch validation standard
  priority: high · status: planned · added: 2026-10-23
  Define the evidence that separates experimental from validated; a completed installer is not proof.
- [ ] **R-023** Signed release pipeline
  priority: critical · status: planned · added: 2026-10-23
  Version, artifacts, hashes, signatures, compatibility manifest, notes, limitations, and rollback steps.
- [ ] **R-024** Change-aware CI for affected packages
  priority: high · status: planned · added: 2026-10-23
  Run fast checks for affected packages and reserve VM tests for changes that touch installation or system behavior.
- [ ] **R-025** Vulnerability intake and disclosure process
  priority: high · status: planned · added: 2026-10-23
  Private intake, triage, coordinated disclosure, and public advisory template.
- [ ] **R-026** Contribution guide with review and testing requirements
  priority: medium · status: planned · added: 2026-10-23
  How to propose, test, and review changes, including evidence requirements.
- [ ] **R-027** Public support matrix and support policy
  priority: medium · status: planned · added: 2026-10-23
  Which builds, architectures, and components are supported, experimental, or out of scope.
- [ ] **R-028** Funding program: donations, sponsorships, and optional commercial services
  priority: medium · status: planned · added: 2026-10-23
  Essential security fixes stay free; optional services are documented separately.
- [ ] **R-029** Public documentation for catalogs and test results
  priority: low · status: planned · added: 2026-10-23
  Publish catalog entries, provenance, and test outcomes for users and contributors.
- [ ] **R-039** Flutter desktop app shell with diagnostics and status
  priority: high · status: planned · added: 2026-10-23
  Windows-first Flutter app that loads the Rust library and shows diagnostics, update status, and catalog state.
- [ ] **R-040** JSON contract between the Flutter UI and the Rust core
  priority: high · status: planned · added: 2026-10-23
  Versioned DTOs and fixtures; all data crosses the FFI boundary as UTF-8 JSON, never as Rust struct layout.
- [ ] **R-041** UI integration tests against the real FFI library
  priority: medium · status: planned · added: 2026-10-23
  Tests load agent_ffi.dll, assert diagnostics, and verify every returned string is freed.
## Deferred

- [ ] **R-034** Windows 10 21H2 and LTSC 2019 support
  priority: low · status: deferred · added: 2026-10-23
- [ ] **R-035** arm64 builds
  priority: low · status: deferred · added: 2026-10-23
- [ ] **R-036** VirtualBox, VMware, and QEMU backends
  priority: low · status: deferred · added: 2026-10-23
- [ ] **R-037** Background service with automatic patch application
  priority: low · status: deferred · added: 2026-10-23
- [ ] **R-038** Generic binary patching engine
  priority: low · status: deferred · added: 2026-10-23
## Done

- [x] **R-030** Inspect environment and select the implementation stack
  priority: medium · status: done · done: 2026-10-23
  Hyper-V detected; Rust workspace for native components with Node tooling for scripts.
- [x] **R-031** Create the monorepo skeleton and package stubs
  priority: medium · status: done · done: 2026-10-23
  Eight packages under packages/ plus workspace configuration and shared dependencies.
- [x] **R-032** Install Stillmark for planning
  priority: medium · status: done · done: 2026-10-23
  Global OpenCode skill and /stillmark command; project initialized with private plans.
- [x] **R-033** Establish initial planning documents
  priority: medium · status: done · done: 2026-10-23
  Roadmap, task plan, progress, and findings files created and kept current.
- [x] **R-043** Install the Flutter SDK and scaffold the desktop app
  priority: medium · status: done · done: 2026-10-23
  Flutter 3.47.7 stable installed, Windows desktop enabled, app scaffolded in packages/ui.
- [x] **R-042** Documented Flutter and Visual Studio toolchain setup
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-23
  Exact versions, flutter doctor checks, and build steps for contributors on Windows.
- [x] **R-044** Windows runner builds and launches with the Rust library bundled
  priority: medium · status: done · done: 2026-10-23
  Flutter release build verified; agent_ffi.dll is installed next to the executable and the window opens centered.
- [x] **R-002** Versioned catalog schema contracts
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Schemas for patch, artifact, target build, and validation result, with valid and invalid fixtures.