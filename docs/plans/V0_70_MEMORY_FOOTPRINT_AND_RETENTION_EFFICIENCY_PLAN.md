# HydraCache 0.70.0 Memory Footprint & Retention Efficiency - Codex Execution Plan

> **At a glance**
> - **What:** turn the exploratory memory findings into causal, release-gated improvements across
>   the embedded cache, external client surfaces, RESP edge, HC/2 connection plane, indexes,
>   expiry/reclamation, persistence, and allocator behavior. The release first measures logical
>   live bytes and every retained queue/index, then removes unbounded retention, fixes byte-weight
>   accounting, compacts representations, reduces copies, and proves bounded recovery under long
>   churn. Redis is a pinned control and implementation reference, not a semantic or numerical
>   target; Hazelcast remains a JVM-context comparison with heap/native telemetry kept separate.
> - **Why:** fresh-process evidence shows HydraCache is already small (case-level median RSS p50
>   `8.40 MiB`, versus Redis `11.46 MiB` and Hazelcast `269.37 MiB`), but the reused-process and
>   short soak experiments exposed high-water retention after expiry/reset and a much larger
>   accumulated workload footprint. Source audit found concrete amplification candidates:
>   value-only Moka weights, cloned tenant/namespace/key strings, lazy expiry, duplicated tag and
>   generation metadata, and append-only invalidation/idempotency/audit collections. The evidence
>   does **not** yet prove a generic leak, so the release requires causal counters and allocator
>   evidence before changing implementation or defaults.
> - **After (depends on):** `0.69.0`; consumes the qualified `0.67.1` reference methodology, the
>   final `0.68` HC/2 connection/listener/session design, and the `0.69` executable client matrix.
> - **Unblocks:** defensible memory sizing, a bounded long-lived daemon claim, per-entry/per-client
>   capacity guidance, and later data-structure tuning without repeating the attribution work.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - performance: [`../PERFORMANCE.md`](../PERFORMANCE.md) -
> source reports: [`../testing/perf-scenarios/0.67/results/memory-investigations-report-20260804.md`](../testing/perf-scenarios/0.67/results/memory-investigations-report-20260804.md),
> [`../testing/perf-scenarios/0.67/results/memory-leak-analysis-20260803.md`](../testing/perf-scenarios/0.67/results/memory-leak-analysis-20260803.md),
> [`../testing/perf-scenarios/0.67/results/comparative-memory-20260802.md`](../testing/perf-scenarios/0.67/results/comparative-memory-20260802.md).

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md),
[`docs/GATES.md`](../GATES.md), and [`docs/PERFORMANCE.md`](../PERFORMANCE.md) first. This is a
**measurement-led optimization** release. It may change internal representation and explicit
resource policies, but it must not weaken correctness, consistency, zero-error requirements,
privacy, fail-closed behavior, compatibility windows, SLOs, or the `0.67` measurement contract.
An RSS reduction without proof that live data and mandatory records are preserved is a failure.

## Evidence boundary: what is known and what is not

| Evidence | Observed result | Valid conclusion | Invalid conclusion |
| --- | --- | --- | --- |
| Fresh-process 10-experiment bundle | Hydra median RSS p50 `8.40 MiB`; Redis `11.46 MiB`; Hazelcast `269.37 MiB` | Hydra cold/ordinary fresh-process footprint is not intrinsically JVM-sized and is competitive in this bundle | Hydra always uses less memory at every cardinality or feature profile |
| Reused-process 144-workload comparison | Hydra median case RSS about `296-297 MiB`, maximum `583.7 MiB`; Redis median about `14 MiB`, maximum `26.1 MiB` | Accumulated live state, metadata, service state, and/or allocator high-water require attribution | A generic HydraCache memory leak is proven |
| Three-minute fixed-keyspace soak | Hydra RSS/anon slope about `+5.29 MiB/min` during load checkpoints | Per-key structures, warm-up, or allocator retention are candidates | The slope remains unbounded after steady cardinality |
| Three-minute expiry soak | Hydra RSS slope `+2.57 MiB/min`; Redis approximately flat | Expiry cleanup and allocator recovery are the highest-priority follow-up | Expired values alone remain live |
| Three-minute reset soak | Hydra RSS slope `+9.55 MiB/min`; immediate reset did not restore cold RSS | All maps/indexes/history plus allocator reuse must be checked together | `FLUSHALL` is semantically incorrect without logical counters |
| Idle after load | Hydra `+0.057 MiB/min`, effectively plateau/noise | Continuous no-traffic growth was not observed | High-water memory is acceptable or fully reusable |

The W0 baseline must repeat the positive screens for 30-60 minutes across at least three fresh
processes and must synchronize application counters with process/cgroup/allocator samples. Until
then, `possible-growth` remains a screening label, never `leak`.

## Archived-run provenance (mandatory input)

The original full server-campaign archive is intentionally not stored in ordinary clones. The
release plan pins both the human branch name and the immutable commit so a force-push or later
branch update cannot silently change the input:

| Field | Required value |
| --- | --- |
| Raw archive branch | `explore/0.67-telemetry-hazelcast` |
| Remote-tracking name | `origin/explore/0.67-telemetry-hazelcast` |
| Exact raw archive commit | `dbc2f82f7f303528b3cca7842818730c82232b9c` |
| Annotated archive tag | `explore-0.67-telemetry-20260803` |
| Curated-report branch | `agent/exploratory-perf-reports` (`origin/agent/exploratory-perf-reports`) |
| Curated-report branch tip at plan creation | `c203f361cd3e0cc7d1e1a8627f591055a3fe4bfa` |
| In-repository archive index | [`../testing/perf-scenarios/0.67/EXPLORATORY_ARCHIVE.md`](../testing/perf-scenarios/0.67/EXPLORATORY_ARCHIVE.md) |

The raw branch contains CSV/JSONL telemetry, logs, container metadata, host receipts, accepted and
rejected attempts, and per-run checksums. The curated branch contains human-sized methodology and
reports. `0.70` may consume both, but only the exact raw archive commit is the authoritative
historical byte identity. Branch and tag names are lookup aids, not substitutes for the SHA.

Before W0 imports or derives a baseline, the executor must run:

```bash
git fetch origin --tags
test "$(git rev-parse origin/explore/0.67-telemetry-hazelcast)" = \
  dbc2f82f7f303528b3cca7842818730c82232b9c
git worktree add --detach ../hydracache-0.67-memory-archive \
  dbc2f82f7f303528b3cca7842818730c82232b9c
git -C ../hydracache-0.67-memory-archive status --short
```

The checkout must be clean. Validate every available `SHA256SUMS` file before reading its bundle.
Generate a machine-readable `historical-input-receipt.json` containing branch, tag, exact commit,
relative source paths, file sizes and SHA-256 digests for every raw file used by an analysis. New
derived reports go under the 0.70 candidate evidence tree; archived originals are never rewritten,
normalized, deleted, or mixed into qualification/bootstrap samples. A missing archive path or
checksum is `unavailable(reason)` and blocks any conclusion that depends on it.

## Source reflection: existing applications

| Source | Inspected mechanisms | Principle adopted | Boundary |
| --- | --- | --- | --- |
| Redis | `redis/src/zmalloc.c`, `object.c` memory stats/doctor, `expire.c` active expiry, `evict.c` maxmemory checks and sampled eviction, SDS compact strings, client-memory limits | Separate allocated/active/resident/fragmentation; account before admission; active bounded expiry; compact key/value representation; distinguish dataset from overhead and client buffers | Do not copy Redis single-threaded architecture, command/data-type surface, eviction semantics, or treat Redis RSS as an absolute budget |
| Moka | Hydra's existing `moka::future::Cache` integration and Moka eviction/listener tests | Keep the proven concurrent backend; make weight represent total retained cost; explicitly drive/await maintenance in reclamation gates | No replacement cache engine without a separate measured decision and correctness corpus |
| Caffeine | weighted eviction, maintenance, simulator/trace discipline | Validate weigher accuracy against actual retained shapes and preserve hit-rate behavior under a fixed byte budget | Java object-layout numbers are not Rust layout numbers |
| ScyllaDB | reader concurrency semaphore and count+memory admission | Admit by bytes as well as count; queues consume the same budget as active work; fail loud before allocation-heavy work | No unbounded per-request accounting complexity on the cache fast path |
| TigerBeetle | `src/static_allocator.zig`, bounded client/session pools | Bound long-lived pools and make peak capacity explicit; post-start allocation can be eliminated for selected control structures | HydraCache remains a general-purpose library; full static allocation is not a release goal |
| Hazelcast | native/JVM memory separation, heap sizing, map/index/client resource categories | Report JVM heap separately from RSS/native memory and compare equivalent feature profiles | Hazelcast warm-up/JIT/heap behavior is not attributed to HydraCache and is not a release gate |

## Current code map and hypotheses to falsify

Re-grep these locations on the post-`0.69` base before implementation because `0.68` may move the
client dispatch structures:

| Current location | Observed shape | Memory risk to test |
| --- | --- | --- |
| `crates/hydracache/src/builder.rs` | Moka `weigher` counts `entry.value.len()` only | Keys, `CacheEntry`, tags, expiry and index duplication do not consume capacity; byte limit can materially understate resident cost |
| `crates/hydracache/src/entry.rs` | `Bytes` + `Vec<String>` + `Option<Instant>` | Tags and per-entry allocation/layout dominate small values; empty-tag capacity may be avoidable |
| `crates/hydracache/src/tag_index.rs` | `HashMap<String, HashSet<String>>`, generation maps and cloned keys/tags | Key/tag strings are duplicated; invalidated-key generation tombstones may outlive entries |
| `crates/hydracache-client-transport-axum/src/lib.rs` | `BTreeMap<(String,String,String), StoredValue<Vec<u8>>>` | Three independently allocated strings per record, tree-node overhead, cloned read values, and an edge-local store distinct from the embedded cache |
| same client surface | `Vec<InvalidationEvent>`, `BTreeSet<(tenant,idempotency)>` | Append-only histories grow with mutations/unique request ids unless `0.68` replaces them with bounded protocol-owned state |
| `crates/hydracache-observability/src/audit.rs` | test/small-adapter `InMemoryAuditSink` is append-only `Vec<AuditEvent>` | Accidentally using an in-memory test sink in a daemon retains every audit record |
| `crates/hydracache-redis-compat/src/lib.rs` | RESP conversion through `Vec<u8>`, `Bytes::copy_from_slice`; bidirectional tag maps with copied keys | Decode/encode copies and duplicate tag membership amplify pipeline and tag workloads |
| HC/2 from `0.68` | connection-owned pending calls, outbound lanes, subscriptions and sessions | Per-connection floor, queued bytes, reconnect storms and slow consumers can dominate a mostly-empty cache |
| durable store | process RSS plus cgroup file/page cache | File-backed growth can be mistaken for Rust live-object growth |

These are hypotheses, not pre-authorized rewrites. Each implementation W-item begins with a red
measurement/canary that attributes the corresponding bytes.

## Scope and invariants

The optimized release must cover these separately reported profiles:

1. embedded local cache with no optional services;
2. native HC/2 server and SDK clients;
3. RESP edge with the same supported command subset;
4. persistence off and each supported durable mode;
5. tags off/on, TTL off/on, listeners off/on, lock sessions off/on;
6. one and multiple tenants; idle and 1/10/100/1,000 connections;
7. fixed-cardinality steady state, unbounded-cardinality admission, churn, reset, and restart.

Permanent invariants:

- `R-1` semantics and consistency do not change for memory savings.
- `R-3`: pressure, cleanup and profiler failures are loud; no silent data or audit loss.
- `R-4`: HC/1 and HC/2 compatibility readers remain registered.
- `R-6`: no per-key/client metric-label explosion.
- `R-7`: Redis/Hazelcast comparisons remain scoped evidence, never a universal ranking.
- `R-9`: no disk spill/event-log feature is introduced to disguise resident memory.
- `R-10`: embedded defaults and hot path change only with equivalent-or-better measured evidence.
- `R-11`: skipped/unavailable profilers and unstable runs remain visibly non-green.

## Non-goals

- No new cache operation, Hazelcast compatibility surface, Redis command, consistency level, or
  cluster algorithm.
- No lowering cache capacity, keyspace, payload, repetitions, SLO, zero-error requirement, or
  workload duration to manufacture a smaller number.
- No claim that RSS returns to cold-start after every free; live/active/resident/retained and reuse
  are reported separately.
- No global allocator change based on one Linux machine; Windows/macOS and sanitizer behavior must
  remain buildable and documented.
- No jemalloc/mimalloc mandate. The system allocator remains valid if it wins the complete gate.
- No removal of mandatory audit, invalidation repair, idempotency, session fencing, or listener
  replay semantics. They become bounded with explicit overflow behavior.
- No official numerical Redis/Hazelcast superiority claim from exploratory evidence.

## Implementation map for audits

Populate the implementation column and exact command as W-items land.

| Item | Deliverable | Primary proof | Boundary |
| --- | --- | --- | --- |
| W0 | causal baseline + frozen comparison contract | repeated synchronized memory receipts | no optimization before attribution |
| W1 | application/allocator memory observability | counters reconcile with live state | bounded labels; no secrets |
| W2 | byte-accurate capacity/admission | synthetic and live object-weight corpus | no hidden capacity reduction |
| W3 | bounded histories/queues | mutation/idempotency/audit/event churn | no silent loss |
| W4 | expiry/delete/reset reclamation | repeated TTL/reset with exact logical zero | no full-map pause on hot path |
| W5 | compact entry/key/value representation | bytes-per-entry matrix + API corpus | wire and key semantics unchanged |
| W6 | tag/generation compaction | tag churn + invalidate-during-load proof | stale loads still fenced |
| W7 | copy/allocation reduction | allocation/op and retained-byte profiles | no unsafe lifetime escape |
| W8 | allocator experiment and selection | allocated/active/resident/retained A/B | portable fallback required |
| W9 | optional-service/profile ablation | one-factor receipts | defaults do not silently change |
| W10 | HC/2 per-connection efficiency | slow-client/reconnect/1k-connection soak | quotas and fairness retained |
| W11 | durable/page-cache separation | anon/file/slab + recovery proof | durability unchanged |
| W12 | long soak + cross-target regression | fixed-key/TTL/reset/connection matrix | same fingerprint and workload |
| W13 | governance/docs/release decision | exact-candidate require-ship evidence | claims match receipts |

## W0. Freeze a causal memory baseline before changing code

Add `docs/testing/perf-scenarios/0.70/memory-efficiency-v1.toml`, a typed report schema, and a
`hydracache-loadgen memory-efficiency` orchestration entry. Every target/case must record:

- source SHA, binary SHA, build profile/features, allocator, image digest, host fingerprint,
  affinity, cgroup limit, kernel, service profile and exact command;
- fixed unique-key cardinality verified independently from request count;
- logical key/value/tag/index/event/idempotency/audit/pending/subscription/session counts and bytes;
- process `VmRSS`/`VmHWM`, `smaps_rollup` RSS/PSS/anon/file, cgroup
  `memory.current`/`memory.peak`/anon/file/slab, threads and FDs;
- allocator allocated/active/resident/retained/mapped fields when supported, otherwise
  `unavailable(reason)`; never substitute RSS;
- load, settle, delete/expire/reset and post-idle checkpoints with monotonic timestamps;
- RPS, p50/p95/p99/max latency, errors/timeouts/retries, CPU and context switches.

W0 also creates and validates the `historical-input-receipt.json` described above. Historical raw
rows may guide hypotheses and scenario design, but they are not silently pooled with new baseline
samples: different source SHA, host fingerprint, instrumentation or workload contract remains a
separate cohort.

Required baseline cases use fresh processes and the same workload contract for baseline/candidate:

1. cold idle 5 minutes;
2. 1k/10k/50k/250k keys at 64/256/1,024/4,096-byte values;
3. six and sixty fixed-keyspace rewrite cycles;
4. sixty TTL fill-expire-idle cycles;
5. sixty fill-delete/namespace-reset cycles;
6. tags 0/1/4/16 per entry and repeated tag invalidation;
7. listeners and HC/2 connections 1/10/100/1,000 including slow consumers;
8. persistence off/on with anon/file separation;
9. 60-minute and six-hour steady-state screens; 24-hour scheduled confirmation for ship.

At least three fresh-process repetitions are required for attribution; reference comparison uses the
`0.67.1` five-sample same-fingerprint rule. W0 freezes baselines but sets no optimization target
from the candidate. Budgets are proposed from the pre-change distribution, independently reviewed,
and committed before W2-W11 results are visible.

**Canaries:** lie about unique-key count; reuse a dirty process as fresh; swap allocator/build
features; omit an unavailable field; resolve the archive branch to a commit other than
`dbc2f82f7f303528b3cca7842818730c82232b9c`; mutate one archived raw byte. Each must invalidate the
receipt.

## W1. Add synchronized live-object and allocator observability

Introduce a bounded `MemoryFootprintSnapshot` assembled from subsystem-owned counters rather than
walking every map on the hot path. It must expose, using bounded labels:

- live entries and logical key/value bytes;
- estimated retained bytes by entry/key/tag/generation/expiry metadata;
- client-surface/RESP/HC/2 store entries if any remain after `0.68`;
- event ring occupancy/bytes/dropped-or-repair-required count;
- idempotency records/bytes and oldest age;
- audit buffered records/bytes and sink failures;
- pending loads/invocations, outbound queued bytes, subscribers, sessions and connections;
- durable-store logical bytes plus page-cache/file-backed metrics where available;
- allocator allocated/active/resident/retained and fragmentation ratios where supported.

Snapshot fields must be available to the privileged Admin/diagnostic path and the experiment
artifact, not exported with key/client identifiers. Incremental byte counters use checked/saturating
updates and a scheduled exact reconciliation scan in tests/nightly. Counter drift above the frozen
tolerance fails loud.

**Required tests:** insert/replace/delete/expire/flush/tag-churn/subscription-close/session-loss and
error/cancel paths reconcile to exact logical counts; secrets and keys do not appear in metrics,
logs or reports.

**Canary:** a fixture retains a hidden secondary-index entry without updating the snapshot; the
exact reconciliation gate must detect it.

## W2. Make capacity and admission account for total retained bytes

Replace the current value-only Moka weight with a reviewed estimator covering at least encoded key,
value, `CacheEntry`, tag vector/string bytes, expiry metadata and measured index amplification. Keep
the estimator deterministic, O(1) in already-known lengths, saturating to Moka's `u32` weight and
consistent across insert/replace paths.

For server/client surfaces, enforce both count and byte budgets before allocation-heavy decode or
mutation:

- dataset live bytes and entries per tenant/namespace plus global reserve;
- queued/inflight/output bytes separate from dataset bytes but charged to the same pressure model;
- oversized single requests and atomic batches rejected before partial mutation;
- replacement charges only the positive delta while retaining rollback safety;
- eviction/rejection policy remains explicitly configured; no silent semantic switch.

Create a golden object-weight corpus for empty/small/large keys, 0/1/16 tags, TTL/no-TTL and maximum
payload. Compare the estimator to heap-profiler retained deltas in W0; document conservative error
bounds. A capacity configuration continues to represent an advertised logical budget, and migration
guidance explains any change from legacy value-only units.

**Canaries:** remove key/tag metadata from the estimator; overflow `u32`; admit a batch whose
aggregate exceeds the byte budget. All must turn the capacity gate red.

## W3. Replace append-only runtime histories with bounded contracts

Re-audit the post-`0.68` types. No production daemon path may retain an unbounded
`Vec`/`Set`/`Map` keyed by mutation, request id, connection or audit event.

- Invalidation/event replay uses the bounded watermark ring from the live listener contract.
  Overflow advances an explicit minimum replay watermark and requires snapshot/clear repair; it
  never silently drops correctness state.
- Idempotency retains the complete prior outcome, not only a key, in a bounded TTL/size cache.
  Expired records make retry safety explicit; unsafe duplicate ambiguity fails loud.
- Mandatory audit uses the configured durable/streaming sink and fail-closed semantics. An
  in-memory sink is test/small-adapter-only and must have an explicit bound/overflow policy.
- Diagnostics journals, lock/session tombstones and reconnect histories receive count+byte+age
  limits with counters and deterministic cleanup.
- Queue capacity is charged in bytes, not only items; a single maximum-sized frame cannot bypass
  the intended memory ceiling.

**Required tests:** one million mutations/unique idempotency keys/audit records under a fixed data
keyspace reach a plateau; replay gaps repair correctly; mandatory audit sink pressure blocks or
rejects before mutation according to contract; dropping every connection releases all owned state.

**Canaries:** restore append-only event or idempotency behavior; disable cleanup on one close path;
pretend a replay gap is success. The soak/semantic gate must fail.

## W4. Make expiry, delete and reset reclaim every logical owner

Build a single ownership ledger for each entry: primary value, expiry scheduling, tag membership,
generation fence, persistence record, client-surface metadata and event/idempotency references.
Every removal cause executes an idempotent cleanup transaction over that ledger.

- Add bounded active expiry/maintenance so expired-but-never-read entries do not remain forever.
  Work per tick is budgeted by count/time and tested against p99 latency.
- Delete, expiry, capacity eviction, tag invalidation, namespace reset, cluster invalidation and
  shutdown converge on the same cleanup primitive.
- Await/drive Moka maintenance where the public operation promises reset completion; distinguish
  logical completion from later allocator page release.
- Retire tag/key generation tombstones only when no pre-invalidation load can publish stale data;
  use epochs/generation references rather than unbounded historical keys.
- Batch reclamation is bounded and observable; backlog has a hard limit and admission response.
- `flush`/RESP reset success requires exact logical zero for the addressed scope, not cold RSS.

**Required tests:** 100 TTL cycles, 100 fill-delete cycles and 100 reset cycles return all logical
counters to the frozen empty-state values; concurrent load/invalidate never resurrects an entry;
post-idle allocator active bytes/RSS meet W0 budgets without an unbounded cleanup queue.

**Canaries:** skip tag unregister, expiry-wheel removal, generation retirement or maintenance drive;
each produces counter drift or stale resurrection and fails.

## W5. Compact primary key, entry and value representations

Use W0/W1 retained-size evidence to select, not assume, representation changes:

- avoid three independent `String` allocations for tenant/namespace/key in the client store;
  consider interned/validated tenant and namespace ids plus one canonical byte key;
- remove the RESP `redis-binary-v1-` hexadecimal doubling when a binary-safe canonical structured
  key can preserve exact equality and compatibility;
- use `Bytes`/shared immutable storage where ownership crosses protocol/cache boundaries and a copy
  is not required for safety;
- store empty/small tag sets inline only if measured (`SmallVec` or equivalent), and share repeated
  tag/namespace strings only with bounded interning and reclamation;
- compare measured `BTreeMap`, randomized hash map, Moka-backed and slab/handle layouts using the
  same collision, iteration, determinism and concurrency requirements;
- prevent a compact form from creating an expensive decode allocation on every read.

The selected design requires an ADR if it changes the canonical in-memory key representation or
allocator. Wire keys and public serialization do not change. Hash collision behavior is adversarially
tested; deterministic evidence records normalize ordering instead of depending on hash iteration.

**Gate:** materially lower reviewed bytes-per-entry for small-value and tag-heavy matrices while
meeting the frozen latency/CPU/error/hit-rate budgets. A statistically inconclusive variant is not
merged merely because `size_of` is smaller.

## W6. Compact tag and generation indexes without weakening stale-load fencing

Rework `TagIndex` from copied strings in multiple maps toward canonical key/tag handles or another
measured compact representation. Requirements:

- one canonical allocation per distinct live key/tag within the index;
- bidirectional membership remains exact;
- empty sets and dead generation entries are reclaimed;
- global/key/tag generation fencing still rejects a load started before invalidation;
- handle reuse cannot make an old snapshot current (generation/epoch protects ABA);
- tag invalidation cost remains proportional to affected membership, not whole keyspace;
- index memory is included in capacity/admission and diagnostic counters.

Use property tests for arbitrary register/unregister/invalidate/load interleavings and a reference
model. Run tag distributions 0/1/4/16/64 per entry plus one-hot and high-fanout tags.

**Canaries:** reuse an id without changing generation; retain empty membership; delete a generation
too early. Model/deterministic tests must catch all three.

## W7. Remove avoidable protocol and cache copies

Instrument allocations/op and bytes/op for HC/1, HC/2, RESP and embedded get/put/batch paths. Then:

- decode from bounded `Bytes`/slices and transfer ownership into the cache where safe;
- avoid `Bytes::copy_from_slice` followed by `to_vec`/clone chains;
- return shared immutable values internally while SDK/public boundaries preserve ownership rules;
- reuse encode buffers only through bounded pools charged to connection memory;
- right-size vectors from validated batch lengths and shrink/drop exceptional large buffers;
- avoid cloning keys/tags merely to emit an event when no observer is registered;
- keep sensitive buffers out of long-lived pools or zeroize where the security contract requires.

Unsafe zero-copy is not a goal. Any borrowed view must be proven not to escape the frame lifetime;
Miri, sanitizers, cancellation tests and fuzzing remain green.

**Gate:** reduced allocation/op and copied-bytes/op on the golden paths, with no increase outside
the frozen p99/CPU thresholds and no cross-request data disclosure.

## W8. Compare allocators and define reclamation behavior

Add mutually exclusive, opt-in build profiles for the system allocator and the candidate Linux
allocators selected in W0 (for example jemalloc and mimalloc). Record exact crate/version/config.
For each allocator capture allocated, active, resident, retained/mapped and purge behavior through
the same load-reset-idle matrix.

Selection rules:

1. live logical bytes and application behavior must be identical;
2. resident/active recovery and fragmentation improve across at least three fresh processes;
3. p99, throughput, CPU and context-switch budgets remain green;
4. no sanitizer, Miri, static-link, licensing, cross-compilation or unsupported-platform regression;
5. explicit purge, if used, is rate/budget limited and measured for latency spikes;
6. platforms without the selected allocator retain a documented system fallback.

If no candidate wins the complete matrix, retain the system allocator and publish the negative
result. An allocator is not a substitute for fixing live-object retention in W3-W6.

**Canary:** report only RSS while hiding allocator active/retained divergence; schema validation
must reject the comparison.

## W9. Quantify optional services and introduce explicit deployment profiles

Run one-factor ablations for Admin API, metrics, RESP, HC/1, HC/2, persistence, cluster member,
listeners and diagnostics. Produce cold, per-connection, per-mutation and steady-state deltas.

If the evidence supports it, add named explicit profiles such as `embedded-minimal`,
`server-minimal`, `server-client`, and `server-full`. Profiles expand into ordinary reviewed config;
operators can inspect the effective configuration. Existing defaults do not silently change in
0.70. A future default change requires its own ADR/migration notice.

Disabled services must allocate no listener, background task, channel, history or large dependency
state. Feature flags must not split correctness semantics. Startup receipts record the profile and
effective components.

**Canary:** a disabled service secretly starts its channel/background task; the ablation counter
and thread/resource gate must fail.

## W10. Optimize and hard-bound the HC/2 connection plane

Measure and budget per connection:

- transport/TLS buffers;
- decoder/encoder scratch;
- pending invocation table and retained request/reply bytes;
- reply/control/event queues;
- subscription replay state;
- topology snapshot and lock sessions;
- authentication/audit state and task stacks.

Implement shared immutable topology/schema material, right-sized initial buffers, hard count+byte
limits, fair global/per-tenant reservations and deterministic release on every close/cancel/drain
path. Large buffers must not remain pinned to an idle connection indefinitely. Slow consumers and
reconnect storms receive explicit backpressure/rejection and repair, never unbounded queues.

Required real-process cases: 1/10/100/1,000 idle connections; 100 slow subscribers; maximum-frame
abuse; reconnect storm; rolling restart; lock-session loss; cancellation at every allocation-owning
state transition. Per-connection steady-state cost and high-water are published with TLS on/off
separated.

**Canaries:** leak a pending invocation, event buffer, subscription or TLS task on close; the
post-close exact-zero and slope gates must fail.

## W11. Separate durable-store memory from page cache and tune it safely

For every supported persistence mode report application anon, allocator, mapped/file-backed,
cgroup file/slab and durable logical bytes. Correlate write buffers, compaction/scrub/checkpoint
queues and OS page cache with latency and recovery.

- bound durable write/compaction/checkpoint buffers by bytes;
- avoid keeping decoded and sealed/serialized copies longer than required;
- ensure completed checkpoints/snapshots release staging buffers;
- expose page-cache/mapped growth as file-backed, not as a Rust-object leak;
- run under explicit cgroup memory limits and prove predictable admission before OOM;
- preserve crash recovery, fsync/durability mode and encryption/redaction contracts exactly.

Do not use `drop_caches` inside a measured workload. Cold-cache experiments are separate,
privileged, disclosed receipts. Any storage-engine tuning must pass restart, corruption, disk-full,
backup/restore and checkpoint gates.

**Canary:** retain a completed snapshot buffer or misclassify cgroup file bytes as allocator anon;
the synchronized counter/reconciliation gate must fail.

## W12. Long-duration proof and comparative regression matrix

Run the frozen W0 workload on the exact candidate and independently reviewed baseline:

- per-PR fast structural/unit/property tests;
- scheduled 60-minute fixed-keyspace, TTL, reset and HC/2 connection churn;
- six-hour multi-scenario soak for candidate iterations;
- 24-hour same-fingerprint ship confirmation with five serialized successful samples where the
  `0.67.1` reference contract requires them;
- pinned Redis comparison with equivalent persistence/service/cardinality settings;
- Hazelcast comparison only with fixed `-Xms/-Xmx`, JMX heap used/committed/max, GC and native RSS;
  missing JVM telemetry remains unavailable, not inferred.

Primary gates are candidate-versus-frozen-Hydra baseline, not candidate-versus-Redis:

1. exact logical data/index/history counts and zero semantic errors;
2. no positive post-warmup slope beyond the independently frozen confidence bound in two
   independent anon/resident signals;
3. expiry/reset returns logical owners to zero and allocator active/resident memory to the reviewed
   recovery envelope after the fixed idle window;
4. bytes-per-entry and per-connection targets frozen before candidate measurement are met;
5. existing SLO, p99, throughput floor, CPU, spread, calibration, affinity and quota gates remain
   unchanged and green;
6. no OOM, unbounded queue, thread/FD growth, secret leakage or fail-open behavior;
7. failed/unstable samples are rejected and originals retained unchanged.

Statistical method, warm-up exclusion, confidence interval, slope window and recovery ratio are
versioned in the scenario contract. No hand-selected interval or last-sample comparison.

## W13. Governance, documentation and release decision

- Add `docs/testing/release-evidence/0.70.toml` with exact-candidate receipts for W0-W12 and
  `release-evidence --release 0.70 --require-ship`.
- Register profiler, allocator, long-soak, cgroup and cross-target lanes plus dynamic canaries in
  the gated/canary registries; skip-loud and quarantine-expiry rules remain unchanged.
- Add `docs/performance/memory-accounting.md`: metric definitions, live/active/resident/retained,
  anon/file/slab, logical ownership, measurement pitfalls and reproduction commands.
- Add `docs/performance/memory-sizing.md`: measured per-entry/per-connection/profile guidance with
  exact platform/feature scope and no universal claim.
- Update the earlier exploratory reports with links to the causal release evidence; do not rewrite
  historical numbers or promote them to qualification evidence.
- Reconcile `PERFORMANCE.md`, `GATES.md`, `TESTING.md`, `COMPAT.md`, `FEATURE_MATRIX.md`,
  `releases.toml`, `INDEX.md`, release notes and any resolved technical debt.
- Record an ADR for allocator/default/profile or canonical in-memory-key changes; negative A/B
  findings remain in the evidence ledger.

**DoD:**

```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- memory-contract-check --release 0.70
cargo run --manifest-path crates\xtask\Cargo.toml -- release-governance-check --release 0.70
cargo run --manifest-path crates\xtask\Cargo.toml -- release-evidence --release 0.70 --require-ship
cargo run --manifest-path crates\xtask\Cargo.toml -- doc-check
```

## Required test and experiment inventory

| Category | Mandatory proof |
| --- | --- |
| Accounting | exact insert/replace/delete/expire/evict/flush reconciliation; estimator golden corpus; counter-drift canary |
| Retention | million-mutation bounded-history test; TTL/reset/tag/idempotency/audit churn plateau; cleanup backlog bound |
| Correctness | stale-load fencing, listener gap repair, idempotent retry outcome, mandatory audit fail-closed, lock/session fencing |
| Representation | property/differential corpus over old/new key-entry-index forms; adversarial hash/collision and ABA tests |
| Allocation | allocations/op, bytes copied/op, Miri/sanitizer/fuzz/cancellation, buffer-pool secret isolation |
| Connection | 1k idle, slow consumer, reconnect storm, oversized frame, TLS on/off, exact post-close zero |
| Persistence | anon/file split, memory-limit admission, compaction/checkpoint release, crash/disk-full/backup/restore |
| Performance | coordinated-omission-safe latency/RPS/CPU plus memory on frozen baseline/candidate, unchanged SLOs |
| Long soak | 60-minute, six-hour and 24-hour tiers with fixed cardinality, slope/recovery statistics and immutable raw artifacts |

## Risk register

| Risk | Failure mode | Control |
| --- | --- | --- |
| Optimize a false leak | Representation churn yields no resident benefit | W0 causal counters and allocator fields before code changes |
| Hide memory by lowering capacity | Smaller dataset looks efficient | fixed cardinality/logical bytes and unchanged capacity/workload contract |
| Under-count metadata | Admission says under budget while RSS grows | W1 reconciliation + W2 conservative estimator corpus |
| Break stale-load safety | Generation tombstone removed too early | W4/W6 epoch reference model and resurrection canaries |
| Lose listener correctness | Bounded ring drops events silently | explicit watermark gap + repair; silent-gap canary |
| Lose idempotency | TTL eviction repeats an unsafe mutation | outcome cache, operation retry matrix and ambiguity fail-loud |
| Lose mandatory audit | Bounded sink drops security event | fail-closed sink pressure gate; no drop policy for mandatory events |
| Allocator wins one microcase | Worse CPU/p99/portability in production | full A/B matrix, independent review and system fallback |
| Buffer pooling leaks data | Next tenant observes stale bytes | zeroization/ownership tests and cross-tenant canary |
| Active expiry pauses requests | Tail latency spikes during cleanup | time/count budget, fairness, p99 gate and backlog admission |
| Page cache misread as leak | Wrong code path optimized | anon/file/slab/allocator separation and durable counters |
| HC/2 adds per-client amplification | Empty daemon is small but 1k clients exhaust memory | W10 per-connection byte budgets and reconnect/slow-client soak |
| Historical reports overclaimed | Exploratory numbers become marketing claims | scope labels preserved; only W12/W13 evidence may support sizing |

## Gates: definition of done

- Every retained production collection has an explicit owner, count/byte/age bound, overflow
  behavior, metric and cleanup test; no mutation/request/client-indexed append-only collection.
- Dataset capacity accounts for keys, values and material metadata with a reviewed conservative
  error bound; queues/inflight work are admitted by bytes and count before expensive allocation.
- Expiry/delete/evict/tag invalidation/reset clean every logical owner without stale resurrection;
  logical counters reconcile exactly after 100-cycle gates.
- The selected representation/allocator changes beat the frozen baseline on the scoped memory
  targets and remain within unchanged correctness, latency, throughput, CPU and error gates.
- HC/2 connection/pending/event/subscription/session memory is bounded, fair and released on every
  close/cancel/drain path; 1,000-connection and reconnect/slow-client cases plateau.
- Persistence evidence separates anonymous allocator memory from file/page cache and remains green
  under cgroup pressure, recovery, disk-full, backup/restore and checkpoint tests.
- The 24-hour candidate evidence shows bounded post-warmup behavior under fixed cardinality and
  recovery within the reviewed envelope; unstable/failed samples do not count.
- Redis and Hazelcast are pinned controls with equivalent disclosed profiles; Hazelcast heap is
  measured by JMX or marked unavailable. Neither control defines Hydra correctness or a universal
  memory target.
- `release-evidence --release 0.70 --require-ship`, all dynamic canaries, `doc-check`, workspace
  gates and compatibility windows are green on the exact candidate SHA.

## Final release decision

Ship `0.70.0` only when memory improvements are causally tied to measured owners, every long-lived
collection and connection has an enforced budget, expiry/reset reclaim all logical state, and the
same candidate preserves all existing correctness and performance contracts. A smaller RSS caused
by reduced workload, disabled required semantics, missing evidence, unmeasured page cache, or a
candidate-derived baseline is an automatic no-ship.
