# Catalogs

## Compatibility catalog

The compatibility catalog lists devices, drivers, and software with known compatibility status for supported Windows builds.

- Location: [`packages/compatibility-catalog/`](../../packages/compatibility-catalog/)
- Validation: `yarn validate:catalog`
- Schema: [`packages/catalog-schema/`](../../packages/catalog-schema/)
- Entries: [`packages/compatibility-catalog/entries/`](../../packages/compatibility-catalog/entries/)

## Patch catalog (planned)

A security patch catalog will track provenance, sources, affected builds, validation status, and rollback steps. Until then, patch entries are described in the schema.

## Validation results

Validation results are produced by `compatibility-validate` and should be recorded in CI when available. Evidence for validated mitigations follows the [patch validation standard](../patch-validation-standard.md).

## Contributing

Add entries following the schema and include sources with retrieval dates. Run validation before committing.
