# ADR-001: Technology Stack Selection

## Status
Accepted

## Context
Need to select technologies for a monorepo that will:
- Run native operations on Windows (patching, system operations)
- Need memory safety and security
- Support VM automation
- Have catalogs/schemas as data
- Be distributed simply

## Decision
- **Core/native components (agent, patch-core)**: Rust - memory safety, no runtime, good Windows API support, single binary distribution
- **VM harness/automation**: Rust or Go - both fine; start with Rust for consistency
- **Catalog schemas/validation**: TypeScript/Node.js for flexibility and easy JSON schema validation, or Rust with serde
- **CLI tooling/scripts**: Node.js/TypeScript for cross-platform scripts
- **Build system**: Cargo workspace for Rust packages, npm workspaces for Node tooling if needed

Start with Rust as primary for performance-critical/sensitive components, Node.js for tooling. Use a simple monorepo structure under packages/ with language-appropriate build configs.

## Rationale
- Rust matches "security of memory, compatibility and distribution simple" requirements
- All tools available in environment
- Easy to create self-contained binaries

## Consequences
- Mixed language monorepo - need clear boundaries
- Need both cargo and node toolchains
