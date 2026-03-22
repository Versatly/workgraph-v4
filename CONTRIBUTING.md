# Contributing to WorkGraph v4

## First Principle

WorkGraph is a durable semantic system. Documentation and code contracts must move together.

Before changing coordination, graph, actor, or trigger semantics:

1. read `docs/foundation.md`
2. read `docs/context-graph.md`
3. read `docs/operating-model.md`
4. update those docs in the same turn if your code changes their meaning

## Toolchain

WorkGraph targets Rust stable, Rust 2024 edition, and MSRV 1.85.

```bash
rustup toolchain install stable --component clippy --component rustfmt
```

## Validation

Run this before opening a pull request:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Architectural Rules

- Respect crate layering. Lower layers may not depend on higher layers.
- Keep `wg-types` as the durable source of shared contracts.
- Keep kernel crates factored so models, validation, persistence, and query logic do not collapse into one file.
- Keep surface crates factored so parsing, execution, rendering, and discovery remain separate concerns.
- Prefer explicit wiring over hidden initialization.
- Avoid unsafe code.
- Use real filesystem-backed tests for storage and ledger behavior.

## Semantic Rules

- Do not reduce WorkGraph into a task tracker or generic memory system.
- Do not let “context graph” drift back into a fuzzy wiki-link graph.
- Do not add trigger behavior that bypasses durable event and action-plan semantics.
- Do not let thread completion become a plain status flip once exit criteria and evidence exist.
- Do not introduce new actor identity systems when `ActorId` already serves as the stable logical actor reference.

## Contributor Expectations

- When adding a new primitive contract, register it in `wg-types` and expose it through discovery surfaces where appropriate.
- When changing CLI JSON results, bump and preserve the machine-readable envelope intentionally.
- When changing a coordination primitive, update tests that prove persistence, round-tripping, and rendering.
- Keep markdown artifacts readable enough that a human or agent can inspect the workspace directly without proprietary tooling.
