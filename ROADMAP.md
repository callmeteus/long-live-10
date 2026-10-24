# Roadmap

Community support for Windows 10 after conventional support ends. Items describe intended capabilities; only the Done section reports delivered work.

## Backlog

- [ ] **R-005** VM smoke test tier
  priority: high · status: planned · added: 2026-10-23
  Install the agent, detect the guest build, validate the catalog, run a harmless operation, then revert.
- [ ] **R-015** Self-update with failure recovery
  priority: high · status: planned · added: 2026-10-23
  Safe update of the agent itself with rollback when installation or startup fails.
- [ ] **R-020** First compatibility fix validated in a VM
  priority: medium · status: planned · added: 2026-10-23
  Small reproducible case with apply, verification, and revert evidence.
- [ ] **R-021** First experimental security patch with technical evidence
  priority: high · status: planned · added: 2026-10-23
  Documented vulnerability, bounded mitigation, and tests that prove the mitigated behavior.
- [ ] **R-046** CVE-2026-96951: Windows Shell Mark of the Web bypass, actively exploited
  priority: critical · status: planned · added: 2026-10-23
  Command injection in Windows Shell rated 8.8; added to CISA KEV on 2026-10-13 as actively exploited; weaponized through malicious LNK files; Windows 10 22H2 fixed in build 19045.7807. The community fix must restore MotW handling and prove the bypass no longer works in a VM.
- [ ] **R-047** CVE-2026-85880: actively exploited Windows heap overflow
  priority: critical · status: planned · added: 2026-10-23
  Heap-based buffer overflow (CWE-122) rated 7.8 disclosed 2026-09; affects Windows 10 1607 through 22H2; 22H2 fixed in build 19045.7725. Identify the exact component from MSRC guidance and deliver a bounded mitigation with VM evidence.
- [ ] **R-048** CVE-2026-100219 and CVE-2026-105016: Microsoft QUIC remote code execution
  priority: critical · status: planned · added: 2026-10-23
  Two critical RCEs in msquic rated 9.8 disclosed 2026-10: use-after-free and heap-based buffer overflow; no authentication or user interaction required. Confirm Windows 10 22H2 affected builds from MSRC before planning any work.
- [ ] **R-049** October 2026 client decoder RCE batch
  priority: high · status: planned · added: 2026-10-23
  Critical RCEs disclosed 2026-10-13: CVE-2026-102225 (Text Shaping 8.8), CVE-2026-96943 and CVE-2026-96202 (Raw Image Extension 8.8), CVE-2026-100220 (Media Video Decoder 8.8), CVE-2026-83934 (GDI+ 8.8), CVE-2026-108457 (Windows PDF 8.1), with CVE-2026-88073 (Graphics Kernel elevation of privilege 8.8) and CVE-2026-70187 (Remote Desktop Services RCE 8.1). Confirm 22H2 affected builds and order by real exposure.
- [ ] **R-050** CVE-2026-100202: Message Queuing remote code execution
  priority: high · status: planned · added: 2026-10-23
  Queue Manager RCE rated 9.8 disclosed 2026-10. MSMQ is an optional client component, so document real-world exposure and confirm 22H2 applicability first.
- [ ] **R-051** CVE-2026-40415: TCP/IP use-after-free remote code execution
  priority: high · status: planned · added: 2026-10-23
  Use-after-free in tcpip.sys published 2026-05-12 enabling remote kernel code execution with high attack complexity. Confirm which Windows 10 builds Microsoft fixed and whether unpatched 22H2 systems remain affected.
- [ ] **R-052** CVE-2026-20805 and CVE-2026-20816: DWM disclosure and Installer elevation from early 2026
  priority: medium · status: planned · added: 2026-10-23
  CVE-2026-20805 (Desktop Window Manager information disclosure) was reported exploited in the wild in 2026-01; CVE-2026-20816 (Windows Installer elevation, 7.8, TOCTOU race). Sources are community triage rather than CISA KEV, so verify exploitation status and applicability before starting.
- [ ] **R-053** Suppress Microsoft end-of-support migration prompts
  priority: medium · status: planned · added: 2026-10-23
  Hide the one-per-period full-screen migration notice, the taskbar migration warning, and related prompts Microsoft pushes on Windows 10 after the support end; keep the agent's own diagnostics as the single status surface.
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
- [x] **R-001** Unify monorepo commands and dependency rules
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Root commands build, test, lint, and validate each package; package dependencies stay directional and documented.
- [x] **R-007** Patch applicability evaluation
  priority: critical · status: done · added: 2026-10-23 · done: 2026-10-23
  Match build, architecture, component version, and prerequisites; refuse to apply when applicability is undetermined.
- [x] **R-008** Operation planning and conflict detection
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Ordered plan with conflicts, prerequisites, and explicitly permitted operation types.
- [x] **R-012** Manifest-based updater with signature and hash verification
  priority: critical · status: done · added: 2026-10-23 · done: 2026-10-23
  Versioned metadata, trusted keys, and rejection of unsigned or tampered artifacts.
- [x] **R-013** Downgrade, replay, and expiry protection
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Monotonic versions, expiring metadata, and revocation of compromised artifacts.
- [x] **R-019** Compatibility engine with requirement resolution
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Resolve OS, software, device, driver, and requirement combinations against catalog entries.
- [x] **R-023** Signed release pipeline
  priority: critical · status: done · added: 2026-10-23 · done: 2026-10-23
  Version, artifacts, hashes, signatures, compatibility manifest, notes, limitations, and rollback steps.
- [x] **R-024** Change-aware CI for affected packages
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Run fast checks for affected packages and reserve VM tests for changes that touch installation or system behavior.
- [x] **R-026** Contribution guide with review and testing requirements
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-23
  How to propose, test, and review changes, including evidence requirements.
- [x] **R-027** Public support matrix and support policy
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-23
  Which builds, architectures, and components are supported, experimental, or out of scope.
- [x] **R-028** Funding program: donations, sponsorships, and optional commercial services
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-23
  Essential security fixes stay free; optional services are documented separately.
- [x] **R-039** Flutter desktop app shell with diagnostics and status
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Windows-first Flutter app that loads the Rust library and shows diagnostics, update status, and catalog state.
- [x] **R-040** JSON contract between the Flutter UI and the Rust core
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Versioned DTOs and fixtures; all data crosses the FFI boundary as UTF-8 JSON, never as Rust struct layout.
- [x] **R-041** UI integration tests against the real FFI library
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-23
  Tests load agent_ffi.dll, assert diagnostics, and verify every returned string is freed.
- [x] **R-014** Stable and experimental release channels
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-23
  Channel selection in the updater with separate metadata per channel.
- [x] **R-022** Patch validation standard
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-23
  Define the evidence that separates experimental from validated; a completed installer is not proof.
- [x] **R-003** Agent diagnostics command
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-24
  Report OS build, architecture, installed patches, and catalog state without administrator rights.
- [x] **R-017** Security patch catalog with provenance and evidence
  priority: critical · status: done · added: 2026-10-23 · done: 2026-10-24
  CVEs, affected builds, artifact origin, test results, licenses, and lifecycle status per fix.
- [x] **R-025** Vulnerability intake and disclosure process
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-24
  Private intake, triage, coordinated disclosure, and public advisory template.
- [x] **R-029** Public documentation for catalogs and test results
  priority: low · status: done · added: 2026-10-23 · done: 2026-10-24
  Publish catalog entries, provenance, and test outcomes for users and contributors.
- [x] **R-009** Backup and recovery before privileged changes
  priority: critical · status: done · added: 2026-10-23 · done: 2026-10-24
  Restore point or file backup plus a documented recovery path for every supported operation.
- [x] **R-011** Narrow privileged interface
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-24
  Unprivileged detection and catalog access separated from an allow-listed elevated helper.
- [x] **R-010** Apply, validate, and roll back permitted operations
  priority: critical · status: done · added: 2026-10-23 · done: 2026-10-24
  Only allow-listed operation types; validation proves the intended effect instead of installer exit codes.
- [x] **R-018** Compatibility catalog for devices, drivers, and software
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-24
  Hardware IDs, driver versions, tested builds, dependencies, known issues, and limitations.
- [x] **R-016** Offline and catalog-unavailable behavior
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-24
  Defined behavior when the network, catalog, or release host is unreachable.
- [x] **R-045** Triage newly disclosed Windows 10 CVEs after end of support
  priority: high · status: done · added: 2026-10-23 · done: 2026-10-24
  Monthly intake from MSRC, CISA KEV, and vendor advisories; classify by exploitation status, severity, and 22H2 applicability; feed the security patch catalog and the experimental patch work.
- [x] **R-004** Hyper-V VM harness for disposable tests
  priority: critical · status: done · added: 2026-10-23 · done: 2026-10-24
  Base image, checkpoints, artifact transfer, guest command execution, log capture, and teardown.
- [x] **R-006** Scheduled full test matrix in VMs
  priority: medium · status: done · added: 2026-10-23 · done: 2026-10-24
  Supported builds, clean install, upgrade, interrupted install, rollback, and offline scenarios.