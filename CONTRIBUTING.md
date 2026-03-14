# Contributing to WorkGraph v4

## Toolchain

WorkGraph v4 targets:

- Rust stable
- Rust 2024 edition
- MSRV 1.85

Install the required toolchain with:

```bash
rustup toolchain install stable --component clippy --component rustfmt
```

## Workspace structure

The repository is a layered Rust workspace. Lower layers may not depend on higher layers.

- **Layer 0 — Foundation:** pure utilities and data types
- **Layer 1 — Kernel:** domain truth and storage logic
- **Layer 2 — Execution:** adapters, dispatch, triggers, connectors
- **Layer 3 — Transport:** network and synchronization
- **Layer 4 — Surface:** CLI, MCP, API, projections
- **Layer 5 — Integration:** optional integrations such as Obsidian sync and OpenTelemetry

All crates live under `crates/`, except for the binary at `bins/workgraph/`.

## Development workflow

Before opening a pull request, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Coding standards

- Keep crates small and cohesive.
- Prefer explicit wiring over hidden initialization.
- Document every public item.
- Avoid unsafe code.
- Use real filesystem-backed tests for storage and ledger behavior.
- Keep markdown and YAML outputs human-readable.

## Dependency management

- Add third-party dependencies only in the root `Cargo.toml` under `[workspace.dependencies]`.
- In member crates, reference shared dependencies with `workspace = true`.
- Respect the architectural layering when adding internal crate dependencies.
