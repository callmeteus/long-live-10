# Contributing to Long Live 10

Long Live 10 extends Windows 10 22H2 x64 with a versioned security catalog, a local patch engine, and a desktop interface. Contributions are welcome when they keep the project testable, reversible, and honest about its limits.

Read the [support matrix](docs/support-matrix.md) before proposing a feature. It defines what is supported, what is experimental, and what is out of scope.

## Ground rules

- Never apply an experimental patch to a physical host. Use a disposable virtual machine.
- Do not redistribute proprietary Microsoft or vendor binaries. Link to the vendor source instead.
- Do not weaken signature checks, hash checks, recovery records, or explicit confirmation steps.
- Do not add activation, licensing, or entitlement bypasses.
- Keep user-facing text free of internal security disclaimers and implementation jargon.
- Write code comments, identifiers, and documentation in English.

## Setup

```powershell
yarn build
yarn test
yarn lint
yarn format
yarn validate:catalog
yarn test:ci
```

Flutter commands need the stable Flutter SDK and Visual Studio 2022 with the C++ workload:

```powershell
cargo build -p agent-ffi
cd packages/ui
flutter pub get
flutter analyze
flutter test
```

The real-library Flutter test loads `agent_ffi.dll` when `LL10_AGENT_FFI` points at the built library. The CI workflow sets this automatically.

## Package boundaries

| Package | Owns |
|---------|------|
| `catalog-schema` | Patch, artifact, build, and validation contracts |
| `compatibility-catalog` | Compatibility entries and their fixtures |
| `compatibility-engine` | Requirement resolution |
| `patch-core` | Applicability and planning |
| `updater` | Signed release manifests and artifact verification |
| `release-tools` | Packaging, hashing, and signed release generation |
| `agent` | Diagnostics and the bounded command line |
| `agent-ffi` | C ABI and JSON marshalling |
| `ui-contract` | Versioned interface payloads |
| `ui` | Flutter desktop application |
| `vm-harness` | Hyper-V automation for disposable guests |

Dependencies stay directional. Lower-level packages must not import higher-level ones. New shared behavior belongs in the package that already owns the contract.

## Tests

Every behavior change needs tests for the happy path and the failure path.

- Rust tests use `tests/` at the package root or `#[cfg(test)]` modules next to the code.
- Flutter tests live in `packages/ui/test/`.
- CLI behavior is tested through the built binary with `CARGO_BIN_EXE_<name>`.
- Catalog changes require valid and invalid fixtures where the schema can reject input.
- VM-dependent tests must stay ignored or gated until a prepared image exists.

A change is not ready when a warning remains in a file you touched.

## Catalog entries

- Every entry carries the current schema version.
- Source URLs must use HTTPS, and `retrieved_at` uses `YYYY-MM-DD`.
- Evidence must point at the vendor or Microsoft advisory that supports the claim.
- Experimental entries must stay experimental until the VM tier records apply, verification, and revert evidence.
- Do not add an entry that only works by bypassing entitlement checks.

Run the validators before opening a pull request:

```powershell
yarn validate:catalog
```

## Patch and execution changes

- Applicability must refuse when the guest state is unknown. Do not guess a build, UBR, architecture, component version, or prerequisite.
- Planning must keep a recovery path for every permitted operation.
- New operation types need an executor, a journal record, a revert path, and tests.
- Package and registry operations remain out of scope until a VM-validated tier exists.
- `apply` and `revert` remain dry runs by default. Never remove the explicit confirmation requirement.

## Interface changes

- The Rust core and Dart models share fixtures under `packages/ui-contract/fixtures/`. Update both sides in the same change.
- Payloads carry `contract_version`, and consumers reject unsupported major versions.
- UI strings exist in English and Portuguese.
- Use the project icon registry and design-system components instead of raw icon libraries.
- Prefer compact, responsive layouts over a narrow centered column on desktop.

## Planning

Planning files are split by audience:

| File | Audience | Versioned |
|------|----------|-----------|
| `ROADMAP.md` | Public strategy | Yes |
| `.stillmark/plans/TASK_PLAN.md` | Private execution plan | No |
| `.stillmark/plans/PROGRESS.md` | Private status | No |
| `.stillmark/plans/FINDINGS.md` | Private technical notes | No |

Update `ROADMAP.md` through the Stillmark CLI instead of editing it by hand.

The CLI is installed from the [latest Stillmark release](https://github.com/callmeteus/stillmark/releases/latest):

```bash
yarn install -g https://github.com/callmeteus/stillmark/releases/latest/download/stillmark.tgz
```

Stillmark does not ship an OpenCode adapter yet. Install the agent skill manually:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.config\opencode\skills\stillmark"
Copy-Item -Recurse -Force "<stillmark-repo>\templates\cursor\.cursor\skills\stillmark\*" `
    "$env:USERPROFILE\.config\opencode\skills\stillmark\"
```

Copy `templates/cursor/.cursor/commands/stillmark.md` to `$env:USERPROFILE\.config\opencode\commands\stillmark.md` and append `$ARGUMENTS` at the end.

## Commits and pull requests

- Write commit messages and pull request descriptions in English.
- Keep commits focused. Include tests, documentation, and the code that needs them.
- Describe the user-visible effect, the safety boundary, and the verification you ran.
- Do not include generated build output, VM images, proprietary binaries, or local plan files.
- Link the related issue or roadmap item when one exists.

## Security reports

Do not open a public issue for a vulnerability in Long Live 10 itself. Use GitHub's private security advisory flow for the repository. Include the affected version, reproduction steps, and the impact you observed. Do not include secrets or third-party proprietary files.

Reports about Microsoft products should go to Microsoft. This project can document a mitigation, but it cannot replace a vendor fix.

## Code of conduct

Be direct and technical. Harassment, personal attacks, and disclosure of private information are not acceptable. Maintainers may close changes that ignore the support matrix or the safety rules above.

## License

Contributions are accepted under the MIT License used by the repository. By opening a pull request you confirm that you have the right to submit the change.
