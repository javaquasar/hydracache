# HydraCache 0.74.0 RESP and Native Throughput Efficiency Plan

> **At a glance**
>
> - **What:** remove measured RESP pipeline bottlenecks and shared native-path costs in parsing,
>   response batching, request metadata, time/expiry checks, key ownership, client-store
>   concurrency, multi-key execution, value ownership, and optional durable writes while preserving
>   command order, bounded memory, compatibility, and failure semantics.
> - **Why:** the retained same-host 0.67.1 comparison reached 67.9%/67.6% of Redis for GET/SET at
>   pipeline depth 1 but only 18.1%/18.0% at depth 10. Source inspection identifies a concrete
>   amplification chain: per-command input-buffer compaction, per-response allocation,
>   `write_all` plus `flush` for every reply, repeated translation ownership, and one global
>   `Mutex<BTreeMap<...>>` on the client surface. If matched native and RESP results converge, the
>   shared request/store path becomes the primary optimization target rather than the codec alone.
> - **After (depends on):** published `0.73.0` and its final exact-candidate archive. The frozen
>   0.73 candidate must not be modified to begin this work.
> - **Unblocks:** a defensible node-local RESP throughput improvement, a new same-box Redis
>   comparison with identical semantics, the native/batch baseline required by the 0.75 extended
>   IMap surface, and evidence for later distributed RESP work without turning HydraCache into a
>   Redis clone.
> - **Status:** planned. Historical measurements locate the opportunity; no 0.74 baseline,
>   threshold activation, candidate, or numerical improvement is claimed yet.

Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · gates:
[`../GATES.md`](../GATES.md) · predecessor:
[`V0_73_EVIDENCE_DRIVEN_PERFORMANCE_EFFICIENCY_PLAN.md`](V0_73_EVIDENCE_DRIVEN_PERFORMANCE_EFFICIENCY_PLAN.md).

Read `CLAUDE.md`, `docs/RULES.md`, `docs/GATES.md`, `docs/COMPAT.md`, the final 0.73 release
evidence, and the retained 0.67.1 reference report before implementation. This plan inherits R-1
through R-11. In particular, higher request counts do not justify reordered commands, changed
Redis replies, unbounded buffers, weakened tenant accounting, hidden rejection, or a compatibility
shortcut.

## Measured motivation and claim boundary

The retained 0.67.1 AX42 comparison used the same host, the same pinned `redis-benchmark 7.2.5`
binary, alternating order, and five repeats:

| Operation | Pipeline | HydraCache | Redis | HydraCache / Redis |
| --- | ---: | ---: | ---: | ---: |
| GET | 1 | 59,630 req/s | 87,796 req/s | 67.9% |
| SET | 1 | 59,382 req/s | 87,719 req/s | 67.6% |
| GET | 10 | 136,426 req/s | 757,576 req/s | 18.1% |
| SET | 10 | 132,979 req/s | 746,269 req/s | 18.0% |

Those values describe one old candidate and one exact host. They are hypothesis evidence, not a
0.74 baseline and not a portable Redis-relative claim. The useful observation is the shape: the
gap grows sharply with pipeline depth. Release 0.73 removed a second full-frame RESP output
allocation, but its fixed offered-load qualification did not measure a new saturation knee and did
not change the per-reply flush policy. W0 must therefore repeat the comparison against the
published 0.73 binary before any 0.74 numerical threshold is activated.

The initial source audit identifies these candidate owners:

| Owner | Current seam | Why it can scale poorly |
| --- | --- | --- |
| Input compaction | `RedisRespServer::serve_connection`; `buffer.drain(..consumed)` after every command | Each command can move the unread suffix of a large pipeline, turning one received batch into repeated copies. |
| Output syscalls | `RedisRespServer::write_response`; one encode, `write_all`, and `flush` per reply | Pipeline depth reduces client round trips but HydraCache still performs per-command output work. |
| Translation | `translation_context`, request-id formatting, command/plan/response intermediate values | Ordinary GET/SET can pay allocations and clone state unrelated to the command. |
| Binary key expansion | `redis_key_to_structured_key` | Hex encoding expands nonempty binary keys and creates owned strings before store access. |
| Store serialization | `ClientSurfaceState::store: Mutex<BTreeMap<StoreKey, StoredValue>>` | Independent connections serialize at one lock even when keys and tenants do not overlap. |
| Multi-key dispatch | `RedisExecutionPlan` and `execute_plan` | Per-request vectors and repeated dispatch can retain single-key overhead inside MGET/MSET/DEL/EXISTS. |
| Durable flush | `DurableValueStore::{upsert,remove,...}` and `flush` | Persistence may flush per logical mutation; changing this is a durability-policy decision, not a free RESP optimization. |

Source appearance authorizes measurement only. Every product mutation below still requires a D2
proposal backed by W1 attribution.

## Scope

0.74 covers:

1. a cursor-based bounded RESP input parser that compacts once per received batch;
2. bounded response coalescing with explicit fairness and flush rules;
3. removal of command-independent translation and script-cache work;
4. a separately reviewed binary-key representation proposal with migration/rollback proof;
5. measured client-store sharding with a separate expiry index if global-lock contention is real;
6. vectorized multi-key execution that preserves Redis duplicate and ordering semantics;
7. optional specialized GET/SET decode-to-store and store-to-encoder paths;
8. evidence-driven atomic ordering, buffer reuse, runtime scheduling, and syscall tuning;
9. an optional durability batching proposal that preserves the selected durability contract;
10. same-host HydraCache/Redis and exact-candidate long-run qualification.

## Non-goals

- No Redis Cluster, MOVED/ASK, Redis replication, full command surface, Lua generality, or claim of
  being a Redis replacement.
- No parallel execution of commands from one connection when it could change visible order.
- No weakening of atomic MSET, duplicate-key behavior, TTL visibility, tenant isolation, quotas,
  audit, admission, keyspace events, shutdown, or drain behavior.
- No unbounded input/output pool, cross-connection buffer reuse, retained credential data, or
  buffer growth based only on the largest request ever observed.
- No comparison of HydraCache with TLS/persistence enabled against Redis with those costs disabled.
- No RESP optimization may consume native Rust/client-surface capacity. A better RESP ratio with a
  worse embedded, direct client-surface, HC/1, or HC/2 result is a release regression, not a win.
- No allocator change, map replacement, sharding, relaxed atomic, or fsync change without its own
  measured owner and proposal.
- No edit to the frozen 0.73 product candidate, workload, thresholds, evidence, or tag.

## Identity and decision model

Use distinct identities:

| Identity | Meaning | Permitted use |
| --- | --- | --- |
| `B73` | Published, unmodified `v0.73.0` source and binaries | Compatibility baseline and historical product comparison. |
| `I74` | 0.74 pre-optimization source with accepted measurement instrumentation only | Primary baseline for 0.74 candidates. |
| `I74-profile` | Stack/syscall/lock profiling build | D1 attribution only; never a numerical candidate baseline. |
| `P74-*` | One isolated proposal against `I74` | Focused D3 comparison only. |
| `C74` | One exact composition of accepted proposals | Integrated qualification and release candidate. |
| `R74` | Pinned Redis server/tool/container identity | Same-box reference only, never a HydraCache release gate by itself. |

The decision sequence remains `D0 baseline-ready -> D1 owner-classified -> D2 proposal-authorized
-> D3 focused qualification -> D4 integrated exact-candidate qualification`. A proposal may finish
as `accepted`, `measured-no-win`, `rejected`, `not-applicable`, or `deferred` with a concrete
external blocker. Negative results remain in the evidence archive.

## Work-item sequence

```text
W0 -> W1 -> W2 -> W3
            |     |
            +---> W4 -> W5 -> W6 -> W7 -> W8 -> W9
                                           |     |
                                           +-----+-> W10 -> W11 -> W12
```

W2 and W3 deliberately precede representation and store changes: they attack the source-proven
pipeline amplification without changing stored identity. W4-W9 are conditional and remain separate
proposals so a gain cannot be assigned to the wrong mechanism. W10 integrates only accepted work.

## Global workload and acceptance contract

W0 freezes a finite matrix before candidate measurements:

- RESP2 and RESP3;
- GET and SET at pipeline depths 1, 10, 50, and 100;
- mixed GET/SET and MGET/MSET/DEL/EXISTS at batch sizes 1, 8, 32, and 256;
- concurrency 1, 8, 32, and 128 connections;
- 16-byte, 256-byte, 4 KiB, and 1 MiB values where the command supports them;
- ASCII, empty where legal, NUL/high-bit binary, 16-byte, 256-byte, and maximum accepted keys;
- hit ratios 0%, 50%, 95%, and 100% for read cells;
- plaintext and mTLS as separate cohorts;
- persistence off and each supported persistence policy as separate cohorts;
- malformed, truncated, oversized, disconnect, slow-reader, subscription, and graceful-drain cells.

### Native-first comparison and non-regression contract

W0 freezes native controls beside every affected RESP cell. "Native" is not one pooled number; the
following surfaces remain separate:

1. embedded Rust `HydraCache` GET/insert operations;
2. direct in-process `ClientSurfaceState::dispatch_verified_request` GET/PUT and batch operations;
3. HC/1 over its real supported transport;
4. HC/2 over its real supported transport, with plaintext/mTLS identity kept explicit.

All four surfaces receive an `I74/C74` self-baseline guard. For the same request trace, key/value
corpus, concurrency, hit ratio, duration, and final state, C74 must retain at least 98% of native
goodput and keep native CPU/op and p99 within 3% of I74. Gross allocation per operation may not
regress by more than 5% unless an independently accepted native proposal names and justifies the
new owner. Errors, timeouts, rejections, incomplete operations, logical result, final cardinality,
and resource cleanup must match exactly. These are release-blocking guards, not informational
charts.

The direct in-process client surface is also compared with the RESP facade because RESP delegates
to that surface. Use semantically equivalent GET/SET and MGET/MSET/DEL/EXISTS traces after removing
only protocol framing from the native input. Under equal offered work, the lower bound of the 95%
interval for `native_goodput / resp_goodput` must be at least `1.00`; the upper bounds for
`native_cpu_per_operation / resp_cpu_per_operation` and `native_p99 / resp_p99` must be at most
`1.00`. Native allocation and copied bytes per logical operation must not exceed RESP. If harness
overhead prevents those relationships on I74, W0 must first repair or reject the comparison; it may
not loosen the candidate guard after observing C74.

Embedded Rust, HC/1, and HC/2 remain visible controls but are not required to beat plaintext RESP
across unlike semantics. In particular, authenticated mTLS HC/2 is compared numerically only with
the same native transport/configuration and with an equally secured RESP cohort when a cross-surface
row is reported. No aggregate HydraCache number may hide a harmed native surface.

### Native API optimization ledger

If matched direct-native and RESP results are close, 0.74 treats that as evidence that shared work
below the protocol facade owns the limit. The following ten investigations are mandatory; product
mutation remains conditional on their D1 evidence:

| Id | Investigation | Plan owner | Expected affected shape |
| --- | --- | --- | --- |
| N1 | Lazy successful-request audit metadata instead of unconditional request-id/namespace cloning | W4d | Single-key allocation and CPU/op at all concurrency levels. |
| N2 | One request timestamp, production clock without the test-clock mutex, and a cheap expiry-due gate | W4e | Single-client GET/PUT CPU and p99; expiry storms remain separate. |
| N3 | Remove one global exclusive client-store lock as a concurrency ceiling | W6b | 8-128 independent clients and disjoint tenants/keys. |
| N4 | Stop repeatedly constructing `(String,String,String)` and joining `StructuredKey` segments | W5 | Point lookup allocation/CPU and retained key bytes. |
| N5 | Add a generation-fenced `VerifiedClientSession` so stable identity/namespace validation is reusable | W8a | Repeated trusted native operations without weakening policy changes. |
| N6 | Prove and, where absent, implement `Bytes` ownership transfer for native values | W8b | Large-value PUT/GET copied bytes and allocations. |
| N7 | Execute native and RESP multi-key operations as one validated bounded batch | W7 | MGET/MSET/DEL/EXISTS at 8-256 keys. |
| N8 | Audit and relax only diagnostic `SeqCst` operations with a happens-before proof | W9a | High-rate single-key CPU/op; exact barriers remain unchanged. |
| N9 | Split live read lookup from version-conditional expired-entry cleanup | W6a | Concurrent hit path; expired values still disappear exactly once. |
| N10 | Profile embedded `HydraCache` independently from `ClientSurfaceState` and network APIs | W1 | Distinguishes codec/Moka/policy/event cost from client-surface cost. |

Every row ends with an evidence receipt even if it closes `measured-no-win`. Similar top-line
throughput is not sufficient attribution: W1 must identify CPU, allocation, copy, lock, clock,
expiry, codec, policy-maintenance, event, and scheduling components before a row advances to D2.

The primary 0.74 product claim is selected before D2 from the repeated `B73/I74` baseline. The
expected primary surface is pipeline-10/50 GET/SET goodput at the frozen p99 SLO, but the plan must
record `measured-no-owner` instead if W1 contradicts that hypothesis. Candidate goodput counts
errors, timeouts, rejections, late operations, and incomplete operations in the denominator.

Minimum proposal rules:

- zero unexpected errors, timeouts, protocol differences, command reordering, or owner leaks;
- no more than 2% goodput regression and 3% CPU/op or p99 regression in unaffected frozen cells;
- no increase in configured maximum retained input/output bytes per connection;
- at least five counterbalanced, independently started `I74/P74` pairs for a numerical D3 claim;
- a 95% interval and a preregistered practical minimum effect, never best-sample selection;
- closed-loop `redis-benchmark` remains supplemental; open-loop scheduled-send evidence owns the
  SLO knee and overload claims.

Suggested D2 floors, to be confirmed or tightened by independent review before candidate data, are:

- W2: at least 80% fewer bytes moved by input compaction at pipeline 50;
- W3: at least 80% fewer write/flush calls per reply at pipeline 10 and at least 20% better
  pipeline-10 goodput, with pipeline-1 regression inside 2%;
- W4/W7/W8: at least 20% lower gross allocation or CPU/op in the affected cells;
- W5/W6: at least 20% better high-concurrency or multi-key primary metric with all atomicity and
  tail guards green;
- W9: at least 20% persistence-on goodput improvement with byte-identical recovery outcome and no
  durability weakening.

If the repeated baseline makes a floor impractical or irrelevant, change it only in a reviewed
pre-candidate contract commit and preserve the superseded draft. Never tune a threshold after seeing
candidate measurements.

## W0. Close 0.73 and freeze the 0.74 comparative baseline

**Why:** the 0.67.1 ratios predate the 0.73 encoder and ownership improvements. Starting from them
would attribute old costs to new code.

**Files/artifacts:** create `docs/testing/performance/0.74/{baseline-identities,scenario-matrix,
statistics,proposal-registry,host-profile}.toml`, `tools/resp-pipeline-profile-074/`, and extend the
existing 0.67 load generator rather than creating an incompatible benchmark framework. Add
`cargo xtask performance-contract-check --release 0.74` support in
`crates/xtask/src/performance_contract.rs` and focused contract tests in
`crates/xtask/tests/performance_contract_074.rs`.

**Implementation steps:**

1. Verify the annotated `v0.73.0` tag, peeled commit, release archive, binaries, Cargo.lock, feature
   set, allocator, TLS libraries, and supported-target receipts.
2. Classify every `v0.73.0..I74` runtime delta; only bounded instrumentation may enter `I74`.
3. Rebuild or reuse the exact pinned Redis benchmark/server identity through a reviewed provenance
   row. A version change is a new reference identity, not an in-place update.
4. Freeze CPU placement, IRQ policy, power state, NIC/loopback mode, socket settings, toolchain,
   profile, value/key corpus, connection count, pipeline depth, warmup, duration, run order, and
   shutdown policy.
5. Run at least three fresh B73/I74 screens and the same-box HydraCache/R74 reference cells. Record
   throughput, scheduled latency, CPU/op, context switches, syscalls, bytes copied, allocations,
   lock wait/hold, and queue/buffer high-water values.
6. Run the matched embedded/direct-client/HC1/HC2 native controls and validate the direct-native
   versus RESP relationship before allowing a candidate measurement.
7. Select one primary metric per proposal and seal all thresholds before product mutation.

**Tests/gate:** contract tests reject mismatched Redis tools, different request traces, changed
pipeline depth, unequal TLS/persistence, missing errors, mixed binaries, reordered blocks, missing
final samples, nonfinite metrics, candidate-derived thresholds, unmatched native/RESP traces, or a
pooled native result that omits one surface. One red canary compares secure HydraCache with insecure
Redis and must be rejected; another presents a RESP gain with a 3% direct-native goodput regression
and must reject the candidate.

## W1. Attribute the RESP pipeline cost by stage

**Why:** the source contains plausible owners, but only measurement may authorize W2-W9.

**Where to measure:**

- `crates/hydracache-redis-compat/src/lib.rs`: `serve_connection`, RESP2/RESP3 decode,
  `translation_context`, `execute_plan`, encode, `write_response`;
- `crates/hydracache-server/src/redis_tcp.rs`: accept/task/socket/TLS path;
- `crates/hydracache-client-transport-axum/src/lib.rs`: `ClientSurfaceState`, request dispatch,
  store/idempotency/tenant locks, expiry work;
- `crates/hydracache/src/cache.rs` and the retained embedded performance harness: ordinary native
  GET/insert control path;
- HC/1 and `crates/hydracache-server/src/hc2.rs`: native network API regression controls;
- `crates/hydracache/src/grid/durable_store.rs`: lookup/merge/encode/write/flush stages.

**Measurements:** commands per read, input bytes received, parser-consumed bytes, bytes moved during
compaction, compaction count, decoded objects, request-id bytes, script-cache clones, store lock
wait/hold nanoseconds, locks per command, output frames/bytes, output-buffer high-water, write calls,
flush calls, short writes, pending write duration, yield count, allocations and copied bytes per
stage. Syscall/stack capture stays in `I74-profile`; production counters are accepted only if they
are bounded, label-safe, and pass an instrumentation-off overhead comparison.

For N10, add `tools/native-api-profile-074` with four independently reported paths: raw embedded
GET/put, typed embedded GET/put including codec cost, direct `ClientSurfaceState` GET/PUT, and
`get_or_insert_with` hit/single-flight behavior. Attribute key creation, codec encode/decode, Moka
lookup, policy maintenance, stats, tag/generation access, removal observer, mutation/access events,
load-breaker/single-flight, and Tokio scheduling. Subscriber-off and subscriber-on cells remain
separate. Never compare typed serialization work with a raw RESP byte value and call the difference
cache overhead.

**Tests/gate:** deterministic fixtures must make each counter move for exactly one intended owner.
A hidden extra `flush`, deliberate suffix copy, long-held store mutex, and script-cache clone must
either appear in attribution or make reconciliation fail. Instrumentation overhead itself must stay
inside the 2% goodput and 3% CPU/p99 guards before `I74` freezes. Extend
`crates/hydracache/tests/performance_smoke.rs` only for semantic preflight; numerical evidence comes
from fresh release processes, not test-harness wall-clock assertions. Add allocation/copy fixtures
for raw versus typed values and a canary that intentionally charges codec bytes to Moka; the owner
classifier must reject it.

## W2. Cursor-based input parsing and one compaction per batch

**Current seam:** `serve_connection` extends a `Vec<u8>`, decodes the first command, and executes
`buffer.drain(..consumed)` inside the command loop. A deep pipeline can repeatedly move the unread
suffix.

**Change:** maintain `read_start` and decode `&buffer[read_start..]`. Advance the cursor after each
complete frame. Compact only when returning to read, when reclaimed prefix crosses a frozen
threshold, or before a bounded growth decision. Reset indices without allocation when the buffer is
fully consumed.

```rust
struct RespReadBuffer {
    bytes: Vec<u8>,
    start: usize,
}

impl RespReadBuffer {
    fn unread(&self) -> &[u8] { &self.bytes[self.start..] }
    fn consume(&mut self, count: usize) -> Result<(), DecodeLimitError>;
    fn compact_before_read(&mut self);
}
```

No borrowed command may survive a buffer mutation or `.await`. RESP dialect changes caused by
`HELLO` take effect for the next command in the same pipeline. Malformed frames retain the existing
error/close boundary; QUIT retains the committed-response boundary; configured frame and read-buffer
limits remain authoritative.

**Files:** `crates/hydracache-redis-compat/src/lib.rs`; existing
`tests/resp_boundaries.rs`, `tests/resp_resource_smoke.rs`, and a new
`tests/resp_pipeline_buffer_074.rs`; `tools/resp-pipeline-profile-074/src/main.rs`.

**Tests:** split every golden command at every byte boundary; partial pipeline across reads; 1/10/50/
100 commands in one read; mixed RESP2/HELLO RESP3 pipeline; malformed middle frame; maximum frame;
empty suffix; disconnect; QUIT; subscription transition; bounded-capacity reuse. A counting buffer
fixture proves one compaction per read cycle rather than per command. Fuzz the cursor/compaction
state against the existing decoder oracle.

**Acceptance:** semantic bytes are identical, buffer capacity remains bounded, copied suffix bytes
clear the preregistered floor, and pipeline-1 allocation/p99 do not regress.

## W3. Bounded output coalescing and connection fairness

**Current seam:** `write_response` creates one encoded `Vec`, calls `write_all`, and calls `flush`
for every command. Pipeline input therefore does not become pipeline output efficiently.

**Change:** add an internal connection-local output batch and an encoder that appends to it. Execute
commands in order, append replies in order, and write when any frozen boundary is reached:

- no more immediately decodable commands remain;
- maximum replies per turn is reached;
- maximum encoded batch bytes is reached;
- a reply larger than the normal batch must make bounded one-item progress;
- QUIT, protocol error, subscription acknowledgement/event, graceful drain, or shutdown requires
  prompt delivery under the existing contract.

The initial private limits are proposed as 256 replies and 1 MiB encoded bytes, subject to W0/D2
review before candidate data. They are bounds, not promises to retain that much memory. Oversized
legal replies are written directly or through a one-item bounded path. After each command quantum,
the connection yields so a large pipeline cannot starve other sockets.

```rust
struct RespWriteBatch {
    bytes: bytes::BytesMut,
    replies: usize,
}

impl RespWriteBatch {
    fn append(&mut self, value: RespValue, dialect: RespDialect) -> Result<(), RedisCompatError>;
    fn should_flush(&self, next_len: usize) -> bool;
    async fn flush_to<W: AsyncWrite + Unpin>(&mut self, writer: &mut W) -> io::Result<()>;
}
```

**Files:** `crates/hydracache-redis-compat/src/lib.rs`, optionally a private `src/io_batch.rs`,
`tests/resp_resource_smoke.rs`, new `tests/resp_pipeline_write_074.rs`, server lifecycle/drain tests,
and the 0.74 profile tool.

**Tests:** a scripted `AsyncWrite` records write/flush calls, partial writes, Pending transitions,
disconnects, and injected errors. Assert byte-identical concatenated replies for RESP2/RESP3,
pipeline boundaries, auth failure, malformed input, HELLO, QUIT, subscriptions, slow readers,
oversized response progress, cancellation, and graceful drain. A fairness test runs one pipeline-1000
connection beside latency-sensitive pipeline-1 connections and checks the frozen p99/fairness guard.

**Acceptance:** the preregistered write/flush reduction and pipeline throughput floor pass without
larger retained bounds, changed wire bytes, lost committed responses, reordered replies, or a
pipeline-1/tail regression.

## W4. Remove command-independent translation work

W4 is split into independently measurable proposals; failure of one does not block the others.

### W4a. Script cache lookup without full-map cloning

`translation_context()` currently clones the loaded-script collection while preparing ordinary
commands. Refactor translation so GET/SET/MGET/MSET/DEL/EXISTS/TTL paths do not copy script state.
`EVALSHA` and `SCRIPT EXISTS` use a bounded read guard or immutable `Arc` snapshot. Do not hold a
standard mutex across `.await`, and keep script visibility/order semantics unchanged.

### W4b. Request identity and plan allocation

Profile request-id formatting, `RedisCommand -> RedisExecutionPlan -> ClientRequestEnvelope`, and
response reduction. Reuse validated namespace/tenant handles per connection where they are
immutable. Replace formatting only if audit uniqueness and redaction remain exact. Avoid a hidden
global ID bottleneck and do not remove request identity from diagnostics.

### W4c. Direct encoding API

Add `extend_encode_resp_value(&mut BytesMut, ...)` so W3 appends directly without a temporary
per-response vector. Retain the public convenience functions by delegating to the same encoder.
RESP2/RESP3 goldens, error strings, null shapes, binary payloads, and maximum-frame behavior remain
byte-identical.

### W4d. Lazy audit and successful-request metadata

`ClientSurfaceState::handle_request` currently clones `envelope.request_id` and materializes
`request_namespace(...).as_str().to_owned()` before knowing whether rejection audit is needed.
Replace this with a borrowed `RequestAuditContext` that creates owned strings only when an audit
event is actually committed. Successful GET/PUT must retain the same dispatch and mutation metrics;
authorization, quota, policy, and malformed-request failures must retain byte-identical audit
fields and redaction.

The candidate must not defer an audit event beyond the lifetime of borrowed request data and must
not retain the request envelope. Test successful and every rejected branch with an allocation
counter, then compare the exact audit sequence and serialized fields to the canonical path. A
canary that drops one quota/auth failure audit must fail both the audit model and conformance gate.

### W4e. One timestamp and a cheap expiry-due gate

The current request path can call `now_ms()` before the sweep and again inside GET/TTL/mutation
handling. Its production path also checks a mutex-backed test-clock option and uses `SeqCst` for the
time floor and sweep schedule. Introduce a request-scoped `RequestTime` captured once and passed to
all command helpers. Represent the normal system clock without a mutex; the injected deterministic
clock remains explicit and fully testable. Use a cheap atomic due check before taking the expiry
cursor/store locks, while retaining compare-and-claim behavior so concurrent requests do not run
the same ordinary sweep.

Clock monotonicity, deadline behavior, TTL boundary visibility, backward wall-clock movement,
cursor fairness, forced complete diagnostics, and tenant quota release are unchanged. Tests freeze
time immediately before/at/after expiry, race many requests at the sweep boundary, move the injected
clock backward, and prove exactly one claimed ordinary sweep. Acceptance requires a measured CPU or
lock reduction on non-expiring GET/PUT with no delayed logical expiry outside the existing bound.

**Files/tests:** `crates/hydracache-redis-compat/src/lib.rs`, codec modules if extracted,
`tests/resp_boundaries.rs`, `tests/redis_mined_edge_corpus.rs`, `tests/redis_clients.rs`, and new
`tests/translation_ownership_074.rs`; native N1/N2 coverage belongs in new
`crates/hydracache-client-transport-axum/tests/native_hot_path_074.rs`. Allocation fixtures
distinguish script-empty/script-populated GET/SET from EVALSHA and successful requests from every
audited rejection. Acceptance requires an affected-cell allocation or CPU win and no semantic,
audit, TTL, clock, or contention regression.

## W5. Binary-safe key representation with an explicit compatibility bridge

**Current seam:** `redis_key_to_structured_key` maps a byte key to a prefixed hex string. This is
safe and compatible but expands key bytes and allocates before client-store lookup. The shared
native path then calls `StructuredKey::stable_key()`, which joins `Vec<String>` segments into a new
`String`, and constructs `StoreKey = (String, String, String)` for lookup. Thus N4 applies even when
RESP translation is absent.

**Decision first:** W5 begins with an ADR and owner measurement. Compare:

1. retain the existing canonical string and optimize only temporary ownership;
2. add a binary `StructuredKey` variant while retaining the exact old canonical identity;
3. use an internal interned/hashed handle with collision-checked original bytes;
4. defer because the migration cost exceeds the measured gain.

Also evaluate a representation-only native candidate that caches an immutable validated stable key
inside `StructuredKey`, or carries canonical tenant/namespace/key handles into the store, without
changing wire serialization. Measure construction once versus every lookup, string comparison
bytes, hash cost, and retained overhead. A cached representation is accepted only when repeated
lookups amortize its retained bytes; one-shot keys remain a control.

Any representation used in a wire frame, durable record, event, tag index, snapshot, audit, or
diagnostic is registered in `docs/COMPAT.md`. If old and new binaries can touch the same store, use
dual-read/single-write with an idempotent migration marker, or refuse before the first incompatible
write. Rollback must either read the new representation or fail loudly before mutation. Hash-only
identity, collision ambiguity, lossy UTF-8, and silent prefix reinterpretation are forbidden.

**Files:** `crates/hydracache-redis-compat/src/lib.rs`, structured-key definitions in the client
protocol/core crates, `crates/hydracache-client-transport-axum/src/lib.rs`, tag/event adapters,
durable codecs only if evidence authorizes them, `docs/COMPAT.md`, and a new ADR.

**Tests:** empty/NUL/high-bit/long/prefix-collision keys; published 0.63/0.65/0.72/0.73 fixtures;
old-write/new-read, new-write/restart, mixed binary operation, same-disk rollback or pre-mutation
refusal; tag lookup/invalidation; TTL; tenant isolation; audit redaction; fuzzed collisions; maximum
key; MSET duplicate keys. A canary that aliases two byte keys or reinterprets an old prefix must fail.

**Acceptance:** copied/retained bytes and CPU improve by the registered minimum, compatibility is
fully green, and no migration is accepted merely to improve a microbenchmark.

## W6. Split the read path and shard the client store only with measured evidence

**Current seam:** `ClientSurfaceState` owns one `Mutex<BTreeMap<StoreKey, StoredValue>>` plus related
quota, idempotency, and expiry state. Even a live GET uses a mutable exclusive store reference
because lookup may discover and remove an expired value. Replacing it without an ownership model
could break atomic batches or cleanup.

### W6a. Read-mostly live lookup with version-conditional expiry cleanup

Measure live-hit lock hold separately from expired cleanup. A candidate may read under a shard/read
guard and return a live immutable `Bytes` clone immediately. If it observes expiry, it records the
entry version/generation, acquires the mutation path, and removes only if the same version remains
expired. Quota release, mutation counters, and expiry events occur exactly once after successful
conditional removal. A delayed cleanup cannot remove a replacement under the same key.

The read guard may not be held while acquiring isolation/quota mutation state or publishing events.
Tests cover expiry-versus-replace, expiry-versus-invalidate, two concurrent expired readers,
backward/forward injected time, cancellation between observation and cleanup, and version wrap or
fail-loud handling. Loom/deterministic schedules must reproduce an intentionally unconditional
cleanup canary and show it deleting the replacement before the fixed candidate is accepted.

### W6b. Stable store sharding

**Design candidates:**

- stable power-of-two shards selected from tenant, namespace, and full key bytes;
- an immutable shard table and independently locked value maps;
- a separate bounded expiry structure/cursor so value sharding does not lose fair TTL progress;
- ordered multi-shard acquisition for atomic batches, with duplicate shards deduplicated first;
- per-tenant quota reservation before mutation and exact rollback on any failed shard;
- quiescent diagnostics that aggregate all shards without presenting a mixed snapshot as exact.

The hash seed/build identity is recorded where it affects repeatability, but shard placement is not
wire or durable authority. Never use a per-key task or unbounded shard count. A sharded design must
show a concurrency win; single-thread map speed alone is insufficient.

W6b starts only if W1 still measures material wait after W6a. `RwLock` versus sharded mutexes is a
measured choice, not a source-style preference. Empty-shard memory and single-client overhead are
part of the decision.

**Files:** `crates/hydracache-client-transport-axum/src/lib.rs`, optionally private
`src/store.rs`/`src/expiry.rs`, existing client-surface/accounting tests, and new
`tests/store_sharding_074.rs` plus model/property tests.

**Tests:** same/different shard concurrent GET/SET; adversarial collisions; duplicate batch keys;
cross-shard atomic MSET failure; lock-order deadlock model; concurrent expire/replace/delete/reset;
cursor wrap; quota rollback; idempotency; cancellation/poison recovery; exact snapshot; 100 rewrite
cycles. Loom or deterministic scheduler coverage exercises lock ordering. A forced reverse-order
canary must deadlock-detect or fail the model gate.

**Acceptance:** W6a must improve the measured live-read owner without changing cleanup semantics.
W6b must improve high-concurrency throughput/lock-wait by the registered minimum, while pipeline-1,
memory per empty shard, expiry p99, atomicity, and exact accounting stay within bounds. If contention
is not material after W6a, W6b closes `measured-no-win` and the single store remains.

## W7. Vectorize MGET, MSET, DEL, and EXISTS

**Why:** a network pipeline and a Redis multi-key command are different batching levels. Both must
avoid repeating single-key framework cost.

**Required semantics:**

- MGET returns one result per input position, including duplicates and misses, in input order;
- MSET validates the whole batch first, is atomic, and uses the existing last-write-wins rule for
  duplicate keys;
- DEL counts a key only when it is actually removed; repeated keys cannot inflate removals;
- EXISTS follows the supported Redis duplicate-count behavior and existing conformance fixtures;
- tenant quota, TTL, audit, idempotency, mutation events, tag cleanup, and admission are unchanged.

**Change:** add explicit batch operations at the `ClientSurfaceState` seam. Validate identity and
namespace once, group shards once if W6 is accepted, reserve final deltas once, execute under a
bounded lock set, and emit the same ordered response. Do not parallelize within one connection if
that would reorder its commands or events.

The native batch API is the primary implementation; RESP translation delegates to it rather than
building a second batch engine. Precompute canonical keys and final last-write-wins quota deltas
once. Avoid calling `stable_key()` repeatedly for sizing, admission, lookup, audit, and event
publication. Allocate the result vector exactly once and reuse input position metadata rather than
cloning keys into a second command list.

**Files:** client request/response definitions only if the existing batch variants are insufficient;
`crates/hydracache-client-transport-axum/src/lib.rs`, `crates/hydracache-redis-compat/src/lib.rs`,
conformance manifest/fixtures, `tests/client_surface_conformance.rs`, and new
`tests/redis_batch_execution_074.rs`.

**Tests:** reference-model sequences with duplicates, misses, expired keys, quota boundaries,
mid-batch injected failure, cancellation, concurrent readers, maximum batch, binary keys, and event
ordering. Run the pinned Redis oracle only for the declared supported semantics. Acceptance requires
fewer dispatches/locks/allocations and the D2 multi-key throughput floor with every compatibility
row unchanged.

## W8. Verified native session, value ownership, and optional GET/SET fast path

### W8a. Generation-fenced `VerifiedClientSession`

Add a typed session object only after W1 measures repeated identity/namespace/protocol work. Session
creation validates the client identity, tenant mapping, namespace, protocol capabilities, and
static limits once, then stores immutable handles. Any mutable isolation/auth/policy source exposes
a generation; each operation compares a cheap generation token and revalidates before proceeding
when it changes. Revocation therefore takes effect no later than the existing request boundary.

Per-request deadlines, idempotency keys, quotas, admission, audit, mutation events, and request IDs
remain request-scoped. The session is `Send`/`Sync` only if its handles are, owns no unbounded
history, and releases all tenant/session owners on drop. The existing envelope API delegates to the
same canonical operation core so the fast path cannot fork semantics.

Tests change tenant authorization and limits between two session calls, revoke access concurrently,
reuse a namespace, cross tenants deliberately, cancel/drop a session, and compare audit/metrics/
results with ordinary dispatch. Semver/API docs and compile tests cover the public surface if the
session is exported; an internal-only first candidate is allowed.

### W8b. `Bytes` ownership and zero-copy native values

Create an ownership matrix for raw native PUT/GET, typed embedded PUT/GET, direct client-surface,
RESP, HC/1, and HC/2. Record where `Vec<u8>` becomes `Bytes`, where refcount clones occur, and where
serialization necessarily creates new bytes. The raw native path should accept owned `Bytes`, move
it into `StoredValue`, and return a cheap immutable `Bytes` clone. Do not promise zero copy for typed
codec serialization or network encoding.

Add pointer-sharing test hooks behind the existing profiling/test feature, plus allocation tests at
0, 64 B, 4 KiB, and 1 MiB. Replacement, expiration, eviction, event fan-out, quota accounting, and
caller drop must not mutate or prematurely free shared bytes. Any new public raw-value method must
use the existing capacity/admission semantics and cannot expose mutable backing storage.

If the current path already satisfies this ownership contract, W8b records `measured-no-win` and
adds only the proof; it must not manufacture a new wrapper API.

### W8c. Specialized GET/SET execution with canonical fallback

W8c starts only after W2-W7 and W8a/W8b settle, because otherwise it can obscure their ownership.
Profile the
remaining hot GET/SET chain:

```text
wire bytes -> decoded frame -> RedisCommand -> plan -> ClientRequestEnvelope
           -> ClientResponse -> RespValue -> encoded batch
```

A candidate may use a short-lived borrowed command view and direct response encoder for simple
GET/SET while delegating every option, error, extension, script, subscription, and unsupported form
to the canonical path. SET must transfer or copy owned value bytes exactly once before the read
buffer can mutate. Borrowed credentials or payloads never cross `.await`, enter another connection,
or remain in a reusable buffer after command completion.

**Files/tests:** `crates/hydracache-client-transport-axum/src/lib.rs`, client-protocol types only if
the session is public, parser/translator internals in `hydracache-redis-compat`, shared semantic
reducer, new `tests/verified_session_074.rs` and `tests/native_value_ownership_074.rs`, and golden
differential tests that execute every supported GET/SET form through both paths. Include malformed
and auth corpora, Miri for lifetime-sensitive helpers, fuzz, cancellation, redaction, policy-
generation, and pointer-ownership tests. A feature/test switch forces the slow canonical path so
differential testing cannot accidentally compare the fast path with itself.

**Acceptance:** the registered allocation/CPU floor passes for simple GET/SET; every response byte,
mutation result, event, counter, error, and audit record matches the canonical path. Otherwise W8 is
removed without affecting W2-W7.

## W9. Runtime, syscall, buffer, and durability follow-ups

W9 contains small independent proposals and one explicitly separate durability proposal.

### W9a. Atomic ordering audit

Review per-command metrics and request-id atomics currently using `SeqCst`. Weaken ordering only
with a written happens-before argument and concurrency test. Authority, admission, lifecycle, and
exact-snapshot atomics retain stronger ordering where required. A microbenchmark-only change without
measurable CPU benefit is rejected.

Classify every candidate atomic as diagnostic counter, uniqueness generator, monotonic time floor,
expiry claim, lifecycle owner, publication barrier, or exact-snapshot barrier. `dispatch_attempts`,
independent rejection counters, and non-synchronizing statistics may be candidates for `Relaxed`;
time floors, sweep ownership, removal accepted/acknowledged, generation fences, and quiescent
accounting require their existing or explicitly proven ordering. Add litmus tests and a source
registry so a broad mechanical replacement cannot pass review.

### W9b. Connection-local buffer reuse

Reuse bounded read/write capacity inside one connection. Shrink only at a reviewed idle/request
boundary and never on every request. Clear sensitive bytes before any cross-principal reuse; the
default design has no cross-connection pool. Track capacity high-water and prove close drops it.

### W9c. Scheduler and socket behavior

Measure Tokio poll time, task wakeups, voluntary/involuntary context switches, read/write syscalls,
short writes, socket buffer occupancy, and TLS record behavior. Tune `TCP_NODELAY`, write batching,
task budget, or socket sizes one factor at a time. Platform defaults remain unless an admitted-host
win and supported-platform correctness matrix pass.

### W9d. Optional durable group commit

Persistence-off Redis comparisons cannot authorize a durability change. For each supported durable
mode, determine whether multiple already-accepted mutations may share one flush without changing
the documented acknowledgement point. If the current contract requires persistence before reply,
the batch waits for the same durable boundary; it may share a flush but may not acknowledge early.
Bound operations, bytes, and maximum wait; fail every affected command loudly on flush failure.

### W9e. Linux allocator follow-up after owner attribution

Consume, but do not reinterpret, the 0.73 allocator deferral. Open a new allocator proposal only
after a supported dedicated-Linux profiler reports allocated, active, resident, retained, arena,
thread-cache, and purge/reuse behavior for the accepted W2-W8 pipeline candidate. Compare the
system allocator, the exact supported mimalloc version, and jemalloc only where the target matrix
supports them, using identical binaries apart from the mutually exclusive allocator feature.

An allocator candidate must improve a named pipeline or churn owner rather than RSS alone. It must
also keep CPU/op, p99, virtual memory, thread count, post-idle refill behavior, binary size, startup,
and every supported-target build inside preregistered bounds. Windows and macOS retain explicit
system fallbacks; an allocator unavailable on one supported target cannot silently become its
default. Purging is measured as a separate policy from allocator replacement. Without the complete
Linux owner telemetry, W9e closes `deferred` and the system allocator remains unchanged.

**Tests:** atomic litmus/model tests, sensitive-buffer overwrite checks, close/reconnect capacity
reconciliation, syscall fixtures, slow reader, TLS/plaintext separation, crash before/during/after
group flush, ENOSPC, torn/corrupt recovery, byte-identical restart, mutually exclusive allocator
feature checks, and allocation/reuse fixtures. W9d requires a COMPAT/durability review even if no
durable format changes; W9e requires the full supported-target allocator build matrix.

## W10. Compose accepted candidates and rerun interaction cells

Create a composition ledger containing proposal id, baseline SHA, patch/tree digest, touched owners,
tests, primary metric, regression metrics, compatibility class, reviewer, and rollback. Compose in
this order unless a pre-candidate review changes it:

1. W2 input cursor;
2. W3 output coalescing;
3. accepted W4 translation/encoder proposals;
4. W5 key representation if accepted;
5. W6a read-path split and W6b store sharding if accepted;
6. W7 vectorized multi-key operations;
7. accepted W8 verified-session, ownership, and fast-path proposals;
8. accepted W9 proposals.

After every step rerun the touched semantic suite and focused measurement. W2+W3 share connection
buffer ownership; W3+W4c share encoding; W5+W6+W7 share key/store ownership; W3+W9c share syscall
behavior. Each combination therefore needs an explicit interaction cell. Isolated percentage gains
are never added together.

Freeze `C74` only after every proposal has a terminal disposition and the full runtime/dependency/
feature/file ledger closes. Any later runtime, dependency, threshold, workload, instrumentation, or
profile change creates a new candidate identity.

## W11. Focused, same-box Redis, compatibility, and long-run qualification

Run four separate evidence families:

1. **I74/C74 product comparison:** counterbalanced open-loop cells establish goodput-at-SLO,
   CPU/op, p99, allocations, copied bytes, syscalls, lock wait, and bounded resources.
2. **C74/R74 reference comparison:** the same pinned closed-loop tool, host, dataset, command,
   connection, pipeline, TLS, and persistence settings. Report both absolute values and ratio only
   with the full identity. Redis is context, not the pass/fail oracle.
3. **B73/C74 compatibility:** real published binaries and fixtures cover RESP2/RESP3, all supported
   commands, binary keys, auth/rediss, subscriptions/events, tenant limits, TTL, restart, rolling
   mixed binaries where supported, same-disk rollback or fail-loud migration, and graceful drain.
4. **Native-first guard:** matched embedded Rust, direct client-surface, HC/1, and HC/2 `I74/C74`
   cells enforce their individual non-regression budgets. The direct client-surface versus RESP
   rows additionally enforce native goodput, CPU/op, p99, allocation, and copied-byte dominance on
   equivalent operations. Results are reported per surface and per command; none are pooled away.

Long-run qualification uses the exact `C74` and equal-duration `I74`: focused five-pair evidence
first, then six-hour qualification, then a separately authorized 24-hour confirmation only if the
six-hour packet passes independent hash/guard review. Freeze offered work, pipeline distribution,
connection churn, payloads, hit ratio, seed, phase schedule, calibration, estimator, thresholds,
and artifact budget before dispatch. No automatic retry and no transfer across a package/source
identity change.

The long run must include pipeline-1 latency-sensitive traffic alongside pipeline-10/50 throughput
traffic so batching cannot hide starvation. Include slow readers, reconnects, subscriptions, TTL,
multi-key commands, and persistence as separately visible sub-surfaces. One aggregate win cannot
hide a harmed command, protocol, or native surface.

## W12. Release evidence, documentation, and rollback

Add `docs/testing/release-evidence/0.74.toml`, release-scoped expected-red canaries, immutable raw
artifact manifests, nested SHA-256 verification, package/SBOM/advisory/license receipts, and
`docs/releases/0.74.0.md`. Update performance documentation with exact scope and retain all negative
results.

Every accepted change documents:

- activation/default state and whether restart is required;
- input/output/store memory bound;
- wire, durable, public API, and configuration impact;
- oldest compatible reader/writer and mixed-version behavior;
- emergency disable or old-binary rollback procedure;
- which evidence must be repeated after a dependency or source change.

The release note may claim only metrics whose exact `C74` artifact passes D4. It must state that
HydraCache and Redis have different product goals, distinguish node-local RESP from distributed
capacity, and avoid portable or universal superiority language.

## Concrete file map

| Area | Existing files expected to change | New focused coverage/artifacts |
| --- | --- | --- |
| RESP connection and codec | `crates/hydracache-redis-compat/src/lib.rs` and optional private modules | `resp_pipeline_buffer_074.rs`, `resp_pipeline_write_074.rs`, `translation_ownership_074.rs` |
| TCP/TLS hosting | `crates/hydracache-server/src/redis_tcp.rs` | focused server lifecycle pipeline/fairness tests |
| Client store | `crates/hydracache-client-transport-axum/src/lib.rs` and optional private store/expiry modules | `store_sharding_074.rs`, property/model tests |
| Native performance controls | `crates/hydracache/src/cache.rs`, embedded cache harness, client-surface dispatch harness, HC/1 and HC/2 process harnesses | `tools/native-api-profile-074`, matched `native-vs-resp-074` traces and per-surface non-regression receipts |
| Batch semantics | Redis translator plus client request/state code | `redis_batch_execution_074.rs`, conformance manifest rows |
| Key compatibility | structured-key/core/protocol definitions only if W5 is accepted | ADR, `docs/COMPAT.md`, cross-version fixtures |
| Durability | `crates/hydracache/src/grid/durable_store.rs` only after W9d authorization | crash/ENOSPC/group-flush tests and receipts |
| Profiling | existing loadgen/0.67 comparison framework | `tools/resp-pipeline-profile-074`, `docs/testing/performance/0.74/*` |
| Governance | `crates/xtask/src/performance_contract.rs`, release evidence code | `performance_contract_074.rs`, canary/evidence registry |

This is an expected ledger, not blanket authorization. Each D2 proposal records the exact subset
before mutation; unrelated dependency or formatting churn is excluded.

## Fast and scheduled gates

Focused implementation cadence:

```powershell
cargo test -p hydracache-redis-compat --locked
cargo test -p hydracache-client-transport-axum --locked
cargo test -p hydracache --locked
cargo test -p hydracache-server --test server_lifecycle redis --locked
cargo check -p hydracache-redis-compat --all-targets --locked
cargo clippy -p hydracache-redis-compat --all-targets --locked -- -D warnings
cargo check -p hydracache-client-transport-axum --all-targets --locked
cargo clippy -p hydracache-client-transport-axum --all-targets --locked -- -D warnings
cargo test -p xtask --test performance_contract_074 --locked
cargo xtask performance-contract-check --release 0.74
```

Milestone and pre-tag gate:

```powershell
cargo xtask verify
cargo deny check
cargo xtask doc-check
cargo xtask release-evidence --release 0.74 --require-ship
```

Scheduled/pre-release gates additionally run decoder fuzzing, Miri for accepted borrowed-buffer
helpers, loom/deterministic lock-order tests if W6 is accepted, pinned real Redis oracle/client
interop, real plaintext and mTLS processes, same-box Redis comparison, compatibility replay,
six-hour qualification, and 24-hour confirmation. A missing required external capability is a red
gate or an explicitly recorded blocker, never a silent pass.

## Final release decision

0.74 ships only when all of the following are true:

- the published 0.73 predecessor and exact `I74`/`C74` identities are closed;
- W1 attributes the owners and every W2-W9 proposal has a terminal evidenced disposition;
- accepted pipeline changes preserve byte-for-byte replies, order, fairness, bounded memory,
  disconnect/drain behavior, and zero-error execution;
- embedded Rust, direct client-surface, HC/1, and HC/2 each pass their own I74/C74 non-regression
  guard, and the matched direct-native path does not lose to its RESP wrapper;
- accepted representation/store/batch changes preserve key identity, TTL, atomicity, quotas,
  tenant isolation, audit, events, restart, and rollback;
- focused comparisons, integrated interaction cells, compatibility, six-hour qualification, and
  24-hour confirmation pass on the same frozen candidate;
- release evidence, packages, SBOM, dependency policy, supported targets, documentation, and
  immutable artifact hashes are green.

If only W2/W3 pass, 0.74 may ship as a narrow pipeline-efficiency release. If no candidate clears
its practical effect and regression gates, publish the measurements and defer the changes rather
than weakening thresholds. The tag must resolve to the measured `C74`, not a documentation-only
substitute.
