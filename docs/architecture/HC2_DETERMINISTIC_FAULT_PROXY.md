# HC/2 Deterministic Fault Proxy

## Status and scope

H19 adds a replayable, bounded byte-stream fault scheduler to the
non-production `hydracache-client-plane-spike` crate. It is test infrastructure,
not a production listener or a network emulator. H11 and H20 now consume
retained fault traces, but this document is not evidence that H03 is complete.
Direct dedicated-TCP, HTTP/2, and gRPC loopbacks remain the control group.

The proxy addresses a specific evidence gap: ordinary loopbacks cannot
reliably reproduce fragmentation, delayed bytes, half-open directions, resets,
or a close at an exact stream offset. A failing case must therefore identify
the exact seed, direction, action list, input shape, and privacy-safe trace.

## Deterministic contract

`FaultPlan` is the complete input to the scheduler:

- schema and bounded case ID;
- `u64` seed and one explicit direction;
- maximum buffered bytes and maximum trace events;
- at most 64 ordered actions.

The same plan and the same logical input chunks produce the same delivery
ticks, chunk boundaries, terminal state, and hashes on every supported host.
The retained replay format regenerates synthetic input from the seed and chunk
sizes. It stores no raw payload bytes.

| Action | Semantics |
| --- | --- |
| `pass` | preserve chunks and bytes |
| `fragment` | seeded split into non-empty chunks no larger than the bound |
| `coalesce` | join at most N adjacent chunks while preserving byte order |
| `delay` | add logical delivery ticks |
| `reorder_adjacent` | swap adjacent logical packets only; never reorder bytes inside a packet |
| `duplicate` | emit a bounded number of extra packet copies |
| `drop` | drop every Nth logical packet |
| `block_direction` | deliver nothing and record `blocked` |
| `half_open` | retain scheduled bytes and record `half_open` |
| `reset` | discard pending bytes and record `reset` |
| `late_delivery` | add ticks, including after a half-open transition |
| `bandwidth_pressure` | split by bytes/tick and insert a deterministic window-boundary pause |
| `close_after_bytes` | retain the exact stream prefix and record `closed_after_bytes` |

Invalid zero bounds, excessive duplication, oversized buffers, excessive
actions/chunks/ticks, trace overflow, and arithmetic overflow fail closed.
Window pressure is strictly stream-order preserving; its unit test caught and
prevented overlapping tick assignments during H19 development.

## Trace and privacy boundary

Every action trace records only action parameters, chunk/byte counts, maximum
logical tick, and terminal state. Every delivery records sequence, tick, length,
and SHA-256. The top-level trace binds input/output lengths and SHA-256 values.
The replay artifact is limited to 64 KiB by `xtask`; the scheduler separately
enforces its byte and event budgets.

This is intentionally insufficient to reconstruct an application payload. A
future failing production-derived test must retain only a safe synthetic
reproducer, never credentials, certificates, keys, values, tenant names, or
raw frames from users.

## Executable evidence

`tests/fault_proxy.rs` proves:

- all thirteen action kinds and all terminal outcomes;
- exact same-seed replay and a changed seeded fragmentation schedule;
- bounded JSON and fail-closed plan/buffer/trace limits;
- half-open followed by deterministic late delivery;
- byte-preserving fragment/coalesce/delay/window pressure for every transport
  candidate, followed by successful decode of the same semantic frame;
- identical fail-closed truncated and duplicated-frame outcomes for every
  candidate;
- delivery through real Tokio async byte streams, not only a pure model;
- replay tamper detection.
- exact replay of all eight H11 recovery traces before the recovering-client
  lifecycle matrix exercises handshake fallback, restart, leader movement,
  subscription repair, invocation replay, session loss, duplicate suppression,
  and reconnect exhaustion.

The retained seed and reproduction policy are documented in
[`../testing/hc2-fault-proxy/README.md`](../testing/hc2-fault-proxy/README.md).

## Reproduction

```text
cargo xtask client-plane-fault-check
cargo xtask client-plane-fault-check --replay docs/testing/hc2-fault-proxy/h19-seed-1592590353.json
cargo xtask client-plane-fault-check --seed 1592590353 --output target/hc2-fault-traces/replay.json
cargo test -p hydracache-client-hc2 --test reconnect_repair --locked
cargo xtask client-plane-spike-check
```

The first command replays the retained trace exactly and runs the focused proxy
tests. The combined spike gate also verifies the retained trace, while its full
crate test run keeps the three direct loopbacks as controls.

## Remaining integration boundary

H19 is `in progress`, not complete. H20 binds client half-close,
uncooperative-peer deadline, reset, and forced-close behavior to retained fault
plans. H11 now binds its complete reconnect and repair matrix. H03 still needs
to bind every concrete transport-candidate lifecycle failure before H19 can
reach `complete`.
