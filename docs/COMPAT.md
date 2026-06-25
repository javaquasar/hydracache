# HydraCache Compatibility Register

This file tracks durable and wire-visible artifacts whose versions matter during
rolling upgrades. Runtime-only Rust types are intentionally out of scope unless
they are persisted or transmitted across processes.

## Versioned Artifacts

| Artifact | Current Version | Writer | Reader Compatibility | Failure Mode |
| --- | --- | --- | --- | --- |
| `CacheInvalidationFrame` | `1` | `hydracache` invalidation bus publishers | Readers accept version `1` only. Unknown versions are rejected before apply. | Decode error is reported and the receiver continues. |
| `hydracache_invalidation_outbox` schema | `1` | `hydracache-db` outbox writers or application SQL writers | Workers accept schema version `1`. Unknown future versions must fail loud before draining. | Worker refuses to start; intent is left durable and pending. |
| `hydracache_hook_schema` schema | `1` | `hydracache-db` generated hook installers | Reconciliation expects version `1` for installed hook plans. Missing or mismatched rows report drift. | Staging/release gates can fail before silently trusting disabled or stale hooks. |
| `RaftLogStore` in-memory format | `1` | `hydracache-cluster-raft` metadata runtime | 0.41 tests cover append/replay, snapshot recovery, suffix truncation, and compaction guard semantics. Future durable engines must register their own format before rollout. | Runtime fails loud on store errors; unknown future durable formats must refuse startup. |
| HTTP replication/peer encoded-value transport | `1` | `hydracache-cluster-transport-axum` clients | Strict routes require `x-hydracache-wire-version: 1`; mismatches are rejected before payload apply. | Route returns upgrade-required style safe rejection; counters can record wire-version failures. |
| `DurableRaftLogStore` format | `1` | `hydracache-cluster-raft` durable-log feature | Readers accept format `1` and refuse unknown future versions before opening a store. | Store open fails loud; no committed command is acknowledged from an unknown format. |
| `ReplicatedValueRecord` durable format | `1` | `hydracache` durable-values feature | Readers accept format `1`; records carry partition, version, epoch, and value/tombstone state. | Unknown future formats must refuse startup before serving replicated values. |
| `ChecksummedReplicatedValueRecord` durable envelope | `1` | `hydracache` scrubber/checksum helpers | Readers accept envelope format `1`; the envelope stores a deterministic checksum over `ReplicatedValueRecord` payload fields. Scrubbers verify before serving and may repair from valid peer copies. | Checksum mismatch is reported; unrepairable corruption is not served. Unknown future envelope formats fail closed. |
| `DurableValueStore` on-disk value-store format | `1` | `hydracache` `durable-value-store` feature | Readers accept store format `1` only. Records are length-prefixed binary envelopes containing the cache key, `ReplicatedValueRecord`, tombstone/value state, and checksum metadata. The value engine is separate from the raft log engine. | Store open refuses unknown future formats. Corrupt or mismatched records fail loud and are not served. |
| `DurabilitySnapshotManifest` format | `1` | `hydracache` durability write path | Readers accept manifest format `1` only. The manifest records namespace, snapshot scheduler time, interval, and the covered `(partition, version, epoch)` watermark with a deterministic checksum. | Unknown future manifest formats or checksum mismatches fail loud before recovery trusts the snapshot watermark. |
| CRDT value encoding | `1` | `hydracache` active-active CRDT helpers | The 0.45 serde shape for `GCounter`, `PnCounter`, `OrSet`, `LwwRegister`, and OR-set tags is the registered CRDT value encoding version `1`. Durable or replicated adapters must keep this shape stable or introduce a new explicit version before emitting a changed encoding. | Unknown future CRDT value encodings must fail closed before merge/apply so stale readers do not converge on different metadata. |
| WAN `RegionLink` replication batch frame | `1` | `hydracache` active-active region-link helpers | The 0.45 serde shape for `GeoBatch`, `GeoWrite`, `IdempotencyKey`, and `GeoBatchApplyReport` is the registered WAN batch frame version `1`. Readers must preserve idempotency-key pairing and HLC/epoch/version fields. | Unknown future WAN frames must be rejected before apply; mismatched entry/idempotency vectors are already refused. |
| Cross-region anti-entropy digest exchange | `1` | `hydracache` region-link anti-entropy helpers | The 0.45 serde shape for `PartitionDigest`, `VersionSummary`, and CRDT metadata GC confirmation is the registered digest exchange version `1`. Readers compare `(version, epoch)` summaries only within the matching partition. | Unknown future digest versions must fail loud before diffing so repair cannot silently skip or over-apply keys. |
| Replayable invalidation stream snapshot | `1` | `hydracache` invalidation-ring durable adapters | The 0.46 `InvalidationRingSnapshot` shape is the registered retained-window format version `1`: partition, capacity, head sequence, next sequence, and retained invalidation events. Durable adapters must keep sequence semantics stable. | Unknown future stream snapshots must refuse restore before serving; subscribers outside retention fall back to clear-partition semantics. |
| Hinted-handoff record format | `1` | `hydracache` hinted-handoff stores | The 0.46 serde shape for `Hint`, `HintBudget`, `HintOutcome`, `HintReplayDecision`, and `InMemoryHintStore` is the registered handoff record format version `1`. Readers must retain target, key, partition, version, epoch, sealed bytes, and creation time. | Unknown future hint records must be rejected before replay; over-budget or expired hints fall back to Merkle repair rather than silent loss. |
| Merkle repair exchange format | `1` | `hydracache` Merkle repair helpers | The 0.46 serde shape for `MerkleTree`, `KeyRange`, `RepairToken`, `RepairSession`, and `RepairReport` is the registered repair exchange version `1`. Readers interpret ranges as inclusive key ranges and watermarks as incremental repaired-key cursors. | Unknown future repair exchanges must fail loud before applying foreground or scheduled repair. |
| Session token wire format | `1` | `hydracache` causal/session helpers and client context carriers | The 0.47 serde shape for `SessionToken`, `SessionId`, `SessionRequest`, `SessionLifecycleDecision`, and failover recovery reports is the registered session-token wire format version `1`. Tokens carry a bounded watermark, nonce, issue time, and MAC. | Unknown future token formats, forged MACs, wrong-session tokens, replays, and expired tokens fail loud or downgrade through the documented sessionless rebuild path. |
| Session watermark format | `1` | `hydracache` causal/session helpers | The 0.47 serde shape for `SessionWatermark`, `PartitionKey`, and `VersionStamp` is the registered watermark format version `1`. Readers preserve bounded coarsening semantics and compare `(version, epoch, HLC)` stamps. | Unknown future watermark formats must fail closed before using them to satisfy read-your-writes, monotonic-read, or causal-read guarantees. |
| Single-key conditional/fenced-lock engine state | `2` | `hydracache::SingleKeyConditionalStore` and future applied lock-command paths | `0.52.0` extends fenced-lock state from token-only holds to session-bound `LockHold { owner, fence, holds, lease_deadline }` with logical-time lease expiry and new `ConditionalError` variants `LeaseExpired`, `NotOwner`, and `ReentrancyLimit`. Fence tokens are assigned only by the applied deterministic state machine. | Readers or services that cannot understand version `2` lock state or the new error variants must fail loud before serving fenced-lock/CAS operations; stale owners, expired leases, non-owners, and weak consistency levels are rejected and counted. |
| `ControlPlaneSnapshot` format | `1` | `hydracache` self-heal snapshot helpers | Readers accept format `1` and refuse unknown future versions before restore. | Restore fails loud before rebuilding topology from an unsupported snapshot. |
| `BackupManifest` format | `1` | `hydracache` object-store backup helpers | Readers accept manifest format `1`, verify object length/checksum, and refuse unknown future manifest versions before restore/PITR replay. | Restore fails loud; corrupt or unknown-format backups are not served. |
| External client HTTP route boundary | `1` | `hydracache-client-transport-axum` | Public clients use `/client/v1/*`; internal member routes remain under `/cluster/*` and are not part of the public client compatibility surface. Unknown future client route versions are refused instead of falling through to member handlers. | Unauthenticated, oversized, malformed, or wrong-route requests are rejected before protocol dispatch or state mutation. |
| HydraCache external client protocol | `2` | `hydracache-client-protocol` and `hydracache-client-transport-axum` | Protocols `1` and `2` use custom length-prefixed binary frames over HTTP/2; the request payload is a typed HydraCache message carrying `protocol_version`, request id, context, deadline, idempotency key, stable operations, stable errors, and B1 watermark fields. Protocol `2` keeps the same framing and adds the 0.52 IMap/Fenced Lock operation family: `TryLock`, `Unlock`, `RenewLockLease`, `ForceUnlock`, `GetLockOwnership`, `CompareAndSet`, `RemoveIfValue`, `SubscribeEntryEvents` and responses `LockAcquired`, `LockBusy`, `LockReleased`, `LockLeaseRenewed`, `LockOwnership`, `CasApplied`, `CasMismatch`. Entry-listener projections are cache signals, not business event logs. v2-only operations must be gated on negotiated version `2` or newer. See ADR `docs/adr/0007-client-wire-framing.md`. | Out-of-window versions, malformed/truncated frames, oversized frames, v2-only operations on v1 envelopes, and unknown future protocol versions are refused loud before mutation. Region-scoped subscriptions narrow delivery, not correctness. |
| Hibernate L2 provider contract | `1` | `hydracache-client-protocol::hibernate` and external `hydracache-hibernate` providers | Hibernate ORM 6.x providers map regions to HydraCache namespaces, access strategies to stable L2 consistency labels, and query cache regions to timestamp/bulk invalidation. See `docs/integrations/hibernate.md` and ADR `docs/adr/0006-why-not-clone-hibernate-hikaricp.md`. | Unsupported Hibernate versions, unsupported query-cache mode, or unknown future mapping versions must fail loud at provider bootstrap. HydraCache never joins JVM transactions. |
| Client SDK conformance manifest | `1` | `hydracache-client` and `sdks/python` | `crates/hydracache-client/tests/fixtures/conformance/client_v1.json` is the language-agnostic scenario set for protocol-v1 SDKs. Rust and Python SDKs are supported only when their runners pass this manifest. | SDKs that do not pass the manifest are not claimed as supported. Manifest major changes require a new compatibility entry and SDK semver mapping. |
| Java migration toolkit contract | `2` | `hydracache-client-protocol::java_migration` and external JVM artifacts | Java/Spring/Hibernate migration artifacts use the protocol-v1/v2 client contract, stable Java exception mapping, safe codec/schema registration, Spring Cache `native`/`jcache`/`none` modes, Java lock operation mapping, Java IMap conditional write mapping (`replace`, `replace-if-present`, `remove-if-value`), Java IMap entry-listener projection mapping, and the checked-in Hazelcast API manifest at `crates/hydracache-client-protocol/manifests/unsupported_hazelcast_apis.txt`. See `docs/integrations/java-migration.md`. | Unknown future contract/manifest versions, ambiguous codec ids, reflective or Java-native serializers, member-mode app JVM topology, unsupported Hazelcast APIs, non-admin `ForceUnlock`, unaudited admin force-unlock attempts, and attempts to treat entry listeners as `Ringbuffer`/`ReliableTopic` business logs fail loud with migration hints. |
| `ResidencyPolicy` control-plane format | `1` | `hydracache` residency governance | Policies are committed at a control-plane epoch per namespace with optional per-key overrides. Readers accept format `1`, enforce allowed regions at placement, WAN value movement, read serving, and include-value invalidation decisions, and report the enforced epoch. | Unknown future policy formats are rejected before commit. Unsatisfiable in-policy RF, forbidden boundary crossing, stale policy epochs, and forbidden-region reads fail loud and emit audit-ready events. |
| Tenant status JSON schema | `1` | `hydracache-observability` and `/client/v1/status` | `TenantStatus` is scoped to the verified caller tenant and includes schema version, namespace usage/quota, rate/fair-share state, and near-cache/subscription health. | Unknown future status schema versions must be treated as incompatible by strict clients. Servers must not include other tenants in a caller-scoped status response. |
| Consumer audit event schema | `1` | `hydracache-observability` audit recorders/sinks | Audit envelopes carry schema version `1` plus redacted governance/security/admin events. Keys are omitted or hashed; values are never logged. | Mandatory governance/security event sink failures fail closed. Future schema versions require an explicit compatibility entry before operator log readers accept them. |
| Simulator snapshot JSON schema | `4` | `hydracache-sim` and `hydracache-sim-wasm` | Readers accept the current schema only and reject unknown future versions. Version `1` carried seed, step, logical time, nodes, links, sampled keys, real invariant verdict, and progress. Version `2` adds the 0.53 W1 election/formation fields: `formation_phase`, `election_source`, `election_disclosure`, and per-node `vote_state`, `voted_for`, `votes_received`. Version `3` adds typed `in_flight` network/election messages plus `over_budget.in_flight_summarized` for bounded rendering. Version `4` adds manual-mode `clients`, `subscribers`, and `sync_progress`. | Strict readers reject unknown future schema versions before rendering or replaying a shared seed. The demo must not synthesize a green verdict outside `InvariantChecker`; `election_source = "sim-model"` must be presented as a teaching model, not a production consensus claim. In-flight render summaries and subscriber buffer drops must fail loud through over-budget/drop counters instead of silently dropping evidence. |
| `ReplayScriptV1` simulator control artifact | `1` | `hydracache-sim`, `hydracache-sim-wasm`, sandbox `/sim/control`, and browser share/replay surfaces | Version `1` carries `seed`, `mode`, optional `scenario`, and ordered `ControlActionV1` actions (`step`, topology verbs, `push_event`, `subscribe`, `mode_change`) with logical `at_step`. Readers accept version `1` only and reject oversized scripts above `MAX_REPLAY_ACTIONS`. | Unknown future replay versions and over-budget action lists are rejected before execution. Topology actions not implemented by the current work item fail loud rather than being ignored or emulated. |

## Upgrade Rules

- Writers may not emit a newer durable or wire artifact until readers in the
  deployment explicitly support it.
- Unknown future schema versions fail closed. A worker must not silently drain a
  table it does not understand.
- Unknown wire versions are treated as decode errors, not panics.
- Forward-only migrations must be idempotent: applying the same migration twice
  leaves the artifact at the same version.

## 0.37 Baseline

`0.37.0` starts this register with the existing invalidation frame and the new
database invalidation outbox schema. Later cluster releases should append raft
log format, replicated value record format, and public client protocols here
before claiming rolling-upgrade compatibility.

## 0.38 Correctness Automation

`0.38.0` adds hook-schema compatibility tracking and reconciliation drift
reports. These reports are assisted-mode guardrails: they make missing hook
schema rows, mismatched hook versions, outbox backlog, and dead-lettered rows
visible to CI/staging gates. They do not make HydraCache a transparent DB proxy
and do not remove the need to install hooks/outbox migrations in the database.

## 0.41 Grid Slice

`0.41.0` registers the first distributed-grid durable and wire-visible seams:
`RaftLogStore` format version `1` for the metadata log seam and HTTP wire
version `1` for encoded replicated/peer value transport. The release ships an
in-memory store and feature-gated example path only; production durable engine
selection remains future hardening work and must add its concrete on-disk format
to this register.

## 0.42 Grid Hardening

`0.42.0` registers the supported durable raft-log format version `1` and the
replicated value-record format version `1`. The durable raft seam refuses unknown
future format versions before opening a store. Replicated value records persist
sealed bytes plus `(partition, version, epoch)` and tombstone state so restart and
anti-entropy can converge without resurrecting deleted keys.

## 0.43 Geo-Distribution And Elasticity

`0.43.0` registers control-plane snapshot format version `1` for operational
self-healing backup/restore. Upgrade checks keep the 0.42 -> 0.43 rolling window
bounded to raft-log format `1`, replicated value-record format `1`, and
invalidation wire frame version `1`; incompatible jumps fail loud before a mixed
cluster step is accepted.

## 0.44 Deterministic Simulation And Scrubbing

`0.44.0` adds checksummed replicated-value envelopes and a scrubber gate for
durable value records. The underlying `ReplicatedValueRecord` payload format
remains `1`; the new `ChecksummedReplicatedValueRecord` envelope format `1`
detects corruption before serving and can repair a corrupt primary copy from a
valid peer copy. Unknown future envelope formats fail closed.

## 0.45 Active-Active Multiregion

`0.45.0` registers the first active-active cross-region data shapes as
compatibility version `1`. CRDT values (`GCounter`, `PnCounter`, `OrSet`,
`LwwRegister`) converge only if all durable and replicated adapters preserve the
same metadata shape. WAN batches (`GeoBatch` / `GeoWrite`) carry HLC,
origin-region, epoch/version, sealed value bytes, and one idempotency key per
entry. Anti-entropy digests (`PartitionDigest` / `VersionSummary`) compare
per-key `(version, epoch)` summaries for one partition.

The 0.45 code exposes these as serde/runtime shapes rather than standalone
codec wrappers. That shape is now registered as format `1`; any future external
codec, durable adapter, or wire transport that changes it must introduce an
explicit versioned envelope and reject unknown future versions before
merge/apply/diff.

## 0.46 Cluster Resilience And Coordination

`0.46.0` registers the resilience artifacts that can survive a transient outage
or cross-process repair path. The replayable invalidation stream snapshot
retains partition, capacity, sequence window, and invalidation events. Hinted
handoff records retain target, key, partition, version, epoch, sealed bytes, and
creation time so replay can suppress stale writes and tombstone resurrection.
Merkle repair exchanges carry inclusive key ranges, tree summaries, repair mode,
and incremental repaired-key watermarks.

The 0.46 shapes are registered as version `1`. Durable adapters must not restore
unknown future invalidation-ring snapshots or hint records silently. Repair
peers must reject unknown future exchange shapes before using them to decide
which keys to exchange.

## 0.47 Cross-Region Session Consistency

`0.47.0` registers the client-visible session consistency formats as version
`1`. Session tokens carry a stable session id, bounded watermark, nonce, logical
issue time, and tamper-evident MAC. Session watermarks carry bounded
partition/region stamps using `(version, epoch, HLC)` ordering and explicit
coarsening accounting.

These formats are part of the causal/read-your-writes contract. Unknown future
token or watermark formats must fail closed before the reader treats them as
evidence for session guarantees. Forged, replayed, wrong-session, or expired
tokens already take the loud error or documented sessionless-rebuild path.

## 0.48 Production Deployment And Security

`0.48.0` registers `BackupManifest` format version `1` for off-host full
backups and PITR restore. Restore validates manifest version, object length, and
checksums before rebuilding a dataset, and refuses unknown future manifest
formats before replaying PITR records.

## 0.49 Ecosystem And External Consumers

`0.49.0` reserves the external client HTTP route boundary at `/client/v1/*`.
The boundary is intentionally separate from internal `/cluster/*` member routes:
public clients cannot hit member handlers by path confusion, and anonymous or
oversized client requests are rejected before protocol dispatch or cache state
mutation. The stable client wire protocol itself is registered as its own artifact
when W1 publishes protocol version `1`.

W1 registers the HydraCache external client protocol version `1`. Framing is a
custom `u32 body_len | u16 protocol_version | postcard payload` binary envelope
over the existing HTTP/2 route boundary, not gRPC. Readers reject unknown future
protocol versions loud. Region-scoped `SubscribeInvalidations` is a dissemination
filter: it may narrow delivery, but it never hides correctness-relevant
cross-region invalidations, and `include_value` is residency-gated.

W2 registers Hibernate L2 provider contract version `1`. The provider contract is
not a Hibernate clone: an external Java `hydracache-hibernate` artifact implements
Hibernate ORM 6.x SPI and uses the W1 protocol. Regions map to namespaces,
`read-only` maps to `strong-immutable`, `nonstrict-read-write` maps to
`best-effort-invalidate`, and `read-write` / `transactional` map to
`invalidate-on-commit`. Query cache support is timestamp/bulk invalidation or a
loud bootstrap refusal.

W3 registers the client SDK conformance manifest version `1`. The Rust reference
SDK (`hydracache-client`) and the Python SDK (`sdks/python`) use the same
language-agnostic scenarios for protocol negotiation, get/put/invalidate,
near-cache repair, deadlines/retry/idempotency, and stable W4/W5 error contracts.
An SDK is not considered supported unless its runner passes the manifest.

W4 binds public client identities to a bounded tenant roster before namespace
ownership, quota accounting, or tenant-labelled metrics. Over-quota and over-rate
conditions use the existing protocol-v1 stable errors `TenantQuota` and
`RateLimited` with retry-after hints; unknown tenants are refused before dispatch.

W5 registers residency policy format version `1` for authoritative
namespace/key governance. Residency is distinct from performance placement: it
filters legal regions first, then asks placement to satisfy RF inside that set.
WAN links refuse to ship pinned value bytes over forbidden boundaries, reads from
forbidden regions fail closed even if a stale local copy exists, and
policy-narrowing reports explicit eviction/degraded remediation actions.

W6 registers tenant status JSON schema `1` and consumer audit event schema `1`.
`GET /client/v1/status` is read-only and scoped to the verified tenant identity.
Audit events are append-only, redacted, and mandatory for governance/security
refusals: if the configured mandatory sink is unavailable, the guarded operation
fails closed rather than proceeding without an audit trail.

W7 registers Java migration toolkit contract version `1`. The contract is
cache-focused: typed Java clients, Spring Boot 2/3/4 starters, Spring Cache
`native`/`jcache`/`none`, Hibernate L2 delegation, listeners, Micrometer, and
Actuator probes are allowed to build on protocol-v1 behavior. The toolkit must
not imply Hazelcast wire or API compatibility. Unsupported Hazelcast APIs are
listed in `unsupported_hazelcast_apis.txt` and must fail loud with migration
hints instead of becoming silent no-ops.

## 0.50 Interactive Simulator Demo

`0.50.0` registers simulator snapshot JSON schema version `1`. The schema is a
communication surface for the interactive demo and sandbox simulator routes, not
a correctness gate. It serializes the real deterministic `SimWorld` state and
the real `InvariantChecker` verdict. Unknown future schema versions are rejected
loudly by strict readers before rendering, seed sharing, or route consumers treat
the payload as compatible.

## 0.51 Configurable Persistence

`0.51.0` registers `DurableValueStore` on-disk value-store format version `1`
for the feature-gated value-plane persistence backend. The store persists sealed
replicated value records and tombstones in length-prefixed binary envelopes and
verifies checksums before serving. It is intentionally a separate engine from
the raft log store. Unknown future store formats refuse startup; corrupt or
mismatched records fail loud and are not served.

W4 registers `DurabilitySnapshotManifest` format version `1`. Snapshot manifests
record the namespace and covered write watermark `(partition, version, epoch)` so
recovery can fail loud instead of guessing what a scheduled snapshot contained.
Unknown future manifest formats or checksum mismatches are rejected before the
manifest is trusted.

## 0.52 IMap And Fenced Lock Java Surface

`0.52.0` bumps the HydraCache external client protocol to version `2` while
keeping the protocol `1` reader window open. The frame shape stays the ADR-0007
custom length-prefixed binary envelope over HTTP/2; v2 is an operation-family
extension, not a transport swap. Ordinary protocol-v1 cache requests continue to
receive protocol-v1 response frames/envelopes. Any v2-only IMap/Fenced Lock/CAS
operation sent on a v1 envelope is rejected with `IncompatibleVersion` before
dispatch so older clients never observe a v2-only response shape.

W3/W5 add the v2 fenced-lock and IMap CAS request/response family: `TryLock { ns, key,
lease_ms, wait_ms, level }`, `Unlock { ns, key, fence }`, `RenewLockLease { ns,
key, fence, lease_ms }`, `ForceUnlock { ns, key }`, `GetLockOwnership { ns, key }`,
`CompareAndSet { ns, key, expected, new_value, level }`, and
`RemoveIfValue { ns, key, expected, level }`, and
`SubscribeEntryEvents { ns, region, from, include_value, projection }`; responses include
`LockAcquired { fence }`, `LockBusy`, `LockReleased`, `LockLeaseRenewed`,
`LockOwnership { fence, locked }`, `CasApplied { new_version }`, and
`CasMismatch { current }`. `CasExpectation::Present` is the replace-if-present
form and must mismatch absent/tombstoned keys rather than inserting. Weak
consistency levels are rejected as a stable conflict envelope; non-leader
lock/CAS endpoints return a retryable backend unavailable envelope with a leader
hint instead of applying locally.

W6 registers `EntryEventProjection`, `EntryEventSource`, `EntryEventKind`,
`EntryEvent`, and `EntryListenerContract`. The only supported listener contract
is a bounded, coalesced cache signal with lag/drop counters; it must not be
interpreted as a durable business event log. Sources that do not prove
`Removed` or `Evicted` project to `Invalidated`, and ordinary writes project to
`Upserted` rather than fabricating separate `Added`/`Updated` transitions.

The single-key conditional/fenced-lock engine state remains registered as
version `2`: session-bound lock ownership, logical leases, reentrancy count, and
the conditional tombstone used by remove-if-value are applied through the
deterministic state machine. Readers or services that do not understand those
states or error variants must fail loud before serving the 0.52 lock/CAS surface.

## 0.53 Interactive Cluster Lab

W1 bumps the simulator snapshot JSON schema to version `2`. The new fields make
the previously implicit cluster-formation/election model visible to browser,
WASM, and sandbox consumers: top-level `formation_phase`, `election_source`, and
`election_disclosure`, plus per-node `vote_state`, `voted_for`, and
`votes_received`. The current W1 election path is explicitly `sim-model`; it is a
deterministic teaching model over the simulator FSM and must not be described as
the production consensus implementation. Unknown future snapshot versions
continue to fail closed before rendering.

W2 bumps the simulator snapshot JSON schema to version `3`. The new `in_flight`
array carries typed network and election messages (`heartbeat`, replication,
ack, vote request/response, and leader heartbeat), with deterministic ids,
source/destination nodes, optional keys, sequence/term, and logical delivery
timing. The array is capped by `MAX_IN_FLIGHT_RENDERED`; omitted records are
reported through `over_budget.in_flight_summarized` so UI and replay consumers
can detect summarized evidence instead of mistaking it for an empty network.

W3 bumps the simulator snapshot JSON schema to version `4` and registers
`ReplayScriptV1` format `1`. The new manual-mode fields expose client actors,
namespace subscribers, bus-carried subscriber events, lag/drop counters, and
per-node sync progress. `ControlActionV1` is the shared control surface for
native tests, WASM, and sandbox routes; actions outside the current implemented
work item fail loud until their owning work item lands.
