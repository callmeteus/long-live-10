# Patch Core

Logic for applicability checking, operation planning, conflicts, controlled execution, validation and rollback.

## Principles
- Never execute arbitrary scripts
- Only explicitly permitted operations
- Always prepare recovery before changes
- Refuse if applicability cannot be determined safely
