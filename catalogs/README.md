# Catalog

Versioned catalog data consumed by the agent, updater, and compatibility tooling.

## Layout

| Directory | Contract |
|-----------|----------|
| `target-builds/` | `TargetBuild` |
| `patches/` | `PatchEntry` |
| `artifacts/` | `Artifact` |
| `validation-results/` | `ValidationResult` |

Directories are created as entries are added. The validator scans recursively, so nested grouping is allowed as long as each file matches one of the contracts.

## Validation

From the repository root:

```powershell
npm run validate:catalog
```

Pass one or more files or directories to validate a subset:

```powershell
npm run validate:catalog -- catalogs/target-builds
```

The command exits with a non-zero status when any file is malformed or fails semantic validation. Every contract carries a schema version and is validated through `catalog-schema`.

## Rules

- Do not commit secrets, private download URLs, or license keys.
- Artifact sources must use HTTPS.
- Every patch must reference at least one CVE and one target build.
- Validation results must include at least one check and a consistent overall status.
- Fixtures and synthetic examples belong in `packages/catalog-schema/tests/fixtures/`, not here.
