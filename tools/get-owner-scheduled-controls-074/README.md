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

RESP2 GET now has a bounded per-response sidecar on one real loopback TCP
connection, with outstanding ceilings 1/10/50 (not fixed-size response-paced
batches). Cancelled callers retain FIFO tombstones until their reply/close.
`RespControl::run` is single-use, shares the original calendar with the driver and
includes wire-owner drain in elapsed goodput accounting. It reports frame-observed
timestamps separately from byte-validated operation completion; neither is batch
time divided by depth. RESP3/mTLS and sealed numerical execution remain absent.

Native 32/128 fixtures synchronize GET/PUT clients with a barrier, verify real HC2
connection accounting, exact values and shutdown. They assert no real-clock
performance threshold or native nonregression.

`start_connections` additionally supports 1/8/32/128 actual accepted TCP sockets
and GET or SET of the same fixed preloaded payload. Each socket owns its FIFO,
parser, permits and local wire ordinal; `(connection_id, wire_ordinal)` is the
wire identity. Sequence modulo socket count fixes the route before any response.
The fixture store is shared within a control, and setup/final GET byte oracles
check it through every connection. SET success checks exact `+OK`; final GET
checks retained bytes. No global socket ordering or conflicting-write oracle is
claimed. Wire drain has one five-second group budget. This remains semantic-only,
with no RESP3/mTLS, multi-key adapter, numerical series or B0 retry.
