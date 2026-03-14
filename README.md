# WorkGraph v4

WorkGraph v4 is a Rust context graph and coordination daemon for AI-native companies.

## Architecture

- Layer 0: Foundation crates (`wg-types`, `wg-error`, `wg-paths`, `wg-fs`, `wg-encoding`, `wg-clock`)
- Layer 1: Kernel crates (`wg-store`, `wg-ledger`, `wg-registry`, plus stubs for additional kernel modules)
- Layer 2-5: Execution, transport, surface, and integration crates are scaffolded for Phase 0
- Binary: `bins/workgraph` wires the CLI entrypoint

## Phase 0 Goals

- Filesystem-first markdown primitives
- Immutable JSONL ledger with hash chain verification
- Built-in primitive type registry
- CLI commands: `init`, `status`, `create`, `query`, `show`
