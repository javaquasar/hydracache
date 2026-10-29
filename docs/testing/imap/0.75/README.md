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

The next lifecycle helpers operate on that same local owner without adding wire operations:

- client/generation-scoped retirement frees completed outcomes only behind a monotonic fence;
  pending work is protected, and a retired identity cannot be admitted again;
- aggregate logical payload budgets cover values, outcomes, pending requests, listeners, stream
  buffers and repair snapshots, with admission checked before visible mutation;
- listener repair captures a bounded snapshot and watermark atomically before live delivery;
- version-conditional bulk returns ordered per-item results, and bounded batch outcome lookup
  preserves exact pending/completed/unknown certainty;
- expiry maintenance examines a bounded number of entries per step while expired values stay
  invisible before physical cleanup;
- quiescent checkpoint/restore is an in-memory test helper, retains TTL/outcomes/retirement fences
  and creates a fresh map instance so old instance tokens and cursors cannot cross restoration.

These budgets describe owned logical payload, not process RSS or a production memory guarantee.
Listener repair provides snapshot/cutover rather than a durable event history. Bulk operations
retain their per-item semantics and do not provide distributed transactions. Checkpoints assign
no persisted format, wire version or production durability identity.

The shared differential gate also executes real worker-thread schedules in the Rust and Java
services. Its independent checker accepts legal winners consistent with invocation/return edges,
validates async cancellation/recovery and listener effects, and rejects a deliberately broken Java
history. The development schedule format and deterministic shrinker remain bounded.

The release-preparation tranche composes actual `UnifiedIMapService` owners with the bounded
loopback fault transport. Owner decisions and retained receipts are installed on backups without
re-evaluating conditional commands. Cross-partition bulk has ordered per-item receipts and a
bounded coordinator journal backed by another service ledger. An unproved/pending coordinator
blocks promotion. Replica images preserve the logical map incarnation, absolute TTL, versions,
receipts and retirement fences; these runtime images differ from fresh-instance test checkpoints.
All 40 operations have an explicit cluster scope. Map-wide operations are limited to a single
partition until a separate cross-partition contract exists; unsupported cases fail explicitly.

The authenticated registry connects actual service owners to bounded fair admission and namespace
lifecycle. Queued work, pending identities, streams and snapshot leases contribute real ownership
to drain/reclamation. A deleted generation fences old handles before reuse; registry destruction
uses its reserved safety lane. Registry callbacks run outside its metadata lock.

Capability audit separates the 21-operation bounded reference, the 40-operation unified reference
and seven local lifecycle families, and the existing SDK's eleven capability tokens. The existing
SDK keeps its narrow production behavior; these descriptors do not activate the 0.75 backend.
Migration fixtures exercise bounded staging, codec/TTL/count/digest checks and cold repopulation
dry-runs. See `docs/integrations/hazelcast-to-hydracache-0.75.md`.

```powershell
cargo xtask imap-capability-audit --release 0.75
cargo xtask imap-migration-dry-run --release 0.75
cargo xtask imap-ci-check --release 0.75 --mode pr
cargo xtask imap-comparison-check --release 0.75
cargo xtask imap-release-admission --release 0.75
```

The pinned CI workflow defines Rust 1.88/1.94 and Java 17/21 lanes: one two-seed shard per PR,
four disjoint shards per nightly matrix. Successful receipts are written only after actual local
proof/tooling execution with unchanged clean source. The aggregate gate rejects missing, failed,
mixed-source or undeclared lanes. Failure diagnostics are kept separately from matrix receipts.
Scheduled/default-branch execution and remote matrix results remain separate from local tests of
the workflow contract.
On Windows, nested tooling tests build in `target/imap-075/tooling` to avoid replacing the running `xtask.exe`.

The integrated runtime additionally connects `new_clustered` namespace registries to the
actual proxy/owner/replica services. Held admission permits bind tenant, affected partitions
and keys; authenticated request, replica-apply and ACK contexts are checked against configured
member credentials, generations, trust epochs, replay identity and original deadlines.
This uses the existing member identity/authorization interfaces in a test channel and makes
no mTLS or real-daemon authentication claim. Forged ingress is refused before admission;
an invalid replica or ACK cannot prove a mutation that an owner may already have applied.

Typed cluster subscriptions own real per-owner listener registrations. Partition cut vectors,
generation changes, explicit gaps and bounded repair describe continuity; no global event
order is implied. Prefix and optional exact-key filters, including binary and empty keys,
share the local Rust/Java contract. Coordinator streams keep their own bounded buffers and
release registrations on close, auth failure and namespace fencing.

Online transfers keep the source serving while bounded snapshot chunks and ordered sparse
owner decisions reach staging. A bounded cursor retains key/record/receipt fingerprints and
fences; deltas carry only changed stored records, deletes, original outcomes, retirement fences
and authoritative TTL/version metadata. Borrowed preflight measures the sparse payload before
cloning. Checksum/order/resume and source/staging stamp
equality fence final cutover. Pending roots pause unsafe cuts; exhausted count/byte credits
backpressure before source mutation and resume after delta application. Epoch/corruption
failures require explicit restart; a confirmed source write is never silently dropped.

The request-evolution tranche keeps one immutable Java `RequestSnapshot` from validation through
digest/admission and execution; builders remain caller-thread-confined. Held cluster permits bind
namespace generation, canonical request, original deadline and grant. Physical attempt leases
retain credit even if their permit is dropped; overlapping or changed requests fail explicitly,
while exact retries use the retained outcome.

Optional strict namespace policy uses bounded immutable Read/Write/Listen/Admin grants, scoped
keys and policy epochs. Attenuation cannot widen rights. Policy changes revoke queued and
buffered access, including listener values, and wake outside dispatcher/registry locks. Legacy
test contexts work only while strict policy mode is absent; no policy context falls back to them.
Authenticated replay windows have bounded lanes and irreversible lower fences; old nonces cannot
become fresh after memory retirement or trust rotation. Fixed health fields report replay usage.

Cluster invocation futures expose real queue/reserved/dispatch checkpoints. Reservation writes
and backs up the actual root journal before reporting Reserved. Cancellation before admission
is NotApplied for a fresh identity; retained proved results take precedence on a cancelled retry.
After possible owner decisions it preserves Unknown or a retained result backed
by every required data/coordinator proof. Pending journal image ports retain reservation credit;
ordinary data/checkpoint and online cutover paths still refuse uncompleted roots.

Rust and Java typed async helpers cover version bulk, ordered outcome lookup, retirement,
incremental expiry, listener snapshots and checkpoints. They share actual service ledgers and
bounded ownership rather than another value map. Retried snapshots retain their original cut
without clearing newer events; undelivered leases close on cancellation/drop. These are local
helper types, not expanded production SDK operations.

The common Java oracle additionally accepts bounded typed v2 scenarios. Its independent model
checks token equality/freshness/instance scope, mixed per-item bulk outcomes, lost-response
recovery, pending/cancelled roots, retirement, resync/restore, actual owner counts and event
causality. The actual adapter keeps only real handles and token references. Corrupted result
ordering, TTL, events, tokens, time and ownership must fail independent negative controls.
These sequential local scenarios supplement the concurrent/cluster history checkers; they do
not qualify production transport, global iteration or durable recovery. Fingerprint v2 and
explicit legacy migration are registered in `docs/COMPAT.md`.

Data, safety and repair lanes have separate permits/bytes and per-lane tenant fairness.
Multi-partition commands reserve every affected partition and key before dispatch. Replica
credits stay owned until actual work completes. Runtime readiness/drain observations derive
from actual owner reachability, backup proofs, transfers and retained owners; production
capability remains disabled even when the test runtime reports ready.

The actual-cluster campaign stages apply, replication, ACK and client response by retrying the
same retained identity through deterministic transport gates. It checks real service receipts,
proved replica snapshots, bulk ordering/version stability, expiry after promotion and an
independent real-time history oracle. Search-budget exhaustion is distinct from a proved
violation; negative observation canaries and the bounded shrinker test the checker itself.
These are logical schedules, not process-kill, real-network or durable-restart qualification.
Failures retain bounded original/minimized schedules, seed, source identity when available and
explicit local/fixture scope under `target/imap-075/cluster-failures`; prior attempts are never
overwritten. Replay a retained schedule with `imap-cluster-chaos --release 0.75 --seed <seed>
--schedule <workspace-relative-json>`.

```powershell
cargo xtask imap-cluster-chaos --release 0.75 --seed 117 --rounds 3
cargo xtask imap-semantic-audit --release 0.75
cargo test -p hydracache-fuzz --test imap_cluster_075 --locked
```

The dedicated `fuzz_imap_cluster_075` target runs a retained hostile corpus in normal tests
and bounded mutation fuzzing in the existing scheduled fuzz job. Corpus/artifact retention,
input/time limits and failure propagation are guarded. Scheduled execution is distinct from
local corpus replay. CI receipt v2 executes this campaign for each disjoint seed and retains
the local Rust/Java differential. The provisional method ledger and actual-owner Java semantic
adapter are described in `docs/integrations/imap-compatibility-0.75.md`; borrowed expectations
are adapted source-pinned cases, not an execution claim for the official Hazelcast product.

The comparison contract validates identical backup, persistence, TLS, TTL, previous-value, retry
and listener semantics, paired-run floors, balanced order and retention of negative results.
Unresolved product digests block readiness. S2 also requires a proved backup and qualified real
cluster producer. No numerical result or public claim is generated by this validator.

Release admission validates complete exact-source production manifest structure, nested digests,
operation/scenario coverage, zero skipped cases, predecessor tag, clean candidate source and open
production dependencies. Foundation/model/CI/dry-run receipts cannot substitute for production
receipts. The production producer verifier adapters do not exist yet, so `--require-ship` remains
blocked even for structurally valid synthetic manifests. Positive fixture tests validate the
checker, not real product qualification.

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

## Clustered API evolution

The same authenticated registry now offers cooperative read futures without mutation-root
reservations, actual-cluster typed version bulk/outcome/retirement/expiry helpers and bounded
opt-in mutation retries. Retry attempts retain the original command identity, admission receipt
and deadline; an explicit expected epoch is never refreshed. Namespace policy epochs revoke
queued calls, active attempts and streams in that namespace without cancelling another map.
The global revocation API remains available and must advance beyond every effective epoch.

Repair and online drain can advance bounded chunks/deltas under Repair-lane permits and retain
their staging/progress between steps. Delta count and byte credits are reserved before owner
or journal mutation. Applying acknowledged deltas frees credit for a later foreground retry;
epoch changes or corrupt state still require explicit restart. Pending roots block an unsafe
snapshot cut. Invocation diagnostics page real phases, remaining deadlines, owned credits and
fixed blocking reasons; opaque cursors bind scope, generation, policy and observed state. They
require Admin authorization and expose no key/value/client/tenant credentials.

The shared v2 corpus executes all 23 extended actions on actual Rust and Java services against
the independent Java model, including pending roots, stale tokens, ordered mixed bulk results,
retirement, listener leases and restore. Normal correctness/CI runs include this check and eight
real process fault controls. DTO preflight, process time/output bounds, closed JSON shapes and
bounded reference-preserving shrink/replay are mandatory. Retained scripts live under
`target/imap-075/differential-failures` and are content-addressed without overwriting conflicts.

```powershell
cargo build -p hydracache-cluster-testkit --bin imap-extended-owner-075 --locked
cargo xtask imap-extended-differential --release 0.75 --seed 117
cargo xtask imap-extended-differential --release 0.75 --script target/imap-075/differential-failures/<retained-script>.tsv
```
