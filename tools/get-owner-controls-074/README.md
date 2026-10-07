# Private GET owner local phase B0 controls

Standalone tooling, not product code or a qualification replacement. The contract
is `docs/testing/performance/0.74/get-response-owner-phase-b0-contract.toml`.
Build four release variants from one clean sealed commit with separate target
directories: default, `get-owner`, `allocation-profile`, and
`allocation-profile,get-owner`. Always use this tool's `--locked` lockfile.

The timing binary has no global counting allocator, loadgen dependency, socket
counter wrapper or per-stage timer. Product profile collection is disabled.
The allocation binary compiles the historical tool-only System allocator by
source reference, without modifying it. Its CPU/latency/goodput are diagnostics,
not timing evidence. Unprofiled gross bytes are null, not a zero-allocation claim.

The runner seals source, compiler, lock, contract, four binaries and all 400
attempts before work. A READY/GO handshake applies CPU placement before warmup.
Every fresh process preloads deterministic values, warms up, runs the fixed
workload and verifies response/final bytes. Both builds have the same key mapping,
payloads, counts, no-TTL semantics and production clocks. Native controls use
Redis-shaped structured keys; do not extrapolate to all native workload shapes.

RESP uses real unwrapped loopback TCP with NODELAY, RESP2 plaintext and one
connection per logical client. Timed goodput/CPU includes connection/task setup,
but excludes corpus/preload/warmup/receipt work. Latency measures a closed-loop
exchange (including read/write/flush) at p1, or the **whole pipeline batch** at p50.
Neither is scheduled per-operation latency. ClientSurface dispatch is synchronous;
two fixed runtime workers and explicit bounded yields prevent task monopolization.
Concurrency 8 is eight logical tasks, not eight dedicated CPU workers.

This is only a local early guard: independent HC1/HC2, matched mTLS/RESP3,
concurrency 32/128, scheduled latency, miss/error/slow-reader/transition cohorts,
allocator active/resident/retained and timed idle/refill remain required. The
profiling allocator reports requested layouts, not allocator retention or RSS.
No accepted proposal, integrated C74, ship-ready gate or expensive run results.

Tests: `cargo test --manifest-path tools/get-owner-controls-074/Cargo.toml
--all-features --locked -- --test-threads=1` (allocator tests are serialized).
Also test default and `get-owner`, check all targets/features, and strict clippy.

From the repository root, after committing instrumentation and checking clean Git:

```powershell
cargo build --release --locked --manifest-path tools/get-owner-controls-074/Cargo.toml --target-dir target/get-owner-b0-timing-off
cargo build --release --locked --manifest-path tools/get-owner-controls-074/Cargo.toml --features get-owner --target-dir target/get-owner-b0-timing-on
cargo build --release --locked --manifest-path tools/get-owner-controls-074/Cargo.toml --features allocation-profile --target-dir target/get-owner-b0-allocation-off
cargo build --release --locked --manifest-path tools/get-owner-controls-074/Cargo.toml --all-features --target-dir target/get-owner-b0-allocation-on
python scripts/perf/performance_get_owner_controls_074.py --timing-off target/get-owner-b0-timing-off/release/get-owner-controls-074.exe --timing-on target/get-owner-b0-timing-on/release/get-owner-controls-074.exe --allocation-off target/get-owner-b0-allocation-off/release/get-owner-controls-074.exe --allocation-on target/get-owner-b0-allocation-on/release/get-owner-controls-074.exe --output target/get-owner-b0-NEW-SHA
```

Use a fresh packet name containing the actual source SHA, not a repeated output.
Do not edit the sealed source/contract, compile during sampling, or rerun after
invalidation. Retain the complete or invalid packet before any new measurement
decision. A complete packet replays with `--replay <packet>`; it cannot promote.
