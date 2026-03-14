# WorkGraph v4

**WorkGraph v4 is the Rust-native context graph and coordination daemon for AI-native companies.**

It stores what other systems miss: the institutional context, decision traces, policies, and relationships that let humans and AI agents coordinate without losing the thread.

## Phase 0 scope

This repository currently implements the Phase 0 foundation described in `AGENTS.md`:

- A Rust workspace monorepo with layered crates
- A markdown-native workspace layout for primitives such as `org`, `client`, and `decision`
- A JSONL ledger with hash-chain verification
- A clap-based CLI for workspace initialization, creation, querying, and inspection

Future phases will add adapters, triggers, transport, MCP, and API surfaces on top of this core.

## Architecture

```text
Layer 0  Foundation  -> wg-types, wg-error, wg-paths, wg-fs, wg-encoding, wg-clock
Layer 1  Kernel      -> wg-store, wg-ledger, wg-registry, wg-thread, wg-mission, wg-graph
Layer 2  Execution   -> dispatch, adapters, triggers, connectors
Layer 3  Transport   -> transport, federation, networking, signaling
Layer 4  Surface     -> CLI, MCP, API, projections, markdown views
Layer 5  Integration -> Obsidian sync, OpenTelemetry
```

The storage model is intentionally filesystem-first:

- each primitive is stored as a markdown file with YAML frontmatter
- directories are organized by plural primitive type names such as `orgs/`, `clients/`, and `decisions/`
- every mutation is recorded in `.workgraph/ledger.jsonl`

## Quick start

```bash
# Build the binary
cargo build --release

# Initialize a new workspace
./target/release/workgraph init

# Create primitives
./target/release/workgraph create org --title "Versatly"
./target/release/workgraph create client --title "Hale Pet Door"
./target/release/workgraph create decision --title "Rust for WorkGraph v4"

# Inspect the workspace
./target/release/workgraph status
./target/release/workgraph query org
./target/release/workgraph show org/versatly
```

## CLI commands

The Phase 0 CLI currently supports:

- `workgraph init`
- `workgraph status`
- `workgraph create <type> --title "..." [--field key=value]`
- `workgraph query <type> [--filter key=value]`
- `workgraph show <type>/<id>`
- `--json` output for all commands

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

See `CONTRIBUTING.md` for workspace conventions and crate layering rules.

## License

Licensed under either of:

- Apache-2.0
- MIT
