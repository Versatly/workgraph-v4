# Contributing

## Toolchain

- Rust 1.85+
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

## Development standards

- Keep crates small and single-purpose
- Add doc comments on public functions
- Prefer explicit wiring over hidden initialization
- Use tempdir-based tests for filesystem logic
