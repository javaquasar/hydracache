# HydraCache 0.75 Extended IMap foundation evidence

This directory is provisional, test-only foundation work. It does not enable a production
distributed IMap capability, allocate wire or durable identities, or close W0 before 0.74 is
published.

Run the bounded authority model and validate its exact-source receipt:

```powershell
cargo xtask imap-value-plane-model --release 0.75 --seed 117 --output target/imap-075/model-receipt.json
cargo xtask imap-foundation-evidence-check --release 0.75 --receipt target/imap-075/model-receipt.json
```

Execute all locally available foundation probes, write the thirteen canonical receipts into a new
directory, and validate the set as one exact-source unit:

```powershell
cargo xtask imap-foundation-evidence-generate --release 0.75 --seed 117 --output target/imap-075/foundation
cargo xtask imap-foundation-evidence-check --release 0.75 --receipts target/imap-075/foundation
```

The generator runs the bounded authority explorer, deterministic response-loss/replay probe,
128-step seeded stateful chaos campaign, linearizability oracle, acknowledged-owner-loss/RPO probe,
and replayable transfer/listener/bulk/lifecycle/ACK/security/surface/explorer/Java proofs. The
testkit also sweeps deterministic
three-node schedules for proxy routing, replication
proof, promotion, partition-scoped repair/rebalance, epoch catch-up, partial bulk and listener
overflow, plus executable guards for every threat in `security-contract.json`.

The provisional API also includes a test-only bounded collection/lifecycle tranche: `size`,
`is_empty`, resumable `contains_value_bounded`, `clear_detailed`, `destroy`, `evict`,
`evict_all_detailed`, and paged `key_set`, `values`, and `entry_set` views. Pages carry a snapshot
revision and reject stale cursors; bounded searches return `partial` instead of inventing absence;
collection-wide mutations preflight their item budget before changing visible state. These names
and semantics allocate no production wire or durable identity.

The next twelve local API items extend the same Rust and Java references with `get_or_default`,
byte-exact `contains_entry`, void `set`, a `get_entry_view` limited to value/version/remaining TTL,
identity-aware `try_put` and `try_remove`, explicit-entry `replace_all_detailed`, listener removal,
Rust futures, Java `CompletionStage`, lazy page iterators and a capability descriptor. Timed calls
use logical deadlines and distinguish `applied`, `not_applied` and `outcome_unknown`; retries retain
the admitted identity. Bulk replacement accepts at most 256 explicit entries, preflights aggregate
request/response bytes, preserves existing TTL and returns request-order receipts. Retained bulk
outcomes reject identity conflicts and dedup-capacity exhaustion explicitly.
Retained outcomes take precedence over cancellation or an expired retry deadline. If a retained
identity cannot be inspected, the Rust facade returns `outcome_unknown` rather than asserting that
an earlier invocation was never applied.
Iterator adapters retain at most one bounded page and fail on stale continuity instead of silently
starting over. The descriptor exposes planned surface support alongside disabled production
capability and explicit unsupported families. These additions introduced 35 provisional
operations; the three interface helpers are not extra wire operations.

The extension contract and its negative canaries are part of `imap-contract-check`. Rust tests are
`value_plane_collection_075`, `value_plane_subscriptions_075`, `value_plane_async_075` and
`value_plane_capability_075`; Java runs `ExtendedIMapApiTest` and `AsyncIMapFacadeTest` through the
same Maven coverage gate. Collection-wide clear/destroy/scan models remain local reference work;
production collection admission still needs its own threat/cost review and W0 closure. Bulk
replacement is still not distributed transactions.

The unified local IMap service now dispatches all 40 provisional operations into the
same collection store and retained identity ledger. Generation- and instance-scoped version tokens
support replace/remove/TTL conditionals; the read-only outcome query distinguishes pending,
completed and unknown without re-executing a request. Rich listeners have bounded opt-in values,
expiry/eviction/clear/destroy events and deterministic registration cleanup. Typed Rust/Java errors
retain their domain reason, and fixed-field statistics describe only local owners and counters.

Rust streams and Java Flow publishers fetch on demand, retain at most one bounded page and close
on cancellation, error or completion. The independent Rust/Java execution differential compares
outcomes, errors, values, TTL, versions, listener events and statistics after every step; it includes
an actual broken-Java canary and deterministic failure shrinking. Its TSV format is development
test input and assigns no production wire identity.

Run the standalone cross-language gate after the Java toolchain is available:

```powershell
cargo xtask imap-api-differential --release 0.75 --seed 117 --steps 128
```

The shared `imap-distributed-correctness` gate runs this comparison after Maven and also retains
the existing isolated authority/replication proofs. Local service and stream tests do not admit a
production backend or establish cluster statistics.

Run all locally available Rust distributed-foundation proofs and optionally retain their evidence
with one command:

```powershell
cargo xtask imap-distributed-correctness --release 0.75 --evidence target/imap-075/local-proof
```

Validate the operation/security contracts and the pinned Hazelcast source provenance:

```powershell
cargo xtask imap-contract-check --release 0.75
cargo xtask imap-hazelcast-source-check --release 0.75
```

Passing these gates means only that the foundation is internally consistent. Production routing,
backend activation, final protocol identities, performance claims, and release admission remain
blocked by the dependencies recorded in `status.json`.
