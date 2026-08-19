# HydraCache 0.71.0 Memory Footprint & Retention Efficiency - Codex Execution Plan

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
>   does **not** yet prove a generic leak. The `0.70` GitHub-hosted follow-up instead found exact
>   logical-owner zero after reset, flat idle tails, and residual Hydra RSS consistent with an
>   allocator/runtime high-water candidate. `0.71` therefore requires causal counters, corrected
>   TTL coverage, and allocator evidence before changing implementation or defaults.
> - **After (depends on):** `0.70.0`; consumes its allocation-owner inventory, retained-state
>   snapshots, bounded-lifecycle fixes and deterministic local diagnostic harness, plus the
>   qualified `0.67.1` reference methodology and the `0.69` executable client matrix.
> - **Unblocks:** defensible memory sizing, a bounded long-lived daemon claim, per-entry/per-client
>   capacity guidance, and later data-structure tuning without repeating the attribution work.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - performance: [`../PERFORMANCE.md`](../PERFORMANCE.md) -
> source reports: [`../testing/perf-scenarios/0.70/results/github-hosted-memory-diagnostic-20260816.md`](../testing/perf-scenarios/0.70/results/github-hosted-memory-diagnostic-20260816.md),
> [`../testing/perf-scenarios/0.67/results/memory-investigations-report-20260804.md`](../testing/perf-scenarios/0.67/results/memory-investigations-report-20260804.md),
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
| `0.70` GitHub-hosted reset/idle diagnostic | Hydra median RSS `11.77 MiB` versus Redis `17.34 MiB`; median PSS-anon `4.66 MiB` versus `9.30 MiB`; both idle-tail RSS slopes were zero; Hydra owner snapshots were exactly zero after every reset | No sustained process-level leak was observed in the fixed-cardinality/reset/idle windows; the remaining Hydra RSS is an allocator/runtime high-water candidate | Pages are proven reusable, TTL reclamation is proven, or a noisy hosted-runner RSS value is a release budget |

The W0 baseline must repeat the positive screens for 30-60 minutes across at least three fresh
processes and must synchronize application counters with process/cgroup/allocator samples. Until
then, `possible-growth` remains a screening label, never `leak`.

The `0.70` hosted artifact has one known evidence gap: its TTL workload checkpoint occurred after
the original fixed collector window. The harness now extends collection through the final
checkpoint and rejects incomplete rows, but `0.71` must repeat this case before treating TTL as
reclamation evidence. The hosted result is a useful order-of-magnitude screen, not ship evidence.

The local WSL2/Docker measurement campaign that was originally listed as a 0.70 release blocker is
an explicit 0.71 W0 input. Run the corrected TTL, fixed-keyspace, reset and post-idle cases from a
clean checkout, bind source/binary/image identities, retain raw telemetry and record the WSL2,
kernel, Docker and cgroup fingerprint. This local campaign is diagnostic and cannot replace the
same-fingerprint dedicated-host qualification required for 0.71 ship evidence.

## Investigation hand-off from 0.70

Resolve these questions in order so representation work is not used to mask allocator behavior:

1. **Allocator high-water or live ownership:** after every reset, reconcile exact subsystem-owner
   zero with allocator `allocated`, `active`, `resident`, `retained`/`mapped`, process PSS-anon and
   cgroup anon at the same monotonic checkpoint. A non-zero RSS alone is not a leak verdict.
2. **Reuse versus purge:** after the first high-water cycle, refill the same cardinality without
   process restart and measure fresh OS allocation, allocator reuse and explicit/rate-limited purge
   separately. Returning to cold RSS is not required when pages are demonstrably reusable.
3. **Correct TTL tail:** repeat fill-expire-idle with telemetry covering the final workload
   checkpoint, exact logical-owner zero, bounded cleanup backlog and a fixed post-expiry idle
   window. Any uncovered checkpoint invalidates the row.
4. **Cardinality and payload amplification:** run geometric key counts and 64/256/1,024/4,096-byte
   values; report bytes per live entry, metadata/value amplification and post-reset residuals. Do
   not infer a slope by concatenating separate fresh processes.
5. **Allocator A/B:** compare the system allocator with opt-in Linux candidates on identical
   binaries/workloads across at least three fresh processes. Include CPU, p99, context switches,
   portability and licensing; a one-case RSS win is insufficient.
6. **Service and storage attribution:** isolate Admin API, RESP, HC/2, persistence and connection
   profiles one factor at a time, keeping process anon, mapped/file and cgroup file/slab separate.
7. **Long-lived correctness owners:** continue to treat conditional tombstones and repair
   watermarks as correctness state. Reclamation requires an ordering-safe proof, not a memory cap
   or diagnostic reset.

### CI detection tiers

Memory regression detection is split by signal stability and cost:

| Tier | Trigger | What may fail the tier | Evidence role |
| --- | --- | --- | --- |
| `Memory Regression Fast` | every pull request and protected-branch push | deterministic owner counts/bytes not returning to their declared bound, retained HC/2 state after close, allocation tracker defects, or telemetry coverage accepting an uncovered checkpoint | early structural tripwire; required PR check |
| `Memory Diagnostic (GitHub Hosted)` | explicit workflow dispatch | incomplete Hydra/Redis rows, workload/collector failure, non-zero errors, or missing final-checkpoint coverage | non-promotable order-of-magnitude screen with raw artifact |
| scheduled/dedicated 0.71 lanes | 60-minute weekly, six-hour candidate, 24-hour ship confirmation | frozen slope/recovery/bytes-per-owner budgets on an approved fingerprint | qualification and exact-candidate release evidence |

The fast tier intentionally has no absolute RSS/PSS threshold: GitHub-hosted VM placement and
shared-image drift make such a number too noisy for a PR gate. It fails on deterministic ownership
and coverage contracts first; numerical memory budgets are frozen from repeated same-fingerprint
W0 samples and enforced only in the matching scheduled/dedicated tier.

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
reports. `0.71` may consume both, but only the exact raw archive commit is the authoritative
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
derived reports go under the 0.71 candidate evidence tree; archived originals are never rewritten,
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
| Rust standard library / `cargo-semver-checks` | Predrag Gruevski's 2026-08-15 case study, [Protecting the Rust standard library from accidental breakage](https://predr.ag/blog/protecting-the-rust-stdlib-from-breakage/), and the upstream CI integration | Humans do not reliably spot compatibility changes caused by trait methods, object safety, auto-traits or apparently-private representation edits; compare the effective supported API automatically against an immutable baseline and encode public/non-public status once so every lint benefits | HydraCache is a normal crates.io workspace: use ordinary stable-crate SemVer checks, not the experimental stdlib-only `--stability-aware` mode; Linux rustdoc evidence alone is not a cross-platform compatibility claim |

## Public API breakage guardrail derived from the stdlib case study

Memory optimization is unusually exposed to accidental source breakage. Replacing a private
`String` with `Rc<str>`, changing a wrapper, adding a field, sealing a trait or altering a feature
edge can change a public type's `Send`/`Sync`, object safety, constructibility or availability even
when signatures look unchanged. Review and ordinary unit tests are insufficient controls.

W0 and W13 therefore establish a release-scoped public API contract:

1. After `0.70.0` is published, add `docs/testing/compat/v0.70.0.json` with the immutable tag,
   resolved commit, complete publishable-library package set, feature profiles, target/toolchain
   identity and the pinned `cargo-semver-checks` version. A branch name, moving registry result or
   candidate-derived rustdoc is not an acceptable baseline.
2. Bootstrap `cargo-semver-checks 0.49.0` against the current `0.48.0` evidence before changing the
   pin. Record disagreements and accept the upgrade only after false positives are resolved. Use
   ordinary stable-crate checking; the article's unstable `--stability-aware` mode models Rust
   stdlib attributes and is out of scope here. Run the tool on a pinned analysis toolchain meeting
   its Rust 1.91+ requirement; this does not raise HydraCache's MSRV, which remains independently
   enforced by the MSRV downstream-consumer lane.
3. Run blocking comparisons for default features, all features and every supported exported
   feature profile defined by the canonical feature matrix. Use `cargo metadata`/the tool's feature
   model rather than a second handwritten interpretation of `Cargo.toml`.
4. Add compile-time downstream witnesses for public types affected by W5-W10: required
   `Send`/`Sync`/`Unpin`, trait object construction where object safety is promised, public struct
   construction, feature-selected imports and proc-macro expansion. A representation change may
   land only when both the automated API diff and these consumers remain green.
5. Treat declared preview or intentionally non-public surfaces separately: their changes are
   review-visible but do not block as stable API breakage. Do not add `#[doc(hidden)]`, a private
   feature, an allow-list entry or a baseline rewrite merely to silence a new violation.
6. Preserve the tool's distinct outcomes in CI: a detected SemVer violation and an inability to
   complete analysis are both red, but receive different machine-readable reasons. The receipt
   includes stdout/stderr, tool version, rustc/rustdoc identity, package/feature/target matrix and
   baseline/candidate SHAs.
7. The primary rustdoc diff runs on pinned x86-64 Linux. Pair it with existing Windows, MSRV and
   supported-target downstream consumer builds; do not claim that one host proves target-specific
   APIs or auto-traits everywhere.

The blocking PR lane runs for every change to a publishable crate or its feature/dependency graph,
including private implementation files because private representation can alter public auto-traits.
The full workspace matrix is ship-mandatory on the exact candidate SHA. Tool upgrades are first
report-only against the same frozen fixtures, then become blocking through a reviewed pin change.

## Current code map and hypotheses to falsify

Re-grep these locations on the post-`0.69` base before implementation because `0.68` may move the
client dispatch structures:

| Current location | Observed shape | Memory risk to test |
| --- | --- | --- |
| `crates/hydracache/src/builder.rs` | Moka `weigher` counts `entry.value.len()` only | Keys, `CacheEntry`, tags, expiry and index duplication do not consume capacity; byte limit can materially understate resident cost |
| `crates/hydracache/src/entry.rs` | `Bytes` + `Vec<String>` + `Option<Instant>` | Tags and per-entry allocation/layout dominate small values; empty-tag capacity may be avoidable |
| `crates/hydracache/src/tag_index.rs` | `HashMap<String, HashSet<String>>`, generation maps and cloned keys/tags | Key/tag strings are duplicated; invalidated-key generation tombstones may outlive entries |
| `crates/hydracache-client-transport-axum/src/lib.rs` | `BTreeMap<(String,String,String), StoredValue<Vec<u8>>>` | Three independently allocated strings per record, tree-node overhead, cloned read values, and an edge-local store distinct from the embedded cache |
| same client surface | 0.70 removes the unread invalidation history and bounds idempotency outcomes | Measure the bounded map's per-record cost and TTL sweep amplification; do not reopen the append-only defect |
| `crates/hydracache-observability/src/audit.rs` | 0.70 caps the test/small-adapter `InMemoryAuditSink` and fails mandatory writes closed at capacity | Measure the bounded event cost and add an operator sink without weakening fail-closed semantics |
| `crates/hydracache/src/grid/conditional.rs` | conditional records retain delete tombstones; session heartbeats retain unique session ids | Design ordering-safe watermark GC before optimizing representation; never delete a tombstone solely to reduce RSS |
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
- Published Rust API and feature compatibility is checked against the immutable `v0.70.0`
  baseline; an internal memory optimization cannot silently remove an auto-trait, break object
  safety, alter a supported feature edge or make a public type unconstructible.

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
| W0 | causal baseline + frozen comparison contract, including the transferred WSL2/Docker campaign | repeated synchronized memory receipts with corrected TTL coverage | no optimization before attribution; local VM results are non-promotable |
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
| W13 | governance/docs/release decision + public API protection | exact-candidate require-ship and `v0.70.0` API-diff evidence | claims match receipts; private representation changes cannot break supported downstream code |

## Mandatory execution controls for W0-W13

The following ten controls are part of the release contract, not optional recommendations. They
strengthen W0-W13 without adding parallel work-item identities to `releases.toml`. An implementation
PR names both its W-item and every applicable control (`S1`-`S10`) in its evidence receipt. W13
rejects a release when a required control has no disposition, when an artifact is candidate-derived
but presented as a baseline, or when a prose claim is not backed by a registered executable gate.

All new machine-readable artifacts use UTF-8, stable field ordering when rendered, explicit schema
versions and SHA-256 digests. Every receipt records `release`, `source_sha`, `tested_sha`,
`baseline_sha`, `scenario_digest`, `host_fingerprint`, `toolchain`, `started_at`, `finished_at`,
`result`, and `ship_evidence_eligible`. A missing capability is represented as
`unavailable(reason)` and is red whenever the control declares that capability mandatory. Raw
evidence is immutable; reruns get a new attempt id and never overwrite a failed attempt.

The controls execute in this order:

1. `S9` makes the CI topology bounded and observable before expensive campaigns begin.
2. `S1`, `S2`, `S4`, `S5` and `S7` establish ownership, attribution, statistics, instrumentation
   overhead and the host contract.
3. `S3` authorizes or rejects individual W2-W11 changes from frozen evidence.
4. `S6` governs allocator experiments; `S8` governs upgrade, rollback and mixed-version behavior.
5. `S10` determines the minimum releasable result and the exact claims allowed in W13.

### S1. Machine-readable memory ownership registry

**Outcome.** Every long-lived allocation has one stable owner id and an executable answer to
"who frees this memory, when, under what bound, and which test proves it?" The registry extends the
0.70 inventory from human evidence into a source-checked release artifact. It does not claim that a
static scan can prove runtime lifetime by itself.

**Artifacts and schema.** Add:

- `docs/testing/memory/0.71/ownership-registry.toml`;
- `docs/testing/schemas/memory-ownership-v1.schema.json`;
- `target/memory-evidence/0.71/ownership-inventory.json` as generated evidence;
- `cargo run -p xtask --locked -- memory-owner-inventory --release 0.71` to generate candidates;
- `cargo run -p xtask --locked -- memory-ownership-check --release 0.71` to validate closure.

Each `[[owner]]` record contains `owner_id`, subsystem, owning Rust type, source file and symbol,
allocation sites, keying dimension, retained unit, creation transition, terminal transitions,
cleanup function/guard, count/byte/age bounds, overflow behavior, aggregate snapshot fields,
security classification, focused tests, slow evidence ids and disposition. Dispositions are
`bounded`, `ephemeral`, `external_allocator`, `file_backed`, or `not_applicable(reason)`. A source
candidate cannot be suppressed by a path-only allow list; an exemption identifies the type/symbol,
reviewer, reason and expiry release.

**Implementation.** Implement an `xtask` source inventory pass using the Rust syntax tree rather
than regular-expression counts. Seed candidates from fields and statics containing owning
collections, channels, pools, `Arc`/`Weak`, task handles, semaphores, buffers and explicit allocator
arenas in production crates. Follow type aliases within the workspace and record uncertainty
instead of guessing across macros or external crates. Merge these candidates with the 0.70 retained
snapshots and the runtime site profiles from S2. A registry entry closes a candidate only when its
symbol still resolves and its declared snapshot/test ids exist. Reviewers then trace writers,
clones and terminal transitions and populate the lifecycle fields. A generated coverage report
lists registered, newly discovered, exempted, moved and stale records.

The inventory is rerun for every PR that changes a production Rust file, dependency/feature graph,
capacity setting or channel/pool construction. An optimization that introduces a new owner updates
the registry in the same commit. W1 counters use `owner_id` internally but never expose keys,
tenant ids, connection ids or other unbounded metric labels.

**Tests and canaries.** Add `crates/xtask/tests/memory_ownership_071.rs` with fixture crates that
contain a bounded map, hidden secondary map, `Arc` cycle, channel, detached task, type alias and
macro-generated owner. Assert that unresolved and stale entries fail, a renamed symbol invalidates
the record, expired exemptions fail, and a synthetic unregistered owner is detected. Focused
product tests cited by each record exercise every terminal transition, including cancellation,
unwind and last-handle drop. The S1 canary adds a secondary index without a registry record; both
the inventory check and W1 reconciliation must turn red.

**Gate.** `memory-ownership-check` passes only when every generated candidate has a current
disposition, every `bounded` owner has count and byte limits plus overflow behavior, every cleanup
path and snapshot field resolves, and every referenced test/evidence id is registered. Zero stale
records and zero unreviewed candidates are required on the exact candidate SHA.

### S2. Phase-correlated allocation-site and lifetime profiles

**Outcome.** The release can distinguish allocation rate, live-object retention and allocator page
retention and can point from a growth phase to concrete resolved stacks. A process-level RSS delta
without a phase and allocation-site attribution remains a screen, never authorization for a
rewrite.

**Artifacts and provider interface.** Extend `hydracache-loadgen memory-efficiency` with the fixed
phases `cold`, `fill`, `steady`, `expire_or_delete`, `reset`, `refill`, `post_idle` and `shutdown`.
Write monotonic `phase-timeline.jsonl` markers containing phase id, sequence, owner snapshot digest
and external telemetry checkpoint. Add profiler adapters under `scripts/perf/memory-providers/` for
the selected Linux system-allocator profiler and for each allocator admitted by S6. Each adapter
implements `probe`, `start`, `mark`, `snapshot`, `stop` and `normalize`; it records an exact binary,
tool version/container digest, command and symbol-file digest.

Normalized output under `target/memory-evidence/0.71/profiles/<attempt>/` contains, per phase,
allocation count, allocated bytes, deallocated bytes, live bytes where the provider supports them,
peak live bytes, folded stacks and a baseline-to-candidate stack diff. Preserve unsymbolized frames
and an `unattributed_bytes` field; never redistribute unknown bytes across known stacks. The W0
contract freezes the maximum acceptable unattributed share before candidate results are visible.
Profiler output contains symbols and aggregate sizes only; payloads, keys, credentials and raw
buffers are forbidden and checked before artifact upload.

**Implementation.** Emit phase markers from the orchestrator immediately before and after the same
quiescent checkpoints used by W1. Align profiler samples, allocator fields, process/cgroup samples
and logical owner snapshots by monotonic timestamp and phase sequence. Produce two comparisons:
gross allocations within each phase and live-stack delta between phase exit and the end of the next
cleanup/idle phase. Run a refill of identical cardinality so S6 can distinguish allocator reuse from
new OS mappings. Require debug symbols in the profiling build while keeping optimization, features
and codegen settings otherwise identical to the measured release build; bind both build recipes in
the receipt.

**Tests and canaries.** Add provider contract tests using a fixture binary with one temporary
allocation path, one deliberately retained path, one freed-on-reset path and one background-thread
allocation. Tests verify phase ordering, monotonic timestamps, stack symbol resolution, correct
live/gross classification, shutdown capture, provider failure propagation and secret redaction.
Synthetic missing markers, overlapping phases, a mismatched symbol file and an intentionally
retained vector must fail normalization or produce the expected positive live-stack delta.

**Gate.** W2-W11 implementation is authorized only when the relevant proposal references an S2
phase/stack and an S1 owner id, or records why the bytes are allocator/file-backed rather than a
Rust owner. Required dedicated-host runs reject missing final phases, excessive unattributed bytes,
mixed build identities or a profiler that exits unsuccessfully.

### S3. Evidence-locked stop/go decision gates

**Outcome.** Measurement, implementation and acceptance are separate reviewable decisions. A
promising candidate result cannot retroactively change its baseline, target, workload or regression
budget.

**Artifacts.** Add `docs/testing/memory/0.71/decision-gates.toml` plus a
`memory-decision-check --release 0.71` command. Each `[[proposal]]` has a stable id, W-item,
applicable S-controls, S1 owner ids, S2 stack ids, hypothesis, alternative explanations, immutable
baseline receipts/digests, predeclared primary metrics, minimum effect, allowed regressions,
authorized files/surfaces, decision state, reviewer identity and resulting PR/candidate receipts.

**Decision sequence.** The checker enforces five transitions:

- `D0 baseline-ready`: S1 inventory, S4 statistics contract, S5 overhead, S7 host fingerprint and
  corrected TTL coverage are complete.
- `D1 classified`: the observed delta is classified as live ownership, allocation churn,
  allocator fragmentation/high-water, file/page cache, service overhead, or inconclusive, with
  evidence for rejected alternatives.
- `D2 authorized`: an independent review freezes the baseline, target, workload and regression
  budgets and authorizes only the named W2-W11 surface.
- `D3 implementation-accepted`: focused correctness tests and the predeclared comparison pass. A
  no-win or inconclusive result becomes immutable negative evidence, not a rewritten target.
- `D4 candidate-qualified`: W12 reproduces every accepted result on the exact candidate and W13
  confirms that release claims match accepted proposals.

No W2-W11 representation, allocator, default or resource-policy change starts before its D2
receipt. Safety fixes for a proven unbounded owner may start immediately only through a separate
fail-loud defect receipt; they still cannot claim an efficiency win without D2-D4.

**Tests and canaries.** Unit tests exercise legal and illegal state transitions. Integration
fixtures attempt to use a candidate-derived baseline, alter a scenario after D2, accept an
unregistered source file, reuse a receipt from another SHA/host, omit a rejected alternative and
turn an inconclusive result into `accepted`. Every case must fail with a stable reason code.

**Gate.** W13 requires D4 for every implemented proposal and an explicit negative/deferred
disposition for every investigated proposal. A proposal may not be marked accepted solely because
`size_of`, one RSS sample or a different-host comparison improved.

### S4. Pre-registered statistical and practical-significance contract

**Outcome.** Numerical decisions are reproducible and resistant to hand-selected windows,
candidate-aware thresholds and autocorrelated time-series noise.

**Artifact.** Add `docs/testing/memory/0.71/memory-statistics-v1.toml`, its JSON schema and
`memory-statistics-check --release 0.71`. Freeze and review its digest during D0. It defines primary
and diagnostic metrics, sample cadence, fixed warm-up/settle/idle windows, steady-state eligibility,
minimum repetitions, pairing order, confidence level, multiple-comparison control, missing-row
policy, practical minimum effect and unchanged CPU/latency/throughput budgets. Values are derived
from baseline-only samples; the candidate cannot write or amend this file.

**Method.** Use independently started processes and alternating baseline/candidate order on one S7
fingerprint. Analyze each scenario independently. Run-level rejection is permitted only for a
predeclared infrastructure fault such as identity mismatch, telemetry gap, runner instability or
non-zero workload error; a numerically inconvenient sample remains included. Estimate phase slopes
with the committed robust estimator and moving-block bootstrap sized from baseline autocorrelation.
Report estimate, interval, sample count, window and residual diagnostics. Compare paired
bytes-per-owner/recovery metrics with the committed paired estimator and confidence interval.
Correct the family of primary decisions with the predeclared method; diagnostic metrics are clearly
non-gating. A result must clear both the statistical interval and the absolute/relative practical
effect frozen at D0. "No detected regression" is not evidence of improvement.

The exact estimator implementation, random seeds and numeric precision live in a versioned module,
not an ad-hoc notebook. The report includes every sample and rejected-at-preflight attempt. It does
not silently interpolate telemetry gaps or concatenate fresh processes into one slope.

**Tests and canaries.** Golden time-series fixtures cover flat noise, known positive growth,
warm-up followed by plateau, a late change point, autocorrelated noise, missing final checkpoint,
counter reset, clock regression and one extreme but valid sample. Property tests verify row-order
invariance and deterministic seeded bootstrap output. Canaries select a flattering sub-window,
derive the threshold from candidate data, remove a valid outlier and treat a confidence interval
crossing the frozen limit as green; each must fail.

**Gate.** Every numerical D3/D4 decision references the immutable statistics-contract digest and
includes a machine-readable verdict. Hosted/shared-runner results remain structurally useful but
cannot satisfy this gate.

### S5. Instrumentation overhead and snapshot-coherence budget

**Outcome.** Memory diagnostics neither create the apparent regression nor hide it, and a snapshot
never presents counters captured from unrelated logical epochs as an exact state.

**Modes and implementation.** Provide three explicit modes: `off` (same production binary with
runtime collection disabled), `production` (bounded counters and privileged snapshots), and
`profile` (S2 tooling/build). The `off` and `production` comparison uses the same binary identity;
the profile build is separately identified and cannot supply production sizing numbers. Counters
use checked updates, stable units and bounded storage. Hot paths do not acquire one global memory
lock.

Implement a snapshot coordinator that issues a monotonically increasing snapshot epoch and records
per-subsystem sequence/version before and after collection. An exact gate snapshot is accepted only
at a declared quiescent barrier when all subsystem versions are stable and the workload checkpoint
acknowledges the same epoch. A concurrent diagnostic read may return
`consistency=observed_non_atomic` with its sequence range, but it cannot be used for exact
reconciliation. Counter overflow, unavailable subsystem, retry exhaustion and epoch mismatch fail
loud.

During W0, measure `off`, `production` and `profile` across cold, small-value hot path, tag-heavy,
HC/2 1k-connection and reset cases. Freeze maximum production-mode deltas for resident bytes,
bytes/owner, allocations/op, CPU, throughput and p99 before optimization work. Profile-mode
overhead is reported but is not held to production limits.

**Tests and canaries.** Unit tests cover checked increments/decrements, overflow, replacement
deltas, double cleanup, cancellation and snapshot retry. A concurrency test mutates two subsystems
during capture and proves the result is marked non-atomic or retried to a coherent epoch. A
failpoint omits one subsystem acknowledgement; exact reconciliation must fail. Real-process A/B
tests prove disabled collection retains no background task, channel or growing buffer and that
production mode remains inside its frozen overhead envelope.

**Gate.** W1 is incomplete until the overhead receipt and coherent-snapshot tests pass. W12 rejects
exact-zero/reconciliation claims made from `observed_non_atomic` snapshots or from a profile build
presented as the release binary.

### S6. Allocator fragmentation, arena and reuse diagnosis

**Outcome.** W8 explains why allocated, active and resident memory diverge instead of selecting an
allocator from one RSS number.

**Capability matrix.** Add `docs/testing/memory/0.71/allocator-capabilities.toml`. For system,
jemalloc and mimalloc candidates record target support, feature/build flags, crate/native versions,
license, statistics APIs, profiling provider, purge API and availability of size classes, arenas,
thread caches, dirty/muzzy pages, mapped/retained bytes and peak fields. Unsupported fields remain
`unavailable(reason)`; an allocator can enter D2 only when the primary comparison fields required
for that target are available.

**Implementation.** Expose mutually exclusive build features with a compile-time error for zero or
multiple non-default candidate allocators where an explicit allocator build is requested. Keep the
system allocator as the portable default until D4 and an ADR authorize otherwise. At every S2 phase
capture allocator totals plus supported size-class/arena/thread-cache detail. Record thread count,
arena count, page size, transparent huge-page mode and fresh mapping/page-fault deltas. After reset,
run: fixed idle without purge, identical-cardinality refill, rate-limited explicit purge where
supported, and a second refill. This separates reusable pages from unreusable fragmentation and
quantifies purge latency spikes.

Candidate selection compares at least three fresh processes in W0 and the full W12 repetition
contract. Live logical state must be byte-identical. A resident-memory win is rejected if active
fragmentation, CPU, p99, context switches, static linking, sanitizer, Miri, target builds or license
requirements fail. Arena/thread-cache tuning is a separate proposal from allocator replacement.

**Tests and canaries.** Adapter unit tests validate units, monotonic/non-monotonic field semantics,
overflow and unavailable fields against captured provider fixtures. Compile tests cover every
supported allocator/target combination and mutual exclusion. A synthetic multi-thread allocation
fixture creates size-class fragmentation, proves refill reuse, and verifies purge is bounded and
observed. Canaries hide active/retained fields, compare different logical datasets, change THP or
arena count between samples and report RSS only; schema and D3 checks must reject them.

**Gate.** An allocator/default change requires an ADR, green S6 capability and portability matrix,
the exact same W0/W12 workload and a documented fallback. If no candidate clears every gate, W8
records `measured-no-win` and retains the system allocator.

### S7. Dedicated-host stability and fingerprint protocol

**Outcome.** Numerical evidence comes from one admitted and continuously verified environment;
configuration drift creates a new cohort instead of contaminating a baseline.

**Profile and preflight.** Add
`docs/testing/perf-host-profiles/memory-reference-071-v1.json` and
`perf-memory-preflight --release 0.71 --profile memory-reference-071-v1`. The profile pins or
records hardware model, sockets/cores/SMT, NUMA topology and binding, RAM, firmware/microcode,
kernel/distro, page size, CPU governor, turbo policy, thermal limits, transparent huge pages, swap,
overcommit, KSM, cgroup version/limits, container runtime/storage driver, clock source, filesystem,
allocator knobs and required tool versions. The preflight also records competing load, available
memory, major faults, throttling and temperature before and after calibration.

Run the existing stability calibration before compilation and before each long scenario. Pin the
daemon, load generator and collectors to reviewed CPU/NUMA sets and ensure collectors do not share
the daemon's reserved cores. Disable or pin swap/THP/turbo only through the reviewed host profile;
never mutate the host opportunistically inside a measurement. Baseline and candidate run in
alternating order within one admitted window. Re-read mutable probes after every case and include
their digest in the receipt.

**Tests and canaries.** Test the fingerprint builder against committed `/proc`, `/sys`, cgroup and
runtime fixtures for supported and missing capabilities. Verify canonical hashing is stable across
JSON field order but changes for governor, THP, swap, NUMA, kernel, microcode, cgroup limit or tool
version drift. Integration canaries alter one mutable probe between cases, add synthetic competing
load, exceed the calibration spread and omit the post-run fingerprint; every attempt must become
ineligible rather than retried silently.

**Gate.** Only receipts with the reviewed profile id, exact fingerprint, green calibration,
complete pre/post probes and serialized lease may satisfy S4/W12 numerical gates. WSL2, Docker
Desktop and GitHub-hosted modes always retain `ship_evidence_eligible=false` even when their rows
are complete.

### S8. Upgrade, rollback and mixed-version memory compatibility

**Outcome.** Internal compaction cannot make persisted data, rolling clusters or operator recovery
unsafe. R-4 remains authoritative: migrations are forward-only and idempotent, so "rollback" means
either proven compatibility with unchanged bytes or a loud pre-mutation refusal plus the documented
restore procedure, never an invented reverse migration.

**Compatibility matrix.** Add `docs/testing/compat/memory-071.toml` with exact `v0.70.0` tag/commit,
published package/binary identities, candidate SHA, feature/profile combinations, durable formats,
wire generations and expected outcomes. Default policy is no durable or wire format change for a
pure in-memory representation optimization. Any exception updates `docs/COMPAT.md`, carries a
version marker, reader window, backup requirement, failure mode and ADR before implementation.

**Execution.** Build immutable 0.70 and candidate binaries independently. Exercise: 0.70 create and
0.71 read/mutate/restart; candidate create and candidate restart; candidate-to-0.70 rollback when
bytes remain compatible; otherwise verify 0.70 refuses before mutation and restore the captured
0.70 backup. Run a rolling 0.70/0.71 cluster in every supported role order and verify cache
semantics, generation fencing, invalidation, HC/1/HC/2, sessions and persistence while members are
mixed. Complete the rollout and the permitted rollback/restore path under the same memory limits.
Include snapshot/checkpoint, empty store, maximum-sized record and crash-during-upgrade cases.

Public Rust compatibility remains covered separately by `cargo-semver-checks` and downstream
witnesses. S8 adds runtime/durable evidence; one cannot substitute for the other.

**Tests and canaries.** Unit tests cover version marker parsing, unknown-future refusal and
idempotent migration. Integration tests consume real 0.70 fixtures and independently built
binaries. Canaries silently reinterpret an old key, write before discovering an unsupported
version, mix candidate-generated fixtures into the baseline, and claim rollback after only a fresh
start; each must fail the compatibility receipt.

**Gate.** Every accepted W5-W11 proposal declares `runtime_only` or cites its S8 matrix rows. All
mandatory rows must match their expected success or fail-loud outcome on the exact candidate.

### S9. CI reliability, deduplication and bounded execution

**Outcome.** A red product gate is distinguishable from runner/infrastructure failure, no command
can consume an unbounded six-hour default timeout accidentally, and one commit is not subjected to
duplicate equivalent work merely because `main` and a tag point to the same SHA.

**Workflow topology.** Before W0 campaigns, inventory every workflow trigger, job, dependency,
concurrency key, timeout and artifact consumer in
`docs/testing/memory/0.71/ci-topology.json`. Add
`cargo run -p xtask --locked -- ci-topology-check --release 0.71`. Classify jobs as `core`,
`release-only`, `scheduled-diagnostic`, `manual-protected` or `publish`. A tag release reuses
successful SHA-bound core receipts and executes only release-only work. If policy requires a core
rerun on the immutable tag, the topology records that intentional duplication and publication
consumes exactly one designated admission, not every successful `CI` workflow completion.

Remove tag triggers from workflows with no release consumer, including the Simulator Demo. Scope
legacy release proofs such as HC/2 0.68 admission to their release family or explicit dispatch.
Make crates publication depend on the designated successful tag admission for the exact release,
so a later `main` rerun at an old tagged SHA cannot start another publication attempt. Concurrency
keys include workflow purpose and candidate SHA; superseded diagnostics may cancel, but release and
publication jobs never cancel an already publishing candidate.

Every job declares `timeout-minutes`; every external install, browser download, profiler, soak and
child-process orchestration has a smaller command timeout. Split composite steps such as Management
Console into dependency install, Playwright browser provisioning, build and tests. Wrap long
commands with `scripts/ci/run-with-heartbeat.py`, which writes periodic monotonic progress,
terminates the complete child process tree at the reviewed deadline and emits one of
`product-failure`, `timeout`, `runner-loss`, `tool-unavailable`, `cancelled` or `success`. Silent
automatic retries are forbidden. A manual rerun preserves the failed attempt and receives a new
attempt id.

Define artifact size/retention budgets by lane. Upload compact summaries plus immutable raw
evidence needed for review; reject missing artifacts and expire non-promotable hosted diagnostics
sooner than ship evidence. A preflight estimates runner-hours for the selected campaign and
requires the protected environment for six-hour/24-hour work.

**Tests and canaries.** Static topology tests assert all jobs/time-sensitive commands have
timeouts, release consumers have one producer, artifact names include SHA/run/attempt, and no
equivalent job is triggered by both `main` and tag without an explicit reuse/rerun disposition.
Fixture workflows model duplicate triggers, a missing timeout, an artifact from another SHA and two
publish producers. Integration canaries run a bounded sleeping child, a silent child, a child that
spawns descendants and a failing Playwright provisioning stub; the watchdog must terminate them,
retain logs and classify the reason correctly.

**Gate.** `ci-topology-check`, watchdog tests and a successful bounded dry run must be green before
W0 long campaigns. W13 rejects a candidate qualified by a force-cancelled, timed-out, silently
retried, mixed-attempt or duplicate-consumer lane.

### S10. Minimum releasable result and evidence-based deferral

**Outcome.** Release 0.71 has a finite mandatory core and can publish an honest negative result
without forcing speculative rewrites. Confirmed safety or unbounded-retention defects cannot be
deferred merely to keep the schedule.

**Mandatory foundation.** The following remain ship-mandatory even when no representation or
allocator candidate wins:

1. S1 ownership registry and exact closure check;
2. corrected TTL/fixed-key/reset/refill baseline with S4/S5/S7 receipts;
3. coherent W1 production counters and instrumentation-overhead budget;
4. W2 total retained-byte accounting and pre-allocation count+byte admission for shipped surfaces;
5. W3/W4 bounds and cleanup for every confirmed unbounded owner;
6. per-PR structural memory regression gate plus bounded scheduled/dedicated orchestration;
7. S8 compatibility classification, S9 reliable CI topology, W13 governance and scoped memory
   accounting/sizing documentation.

W5-W11 efficiency proposals are evidence-conditional. Each receives exactly one disposition:
`implemented-and-qualified`, `measured-no-win`, `not-applicable`, or
`deferred(issue,reason,next_evidence)`. A `deferred` disposition is permitted only for an efficiency
opportunity whose current owner is already bounded and correct. A proven unbounded collection,
unreleased logical owner, fail-open admission path, cross-tenant buffer disclosure or incompatible
durable write is a release blocker until fixed or the affected feature is explicitly removed through
the normal compatibility process; it cannot be labeled an optimization deferral.

**Claim construction.** Generate `target/memory-evidence/0.71/release-claims.json` from accepted D4
proposals and negative dispositions. Release notes may claim only the listed surfaces, scenarios,
fingerprints and metrics. If no candidate clears practical significance, the release states that it
improves accounting, bounds and diagnostic confidence but makes no numerical RSS/Redis/allocator
claim. Memory sizing guidance distinguishes measured baseline, enforced budget and illustrative
example.

**Tests and canaries.** Extend release-governance tests to reject a missing proposal disposition,
a deferred safety defect, a numerical claim without D4 evidence, a Redis/Hazelcast universal claim,
an optimization accepted below the practical threshold, and release notes that omit a negative
result. A golden no-win release fixture must pass with the system allocator and existing bounded
representations unchanged; a fixture with one unbounded owner must remain red.

**Gate.** `release-evidence --release 0.71 --require-ship` consumes the generated claim file,
decision ledger and all mandatory foundation receipts. Ship is allowed with zero optional wins but
never with a missing foundation receipt or unresolved safety/retention defect.

### Control-to-work-item dependency map

| Control | Must be complete before | Consumed again by |
| --- | --- | --- |
| S1 ownership registry | D1 and W2-W7/W10-W11 changes | W12 reconciliation, W13 closure |
| S2 phase/site profiles | D1/D2 authorization | W5-W8 acceptance and W12 reproduction |
| S3 decision gates | any W2-W11 implementation | W13 claim generation |
| S4 statistics contract | candidate measurement | W12 numerical verdicts |
| S5 overhead/coherence | W1 completion | every exact-zero and sizing claim |
| S6 allocator diagnosis | W8 implementation | allocator ADR and W12 |
| S7 host protocol | numerical baseline | W12 dedicated evidence |
| S8 compatibility | representation/persistence merge | W13 release admission |
| S9 CI reliability | long campaign or release workflow | all exact-attempt receipts |
| S10 minimum result | release scope freeze | release notes and final ship decision |

### Required implementation sequence and focused verification

Implement the controls in reviewable commits. Do not combine baseline acquisition with a product
optimization, and do not combine an allocator/default change with its statistics-contract change.
The following sequence is mandatory:

1. **CI safety commit:** implement S9 topology validation, explicit timeouts, step splitting,
   heartbeat/watchdog and duplicate-trigger controls. Run the sleeping/process-tree canaries before
   enabling any new workflow.
2. **Contract commit:** add S1/S3/S4/S6/S7/S8/S10 schemas, parsers and intentionally red incomplete
   registries. Parser/schema fixture tests land before production instrumentation.
3. **Observability commit:** implement S1 source inventory, W1/S5 coherent production snapshots and
   S2 phase/provider adapters. Reconcile against the existing 0.70 retained-state snapshot tests.
4. **Baseline commit:** bind immutable 0.70 inputs, run WSL2/GitHub-hosted diagnostic validation,
   qualify the dedicated host and freeze D0 artifacts. This commit changes evidence/contracts only;
   it contains no W2-W11 optimization.
5. **One proposal per change:** create a D1/D2 record, add its red canary/focused test, implement the
   smallest W2-W11 change, record D3, and merge only when correctness and regression budgets are
   green. Negative variants are retained in evidence but removed from production code.
6. **Candidate commit:** freeze the exact candidate, execute S8/W12, resolve every S10 disposition,
   generate claims and run W13 admission. No baseline/statistics/host-contract edit is permitted in
   this commit.

The planned focused test targets and commands are part of the implementation contract. Create the
named target when implementing its control; a differently named target must update this table and
the release-evidence registry in the same review.

| Control | Test target / command | Required coverage |
| --- | --- | --- |
| S1 | `cargo test -p xtask --test memory_ownership_071 --locked` | AST/type-alias/macro fixtures, hidden owner, stale symbol, exemption expiry, registry closure |
| S2 | `cargo test -p xtask --test memory_profile_071 --locked` | provider lifecycle, phase alignment, retained/freed/background stacks, symbol mismatch, redaction |
| S3 | `cargo test -p xtask --test memory_decision_071 --locked` | D0-D4 state machine, frozen digests, unauthorized surface, mixed SHA/host, no-win disposition |
| S4 | `cargo test -p xtask --test memory_statistics_071 --locked` | golden time series, autocorrelation/bootstrap, missing rows, deterministic precision, manipulation canaries |
| S5 | `cargo test -p hydracache --test memory_snapshot_071 --locked -- --test-threads=1` | counter lifecycle, coherent epoch, concurrent mutation, overflow, absent acknowledgement, overhead modes |
| S6 | `cargo test -p xtask --test allocator_matrix_071 --locked` | capability fixtures, units, mutual exclusion, size classes/arenas, refill/purge classification |
| S7 | `cargo test -p xtask --test memory_host_profile_071 --locked` | `/proc`/`/sys`/cgroup fixtures, canonical fingerprint, mutable drift, calibration and lease |
| S8 | `cargo test -p xtask --test memory_compat_071 --locked` | matrix validation, real-binary identities, upgrade, mixed version, rollback or pre-mutation refusal |
| S9 | `cargo test -p xtask --test ci_reliability_071 --locked` | workflow graph, timeouts, watchdog descendants, heartbeat, artifact identity, one publish producer |
| S10 | `cargo test -p xtask --test release_governance_071 --locked` | mandatory foundation, all dispositions, generated claims, no-win green and safety-defect red fixtures |

After every control commit run its focused target plus:

```powershell
cargo check -p xtask --all-targets --locked
cargo clippy -p xtask --all-targets --all-features --locked -- -D warnings
```

When product crates change, replace/add the affected packages in those two commands and run their
focused tests. At each D0-D4 milestone run `cargo xtask verify`. Before merge, candidate freeze and
tagging, run the full W13 DoD block. Long/dedicated cases execute only through their registered
workflow; a local ad-hoc run is diagnostic and cannot be copied into the ship receipt directory.

## W0. Freeze a causal memory baseline before changing code

Add `docs/testing/perf-scenarios/0.71/memory-efficiency-v1.toml`, a typed report schema, and a
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

W0 owns the `D0 baseline-ready` transition. Before that transition it must also produce the S1
ownership inventory, S2 provider qualification and phase timeline, frozen S4 statistics contract,
S5 instrumentation-overhead receipt, admitted S7 host profile and green S9 bounded dry run. The
scenario digest includes all six artifact digests, so changing one creates a new baseline cohort.
The baseline binary is built from immutable `v0.70.0`; the candidate branch may implement tooling
needed to read it, but candidate product behavior or candidate-derived thresholds cannot enter the
baseline distribution.

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

Before using these cases to authorize W2-W11 implementation, execute and archive the transferred
WSL2/Docker campaign for fixed-keyspace, corrected TTL, reset and post-idle. A missing final TTL
checkpoint, dirty checkout, unbound binary/image, incomplete logical-owner snapshot or absent host
fingerprint invalidates the campaign. Its receipt must remain `ship_evidence_eligible=false`; the
campaign closes the 0.70 investigation hand-off but does not satisfy 0.71 dedicated-host gates.

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

The snapshot implements S5 epoch/sequence coherence. Gate-bearing exact snapshots are collected at
a quiescent workload barrier and carry one acknowledged epoch plus stable before/after subsystem
versions. Non-quiescent Admin reads are explicitly marked `observed_non_atomic` and cannot satisfy
reconciliation, exact-zero or sizing gates. Every field maps to one or more S1 owner ids; a counter
without an owner or a registered owner without an observable field is a closure failure unless its
approved disposition is `external_allocator` or `file_backed`.

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
For every public type whose transitive private representation changes, freeze compile-time
`Send`/`Sync`/`Unpin`, object-safety and construction witnesses before the rewrite. The ordinary and
all-feature `cargo-semver-checks` comparisons against `v0.70.0` must remain clean; a smaller layout
does not justify an accidental source-compatibility break.

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

Apply S6 in full: publish the capability matrix, capture supported size-class/arena/thread-cache and
dirty/muzzy detail, and execute no-purge/refill/purge/refill phases under one S7 fingerprint. Treat
allocator replacement and allocator tuning as separate S3 proposals with independently frozen
targets and regression budgets.

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
0.71. A future default change requires its own ADR/migration notice.

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

- per-PR `Memory Regression Fast` structural/unit/property tests, serialized where allocation or
  retained-owner global state is observed. The lane covers allocation-scope isolation, exact
  cache/tag/index cleanup, client-surface diagnostic reset, HC/2 close/session/subscription release,
  and fail-closed final-checkpoint telemetry coverage;
- scheduled 60-minute fixed-keyspace, TTL, reset and HC/2 connection churn;
- six-hour multi-scenario soak for candidate iterations;
- 24-hour same-fingerprint ship confirmation with five serialized successful samples where the
  `0.67.1` reference contract requires them;
- pinned Redis comparison with equivalent persistence/service/cardinality settings;
- Hazelcast comparison only with fixed `-Xms/-Xmx`, JMX heap used/committed/max, GC and native RSS;
  missing JVM telemetry remains unavailable, not inferred.

Every numerical row uses the immutable S4 contract and an admitted S7 lease/fingerprint. Every
accepted optimization is reproduced by proposal id from S3 and includes its S1 owner/S2 stack
attribution. S9 supplies bounded jobs, attempt identity and heartbeat classification; failed,
cancelled or superseded attempts remain in the ledger but cannot contribute samples. Execute an S8
rolling upgrade/rollback-or-refusal matrix before the final 24-hour campaign so incompatible bytes
cannot invalidate an otherwise expensive ship run late.

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
The fast lane detects structural regressions but cannot promote an RSS result or replace any
duration/fingerprint requirement above.

## W13. Governance, documentation and release decision

- Add `docs/testing/release-evidence/0.71.toml` with exact-candidate receipts for W0-W12 and
  `release-evidence --release 0.71 --require-ship`.
- Register profiler, allocator, long-soak, cgroup and cross-target lanes plus dynamic canaries in
  the gated/canary registries; skip-loud and quarantine-expiry rules remain unchanged.
- Add `docs/testing/compat/v0.70.0.json` and a blocking `Public API Compatibility 0.71` lane using
  reviewed `cargo-semver-checks 0.49.0`. It covers the complete publishable-library package set,
  default/all/supported feature profiles, immutable baseline/candidate SHAs and machine-readable
  distinction between a compatibility violation and an analysis/tool failure.
- Extend downstream compile witnesses for auto-traits, object-safe trait objects, public struct
  construction, exported feature names/edges and proc-macro output. Keep Windows/MSRV/target
  consumers mandatory because the primary rustdoc diff is x86-64 Linux-scoped.
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
- Add S1-S10 artifacts and their executable checks to `docs/testing/release-evidence/0.71.toml`.
  Generate `release-claims.json` from D4 decisions rather than writing numerical release claims by
  hand; verify every W5-W11 proposal has the S10 disposition permitted by its safety status.
- Register `ci-topology-check` and watchdog canaries before enabling long-duration workflows. The
  release admission consumes one designated tag-admission attempt and rejects duplicate/mixed
  workflow attempts even when their individual jobs are green.

**DoD:**

```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- memory-owner-inventory --release 0.71 --check
cargo run --manifest-path crates\xtask\Cargo.toml -- memory-ownership-check --release 0.71
cargo run --manifest-path crates\xtask\Cargo.toml -- memory-statistics-check --release 0.71
cargo run --manifest-path crates\xtask\Cargo.toml -- memory-decision-check --release 0.71
cargo run --manifest-path crates\xtask\Cargo.toml -- ci-topology-check --release 0.71
cargo run --manifest-path crates\xtask\Cargo.toml -- memory-contract-check --release 0.71
cargo run --manifest-path crates\xtask\Cargo.toml -- compat-check --release 0.71
cargo run --manifest-path crates\xtask\Cargo.toml -- release-governance-check --release 0.71
cargo run --manifest-path crates\xtask\Cargo.toml -- release-evidence --release 0.71 --require-ship
cargo run --manifest-path crates\xtask\Cargo.toml -- doc-check
```

## Required test and experiment inventory

| Category | Mandatory proof |
| --- | --- |
| Ownership registry | syntax-tree inventory covers every long-lived candidate; symbols/snapshots/tests resolve; hidden-owner, stale-record and expired-exemption canaries fail |
| Phase attribution | cold/fill/steady/cleanup/refill/idle/shutdown markers align with logical, allocator and process samples; retained/freed fixture stacks and secret redaction pass |
| Decision governance | legal D0-D4 transitions pass; candidate baseline, post-freeze scenario edit, mixed SHA/host and inconclusive-as-win canaries fail |
| Statistics | flat/growth/plateau/change-point/autocorrelated/missing-row golden series; deterministic bootstrap and practical-significance verdict |
| Instrumentation | off/production/profile A/B overhead, coherent epoch snapshot, concurrent mutation, missing acknowledgement and overflow tests |
| Accounting | exact insert/replace/delete/expire/evict/flush reconciliation; estimator golden corpus; counter-drift canary |
| Retention | million-mutation bounded-history test; TTL/reset/tag/idempotency/audit churn plateau; cleanup backlog bound |
| Correctness | stale-load fencing, listener gap repair, idempotent retry outcome, mandatory audit fail-closed, lock/session fencing |
| Representation | property/differential corpus over old/new key-entry-index forms; adversarial hash/collision and ABA tests |
| Public API | pinned `v0.70.0` SemVer diff for every publishable library under default/all/supported feature profiles; `Send`/`Sync`/`Unpin`, object-safety, construction, feature and proc-macro downstream witnesses; Windows/MSRV consumers |
| Allocation | allocations/op, bytes copied/op, Miri/sanitizer/fuzz/cancellation, buffer-pool secret isolation |
| Connection | 1k idle, slow consumer, reconnect storm, oversized frame, TLS on/off, exact post-close zero |
| Persistence | anon/file split, memory-limit admission, compaction/checkpoint release, crash/disk-full/backup/restore |
| Allocator | capability fixtures, size-class/arena/thread-cache attribution, identical-state refill reuse, bounded purge and supported-target build matrix |
| Host qualification | canonical fingerprint fixtures, pre/post drift, competing-load/calibration, lease serialization and WSL2/hosted non-promotion |
| Compatibility | real 0.70-to-0.71 upgrade, candidate restart, permitted rollback or pre-mutation refusal/restore, rolling mixed-version cluster and old checkpoint fixtures |
| CI reliability | topology uniqueness, job/command timeouts, process-tree watchdog, heartbeat, SHA/run/attempt artifacts and single publication producer |
| Minimum release | complete mandatory foundation; every optional proposal disposed; no-win release accepted; unbounded-owner and unsupported-claim fixtures rejected |
| Performance | coordinated-omission-safe latency/RPS/CPU plus memory on frozen baseline/candidate, unchanged SLOs |
| Long soak | 60-minute, six-hour and 24-hour tiers with fixed cardinality, slope/recovery statistics and immutable raw artifacts |

## Risk register

| Risk | Failure mode | Control |
| --- | --- | --- |
| Optimize a false leak | Representation churn yields no resident benefit | W0 causal counters and allocator fields before code changes |
| Static inventory misses a runtime owner | Unregistered collection/task/channel retains memory | S1 AST candidates + 0.70 snapshots + S2 runtime stacks and hidden-owner canary |
| Profiler points at the wrong phase or binary | Stack looks causal but belongs to warm-up or another build | S2 phase sequence, binary/symbol digests and live-stack differential fixtures |
| Candidate influences its own gate | Threshold/window is moved after results are visible | S3 D0-D4 ledger + immutable S4 baseline-only digest |
| Statistics manufacture a win | Autocorrelation, selected window or removed sample understates growth | S4 fixed windows, moving-block inference and manipulation canaries |
| Diagnostics distort the result | Counters/profilers create memory or latency regression | S5 off/production/profile A/B and frozen overhead budget |
| Hide memory by lowering capacity | Smaller dataset looks efficient | fixed cardinality/logical bytes and unchanged capacity/workload contract |
| Under-count metadata | Admission says under budget while RSS grows | W1 reconciliation + W2 conservative estimator corpus |
| Break stale-load safety | Generation tombstone removed too early | W4/W6 epoch reference model and resurrection canaries |
| Lose listener correctness | Bounded ring drops events silently | explicit watermark gap + repair; silent-gap canary |
| Lose idempotency | TTL eviction repeats an unsafe mutation | outcome cache, operation retry matrix and ambiguity fail-loud |
| Lose mandatory audit | Bounded sink drops security event | fail-closed sink pressure gate; no drop policy for mandatory events |
| Allocator wins one microcase | Worse CPU/p99/portability in production | full A/B matrix, independent review and system fallback |
| Allocator RSS hides fragmentation | Arena/thread-cache/dirty pages remain unusable | S6 size-class/arena detail and no-purge/refill/purge/refill sequence |
| Host drift invalidates comparison | Governor, THP, NUMA, swap or competing load changes mid-run | S7 canonical pre/post fingerprint and serialized lease |
| Internal compaction breaks rollback | Old binary misreads new durable state or mixed cluster diverges | S8 real-binary upgrade, fail-loud downgrade and rolling-version matrix |
| CI hangs or duplicates release work | Six-hour default timeout or main/tag duplication wastes runners and produces mixed evidence | S9 bounded steps, watchdog, topology graph and one designated admission |
| Scope forces speculative rewrite | Release waits for or accepts an unproven optimization | S10 mandatory foundation plus evidence-based optional dispositions |
| Buffer pooling leaks data | Next tenant observes stale bytes | zeroization/ownership tests and cross-tenant canary |
| Active expiry pauses requests | Tail latency spikes during cleanup | time/count budget, fairness, p99 gate and backlog admission |
| Page cache misread as leak | Wrong code path optimized | anon/file/slab/allocator separation and durable counters |
| HC/2 adds per-client amplification | Empty daemon is small but 1k clients exhaust memory | W10 per-connection byte budgets and reconnect/slow-client soak |
| Historical reports overclaimed | Exploratory numbers become marketing claims | scope labels preserved; only W12/W13 evidence may support sizing |
| Private memory rewrite breaks public Rust API | A compact field removes `Send`/`Sync`, breaks object safety/struct construction or changes a feature edge while unit tests remain green | immutable `v0.70.0` API baseline, pinned `cargo-semver-checks`, explicit downstream witnesses and cross-target/MSRV consumers |

## Gates: definition of done

- Every retained production collection has an explicit owner, count/byte/age bound, overflow
  behavior, metric and cleanup test in the S1 registry; no unresolved inventory candidate and no
  mutation/request/client-indexed append-only collection.
- Every accepted memory change has S2 phase/stack attribution and a legal S3 D0-D4 chain using the
  immutable S4 contract. Instrumentation remains inside S5 overhead limits and every exact snapshot
  is epoch-coherent.
- Dataset capacity accounts for keys, values and material metadata with a reviewed conservative
  error bound; queues/inflight work are admitted by bytes and count before expensive allocation.
- Expiry/delete/evict/tag invalidation/reset clean every logical owner without stale resurrection;
  logical counters reconcile exactly after 100-cycle gates.
- The selected representation/allocator changes beat the frozen baseline on the scoped memory
  targets and remain within unchanged correctness, latency, throughput, CPU and error gates. A
  measured no-win keeps the old implementation and is an acceptable S10 disposition.
- Allocator conclusions include S6 fragmentation/reuse fields on an S7 admitted fingerprint; RSS
  alone cannot select an allocator or authorize purge behavior.
- HC/2 connection/pending/event/subscription/session memory is bounded, fair and released on every
  close/cancel/drain path; 1,000-connection and reconnect/slow-client cases plateau.
- Persistence evidence separates anonymous allocator memory from file/page cache and remains green
  under cgroup pressure, recovery, disk-full, backup/restore and checkpoint tests.
- S8 upgrade, mixed-version and rollback-or-loud-refusal evidence is green for every affected
  runtime/durable surface.
- The 24-hour candidate evidence shows bounded post-warmup behavior under fixed cardinality and
  recovery within the reviewed envelope; unstable/failed samples do not count.
- Redis and Hazelcast are pinned controls with equivalent disclosed profiles; Hazelcast heap is
  measured by JMX or marked unavailable. Neither control defines Hydra correctness or a universal
  memory target.
- Every publishable Rust library passes the pinned `v0.70.0` API comparison for default, all and
  supported feature profiles; required auto-trait/object-safety/construction consumers pass on the
  primary target plus the declared Windows/MSRV matrix. A tool failure is red, not unavailable-green.
- `release-evidence --release 0.71 --require-ship`, all dynamic canaries, `doc-check`, workspace
  gates and compatibility windows are green on the exact candidate SHA. S9 proves the receipts
  come from bounded, non-duplicated designated attempts, and S10 proves every optional proposal and
  release claim has an allowed evidence-backed disposition.

## Final release decision

Ship `0.71.0` only when memory improvements are causally tied to measured owners, every long-lived
collection and connection has an enforced budget, expiry/reset reclaim all logical state, and the
same candidate preserves all existing correctness and performance contracts. A smaller RSS caused
by reduced workload, disabled required semantics, missing evidence, unmeasured page cache, or a
candidate-derived baseline is an automatic no-ship.
