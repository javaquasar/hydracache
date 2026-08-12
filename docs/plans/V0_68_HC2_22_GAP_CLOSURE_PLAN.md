# HydraCache 0.68 HC/2 — 22-Gap Closure And Redis Listener Execution Plan

> **Purpose.** This companion plan turns the 22 known gaps between the current
> non-production HC/2 spike and a defensible client plane into independently
> auditable work packages. It is subordinate to
> [`V0_68_GENERATED_CLIENT_PLANE_FOUNDATION_PLAN.md`](V0_68_GENERATED_CLIENT_PLANE_FOUNDATION_PLAN.md),
> inherits R-1 through R-11, and does not change the `0.68` dependency on an
> unfinished `0.67.1`.
>
> **Commit rule.** Each H-item receives its own implementation commit and push
> only after its Definition of Done is green. Documentation-only progress,
> mocks, or a sans-I/O proof cannot mark a production row complete.
>
> **Release reconciliation.** Completion of this H-ledger does not replace the
> original W0-W12 deliverables. In particular, H15 is generation evidence rather
> than a production Python SDK, H16 does not include the Hazelcast-shaped facade,
> H22 is not the 0.68 release-evidence manifest, and H23 is an additive Redis
> compatibility deliverable rather than a twenty-third HC/2 gap. See
> [`V0_68_RELEASE_CLOSURE_LEDGER.md`](V0_68_RELEASE_CLOSURE_LEDGER.md).

## Status ledger

| ID | Gap | Status | Completion commit | Primary evidence |
| --- | --- | --- | --- | --- |
| H01 | Spike is not production-integrated | complete | `feat(client-plane): complete production HC2 listener integration` | accepted gRPC adapter, off-by-default config, eager mTLS/readiness, shared HC/1 dispatch, push, bounded ownership/drain, internal privacy-safe metrics mount, port-conflict and real-daemon gates green |
| H02 | HC/2 schema is only a minimal envelope | complete | `feat(client-plane): establish authoritative HC2 schema registry` | generated descriptor, stable IDs, breaking canaries, Rust/Java golden and unknown-field proofs |
| H03 | Transport ADR remains Proposed | complete | `test(client-plane): accept gRPC transport bakeoff` | all three real mTLS candidate gates, hostile corpus/gap, two-pass Rust/Java generation, machine-checked cost/evidence manifest, ADR-0019 accepted |
| H04 | Authenticated discovery is represented by a boolean | complete | `feat(client-plane): authenticate discovery by type` | opaque proof token, cluster binding, compile-fail tests |
| H05 | Authenticated-unsupported fallback is caller-asserted | complete | `feat(client-plane): bind fallback to verified attempts` | opaque attempt IDs, peer/endpoint/generation binding, stale/forgery matrix |
| H06 | Endpoint authority is an unparsed string | complete | `feat(client-plane): parse canonical endpoint identities` | strict URI corpus + adapter/SNI/origin policy |
| H07 | Discovery rollback is not prevented | complete | `feat(client-plane): reject discovery rollback and equivocation` | stateful replay/cluster/node-epoch matrix |
| H08 | Advertisement cannot represent multi-node endpoints | complete | `feat(client-plane): model bounded multi-node discovery` | three-node/duplicate/conflict/bound tests |
| H09 | Lifecycle checks are runtime-only, not typestate | complete | `feat(client-plane): make bootstrap sequencing typestate-safe` | compile-fail bootstrap + runtime race/cleanup tests |
| H10 | Connection generation/stale-message fencing is absent | complete | `feat(client-plane): fence stale connection generations` | nonzero checked epoch, wire/runtime fencing, reconnect/reused-ID matrix |
| H11 | Reconnect and subscription/session repair are absent | complete | `feat(client-plane): add bounded reconnect and repair` | separate bounded reconnect/retry policies, generation replacement, endpoint preference, conservative subscription repair/dedupe, permanent session loss, and eight retained deterministic lifecycle traces |
| H12 | Resource bounds omit byte/retry/session/global budgets | complete | `feat(client-plane): bound all retained resource classes` | ownership ledger, byte/count boundaries, atomic admission, zero-close proof |
| H13 | Negative TLS matrix is incomplete | complete | `test(client-plane): complete TLS authorization matrix` | real rustls/tonic hostname/time/EKU/CA/version matrix, bounded chain policy, authorization typestate, rotation and zero-dispatch proofs |
| H14 | Discovery signing/replay protection is absent | complete | `feat(client-plane): sign replay-safe discovery` | canonical Ed25519 bytes, bounded parser/trust/time policy, Rust/Java vector, replay/rotation/recovery matrix |
| H15 | Hermetic Python generation is absent | complete | `feat(client-plane): make Python generation hermetic` | vendored protoc, descriptor-derived stubs/metadata, hash-bound two-platform wheelhouse, clean generation, golden/unknown-field/bidi/import proof |
| H16 | Java is a codec fixture, not a production SDK | complete | `feat(client-plane): add Java reconnect and production interop` | publishable Java 17 SDK/external consumer, fail-closed PKI matrix, bounded reconnect/replay and repair, two-process Rust recovery, real production-daemon mTLS/drain, thread/resource checks, and retained H18 JAR/POM baseline green |
| H17 | Rust production SDK still uses HC/1 HTTP | complete | `feat(client-plane): complete Rust HC2 production interop` | native SDK/process/package/HC/1 and H11 recovery gates plus real production-daemon mTLS/data/push/session/drain proof green |
| H18 | Real previous-artifact compatibility matrix is absent | complete | `test(client-plane): prove generation 5-to-6 compatibility` | nine-row fail-closed matrix uses retained Rust/Java clients plus retained Linux/Windows generation-5 and generation-6 daemons; both directions, rolling replacement, negotiation, unknown fields, deprecation, coexistence, and clean drain are green |
| H19 | Deterministic network fault proxy is absent | complete | `test(client-plane): complete deterministic fault bindings` | all bounded actions, real async stream, exact replay, eight H11 and three H20 lifecycle traces, plus four plans applied unchanged to every H03 candidate codec |
| H20 | HTTP/2 graceful shutdown remains unresolved | complete | `fix(client-plane): bound HTTP2 graceful drain` | two-GOAWAY controller, deadline/reset reasons, retained fault traces, real TLS/H2 zero-accounting task joins without abort |
| H21 | Observability is local to the spike | complete | `feat(client-plane): export bounded privacy-safe diagnostics` | v1 typed metric/trace export, 64 salted tenant buckets, bounded trace ring, privacy/cardinality/reconnect/close evidence, operator runbook |
| H22 | Linux CI, interop, fuzz, and soak gates are absent | in progress | `ci(client-plane): install Linux interop fuzz and soak gates` | four-lane fail-closed contract, pinned workflow/container, receipts/admission canaries, Docker proof, runbook, and exact Ubuntu 24.04 fixed-host preflight implemented; exact hosted Linux/Docker/fuzz receipts and strict `main` protection are active; a labelled fixed-host receipt remains operational evidence |
| H23 | Redis API clients cannot consume mutation events | complete | `feat(redis): add bounded keyspace event listeners` | shared metadata-only mutation bus, RESP2 arrays/RESP3 pushes, exact/pattern subscriptions, auth and atomic count/retained-byte bounds, explicit lag disconnect, redis-rs and cross-protocol tests; exact implementation commit `2fef344` passed PR Rust, MSRV, Docs, docs-site, HC/2 Linux/Interop, Java 17/21, and both performance tripwires in runs `31551909019`, `31551909021`, and `31551909027` |
| H24 | Ordinary native listener tests are internal-only | complete | `test(events): prove native listener public contract` | external-crate black-box coverage for raw and typed subscriptions, filters, callback unsubscribe, access opt-in, bounded lag, and resume; exact implementation commit `20c7d86` passed PR Rust, MSRV, Docs, docs-site, HC/2 Linux/Interop, Java 17/21, and both performance tripwires in runs `31556527618`, `31556527634`, and `31556527643` |
| H25 | Native backend puts are invisible to Redis subscribers | complete | `feat(redis): bridge native puts to event subscribers` | lazy metadata-only native bridge, exact namespace fence, real Redis TCP client, generated documentation example; exact implementation commit `df7e754` passed PR Rust, MSRV, Docs, docs-site, HC/2 Linux/Interop, Java 17/21, and both performance tripwires in runs `31575224071`, `31575224099`, and `31575224068` |
| H26 | Native-to-Redis event failure boundaries are not independently frozen | in progress | `test(redis): harden native event bridge boundaries` | exact/custom namespace, RESP3, metadata-only separation, receiver teardown/no replay, forced lag/non-blocking writer/recovery, mixed-source no-duplicate, AUTH, and authenticated rediss tests are green locally; hosted evidence pending |

## Global execution rules

- Complete work in dependency order, not numerical order. H02/H04/H06/H09 can
  precede H01; production integration cannot precede schema and security gates.
- Keep HC/1 v1-v4 separately identifiable and runnable until a later explicit
  compatibility decision. No HC/2 decoder is mounted on an HC/1 route.
- All transports share one generated semantic contract. Adapter selection never
  creates per-transport operation, error, retry, bound, or security behavior.
- A fallback after TLS, authentication, authorization, generation, capability,
  or malformed-peer failure is always a downgrade error.
- Tests must name the falsifiable property. Acknowledgement without effect,
  skipped optional tooling, or checking a model instead of a socket is not green.
- Each completed H-item updates this ledger with exact commit and evidence paths.
- Every completion commit runs focused tests, `clippy -D warnings`, documentation
  checks, and `git diff --check`; affected compatibility gates also run.

## H01. Integrate the selected HC/2 adapter into production server seams

**Current truth.** `hydracache-client-plane-spike` is `publish = false` and is
not used by `hydracache-server`, `ClientDispatch`, daemon configuration, or an
SDK. Loopback success proves transport libraries, not a deployable listener.

**Implementation.** Introduce production crates only after H02/H03 prerequisites:
a transport-neutral connection runtime, selected adapter, validated
`client_plane` config, listener bind/readiness/drain, and existing identity,
tenant, quota, audit, and `ClientDispatch` reuse. HC/2 stays off by default.

**Evidence.** Start a real daemon process from config, reject plaintext and
untrusted clients, negotiate HC/2, execute an operation through real dispatch,
push an event, drain, and prove all accounting returns to zero. Prove HC/1 still
works concurrently and that port conflicts fail before partial startup.

**Dependencies.** H02, H03, H04, H06, H09, H12, H13. **Risk/rollback.** New
listener expands attack surface; rollback is disabling the off-by-default HC/2
config without changing core or HC/1.

**Done.** Real-process evidence is mandatory; importing the spike crate is not.

**Implementation evidence (2026-08-11).** `hydracache-server` now owns an
off-by-default generated gRPC HC/2 listener with mandatory mTLS identity,
eager TLS/readiness validation, shared HC/1/HC/2 dispatch state, applied-only
push events, and RAII connection/subscription/session accounting. Real daemon
tests prove bidirectional HC/1/HC/2 visibility, clean admin drain, and
fail-before-readiness behavior for port conflicts and unreadable TLS material;
the in-process socket gate rejects plaintext and a foreign client CA and proves
abrupt-close zero accounting. The same gates are wired into the Linux and
Docker HC/2 workflow through `client-plane-spike-check`. See
`docs/architecture/HC2_PRODUCTION_LISTENER.md`.

**Completion evidence (2026-08-11).** H03 accepted gRPC and the production
service now appends its aggregate H21 resource gauges and rejection counter to
the existing internal `/metrics` endpoint. The only label is the closed
`transport="grpc_bidirectional"` value; client identities, tenant IDs,
authorities, certificates, keys, and values are excluded. Unit tests prove the
privacy/cardinality boundary, while the real daemon process proves the series
are live during an authenticated mTLS HC/2 connection and remain absent from
the public client surfaces. H01 is complete; SDK and retained-artifact claims
remain independently owned by H16-H18.

## H02. Replace the minimal envelope with the authoritative HC/2 schema

**Current truth.** The W0 proto has only generation, kind, correlation id, and
opaque payload. It cannot define independent-language operations or evolution.

**Implementation.** Add a reviewed schema registry for handshake/capabilities,
stable errors, deadlines, cancellation, idempotency, data/TTL/CAS/batch,
subscriptions/watermarks/gaps, topology, sessions/fencing, and diagnostics.
Reserve removed field numbers/names and stable operation IDs. Generate contract
metadata rather than duplicating retry/version rules in SDKs.

**Evidence.** Descriptor/golden corpus, schema lint, duplicate/reserved ID
refusal, unknown-field round trip, additive-change acceptance, and breaking-
change canaries across generated Rust and Java. H15 independently gates clean,
hermetic Python generation from this same registry before the overall
cross-language gate can close.

**Dependencies.** W0 transport envelope. **Risk/rollback.** Premature surface
freezes debt; keep schema `v2alpha` until H03 and cross-language gates pass.

**Done.** No SDK owns handwritten wire IDs or a second operation definition.

## H03. Accept ADR-0019 only after the transport bake-off is complete

**Current truth.** gRPC is provisional; all adapters remain experimental. TLS,
256 correlations, deterministic faults, shutdown, and Python generation are
green, but the hostile socket corpus, equivalent Rust/Java clean-generation
proof, and the final operational-cost decision remain red.

**Implementation.** Complete the same boolean harness on all candidates,
capture dependency/operational costs, select one primary adapter, and retain or
remove losing spikes explicitly. Maturity cannot exceed evidence.

**Evidence.** ADR acceptance table with exact commands/artifacts for TLS,
identity, slow consumer, gap, cancellation, half-close, reset, disconnect,
malformed corpus, generation, and language fixtures.

**Dependencies.** H13, H15, H19, H20, H22 portions. **Risk/rollback.** Selecting
on convenience hides lifecycle defects; fallback is keeping ADR Proposed.

**Done.** ADR status changes in the same commit as the final green evidence.

**Completion evidence (2026-08-11).** Every candidate now runs the malformed,
truncated, oversized, cross-candidate, unknown-generation, and slow-consumer
gap corpus across real mTLS sockets. Rust code generation is byte-identical
across two isolated clean Cargo target directories; Java produces the same 65
files byte-for-byte across two `mvn clean generate-sources` runs. The
machine-checked `docs/testing/hc2-transport-bakeoff.json` retains the exact
boolean set, evidence paths, commands, dependency count, generated-source
sizes, debug test binary sizes, and operational tradeoffs. ADR-0019 accepts
gRPC/protobuf as the production-primary generation-5 adapter while retaining
dedicated TCP/TLS and HTTP/2 only as non-production comparison spikes.

## H04. Make authenticated discovery unforgeable in the API

**Current truth.** `DiscoveryAdvertisement::validate(bool)` lets callers assert
authentication with `true` and `ClientTransportPolicy::begin` assumes it.

**Implementation.** Separate decoded/untrusted discovery from an
`AuthenticatedAdvertisement` wrapper constructible only by the verified
channel/signature boundary. Selection accepts only the wrapper.

**Evidence.** Compile-fail or visibility tests prevent direct construction;
runtime tests reject untrusted, wrong-cluster, and invalid-generation documents.

**Dependencies.** None. **Risk/rollback.** Over-coupling TLS to policy; use a
narrow proof token so signed discovery can be added by H14.

**Done.** No public boolean or unchecked conversion can reach selection.

**Completion evidence (2026-08-09).** Selection now accepts only
`AuthenticatedAdvertisement`. Conversion from decoded discovery requires the
opaque `VerifiedDiscoveryEvidence` token, whose constructor and fields are not
public outside the adapter crate. The proof is bound to the expected cluster
identity; wrong-cluster and wrong-generation payloads fail closed. Two Rustdoc
compile-fail tests prove that neither the wrapper nor the proof can be forged by
an SDK caller, and runtime policy tests cover the accepted and rejected paths.

## H05. Bind fallback-safe unsupported responses to verified attempts

**Current truth.** `AuthenticatedUnsupported` is a caller-selected enum variant.

**Implementation.** A transport attempt receives an opaque ID and verified peer
binding. Only a decoded response from that attempt can create a fallback-safe
unsupported outcome. All local classification and unverified responses fail
closed.

**Evidence.** Wrong-attempt, stale-attempt, forged unsupported, and security-
failure tests; only current verified availability/unsupported advances order.

**Dependencies.** H04, H10. **Risk/rollback.** Excess state complexity; keep
attempt records bounded to configured preference length.

**Done.** Callers cannot manufacture a downgrade-permitting failure.

**Completion evidence (2026-08-09).** Selection now returns an opaque
`TransportAttempt` with a process-unique nonzero ID bound to connection
generation, authenticated cluster/epoch, canonical endpoint, adapter, and
bounded preference sequence. `after_outcome` accepts only an opaque outcome for
the exact current attempt. Availability is attempt-bound; unsupported additionally
requires `VerifiedAttemptEvidence` created by the authenticated peer seam.
Security/protocol outcomes remain terminal. Compile-fail proof prevents SDK
callers from constructing an outcome, while runtime tests reject wrong attempt,
same-generation parallel attempt, stale attempt, previous connection generation,
foreign peer evidence, and local unsupported classification. Only current
verified availability/unsupported advances the bounded order.

## H06. Replace string authority with a strict endpoint type

**Current truth.** Authority validation checks only empty/maximum length.

**Implementation.** Parse transport-specific schemes, DNS/IPv4/IPv6, required
port, SNI name, and prohibit userinfo/path/query/fragment. Bind scheme to
adapter and separate configured bootstrap rules from discovered-address policy.

**Evidence.** Table-driven valid/invalid URI corpus including IPv6 brackets,
IDN policy, zero/out-of-range ports, embedded credentials, wrong scheme, and
hostname/SNI mismatch.

**Dependencies.** None. **Risk/rollback.** URI normalization ambiguity; retain
canonical parsed fields and never compare raw strings for identity.

**Done.** Networking APIs receive structured authority, not an arbitrary string.

**Completion evidence (2026-08-09).** `TransportAuthority` retains only parsed,
canonical fields: adapter-bound scheme, DNS/IPv4/IPv6 host, non-zero `u16` port,
explicit TLS server name, and address origin. It rejects userinfo, path, query,
fragment, unbracketed IPv6, IDN input, invalid labels, missing/zero/overflow
ports, wrong schemes, and DNS/SNI mismatches. Authenticated discovery also
rejects unspecified, multicast, and localhost advertisements, while explicit
bootstrap configuration retains local-test access. Policy validation rechecks
that a parsed authority belongs to its declared adapter.

## H07. Prevent discovery rollback and cross-cluster rebinding

**Current truth.** A valid but old document can replace a newer view.

**Implementation.** Bind client state to cluster ID and highest authenticated
epoch; reject lower epochs, unexpected cluster changes, contradictory same-
epoch documents, and stale endpoint generations. Persist binding where SDK
policy requires it.

**Evidence.** Replay, same-epoch equivocation, cluster swap, reconnect, and
rolling-forward tests.

**Dependencies.** H04, H08, H14 for signed mode. **Risk/rollback.** Recovery
after intentional cluster replacement needs explicit operator reset, never
automatic downgrade.

**Done.** Authenticated replay cannot change routing or transport selection.

**Completion evidence (2026-08-09).** `DiscoveryState` survives reconnects and
binds itself on first acceptance to one cluster ID, highest authenticated
document epoch, accepted document, and highest epoch for every observed node.
Lower document epochs, contradictory same-epoch documents, automatic cluster
changes, and node-epoch rollback inside a newer document are rejected before
state mutation. Exact replay is idempotent, forward epochs advance, and an
intentional cluster replacement requires the named operator-reset method.
Runtime tests cover replay, equivocation, cross-cluster rebinding, reconnect,
node rollback, rolling forward, and explicit reset.

## H08. Model multi-node transport endpoints explicitly

**Current truth.** Duplicate candidates are rejected, so one cluster document
cannot represent the same adapter on several nodes.

**Implementation.** Keep one candidate listener per node policy, but advertise
bounded `(node_id, candidate, endpoint, readiness, epoch)` entries. Detect exact
duplicates and contradictions, not legitimate same-candidate nodes.

**Evidence.** Three-node advertisements, partial listener readiness, node
replacement, duplicate node IDs, conflicting authorities, and bounded-size
tests.

**Dependencies.** H06. **Risk/rollback.** Client smart-routing overclaim;
first release remains honest single-selected-endpoint mode.

**Done.** Discovery represents multiple nodes without creating ownership truth.

**Completion evidence (2026-08-09).** A discovery document now owns up to 256
unique `NodeAdvertisement` records. Each record has a bounded stable node ID,
non-zero node epoch, and at most one ready endpoint per transport. The same
transport may legitimately appear on three nodes. Exact duplicate node IDs,
per-node contradictory transports, cross-node authority reuse, invalid epochs,
empty/oversized records, and oversized documents fail closed. Selection remains
deliberately first-compatible-endpoint; node records carry connectivity, not
partition, leader, or ownership authority. The former cyclic H07 dependency was
corrected: H07 consumes this model, while H08 depends only on H06.

## H09. Strengthen lifecycle with typestate at bootstrap boundaries

**Current truth.** Runtime enum checks are explicit but every method remains
callable from every state.

**Implementation.** Use `Connection<Created> -> Connection<TlsVerified> ->
Connection<Authenticated> -> Connection<Ready>` for bootstrap, then a bounded
runtime handle for ready/draining/closed concurrency. Keep one cleanup owner.

**Evidence.** Compile-fail tests for dispatch-before-ready plus runtime tests for
drain, concurrent cancellation, idempotent close, and zero accounting.

**Dependencies.** H04 security proof seam. **Risk/rollback.** Generic explosion;
limit typestate to bootstrap and expose object-safe ready runtime.

**Done.** Invalid bootstrap sequencing is unrepresentable in production code.

**Completion evidence (2026-08-09; authorization gate extended by H13 on
2026-08-10).** `BootstrapConnection<State>` owns the linear `Created ->
TlsVerified -> Authenticated -> Authorized -> Ready` transition. Each successful
method consumes the previous state, and only negotiation yields the bounded
`SpikeConnection` runtime used for dispatch, drain, and close. The old public
runtime bootstrap methods are private. Rustdoc compile-fail tests prove that
created or merely authenticated connections expose neither dispatch nor
negotiation. Runtime tests retain negative TLS/identity/authorization checks,
prove drain behavior, cancellation versus completion has exactly one winner,
idempotent close, and zero accounting.

## H10. Fence every connection generation and stale message

**Current truth.** Correlation IDs are not bound to a connection generation.

**Implementation.** Add monotonic client connection generation to pending
calls, subscriptions, events, sessions, attempts, and completions. Late old-
generation data is counted and refused before state mutation.

**Evidence.** Late reply/event/cancel/heartbeat after reconnect, reused
correlation ID, and exactly-one completion tests.

**Dependencies.** H02 schema, H09 runtime. **Risk/rollback.** Counter wrap or
restart reuse; use explicit client/session identity plus checked generation.

**Done.** Old connections cannot complete or repair new-generation work.

**Completion evidence (2026-08-09).** `ConnectionGeneration` is nonzero,
monotonic, explicit at bootstrap, encoded in every spike frame, and refuses
counter wrap. The ready runtime validates it before dispatch or mutation for
replies, cancellation, events, and heartbeats; queued output carries the owning
generation. A three-adapter reconnect matrix reuses correlation and
subscription IDs, injects late old-generation wire and API actions, proves
unchanged accounting and zero dispatch, then proves exactly one current-
generation completion. Refusals increment a privacy-safe stale-generation
counter. The authoritative H02 envelopes carry the same generation for
invocations, subscriptions, and session traffic; H05/H11 must use this seam
when they add attempts and repair state.

## H11. Implement bounded reconnect, subscription repair, and session loss

**Current truth.** No production reconnect, backoff, listener re-registration,
watermark resume, conservative gap repair, or fenced-lock session behavior.

**Implementation.** Separate reconnect policy from invocation retry; bounded
backoff/jitter/attempts, endpoint selection, re-registration, watermark resume,
gap-triggered conservative repair, event dedupe, and fail-loud lock session loss.

**Evidence.** Seeded disconnect at every lifecycle point, server restart,
leader hint change, gap, duplicate event, exhausted retry, and lock ownership
loss matrix.

**Dependencies.** H05, H07, H08, H10, H12, H19. **Risk/rollback.** Retry storm
or false lock ownership; default to bounded failure, never silent continuation.

**Done.** Reconnect yields one completion and conservative cache/lock state.

**Completion evidence (2026-08-10).** The preview Rust SDK now wraps each
terminal single-generation `Hc2Client` in a transport-neutral recovery owner.
Reconnect attempts, exponential backoff, deterministic jitter, endpoints, and
overall deadlines are bounded; invocation retry has a separate policy and
replays only reads/read-only batches or idempotency-key-bound mutations.
Concurrent failures install exactly one incremented generation, verified leader
hints can select only configured same-transport endpoints, and cluster identity
cannot change. Logical subscriptions survive registration replacement, wake a
blocked reader with an out-of-band gap, require explicit conservative repair,
resume from the confirmed watermark, and discard duplicates. Every old-
generation fenced session is permanently lost and never silently reacquired.
Tests cover initial handshake selection, one safe completion, unsafe mutation
non-replay, retry exhaustion, blocked-reader repair, duplicate events, leader
change, and session loss. Eight payload-free H19 artifacts retain the exact
lifecycle plans. The contract and H01/language integration boundary are in
`docs/architecture/HC2_RECONNECT_REPAIR_POLICY.md`.

## H12. Bound every byte and connection-owned resource

**Current truth.** Frame-count bounds exist, but byte, batch, retry, deadline,
topology, session, tenant, and global budgets are incomplete.

**Implementation.** Define per-frame/message/batch/inbound/outbound byte budgets;
pending/reply/event/control/retry/reconnect/deadline/subscription/topology/session
count and byte budgets; per-identity/tenant/global admission; stable overload
errors; atomic accounting.

**Evidence.** Boundary and one-over tests, slow consumer, adversarial declared
length, cancellation races, no lost pending work on queue rejection, and zero
accounting after close.

**Dependencies.** H02 schema, H09 runtime. **Risk/rollback.** Limits too low;
make them validated configuration with safe defaults, never unbounded zero.

**Done.** The ownership ledger names and tests every retained allocation class.

**Completion evidence (2026-08-09).** Validated nonzero configuration now
bounds frame/message bytes, batch items, pending calls, reply/event/control
frame bytes and counts, retries, reconnects, deadlines, subscriptions, topology
nodes/bytes, and generation-fenced sessions/bytes. Reply rejection preserves
pending work; topology/session changes and identity/tenant/global admission are
atomic. Admission permits release by RAII. Boundary/one-over, event byte overflow
to one gap, adversarial length, cancellation, and disconnect tests cover all
owners named in `docs/architecture/HC2_RESOURCE_OWNERSHIP.md`; close returns the
expanded accounting snapshot to zero.

## H13. Complete the TLS and authorization negative matrix

**Current truth.** Unknown CA and missing client certificate are green.

**Implementation.** Test wrong hostname/SNI, expired/not-yet-valid certs,
wrong-client CA, EKU, chain depth/size, TLS version/cipher policy, identity and CA
rotation, authenticated authorization denial, and adapter fallback refusal.

**Evidence.** Same falsifiable matrix for all production-capable adapters;
dispatch counter stays zero on every pre-dispatch failure.

**Dependencies.** H06 endpoint/SNI, H05 failure provenance. **Risk/rollback.**
Synthetic PKI can miss operational rotation; add real-process rotation proof.

**Done.** Every security failure is classified, observable, and non-fallback.

**Completion evidence (2026-08-10).** `SecurityRejection` defines stable,
privacy-safe labels and maps every TLS/authentication/authorization failure to a
terminal H05 outcome. Real rustls and tonic loopbacks reject hostname/SNI,
expired/not-yet-valid server and client certificates, wrong client CA, wrong
server/client EKU, missing identity, unknown CA, and incompatible TLS versions
before any protocol byte or generated gRPC service open. A bounded common chain
policy proves exact/one-over depth and byte behavior. Bootstrap typestate now
requires `Authenticated -> Authorized -> Ready`; authenticated denial cannot
produce a dispatch-capable value. Rotation tests accept an old client CA during
explicit overlap and reject it after removal. The complete contract, production
integration boundary, and evidence commands are recorded in
`docs/architecture/HC2_TLS_AUTHORIZATION_POLICY.md`.

## H14. Add signed discovery with replay-safe key rotation

**Current truth.** Only authenticated-channel discovery is modeled.

**Implementation.** Define canonical bytes, signature algorithm, key ID,
issued/expiry times, cluster ID, epoch, generation, bounded endpoints, trust root,
rotation overlap, and explicit reset/recovery workflow.

**Evidence.** Tamper, unknown key, expired/not-yet-valid, replay, equivocation,
rotation overlap, removed key, and canonicalization vectors across languages.

**Dependencies.** H02, H04, H06-H08. **Risk/rollback.** A second PKI; channel-
authenticated explicit endpoints remain the initial safe mode.

**Done.** Offline discovery cannot be altered or replayed into accepted state.

**Completion evidence (2026-08-10).** The non-production spike now signs a
domain-separated canonical Ed25519 artifact that includes key ID, validity,
cluster, discovery epoch, HC/2 generation, nodes, and every endpoint security
and routing property. Decode is bounded and canonical; verification produces an
opaque post-signature type. `OfflineDiscoveryState` composes strictly increasing
issue time with the H07 cluster/document/node epoch gate. Tests cover tamper,
unknown/duplicate/excess/removed keys, time bounds, replay, equivocation,
rollback, rotation overlap, atomic cluster replacement, hostile framing, input
ordering, and a Java 17 reconstruction/verification of the fixed Rust vector.
The contract and rollback runbook are in
`docs/architecture/HC2_SIGNED_DISCOVERY_POLICY.md`. Explicit endpoints and
authenticated-channel discovery remain available, and this does not mark H01
production integration complete.

## H15. Make Python generation hermetic and independently executable

**Current truth.** Python 3 exists locally but protobuf/grpc tools are absent;
the gate must not hide a network `pip install`.

**Implementation.** Pin generator/runtime versions and hashes, use the vendored
protoc path, support an offline cache/wheelhouse, generate messages/stubs and
contract metadata, and add a clean-generation comparison.

**Evidence.** Clean checkout generation, offline-capable install, golden bytes,
unknown fields, streaming loopback, and package import on supported Python.

**Dependencies.** H02. **Risk/rollback.** Platform wheel drift; explicitly list
supported Python/OS matrix and fail required CI when missing.

**Done.** Python is generated/tested by the main HC/2 gate without internet.

**Completion evidence (2026-08-10).** `xtask` now generates Python messages and
type hints with `protoc-bin-vendored 3.2.0`, derives gRPC stubs and stable JSON
contract metadata from the descriptor, and compares the complete checked-in
package byte-for-byte. The main HC/2 gate verifies an exact SHA-256 wheelhouse,
creates a fresh venv using `pip --no-index --require-hashes`, and tests import,
the shared Rust/Java golden bytes, unknown-field retention, metadata, and a real
bidirectional loopback. Required CI pins CPython 3.12/Linux x86-64; local
evidence pins CPython 3.13/Windows x86-64. Other platforms fail loud instead of
building from source or consulting an index. The supply-chain contract and
extension/rollback rules are in `docs/architecture/HC2_PYTHON_GENERATION.md`.
This remains a non-production fixture and does not complete H01 or publish an
SDK.

## H16. Build a production Java HC/2 SDK

**Current truth.** Maven builds a generated codec golden test only.

**Implementation.** Publishable module with connection, invocation, listener,
topology, session, deadline/cancellation, transport policy, metrics, and stable
errors. Keep generated wire code internal and expose an idiomatic API.

**Evidence.** Real daemon interoperability, reconnect/repair, TLS matrix,
thread/leak checks, package metadata, examples, and previous-artifact tests.

**Dependencies.** H01-H15 relevant foundations. **Risk/rollback.** Premature API
promise; publish preview coordinates until compatibility gates pass.

**Done.** A consumer project builds and runs without repository internals.

**Completion evidence (2026-08-11).** The Java 17
preview module now exposes an idiomatic generated-wire-free API for data, CAS,
batch, listeners, topology, fenced sessions, deadlines/cancellation, stable
errors, transport policy, and metrics. `cargo xtask client-plane-java-sdk-check`
builds a separate Rust mTLS conformance process, executes positive and negative
PKI interop, kills and replaces a real Rust process through the Java H11
recovery owner, proves conservative replay, explicit subscription repair,
cluster pinning, and permanent fenced-session loss, then starts the actual
`hydracache-server` binary for mTLS operation/listener/session and clean admin
drain evidence. It requires zero-resource shutdown, installs the
JAR/POM/source/Javadoc artifacts, and builds/runs `tests/java-hc2-consumer`
using only the installed coordinate. H18 separately retains the first immutable
Java JAR/POM and executes it from an isolated Maven repository. Future
production old/new and rolling rows remain H18 work; they are not prerequisites
for completing the first preview SDK. The exact contract and limitations are
documented in `docs/architecture/HC2_JAVA_SDK.md`.

## H17. Build the transport-neutral production Rust HC/2 SDK

**Current truth.** Existing production client is HC/1 HTTP request/reply.

**Implementation.** Reuse generated schema and shared policy; connection,
invocation, listener, topology, session, retry/deadline, and observability
services behind adapter traits. Preserve HC/1 as a separate client identity.

**Evidence.** Real daemon tests for each enabled adapter, cancellation/reconnect,
no stale completion, boundedness, and HC/1 regression.

**Dependencies.** H01-H13. **Risk/rollback.** Accidental HC/1 behavior change;
separate modules/features and golden compatibility tests.

**Done.** Native HC/2 operations use no spike type or handwritten wire codec.

**Completion evidence (2026-08-11).** The distinct
`hydracache-client-hc2` preview crate now owns the authoritative generated
contract and exposes generated-wire-free native APIs for data/CAS/batch,
bounded listeners with explicit gap repair, monotonic topology, fenced
sessions, deadlines/cancellation, stable errors/retry advice, and pull-based
metrics. The public adapter boundary has one enabled implementation:
bidirectional gRPC over mandatory mTLS; it has no plaintext or HC/1 fallback.
`cargo xtask client-plane-rust-sdk-check` runs the SDK against both a separate
mTLS conformance process and the real `hydracache-server`, proves cancellation
cannot produce a stale completion, enforces pending/listener bounds, replays
the H11 reconnect/repair/session-loss matrix, requires zero retained owners,
and separately runs the unchanged HC/1 conformance suite. The production row
verifies cluster identity, PUT/GET, push subscription, fenced session, clean
client close, admin drain, and successful process exit. The gate finally
packages and verifies the crate. Exact scope and reproduction are documented
in `docs/architecture/HC2_RUST_SDK.md`.

## H18. Prove compatibility with retained real artifacts

**Current truth.** The first retained HC/2 clients exist and execute against the
current production daemon, but no retained older production daemon or later
contract generation exists yet.

**Implementation.** Retain exact previous client/server artifacts once HC/2
preview exists; test old/new both ways, HC/1+HC/2 concurrent listeners, rolling
upgrade, capability negotiation, unknown fields, and planned deprecation.

**Evidence.** Checksummed artifacts and machine-readable matrix; unsupported
rows fail loud rather than skip.

**Dependencies.** H01, H02, H16, H17. **Risk/rollback.** Self-comparison; use
immutable artifacts from prior release commits.

**Done.** Compatibility claims point to retained binaries, not source models.

**Completion evidence (2026-08-11).** The first immutable client baseline is retained under
`docs/testing/hc2-compat/artifacts/h17-preview-d1d1d44`. Its manifest binds the
Rust `.crate`, Java JAR/POM, byte lengths, SHA-256 digests, producer commit/tree,
and exact contract blob. H18 additionally retains Linux and Windows production
daemon archives from generation-5 commit `539d74e7` and generation-6 commit
`00508c68`, including internal executable receipts. Generation 6 accepts the
bounded 5..=6 window, advertises minimum/preferred/deprecated policy, and keeps
the exact selected-generation equality gate.

`cargo xtask client-plane-compat-check --require-complete` verifies every
outer and inner identity and executes all nine rows: old Rust/Java clients to
the new peer and daemon, current clients to the old daemon, HC/1+HC/2 shared
dispatch, rolling connection replacement, capability negotiation, additive
unknown fields, and planned deprecation. The rolling test starts both retained
daemon binaries, advances the recovery connection generation, verifies
post-replacement operations, and drains both. Its scope is protocol/lifecycle
compatibility, not replicated state migration between independent fixtures.
The matrix reports `pass=9`, `baseline-smoke=0`, and `blocked=0`; no skip state
exists. See
`docs/architecture/HC2_COMPATIBILITY_ARTIFACTS.md` for the policy and exact
reproduction commands.

## H19. Add a deterministic replayable network fault proxy

**Current truth.** Loopbacks cannot inject all byte/stream timing failures.

**Implementation.** Seeded proxy actions for fragment, coalesce, delay, reorder
where legal, duplicate, drop, one-way block, half-open, reset, late delivery,
bandwidth/window pressure, and close after byte N. Persist seed and action trace.

**Evidence.** Same semantic fault scenarios for candidates; failing seed replay
locally and in CI; bounded trace/artifact size.

**Dependencies.** W0 adapters. **Risk/rollback.** Proxy tests itself; unit-test
its byte schedule and retain direct loopbacks as controls.

**Done.** Every lifecycle failure used by H03/H11/H20 has a deterministic trace.

**Completion evidence (2026-08-10).** The bounded transport-neutral scheduler,
all declared actions, real async-stream delivery, exact retained replay, tamper
refusal, and payload-free artifact gate are green. Eight H11 traces bind the
reconnect/repair lifecycle and three H20 traces bind bounded drain termination.
Four additional retained plans are applied unchanged to the encoded semantic
frame of gRPC, bidirectional HTTP/2, and dedicated TLS/TCP: fragmentation,
coalescing, delay, and window pressure preserve decoding; close-after-byte,
duplicate, and reset fail closed. Sixteen checked-in artifacts are replayed by
`cargo xtask client-plane-fault-check`; direct loopbacks remain controls. See
`docs/architecture/HC2_DETERMINISTIC_FAULT_PROXY.md`.

## H20. Resolve HTTP/2 graceful drain and forced termination

**Current truth.** Application accounting reaches zero, then fixture transport
tasks are aborted because the h2 connection can remain alive.

**Implementation.** Define drain initiator, GOAWAY sequence, active-stream
completion, no-new-stream rule, deadline, reset/forced close, TLS close-notify,
and reason metrics. Never wait forever for peer cooperation.

**Evidence.** Clean drain, active request, client half-close, server GOAWAY,
uncooperative peer timeout, reset, and zero-accounting process/socket tests.

**Dependencies.** H09, H12, H19. **Risk/rollback.** Data loss on premature
forced close; only force after bounded deadline and explicit outcomes.

**Done.** No task abort is needed in the happy-path H2 test.

H20 is `complete`. `Http2DrainController` defines the initiator, two-GOAWAY
sequence, no-new-stream rule, active-stream boundary, TLS close-notify attempt,
absolute deadline, reset, final transport-owner drop, and privacy-safe reason
counters. The real mTLS/H2 loopback now joins both tasks without
`JoinHandle::abort`; it accepts cooperative completion or the explicitly
bounded deadline path and proves zero HydraCache accounting in either case.
Client half-close, blocked-peer timeout, and peer reset have retained H19
replay artifacts. See `docs/architecture/HC2_HTTP2_DRAIN_POLICY.md`.

## H21. Export stable privacy-safe observability

**Current truth.** Spike counters are local and unnamed outside the object.

**Implementation.** Stable bounded-cardinality metrics/tracing for transport,
state, connection generation, pending/queue bytes, retry, cancellation, gap,
repair, TLS/auth reason, deadline, session heartbeat/loss, and drain reason.
Never emit keys, values, credentials, certificates, or raw tenant IDs.

**Evidence.** Metric snapshots, cardinality/privacy tests, reconnect and failure
increments, zero gauges after close, diagnostics schema and operator runbook.

**Dependencies.** H09-H13. **Risk/rollback.** Cardinality/memory growth; enums
and hashed/bounded labels only.

**Done.** Operators can explain connection outcomes without sensitive data.

H21 is `complete` for the non-production client-plane contract.
`ClientPlaneDiagnostics` exports the versioned
`hydracache.hc2.client_plane.v1` schema through a telemetry-neutral sink. Its
transport/state/reason labels are closed enums, tenant correlation is limited
to 64 deployment-local salted buckets, generation is a gauge rather than a
label, and retained trace history is capped at 128 records. Contract tests
prove named series, bounded cardinality, JSON privacy, reconnect/failure
increments, and zero resource gauges after close. The catalog and incident
procedure are retained in
`docs/operations/HC2_CLIENT_PLANE_OBSERVABILITY.md`. H01 still owns mounting
this contract on the selected production listener/exporter; H21 does not
promote an adapter.

## H22. Install required Linux CI, interop, fuzz, and soak gates

**Current truth.** Main spike gates are green on local Windows; no required
Ubuntu workflow proves all languages, Docker interop, fuzz corpus, or soak.

**Implementation.** Fast required Linux job for format/clippy/schema/clean-
generation/Rust-Java-Python golden/lifecycle; Docker process interop; scheduled
fuzz and fixed-host soak with retained metadata. Shared CI is correctness and
regression evidence, never an absolute capacity claim.

**Evidence.** Required branch checks, intentional canary failures, artifact
retention, pinned actions/toolchains/images, timeout/concurrency policy, and
documented self-hosted tier.

**Dependencies.** All gates as they land. **Risk/rollback.** Flaky required job;
deterministic seeds, bounded retries only for infrastructure, never test failure.

**Done.** Release automation refuses an HC/2 claim when any required row is red.

**Implementation evidence (2026-08-10).**
`.github/workflows/hc2-client-plane.yml` and
`docs/testing/hc2-ci/h22-gates.json` define four exact, release-required lanes
with pinned actions, language toolchains, image digests, timeouts, concurrency,
and 30-day artifact retention. `client-plane-ci-check` rejects workflow/contract
drift; versioned receipts bind lane-specific metadata to one full candidate
SHA; `client-plane-ci-admission` rejects missing, duplicate, red, malformed, or
mixed-candidate evidence. Integration tests retain intentional missing-lane,
red-lane, and mixed-SHA canaries. The pinned Dockerfile proves Rust, Java 17,
Maven 3.9.11, and offline Python process interoperability, while the bounded
fuzz target and labelled fixed-host soak cover the scheduled tiers. Exact
commands, branch-check names, fixed-host activation, pin rotation, and the
non-capacity boundary are documented in
`docs/operations/HC2_CI_INTEROP_FUZZ_SOAK.md`.

**Hosted activation evidence (2026-08-11).** Exact candidate
`93f0de21568d60b77aa134fb44afcf8d6ceb586f` passed the hosted Linux, pinned
Docker interoperability, and bounded fail-closed fuzz lanes in workflow run
[`31488636299`](https://github.com/javaquasar/hydracache/actions/runs/31488636299).
Their job IDs, artifact IDs, and upload digests are retained in
`docs/operations/HC2_CI_INTEROP_FUZZ_SOAK.md`. Pull-request run
[`31488620419`](https://github.com/javaquasar/hydracache/actions/runs/31488620419)
independently passed the two required lanes. Repository branch protection was
then read back as strict on `main`, requiring `HC/2 Linux Required` and `HC/2
Docker Interop` from GitHub Actions, enforcing the rule for administrators, and
disallowing force pushes and deletion.

H22 remains `in progress` only until a labelled fixed host produces a retained
same-candidate soak receipt. Release admission correctly remains unavailable
while that fourth lane is absent. This operational step may not be replaced
with local evidence, a skipped lane, or the hosted correctness receipts above.
The checked-in fixed-host contract now also fails before the soak unless the
runner is non-root Ubuntu 24.04 x86_64, carries a bounded safe identity, exposes
the required native tools, and has the exact clean candidate checkout. Its
metadata is retained by the existing always-upload step.

## H23. Add Redis API keyspace event listeners

**Current truth.** The RESP facade can mutate and read the shared client
surface, but Redis clients have no live event-consumption path. The existing
native and HC/2 listener contracts cannot be advertised as Redis Pub/Sub: wire
shapes, RESP2 subscribed mode, RESP3 push frames, pattern matching, auth,
backpressure, and the node-local deployment boundary are different contracts.

**Implementation.** Add an off-by-default Redis keyspace-notification subset to
the live RESP connection. Support `SUBSCRIBE`, `UNSUBSCRIBE`, `PSUBSCRIBE`, and
`PUNSUBSCRIBE`; publish `__keyspace@0__:<key>` and
`__keyevent@0__:<event>` for successful `set`, `del`, `expire`, and `persist`
transitions. All protocols publish through one metadata-only, tenant-fenced,
bounded `ClientSurfaceState` mutation bus. RESP2 uses arrays and its restricted
subscribed-command mode; RESP3 uses push frames and retains ordinary command
execution. Authentication precedes admission, unique exact plus pattern
subscriptions have non-zero per-connection count and retained-byte bounds, and
a lagged receiver is closed loudly without delaying the writer.

**Compatibility boundary.** This is Redis keyspace-notification compatibility,
not arbitrary Pub/Sub. `PUBLISH`, durable history, replay, acknowledgements,
exactly-once delivery, values in messages, Redis multi-db, cross-daemon delivery,
and `CONFIG SET notify-keyspace-events` remain out of scope. The existing
`redis_scope:node-local` claim is unchanged. Consumers repair with ordinary
reads after reconnect or any gap.

**Evidence.** Pin the four command rows and their route owners in
`redis_compat_conformance.json`. Unit tests prove parser-neutral commands,
binary glob matching, RESP2/RESP3 encodings, tenant fencing, and bounded lag.
A real TCP `redis-rs` integration proves exact and pattern messages,
RESP-originated and HC/2-shaped shared-surface mutations, auth-before-subscribe,
disabled-by-default behavior, atomic unique-subscription admission against count
and retained-byte limits, unsubscribe, and zero active-subscriber accounting.
Server config tests prove explicit enablement and invalid-limit startup
rejection. The normal PR Rust/MSRV/Docs checks must be green on the
implementation commit.

**Dependencies.** Existing Redis RESP facade, H01 shared production dispatch,
and H12 resource-bound principles. **Risk/rollback.** Slow clients, tenant data
leakage, or an accidental queue claim are the primary risks. Rollback is
`HYDRACACHE_REDIS_KEYSPACE_EVENTS_ENABLED=false`; ordinary RESP cache commands
continue unchanged.

**Done.** Implementation, conformance manifest, operator documentation, focused
local tests, and exact GitHub PR checks are green. A model-only subscriber,
unbounded queue, value-bearing message, or prose-only claim is not completion.

## H24. Prove ordinary native API listeners as an external client

**Current truth.** The native event implementation already has broad unit
coverage, but those tests live under `src/tests` and compile inside the
`hydracache` crate. That boundary does not by itself prove that an application
can construct, filter, consume, and stop listeners using only the exported API.

**Implementation.** Add a dedicated integration target compiled as an external
crate. Exercise `HydraCache::subscribe`, the convenience subscriptions,
`TypedCache` namespace helpers, and callback handles through successful public
cache operations. Keep access events opt-in and retain the existing bounded,
non-blocking broadcast semantics.

**Evidence.** The target proves exact public metadata, kind/key-prefix/tag/origin
filter composition, typed physical-key isolation, callback delivery and
explicit unsubscribe, disabled-by-default and enabled access events, visible
`CacheEventRecvError::Lagged`, the lag counter, and continuation with
`next_event`. The test may not import private modules. It remains separate from
the Redis keyspace-notification wire test and remote HC/2 server-push evidence.

**Done.** `cargo test -p hydracache --test native_event_listeners --locked` and
the existing in-crate event suite are green. No listener guarantee is inferred
from protocol structs, acknowledgements, or documentation alone.

The exact implementation commit `20c7d86` passed the full PR Rust and MSRV
matrix in workflow run
[`31556527618`](https://github.com/javaquasar/hydracache/actions/runs/31556527618),
HC/2 Linux, Docker interoperability, and Java 17/21 in run
[`31556527634`](https://github.com/javaquasar/hydracache/actions/runs/31556527634),
and the documentation-site build in run
[`31556527643`](https://github.com/javaquasar/hydracache/actions/runs/31556527643).

## H25. Project native backend puts to Redis keyspace subscribers

**Current truth.** H23 proves RESP-originated and HC/2-shaped mutations on the
shared verified client surface. H24 proves the ordinary embedded native event
API. Neither item proves that an existing Redis subscriber observes a backend
that writes through `ServerRuntime`'s ordinary `HydraCache` handle.

**Implementation.** Give the server-owned RESP executor a metadata-only native
cache subscription. Create it lazily after successful Redis subscription
admission, multiplex it with the existing client-surface subscriber, and map
only successful `Stored` events under the configured `<namespace>:` physical
key prefix to Redis `set`. `cache.typed::<T>("redis")` is the default ergonomic
producer. Strip the prefix for the Redis key; do not copy values or merge the
native and RESP stores. Preserve the existing fail-loud lag disconnect.

**Evidence.** A real TCP integration target uses `redis-rs` to subscribe,
performs an out-of-namespace native put that must remain invisible, then writes
through the native typed cache and verifies both keyspace and keyevent message
shapes. The public mdBook includes the same compile-checked setup from
`docs-site/examples`.

**Done.** Production bridge, namespace-fence regression, conformance manifest,
generated documentation example, focused Rust tests, docs example check, and
normal PR Rust/MSRV/Docs checks are green. This item does not claim value-store
unification, native-to-Redis reads, approximate mappings for other native event
kinds, cross-daemon delivery, or durable replay.

The exact implementation commit `df7e754` passed the full PR Rust and MSRV
matrix in workflow run
[`31575224071`](https://github.com/javaquasar/hydracache/actions/runs/31575224071),
HC/2 Linux, Docker interoperability, and Java 17/21 in run
[`31575224099`](https://github.com/javaquasar/hydracache/actions/runs/31575224099),
and the documentation-site build in run
[`31575224068`](https://github.com/javaquasar/hydracache/actions/runs/31575224068).

## H26. Harden native-to-Redis event failure boundaries

**Current truth.** H25 proves the real default-namespace happy path and one
negative out-of-namespace write. It does not independently bind the native
source to exact subscriptions, RESP3, custom namespace collision behavior,
receiver teardown, no-replay semantics, the forced lag path, simultaneous
client/native sources, or authenticated TLS delivery.

**Implementation.** Keep the H25 producer invariant unchanged and add
black-box characterization at the existing public seams. Exercise an exact,
case-sensitive custom namespace; preserve native/RESP value-store separation;
refuse approximate remove-to-`del` projection; prove RESP3 push framing; drop
both event receivers at zero subscriptions; and force native receiver lag with
a one-entry event ring and a blocked socket. The forced-lag gate must show that
writers remain bounded, the stable error is emitted, accounting returns to
zero, and reconnect sees only future events. Run native and verified-surface
puts concurrently without claiming a global cross-source order. Complete the
security matrix with password AUTH and a real authenticated `rediss://`
connection.

**Evidence.** `redis_event_listeners` contains the native exact/custom
namespace, RESP3, lifecycle/no replay, forced lag/recovery, mixed-source, and
AUTH tests. `server_lifecycle` retains the default real-`redis-rs` proof and
adds the real TLS subscription. The conformance manifest names every test and
the focused H26 command uses the `native_backend` server-test filter.

**Done.** Focused listener and server lifecycle tests, conformance/doc checks,
formatting, workspace checks, and normal hosted PR gates are green. H26 does
not add value replication, replay, a global cross-source order, approximate
native event mappings, or cross-daemon delivery.

## Dependency-oriented execution order

```text
H04 -> H05
H06 -> H08 -> H07 -> H14
H02 -> H10 -> H11
H09 -> H12 -> H20
H13 + H15 + H19 + H20 -> H03
H02 + H03 + H04 + H06 + H09 + H12 + H13 -> H01
H01 -> H16 + H17 -> H18
H10 + H11 + H12 -> H21
all applicable evidence -> H22
H01 + H12 + Redis RESP facade -> H23
native cache event API -> H24
H23 + H24 + server-owned runtime -> H25
H25 -> H26
```

H01 production integration deliberately occurs after its prerequisites. This
order prevents a real daemon listener from making an immature wire contract or
downgrade policy externally reachable.
