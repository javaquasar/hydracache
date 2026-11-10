# HydraCache 0.77.0 RESP Decoding, Store Sharding, and Durable Group Commit Plan

> **At a glance**
>
> - **What** — test a decoder that reads RESP frames directly from the connection buffer, independent locks for the measured client-store shards, and group commit for acknowledged durable writes.
> - **Why** — the current code copies unread input on each decode attempt. Historical profiles also found a contended client-store mutex. A durable write path introduces a further cost when each operation requires a separate disk sync.
> - **After** — published `0.76.0` and its admitted `0.75` data authority. Use published `0.76.0` as the product baseline. Reproduce historical `0.74` findings against the new source before selecting candidates.
> - **Unblocks** — higher throughput for verified RESP workloads and higher durable write capacity within the same acknowledgement contract.
> - **Status** — planned. Each optimization needs its own correctness and performance evidence. The durability profile needs end-to-end recovery proof before it can accept traffic.

Read the [repository entry point](../../CLAUDE.md), [rules](../RULES.md), [gates](../GATES.md), [compatibility register](../COMPAT.md), and the published predecessor evidence. The release inherits R-1 through R-11. The [release manifest](releases.toml) and [index](INDEX.md) define sequencing.

## Release boundaries

The three workstreams target different costs. Decoding changes how the server obtains one validated command. Store sharding changes how verified operations acquire access to local records within an existing authority. Group commit amortizes durable I/O across independently admitted writes. Each can be measured and reverted independently, subject to the storage-format rules for a durable namespace.

The RESP2/RESP3 command and error contract, request and response order, tenant isolation, mutation visibility, quota accounting and supported TTL behavior are compatibility gates. The existing delivery rule applies across all three workstreams. The connection finishes the preceding reply's `write_all` and `flush` before dispatching its next command. That completion refers to the server's writer, with durable commit established separately for a successful durable mutation. [RESP contract](../integrations/redis-compat.md) · [Historical delivery proof](#historical-artifacts)

The `0.75` owner-routing and cross-surface contract remains the integration seam. In `0.77`, RESP, HC/1 and HC/2 operations that address the same configured namespace use the same admitted backend. A synchronous in-memory backup apply and an acknowledgement backed by disk are distinct states. At W0, trace the published predecessor from dispatch through commit and recovery, then bind the plan to that implementation. The current `0.75` local reference and abstract durability model supply fixtures and oracles. A production claim requires daemon receipts. [IMap plan, W10](V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#w10-complete-the-distributed-hc2-imap-value-plane-and-its-safety-net) · [IMap provisional ledger](../integrations/imap-compatibility-0.75.md) · [0.76 authority contract](V0_76_SEMANTIC_CACHE_AND_VECTOR_INDEX_PLAN.md#inherited-075-boundary)

R-2 applies to multi-key operations. A local sharded store must preserve its existing atomic `MSET` behavior. The new durable profile admits an atomic batch only within one existing authority and one log. A batch spanning authority groups is rejected as a whole before mutation using the registered surface error. Partition-grouped bulk APIs keep their explicit partial outcomes. The profile is opt-in and scoped to a single region and one data group per admitted namespace in the first release. The default cache, `hydra-moka`, the Redis command catalog, and the `0.76` ANN implementation are outside the optimization scope.

## Design inputs and existing evidence

| Mechanism | What the source establishes | Consequence for `0.77` |
| --- | --- | --- |
| KiviDB parses a connection buffer using a moving offset. | The worker dispatch loop advances `parse_offset` through already received input. [KiviDB 1.2.0 network loop](https://github.com/kividbio/kividb/blob/v1.2.0/src/server_uring.rs#L803-L839) | Isolate the input copy in the HydraCache decoder before reconsidering compaction or a new transport. |
| W2 removed input suffix movement. | Pipeline-10 moved bytes fell to zero. Pipeline-1 GET CPU/op rose 4.30% and SET goodput fell 2.45% in the final local pairs. [W2 receipts and decision](../testing/performance/0.74/w1-local-attribution.md#w2-candidate-and-negative-result) | Preserve those negative results. W2 of this plan changes decoder copying in isolation. |
| W3 batched replies. | Loopback TCP pipeline-10 goodput improved about 81% for GET and 83% for SET. Pipeline-1 guards failed. The adaptive attempt executed a second `SET` behind a blocked reply and failed before timing. [W3 TCP pairs](../testing/performance/0.74/w1-local-attribution.md#w3-real-tcp-attribution-and-bounded-batch-retry) · [Adaptive result](../testing/performance/0.74/w3-adaptive-baseline.md#candidate-disposition) | Keep the delivery order. Disk batching will combine work from independent connections at the storage boundary. |
| KiviDB uses independently locked in-memory shards. | Each shard has its own lock. HydraCache's historical c8 live GET profile recorded about 14.8 μs of store lock wait and 1.0 μs of hold per operation. The single global `RwLock` candidate regressed c8 GET goodput by 3.74% and raised CPU/op. [KiviDB shard code](https://raw.githubusercontent.com/kividbio/kividb/master/src/store/mod.rs) · [W6 attribution and rejection](../testing/performance/0.74/w1-local-attribution.md#w6a-live-read-and-expiry-attribution) | Re-measure wait by owner. Independent shard locks are a separate hypothesis. The historical result concerns one shared lock. |
| KiviDB amortizes writes to its AOF. | Multiple waiting connections share a write. In `1.2.0` the acknowledgement path normally waits for file write and the AOF syncs periodically. An AOF backlog can outlast its reply wait limit. [KiviDB 1.2.0 acknowledgement contract](https://github.com/kividbio/kividb/blob/v1.2.0/docs/PERSISTENCE.md#when-a-write-is-acknowledged) | Adopt amortization with a stronger barrier. A queued write, page-cache write, or expired wait budget cannot authorize durable success. [Raft persistence and commitment](https://raft.github.io/raft.pdf) |

The KiviDB sources provide design references. Implement the selected mechanisms in HydraCache and record dependency and license provenance with the release evidence. [KiviDB license](https://github.com/kividbio/kividb/blob/v1.2.0/LICENSE) · [HydraCache license](../../LICENSE)

### Historical artifacts

The following paths are pinned to the `0.74` development source `7fa33b0681b7908cfa2599696fe51b53d22c845d`. Read them with `git show <commit>:<path>` from a clone that contains the `0.74` history. Carry their source identities and any required receipts into W0. They document historical experiments and guard design.

- `docs/testing/performance/0.74/w3-delivery-frontier-review.md` enumerates partial-write and pending-flush cuts.
- `docs/testing/performance/0.74/w3-staged-execution-review.md` demonstrates failures of early execution and speculative replies under native mutation, quota and TTL races.
- `docs/testing/performance/0.74/w9c-linux-kernel-attribution.md` confirms one successful server write per reply in the measured Linux cells.
- `docs/testing/performance/0.74/release-admission-contract.toml` records the separation of finite, six-hour and 24-hour runs.

## Execution order and evidence

```text
W0 contract and baseline → W1 owner attribution
                             ├─ W2 decoder → W3 decoder proof ─────┐
                             ├─ W4 shard store → W5 shard proof ───┼─ W9 composition → W10 admission
                             └─ W6 durable baseline + W8 baseline faults
                                → W7 group commit → W8 grouped faults ─┘
```

W0 and W1 precede product changes. An affected path needs an isolated comparison before it enters the composed candidate. The durable write stream needs a valid batch-of-one implementation before the batching experiment. A missing predecessor, an unmeasurable owner or a failed experiment has a recorded disposition and supporting evidence. The release may publish a narrower scope with rejected improvements removed and their claims omitted. A requested durable mode refuses activation when the authority or durability gate is unavailable. [0.74 evidence discipline](V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md)

### W0. Freeze the predecessor and define the claims

**Implementation.** Pin the published `0.76.0` tag and artifact checksums, the source and lockfile of its data-plane dependency, build/toolchain, host/kernel, allocator, TLS settings, load-generator revisions and output schema. Record the actual implementations selected at server boot for each surface. Trace a plain write, a conditional mutation, an expired GET and an atomic `MSET` from public dispatch to store, quota, event, response and, where configured, durable proof. Record the acknowledged failure model, permissible retries and the existing ordering/visibility oracle for each baseline profile.

Create `docs/testing/performance/0.77/contract.toml` and a work-item evidence manifest during implementation. Give every cell a named workload, profile, test or oracle, expected result and evidence owner. Keep the prior published baseline separate from an instrumented baseline, a new batch-of-one durable control and the exact assembled release candidate. Freeze all comparison cells and effect thresholds before collecting candidate measurements. Admit candidate-specific configuration at boot and describe its format and downgrade behavior in `docs/COMPAT.md` before changing persisted or wire data. [Compatibility register](../COMPAT.md)

**Tests.** Add `crates/xtask/tests/performance_contract_077.rs` with `rejects_unpublished_predecessor`, `rejects_ram_ack_as_durable`, `rejects_mixed_candidate_receipts` and `rejects_missing_security_or_native_cell`. The guards validate plan-to-test mappings and fail on altered identity, dropped cells, unsupported surfaces or an inactive durable backend. Run `cargo test -p xtask --test performance_contract_077 --locked` and `cargo xtask doc-check`.

**Exit.** The predecessor is usable, every intended comparison has a matched contract, and receipts distinguish protocol success, replica apply, durable commit and client observation.

### W1. Measure the three costs on the published paths

**Implementation.** Extend the existing `hydracache-loadgen`, RESP stage profiler and daemon receipts. Record per-stage bytes and allocation requests, actual complete frames per read, attempts returning incomplete, input compaction, CPU/op and end-to-end goodput. For the store, separate shard/store, tenant admission and expiry lock waits and hold time. Measure direct dispatch and TCP separately. For durable writes, count records, appends, syncs and client-visible completions. Measure pending bytes, queue delay, and replicated, committed and applied indices. Keep expensive instrumentation out of the timing builds and use separate source-bound attribution runs. [0.74 RESP attribution](../testing/performance/0.74/w1-local-attribution.md) · [Historical Linux socket attribution](#historical-artifacts)

For `d` equally sized frames of `L` bytes returned in one read, the current full-suffix copy can copy `L × d × (d + 1) / 2` bytes over the `d` decode calls. Buffer compaction accounts for a separate `L × d × (d − 1) / 2` moved bytes. The formulas describe a scripted read boundary. Record real co-read depth over TCP and TLS before estimating the effect on a network workload. Fragmented input can also cause repeated copies before a frame is complete. The previous W4 single-command allocation share was 7.73% for GET decoding and 19.76% for SET decoding. Use it as a reason to distinguish shallow commands and buffered pipelines. [Current RESP decoders](../../crates/hydracache-redis-compat/src/lib.rs) · [W4 stage profile](../testing/performance/0.74/w1-local-attribution.md#w4-stage-attribution-and-no-candidate-decision)

**Tests.** Add `crates/hydracache-loadgen/tests/performance_attribution_077.rs` with `copies_and_compactions_have_distinct_counters`, `actual_read_depth_is_measured` and `replica_flushes_are_distinct_from_client_acks`. Use a deliberately repeated incomplete read and a deliberate early-ack count as counter canaries. Run `cargo test -p hydracache-loadgen --test performance_attribution_077 --locked`. Run identical-binary A/A with independently started processes on the admitted Linux host before selecting timing cells.

**Exit.** Each admitted hypothesis identifies a measured cost in the target profile and a counter that can falsify the proposed mechanism. For a backend without an admitted disk path, retain the gap as W6 work and collect sync attribution after the batch-of-one control exists.

### W2. Decode a command from the existing RESP input buffer

**Implementation.** Keep the current read loop, buffer removal and response writer in the first candidate. Add a private decode path in `hydracache-redis-compat` using the pinned `redis-protocol` range API. It accepts the supported plain-array frame forms from the current unread slice, validates shape and limits, and copies only the fields needed to construct the existing owned `RedisCommand`. Build `NeedMore`, complete and error outcomes with the same precedence and consumed length as the current decoder. Unsupported or ambiguous shapes use the canonical decoder so RESP3 attributes, aggregate semantics, numeric and UTF-8 conversions, and command errors retain their behavior. The existing public decode helpers remain the differential oracle until compatibility is proved. [Current decoder and frame conversion](../../crates/hydracache-redis-compat/src/lib.rs) · [RESP2 range API](https://docs.rs/redis-protocol/latest/redis_protocol/resp2/decode/fn.decode_range.html) · [RESP3 range API](https://docs.rs/redis-protocol/latest/redis_protocol/resp3/decode/complete/fn.decode_range.html)

The pre-decode `max_frame_bytes` check currently applies to the entire unread slice, including a suffix containing more than one frame. Apply it at the same point. Preserve element, bulk and nesting limits and the first invalid frame that ends dispatch. Re-evaluate the dialect after each `HELLO`, and drop all ranges and borrowed data before mutating the buffer or awaiting I/O. Use the same parser limits and authentication/dispatch path for GET and SET. Count fallback frequency, fallback input bytes and command-argument copies separately. `decode_range` can allocate range metadata for aggregates, so the target is zero full-input copies on the admitted path.

```rust
// Design sketch. Validate borrowed input and produce the same owned command.
match decode_in_place(&unread, dialect, limits)? {
    NeedMore => read_more(),
    Complete { command, consumed } => dispatch_one(command, consumed),
    CanonicalFallback => decode_with_existing_path(&unread, dialect, limits),
}
```

Treat `BytesMut` plus `decode_bytes_mut` or cursor compaction as an independent alternative if attribution justifies it. Measure split-buffer ownership, reallocations and high-water retention before considering it. W2 admission holds buffer movement and response work constant so the result isolates the decoder.

**Tests.** Add `crates/hydracache-redis-compat/tests/resp_input_decode_077.rs` with `plain_frames_match_canonical_decode`, `every_incomplete_prefix_matches_canonical`, `binary_and_numeric_arguments_match`, `unread_buffer_limit_and_error_precedence_match` and `supported_plain_path_copies_no_input_suffix`. Test cap−1/cap/cap+1, multiple legal frames exceeding the unread cap, malformed tails, binary NUL/CRLF, unsupported streaming, nested or attributed frames, duplicate aggregate entries, and valid and invalid UTF-8. Compare the result, consumed length, arguments and emitted error bytes. Run `cargo test -p hydracache-redis-compat --test resp_input_decode_077 --test resp_boundaries --locked`.

**Exit.** Supported complete frames and admitted incomplete prefixes perform zero full-suffix copies. The differential and resource gates pass before any performance claim is considered.

### W3. Prove wire and connection behavior for the decoder

**Implementation.** Replay one byte corpus through baseline and candidate with each split position for short transcripts and a seeded set of fragmentation schedules for larger inputs. Compare every output byte, close point, protocol switch, dispatch count and final visible state. Feed the same signed-in, unauthenticated, subscribed, disconnected and slow-reader schedules through the real listener. Test both RESP dialects over plain TCP and the production TLS/mTLS configuration. Keep transport partial-write and flush errors in the oracle so decoder lookahead cannot change mutation eligibility. Expand the existing RESP fuzz target with bounded differential checks and retain minimized mismatches. [Fuzz workspace](../../fuzz/Cargo.toml) · [Adversarial transport tests](../../crates/hydracache-redis-compat/tests/resp_transport_adversarial_074.rs)

**Tests.** Add `resp_input_connection_077.rs` in `crates/hydracache-redis-compat/tests` with `hello_switch_applies_to_the_next_buffered_frame`, `blocked_reply_preserves_dispatch_order`, `partial_frame_does_not_dispatch`, `quit_stops_buffered_commands` and `subscriber_and_mtls_keep_exact_transcripts`. Reuse production daemon harnesses for cross-surface visibility and Redis oracle rows. Run `cargo test -p hydracache-redis-compat --test resp_input_connection_077 --test resp_transport_adversarial_074 --locked`. Run the existing `fuzz_resp_command` in the scheduled fuzz lane. A canary that treats an incomplete frame as complete must fail the transcript oracle.

**Exit.** Both protocols and both transport configurations meet the same correctness contract. A decoder failure keeps that candidate out of W9.

### W4. Shard the measured client-store owner

**Implementation.** In W0, locate the lock in the admitted backend. The historical `ClientSurfaceState` owns one `Mutex<BTreeMap<…>>`, while the `0.75` plan moves member data behind the shared dispatcher. Attach sharding to the actual contended local backend or one partition's local store. Keep authority assignment, persisted key identity and client routing separate from the in-process shard number. Attribute any win to the measured mode. [Current client store](../../crates/hydracache-client-transport-axum/src/lib.rs) · [0.75 backend seam](V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#w10a-separate-local-and-distributed-backends-behind-one-mutation-contract)

Start with a private array of independently locked maps. Preserve the map representation for the first experiment so the comparison isolates lock topology. Use the complete canonical `(tenant, namespace, key)` with the existing binary-safe framing and a process-lifetime hash seed. Inject a deterministic seed in test fixtures. Freeze a small shard-count sweep during screening and pin one count and memory budget for qualification. Single-key operations take one shard. Measure padded and unpadded layouts only if cache-line contention is independently attributed.

For atomic batches, validate the request and calculate its existing quota delta under the canonical admission rule. Map keys to shard IDs, deduplicate and acquire those shards in increasing order, then apply the whole batch while the necessary guards are held. Preserve input order and duplicate-key behavior in results and events. Readers of an atomic multi-key operation acquire the same relevant set before materializing its outcome. Native partition-grouped partial outcomes follow their own contract. Release store and quota guards before publishing events or waiting for I/O. Keep exact quota reservation and rollback shared with single-key operations. If the tenant ledger becomes the next bottleneck, attribute it separately. [Existing batch and tenant admission](../../crates/hydracache-client-transport-axum/src/lib.rs) · [W6 and W7 design constraints](V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md#w6-split-the-read-path-and-shard-the-client-store-only-with-measured-evidence)

Document a total order for shared admission, shard acquisition and maintenance. Every contender follows that order. Use a bounded round-robin expiry cursor with a total work budget per sweep and demonstrable progress on sparse and full shards. If cleanup crosses an observation boundary, revalidate the stored version and expiry while holding the target shard before removing it, releasing quota and emitting its event exactly once. Namespace reset, drain, accounting and diagnostic snapshots use an explicit admission fence or an existing ordered snapshot path where exactness is promised. Explain both forward progress during contention and the memory/scan overhead introduced by additional shards.

```rust
// Design sketch. Prevalidation and admission use the existing mutation contract.
let shard_ids = sorted_unique_shards(&verified_batch);
with_admitted_batch(verified_batch, |batch| {
    let shards = lock_in_order(&shard_ids);
    apply_atomically(batch, shards)
})
```

**Tests.** Add `crates/hydracache-client-transport-axum/tests/client_store_shards_077.rs` with `duplicate_keys_preserve_last_value_and_event_order`, `atomic_batch_visibility_with_reversed_input_order`, `quota_failure_leaves_no_partial_write`, `expiry_cleanup_cannot_delete_replacement` and `bounded_sweep_reaches_cold_shards`. Cover collision handling, namespaces containing binary keys, oversized requests and linearization of conditional operations. Run `cargo test -p hydracache-client-transport-axum --test client_store_shards_077 --locked` and its existing client-surface regression suite.

**Exit.** Lock ownership is the only changed performance variable. The source proof shows one authoritative copy and the operation tests reproduce predecessor semantics.

### W5. Prove shard isolation, atomicity, and lifecycle under contention

**Implementation.** Compare generated histories against the canonical unsharded reference. Include simultaneous RESP and HC/2 calls against one logical namespace and deterministic interleavings across shard, quota and expiry boundaries. Add a lock-order model or Loom adapter around the new critical sections, with a reverse-order deadlock canary. Reuse tenant and authenticated-route adversarial tests. Pin process RSS, empty-shard cost, per-request lock counts, sweep backlog and fairness bounds. Compare c1, c8 and c32 using uniform, disjoint, skewed and single-hot-key distributions. The single hot key is a guard for unchanged contention behavior. Independent-key cells are the effect target.

**Tests.** Add `crates/hydracache-server/tests/client_store_shards_e2e_077.rs` with `resp_mset_and_hc2_read_share_one_atomic_state`, `slow_client_does_not_hold_shard_or_quota_locks` and `quota_and_expiry_balance_after_churn_and_reset`. Add a canary that publishes the first shard before the whole batch is visible. Use existing [cluster-testkit histories](../../crates/hydracache-cluster-testkit/tests/value_plane_concurrent_075.rs) for replay and [0.75 surface tests](../../crates/hydracache-cluster-testkit/tests/value_plane_surface_075.rs) for the shared dispatcher. Run `cargo test -p hydracache-server --test client_store_shards_e2e_077 --locked`. Register real-daemon, seeded schedule and slow-reader runs in the release evidence matrix.

**Exit.** Atomicity, per-tenant accounting, fair maintenance and native/RESP equivalence pass on the actual backend. W9 admits the optimization only if independent-key gains clear the threshold and hot-key/shallow/native guard cells pass.

### W6. Establish the durable batch-of-one baseline

**Failure contract.** For an admitted namespace, a client-observed successful mutation survives the loss of any one member together with its disk, with the other two members and volumes healthy. This is RPO zero for confirmed mutations in that failure model. A committed later replacement, delete or expiry changes the expected value. Permanent loss of a majority of the volumes requires a separate recovery contract. The duration of unavailability depends on failure detection, election and recovery and is measured separately.

**Authority.** Use one authority and one ordered log for the first durable namespace. Three voters on independent volumes and failure domains establish a durable majority of two. Reuse the predecessor's data authority if it proves durable quorum commit and eligible promotion from the recovered prefix. Otherwise W6 requires a reviewed ADR and an opt-in implementation of one replicated data log behind the existing `ClientDataBackend` seam, with Raft as the concrete reference. The existing topology Raft group keeps its metadata role. The data leader is the sole mutation authority for the admitted namespace, and the registered owner epoch and placement must agree with the data-group term, membership and recovered commit frontier before requests are served. Use fixed membership for the first profile. Admission and recovery must prevent a second leader or a volatile store from accepting writes for the same namespace. [IMap owner and backup contract](V0_75_EXTENDED_IMAP_SURFACE_PLAN.md#w10c-make-owner-writes-authoritative-and-replicate-before-acknowledgement) · [Existing Raft persistence contract](../adr/0002-raft-log-store-durability-contract.md) · [Raft safety](https://raft.github.io/raft.pdf)

**Implementation.**

1. Register the durable profile and the backend selection at bootstrap. Validate storage, group/namespace identity, distinct placement, format and key/operation limits before readiness. The admitted namespace routes all enabled surfaces to one implementation. A local backend remains useful as a control with its own scope.
2. Use a deterministic, versioned mutation record carrying canonical identity, ordered command or effects, conditional operands, expiry decision time/absolute deadline where applicable, and request identity when supported. Apply conditional decisions against the previous committed state. A same-domain atomic `MSET` occupies one log entry and has one apply result. Preserve versions, tombstones, quota, audit and event ordering at the existing visibility boundary. Treat the observation clock as input to expiry semantics. Term, epoch, index and version determine authority and order.
3. Persist each voter's log entry and required consensus state before its persistence proof can advance the quorum. Commit using the chosen data-group protocol and wait for successful local apply before creating the success reply. Reconstruct state from a validated snapshot and committed suffix. An applied marker can advance only with recoverable state, or recovery must replay from the snapshot boundary. Snapshot installation validates format, identity, checksum and coverage before switching manifests or truncating the retained log. Rejoining or promoted owners become ready only after proving and applying their committed prefix. Reads that claim current authority require the admitted read barrier, such as Raft `ReadIndex` plus apply catch-up when using the Raft implementation.
4. Persist the HC/2 `idempotency_key`, fingerprint and replay result with the operation within the existing retention contract. A timeout before proposal can return a proven rejection. Once append or commit is possible, a timeout or response loss can make the outcome unknown until the authority resolves it. RESP retries follow their documented command semantics. Use transport completion, outcome certainty and durable commit as separate states in receipts.

```text
verified request → admitted owner/epoch → durable append on voters
                 → committed index on the data group → local apply/result → response
```

Reuse Sled, the existing log-store, recovery and fault-test machinery where it implements the required contracts. `DurabilityWritePath` demonstrates local flush-before-ack and a separate RAM-queued mode. It cannot supply a distributed success by itself. The `0.75` durability model is an oracle for state transitions. Close the real networked proof before opening the optimization. [Local durability implementation](../../crates/hydracache/src/grid/durability.rs) · [Test-only durability model](../../crates/hydracache-cluster-testkit/src/value_plane_durable_075.rs) · [0.74 W9 follow-ups](../testing/performance/0.74/w9-local-followups.md)

**Tests.** Add `crates/hydracache-cluster-testkit/tests/durable_authority_077.rs` with `success_requires_disk_quorum_and_apply`, `promotion_requires_committed_prefix`, `replay_restores_ttl_tombstone_and_idempotency`, `wrong_group_proof_cannot_settle_mutation` and `same_group_batch_is_one_atomic_apply`. Run `cargo test -p hydracache-cluster-testkit --test durable_authority_077 --locked`. Before W7 timing, run W8 against the production batch-of-one backend and keep its receipts as the matched control.

**Exit.** Batch of one has a working real-daemon, quorum-backed durability proof. If W6 is blocked, the grouped profile stays disabled and its performance comparison has no eligible baseline.

### W7. Amortize durable sync across independent writes

**Implementation.** Add the batching seam inside the admitted data-store/replica pipeline. Keep the existing proposal identities, sequence, conditional decisions, per-mutation apply and event rules. Each storage owner collects ready append work in log order and shares one sync across a contiguous prefix. A per-operation waiter carries group, term/epoch, record identity and index. Release it only when the exact durable quorum and local apply cover that mutation. With Raft `Ready`, persist snapshot/entries/hard-state as required by the existing contract and advance the persisted frontier and outbound persisted messages only after the corresponding sync completes. Apply one mutation at a time in log order, with each existing atomic operation remaining one unit. [Raft `Ready` contract](https://docs.rs/raft/0.7.0/raft/raw_node/struct.Ready.html) · [Current metadata log store](../../crates/hydracache-cluster-raft/src/log_store.rs)

Start a flush immediately when an idle worker receives work. Drain already queued entries up to frozen item and byte caps when the current flush completes. Bound queue depth, retained bytes, oldest wait and in-flight replication per group and tenant. Start screening with 1, 8 and 32 records per flush and a byte cap aligned with the existing legal record limit. A legal record larger than the normal batch budget takes one dedicated flush within the absolute admission cap. Qualify a single setting with the same physical disks and traffic as the batch-of-one control. Evaluate a deliberate collection delay only as a separate hypothesis under the single-client tail gate.

Keep one append/flush owner per ordered stream. A short write or failed sync closes the affected persistence range until recovery or safe retry establishes its outcome. Pending callers retain the original identity and certainty classification. A late completion, duplicate proof or prior term's notification cannot settle a replacement waiter. Client cancellation releases the waiter's resources according to the existing outcome contract and leaves any committed mutation recoverable. The storage worker never holds shard, tenant or connection guards across disk or network waits. Track fair progress across tenants with bounded queues and preserve the RESP per-connection delivery frontier.

```rust
// Design sketch. Batch the storage operation and retain per-entry completion.
let batch = queue.take_ready_prefix(max_records, max_bytes);
let persisted = store.append_and_sync(batch)?;
replication.advance_durable_prefix(persisted)?;
complete_applied_commits_from_current_authority();
```

**Tests.** Add `crates/hydracache-cluster-testkit/tests/group_commit_077.rs` with `one_flush_covers_only_the_synced_prefix`, `batch_one_and_grouped_apply_the_same_history`, `old_epoch_completion_cannot_settle_new_waiter`, `queue_bounds_reject_before_proposal` and `cancelled_waiter_leaves_committed_outcome_replayable`. Include count/byte/deadline limits, same-key ordering, oversize progress, shutdown/drain, buffer and waiter reclamation, duplicate/reordered replies and heterogeneous requests. Run `cargo test -p hydracache-cluster-testkit --test group_commit_077 --locked`.

**Exit.** The count-one and grouped controls share durability, admission, state and compatibility. Only the number of entries covered by a flush differs. The mechanism saves physical syncs in eligible cells and reaches no caller before that caller's proof is complete.

### W8. Prove recovery and acknowledgement boundaries with real clients

**Implementation.** Extend the process-cluster harness with deterministic barriers in the data log, replication, snapshot and response paths. The independent client process records which successful replies it actually received, plus command identity, expected value digest, version/TTL observation and order. Inspect the elected owner through the public surface. Reconcile the history using versions and committed indices. Classify entries without a response separately. For duplicate retries compare the state and emitted event history with a single logical execution. Test the same workloads in batch-of-one and grouped modes. [Daemon cluster harness](../../crates/hydracache-server/tests/support/daemon_cluster.rs) · [Existing leader-kill test](../../crates/hydracache-server/tests/daemon_process_cluster.rs)

| Fault or cut | Assertion |
| --- | --- |
| Kill before proposal, after append before local sync, or after local sync while replica sync is blocked | Every observed success has an exact durable quorum proof and successful apply. Pre-proposal rejection leaves state untouched. A proposed operation without a reply is resolved under the outcome-unknown rules. |
| Replica append before sync, sync before proof, duplicate proof and stale-epoch proof | Only authenticated proof for the exact record, group and current commit path advances acknowledgement. A delivery receipt or partial prefix cannot cover a later mutation. |
| Kill after durable quorum, before apply or before reply | The promoted authority recovers the committed prefix. Same-identity retries return the committed result within the replay contract without reapplying the mutation. |
| Client receives `OK`, then leader is killed and its volume is removed | With both other members healthy, a valid leader is elected and serves each confirmed live value, version and deadline. Recovered deletes remain deleted. Atomic batches remain whole. |
| Isolate the old leader, then lose the required quorum | The isolated side and the group without quorum cannot return new durable successes. Rejoining nodes prove catch-up before serving authority. An admitted single failure recovers within the profile's separately frozen RTO gate. |
| Slow sync, ENOSPC, EIO, short write or cancellation during flush | Pending work remains bounded. Each outcome has the correct certainty. The remaining available profile follows its admission rules. |
| Kill during snapshot, compaction or member rejoin. Restore the remaining volumes | The validated snapshot and log reconstruct all required confirmed state. Corrupted, mismatched or unsupported retained data produces a visible refusal. An unverified snapshot cannot advance compaction. |

**Tests.** Add `crates/hydracache-server/tests/durable_group_commit_e2e_077.rs` with `confirmed_values_survive_owner_and_volume_loss`, `minority_has_no_success`, `reply_loss_retries_committed_identity`, `fsync_failure_cannot_return_ok`, `mset_is_whole_after_promotion` and `interrupted_snapshot_replays_committed_tail`. Run `cargo test -p hydracache-server --test durable_group_commit_e2e_077 --locked` in the fast harness tier and `cargo test -p hydracache-server --test durable_group_commit_e2e_077 --locked -- --ignored --nocapture` with the registered Linux process and storage fault environment. Inject deterministic fault cuts before running scheduled process chaos. Preserve the seed, fault barrier, process identities and first outcome. Canary runs must fail when a replica counts before sync, when a completion releases before apply, when a stale proof is accepted and when recovery skips a committed suffix.

**Exit.** Confirmed mutations survive all admitted single-member losses, and the admission proof detects early ACK and incomplete recovery. Report the observed recovery time and stated failure model together.

### W9. Measure isolated candidates and the exact composition

**Method.** Use the established `0.74` statistical contract as the template. Start on an admitted dedicated Linux host with independent fresh-process A/A controls. Freeze the offered-load grid, warmup, run window, random seeds, host settings, effect floors, SLO and analysis before timing. Run at least five counterbalanced A/B pairs per required cell and report the paired Hodges–Lehmann estimate and paired-bootstrap 95% interval using all valid attempts. Account for every scheduled offer, including errors, rejections, timeouts, late completions and incomplete work. Record any invalid attempt and its frozen exclusion reason. A noisy A/A or missing cell leaves the claim inconclusive. Retain raw receipts and the first outcomes. [0.74 statistics](../testing/performance/0.74/statistics.toml)

| Cohort | Primary effect | Protected workload |
| --- | --- | --- |
| Decoder | Zero full-suffix copies in admitted scripted cases. For measured pipeline-16 GET and SET cells, preselect CPU/op as the effect metric and require its upper ratio bound at most 0.95. | Pipeline 1, binary/fragmented frames, RESP2/3, plain and mTLS, slow readers. Use p50 depth as a stress guard and confirm actual frames per read. |
| Store shards | At least 20% better goodput at the fixed p99 SLO in declared c8/c32 independent-key cells, with decreased measured lock wait in attribution runs. | Single client, hot key, skewed tenant load, mixed TTL/expiry, atomic `MSET`, native direct dispatch and network surfaces, empty-shard and peak-retention budgets. |
| Durable group | At least 20% better durable successes/s at the same SLO under concurrent writes, with at most 0.5 sync calls per persisted mutation on the measured storage owner. Compare the same profile with max group size one. | c1, same-key contention, mixed GET/write, slow replica and saturated queues, read and write p99/p99.9 including queue time, verified quorum and recovery. |

These are proposed W0 thresholds, not results. Apply effect floors to the appropriate confidence bound and keep thresholds fixed after observing the candidate. For unaffected cells, preserve the inherited 0.74 non-regression bounds of at most 2% goodput loss and 3% CPU/op or p99 increase. Keep the 5% native allocations guard and the matched native-vs-RESP control where applicable. Freeze peak and retained memory, queues, expiry progress, fallback rate and cross-tenant fairness limits in W0. Use pipeline depths 1, 16 and 50 with a separate historical depth-10 bridge, 100 B and 1 KiB values, and a bounded large-value retention lane. Include independent concurrent clients c1/c8/c32 and a bounded saturation lane for the durable group. Report p50/p95/p99/p99.9, CPU/op, allocated/copied bytes, lock wait and hold, syscall or poll counts with their measurement boundary, and RSS/PSS. For durable writes also retain group-size distribution, queue age, sync count and latency, append-to-commit and commit-to-apply times, and outcome-unknown counts.

Measure embedded raw/typed, direct backend, HC/1, HC/2 and RESP separately. Match versions, topology, TLS, expiry, replica count and acknowledgement policy when comparing engines. KiviDB `1.2.0` and pinned Redis may be contextual references on compatible cells. KiviDB's published core numbers use persistence off, and its AOF acknowledgement differs from this profile. Classify unmatched durable cases separately. [KiviDB benchmark configuration](https://github.com/kividbio/kividb/blob/master/benchmarks/1.2.0/core/README.md) · [KiviDB AOF](https://github.com/kividbio/kividb/blob/v1.2.0/docs/PERSISTENCE.md#when-a-write-is-acknowledged)

**Tests.** Add `performance_composition_077.rs` to the xtask test suite with `rejects_volatile_durable_comparator`, `rejects_missing_slow_reader_or_native_guard` and `rejects_composition_containing_a_failed_candidate`. Re-run W3, W5 and W8 plus the relevant `0.76` authority, resource and semantic-cache regressions on the exact assembled candidate. In particular verify rebuild and foreground work together where shared resources are affected. Run `cargo test -p xtask --test performance_composition_077 --locked` and the registered isolated and combined campaigns. Run the exact-candidate six-hour qualification and separately admitted 24-hour confirmation for long-run claims. [Historical release admission](#historical-artifacts)

**Exit.** Each included optimization has a valid isolated comparison and the full build meets correctness, resource and performance guards. Interaction failures require removing the responsible candidate and retesting the resulting composition.

### W10. Register operations, compatibility, and release evidence

**Implementation.** Publish one activation and rollback matrix for the exact candidate. Decoder and shard implementation can be rolled back with the predecessor wire behavior and existing namespace identity. Disable grouping by returning to the qualified batch-of-one durable mode using the same log format. Switching a durable namespace to a version without compatible recovery requires an explicit drain and a registered recovery or migration procedure. At startup, reject unsupported versions and mismatched placement, capacity or storage. Document how operators see quorum, backlog, timeouts, rejection, sync errors and recovery progress. Use bounded metric labels and expose per-group details through diagnostics under the existing access controls. [Rules R-3, R-4 and R-6](../RULES.md) · [Compatibility register](../COMPAT.md)

Bind every W item to exact source, binary, dependency/SBOM, topology, harness, compatibility fixtures, canary and test outcomes. Register 0.77 validators, workload manifests, ship-required gates and CI routing using the existing release-evidence framework. The fast tier runs deterministic tests and the bounded corpus. The scheduled tier runs fuzzing, crash and recovery cohorts. The protected tier runs dedicated timing, resource and exact-candidate ship admission. Add an operator table showing which profiles are available and the data/format consequences of rollback.

**Tests.** Extend `performance_contract_077.rs` with `rejects_rollback_to_volatile_profile`, `rejects_unsupported_log_version_before_serving`, `rejects_missing_fault_or_statistical_receipt` and `rejects_relabelled_historical_results`. Test rolling compatibility with the admitted predecessor for the default path, supported downgrade to count-one, failed drain and incompatible-format refusal. Run `cargo test -p xtask --test performance_contract_077 --test performance_composition_077 --locked`, `cargo xtask doc-check`, `cargo xtask verify` and the registered `cargo xtask release-evidence --release 0.77 --receipts-dir target/release-evidence/receipts --require-ship` after wiring it for this release.

**Exit.** Clean-source artifacts, valid compatibility receipts, expected-red canaries and every mandatory candidate-bound gate agree. Release notes state which paths qualified, which hypotheses were rejected and which failure model the durable profile covers.

## Deferred work

Response coalescing that changes mutation eligibility needs a separate contract and adversarial proof. A second parser/buffer strategy, worker pinning, `io_uring`, direct output encoding or a different hash-table representation needs isolated owner evidence. Dynamic membership for the new durable group and multi-region recovery require their own scope and proof. R-2 continues to bound transaction scope. [Historical staged-execution assessment](#historical-artifacts) · [Rules](../RULES.md)

## Release decision

- [ ] The published predecessor and admitted backend identities are pinned.
- [ ] Each included performance change passes its own semantic, resource and statistical gates and the exact composition passes the full regression matrix.
- [ ] The durable profile, if included, proves the same acknowledgement contract with batch of one and with grouping, with zero confirmed-write loss in the admitted single-member failure model.
- [ ] Restart, migration, rollback, monitoring, compatibility and expected-red canaries are verified against the exact artifact.
- [ ] Release notes limit every claim to the measured profile, workload and fault model. An unqualified workstream is excluded from the claim, and an unproved durability profile is unavailable.
