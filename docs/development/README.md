# Development

## Quick Start
`ash
# Build all packages
cargo build

# Run tests
cargo test

# Build specific package
cargo build -p catalog-schema
`

## Safety Guidelines
- Never test destructive operations on the host
- Use VM harness for all Windows system modifications
- Always prepare rollback before applying changes
