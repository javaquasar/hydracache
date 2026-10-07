# Local scheduled/native instrumentation only

No numerical CLI, counting allocator, daemon or qualification launcher is provided.
Use the library's bounded `scheduled::run`, not the source-referenced legacy
`rate::run_open_loop`. All observations are non-promotable and not product claims.

See `docs/testing/performance/0.74/get-response-owner-scheduled-controls-design.md`
and its contract for exact latency/accounting, security and remaining-proof limits.
The standalone lock must be shared by off/on builds in any later sealed cohort;
never mix measurements from this lock with historical tool binaries.

From the repository root:

```powershell
cargo fmt --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --check
cargo test --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --all-targets --locked -- --test-threads=1
cargo test --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --all-targets --features get-owner --locked -- --test-threads=1
cargo check --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --all-targets --all-features --locked
cargo clippy --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --all-targets --all-features --locked -- -D warnings
```

Tests are semantic/instrumentation fixtures only, not performance evidence. No
current-source hosted CI or full workspace verification is implied by these checks.
