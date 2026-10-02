# HydraCache 0.76.0 Semantic Cache and Vector Index Plan

> **At a glance**
>
> - **What:** add a bounded, multi-tenant `HydraSemanticMap<V>` and an LLM-oriented
>   `SemanticCache<V>` adapter. The authoritative response, vector, hard reuse scope, TTL, tags and
>   version live in the distributed 0.75 value plane; a partition-local ANN index is derived and
>   rebuildable. Queries perform hard security/policy filtering, bounded similarity search and an
>   authoritative final read before a response can be reused. Rust, Java and Python HC/2 clients,
>   shadow admission, resource controls, observability and exact-candidate evidence are included.
> - **Why:** exact-key caching misses paraphrases and near-duplicate tool requests, while a generic
>   vector store does not by itself provide HydraCache's TTL, tags, single-flight, invalidation,
>   bounded memory and explicit failure semantics. The useful product is a correctness-first
>   semantic response cache that can avoid an LLM/tool call without crossing tenant, model,
>   prompt-template, retrieval-snapshot or safety-policy boundaries.
> - **After (depends on):** published `0.75.0` with its distributed IMap authority, routing,
>   replication, failover, rebalance, namespace generation, TTL/version/tombstone state and
>   cross-surface evidence. A node-local or unreplicated substitute cannot satisfy this plan.
> - **Unblocks:** AI gateway caching, bounded tool-result reuse, agent workflow acceleration and a
>   later separately scoped agent-memory/retrieval release. It does not itself claim RAG storage or
>   a general vector database.
> - **Status:** planned. Automatic semantic reuse remains off until the exact namespace/model
>   profile passes shadow quality, isolation, resource and failure gates.

Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · gates:
[`../GATES.md`](../GATES.md) · predecessor:
[`V0_75_EXTENDED_IMAP_SURFACE_PLAN.md`](V0_75_EXTENDED_IMAP_SURFACE_PLAN.md).

Read `CLAUDE.md`, `docs/RULES.md`, `docs/GATES.md`, `docs/COMPAT.md`, the final 0.75 release
evidence, `docs/architecture/HC2_DISTRIBUTED_VALUE_PLANE.md`, and the 0.75 authority/failure
matrices before implementation. This plan inherits R-1 through R-11. In particular, approximate
search may broaden discovery but may never weaken tenant isolation, make stale state authoritative,
hide an unavailable partition, execute user expressions remotely, or turn an uncertain response
into a cache hit.

## Release theme and competitive boundary

Embeddings are useful for semantic similarity and semantic search. For large collections, an
indexed nearest-neighbour structure is required rather than an unbounded full scan. These are
established building blocks, not a unique HydraCache claim:

- [OpenAI's embedding research](https://openai.com/index/text-and-code-embeddings-by-contrastive-pre-training/)
  describes embeddings as useful for text similarity and semantic search;
- [OpenAI's retrieval guidance](https://help.openai.com/en/articles/8984342) recommends a vector
  database for fast nearest-neighbour lookup over many vectors;
- [Redis Vector Sets](https://redis.io/docs/latest/develop/data-types/vector-sets/) already expose
  vector storage, HNSW similarity and filters;
- [Redis semantic-cache guidance](https://redis.io/docs/latest/develop/use-cases/semantic-cache/)
  distinguishes full-response semantic reuse from provider prompt-prefix caching and from RAG.

Therefore 0.76 does not compete by adding an undifferentiated `VectorSet`. It combines a narrowly
defined response/tool-result cache with HydraCache's explicit TTL, tags, namespace generation,
single-flight, invalidation, resource accounting and 0.75 distributed authority. The product
claim is intentionally narrower:

> HydraCache 0.76 provides a bounded distributed semantic cache for application-approved LLM
> responses and tool results. A hit requires an authenticated hard scope match, an admitted
> similarity policy, a complete-enough ANN result under the requested availability policy and a
> final authoritative version/TTL validation. The ANN index is disposable; ambiguity degrades to
> a miss and never authorizes response reuse.

The release may report scenario-scoped latency, avoided-call and resource results. It may not say
that HydraCache is a general vector database, that semantic similarity proves answer correctness,
that one threshold works for every application, or that HydraCache is universally faster/cheaper
than Redis or a vector database.

## Inherited 0.75 boundary

0.76 consumes these 0.75 capabilities rather than rebuilding them:

1. canonical tenant/namespace/key framing shared by Rust, Java and Python;
2. epoch-fenced partition authority and at most one server-side owner proxy hop;
3. replicated value/version/TTL/tombstone/idempotency state and explicit acknowledgement levels;
4. failover, repair, rebalance, namespace generation and bounded reclamation;
5. authenticated internal routes and per-tenant/per-partition admission;
6. one authoritative value plane behind enabled HC/1, HC/2 and RESP surfaces;
7. exact-candidate failure, RPO/RTO, rolling-upgrade and release-evidence machinery.

If any of these is absent in the published 0.75 boundary, W0 must either close it before semantic
work begins or reject 0.76 as blocked. A local-only ANN demo may be retained as non-promotable
research but cannot support the distributed `HydraSemanticMap` claim.

## Non-goals

- No server-side call to OpenAI, Anthropic, an embedding model or any other provider. Applications
  supply vectors and own provider credentials, retries, data residency and billing.
- No document chunking, OCR, reranker, retrieval pipeline, generative model hosting, training,
  fine-tuning or general RAG/vector-database claim.
- No arbitrary metadata expression language, remote callback, script, SQL, predicate or user code.
  Filters are a fixed typed hard-scope schema plus a small bounded allowlisted attribute set.
- No semantic coalescing of non-identical prompts. Similar prompts may reuse an admitted completed
  response; only an exact request fingerprint may own an in-flight generation lease.
- No global linearizable ANN snapshot. The response describes completeness, epoch and lag; the
  default auto-reuse policy fails closed when required shards are unavailable or rebuilding.
- No automatic universal similarity threshold. Each auto-reuse profile is admitted by a frozen
  labelled corpus and remains namespace/model/policy specific.
- No raw prompt or response logging, vector labels containing customer data, unbounded `top_k`,
  unbounded dimensions, unbounded index count or unbounded fan-out.
- No claim that provider-side prompt caching is replaced. It remains complementary because it may
  reduce model input work even when the application semantic cache misses.
- No agent memory or durable conversation log. Those require retention, consent, deletion,
  summarization and ordering contracts beyond a rebuildable response cache.

## Mandatory invariants

| Invariant | Required behavior | Falsifier |
| --- | --- | --- |
| Authoritative state | `SemanticRecord` in the 0.75 value plane owns payload, vector, hard scope, TTL, tags and version. | Deleting the ANN index changes authoritative reads or prevents a complete rebuild. |
| Derived index | ANN nodes contain only enough data to produce candidate IDs/versions/scores. | ANN output is returned without an authoritative final read. |
| Hard boundary first | Authenticated tenant, namespace generation and embedding/reuse profile select the physical logical index before similarity search. | Another tenant's candidate is examined and filtered only after scoring. |
| Version validation | A candidate hit must still exist and match indexed version, scope and TTL. | Stale ANN node returns a deleted, expired or replaced response. |
| Safe degradation | Missing shard, epoch mismatch, excessive lag, corrupt checkpoint or uncertain final read becomes `Miss`/`Unavailable`, never a reusable hit. | Partial results are silently relabelled complete. |
| Exact-only coalescing | In-flight suppression is keyed by canonical exact request digest and fenced lease generation. | Similar but distinct prompts share a loader result. |
| Boundedness | Dimensions, vector bytes, payload bytes, top-k, indexes, entries, queues, fan-out, rebuild work and per-tenant resources have enforced limits. | A client can grow work or memory without a configured bound. |
| Privacy | Prompts/responses/vectors are absent from logs, metric labels and traces; optional prompt retention is explicit and encrypted by existing storage policy. | An error or profile exposes request content or vector coordinates. |
| Rebuildability | A clean index can be reconstructed from an exact authority snapshot plus ordered delta without lost accepted mutations. | Ready is reported before watermark/cutover proof. |
| Quality honesty | Auto-reuse requires a profile-specific admitted threshold and corpus receipt; otherwise mode is shadow or manual-inspection only. | A benchmark hit-rate target loosens the threshold after candidate observations. |

## Canonical public model

W0 freezes names, but semantics below are mandatory. The server accepts precomputed vectors; SDK
helpers may integrate application-provided embedding callbacks without adding a provider dependency.

```rust
pub struct SemanticIndexId {
    pub namespace: Namespace,
    pub name: String,
    pub embedding_profile: EmbeddingProfileId,
}

pub struct EmbeddingProfile {
    pub model_id: String,
    pub model_revision: String,
    pub dimensions: u16,
    pub metric: SimilarityMetric, // 0.76: Cosine only
    pub canonicalization_version: u16,
}

pub struct ReuseScope {
    pub locale: Option<String>,
    pub generation_model: ModelIdentity,
    pub prompt_template: Digest,
    pub system_prompt: Digest,
    pub tool_schema: Option<Digest>,
    pub retrieval_snapshot: Option<Digest>,
    pub safety_policy: Digest,
    pub decoding_profile: Digest,
    pub application_scope: Option<Digest>,
}

pub struct SemanticEntry<V> {
    pub id: EntryId,
    pub embedding: EmbeddingVector,
    pub scope: ReuseScope,
    pub value: V,
    pub tags: TagSet,
    pub ttl: TtlDirective,
}

pub struct SemanticQuery {
    pub index: SemanticIndexId,
    pub embedding: EmbeddingVector,
    pub scope: ReuseScope,
    pub top_k: NonZeroU16,
    pub minimum_score: SimilarityScore,
    pub availability: SemanticAvailability,
}

pub enum SemanticLookup<V> {
    Hit(SemanticHit<V>),
    Miss(SemanticMiss),
    Unavailable(SemanticUnavailable),
}

pub struct SemanticHit<V> {
    pub id: EntryId,
    pub value: V,
    pub score: SimilarityScore,
    pub version: EntryVersion,
    pub age: Duration,
    pub remaining_ttl: RemainingTtl,
    pub proof: SemanticHitProof,
}
```

`ReuseScope` is not arbitrary metadata. Every field changes whether reuse is permissible. The
authenticated tenant and namespace generation are injected by the server and cannot be supplied or
overridden by a client payload. The exact list and optionality are versioned in HC/2 and
`docs/COMPAT.md`.

### Vector canonicalization

0.76 supports finite `f32` vectors and cosine similarity only. Wire values use length-delimited
little-endian IEEE-754 bytes with an explicit dimension and canonicalization version. The owner:

1. rejects NaN, infinity, signed non-canonical zero where prohibited, dimension mismatch and a
   zero-norm vector;
2. accumulates the norm in `f64`, normalizes deterministically and stores canonical `f32` bytes;
3. hashes profile identity plus canonical bytes, never language-local array/hash behavior;
4. returns the canonical vector digest in the mutation receipt;
5. proves Rust/Java/Python golden equality on x86_64 and aarch64.

Quantization is deferred until a separately measured proposal proves recall and memory bounds.
Changing canonicalization, dimensions, metric or model revision creates a distinct index profile;
in-place reinterpretation is forbidden.

### Lookup and reuse sequence

1. Authenticate and derive tenant/namespace generation.
2. Validate the fixed embedding profile, vector, `top_k`, deadline and reuse scope.
3. Resolve one `SemanticIndexId` and its current query epoch.
4. Fan out a bounded query to member-local index aggregators for the index shards they currently
   own; each returns at most `top_k` candidate IDs, indexed versions and scores.
5. Merge deterministically by descending score then stable `EntryId`; deduplicate migration overlap.
6. Apply the frozen availability rule. `RequireComplete` is the default for auto-reuse and rejects
   missing/rebuilding/stale shards.
7. Read candidates from authoritative owners in bounded order and validate version, TTL, namespace
   generation, complete hard scope and payload admission.
8. Return the first validated candidate at or above the threshold, otherwise an explicit miss.
9. Record bounded reason counters without logging prompt, response, vector or tenant-controlled text.

An ANN false positive is filtered by the score/hard scope; a stale candidate is filtered by the
authoritative read. An ANN false negative is a cache miss and may cost an LLM call, but cannot serve
the wrong entry. This asymmetry is the core safety contract.

## Authoritative and derived storage layout

```text
0.75 authoritative partition
  SemanticRecordKey(index_id, entry_id)
  SemanticRecord {
    canonical_vector, vector_digest, reuse_scope,
    encoded_value, tags, expires_at, entry_version,
    namespace_generation, created_at, policy_version
  }

member-local derived state
  SemanticIndexShard(index_id, value_partition_set, schema_version)
    FlatReferenceIndex | HnswIndex
    indexed_watermark[partition]
    tombstone_count
    checkpoint_digest
    build_generation
```

Semantic records reuse the 0.75 owner mutation, replication and TTL/tag lifecycle. The index is
not synchronously replicated as authoritative state. A committed semantic mutation emits a
bounded versioned index delta after the authoritative acknowledgement point. Index loss or
corruption triggers rebuild from the authoritative partition snapshot plus deltas. A mutation may
return `Stored { index_visibility: Pending }`; applications needing read-after-index may call a
bounded `await_indexed(version, deadline)`.

Each member owns an aggregator over the value partitions currently assigned to it, avoiding one
network RPC per partition. The coordinator fans out at most once per admitted member, with a
configured member-count ceiling. Rebalance may temporarily produce old/new copies; epoch and
`(entry_id, entry_version)` dedup prevent double candidates. The old shard remains queryable until
the new shard has snapshot-plus-delta watermark parity and the authority epoch commits cutover.

## Index lifecycle and state machine

```text
Absent -> Building(snapshot) -> CatchingUp(delta) -> Ready
   ^            |                    |                |
   |            v                    v                v
   +--------- Failed <----------- Degraded <------ Draining
```

- `Building` and `CatchingUp` may serve shadow diagnostics but cannot satisfy `RequireComplete`.
- `Ready` requires schema/profile match, authority snapshot digest, ordered delta parity and a
  committed query epoch.
- `Degraded` records exact missing partitions/lag reason. It never reports complete.
- `Draining` remains readable through the cutover watermark but accepts no new ownership.
- `Failed` retains bounded diagnostics and schedules a budgeted rebuild; repeated failure opens a
  circuit breaker rather than spinning.

Checkpoint files contain the index schema/profile, covered partition epochs, watermarks, entry
count, canonical vector digest aggregate and payload checksum. They contain no cached response
payload. Unknown/corrupt/truncated checkpoints fail closed and rebuild; they are never partially
accepted.

## Error and availability model

| Outcome | Meaning | Retry/reuse rule |
| --- | --- | --- |
| `Miss::NoCandidate` | Complete admitted search found no valid score. | Application may generate. |
| `Miss::CandidateInvalidated` | ANN candidate was expired/replaced/deleted before final read. | May retry once within the same deadline, then generate. |
| `Miss::ShadowOnly` | Profile has no auto-reuse admission. | Generate; retain comparison receipt. |
| `Unavailable::IndexNotReady` | Required shard is building/catching up. | Never reuse; caller may generate or wait. |
| `Unavailable::Incomplete` | Required member/shard could not answer. | Never relabel as miss in strict mode. |
| `Unavailable::LagExceeded` | Indexed watermark exceeds configured lag budget. | Never reuse until recovered. |
| `Unavailable::AuthorityUncertain` | Final owner/version/TTL validation is uncertain. | Never reuse; use existing HC/2 retry advice. |
| `Rejected::Scope/Profile/Vector` | Request violates hard contract or bounds. | Correct request; no retry loop. |
| `Rejected::Overloaded` | Queue/concurrency/byte admission is full. | Bounded backoff; exact counters expose pressure. |

`BestEffortDiagnostic` may return partial candidate lists for offline evaluation, explicitly marked
non-reusable. Public convenience methods for LLM response reuse always use `RequireComplete` unless
an application opts into a separately named and documented weaker mode.

## Dependency graph

```text
W0
 ├─> W1 ─> W2 ─> W3
 │                 ├─> W4 ─> W5
 │                 └─> W6
 ├─> W7 <────────────── W5,W6
 ├─> W8 <────────────── W1,W2,W7
 ├─> W9 <────────────── W1,W3,W7
 └─> W10 <───────────── W5,W6,W7

W1-W10 ─> W11 ─> W12 ─> W13 ─> W14
```

W4's exact flat index is the permanent oracle and fallback for small admitted indexes. W5 may not
land an approximate implementation until W4's differential and W12's quality corpus can falsify
it. W13 performance work begins only after semantic/security equivalence is green.

## Expected code and artifact map

| Path | Planned responsibility |
| --- | --- |
| `crates/hydracache-core/src/semantic.rs` | Stable semantic IDs, profiles, scopes, outcomes, bounds and codec-independent domain types. |
| `crates/hydracache-semantic-index/` | New internal crate: `SemanticIndex` trait, flat oracle, admitted HNSW implementation, checkpoint/rebuild and differential tests. No network or authoritative payload ownership. |
| `crates/hydracache/src/semantic.rs` | Embedded/local `HydraSemanticMap<V>` and `SemanticCache<V>` facade using the same domain rules. |
| `crates/hydracache-client-hc2/proto/hc2_contract.proto` | Versioned semantic put/query/remove/status/await-indexed messages, fixed hard scope and explicit outcomes. |
| `crates/hydracache-client-hc2/src/{client,types}.rs` | Rust HC/2 API, deadlines, retry advice and exact-fingerprint helper. |
| `crates/hydracache-server/src/semantic.rs` | Authentication-derived scope, request admission, coordinator, authoritative validation and index lifecycle wiring. |
| `crates/hydracache-cluster-transport-axum/src/lib.rs` | Authenticated internal member-local ANN query, rebuild snapshot/delta and cutover frames with epoch/profile fencing. |
| `crates/hydracache-observability/src/management.rs` | Bounded semantic index/readiness/resource/quality counters; no content/vector labels. |
| `sdks/java/hydracache-client-hc2/` | Java semantic types/client and cross-language goldens. |
| `sdks/python/hydracache-client-hc2/` | Python async semantic client, typed models, wheel generation and conformance. |
| `tests/semantic-reference/` | Independent exact cosine oracle, seeded corpora, hard negatives and cross-product differential driver. |
| `tests/semantic-process/` | Multi-daemon routing, failover, rebuild, rolling-upgrade, fairness and security tests. |
| `tests/semantic-benchmark/` | Common HydraCache/Redis/no-cache adapters with precomputed embeddings and open-loop load. |
| `docs/architecture/SEMANTIC_CACHE_076.md` | Authority/index/query/rebuild/failure ADR-level architecture. |
| `docs/testing/semantic/0.76/` | Frozen profiles, corpora, statistics, attempts, manifests and exact-candidate receipts. |
| `docs/security/SEMANTIC_CACHE_076.md` | Threat model, privacy, deletion, tenant isolation, side-channel and operator guidance. |

W0 replaces expected paths with exact post-0.75 paths before implementation. A move cannot remove
the semantic owner, tests or release gate.

## W0. Freeze scope, identities, claims and evidence contracts

**Changes.** Create the architecture decision, capability manifest, scenario matrix, quality
contract, statistics plan, threat model and release-evidence registry before product mutation.
Freeze the published 0.75 source/tag/artifacts and exact Redis comparison identity/configuration.

**Implementation.** Record supported cosine-only profiles, maximum dimensions/top-k/members,
local versus member modes, storage profile, payload limit, default `ShadowOnly`, availability
semantics, corpus provenance and every public non-claim. Freeze provider-independent precomputed
vectors so repeated measurements never call a live embedding API. Define D0 baseline → D1 flat
oracle → D2 ANN candidate → D3 shadow → D4 active/exact-candidate promotion states.

**Tests and evidence.** Add `cargo xtask semantic-contract-check --release 0.76`, release-scoped
dynamic canaries and a coverage manifest. Negative fixtures must reject a changed embedding model,
dimension, metric, scope digest, corpus label, threshold, index configuration or comparison cell.

**Exit.** No implementation starts until the contract distinguishes semantic response caching,
RAG retrieval, provider prompt caching and generic vector storage, and until every later numerical
claim has a frozen estimator and practical threshold.

## W1. Define the stable semantic API and explicit outcomes

**Changes.** Add `SemanticIndexId`, `EmbeddingProfile`, `EmbeddingVector`, `ReuseScope`,
`SemanticEntry`, `SemanticQuery`, `SemanticLookup`, `SemanticHitProof`, availability and status
types to core. Add local/embedded and HC/2 facades without exposing index-library types.

**Implementation.** Use builders/validated constructors so invalid dimension, NaN/infinity,
zero-norm, oversized top-k and inconsistent TTL never cross the API boundary. Keep `put`, `query`,
`remove`, `invalidate_tag`, `await_indexed` and `status` distinct. No boolean result may conflate
miss, incomplete search, overload, uncertain authority or invalid input.

**Tests.** Unit/property tests cover every constructor and outcome. SemVer snapshots, rustdoc
examples and compile-fail tests prevent accidental generic/index leakage. Retained 0.75 clients
must ignore the new capability and remain byte compatible.

**Exit.** Rust signatures, Java/Python mappings, retry advice and unsupported operations are frozen
before HC/2 field numbers or product storage are added.

## W2. Canonicalize exact request identity and hard reuse scope

**Changes.** Define a versioned canonical exact request fingerprint and fixed hard scope. The exact
fingerprint covers normalized application-owned request bytes plus every field whose change can
change a safe answer; the semantic scope controls candidate eligibility.

**Implementation.** Include tenant/namespace generation, generation model/revision, prompt-template
digest, system-prompt digest, tool-schema digest, retrieval-snapshot digest, safety-policy digest,
decoding profile and application scope. Do not infer these from prompt text. Encode with a stable
length-framed codec and domain-separated digest. Tenant comes only from authentication.

**Tests.** Rust/Java/Python golden vectors cover empty/binary/Unicode/max-size fields, omitted versus
empty optionals, reordered input maps, old/new codec versions and tenant forgery. Property tests
prove any semantic field change changes the appropriate identity. Collision fixtures and unknown
versions fail loudly.

**Exit.** One canonical implementation feeds storage, routing, single-flight and receipts; SDKs do
not construct language-specific hashes.

## W3. Store authoritative semantic records in the distributed IMap plane

**Changes.** Add versioned `SemanticRecord` encoding and keys to the 0.75 backend. The record owns
canonical vector, hard scope, payload, tags, TTL, namespace generation and policy version.

**Implementation.** Route mutations through one owner-authoritative operation and inherited backup
acknowledgement. Emit an index delta only after the authoritative commit point. `put` returns record
version, vector digest and `Pending/Visible/Unavailable` index visibility. Delete, expiry, tag
invalidation, replacement and namespace deletion produce versioned removal deltas/tombstones.

**Tests.** Reference-model sequences cover put/replace/remove/expire/tag invalidate/namespace reuse,
duplicate requests and owner failure before/after ACK. Cross-surface tests ensure semantic records
cannot be corrupted by ordinary map APIs or exposed through RESP as an undocumented encoding.

**Exit.** Deleting every derived index loses no authoritative data, and rebuilding records produces
the same live record digest/cardinality.

## W4. Implement the bounded exact flat index as oracle and small-index mode

**Changes.** Introduce `SemanticIndex` with `insert`, `remove`, `search`, `snapshot`, `restore`,
`watermark`, `stats` and `clear`. Implement `FlatReferenceIndex` first.

**Implementation.** Keep vectors in canonical contiguous storage with explicit byte accounting.
Search computes cosine in a deterministic reference path, orders by score then `EntryId`, applies
scope at index selection and enforces top-k/deadline/cancellation. Small namespaces may permanently
use flat mode below a frozen cardinality threshold.

**Tests.** Compare against an independent scalar `f64` oracle, including ties, near-threshold
values, extreme finite components, cancellation and boundary cardinalities. Fuzz vector decoder and
query limits. Miri covers unsafe-free storage code; no SIMD/unsafe is admitted in the oracle.

**Exit.** The flat index is correct, bounded and retained permanently as the ANN differential
oracle, rebuild fallback and deterministic test implementation.

## W5. Add an admitted partition-local HNSW index

**Changes.** Select or implement an HNSW backend only after a dependency/SBOM/MSRV/license/security
review. Keep it behind `SemanticIndex`; no third-party type reaches public or wire APIs.

**Implementation.** Freeze `M`, `ef_construction`, query `ef`, allocation/accounting behavior,
delete/tombstone policy, checkpoint schema and rebuild trigger per profile. The authoritative vector
is never quantized in 0.76. HNSW background work uses bounded CPU, memory, tasks and queues and is
cancellable during drain/rebuild/shutdown.

**Tests.** Differential every seeded operation trace against flat top-k; measure recall@k against
the exact oracle; exercise insertion order permutations, delete/reinsert, duplicate vectors, ties,
checkpoint corruption and repeated rebuild. Fuzz checkpoints and operations. Dependency negative
canary rejects a backend that cannot report/account retained bytes or bound construction work.

**Exit.** Recall, build cost, memory/vector and query tail meet the W0-frozen profile limits without
semantic/security divergence; otherwise flat mode ships alone and no scale claim is made.

## W6. Couple TTL, tags, replacement and deletion to index freshness

**Changes.** Add a versioned bounded index-delta journal per authoritative partition and a
watermark/lag model. Integrate every 0.75 removal cause and namespace reclamation.

**Implementation.** Deltas contain index/profile ID, entry ID, authoritative version, operation,
vector digest and required index data but no response payload. Apply is idempotent and ignores stale
versions. Candidate final reads protect correctness while lag affects completeness/readiness.
Compact journal only after all required index owners/checkpoints pass the safe watermark.

**Tests.** Delay/reorder/duplicate/drop deltas; expire during query; replace vector and scope;
invalidate tag during rebuild; delete/recreate namespace; crash around checkpoint/journal compaction.
Assert no old response is returned, lag is visible, journals/tombstones remain bounded and eventual
rebuild digests equal a clean rebuild.

**Exit.** Every authoritative lifecycle cause has a tested derived-index consequence and a stale
index can cause only explicit unavailability/miss, never stale reuse.

## W7. Implement distributed fan-out, deterministic merge and failover

**Changes.** Add authenticated internal semantic-query frames and a server coordinator. Member-local
aggregators search all locally owned index shards and return bounded candidates plus completeness,
epoch, watermark and lag proofs.

**Implementation.** Fan out once per admitted live member, not once per value partition. Bound
member count, candidate bytes, in-flight queries and deadline. Merge score then `EntryId`, dedup
old/new owners during rebalance, enforce query epoch and validate final candidates through 0.75
authoritative routing. `RequireComplete` rejects any missing/incompatible shard.

**Tests.** Three/five-node real-process suites cover non-owner ingress, owner failure, backup
promotion, concurrent rebalance, stale owner, network partition, duplicate responses, slow member,
deadline/cancel and rolling upgrade. A canary drops one shard and must make strict auto-reuse fail
closed rather than improve apparent latency.

**Exit.** Multi-node results match a single exact oracle for all complete searches; incomplete and
ambiguous states remain explicit and bounded.

## W8. Add exact-only single-flight and generation leases

**Changes.** Provide `lookup_or_generate` helpers without allowing similarity to identify in-flight
work. Local mode reuses existing single-flight; member mode may request an exact-fingerprint fenced
generation lease built on the 0.75 lock/session foundation.

**Implementation.** Flow: semantic lookup → exact fingerprint lease → recheck semantic cache after
lease acquisition → invoke application loader → authoritative put → release. Lease expiry/session
loss fences publication; the LLM call may finish but a stale holder cannot overwrite a newer result.
Applications can choose local-only collapse when they do not want a distributed lease.

**Tests.** Hundreds of identical requests collapse as configured; paraphrases never share the
in-flight result; leader/holder/client death, timeout, cancellation, lease expiry and ambiguous put
preserve fencing. Loom/model tests cover acquire/recheck/publish/release interleavings.

**Exit.** No loader result can be published by a stale generation and no non-identical fingerprint
is coalesced, even when its vector score is 1.0.

## W9. Enforce tenant isolation, privacy and deletion

**Changes.** Define physical logical index selection by authenticated tenant, namespace generation
and embedding profile. Add a threat model for content leakage, timing/cardinality side channels,
malicious vectors, filter forgery and deletion obligations.

**Implementation.** Never search a cross-tenant graph and filter afterward. Do not log prompt,
response, vector coordinates or tenant-controlled scope fields. Raw prompt retention is off by
default; optional retention is a separate encrypted payload field with explicit policy. Namespace
delete fences new operations, removes authoritative records, drains indexes/checkpoints and proves
reclamation before name reuse.

**Tests.** Hostile HC/2/internal frames forge tenant/generation/profile, oversize vectors, NaN/
infinity, decompression/codec bombs and replay epochs. Timing/cardinality tests verify bounded
coarse diagnostics. Secret scanning covers logs, traces, crash artifacts and evidence packages.

**Exit.** Security review and real-process isolation gates prove no cross-tenant candidate access,
not merely no cross-tenant response after application filtering.

## W10. Bound memory, CPU, queues, fan-out and noisy tenants

**Changes.** Add `SemanticIndexConfig` and per-tenant/index accounting: dimensions, entries,
authoritative bytes, index bytes, indexes, top-k, concurrent queries, queue bytes, build/rebuild
work, delta lag, checkpoints and generation leases.

**Implementation.** Admission reserves bytes before mutation. Foreground query, authoritative
mutation, index delta, rebuild and transfer use separate bounded lanes; safety/control traffic
cannot be starved. Eviction remains an authoritative cache decision; index-only eviction may cause
a miss but never pretend the entry was deleted. Tombstone ratio and fragmentation trigger a
budgeted rebuild with hysteresis/circuit breaker.

**Tests.** Boundary−1/boundary/boundary+1 for every count/byte/time limit; hot index beside cold
tenant; slow queries; rebuild storms; namespace churn; memory pressure; disk full; FD/task limits;
cancellation leaks. Assert cold admitted work progresses, process memory is explained and recovery
returns queues/owners to steady bounds.

**Exit.** No client-controlled input creates unbounded work or retained state, and overload has a
typed retryable/non-retryable result rather than timeout-only behavior.

## W11. Ship coherent Rust, Java and Python clients

**Changes.** Extend Rust HC/2, Java HC/2 and Python HC/2 with the same semantic types and methods.
Add high-level response/tool-result helpers that accept application embedding/loader callbacks but
no provider SDK dependency.

**Implementation.** Generated HC/2 messages remain internal; hand-written SDK models expose stable
types. All SDKs default to `ShadowOnly` until an admitted reuse profile is supplied. Examples show
OpenAI-compatible embeddings only as application code with environment-provided credentials; tests
use deterministic local fixture embeddings and never call external services.

**Tests.** Cross-language request/response goldens, live daemon interop, cancellation/deadline,
unknown-field compatibility, wheel/JAR/crate clean-consumer tests and retained 0.75 client matrix.
Python covers async cancellation and binary vector buffers; Java covers little-endian `FloatBuffer`
and lifecycle; Rust covers typed payload codecs.

**Exit.** The same seeded query yields identical identity, scope, ordering, outcome and proof in all
three SDKs, and old clients continue to operate without semantic capability.

## W12. Build shadow-mode quality admission and adversarial corpora

**Changes.** Add `ShadowOnly`, `ManualReuse` and `AutoReuse` profile states. Create versioned labelled
corpora of exact duplicates, paraphrases, acceptable variants, hard negatives, negation, temporal
changes, locale boundaries, tool-schema changes, retrieval updates and safety-policy changes.

**Implementation.** Shadow mode performs lookup but always calls the loader, recording only
privacy-safe candidate ID/score, expected class and comparison outcome. W0 freezes train/tune/test
separation, thresholds and estimator before active candidate results. A profile promotes only from
an untouched evaluation split and binds corpus, embedding model and reuse scope digests.

**Tests.** Require zero security/scope boundary reuse, publish confusion matrix and score
distribution, and calculate false-reuse confidence bounds. ANN recall is compared with flat exact
search separately from semantic answer quality. Threshold mutations and label leakage canaries must
turn the gate red.

**Exit.** Auto-reuse remains disabled for any profile without a reproducible receipt. Hit rate alone
cannot promote a profile; false reuse and incomplete-search behavior are primary release gates.

## W13. Measure latency, avoided calls, quality and cost honestly

**Changes.** Build a common benchmark with HydraCache flat/HNSW, exact-key HydraCache, pinned Redis
semantic cache and no semantic cache adapters. Precompute identical embeddings and separate index
latency from optional end-to-end embedding latency.

**Implementation.** Run same-host counterbalanced pairs over fixed cardinality, dimensions, scope
selectivity, hit/miss/near-threshold mix, payload sizes, top-k, clients and cluster topology. Use
open-loop offered load and retain all errors, rejections, timeouts and incomplete results. Report
goodput, p50/p95/p99/p99.9, CPU/query, bytes/vector, rebuild time, recall@k, semantic precision,
avoided loader calls and avoided input/output tokens. Currency cost is timestamped illustrative
metadata, never the stable gate.

**Tests/evidence.** Semantic equivalence and corpus admission run before timing. Include one/three
members, replication, failover, rebuild and noisy-tenant cells. Publish wins, losses, equivalent,
inconclusive and not-comparable cells; no composite score. Long-run exact-candidate tests prove
resource/lag stability.

**Exit.** Any published advantage names exact profile, topology, corpus, workload, product/tooling
SHA, host, estimator and uncertainty. A faster result with worse false reuse, recall, errors,
incompleteness or resource bounds is rejected.

## W14. Complete operations, upgrades, evidence and ship decision

**Changes.** Add management/readiness views, alerts, runbooks, rolling-upgrade/index-schema
migration, backup/restore treatment, SBOM/provenance and release evidence. The console remains
read-only for semantic data and never displays prompts/responses/vectors.

**Implementation.** Expose bounded metrics for records/indexes, bytes, readiness state, build
generation, lag, query outcomes, validation rejects, rebuild/circuit state, recall diagnostic and
quality profile status. Rolling upgrade supports old readers plus new semantic-capable nodes; an
unsupported index schema rebuilds from authority. Backup owns authoritative semantic records; ANN
checkpoints are optional accelerators and are digest-validated/rebuildable.

**Tests.** Old/new daemon and SDK matrix, interrupted rebuild across restart, checkpoint downgrade,
backup/restore followed by full rebuild, full-cluster restart, cert rotation, clean workspace,
package consumers, docs examples, dynamic canaries and evidence archive verification.

**Exit.** Exact-candidate release evidence binds product/tooling/SDK/proto/index/corpus/profile/
comparison identities and every nested artifact hash. Release notes state admitted profiles and
non-claims. Missing quality/security/distributed evidence blocks the feature rather than weakening
the gate.

## Required test matrix

| Tier | Mandatory proof |
| --- | --- |
| Unit/model | Vector validation/canonicalization, cosine oracle, ordering, scope identity, outcomes, version/TTL/tag transitions, exact lease fencing. |
| Property/fuzz | Wire/record/checkpoint decoders, arbitrary finite vectors, operation sequences, delta ordering, bounds and corrupt/truncated inputs. |
| Concurrency | Query vs replace/delete/expire, lease holder death, rebuild delta cutover, cancellation and shutdown; loom where state is bounded. |
| Cross-language | Rust/Java/Python canonical vectors, scope/fingerprint digests, HC/2 messages, error/retry mapping and live daemon interop. |
| Real process | One/three/five members, non-owner ingress, failover, partition, rebalance, rolling upgrade, restart, rebuild and namespace reuse. |
| Security | Tenant forgery, internal-route auth/replay, malicious vectors, content-free telemetry, deletion/reclamation and evidence secret scan. |
| Quality | Labelled untouched corpus, hard boundaries/negatives, flat-vs-ANN recall, shadow comparison, confidence and threshold canaries. |
| Resource | Entry/index/vector/payload/top-k/member/concurrency/queue/rebuild boundaries, noisy tenant and steady-state cleanup. |
| Performance | Same-semantics open-loop cells, exact/flat/HNSW/Redis/no-cache controls, fixed embeddings, long-run lag/resource stability. |
| Release | Compatibility, SBOM/license/advisory/MSRV, clean crates/JAR/wheel consumers, backup/restore, rollback and immutable evidence. |

## Fast, scheduled and protected gates

**Fast PR gates:** formatting, clippy, doc-check, semantic contract validation, flat oracle,
canonical/golden vectors, model/property tests, protocol compatibility, Rust/Java/Python unit tests,
bounded small-scope authority/index model, hostile input tests and dynamic canaries.

**Scheduled gates:** fuzz corpora, Miri/loom, full cross-language daemon interop, three/five-member
fault/rebalance/rebuild suites, rolling upgrade, namespace deletion, noisy-tenant/resource matrix,
quality corpus and dependency/SBOM audit.

**Protected exact-candidate gates:** admitted host, pinned corpus/profile/index/Redis identities,
same-host comparison, long-run resource/lag proof, full compatibility/package/backup/restore,
independent artifact verification and immutable release archive. Retries are append-only and cannot
replace failed quality or security evidence.

Representative commands are frozen in W0 and added to `docs/GATES.md`; at minimum they include:

```text
cargo xtask semantic-contract-check --release 0.76
cargo nextest run -p hydracache-semantic-index
cargo nextest run -p hydracache --test semantic_cache
cargo nextest run -p hydracache-server --test semantic_process
cargo test -p hydracache-client-hc2 --test semantic_interop
mvn -pl sdks/java/hydracache-client-hc2 test
python -m pytest sdks/python/hydracache-client-hc2/tests
cargo xtask release-evidence --release 0.76 --require-ship
```

## Release-level risks and mitigations

| Risk | Required mitigation |
| --- | --- |
| Similarity serves a plausible but wrong answer | Default shadow mode, hard scope, labelled hard negatives, profile-specific threshold and false-reuse gate. |
| Tenant data reaches another tenant's ANN graph | Physically/logically select index by authenticated tenant+namespace generation before search; hostile proof. |
| Index lag resurrects expired/deleted content | Versioned deltas plus mandatory authoritative final read; stale candidate becomes miss. |
| Rebuild marks ready too early | Snapshot+delta watermark parity, committed query epoch and falsifying early-ready canary. |
| HNSW improves latency by losing recall | Permanent flat differential, frozen recall floor and published misses. |
| Benchmark hides embedding cost | Separate precomputed-index latency and end-to-end embedding cohorts; never blend them. |
| Similar prompts collapse into one in-flight result | Exact canonical fingerprint only; model/loom tests with score-1.0 distinct prompts. |
| One hot tenant consumes CPU/memory | Per-tenant/index quotas and separate query/build/rebuild lanes with cold-progress gate. |
| Vector dependency expands supply-chain risk | Trait isolation, pinned source/checksum, SBOM/license/MSRV/advisory review and clean rebuild. |
| Product drifts into vector database/RAG scope | Capability manifest and release note non-goals; no document/query-language/reranker surface. |

## Deferred work

- Quantized vectors, product quantization, GPU indexes and disk-tier ANN require separate recall,
  portability, resource and recovery proposals.
- Hybrid sparse+dense search, arbitrary metadata predicates, full-text search and reranking belong to
  a retrieval product, not the 0.76 response cache.
- Durable ordered agent memory, conversation history and episodic summarization require consent,
  retention, deletion, ordering and compaction contracts.
- A distributed token bucket and priority lease queue are useful AI gateway structures but are
  independent semantics and should receive their own release plan.
- Cross-region semantic indexes and active-active ANN convergence are not inferred from existing
  multiregion metadata features.

## Final release decision

Release 0.76 is eligible only when:

- the published 0.75 distributed value plane is the sole semantic-record authority;
- public/wire/SDK types expose every miss, unavailable, overload and ambiguous outcome explicitly;
- Rust, Java and Python agree on vector bytes, identity, scope, ordering and outcomes;
- hard tenant/namespace/profile selection happens before similarity search;
- every returned hit passes authoritative version, TTL, generation and hard-scope validation;
- stale/corrupt/missing/rebuilding index state can cause only explicit unavailability or miss;
- flat exact search remains a permanent oracle and all ANN recall/resource thresholds pass;
- TTL, tags, replacement, deletion, eviction and namespace reclamation update or safely invalidate
  derived state without resurrection;
- strict distributed queries prove bounded fan-out, completeness and deterministic merge through
  owner failure, partition, rebalance and rolling upgrade;
- single-flight/generation leases use exact fingerprints only and stale holders cannot publish;
- all per-entry/index/tenant/process memory, CPU, queue, task, fan-out and rebuild limits pass at
  boundary−1/boundary/boundary+1 and under noisy tenants;
- automatic reuse is enabled only for exact admitted corpus/model/policy profiles, with zero hard
  boundary violations and frozen false-reuse confidence gates;
- shadow and active evidence publishes precision, false reuse, recall, misses, incompleteness,
  errors and resource trade-offs beside hit rate and latency;
- Redis/no-cache comparisons use pinned same-semantics identities and publish losses and
  not-comparable cells without a composite score;
- logs, metrics, traces, profiles, crash reports and public artifacts contain no prompt, response,
  vector coordinates, credentials or tenant-controlled identifiers;
- old clients/daemons, backup/restore, full restart, downgrade/rebuild and rollback gates pass;
- exact product/tooling/proto/SDK/index/dependency/corpus/profile/workload/host identities and nested
  artifact checksums are archived immutably;
- release documentation says semantic response cache, not general vector database, RAG platform,
  model correctness guarantee or universal performance advantage.

Anything less ships with semantic reuse disabled or does not ship as 0.76.
