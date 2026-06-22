# Contributing

Thanks for your interest in HydraCache.

## Early Project Guidance

- Prefer small, focused pull requests.
- Keep runtime semantics explicit.
- Avoid adding macro magic before the core runtime is stable.
- Update docs when changing architecture or public-facing behavior.

## Development

Recommended checks:

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
