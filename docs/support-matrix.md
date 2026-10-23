# Support matrix

This document states what Long Live 10 supports today, what is experimental, and what is explicitly out of scope. Support is scoped to disposable virtual machines until a target passes the validation tier described below.

## Support levels

| Level | Meaning |
|-------|---------|
| Supported | Implemented, covered by automated tests, and safe to use within the stated scope. |
| Experimental | Implemented and tested at the unit or widget level, but not yet validated end to end in a disposable VM. Expect changes and verify in a VM first. |
| Out of scope | Not provided by this project. Do not file issues asking for it. |

Nothing in this project is approved for production machines. Every apply or revert flow must run in a disposable VM first.

## Operating systems

| Target | Diagnostics and UI | Patch apply and revert | Notes |
|--------|--------------------|------------------------|-------|
| Windows 10 22H2 x64, build 19045 | Supported | Experimental | Primary target. File-level apply and revert are covered by CLI tests; VM validation is still pending. |
| Windows 10 22H2 x86 | Out of scope | Out of scope | No catalog or test target yet. |
| Windows 10 22H2 ARM64 | Out of scope | Out of scope | No catalog or test target yet. |
| Windows 10 21H2 and earlier | Out of scope | Out of scope | Catalog targets begin at 22H2. |
| Windows 10 LTSC or IoT Enterprise | Out of scope | Out of scope | No validated catalog or test image. |
| Windows 11 | Out of scope | Out of scope | Different servicing and compatibility target. |
| Windows Server | Out of scope | Out of scope | Not a desktop target. |

## Capabilities

| Capability | Level | Evidence and limits |
|------------|-------|---------------------|
| Read-only diagnostics | Supported | Registry read without elevation, versioned JSON payload, Rust and Dart tests. |
| Flutter diagnostics and status screens | Experimental | Widget tests and a real-library integration test pass; manual screen validation after the status screen work is pending. |
| Patch and compatibility catalog validation | Supported | Versioned contracts, valid and invalid fixtures, repository validation in CI. |
| Compatibility requirement resolution | Experimental | Unit tests cover build, architecture, dependencies, hardware IDs, driver versions, and software versions. No VM-validated compatibility fix yet. |
| Agent plan and dry-run | Experimental | `ll10-agent plan` and dry runs are covered by CLI tests. |
| File-level apply and revert | Experimental | `ll10-agent apply` and `revert` replace files inside a required target directory and keep a recovery journal. CLI tests cover round trips, digest mismatches, and journal reuse. No VM run yet. |
| Package or registry operations | Out of scope | The file executor rejects them before writing. A VM-validated execution tier is required first. |
| Release manifest verification | Supported | Ed25519 signatures, key rotation and revocation, expiry, downgrade, and artifact rules are covered by unit tests. |
| Release packaging and signed manifests | Experimental | Local packaging and signing are tested. The GitHub release workflow has not run yet. |
| End-to-end updater | Out of scope | No validated download, install, and recovery flow exists yet. |
| Self-update | Out of scope | Not implemented. |
| Driver or kernel patches | Out of scope | Not implemented and not planned in the current catalog. |
| Code signing bypass, activation bypass, or ESU circumvention | Out of scope | The project does not bypass licensing, activation, or Microsoft entitlement checks. |
| Redistribution of proprietary Microsoft or vendor binaries | Out of scope | The catalog links to vendor sources. It does not redistribute closed binaries. |

## Validation tiers

| Tier | What it proves |
|------|----------------|
| Unit and contract tests | Parsing, validation, planning, signatures, CLI behavior, and UI state handling. |
| Disposable VM | The change applies, produces the intended observable effect, and reverts cleanly on a prepared Windows 10 22H2 image. |
| Physical hardware | Compatibility and recovery across real drivers and devices. Not open yet. |

A capability moves from experimental to supported only after the VM tier passes and the evidence is recorded in the repository.

## Reporting a gap

If a target or capability is missing, open an issue with the target build, architecture, and the evidence you can provide. Do not attach proprietary binaries. See [CONTRIBUTING.md](../CONTRIBUTING.md).
