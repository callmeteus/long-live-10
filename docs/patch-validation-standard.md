# Patch validation standard

This standard defines the evidence required to move a patch from experimental to validated. A completed installer or a zero exit code is not proof of correct mitigation or correct behavior.

## Validation levels

| Level | Evidence required |
|-------|-------------------|
| Experimental | Catalog entry with sources, affected builds, description, and stated limitations. No VM evidence yet. |
| Partially validated | Unit and contract tests for planning, signatures, and applicability. Dry-run and file-level apply/revert covered by CLI tests. |
| Validated | Disposable VM evidence for apply, verification of the intended effect, and clean revert. The evidence is recorded and linked from the catalog entry. |
| Limited | Validated only under specific conditions with documented unsupported cases. |

## Required evidence for validation

For each change moving to validated:

| Item | What to record |
|------|----------------|
| Target | Windows 10 build, UBR, architecture, edition, and whether the VM is clean or has known updates. |
| Catalog entry | Patch or compatibility entry ID, schema version, sources with retrieval dates, and known limitations. |
| Plan | The plan JSON produced by the agent, including operations, dependencies, and conflicts. |
| Apply | Journal, backups, commands executed, and any errors. Only allowed operations may be executed. |
| Verification | A deterministic test that proves the intended effect. The test must fail without the mitigation and pass with it when feasible. |
| Revert | Journal revert output, restored files, and confirmation that the system returns to the pre-apply state. |
| Artifacts | Artifact hashes and sizes, manifest signatures if applicable, and the binary paths used. |
| Logs | Agent, installer, and relevant Windows logs from the guest with timestamps. |
| Rollback procedure | A tested recovery procedure written for someone reproducing the result. |

## Rules

- Never validate with a physical host. Use a disposable VM with checkpoints.
- Never trust exit codes alone. Validation must test the mitigated behavior.
- Refuse validation if applicability is unknown for any required condition.
- Package operations require a VM-validated execution tier. Do not mark package operations validated until such tier exists and passes.
- Changes that weaken signature checks, hash checks, or explicit confirmation cannot be validated.
- Bypasses for activation, licensing, or entitlement checks are never validated.
- Evidence must be reproducible by another contributor following the documented steps.

## Validation checklist

- [ ] Sources verified and recorded
- [ ] Schema validation passes
- [ ] Plan has no unresolved conflicts and is recovery ready
- [ ] Artifacts verified by size and SHA-256
- [ ] Apply succeeds in a disposable VM
- [ ] Intended effect verified by a deterministic test
- [ ] Revert succeeds and state is restored
- [ ] Logs and artifacts captured
- [ ] Limitations and recovery procedure documented
- [ ] Evidence linked in the catalog entry and PR

## Status in this repository

Current state: contracts, planning, and file-level apply/revert are validated at the unit and CLI level. End-to-end VM validation of package operations and complex mitigations is pending. This standard applies to all future entries regardless of the current implementation limits.
