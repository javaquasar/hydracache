# HydraCache 0.72.0 Management Center 2.0 & Operational Visibility - Codex Execution Plan

> **At a glance**
> - **What:** replace the current single-page, read-only `0.57` console with a locally bundled,
>   multi-view HydraCache Management Center. The release adds an operator dashboard, members and
>   partitions, HC/1/HC/2/RESP clients, namespaces and caches, deterministic health checks,
>   persistence/operation/audit views, bounded browser-local history, and an optional guarded
>   Prometheus history adapter. Every view is backed by a versioned, bounded, privacy-reviewed
>   `/management/v1` API and an honest `live`/`modeled`/`unavailable`, fresh/stale,
>   complete/partial contract.
> - **Why:** the existing console renders one `/cluster/overview` snapshot and a narrow `/metrics`
>   strip. HydraCache already records substantially richer topology, replication, resharding,
>   retained-state, admission, client, persistence, and repair signals, but operators cannot
>   correlate them safely. A richer interface is useful only if it cannot fabricate cluster-wide
>   truth, leak tenant data, grow without bound, or show an unreachable member as healthy.
> - **After (depends on):** `0.71.0`. The management plane consumes the retained-owner counters,
>   corrected byte accounting, cleanup contracts, and fixed-cardinality memory methodology from
>   0.70/0.71. UI and schema work may be prepared while the 0.71 dedicated run is in flight, but
>   0.72 ship evidence uses the published 0.71 artifact and the exact frozen 0.72 candidate.
> - **Unblocks:** an operator-grade, evidence-backed management experience; supportable production
>   diagnostics; and the final 1.0 operations/distribution rehearsal without adding a write-capable
>   control plane.
> - **Status:** planned (implementation progress W0-W11; W12 next).
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - compatibility: [`../COMPAT.md`](../COMPAT.md) -
> current console contract: [`../management-center.md`](../management-center.md).

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md),
[`docs/GATES.md`](../GATES.md), and [`docs/COMPAT.md`](../COMPAT.md) first. This is an
**observe-first management-plane release**. It may add read-only diagnostic APIs and locally
bundled UI assets. It must not add an unreviewed state-changing browser action, use wall time as
distributed authority, widen metric label cardinality, expose cache keys or authentication
material, or silently turn missing evidence into a green state.

Implementation progress: W0-W11 are implemented on `feat/0.72-management-center-2`; their source,
tests, bounds and falsifiability canaries are registered in the 0.72 evidence manifests.

## Existing baseline and evidence boundary

The release begins from what the repository actually implements:

| Area | Current state | 0.72 gap |
| --- | --- | --- |
| Browser UI | One static `console/index.html`, `app.js`, and `style.css`; duplicated as embedded server assets | Componentized TypeScript application, deterministic build/embed path, routed pages, responsive and accessible shell |
| Overview | `/cluster/overview` with members, leader, partition summary, aggregate consistency, backup age, lifecycle | Freshness/completeness envelope, cluster aggregation, quorum/term/epoch, repair/replication/reshard/admission and resource panels |
| Metrics | Browser fetches Prometheus text from `/metrics` | Typed snapshot fields for UI; bounded local time series; optional fixed-query Prometheus history |
| Members | ID, role, reachability, generation | Version/config digest, resource snapshot, client/partition counts, drain state, partial/stale truth |
| Clients | Aggregate HC/2 accounting exists; no management list | Bounded privacy-safe HC/1, HC/2, RESP inventory and lifecycle/accounting summaries |
| Namespace/cache | `TenantStatus`, cache counters and retained-state snapshots exist in separate modules | Auth-scoped, bounded namespace/cache summaries without keys, tokens, SQL, or raw tenant secrets |
| Health | Metrics and state exist, but no deterministic health catalogue | Typed rules with PASS/WARN/FAIL/UNKNOWN/DISABLED, evidence, affected scope and remediation |
| Operations | Admin POST endpoints and point status exist outside the UI | Read-only operation/persistence/audit views; browser remains unable to invoke mutation routes |
| Tests | Static marker check plus six Playwright scenarios | Unit/component/schema/property/security/accessibility/process/fault/soak/upgrade coverage and falsifiability canaries |

The current `/cluster/overview` is a point-in-time observation, not a linearizable read. 0.72 must
not relabel it as authoritative cluster state. Where a source does not yet exist, the screen shows
`unavailable(reason)` or omits the field; placeholders and modeled data are never colored green.

## Source reflection: use the Hazelcast reference deliberately

The supplied Hazelcast Management Center screenshots contribute interaction patterns, not code,
branding, data semantics, or a compatibility promise.

| Adopted pattern | HydraCache interpretation |
| --- | --- |
| Persistent navigation and cluster selector | Dashboard, Members, Clients, Namespaces/Caches, Partitions, Health, Persistence, Operations, Metrics, Audit |
| Status cards plus charts and resource table | Quorum/leader/epoch, replication/repair/reshard, hit ratio/admission, bounded history, honest per-member resources |
| Searchable member/client tables | Server-paginated and bounded rows with stable sorting, explicit partial/stale state and privacy-safe identifiers |
| Health-check catalogue | Deterministic HydraCache-specific checks with evidence and remediation, not a copied checklist |
| Dense resource drill-down | Progressive disclosure and responsive tables; keyboard and screen-reader equivalents are mandatory |

Do not copy Hazelcast-only entities into the product. `MultiMaps`, lists, sets, Jet, PN counters,
CPMaps, scripting, WAN replication labels, Hazelcast version fields, and Hazelcast-specific memory
models are not HydraCache capabilities. Their nearest real HydraCache concepts are namespaces,
caches, consistency, replication/repair, HC/2 sessions, fenced locks, persistence, and protocol
clients. A navigation entry exists only when a real, tested source exists.

## Source reflection: use NATS for recovery and cluster truth

The local [NATS reread](../../../nats-server/NATS_HYDRACACHE_REREAD.md) and
[knowledge base](../../../nats-server/NATS_KNOWLEDGE_BASE.md) contribute reliability patterns, not
messaging features or implementation code. NATS keeps route discovery separate from Raft-backed
JetStream authority, reports lost/repaired storage state, preserves a previous valid snapshot until
replacement, reconciles local disk discoveries against authoritative desired state, and explains
placement rejection causes. 0.72 adopts only their read-side consequences:

| NATS pattern | 0.72 interpretation |
| --- | --- |
| Connectivity is separate from authority | Show `discovered`, `transport_authenticated`, `admitted`, `raft_voter/learner`, `caught_up`, and `serving` as distinct formation states |
| Placement returns causal rejection detail | Show a bounded deterministic placement trace with stable reason codes, topology epoch and truncation marker |
| Recovery distinguishes clean/repaired/lost/refused | Show a versioned recovery outcome and bounded lost/corrupt counts; UNKNOWN never becomes PASS |
| Commit and apply are different progress | Display commit/apply lag and do not mark committed-but-unapplied work complete |
| Local recovery cannot revive deleted authority | Add stale-peer catch-up/no-resurrection to the process fault matrix |

Storage layout, checkpoint-plus-tail snapshot construction, crash-safe file replacement and new
decoder fuzz targets are captured as post-0.72 durable-store hardening. This release observes and
tests existing sources; it does not introduce a custom segment store, replace raft-rs, or expose a
repair action in the browser.

## Non-goals and hard boundaries

- No write-capable console controls in 0.72. Drain, reshard, backup, diagnostic reset, compaction,
  membership changes, scripting, config edits, and cache mutation stay outside the browser.
- No hidden time-series database, browser persistence, service worker cache, or unbounded in-memory
  series. Browser-local history means a documented ring since the page was opened.
- No arbitrary PromQL, arbitrary upstream URL, generic HTTP proxy, or credential forwarding from
  browser JavaScript. Optional Prometheus access is server-side and allowlisted.
- No browser-to-member fan-out. The server derives peers only from authenticated committed
  membership and applies concurrency, deadline, page, and byte bounds.
- No metric labels containing cache keys, tenant IDs, namespaces, connection/session IDs, member
  IDs, partition IDs, error strings, or URLs. Diagnostic detail belongs in bounded snapshots.
- No claim that aggregate consistency counters are per-level, that backup acceptance is durable
  completion, that a modeled source is live, or that absence of a response is healthy.
- No autoscaling, new storage engine, new transport/protocol, distributed RESP value plane, live
  restore engine, or redesign of Raft authority.
- No numerical performance or memory claim self-baselined from the candidate. Budgets are frozen
  from the 0.71 release and W0 pre-change baseline before candidate results are visible.

## Cross-cutting contracts

### Observation envelope

Every `/management/v1` response uses one schema and carries enough metadata for the browser to be
honest without inventing state:

```rust
pub struct ManagementEnvelope<T> {
    pub schema_version: u16,
    pub observation_seq: u64,
    pub authority_epoch: Option<u64>,
    pub captured_at_unix_ms: u64, // presentation/freshness only; never distributed authority
    pub source: ObservationSource, // Live | Modeled | Unavailable
    pub completeness: Completeness, // Complete | Partial
    pub stale_after_ms: u64,
    pub warnings: Vec<ManagementWarning>,
    pub data: T,
}

pub struct BoundedPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<OpaqueCursor>,
    pub truncated: bool,
}
```

`observation_seq` and `authority_epoch`, not wall-clock order, prevent snapshot regression.
Unknown schema versions fail loud with an `unsupported-schema` screen. Unknown enum values remain
unknown and cannot fall through to success. Every list has a validated `limit`, stable order,
opaque expiring cursor, total response-byte ceiling, and explicit `truncated` flag.

### Truth rendering matrix

| Source | Freshness | Completeness | Allowed presentation |
| --- | --- | --- | --- |
| live | fresh | complete | normal value/status color |
| live | fresh | partial | value plus visible PARTIAL badge, missing-member count and warning |
| live | stale | any | last observation plus STALE badge; never green health |
| modeled | any | any | MODELED badge and neutral styling; never “live” or PASS |
| unavailable | any | any | unavailable reason and retry state; no previous value masquerading as current |

The browser keeps the last response only to explain a transition. It must not merge values from
different authority epochs or replace a newer sequence with an older one.

### Data classification

| Class | Examples | Rule |
| --- | --- | --- |
| Cluster-safe | counts, roles, versions, bounded resource totals, health IDs | available on the internal admin listener with `management.read` |
| Member diagnostic | node ID/generation, config digest, drain/reachability details | exact committed member only; paginated; redacted errors |
| Tenant-scoped | namespace/cache names, quota and retained-byte summaries | caller must be authorized for that scope; cross-scope totals only after aggregation |
| Sensitive/forbidden | keys, values, SQL, auth tokens, client cert subject, raw remote address, lock token, raw audit payload | never serialized by management DTOs or emitted in logs/evidence |

Every DTO uses an allowlist serializer. Reusing an internal debug structure with sensitive fields
is forbidden even if the UI currently ignores those fields.

### NATS-derived cluster formation contract

Cluster formation is not one boolean and must not be encoded as one. The API reports independent
dimensions so a peer can be reachable but unauthenticated, authenticated but not admitted, or
admitted but not caught up:

```rust
pub struct ClusterFormationSnapshot {
    pub schema_version: u16,
    pub node: OpaqueNodeIdentity,
    pub generation: u64,
    pub candidate_observation_seq: Option<u64>,
    pub authority_epoch: Option<u64>,
    pub discovery: DiscoveryState,       // Unknown | Absent | Discovered
    pub transport: TransportState,       // Unknown | Unreachable | Reachable | Authenticated
    pub admission: AdmissionState,       // Unknown | Pending | Admitted | Rejected
    pub consensus_role: ConsensusRole,   // None | Learner | Voter
    pub catch_up: CatchUpState,           // Unknown | Behind | Current
    pub serving: ServingState,            // Unknown | Blocked | Serving | Draining
    pub blocker: Option<FormationReasonCode>,
    pub source: ObservationSource,
    pub completeness: Completeness,
}
```

The exact Rust ownership may differ, but the wire distinctions are mandatory. Allowed stable
blocker codes include `cluster-identity-mismatch`, `duplicate-node-identity`,
`generation-regression`, `transport-unreachable`, `peer-unauthenticated`, `protocol-incompatible`,
`not-admitted`, `learner-behind`, `quorum-unavailable`, `authority-unknown`, `draining`, and
`source-unavailable`. Human text is localized/presentational and never parsed by automation.

Formation invariants:

1. Discovery is soft evidence and cannot imply admission, consensus role or serving.
2. Transport reachability cannot imply authentication.
3. Only committed Raft membership at the reported authority epoch can yield `Admitted` and
   `Learner|Voter`.
4. `Current` requires an applied position known to satisfy the authoritative catch-up target;
   successful socket exchange is insufficient.
5. `Serving` requires admitted identity/generation, compatible protocol, required authentication,
   current state and a non-draining runtime.
6. Regression to an older observation sequence or authority epoch is rejected, not merged.
7. Missing input produces `Unknown` with a reason; it never produces the most optimistic state.
8. Wall-clock timestamps explain duration only and cannot order distributed membership events.

### NATS-derived placement decision contract

The Management Center does not recompute placement in JavaScript. It renders a bounded trace
produced by the same server-side placement source used by the control plane, or reports the source
as unavailable. The trace is diagnostic evidence, not a second placement authority:

```rust
pub struct PlacementDecisionTrace {
    pub schema_version: u16,
    pub trace_id: OpaqueTraceId,
    pub topology_epoch: u64,
    pub request_digest: RedactedDigest,
    pub requested_replicas: u16,
    pub constraints: BoundedPlacementConstraints,
    pub candidates: BoundedPage<PlacementCandidateDecision>,
    pub selected: Vec<OpaqueNodeIdentity>,
    pub outcome: PlacementOutcome, // Rejected | Proposed | Committed | Applied | Stale | Unknown
    pub commit_index: Option<u64>,
    pub applied_index: Option<u64>,
    pub reason: Option<PlacementReasonCode>,
}
```

Per-candidate reason codes include `wrong-cluster`, `wrong-role`, `not-admitted`, `unreachable`,
`stale-generation`, `protocol-incompatible`, `capability-missing`, `capacity-insufficient`,
`zone-conflict`, `host-conflict`, `required-label-missing`, `excluded-label`, `already-selected`,
`ignored-by-plan`, and `quorum-would-be-unsafe`. The trace records all relevant reasons up to a
frozen candidate/detail/byte limit and sets `truncated=true` when the limit is reached.

Placement invariants:

1. Candidate membership is bound to one committed `topology_epoch`; observations from another
   epoch are excluded or make the trace partial.
2. Candidate and reason ordering is stable. Input hash-map order cannot change selection or bytes.
3. Any tie-break is a pure function of the committed request/topology plus an explicit recorded
   seed; no ambient RNG or wall clock is allowed.
4. `Proposed`, `Committed`, and `Applied` remain distinct. Missing indexes cannot be represented as
   zero, and `applied_index <= commit_index` is validated when both are known.
5. A stale trace cannot be shown as the current plan after the topology epoch advances.
6. Labels, capacities and failure-domain facts are allowlisted summaries; raw config and secrets
   never enter the DTO or evidence bundle.
7. The UI can filter and explain a trace but cannot create, mutate, retry or commit placement.

### NATS-derived durable recovery contract

The Persistence view consumes a retained recovery record rather than inferring recovery from
process liveness. A proposed wire shape is:

```rust
pub struct DurableRecoveryStatus {
    pub schema_version: u16,
    pub scope: RecoveryScope,             // Node | Namespace | Region | RaftMetadata
    pub outcome: RecoveryOutcome,         // Clean | Repaired | Degraded | Refused | Unknown
    pub phase: RecoveryPhase,             // Discover | Validate | Rebuild | Reconcile | Complete
    pub artifact: RecoveryArtifactKind,
    pub artifact_format_version: Option<u32>,
    pub validated_watermark: Option<RecoveryWatermark>,
    pub records_checked: BoundedCount,
    pub records_recovered: BoundedCount,
    pub records_discarded: BoundedCount,
    pub corrupt_records: BoundedCount,
    pub repair: RepairState,               // NotRequired | Pending | Running | Complete | Failed
    pub reason: Option<RecoveryReasonCode>,
    pub started_at_unix_ms: Option<u64>,
    pub completed_at_unix_ms: Option<u64>,
}
```

Stable reasons include `artifact-missing`, `unsupported-format`, `checksum-mismatch`,
`truncated-artifact`, `foreign-identity`, `stale-watermark`, `timeout`, `io-error`, `disk-full`,
`repair-source-unavailable`, `repair-failed`, `reconciliation-failed`, and
`status-not-retained`. Counts saturate and carry a `truncated/overflowed` indicator rather than
wrapping. Paths, keys, values, checksums usable as credentials, raw errors and peer certificates are
forbidden.

Recovery outcome rules:

| Outcome | Required evidence | Serving implication |
| --- | --- | --- |
| `Clean` | all required artifacts validated, no discarded/corrupt authoritative data, reconciliation complete | may serve if independent cluster readiness also passes |
| `Repaired` | damage was detected, bounded loss/repair facts retained, authoritative repair completed and revalidated | may serve only under the documented post-repair gate |
| `Degraded` | usable verified subset exists but repair/debt remains or a non-authoritative derivative was discarded | health cannot be PASS; serving policy is explicit per scope |
| `Refused` | authoritative artifact is corrupt, foreign, unsupported or incoherent | must not publish/serve the affected authoritative state |
| `Unknown` | source missing, stale, partial or status not retained | never interpreted as clean or healthy |

The status record itself is versioned and bounded. If it is persisted, its format is registered in
`docs/COMPAT.md`; if it is memory-only, restart explicitly yields `Unknown/status-not-retained`.
0.72 does not quietly add a durable artifact merely to improve the screen.

### Consensus progress and no-resurrection contract

The Management Center carries `commit_index`, `applied_index`, last installed snapshot index and
catch-up target as separate optional values bound to the same node generation and authority epoch.
`apply_lag = commit_index - applied_index` is computed only when both values are complete and
coherent. A committed command, accepted backup or started repair never maps to `Completed`.

Recovery publication follows one logical transaction:

```text
validated local artifacts
+ authoritative Raft snapshot
+ committed tail after that snapshot
-> desired metadata state
-> complete local reconciliation plan
-> atomic publication of the new observation
```

Local data discovered on disk cannot recreate a removed member, namespace, replica assignment,
generation or epoch. Until reconciliation completes, aggregation reports partial/recovering state;
it cannot publish a mixture of old local state and new authoritative metadata as complete.

### Scope fence for the NATS-derived work

In 0.72 these contracts are read-side schemas, aggregation rules, health rules, UI states and proof
obligations. The release does not add a custom segment/block store, change `DurableValueStore`
format, replace raft-rs, redesign placement, add checkpoint-plus-tail snapshot optimization, expose
automatic repair, or add any browser mutation. If a required live source is absent, the release
ships an honest `Unavailable/Unknown` state or narrows the claimed field; it does not implement a
new authority mechanism to make the page look complete.

## Implementation map and dependency graph

```text
W0 contracts/threat model/baseline
 ├─► W1 frontend build + shell
 ├─► W2 management schema + bounded routes ─► W3 authenticated aggregation
 │                                      ├─► W4 dashboard/history
 │                                      ├─► W5 members/partitions
 │                                      ├─► W6 clients
 │                                      ├─► W7 namespaces/caches
 │                                      ├─► W8 health checks
 │                                      ├─► W9 optional Prometheus history
 │                                      └─► W10 persistence/operations/audit
 └─► W11 security/accessibility/supply chain (applies to W1-W10)
W1-W11 ─► W12 process/fault/soak/performance proof
W2-W12 ─► W13 compatibility/package/upgrade/publication rehearsal
W0-W13 ─► W14 evidence ledger, canary meta-gate, docs and ship admission
```

Each work item is incomplete until its positive tests, degradation tests, bound tests, cleanup
tests, and named red canary exist. Screenshots are supplementary evidence only.

## W0. Freeze the information architecture, threat model, baselines and evidence schema

**Problem.** A large UI can outrun the available truth sources and produce attractive but
unverifiable claims. Security, cardinality, memory, latency, and accessibility boundaries must be
fixed before product code.

**Design.** Add `docs/architecture/management-center-v2.md` containing the page/field/source map,
observation state machine, permissions, data classification, trust boundaries, sequence diagrams,
and an endpoint budget table. Freeze two baselines: unmodified published `v0.71.0` and an exact
pre-feature 0.72 baseline SHA. Record current bundle bytes, endpoint latency/bytes, server retained
owners, browser heap/DOM nodes, FD/task counts, and console test coverage. Define evidence and
canary registries before implementation:

- `docs/testing/management-center/0.72/source-map.toml`
- `docs/testing/management-center/0.72/bounds.toml`
- `docs/testing/canaries/0.72-management-center.toml`
- `docs/testing/release-evidence/0.72/manifest.toml`

The source map names every visible field, authoritative producer, aggregation rule, privacy class,
stale policy, test owner and fallback. Any unowned field is excluded.

For the NATS-derived scope, W0 performs a code-level availability audit before DTO design. It maps
every formation dimension to discovery/transport/admission/Raft/runtime sources; every placement
field to the committed topology and existing placement evaluator; every recovery field to
`RecoveryReport`, `DurableScrubReport`, durable Raft open/apply or backup verification; and every
progress field to coherent Raft term/epoch/commit/apply/snapshot observations. Each row states
whether the source is authoritative, observational, modeled or absent, its lifetime, consistency
boundary, maximum cardinality, redaction policy and behavior across restart.

W0 also freezes:

- stable formation, placement and recovery enum/reason catalogues;
- candidate/reason/count/string/response byte bounds and saturation behavior;
- health truth tables and threshold units/evaluation versions;
- exact old/current reader windows and capability names;
- the W12 fault-taxonomy inventory with links to existing 0.64/0.66 proofs;
- per-feature canaries and the machine-readable feature-to-proof rows;
- which proposed facts remain `Unavailable` because adding their source would change consensus,
  placement or a durable format.

**Implementation steps.** Inventory current `ClusterOverview`, `TenantStatus`, retained-state and
metrics descriptors; enumerate route/auth boundaries in `admin_http.rs`; capture desktop/tablet/
mobile interaction specifications; freeze page/query/response/sample/DOM limits and polling
backoff; create the threat model covering XSS, confused-deputy fan-out, SSRF, enumeration,
cross-tenant disclosure, stale replay, response amplification and denial of service.

**Tests and proof.** Extend `xtask doc-check` with parsers that require one source-map and one
canary row per declared management field/route. Add a fixture with an unowned field and a fixture
with a sensitive field; both must fail. Run the old console tests against `v0.71.0`, store raw
outputs and hashes, and verify the baseline receipt on a clean checkout.

Add governance fixtures for a formation field backed only by discovery, a recovery field inferred
from liveness, a placement reason without a deterministic evaluator, a committed value mislabeled
applied, an unbounded reason list and a NATS fault category without source evidence. Each fixture
must fail with the exact missing authority/bound/test/canary classification.

**Canary `MC72-W0-UNOWNED-FIELD`.** Insert a rendered field without producer/privacy/test metadata;
the registry gate must turn red.

## W1. Build a deterministic TypeScript component shell and single asset pipeline

**Problem.** The current global JavaScript file and duplicated source/server assets do not scale to
multiple views and can drift during packaging.

**Design.** Use strict TypeScript, Vite and a small pinned component runtime (Preact is the preferred
choice; W0 records the final ADR) with locally bundled dependencies and no CDN/runtime fetch. Add a
typed router, persistent HydraCache navigation, cluster selector, status banner, query cache,
error boundary, responsive layout and shared table/chart/status primitives. Use a lightweight
canvas chart library only after bundle/license review. The build emits content-hashed files and a
`dist/manifest.json`; the server embeds that exact output rather than maintaining a second copy.

**Planned files.** `console/src/{app,router,api,state}.ts[x]`, `console/src/components/`,
`console/src/pages/`, `console/vite.config.ts`, `console/tsconfig.json`, and an xtask that verifies
the embedded manifest/digest under `crates/hydracache-server/console/`.

**Tests.** Vitest covers reducers, routing, schema errors, query cancellation, polling backoff and
every component state. Playwright covers direct/deep links, refresh, keyboard navigation, narrow
viewport, offline/reconnect, unsupported schema, partial/stale/modeled/unavailable banners and
DOM bounds. A package test rebuilds twice from the lockfile and compares asset digests after
normalizing declared build metadata. The production bundle must contain no external URL or source
map with workspace paths.

**Canary `MC72-W1-ASSET-DRIFT`.** Alter one embedded byte after build; package verification must
fail before the server binary is accepted.

## W2. Add the versioned, bounded and privacy-safe Management API

**Problem.** `/cluster/overview` and Prometheus text are insufficient contracts for a multi-page UI;
exposing internal state structs would couple the browser to implementation and risk leakage.

**Design.** Create public-within-server DTOs in `hydracache-observability` and same-origin read-only
routes in `hydracache-server`:

```text
GET /management/v1/capabilities
GET /management/v1/summary
GET /management/v1/members?limit=&cursor=
GET /management/v1/formation?limit=&cursor=
GET /management/v1/placement-traces?limit=&cursor=
GET /management/v1/placement-traces/{opaque_id}
GET /management/v1/clients?protocol=&limit=&cursor=
GET /management/v1/namespaces?limit=&cursor=
GET /management/v1/caches?namespace=&limit=&cursor=
GET /management/v1/partitions/summary
GET /management/v1/healthchecks?category=&status=&limit=&cursor=
GET /management/v1/persistence
GET /management/v1/persistence/recovery?scope=&limit=&cursor=
GET /management/v1/consensus/progress?limit=&cursor=
GET /management/v1/operations?limit=&cursor=
GET /management/v1/audit?limit=&cursor=
GET /management/v1/history?...                 # only when W9 capability is enabled
```

The legacy overview remains readable for its compatibility window. New wire shapes, enum policy,
version negotiation and golden JSON fixtures are registered in `docs/COMPAT.md` during
implementation. Capabilities drive navigation; absence of a capability hides the view with an
explanation instead of producing a failing request loop.

**Tests.** Unit tests cover serialization, every enum/source combination, pagination/cursor
expiry, stable order, limit clamping/rejection, response byte ceiling, redaction and unsupported
versions. Golden tests lock canonical JSON. HTTP tests cover content types, GET/HEAD/OPTIONS policy,
unknown query parameters, malformed UTF-8, cancellation, compression limits and error bodies.
Property tests generate adversarial identifiers and nested error strings and assert that forbidden
material never appears.

For formation, placement, recovery and progress routes, golden sets include every current enum and
reason code, absent optional values, saturated/truncated counts, partial source, old/current/unknown
schema and unknown future enum. Cross-field validators reject serving without admission/currentness,
applied greater than commit, `Clean` with corruption/loss/incomplete reconciliation, applied
placement without applied index, and a candidate from the wrong topology epoch. HEAD returns no
body; OPTIONS does not grant write methods; unsupported methods return the documented error without
invoking a source adapter.

**Canary `MC72-W2-LEAK-OR-OVERSIZE`.** A fixture places a key/token and an over-budget member list
inside the internal source; the DTO must redact/reject it and set `truncated`, while a mutant that
serializes the internal struct must fail.

## W3. Implement authenticated, deadline-bound cluster snapshot aggregation

**Problem.** A request received by one node does not automatically constitute cluster truth.
Browser fan-out would create SSRF, topology leakage, incompatible snapshots and unbounded work.

**Design.** Add a local snapshot provider and a protected member-to-member management snapshot
RPC. The receiving server fans out only to the current committed member roster over the existing
authenticated cluster channel; it never accepts member URLs from browser input. Apply fixed
concurrency, per-peer and whole-request deadlines, response-byte budgets, cancellation propagation
and one in-flight refresh per snapshot class. Deduplicate by `(node_id, generation,
authority_epoch, observation_seq)`. Missing, late, incompatible and duplicate peers produce
partial evidence; they are not retried until green or substituted with zero.

The aggregation path is observational only: it cannot propose Raft entries, change membership, or
delay the data plane. Cache only the last bounded immutable snapshot for the declared TTL and drop
it on epoch regression or incompatible schema. Attribute all retained bytes/tasks/FDs through the
0.71 owner snapshot.

Formation aggregation joins soft discovery and transport observations to committed member identity
by explicit node ID plus generation; unmatched candidates remain discovered-only and never enter
authoritative member totals. Placement traces are accepted only for their committed topology epoch.
Recovery results aggregate by bounded scope and preserve all five outcome counts, including
UNKNOWN. Consensus progress is comparable only within the same authority epoch/node generation;
commit/apply lag is not summed across nodes and missing progress is not zero-filled. The reducer
publishes one immutable result only after validation, or a partial result whose missing sources are
enumerated within bounds.

**Tests.** Pure reducer/property tests randomize order, duplicates, epochs and timeouts and require
deterministic output. Transport tests cover one/three/many peers, cancellation, slowloris,
malformed/oversize replies, peer death, restart with new generation, leader change, split network
and mixed 0.71/0.72 membership. A real three-daemon test proves partial→complete recovery and that
data-plane latency remains within the independently frozen budget while the management endpoint is
hammered. Loom or a deterministic scheduler covers refresh coalescing and cancellation cleanup.

Add join tests for same node/different generation, discovery-only and committed-only peers; stale
placement traces; conflicting recovery outcomes from old/new attempts; missing applied progress;
late clean recovery after a newer degraded observation; saturated outcome counts; and cancellation
between source completion and immutable publication. A serial reference aggregator supplies the
independent oracle. After request cancellation or epoch replacement, all in-flight peer tasks and
response buffers return to baseline and no late observation becomes visible.

**Canary `MC72-W3-CONFUSED-FANOUT`.** Inject an uncommitted loopback/metadata-service address and a
late older-epoch success; neither may be contacted or replace the current snapshot. A bypass must
make the test red.

## W4. Deliver the dashboard and bounded browser-local history

**Problem.** Operators need a coherent first screen, but a dashboard that mixes counters,
snapshots, epochs or unlimited samples is actively misleading.

**Design.** The dashboard contains:

- cluster state, quorum, leader, term/authority epoch and membership freshness;
- replication success/failure/backpressure, under-replicated count, repair debt and degraded mode;
- reshard inflight/backfill lag and lifecycle status;
- entries, retained bytes, hit/miss/load, admission pressure and TTL backlog;
- partition distribution and zone under-spread;
- per-member CPU, RSS/retained bytes, uptime, reachability and generation;
- formation summary by discovered/authenticated/admitted/current/serving state and blocker code;
- placement status with selected/rejected counts and latest committed/applied trace outcome;
- Raft commit/apply lag without converting missing data to zero;
- durable recovery outcome counts, repair debt and bounded corruption/loss evidence;
- visible partial/stale/modeled warnings and a “history since this page opened” label.

Charts consume typed snapshots, never scrape arbitrary Prometheus text. A per-tab ring has frozen
series, point and byte limits, drops the oldest sample, pauses while hidden/offline, aborts obsolete
requests, applies jittered backoff, and resets rather than joining incompatible epochs. Counter
rates handle reset/wrap; gauges are not differentiated as counters.

**Tests.** Table-driven unit tests cover metric kind, counter reset, missing sample, NaN/infinity,
epoch transition, partial member set and ring eviction. Component tests cover empty/small/large
values and every truth state. Playwright verifies chart/table synchronization, responsive layout,
offline pause and no growth beyond the frozen DOM/sample budgets after accelerated multi-hour
polling. Snapshot images are reviewed for layout but assertions use DOM/accessibility semantics.

Dashboard tests also cover every formation blocker, placement rejected/proposed/committed/applied,
commit-with-blocked-apply, all five recovery outcomes, nonzero/saturated corruption counts and
stale/partial/modeled/unavailable variants. A newer epoch resets incompatible history; it does not
draw a continuous line across generations. Summary counts reconcile exactly with the underlying
bounded tables, including UNKNOWN and truncated counts. Clicking a summary opens the matching
filtered read-only view without reconstructing status in the browser.

**Canary `MC72-W4-UNBOUNDED-HISTORY`.** Disable ring eviction and hidden-tab pause; the accelerated
soak must exceed the retained sample/DOM budget and fail.

## W5. Add Members and Partitions views with epoch-consistent topology

**Problem.** The current member card omits the data needed to diagnose version/config skew,
draining, resource pressure and partition imbalance. A flat `reachable=true` field also collapses
the entire formation path and makes a discovered or socket-reachable peer look admitted and safe.

**Design.** The Members table and detail drawer show committed node ID, generation, Raft role,
version/protocol compatibility, reachability reason, drain state, uptime, CPU/RSS/retained bytes,
FD/thread/task counts, client count, partition count and a non-reversible configuration digest.
Partitions show only real ownership and backup facts: total/assigned/unassigned, distribution,
under-replication, zone under-spread, repair debt, reshard progress and backfill lag. Individual
partition diagnostics, if enabled, are paginated and never metric labels.

All rows in one topology view must share an authority epoch or be visibly excluded as stale. A
resource metric unavailable on an operating system is `UNKNOWN`, not zero. Config values are not
returned; only a versioned allowlisted digest suitable for skew comparison is exposed.

Add read-only formation and placement detail to this page:

- a compact formation column shows discovery, authenticated transport, admission, consensus role,
  catch-up and serving independently;
- the detail drawer shows the latest stable blocker code and which source supplied each dimension;
- an epoch-consistent timeline shows observed transitions for the current node generation only;
- replaced generations remain available only as bounded historical rows and are never merged with
  the current generation;
- the partitions view links to a bounded `PlacementDecisionTrace` for current committed placement
  or the latest rejected proposal;
- the trace lists selected candidates first, then rejected candidates in stable node/reason order,
  with topology epoch, constraint summary, outcome and explicit truncation;
- commit/apply positions appear together with an unknown-safe apply-lag presentation;
- no member address, failure-domain label or digest becomes a Prometheus label.

Planned read routes are `GET /management/v1/cluster/members`,
`GET /management/v1/cluster/formation`, `GET /management/v1/cluster/partitions`, and
`GET /management/v1/cluster/placement-traces/{opaque_id}`. The exact route split may be reduced,
but all responses use the common envelope, authorization, pagination, deadline and byte ceilings.
Trace lookup is by an opaque bounded ID scoped to the current authorization context; it is not a
hash that permits enumeration of tenant, namespace or node configuration.

**Planned implementation seams.** Add wire/read-model types in `hydracache-observability`, source
adapters and reducers in `hydracache-server`, and page/components in
`console/src/pages/{members,partitions}` plus `console/src/components/{formation,placement}`. The
server adapter consumes existing discovery, committed membership, Raft progress, placement and
resource snapshots. UI code receives no direct placement algorithm and no peer endpoint capable of
browser fan-out.

**Schema and reducer tests.** Cover every formation enum/reason, missing and unknown future values,
same ID/different generation, duplicate ID, generation regression, cluster identity mismatch,
unauthenticated transport, rejected admission, learner behind/current, voter behind/current,
serving/draining and platform-missing resource fields. Reducers reject older observation sequence,
mixed authority epoch and incoherent `applied_index > commit_index`; they preserve selection only
when node identity and generation still match.

**Placement tests.** Provide one minimal fixture for every reason code and pairwise fixtures for
conflicting constraints. Cover zero/one/many candidates, exact replica count, insufficient quorum,
already-selected peers, zone/host uniqueness, capacity at boundary-1/boundary/boundary+1,
capability/version skew and a topology epoch change between proposal and display. Property tests
permute candidate maps and reasons; result and normalized JSON remain byte-identical. Any tie-break
uses a captured seed and produces the same choice/trace on replay. Pagination cannot hide selected
peers, duplicate candidates or alter outcome; overflow sets `truncated` and bounded summary counts.

**API and security tests.** Verify `management.read`, tenant/scope filtering, opaque trace IDs,
404/403 anti-enumeration policy, maximum limits, malformed/expired cursors, response-byte ceiling,
deadline cancellation and no raw endpoint/config/label/token in body, logs or traces. Feed HTML,
Unicode-bidi, oversized and control-character node IDs/reasons through the allowlist serializer and
browser rendering.

**Real-daemon tests.** Cover 3→4→3 membership, drain observation, leader SIGKILL, reshard,
under-replication and repair. Add discovery-without-admission, bad cluster identity, failed peer
authentication, learner catch-up, leader change between commit/apply observations and restart with
the same ID but a new generation. Every transition has a bounded convergence deadline and confirms
that the dashboard and committed control-plane view agree on identity/epoch.

**Frontend tests.** Cover stable sorting/filtering/pagination, accessible non-color formation
badges, keyboard and screen-reader traversal of the placement explanation, narrow viewport,
virtualized/bounded large tables, trace truncation, route refresh, stale data and unavailable detail.
The browser must never issue POST/PUT/PATCH/DELETE or a request directly to a member endpoint.

**Canary `MC72-W5-MIXED-EPOCH`.** Supply balanced partition counts from two epochs; the view must
reject the fabricated balance and show partial/stale evidence.

**Canary `MC72-W5-DISCOVERED-IS-READY`.** Mutate the formation reducer so `Discovered` implies
`Serving`; schema, health and browser truth tests must turn red.

**Canary `MC72-W5-NONDETERMINISTIC-PLACEMENT`.** Iterate candidates in randomized map order or
remove the recorded tie-break seed; permutation/replay and normalized-JSON tests must fail.

**Canary `MC72-W5-COMMIT-IS-APPLY`.** Copy `commit_index` into `applied_index`; blocked-apply and
mixed-progress fixtures must report the false completion.

## W6. Add bounded HC/1, HC/2 and RESP client visibility

**Problem.** Aggregate HC/2 counters cannot explain connection storms, pending work, subscription
pressure, session retention or protocol distribution; naïve per-client detail leaks identity and
grows with churn.

**Design.** Add bounded lifecycle accounting shared by HC/1, HC/2 and RESP. Summary fields include
active/accepted/closed/rejected connections, pending invocations, active subscriptions/sessions,
buffered bytes, reconnecting/slow/quota-rejected counts and cleanup lag. Optional detail rows use a
process-scoped opaque diagnostic ID and allowed protocol/version/state fields. Never expose remote
address, certificate subject, auth token, tenant secret, cache keys, request payload or session/
lock token. Closed entries leave the live registry immediately; a small reason-count ring may be
kept only within W0 bounds.

Tenant/namespace is shown only to a caller authorized for that scope; otherwise the row is grouped
as redacted. Slow-client classification uses explicit queue/latency bounds and monotonic durations,
not wall-clock ordering.

**Tests.** Protocol matrix tests connect, authenticate, request, subscribe, reconnect, cancel,
timeout, overflow, close and crash clients across HC/1 versions, HC/2 and RESP. Assert counts and
buffered bytes reconcile with existing accounting and return to their declared bound after close.
Property tests feed hostile identity/payload material and scan JSON/log/evidence output. Multi-node
tests distinguish node-local clients from complete/partial cluster aggregation. Churn tests use
geometric connections and a fixed owner-snapshot assertion.

**Canary `MC72-W6-CLOSED-CLIENT-LEAK`.** Retain one closed connection or serialize its remote
address; cleanup/privacy tests must fail and identify the owning collection.

## W7. Add auth-scoped Namespaces and Caches views

**Problem.** Operators cannot currently connect quotas and admission pressure to the namespace or
cache consuming retained state, while exposing internal cache content would violate isolation.

**Design.** Namespace summaries expose permitted name/display identifier, cache count, entries,
retained bytes by approved owner class, hit/miss/load, quota/fair-share/admission state,
near-cache/subscription/repair totals and persistence status. Cache detail adds configured capacity
semantics, TTL/expiry backlog, tag/index overhead, conditional/idempotency/audit retained counts,
backup age and load-breaker state. Values are reconciled to 0.71 retained-state accounting and
declare whether they are exact, sampled, modeled or unavailable.

No endpoint returns keys, values, SQL, tags, loader errors containing inputs, listener payloads, or
raw authorization policy. Namespace filters are server-authorized rather than UI-only. Cross-scope
totals are computed after authorization/redaction and cannot be used to enumerate hidden names.

**Tests.** Multi-tenant fixtures prove same-name/different-tenant isolation, forbidden-scope 404/403
policy, no count oracle, stable pagination and redaction. Behavioral tests exercise insert/read/
miss/load/expire/evict/admission reject/tag/index/persist/reset and reconcile every visible counter
with the source snapshot. Fault tests cover failed loader, expired cleanup backlog and unavailable
persistence. Large cardinality tests prove response and DOM bounds.

**Canary `MC72-W7-CROSS-TENANT`.** Authorize tenant A while placing a distinctive tenant B cache
name/key in source state; it must be absent from body length, totals, logs and evidence.

## W8. Implement the deterministic health-check engine and catalogue

**Problem.** Coloring raw metric thresholds in JavaScript produces inconsistent, untestable health
and encourages missing data to look good.

**Design.** Evaluate health on the server from immutable typed inputs. Each rule returns stable ID,
category, `PASS|WARN|FAIL|UNKNOWN|DISABLED`, short title, bounded evidence references, affected
member/count, remediation link/code, source, observation sequence and evaluation version. Browser
filters/searches/displays results but never computes status.

Initial HydraCache catalogue covers: authority/quorum/leader accessibility; membership reachability,
generation, version and config-digest skew; partition assignment/under-replication/zone spread;
replication failures/backpressure; repair debt/degraded mode; reshard stall; retained-memory and
admission pressure; expiry backlog; client/session/buffer bounds; persistence/backup age and disk
availability; audit sink; and optional history-source availability. Checks requiring unavailable OS
or persistence data return UNKNOWN. Thresholds come from reviewed configuration/defaults and are
reported with units; disabled means explicitly disabled, never missing.

Formation and persistence inputs follow the NATS-derived truth split. A discovered/listening peer
is not admitted or serving; a committed entry is not applied; an attempted repair is not recovered.
Placement health includes a bounded `PlacementDecisionTrace`: requested constraints, candidate set
bound to the committed topology epoch, stable per-candidate rejection codes, selected peers and
proposal/commit/apply outcome. Candidate input order cannot change the result or trace.

The catalogue reserves stable NATS-derived checks; IDs and meanings are compatibility artifacts:

| Check ID | Inputs | PASS condition | WARN/FAIL | UNKNOWN |
| --- | --- | --- | --- | --- |
| `HC-FORM-001` formation completion | six formation dimensions for every committed member | every required member is authenticated, admitted, current and serving | a member is pending/behind/draining beyond its reviewed threshold; rejected identity/auth is FAIL | any required dimension/source missing or mixed epoch |
| `HC-RAFT-001` apply progress | term, epoch, commit/applied index and observation sequence | apply lag within the configured unit-bearing bound | lag crosses warning/failure bound or cannot converge during the window | indexes absent, incoherent or from different observations |
| `HC-PLACE-001` placement satisfiable | committed topology, constraints and decision trace | requested replicas selected and applied | rejected/partial placement, unsafe quorum or failure-domain/capacity deficit | trace/source absent, stale or truncated before outcome is known |
| `HC-RECOVERY-001` durable recovery | latest recovery outcome and completion | `Clean`, or reviewed `Repaired` with post-repair verification complete | `Repaired` with debt is WARN; `Degraded`/`Refused` is FAIL for affected authoritative scope | outcome/status source absent or stale |
| `HC-RECOVERY-002` corruption/lost data | checked/discarded/corrupt counts and scrub sequence | zero unaccounted corrupt/lost authoritative records | any verified corruption/loss or repair debt follows policy severity | scan incomplete, overflowed or source unavailable |
| `HC-RECOVERY-003` reconciliation | recovery phase, topology epoch and desired/local diff | authoritative reconciliation complete with no forbidden local object | pending beyond bound is WARN; failed or resurrection evidence is FAIL | desired/local source is partial or incomparable |

No check reports PASS merely because the service process is live. Threshold configuration includes
units, source and evaluation version in evidence. A status may improve only on a newer coherent
observation; stale success cannot overwrite a newer failure. Aggregate status is the worst known
status under an explicit precedence, but UNKNOWN is retained as a separate count/badge and is never
dropped because another member is healthy.

**Tests.** Every rule has a truth table covering boundary-1/boundary/boundary+1, missing/partial/
stale/modeled data and incompatible schema. Property tests ensure deterministic result independent
of input order and wall clock. Golden fixtures cover representative healthy, degraded, partitioned,
mixed-version, memory-pressure and recovery clusters. Mutation tests invert comparisons/statuses;
no untriaged survivor is allowed in health rules. Playwright confirms filtering and accessible
non-color status labels.

Add formation fixtures for identity mismatch, unauthenticated, discovered-not-admitted,
admitted-not-current and caught-up-not-serving peers. Placement tests cover every rejection code,
combined constraints, topology-epoch invalidation, `limit-1/limit/limit+1` detail bounds and
byte-identical same-seed traces. A mutant that randomizes candidate iteration or maps committed to
applied must be killed.

For the six checks above, add a generated completeness assertion proving each stable ID has one
evaluator, documentation/remediation code, truth table, golden fixture, browser state, canary and
evidence-registry row. Test status precedence over all pairs, stale-success suppression, recovery
improvement on a newer sequence, overflow/truncation, counter saturation, unit conversion and
missing thresholds. Property tests generate coherent and incoherent observation combinations;
evaluators must be total, deterministic and panic-free. The browser must display UNKNOWN counts
even when the headline aggregate is FAIL or PASS for another rule.

**Canary `MC72-W8-UNKNOWN-AS-PASS`.** Remove a required input and mutate UNKNOWN to PASS; both the
rule corpus and UI truth matrix must fail.

**Canary `MC72-W8-STALE-RECOVERY-WINS`.** Allow an older `Clean` recovery observation to replace a
newer `Degraded` one; sequence/epoch reducer and golden timeline tests must fail.

**Canary `MC72-W8-TRUNCATED-PLACEMENT-PASS`.** Mark a truncated trace with an unknown outcome as
placement PASS; the `HC-PLACE-001` truth table and browser fixture must fail.

## W9. Add optional, fixed-query Prometheus history without creating an SSRF proxy

**Problem.** Browser-local history cannot show what happened before page load, but a generic
Prometheus proxy would expose credentials, arbitrary network access and unbounded queries.

**Design.** Keep W4 local history as the default. An opt-in server configuration selects one
allowlisted Prometheus origin, credential source and TLS policy. The browser requests only named
query IDs with capped range/step/series/points/response bytes. The server maps IDs to reviewed
query templates over bounded-cardinality Hydra metrics, validates the resolved destination against
scheme/host/port/address policy, enforces DNS rebinding protection, deadlines and concurrency, and
redacts upstream errors. No user PromQL, URL, headers or credentials cross the browser boundary.

History responses use the same truth envelope and distinguish no adapter, no data, timeout,
partial series and stale samples. Current and historical values declare their source so a chart
cannot silently splice incompatible series.

**Tests.** A fake Prometheus covers valid matrix/vector/empty/error/timeout/malformed/oversize and
counter reset. Security tests attempt loopback, link-local, metadata, private-address rebinding,
redirect, credential echo, arbitrary PromQL and range amplification. Integration tests run the
pinned local Prometheus config against Hydra metrics and compare selected latest samples with typed
snapshot tolerances. Browser tests prove graceful fallback to W4 local history.

**Canary `MC72-W9-ARBITRARY-PROXY`.** Bypass query-ID mapping and request a metadata-service URL or
unbounded range; the SSRF/amplification suite must turn red before any socket is opened.

## W10. Add read-only Persistence, Operations and Audit views

**Problem.** Operators need to understand backup freshness, compaction/reshard/drain progress and
security-relevant activity, but the current POST endpoints are not safe to surface as incidental UI
controls and accepted work is not equivalent to completed durable work.

**Design.** Persistence shows configured/enabled state, last verified durable backup/restore-point
identity, age, size/capacity and explicit unavailable/degraded reasons from real sources. Operations
shows bounded status records for drain, reshard, repair, backup and Raft compaction with requested,
accepted, running, completed, failed and unknown states kept distinct. Audit shows paginated,
redacted management/security event metadata. The browser uses GET only and contains no mutation
client or action button. Existing admin POST routes remain separate and their authentication is not
weakened.

Persistence also exposes a versioned, bounded recovery outcome: `clean`, `repaired`, `degraded`,
`refused`, or `unknown`, with validated watermark/artifact version, bounded lost/corrupt counts,
repair state and stable reason codes. It never serializes raw keys, values or paths. If the existing
startup `RecoveryReport` is not retained, the source is `unavailable` until a separately reviewed
status owner exists; the UI must not infer success from server liveness.

W0 must resolve and document the owner/lifetime of each source:

| Displayed fact | Preferred existing source | Retention rule |
| --- | --- | --- |
| durable value recovery | `RecoveryReport` plus durable-store validation result | immutable last completed attempt per bounded scope and current process generation |
| scrub corruption/debt | `DurableScrubReport` and scrub metrics | latest sequence plus bounded cumulative counters; no raw corrupt key |
| Raft metadata recovery | durable Raft log/snapshot open and apply result | current node generation, snapshot/applied watermark and stable reason |
| repair state | existing repair/replication status | bounded active + terminal history with explicit eviction/truncation |
| backup/restore point | externally verified artifact status | only after verification; accepted/flushed/uploaded/verified stay distinct |

If an existing source cannot satisfy the lifetime, completeness or privacy contract, W0 marks the
field `unavailable` and opens a separately reviewed implementation seam. Adding a persisted status
journal changes the durable format and therefore requires a COMPAT entry, crash matrix and old/new
reader window; it is not smuggled into the UI task.

Recovery presentation has three layers:

1. a cluster summary counts clean/repaired/degraded/refused/unknown scopes without hiding UNKNOWN;
2. a bounded table shows node/scope, phase, outcome, watermark, checked/recovered/discarded/corrupt
   counts, repair state, source and freshness;
3. an allowlisted detail drawer explains stable reason/remediation codes and observation identity.

The UI never offers “repair”, “retry”, “delete corrupt data”, “restore” or “mark healthy”. It may
link to a static runbook that describes the separately authenticated operational workflow.

Operation IDs are opaque, bounded and not credentials. An accepted response never renders as
completed. Restart/recovery either reconstructs a durable status or reports unknown; it does not
invent success. Audit query filters are allowlisted and cannot search raw payload.

**Tests.** State-machine tests cover accept→run→complete/fail, duplicate IDs, cancellation, restart,
missing journal and out-of-order observation. Persistence fixtures distinguish accepted backup,
flushed snapshot and externally verified durable artifact, plus clean/repaired/degraded/refused/
unknown recovery, corrupt record, future format, missing status and successful/failed repair. API
goldens cover previous/current/unknown recovery DTO versions; redaction and bounds tests cover lost
detail. Static bundle tests reject POST/PUT/
PATCH/DELETE and known mutation route strings; browser network interception asserts GET/HEAD only.
Audit tests inject keys/tokens/errors and assert redaction, pagination and retention bounds.

**Recovery contract tests.** Verify every legal phase/outcome transition and reject illegal ones:
`Discover→Validate→Rebuild?→Reconcile→Complete`, terminal `Refused`, repair
`NotRequired|Pending→Running→Complete|Failed`, and process restart with retained versus unretained
status. `Clean` is impossible with corrupt/discarded authoritative data, incomplete reconciliation,
failed repair or unknown required counts. `Repaired` requires a detected defect and completed
revalidation. Saturating counts never wrap and overflow forces an explicit partial marker.

**Corruption fixtures.** Cover missing artifact, current/previous/future format, bad checksum,
truncated length/payload, foreign node/cluster identity, stale watermark, one corrupt durable value,
multiple corrupt values beyond detail limit, disk full, permission denied, recovery timeout, repair
source absent, repair failure and reconciliation failure. Each fixture declares `recover`, `repair`,
`degrade`, or `refuse`; no test accepts any other outcome.

**Differential and property tests.** Compare management recovery counts/watermark with the source
`RecoveryReport`/scrub/raft state; generate input order and pagination permutations; assert the same
normalized response. Generate legal state-machine traces and reject illegal transitions without
partial publication. Fuzz the new management DTO decoders and cursor/trace identifiers; this does
not claim a new durable-store decoder fuzz target unless that separate work is implemented.

**Frontend tests.** Render every outcome/phase/repair combination, stale/partial/modeled/
unavailable source, zero and saturated counts, truncated details and long localized remediation.
Assert non-color labels, accessible table/drawer relationships, keyboard operation, narrow viewport,
bounded DOM and no raw error/path/key in text, attributes, URLs, console or screenshots.

**Canary `MC72-W10-WRITE-FROM-UI`.** Add a hidden drain/backup POST or map `accepted` to `completed`;
the static/network/state-machine sentinels must fail.

**Canary `MC72-W10-LIVENESS-IS-CLEAN`.** Remove the recovery source and derive `Clean` from a live
server process; source-contract, health and browser UNKNOWN tests must fail.

**Canary `MC72-W10-LOSS-HIDDEN`.** Drop or zero a nonzero corrupt/discarded count during
aggregation; differential source reconciliation and recovery summary tests must fail.

**Canary `MC72-W10-ILLEGAL-REPAIR-COMPLETE`.** Publish `Repaired/Complete` before post-repair
verification; state-machine and real-process recovery tests must fail.

## W11. Harden authorization, web security, accessibility and dependency supply chain

**Problem.** A richer diagnostic surface increases the impact of XSS, unauthorized enumeration,
dependency compromise and inaccessible status presentation.

**Design.** Introduce a read-only `management.read` capability distinct from write administration
and tenant-scoped capabilities. Keep the default listener on loopback/internal administration
scope; document TLS/reverse-proxy requirements for remote use. Serve a restrictive CSP, no-sniff,
frame-ancestor and referrer policy, same-origin assets and typed JSON content. Escape all values by
construction; no raw HTML rendering. Rate-limit/cancel management reads independently so they do
not starve data-plane/admin recovery paths.

Meet keyboard navigation, visible focus, semantic landmarks/tables, accessible names, reduced
motion and non-color status requirements. Desktop, tablet and narrow-mobile layouts retain the
same truth information. Pin npm dependencies in the lockfile, review licenses/provenance, emit an
SBOM and run vulnerability policy without runtime CDN dependencies.

**Tests.** Authorization matrix tests cover unauthenticated, management-reader, tenant-reader,
cross-tenant reader and write-admin combinations for every route. Stored/reflected XSS fixtures
cover node/cache/error/audit strings and CSP blocks inline/external execution. Run axe-core via
Playwright on every page/state, keyboard-only workflows, zoom/narrow viewport and forced-colors.
Run dependency audit/license/SBOM and secret scanning. Load tests prove rate-limit fairness and
cancellation cleanup.

**Canary `MC72-W11-XSS-AUTH-BYPASS`.** Render a script-bearing node ID and remove one route's
`management.read` guard; CSP/DOM/auth matrix tests must fail.

## W12. Prove behavior with real processes, faults, churn, soak and resource budgets

**Problem.** Mocked pages cannot prove aggregation, cleanup, partial truth, rolling compatibility or
non-interference with a live cluster.

**Design.** Extend `DaemonCluster` and Playwright fixtures to run the production server binary and
embedded production UI. Define tiers before implementation results are visible:

| Tier | Trigger | Scenarios | Admission role |
| --- | --- | --- | --- |
| Fast | every PR | unit/property/component, one/three daemon happy+partial, auth/privacy, bounds, canaries | required |
| Browser integration | every PR or required split job | all pages/states, network failure, accessibility, production bundle | required |
| Scheduled | nightly/weekly | 3/5/7 daemons, client churn, partitions, leader/member restart, reshard/repair, mixed versions, Prometheus faults | required before candidate |
| Candidate | frozen SHA on admitted host | six-hour fixed-cardinality management polling plus representative HC/1/HC/2/RESP traffic | release blocking |
| Ship confirmation | exact candidate on same fingerprint | 24-hour fixed-cardinality/churn/recovery run and upgrade/rollback rehearsal | release blocking |

The scenario matrix includes: no leader/election, minority/majority partition, asymmetric peer
latency, member SIGKILL/restart/new generation, stale and duplicate snapshots, 3→5→3 membership,
under-replication/repair, reshard, disk slow/full for persistence status, Prometheus absent/slow/
malformed, 10x client connection churn, namespace churn, hidden/offline browser, and concurrent
management readers. Every fault has a recovery phase and bounded convergence assertion.

The NATS-derived recovery subset additionally covers corrupt authoritative snapshot, uncommitted
WAL truncation, interrupted artifact replacement, commit-with-blocked-apply, and this sequence:
partition a peer → delete authoritative namespace/placement/member state → compact → restart the
stale peer from the same disk → catch up → prove the UI and control plane never report resurrected
state. Reuse existing HydraCache corruption/failpoint/DaemonCluster tests through evidence links;
do not duplicate a scenario solely for dashboard code.

The source-to-management fault matrix is mandatory:

| Fault class | Injection/setup | Required source result | Required API/UI result | Tier |
| --- | --- | --- | --- | --- |
| clean reopen | stop after durable flush, reopen same disk | same validated watermark/state | `Clean`, fresh source, matching watermark | fast + process |
| missing rebuildable derivative | remove only a declared non-authoritative index/status cache | deterministic rebuild or documented absence | `Repaired`/`Unknown` with reason, never data loss | fast + process |
| bit rot | flip header, payload and checksum regions independently | corrupt record/snapshot detected; never served | affected scope `Degraded` or `Refused`, nonzero bounded corruption evidence | fast + fuzz corpus |
| torn artifact | truncate at header, length, payload, checksum and generation boundaries | verified prior generation/prefix or fail loud under its artifact contract | exact outcome/reason, no optimistic cached status | fast + failpoint |
| ENOSPC/write error | fail before write, mid-write, flush, sync, rename/install | previous valid generation remains; no partial publication | operation failed, recovery truth preserved, disk reason redacted | scheduled + process |
| uncommitted WAL | isolate proposer before quorum then crash/reopen | entry absent/truncated and never applied | no completed operation or applied progress for the entry | fast + process |
| corrupt authoritative snapshot | corrupt newest snapshot with/without valid predecessor | use only contractually valid predecessor or refuse startup/catch-up | `Refused`/recovering, never healthy or serving | fast + process |
| commit blocked before apply | commit command and hold apply seam | commit advances, apply does not | visible lag; operation remains committed/running, not completed | fast + process |
| deletion during stale-peer partition | partition, delete/commit, compact, restart stale disk, catch up | authoritative removal wins; no local resurrection | removed object never reappears in complete view | scheduled + ship |
| foreign identity/disk | open data under another node/cluster generation | fail loud before publish | stable foreign-identity reason, no leaked path/config | fast + process |
| interrupted reconciliation | cancel/crash before local desired-state plan publishes | old complete observation or new complete observation, never mixed | partial/recovering until atomic publication | fast + process |
| concurrent snapshot/aggregation | mutate/commit while snapshot and management collection run | coherent snapshot plus permitted tail semantics | one epoch/sequence or explicit partial result | scheduled |
| bounded resource pressure | max peers/scopes/reasons plus slow readers | queues/tasks/bytes stay within frozen bounds and cancel cleanly | truncation/backpressure explicit; data plane remains inside budget | scheduled + candidate |

Each row names the existing HydraCache test that proves the source invariant and the new management
test that proves faithful projection. When the source proof already exists in 0.64/0.66, the 0.72
test consumes a fixture/receipt or composes with it; it does not fork a weaker duplicate. When no
source seam exists, the row is `missing` and blocks the corresponding UI claim.

Fault execution rules:

- failpoints are named at semantic boundaries and compiled out/disabled in production artifacts;
- every injected fault has a released/recovery phase and a bounded convergence assertion;
- deterministic tests use a recorded seed and logical scheduler; real-process tests record the
  external schedule and exact binary hashes;
- fault activation is positively asserted, so a green test cannot come from an injection that did
  not fire;
- destructive fixture operations target only validated per-test temporary directories;
- retries create a new linked attempt and retain the original failure; they never overwrite it;
- an environmental skip is structured evidence and blocks a required tier rather than passing it;
- process logs and screenshots are redacted before retention, while raw restricted artifacts retain
  hashes and access controls required for investigation.

Freeze budgets from W0/0.71 for server RSS/PSS/retained owners, browser heap/DOM/samples, FD/thread/
task counts, endpoint p95/p99 and bytes, aggregation concurrency, data-plane throughput/tail latency
and error count. The candidate may meet or improve them; it cannot redefine them. Shared hosted
runners provide deterministic structural tripwires only, not numerical memory/latency claims.

**Tests and evidence.** Store seeds, schedules, process logs, browser traces, network captures with
payload redaction, owner snapshots, resource series, endpoint histograms and recovery verdicts.
Same seed/config/binary must normalize to the same event digest. A failed/retried attempt remains in
the ledger. Zero product errors and zero silent skips are required; environmental unavailability is
explicit and blocks the corresponding tier.

**Canary `MC72-W12-FAULT-MASKED`.** Disable one peer, then mutate the aggregator/UI to report
complete/healthy or leak one refresh task per cycle; truth/resource gates must fail during the run.

**Canary `MC72-W12-FAULT-NOT-ACTIVATED`.** Disable the fault hook while leaving the scenario green;
the activation receipt assertion must fail.

**Canary `MC72-W12-RESURRECTION`.** During stale-peer catch-up, publish a locally discovered object
absent from authoritative desired state; source invariant, management snapshot and browser timeline
must all turn red.

**Canary `MC72-W12-RETRY-ERASES-RED`.** Replace a failed attempt with the later successful retry;
ledger-chain and release-evidence verification must reject the bundle.

## W13. Lock compatibility, packaging, upgrade/rollback and publication rehearsal

**Problem.** A green source tree does not prove that the published server contains the tested UI,
that a rolling cluster understands the management schema, or that the package can be consumed.

**Design.** Register all new API/wire/durable artifacts in `docs/COMPAT.md`. Preserve the legacy
overview window. Define 0.71↔0.72 mixed-cluster behavior: 0.71 peers are represented as capability-
limited/partial, never queried with an unsupported snapshot RPC, while 0.72 readers accept only
registered old/current shapes. Build server, console, schemas and SBOM once from the exact candidate
and bind their hashes to the release receipt.

Run clean install/package tests for npm assets and every publishable Rust crate; verify the embedded
manifest in the packaged server. Exercise fresh 0.71→0.72 rolling upgrade, leader changes during
upgrade, 0.72→0.71 rollback before/after browser use, old bookmarks, disabled management API and
fresh downstream consumer compilation. Rehearse the existing crates.io publication automation
without publishing, including topological order, idempotent resume and partial-failure recovery.

**Tests.** Golden/semver checks compare 0.71/current DTOs and public Rust API. Mixed real binaries
exercise capabilities, partial aggregation and static assets. Archive/install tests launch the
packaged binary in an empty directory and inspect served asset digests/CSP/API. Publication dry-run
uses exact artifacts from the candidate evidence tree and rejects rebuilt substitutes.

Register separate compatibility identifiers and goldens for the formation snapshot, placement
trace, durable recovery status, health check catalogue/evaluation version and consensus-progress
projection. Because 0.71 does not expose these shapes, a mixed cluster reports the old peer as
capability-limited/partial with stable `source-unavailable` evidence. It never fabricates zero
apply lag, clean recovery or serving formation. Unknown future reason/status values survive as
unknown through the 0.72 server and browser; they cannot be deserialized into the first enum value.

Mixed-binary scenarios cover an old leader/new followers, new leader/old follower, leadership
change during aggregation, old peer restart during a placement trace and rollback after a 0.72 UI
has observed new DTOs. No management DTO is written into an old durable store unless its format and
reader window were explicitly registered.

**Canary `MC72-W13-MIXED-ARTIFACT`.** Replace the embedded UI or one crate with a different-SHA
build; package, upgrade and publication receipts must reject it.

## W14. Make coverage, canaries and exact-candidate evidence release-blocking

**Problem.** “Tests passed” is not a stable-work proof without test-to-claim coverage, proof that
guards can fail, exact inputs, retained failures and a single fail-closed admission decision.

**Design.** Extend xtask with `management-center-check`, release-scoped canary sweep and evidence
verification. Every new route, DTO field/enum, health rule, UI page/state, permission, bound,
background task and error branch has a row mapping it to at least one test and one evidence file.
The existing workspace line-coverage floor remains at least 88%; it cannot be lowered. In addition,
changed management modules require reviewed diff coverage with no unexplained new executable branch.
Generated code and unreachable platform branches are excluded only by an explicit reviewed row,
not a blanket directory exemption. Coverage is supporting evidence, never a substitute for
behavioral/fault/privacy tests.

W14 also carries a NATS-derived failure-taxonomy coverage table: reopen, missing derivative,
bit rot, torn artifact, ENOSPC/write error, uncommitted WAL, corrupt authoritative snapshot,
catch-up after deletion, concurrent snapshot and resource bounds. Each row is mechanically marked
`covered`, `partial` or `missing` with exact HydraCache test, canary and receipt. Only partial/missing
rows create new work; existing 0.64/0.66 proof is linked rather than rerun under a new name.

The machine-readable registry has one stable row per claim/semantic unit, not one row per broad
work item. Its minimum shape is:

```toml
[[claim]]
id = "MC72-FORMATION-DISCOVERY-NOT-AUTHORITY"
work_item = "W5"
claim = "a discovered peer is not admitted or serving"
source = "committed-membership-plus-discovery-observation"
source_tests = ["..."]
implementation = ["crate/module/symbol"]
behavior_tests = ["..."]
canaries = ["MC72-W5-DISCOVERED-IS-READY"]
tiers = ["fast", "process"]
compat_artifacts = ["management-v1-formation-v1"]
bounds = ["members", "reasons", "response-bytes"]
privacy_rules = ["opaque-node-id", "no-peer-address"]
receipts = ["..."]
status = "planned" # planned | implemented | evidenced | deferred
```

`management-center-check` rejects duplicate/missing IDs, unknown work items, empty test selection,
nonexistent symbols/tests/canaries, evidence without implementation, implementation without a test,
schema fields/reason codes absent from the registry, and `deferred` rows still claimed by UI/docs.
It cross-checks the W12 taxonomy and the frontend route/state inventory. Any registry exemption has
an owner, rationale, expiry and non-ship marker.

The canary meta-gate runs every `MC72-W*` test first against a minimal reversible fault/mutant and
records the expected red result, then runs the real candidate green. No canary that stays green is
credited. `docs/GATES.md`, CI jobs and `cargo xtask verify` are updated only when their commands are
implemented; this plan alone does not claim an unwired gate.

**Planned commands.** Exact names may change only with the manifest/source-map in the same commit:

```bash
npm --prefix console ci
npm --prefix console run typecheck
npm --prefix console run lint
npm --prefix console run test:unit -- --run
npm --prefix console run test:coverage
npm --prefix console run build
npm --prefix console run test:e2e
cargo test -p hydracache-observability --test management_schema_072 --locked
cargo test -p hydracache-server --test management_api_072 --locked
cargo test -p hydracache-server --test management_aggregation_072 --locked
cargo test -p hydracache-server --test management_formation_072 --locked
cargo test -p hydracache-server --test management_health_072 --locked
cargo test -p hydracache-server --test management_recovery_072 --locked
cargo test -p hydracache-server --test management_placement_072 --locked
cargo test -p hydracache-server --test management_progress_072 --locked
cargo test -p hydracache-server --test management_model_072 --locked
cargo test -p hydracache-server --test management_compat_072 --locked
cargo test -p hydracache-server --test management_security_072 --locked
cargo test -p hydracache-server --test management_process_072 --locked
cargo test -p hydracache-fuzz --test fuzz_corpus_regression --locked
cargo test -p xtask --test management_center_072 --locked
cargo xtask management-center-check --release 0.72
cargo xtask canary-sweep --release 0.72 --tier fast
cargo xtask verify
```

Candidate/ship workflows additionally run the W12 scheduled/soak matrix, compatibility/package
checks, full canary sweep and:

```bash
cargo +nightly fuzz run fuzz_management_envelope -- -max_total_time=60
cargo +nightly fuzz run fuzz_management_recovery -- -max_total_time=60
cargo +nightly fuzz run fuzz_management_placement -- -max_total_time=60
cargo +nightly fuzz run fuzz_management_cursor -- -max_total_time=60
cargo xtask release-evidence --release 0.72 --require-ship
```

**Evidence contract.** Each receipt records candidate commit/tree, dirty-state rejection, toolchain
and lockfile digests, OS/runner fingerprint, binary/UI/schema/SBOM hashes, exact command/environment,
start/end monotonic times, seed, exit code, raw-artifact hashes, normalized verdict and reviewer.
The release ledger retains failed, timed-out and superseded attempts. It rejects missing files,
mixed SHAs, unknown schemas, expired evidence, unexecuted canaries, quiet skips and a tag not
resolving to the admitted commit.

**Canary `MC72-W14-PAPER-GREEN`.** Delete one required receipt or provide a green candidate run
without its corresponding red canary; `--require-ship` must fail with the exact missing proof.

## Test principles for all 0.72 functionality

These principles are part of the release contract. They apply to all Management Center work,
including NATS-derived formation, placement, recovery and progress views. A work item is not done
when its happy path renders; it is done only when every new behavior, bound, failure branch and
cleanup owner has executable evidence.

### P1. Test the source truth before the projection

The UI cannot strengthen a weak or absent source. For every displayed fact, the source map records:

```text
operator claim
-> authoritative or observational source
-> source invariant/test
-> aggregation/reducer code
-> API schema field
-> frontend state/component
-> behavioral test
-> falsifiability canary
-> exact-candidate receipt
```

If the authoritative source invariant is missing, the field is `Unavailable/Unknown` or removed
from scope. A mocked green page is not evidence for cluster formation, durable recovery, placement,
commit/apply progress or no-resurrection.

### P2. Cover every new semantic unit, not merely every line

The coverage registry enumerates every new:

- route and method/status/content-type branch;
- DTO field, enum variant, reason code and schema version;
- source adapter and absent/stale/partial/error result;
- reducer transition and out-of-order observation path;
- health rule, threshold boundary and aggregate precedence;
- pagination, truncation, saturation, byte/deadline/concurrency bound;
- authorization scope and redaction rule;
- background task, channel, timer, cancellation and shutdown owner;
- UI route, page, component, loading/empty/error/stale/partial/unknown state;
- compatibility reader/writer and unsupported-version path;
- fault hook and recovery path.

Every registry row has at least one named behavioral test. Incidental execution by another test does
not count. Unreachable platform/generated branches require an explicit reviewed exclusion with a
reason and owner; directory-wide or `/* ignore */` exclusions are forbidden.

### P3. Determinism and replay are mandatory

Unit, property, reducer, health, aggregation and simulated-fault tests do not depend on ambient wall
clock, hash-map order, OS port allocation or unrecorded randomness. They use injected clocks,
stable ordering, reserved listeners where required and a recorded seed. A failure receipt contains
the original seed and the smallest shrunk schedule/case when the framework supports shrinking.

The same code, input, seed, topology epoch and observation stream must produce the same normalized
API body, health verdict, placement trace and event digest. Real-process scheduling cannot be fully
deterministic; its externally driven schedule, monotonic timings and exact binaries are retained so
the run remains explainable and approximately replayable.

### P4. Use independent oracles and differential checks

Do not test a reducer by recomputing expectations with the same reducer. Important paths compare
against a simpler independent model:

- aggregation against a serial reference fold over immutable member snapshots;
- formation state against a table-driven predicate model;
- placement selection/trace against a stable reference evaluator;
- health rules against explicit truth tables;
- recovery status/counts against source `RecoveryReport`, scrub and Raft fixtures;
- local-history rings against an unbounded test vector truncated by the documented policy;
- API serialization against reviewed golden JSON and previous-version fixtures;
- browser-visible values against captured typed API responses, not screen text generated from the
  same component helper.

### P5. Boundary testing is systematic

Every numeric/cardinality/time/byte bound is tested at `limit-1`, `limit`, and `limit+1`, plus zero,
empty, maximum representable/saturated and malformed values where relevant. This applies to page
size, candidates, reasons, warnings, members, namespaces, clients, samples, response bytes,
request bytes, cursor length, string length, concurrency, timeout, apply lag, corruption counts and
retained history. Overflow saturates or rejects explicitly; it never wraps, allocates from an
untrusted length or silently drops evidence without `truncated/partial`.

### P6. Missing, stale, partial and incompatible data are first-class test states

Every page and health rule has fixtures for:

- live/fresh/complete;
- live/fresh/partial;
- live/stale/complete and partial;
- modeled;
- unavailable with each stable reason class;
- older observation sequence;
- different authority epoch or node generation;
- previous/current/unknown future schema;
- unknown enum/reason code;
- timeout, cancellation and malformed payload.

No default branch may map these states to zero, empty-success, PASS, serving, applied, durable or
completed. Unknown future enum values remain visibly unknown through server, JSON parser, reducer,
component and accessibility text.

### P7. Failure injection must prove activation and recovery

Each failpoint test asserts the hook fired at the intended semantic boundary, captures the state
before/during/after, releases the fault and proves bounded recovery or explicit terminal failure.
Crash tests reopen the exact same validated test directory. A fault that did not activate, a test
that only observes the failure phase, or a recovery wait without a deadline is invalid evidence.

Storage/consensus fault tests prefer existing 0.64/0.66 source proofs and compose the management
projection on top. New management-only failpoints target aggregation, sequence/epoch publication,
status retention and cleanup; they do not create alternate consensus behavior.

### P8. Concurrency, cancellation and cleanup are observable

Tests cover cancellation before dispatch, during peer fan-out, after partial response, while a
browser request is dropped, during history update and during server shutdown/restart. After bounded
quiescence, task/channel/timer/listener/HTTP body/trace ownership returns to the frozen baseline.
Late peer results cannot publish into a newer observation. Slow management readers cannot hold
Raft/storage locks, exhaust data-plane permits or prevent drain/recovery.

Use loom/Miri/TSAN only for applicable ownership/interleaving classes; a specialized tool does not
replace a behavioral test. Resource cleanup has direct owner counters and a canary that leaks one
owner, proving the test detects it.

### P9. Security and privacy are behavioral properties

Authorization is tested as a route × role × tenant/scope matrix, including unauthenticated,
management-reader, tenant-reader, write-admin and expired/revoked identities. Test bodies, headers,
logs, traces, metrics, audit records, screenshots, browser storage, URLs and error pages for leaked
keys, values, SQL, tokens, certificates, raw peer addresses, filesystem paths and cross-tenant
names/counts.

Fuzz/malformed input tests run behind request-size and allocation bounds. XSS payloads cover HTML,
attributes, URLs, Unicode bidi/control characters and oversized strings. SSRF tests prove rejection
before socket creation. Read-only enforcement is static (no mutation client/routes in bundle) and
dynamic (browser interception observes only allowed GET/HEAD requests).

### P10. Compatibility is tested with real old and new artifacts

Schema goldens are necessary but insufficient. The mixed-version lane starts actual 0.71 and 0.72
binaries/artifacts, checks capability negotiation, partial projections and unknown-field handling,
and performs forward upgrade and permitted rollback. New readers accept only registered old/current
formats; old readers are never sent a shape they did not advertise. Recovery and placement reasons
unknown to an old peer remain unknown, not success.

Published archive/package tests launch the packaged server and consume its embedded UI/API. Source
tree assets or a rebuilt binary cannot substitute for the receipt-bound candidate artifacts.

### P11. Mutation tests and canaries prove test power

Each work item has at least one narrow reversible canary that breaks its load-bearing invariant.
For formation/recovery/placement specifically, the canary set includes:

| Canary class | Required mutation/fault | Tests that must fail |
| --- | --- | --- |
| discovery inflation | `Discovered -> Serving` | formation truth, health, browser |
| authentication bypass | reachable peer marked authenticated/admitted | auth/formation/process |
| epoch mixing | combine candidates/partitions from different epochs | reducer/property/browser |
| nondeterministic placement | random/hash iteration changes selected peer | permutation/replay/golden |
| commit/apply collapse | copy commit into applied | blocked-apply/operation/health |
| UNKNOWN inflation | missing source maps to PASS/Clean | health/recovery/browser |
| hidden loss | corrupt/discarded count dropped or zeroed | differential/API/summary |
| premature repair | `Repaired/Complete` before revalidation | state-machine/process |
| resurrection | publish stale local object absent from desired state | source/process/timeline |
| fault bypass | injection never fires | activation receipt/meta-gate |
| bound removal | omit page/byte/task cap | boundary/load/resource |
| redaction removal | serialize one forbidden field | privacy/log/browser/evidence scan |

The canary sweep retains an expected red result, restores the exact candidate, then records green.
A canary that stays green, fails for an unrelated compilation error or is not restored blocks the
work item. Mutation survivors are either killed or explicitly triaged with a new test and reviewed
reason; “equivalent” is evidence, not a default label.

### P12. Coverage is a floor, not the acceptance oracle

Workspace line coverage remains at least 88%, and changed management modules have reviewed diff
coverage with no unexplained executable branch. Add branch/function coverage where tooling is
stable. Coverage cannot prove ordering, freshness, authority, privacy, boundedness or recovery, so
all such claims require direct behavioral/fault tests. A high percentage cannot compensate for a
missing registry row, surviving canary, skipped required tier or wrong-SHA artifact.

### P13. Test layers and admission roles are explicit

| Layer | Primary purpose | Required for |
| --- | --- | --- |
| unit/table | enums, predicates, transitions, formatting-free logic | every PR |
| contract/golden | schemas, versions, reason codes, source adapters | every PR |
| property/model/differential | order independence, invariants, oracle comparison | every PR |
| fuzz | parsers/DTO/cursor robustness and bounded allocation | bounded PR smoke + scheduled corpus |
| component | all UI states, accessibility semantics, cleanup | every PR |
| API/security | auth, scope, redaction, bounds, cancellation | every PR |
| browser production bundle | navigation, rendering, network policy, CSP/a11y | required split job |
| in-process integration | fast multi-source aggregation and fault composition | every PR where bounded |
| real-process | OS lifecycle, sockets, disk, SIGKILL, mixed binaries | fast representative + scheduled matrix |
| soak/resource | retention, churn, noninterference, bounded convergence | candidate/ship tiers |
| governance | coverage map, canaries, receipts and artifact identity | every PR for structure; release blocking for evidence |

Mocks/fakes are allowed to force rare states and test pure presentation. They never replace the
real-process source proof for claims involving network identity, authentication, Raft quorum,
durable recovery, disk faults, process death, upgrade or resource isolation.

### P14. Tests are isolated, finite and fail closed

Each test owns an explicit temporary directory, ports/listeners, clock, seed, credentials and
cleanup guard. Parallel tests cannot share cluster identity, storage or global mutable environment.
All waits use a named condition, monotonic deadline and final diagnostic dump; fixed sleeps are not
proof of convergence. Timeouts, panics, setup failures, unavailable dependencies, unexecuted test
filters and empty test selections fail required tiers.

Test sharding is generated from the registry and proves each required category appears exactly once.
`--list`/discovery output and executed counts are retained so a build-tag/filter mistake cannot
silently produce green. Quarantine requires owner, reason, expiry, linked issue and non-ship status;
no release-blocking 0.72 guarantee may be quarantined at ship time.

### P15. Evidence is immutable, exact-candidate and reviewable

Each receipt binds the candidate commit/tree, lockfiles, toolchains, OS/host fingerprint, config,
seed, test selection, start/end monotonic times, binary/UI/schema/SBOM hashes, exit code, canary
identity, raw artifact hashes and normalized verdict. Failed, timed-out, skipped and superseded
attempts remain linked in an append-only attempt chain. Review can reproduce the command and locate
the minimal logs without needing sensitive payloads.

The ship verifier fails on dirty candidates, missing or expired receipts, unknown schema, mixed
SHA, rebuilt substitutes, missing red canary, silent skip, empty shard, lowered budget/coverage,
unexplained retry or release tag not resolving to the admitted commit.

## NATS-derived feature-to-proof matrix

This matrix is the minimum complete proof for the newly added NATS-derived scope. W14 materializes
it in the machine-readable coverage registry.

| Feature/claim | Unit/contract | Property/fuzz | Integration/process | Browser | Canary |
| --- | --- | --- | --- | --- | --- |
| formation dimensions remain distinct | all enum/reason/transition tables | coherent/incoherent observation generation; DTO fuzz | discovery/auth/admission/catch-up/serving sequence on daemons | badges, timeline, stale/unknown | discovered-is-ready; auth bypass |
| epoch-consistent member/partition view | reducer sequence/epoch tables | permutation and regression properties | leader/member restart, 3→4→3, reshard | mixed-epoch visible partial state | mixed-epoch fabrication |
| deterministic placement explanation | every reason and constraint boundary | candidate permutation, same-seed byte identity, trace decoder fuzz | under-replication/repair/topology change | accessible selected/rejected trace and truncation | randomized placement |
| commit and apply are not completion aliases | progress validation/truth tables | generated monotone/coherent index pairs | hold apply after commit, leader restart | visible lag and non-completed operation | commit-is-apply |
| recovery status is honest | phase/outcome/reason and legal transition tables | source differential, DTO fuzz, counter bounds | clean/corrupt/future-format/disk/restart/repair fixtures | all five outcomes plus partial/stale | liveness-is-clean; loss-hidden |
| deleted authority never resurrects | desired/local reconciliation model | snapshot+tail order model | partition→delete→compact→restart→catch-up | object absent; timeline reports recovery only | resurrection |
| failure taxonomy is fully selected | registry row/outcome parser | manifest completeness property | every W12 matrix row or linked prior proof | relevant error/degraded fixtures | fault-not-activated |
| evidence cannot become paper-green | receipt/attempt-chain validation | manifest mutation corpus | exact artifact and mixed-SHA rehearsal | packaged UI digest | retry-erases-red; paper-green |

## Test ownership and coverage map

The implementation may reorganize file names, but it must preserve these independently executable
proof categories and update the source map atomically:

| Proof category | Planned primary tests | What it establishes |
| --- | --- | --- |
| Schema/compatibility | `hydracache-observability/tests/management_schema_072.rs` | version/enums/goldens/bounds/redaction/old-new contract |
| HTTP/auth | `hydracache-server/tests/management_api_072.rs`, `management_security_072.rs` | route semantics, authorization, content security, cancellation |
| Aggregation | `management_aggregation_072.rs` | epoch/sequence reduction, partial truth, fan-out bounds, cleanup |
| Formation | `management_formation_072.rs` | discovery/auth/admission/role/catch-up/serving distinctions, generation and blocker codes |
| Placement | `management_placement_072.rs` | every reason/constraint, epoch binding, deterministic trace, commit/apply outcome and bounds |
| Recovery | `management_recovery_072.rs` | phases/outcomes/reasons, source reconciliation, corruption/loss accounting and illegal transition rejection |
| Consensus progress | `management_progress_072.rs` | coherent term/epoch/commit/apply/snapshot positions and blocked-apply truth |
| Client accounting | `management_clients_072.rs` | HC/1/HC/2/RESP lifecycle, privacy, byte/count reconciliation |
| Namespace/cache | `management_namespaces_072.rs` | tenant isolation, quota/admission/retained-state reconciliation |
| Health | `management_health_072.rs` | deterministic rule truth tables, UNKNOWN/stale behavior, mutants |
| Model/property/differential | `management_model_072.rs`, proptest modules | order independence, reference reducers, legal traces, source-to-DTO equality |
| Decoder fuzz | `fuzz/fuzz_targets/fuzz_management_{envelope,recovery,placement,cursor}.rs` | no panic/hang/partial publish or allocation-before-length-bound |
| Prometheus | `management_prometheus_072.rs` | fixed query mapping, SSRF/range/response bounds, fallback |
| Process/fault | `management_process_072.rs`, scheduled `management_soak_072` | live topology/fault/recovery/non-interference/resource bounds |
| Frontend unit | `console/src/**/*.test.ts[x]` | parsers, reducers, rings, components and every truth/error state |
| Browser | `console/tests/{shell,dashboard,members,formation,placement,clients,namespaces,health,recovery,history,security,accessibility,boundedness}.spec.ts` | production bundle, workflows, every truth state, network policy, responsive/a11y/DOM bounds |
| Mutation/canary | release-scoped canary targets plus mutation report | each load-bearing guard is falsifiable and the responsible suite turns red |
| Compatibility/package | `management_compat_072.rs`, package/archive and mixed-binary jobs | reader windows, capability limits, embedded artifact identity, upgrade/rollback |
| Governance | `xtask/tests/management_center_072.rs` | source/claim/test/canary/evidence completeness, nonempty shards and exact-SHA admission |

“All new code is tested” means every new behavior and failure branch appears in this executable
mapping, not merely that a line executed incidentally. Review blocks a work item if its diff adds a
route, enum variant, async owner, collection, permission, UI state or error without a named test.

## Release gate and required proof bundle

0.72 may ship only when all of the following are true:

1. W0 source map, threat model and independently frozen budgets are reviewed and complete.
2. W1-W11 focused suites are green on the exact candidate; every declared error/truth state and
   bound has a behavioral test.
3. Every `MC72-W0` through `MC72-W14` canary has a retained red run and candidate green run.
4. The workspace test suite, clippy/fmt/docs, existing coverage floor, console production build,
   Playwright matrix, security/accessibility checks and package checks are green.
5. Real 3/5/7-daemon fault scenarios recover with honest partial/stale transitions and no
   data-plane correctness or SLO regression.
6. Six-hour candidate and 24-hour ship-confirmation runs satisfy the frozen server/browser owner,
   FD/task, endpoint, latency and zero-error budgets on the admitted fingerprint.
7. 0.71↔0.72 rolling upgrade and rollback are green; mixed peers remain capability-limited rather
   than misrepresented.
8. The packaged server serves the same hashed UI/schema/SBOM artifacts tested by the evidence run;
   no external runtime assets are present.
9. Security review finds no unauthorized tenant/member detail, key/token/address leakage, XSS,
   arbitrary proxy/fan-out or browser mutation route.
10. `cargo xtask release-evidence --release 0.72 --require-ship` is green for the annotated tag's
    exact commit, with no missing/expired/quietly skipped row.
11. Formation evidence proves discovery, transport authentication, admission, learner/voter,
    catch-up and serving remain distinct under restart, partition, identity mismatch and mixed
    version; no listening peer is promoted by observation alone.
12. Placement evidence covers every stable rejection code and constraint boundary, is invariant to
    candidate input order, replays byte-identically from seed/topology, and invalidates stale-epoch
    traces.
13. Recovery evidence covers all five outcomes, every declared phase/reason, lost/corrupt counter
    bounds, source-to-DTO differential checks and process restart; liveness never substitutes for a
    retained source.
14. Commit-with-blocked-apply shows visible lag and never renders completed; all operation,
    placement and health projections agree on the coherent applied position.
15. Partition→delete→compact→restart/catch-up proves no removed member/namespace/placement is
    resurrected in source state, management snapshots, local history or the browser.
16. Every W12 NATS-derived fault category is `covered` by a source proof plus projection proof or
    explicitly blocks the related claim; fault-activation and retry-chain meta-canaries are green.

A polished screenshot, unit coverage percentage, hosted-only soak, manually clicked demo, or
successful `/healthz` response cannot replace any missing row. If optional Prometheus history is
not fully evidenced, its capability remains disabled; the rest of the release may ship with
bounded local history. If a core read-only page or truth contract is not evidenced, 0.72 does not
ship until the scope is explicitly changed in this plan, `releases.toml`, `INDEX.md`, source map and
claim/evidence registry in one reviewed commit.

## Deferred follow-ons

- Write-capable operations with separate RBAC, audit, confirmation, idempotency, CSRF/replay
  protection and two-person policy where appropriate.
- Persisted first-party historical store, alert routing, notification integrations and shared
  dashboard customization.
- Cross-cluster/fleet view, hosted SaaS management plane and arbitrary user-defined queries.
- Autoscaling, capacity recommendation and numerical sizing claims beyond admitted performance
  evidence.
- Hazelcast data structures, wire compatibility, scripting or other features that HydraCache does
  not implement.
- NATS/JetStream subjects, streams, consumers, delivery policies or a general messaging-server
  identity.
- A custom block/segment value store, changed `DurableValueStore` format, checkpoint-plus-tail
  snapshot optimization, shared atomic file-replacement primitive, new placement algorithm,
  deterministic maintenance jitter and storage/Raft decoder fuzzing. These remain post-0.72
  durable-store/cluster hardening and require their own ADR, compatibility registration,
  measurement and crash/fuzz evidence.
