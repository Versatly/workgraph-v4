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
- Split growing files early. If a file starts owning argument parsing, orchestration, rendering, and helpers, it should be broken up immediately.
- Apply the same rule to kernel crates: if one file owns models, IO, validation, querying, or verification together, split it before adding more behavior.

## Surface crate layout

Surface crates are part of the agent experience. Keep them intentionally modular.

- `wg-cli`
  - prefer `args.rs`, `app.rs`, `commands/`, `output/`, and `util/`
- `wg-mcp`
  - prefer separate modules for server wiring, resources, prompts, and tools
- `wg-api`
  - prefer separate route, service, and serialization modules

As a rule of thumb, once a surface crate has multiple commands or output modes, each command or surface concern should get its own module rather than expanding a single central file.

For agent-native surfaces specifically:

- treat machine-readable output as a real contract, not an afterthought
- keep human rendering as a view over the same typed result model
- include structured recovery hints and likely next actions where practical
- prefer discoverable capability surfaces (`capabilities`, `schema`, etc.) over forcing agents to scrape long help text

## Kernel crate layout

Kernel crates should keep domain types and operational logic separate.

- `wg-store`
  - prefer `document.rs`, `io.rs`, `query.rs`, and `validate.rs`
- `wg-ledger`
  - prefer `model.rs`, `reader.rs`, `writer.rs`, `hash.rs`, and `verify.rs`

When a kernel crate begins mixing public models, persistence, validation, query logic, or verification in one file, that is a signal to split it immediately rather than waiting for a future cleanup pass.

## Dependency management

- Add third-party dependencies only in the root `Cargo.toml` under `[workspace.dependencies]`.
- In member crates, reference shared dependencies with `workspace = true`.
- Respect the architectural layering when adding internal crate dependencies.
