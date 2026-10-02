# HydraCache 0.76.0 Semantic Cache and Vector Index Plan

> **At a glance**
>
> - **What:** add a bounded, multi-tenant `HydraSemanticMap<V>` and an LLM-oriented
>   `SemanticCache<V>` adapter. The authoritative response, vector, hard reuse scope, TTL, tags and
>   version live in the distributed 0.75 value plane; a partition-local ANN index is derived and
>   rebuildable. Queries perform hard security/policy filtering, bounded similarity search and an
>   authoritative final read before a response can be reused. Rust, Java and Python HC/2 clients,
>   active/shadow/previous embedding-profile migration, partition/member/global candidate budgets,
>   resumable optimize/rebuild jobs, bounded batch ingestion, scalar/SIMD distance kernels, capacity
>   admission, provenance-gated writes, ambiguity-aware reuse decisions, continuous post-promotion
>   quality circuits, bounded admission/eviction, deletion receipts, integrity scrubbing,
>   graph-specific observability and exact-candidate evidence are included.
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
- [Hazelcast VectorCollection](https://docs.hazelcast.com/hazelcast/5.7/data-structures/vector-collections)
  demonstrates named vector spaces, bounded partition/member candidate collection, configurable
  ANN construction/search, batch ingestion, explicit optimization and graph-aware operations;
- [Hazelcast's VectorCollection design](https://docs.hazelcast.com/hazelcast/5.7/data-structures/vector-search-overview)
  makes partition fan-out, backup memory, CPU/SIMD, failure headroom and graph-visit telemetry
  visible design concerns rather than hidden implementation details.

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

### Hazelcast-derived design inputs and adaptation boundary

Hazelcast is a design input, not an API-compatibility target. 0.76 adopts the operational lessons
that make a distributed ANN structure supportable, while retaining HydraCache's narrower semantic
reuse contract:

| Hazelcast pattern | 0.76 adaptation | Deliberately not copied |
| --- | --- | --- |
| Multiple named indexes share one metadata value. | One authoritative semantic record may hold a bounded `active`, `shadow` and `previous` vector set so an embedding model can migrate without an unsafe flag day. | Unbounded arbitrary vector spaces or automatic server-side re-embedding. |
| `partitionLimit`, `memberLimit` and final `limit`. | Frozen `partition_candidate_limit`, `member_candidate_limit` and `global_top_k`, with an explicit operational completeness proof. | Client-controlled string hints that can bypass admission or change an admitted auto-reuse profile. |
| `max-degree`, `ef-construction` and `efSearch`. | Versioned server-owned `Latency`, `Balanced`, `Recall` and `Exact` profiles with measured recall/resource envelopes. | Raw backend knobs in the stable public/wire contract. |
| `putAll` for ingestion. | Partition-grouped bounded `put_batch` with per-entry idempotency and typed partial outcomes. | Cross-partition all-or-nothing claims or an unbounded bulk request. |
| Explicit index optimization after churn. | Background copy-on-write optimize/rebuild jobs with checkpoints, cancellation, restart/resume and atomic validated cutover. | In-place maintenance whose interruption can make accepted records unsearchable or whose completion is required for deletion correctness. |
| Graph and operation metrics. | Visited-node, candidate-stage, truncation, tombstone, fallback, rebuild and recall-canary metrics without customer content labels. | Prompt, response, vector or tenant-controlled labels. |
| Partitioned AP vector search with backups and capacity guidance. | 0.75 remains authoritative; the derived ANN may be approximate, but every reusable hit is fenced by epoch/completeness and an authoritative validation. Admission proves `N-1 + rebuild + foreground load`. | Treating an approximate/AP candidate as authoritative or copying a fixed vendor headroom percentage without HydraCache measurements. |

This boundary also avoids inheriting current VectorCollection beta limitations such as unstable API
or rolling-upgrade semantics, only-on-heap assumptions and score values whose exact representation
may vary. HydraCache freezes its wire schema, canonical score/recheck path and upgrade behavior.

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
- No online threshold learning or silent quality-policy mutation from production feedback. Feedback
  may demote a profile or require operator review; only a new immutable profile revision can change
  threshold, margin or decision semantics and re-enable automatic reuse.
- No raw prompt or response logging, vector labels containing customer data, unbounded `top_k`,
  unbounded dimensions, unbounded index count or unbounded fan-out.
- No arbitrary per-query `ef`, graph-degree, partition/member limit or backend-specific string hint.
  A query selects only an admitted named search class, and an auto-reuse policy may pin that class.
- No promise that a new embedding can be derived from an old vector. A profile migration requires
  application-supplied new vectors through dual write or authenticated bounded backfill.
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
| Profile migration | At most one active, one shadow and one previous profile exist per semantic index family; only the admitted active profile may auto-reuse. | A shadow vector becomes reusable, or cutover occurs before coverage/quality/resource proof. |
| Search-budget integrity | Partition, member and global candidate limits come from a versioned admitted profile and every response describes coverage/truncation. | A client raises raw ANN limits, or operationally incomplete fan-out is described as complete. |
| Maintenance isolation | Optimize/rebuild creates and validates a new generation while the old generation remains queryable; cancellation/restart cannot damage authority. | In-place maintenance blocks correctness, loses accepted deltas or exposes a half-built generation. |
| Capacity under failure | Admission includes authority replicas, active index, rebuild double-buffer, journals, scratch space and one-member loss. | Steady state fits, but `N-1 + rebuild + foreground load` exceeds declared memory/CPU/disk bounds. |
| Provenance before reuse | An authenticated, admitted writer class and immutable generation-pipeline digest are stored by authority and validated before reuse. | Any holder of ordinary read credentials can poison a reusable response or forge producer identity. |
| Ambiguity-aware decision | Auto-reuse applies a server-owned immutable minimum score, top-two margin, age and validation-depth policy; clients may only make it stricter. | A client lowers the threshold/margin or a near-tie is reused without the admitted ambiguity rule. |
| Continuous safety | Active profiles retain bounded verification/feedback canaries and automatically demote on a frozen hard safety or quality circuit; promotion is never automatic. | Offline admission remains active indefinitely after measured drift or hard-boundary violation. |
| Eviction coherence | Authority owns admission/eviction and emits the same versioned removal delta as explicit deletion; retained bytes include payload, all vector slots and derived charge. | Index-only state is presented as a live authority record, or eviction resurrects through rebuild. |
| Deletion evidence | Logical non-reuse is immediate and physical reclamation across authority, live/old index generations, checkpoints and retained backups follows a declared SLO with receipts. | A deleted vector remains silently queryable/exported, or documentation promises instant physical erasure that backups cannot satisfy. |
| Runtime integrity | Bounded scrubbing compares derived IDs/versions/vector digests with authority and degrades/rebuilds on mismatch. | A corrupt live graph continues to authorize hits until restart or manual discovery. |

## Canonical public model

W0 freezes names, but semantics below are mandatory. The server accepts precomputed vectors; SDK
helpers may integrate application-provided embedding callbacks without adding a provider dependency.

```rust
pub struct SemanticIndexId {
    pub family: SemanticIndexFamilyId,
    pub embedding_profile: EmbeddingProfileId,
}

pub struct SemanticIndexFamilyId {
    pub namespace: Namespace,
    pub name: String,
}

pub struct EmbeddingProfile {
    pub model_id: String,
    pub model_revision: String,
    pub dimensions: u16,
    pub metric: SimilarityMetric, // 0.76: Cosine only
    pub canonicalization_version: u16,
}

pub enum EmbeddingProfileRole {
    Active,
    Shadow,
    Previous,
}

pub struct EmbeddingSlot {
    pub profile: EmbeddingProfileId,
    pub vector: EmbeddingVector,
    pub vector_version: u64,
}

pub struct EmbeddingSet {
    pub vectors: BoundedVec<EmbeddingSlot, 3>,
}

pub struct EmbeddingProfileSet {
    pub generation: u64,
    pub active: EmbeddingProfileId,
    pub shadow: Option<EmbeddingProfileId>,
    pub previous: Option<EmbeddingProfileId>,
}

pub enum SemanticSearchClass {
    Latency,
    Balanced,
    Recall,
    Exact,
}

pub struct SemanticDecisionPolicy {
    pub revision: u64,
    pub minimum_score: SimilarityScore,
    pub minimum_top_two_margin: SimilarityMargin,
    pub max_candidate_age: Option<Duration>,
    pub minimum_validation_candidates: NonZeroU16,
    pub allow_single_candidate: bool,
    pub require_operational_completeness: bool,
}

pub struct DistributedSearchBudget {
    pub partition_candidate_limit: NonZeroU16,
    pub member_candidate_limit: NonZeroU16,
    pub global_top_k_limit: NonZeroU16,
    pub max_fanout_members: NonZeroU16,
    pub max_candidate_bytes: NonZeroU32,
    pub deadline: Duration,
}

pub struct SearchCompleteness {
    pub expected_partitions: u32,
    pub searched_partitions: u32,
    pub stale_partitions: u32,
    pub unavailable_members: u16,
    pub partition_truncations: u32,
    pub member_truncations: u16,
    pub operationally_complete: bool,
    pub approximate: bool,
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
    pub producer_policy: Digest,
}

pub struct SemanticWriteProvenance {
    pub generation_pipeline: Digest,
    pub source_revision: Option<Digest>,
    pub provenance_schema: u16,
}

pub struct ProducerProvenance {
    pub authenticated_writer_class: WriterClassId,
    pub submitted: SemanticWriteProvenance,
}

pub struct SemanticEntry<V> {
    pub id: EntryId,
    pub embeddings: EmbeddingSet,
    pub scope: ReuseScope,
    pub provenance: SemanticWriteProvenance,
    pub value: V,
    pub tags: TagSet,
    pub ttl: TtlDirective,
}

pub struct SemanticQuery {
    pub index: SemanticIndexId,
    pub embedding: EmbeddingVector,
    pub scope: ReuseScope,
    pub top_k: NonZeroU16,
    pub requested_minimum_score: Option<SimilarityScore>,
    pub requested_minimum_margin: Option<SimilarityMargin>,
    pub search_class: SemanticSearchClass,
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
    pub score: ScoreDisclosure,
    pub version: EntryVersion,
    pub age: Duration,
    pub remaining_ttl: RemainingTtl,
    pub completeness: SearchCompleteness,
    pub proof: SemanticHitProof,
}

pub enum ScoreDisclosure {
    Hidden,
    Bucket(u8),
    Exact(SimilarityScore),
}

pub enum SemanticFeedbackVerdict {
    Accepted,
    Rejected,
    Corrected,
}

pub struct SemanticFeedback {
    pub hit_proof: OpaqueHitProofId,
    pub verdict: SemanticFeedbackVerdict,
    pub application_evaluator: Digest,
}

pub struct SemanticBatchOptions {
    pub request_id: IdempotencyKey,
    pub max_parallel_partitions: NonZeroU16,
    pub acknowledgement: AcknowledgementLevel,
}

pub enum SemanticBatchItemOutcome {
    Stored { version: EntryVersion, index_visibility: IndexVisibility },
    Rejected(SemanticRejected),
    Ambiguous { retry: RetryAdvice },
}

pub struct SemanticBatchOutcome {
    pub items: Vec<(EntryId, SemanticBatchItemOutcome)>,
    pub attempted_partitions: u16,
    pub completed_partitions: u16,
}
```

`ReuseScope` is not arbitrary metadata. Every field changes whether reuse is permissible. The
authenticated tenant and namespace generation are injected by the server and cannot be supplied or
overridden by a client payload. The exact list and optionality are versioned in HC/2 and
`docs/COMPAT.md`.

`EmbeddingSet` is bounded to three distinct profile IDs and is not a generic list. The family-level
`EmbeddingProfileSet` is one epoch-fenced authoritative metadata record, so cutover changes one
generation rather than rewriting every cached record. A normal single-profile put must contain the
current active profile; an optional shadow vector collects comparison evidence but cannot authorize
reuse; a previous vector exists only for the rollback window. Adding or replacing a vector slot
requires an expected authoritative record version so backfill cannot attach a vector to the wrong
payload. Profile roles and search classes are server configuration: a client cannot relabel a
profile or select a cheaper class than the namespace's admitted auto-reuse policy.

`ProducerProvenance.authenticated_writer_class` is derived from the authenticated principal and
cannot be supplied by `SemanticEntry`. The application supplies only `SemanticWriteProvenance` with
versioned pipeline/source digests; the server combines both after validating the allowlisted
`producer_policy`. Changing that policy changes hard reuse scope. This prevents a low-trust bulk
importer, compromised read-only client or test pipeline from placing entries into a production
auto-reuse cohort.

`SemanticDecisionPolicy` belongs to an immutable admitted profile revision. Client-requested score
or margin can only raise the effective requirement; omission selects the server values. Automatic
reuse validates at least two final candidates when available and rejects a top-two near-tie below
the admitted margin. Fewer candidates than the validation minimum reject auto-reuse unless the
immutable corpus-admitted policy explicitly enables `allow_single_candidate`; its default is false.
`ScoreDisclosure` is permission-derived: normal cache helpers default to
`Hidden` or a coarse bucket, while exact scores and diagnostic candidate lists require a separately
rate-limited diagnostic permission. Feedback references an opaque, expiring hit proof and contains
no prompt, response or free-form reason; it can trip a circuit but cannot tune or promote a policy.

### Vector canonicalization

0.76 supports finite `f32` vectors and cosine similarity only. Wire values use length-delimited
little-endian IEEE-754 bytes with an explicit dimension and canonicalization version. Cosine is the
public semantic contract; storage normalizes once and the hot distance kernel computes a dot
product over normalized vectors. The owner:

1. rejects NaN, infinity, signed non-canonical zero where prohibited, dimension mismatch and a
   zero-norm vector;
2. accumulates the norm in `f64`, normalizes deterministically and stores canonical `f32` bytes;
3. hashes profile identity plus canonical bytes, never language-local array/hash behavior;
4. returns the canonical vector digest in the mutation receipt;
5. proves Rust/Java/Python golden equality on x86_64 and aarch64.

Quantization is deferred until a separately measured proposal proves recall and memory bounds.
Changing canonicalization, dimensions, metric or model revision creates a distinct index profile;
in-place reinterpretation is forbidden.

The scalar kernel is the permanent reference. Optional x86_64/aarch64 SIMD kernels are selected by
runtime CPU feature detection and may accelerate ANN traversal only after scalar differential,
alignment, tail-length and unsupported-feature tests pass. Before global merge or response reuse,
the bounded candidate set is rescored through the canonical scalar path against the authoritative
normalized vector. Therefore CPU/vectorization rounding can change traversal and measured recall,
but cannot silently change the published score or deterministic final ordering. Unsupported CPUs
fall back to scalar without changing the wire/profile identity.

### Embedding-profile migration

An index family has exactly one active profile and may temporarily carry one shadow and one previous
profile. Profile state is authority metadata, not an ANN-local flag:

```text
Proposed -> Building -> ShadowReady -> Active -> Previous -> Retiring -> Removed
                 |             |          |          |
                 +----------> Failed <----+----------+
```

1. An operator proposes an immutable profile containing model/revision, dimensions,
   canonicalization, search class and resource/quality contract.
2. Because HydraCache does not call an embedding provider, the application either dual-writes
   active and shadow vectors for new records or performs authenticated, version-checked
   `backfill_profile_batch` calls from its retained source material. An old vector is never treated
   as sufficient input for a new model.
3. Backfill records coverage by live authoritative records and bytes, not by ANN node count. Missing,
   rejected, expired and concurrently replaced records remain explicit.
4. `ShadowReady` requires snapshot-plus-delta parity, the frozen minimum coverage and W12 quality
   receipt. It may answer diagnostic comparisons but cannot return an auto-reusable hit.
5. Cutover is an epoch-fenced metadata transaction: shadow becomes active, active becomes previous,
   query coordinators observe one committed generation, and in-flight old-epoch results fail closed.
6. `Previous` remains indexed for the frozen rollback window and receives either continued dual
   writes or an explicit no-new-writes marker. Rollback is allowed only while its lag/coverage remain
   inside contract.
7. Retirement deletes the vector slot, deltas, checkpoints and metrics state only after rollback
   expiry and an auditable reclamation receipt.

At most three canonical vectors are retained per record. Admission reserves their authority,
replica and derived-index bytes before accepting a profile transition. A failed migration leaves the
existing active profile unchanged.

### Named ANN and capacity profiles

`SemanticIndexConfig` owns backend-independent, versioned profiles. Stable clients see only names:

```rust
pub struct SemanticAnnProfile {
    pub class: SemanticSearchClass,
    pub backend_schema: u16,
    pub max_degree: u16,
    pub ef_construction: u16,
    pub ef_search: u16,
    pub budget: DistributedSearchBudget,
    pub minimum_recall_at_k: Ratio,
    pub max_index_bytes_per_vector: u32,
    pub max_query_cpu: Duration,
}
```

`Latency`, `Balanced` and `Recall` map to administrator-admitted immutable profile revisions;
`Exact` selects the flat oracle only within its cardinality/resource limit. Raw `M`, `ef`, shard
limits and backend hints never appear in normal query messages. Changing a profile creates a new
profile revision and shadow comparison; it cannot mutate the meaning of already archived evidence.

Capacity admission uses a generated worksheet and a machine-readable receipt:

```text
peak_required = authoritative_primary
              + authoritative_backups
              + active_ann_generations
              + previous_and_shadow_profiles
              + rebuild_double_buffer
              + delta_journals_and_tombstones
              + query/build scratch
              + runtime/allocator reserve
              + one-member-loss redistribution reserve
```

The planner consumes measured bytes/vector and scratch high-water marks for the exact candidate; it
does not copy a fixed vendor percentage. A configuration is rejected unless every admitted tenant
fits during `N-1 + one rebuild + frozen foreground load`, with cold-tenant progress and disk/FD/task
headroom. The receipt states which component prevents admission and suggests only bounded remedies.

### Lookup and reuse sequence

1. Authenticate and derive tenant/namespace generation.
2. Validate the fixed embedding profile, vector, `top_k`, deadline and reuse scope, including the
   producer-policy digest required for any reusable record.
3. Resolve the admitted named search class, immutable `DistributedSearchBudget` and
   `SemanticDecisionPolicy`, one active `SemanticIndexId` and its current query epoch. A query may
   request a stricter score/margin or more expensive class only if authorized; it cannot weaken the
   auto-reuse profile.
4. Each owned partition shard returns at most `partition_candidate_limit` candidates to its local
   member aggregator. The aggregator records visited nodes and truncation, deterministically merges
   its shards and returns at most `member_candidate_limit` candidates.
5. The coordinator receives at most `max_fanout_members` responses and
   `max_candidate_bytes`, merges to `global_top_k_limit`, then retains the bounded internal decision
   set; only diagnostic output is later trimmed to requested `top_k`. Merge is descending canonical
   score then stable `EntryId`; old/new migration copies are deduplicated by entry ID, payload
   version and vector version.
6. Apply the frozen availability rule and construct `SearchCompleteness`. `RequireComplete` is the
   default for auto-reuse and rejects
   missing/rebuilding/stale shards.
7. Internally retain `max(requested top_k, minimum_validation_candidates)` within the global budget,
   even when the public helper requests one result. Read that bounded validation depth from
   authoritative owners when available and
   validate version, TTL, maximum candidate age, namespace generation, complete hard scope,
   authenticated writer class, generation-pipeline provenance and payload admission.
8. Canonically rescore and order validated candidates. Return the winner only when it satisfies the
   effective minimum score, validation-depth/single-candidate rule and top-two margin; otherwise
   return explicit `BelowThreshold`, `InsufficientDecisionEvidence` or `AmbiguousNeighbourhood`
   miss. Apply permission-derived score disclosure to the response/proof.
9. Record bounded decision/provenance/circuit reason counters without logging prompt, response,
   vector or tenant-controlled text.

An ANN false positive is filtered by the score/hard scope; a stale candidate is filtered by the
authoritative read. An ANN false negative is a cache miss and may cost an LLM call, but cannot serve
the wrong entry. This asymmetry is the core safety contract.

`operationally_complete` means that all required partitions answered under the admitted finite
budget, correct epoch and lag limit. It does not claim exhaustive nearest-neighbour search: ANN
recall remains a separately measured statistical property. Budget exhaustion/truncation is exposed
in proof and metrics even when it is permitted by the admitted profile. For `Exact`, truncation or
deadline exhaustion is incomplete rather than approximate success.

### Post-promotion verification and emergency demotion

Offline corpus admission is necessary but not permanent proof. Each active profile retains an
immutable verification policy containing a bounded sample rate, minimum evidence count/window,
hard-boundary trip conditions, feedback authenticity/rate limits and soft quality/resource error
budgets. The server never calls a model: the Rust/Java/Python `lookup_or_generate` helper may, when
the application opts in, execute the normal loader for a sampled hit, return according to the
application's declared verification mode and submit only a typed `SemanticFeedback` receipt. An
application may also submit a verdict from its own evaluator using the opaque hit proof.

Feedback is accepted only from an authenticated evaluator class bound to the namespace/profile and
is deduplicated, expiring and quota-limited. It contains no free-form content and cannot modify a
threshold. One verified cross-scope/provenance/hard-boundary violation, or a preregistered sustained
false-reuse/resource/recall breach with sufficient evidence, atomically demotes `AutoReuse` to
`ShadowOnly`, opens a quality circuit and emits an alert/audit receipt. Ambiguous/untrusted feedback
marks evidence unavailable but cannot disable arbitrary tenants or promote a profile. Recovery
requires operator acknowledgement, root-cause evidence and a new or re-admitted immutable profile;
there is no automatic re-enable. The data plane also retains a fast authenticated namespace/profile
kill switch whose propagation and fail-closed behavior are release-tested.

### Bounded batch ingestion contract

`put_batch` and `backfill_profile_batch` exist for cache warming and profile migration. The server
rejects a request before allocation when entry count, encoded bytes, vector bytes, distinct
partitions or requested parallelism exceeds configuration. Accepted entries are grouped by 0.75
authority partition, preserving input identity only for response correlation; each partition uses
one bounded internal frame and one inherited authoritative bulk mutation path.

There is no cluster-wide transaction. Each entry has its own idempotency key and outcome, while the
batch receipt reports attempted/completed partitions. A transport loss may yield `Ambiguous` only
for entries whose owner acknowledgement is unknown; retry uses the same item idempotency keys.
`backfill_profile_batch` additionally requires expected payload version, target profile in
`Building`/`ShadowReady`, and vector digest. It may add or replace only that profile's slot and may
not modify payload, TTL, tags or reuse scope. SDK iterators split larger application streams into
bounded calls and apply backpressure instead of materializing an unbounded collection.

## Authoritative and derived storage layout

```text
0.75 authoritative partition
  SemanticIndexFamilyMetadata(family_id) {
    profile_set_generation, active_profile, shadow_profile?, previous_profile?,
    admitted_search_and_decision_profile_revisions, producer_policy,
    verification_policy, quality_circuit_state, cutover_epoch
  }
  SemanticRecordKey(family_id, entry_id)
  SemanticRecord {
    embedding_slots[profile_id; maximum 3] {
      canonical_vector, vector_digest, vector_version
    },
    reuse_scope, producer_provenance,
    encoded_value, tags, expires_at, entry_version,
    namespace_generation, created_at, policy_version
  }

member-local derived state
  SemanticIndexShard(index_id, value_partition_set, schema_version)
    FlatReferenceIndex | HnswIndex
    indexed_watermark[partition]
    tombstone_count
    visited_nodes/candidate_stage counters
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
`(entry_id, entry_version, vector_version)` dedup prevent double candidates. The old shard remains
queryable until the new shard has snapshot-plus-delta watermark parity and the authority epoch
commits cutover. Vector slots share payload/TTL/tags but have independent versioned index deltas;
removing a profile never removes the authoritative payload while another slot remains.

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

### Optimize/rebuild job contract

Maintenance is a first-class bounded state machine rather than a blocking `optimize()` call:

```text
Queued -> Reserving -> Snapshotting -> Building -> ReplayingDelta
       -> Validating -> CuttingOver -> CleaningOldGeneration -> Completed
             |              |               |
             +----------> Failed <----------+
             +----------> Cancelled
```

An authenticated operator submits an `OptimizeRequest` containing index family, profile, mode, CPU
budget, extra-byte budget and deadline and receives a stable `job_id`. `get_maintenance_status`,
`cancel_maintenance` and `resume_maintenance` are exposed through HC/2 administration and the
read-only management view; ordinary semantic-data credentials cannot start work. Automatic
tombstone/fragmentation triggers create the same auditable job type.

`Reserving` must obtain capacity for a second generation, delta replay and scratch before scanning.
Every phase writes a digest-protected checkpoint containing source authority epoch, source/target
generation, partition watermarks, reserved bytes and last completed unit. Restart resumes only when
those identities still match; otherwise it safely discards the target generation and restarts from
authority. Cancellation stops new work, releases the target generation and leaves the active index
unchanged. `CuttingOver` requires flat-oracle sample parity, full watermark parity, schema/profile
match and an epoch-fenced metadata commit. Old-generation cleanup is retryable and its retained bytes
remain accounted until deletion receipt. Foreground queries continue against the active generation
through all pre-cutover phases.

Maintenance never determines logical deletion: authoritative TTL/version/tombstone validation
already prevents stale reuse. Its purpose is to recover recall, latency and memory after churn.

### Authority admission, weighting and eviction

Semantic records participate in the existing authoritative cache admission/eviction mechanism, but
their weight is not payload bytes alone:

```text
semantic_weight = encoded key/value/scope/provenance/TTL/tag metadata
                + all retained canonical vector slots
                + configured per-profile derived-index charge
                + bounded journal/tombstone overhead
```

The derived charge comes from the admitted exact-candidate measurement and is reconciled with W10
actuals. Client-supplied token price, claimed model cost or arbitrary priority never affects
admission. Tenant/index quotas reserve a safety/control slice and the existing admitted frequency/
recency policy chooses authority victims within the allowed scope. A mutation reserves its full
normal-plus-backup weight before commit; adding shadow/previous vectors can be rejected without
changing the active record.

Authority eviction is a versioned owner mutation with backup acknowledgement and the same removal
delta/tombstone/final-read behavior as explicit removal. It returns no reusable stale entry. A
derived index may evict/rebuild a shard independently under pressure, but that state is reported as
`DerivedOnlyEvicted/IndexNotReady` and cannot imply authority deletion or strict completeness. A
future rebuild rehydrates only currently live authority records. Tests must cover scan/cache
pollution, unique-vector floods, large-payload/small-vector and small-payload/large-vector mixes,
hot/cold tenant competition, vector-slot migration pressure and crash/failover around victim ACK.

### Deletion, retention and live integrity proof

Vectors receive the same security classification as the source prompt/value because embeddings can
leak information. Logical deletion is immediate at the authoritative tombstone/version boundary;
all final reads reject older candidates. Physical reclamation is asynchronous and explicitly
tracked across authority primary/backups, active and retired ANN generations, delta journals,
checkpoints, local temporary files and configured backup archives:

```rust
pub struct SemanticDeletionReceipt {
    pub family: SemanticIndexFamilyId,
    pub entry_or_namespace_digest: Digest,
    pub logical_delete_version: EntryVersion,
    pub live_generation_state: PurgeState,
    pub retired_generation_state: PurgeState,
    pub checkpoint_state: PurgeState,
    pub backup_retention_state: BackupRetentionState,
    pub physical_reclamation_due_at: Timestamp,
}
```

The product publishes distinct logical-non-reuse and physical-reclamation SLOs. A retained immutable
backup is never labelled physically erased; its configured expiry or supported namespace-key
crypto-erasure is reported honestly. Receipts contain digests and states, not customer data. Export,
backup, evidence and diagnostic tools must apply the deletion/tombstone boundary and may not archive
raw vectors in public release evidence.

A separate bounded integrity scrub samples live index nodes and checkpoints, reads authority and
compares family/profile epoch, entry/vector version and vector digest. It also verifies graph edge/
node bounds and checkpoint manifests without logging coordinates. Any authority mismatch removes the
candidate from reusable service immediately, marks the exact partition/profile degraded, queues a
budgeted repair/rebuild and emits a content-free receipt. Scrub has per-tenant CPU/I/O quotas and a
full offline mode for release/incident proof; it is detection and repair, never a second authority.
Tests inject stale/wrong-version nodes, bit-flipped vectors/edges, deleted entries in old generations,
checkpoint corruption, backup retention and crashes during purge/scrub, then prove no post-delete or
post-corruption hit and eventual receipt completion within the declared SLO.

## Error and availability model

| Outcome | Meaning | Retry/reuse rule |
| --- | --- | --- |
| `Miss::NoCandidate` | Complete admitted search found no valid score. | Application may generate. |
| `Miss::BelowThreshold` | Canonically rescored winner is below the effective admitted score. | Generate; do not expose exact score without diagnostic permission. |
| `Miss::InsufficientDecisionEvidence` | Fewer authoritative candidates than the admitted decision policy requires. | Generate; only an explicitly admitted single-candidate policy may relax this. |
| `Miss::AmbiguousNeighbourhood` | Winner/runner-up margin is below the admitted ambiguity bound. | Generate or use explicit manual diagnostic flow; never auto-reuse. |
| `Miss::CandidateInvalidated` | ANN candidate was expired/replaced/deleted before final read. | May retry once within the same deadline, then generate. |
| `Miss::ShadowOnly` | Profile has no auto-reuse admission. | Generate; retain comparison receipt. |
| `Unavailable::QualityCircuitOpen` | Active profile was demoted by a frozen safety/quality rule or kill switch. | Generate; no automatic re-enable. |
| `Unavailable::IntegrityDegraded` | Scrub found authority/index mismatch for a required shard. | Never reuse until repair and readiness proof. |
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
| `crates/hydracache-core/src/semantic.rs` | Stable semantic IDs, embedding roles/sets, provenance, named search classes, decision/budget/completeness, feedback, deletion, batch/maintenance outcomes and codec-independent domain types. |
| `crates/hydracache-core/src/semantic_config.rs` | Immutable ANN/search/decision/producer/verification/admission/capacity profile revisions, validation and compatibility rules; no third-party backend type. |
| `crates/hydracache-semantic-index/src/{lib,flat,hnsw}.rs` | `SemanticIndex` trait, flat oracle and admitted HNSW implementation. No network or authoritative payload ownership. |
| `crates/hydracache-semantic-index/src/{kernel,checkpoint,maintenance}.rs` | Scalar/SIMD normalized-DOT kernels, digest-protected generation checkpoints and resumable optimize/rebuild engine. |
| `crates/hydracache/src/semantic.rs` | Embedded/local `HydraSemanticMap<V>` and `SemanticCache<V>` facade using the same domain rules. |
| `crates/hydracache-client-hc2/proto/hc2_contract.proto` | Versioned semantic put/batch/backfill/query/remove/status/feedback/deletion/await-indexed/maintenance messages, fixed hard scope, disclosure-safe proof and explicit outcomes. |
| `crates/hydracache-client-hc2/src/{client,types}.rs` | Rust HC/2 API, deadlines, retry advice and exact-fingerprint helper. |
| `crates/hydracache-server/src/{semantic,semantic_coordinator}.rs` | Authentication-derived scope/provenance, batch/admission/eviction, three-stage candidate merge, decision policy, authoritative validation and profile cutover. |
| `crates/hydracache-server/src/semantic_quality.rs` | Feedback authentication/dedup, verification sampling, quality circuit/kill switch and demotion receipts; never threshold learning. |
| `crates/hydracache-server/src/semantic_maintenance.rs` | Authorized job submission/status/cancel/resume, capacity reservation, deletion purge, integrity scrub and checkpoint orchestration. |
| `crates/hydracache-cluster-transport-axum/src/lib.rs` | Authenticated bounded batch and member-local ANN frames, candidate-stage proofs, rebuild snapshot/delta and cutover fencing. |
| `crates/hydracache-observability/src/management.rs` | Bounded semantic readiness/resource/quality and graph-efficiency counters; no content/vector labels. |
| `crates/xtask/src/semantic_capacity.rs` | Exact-candidate capacity worksheet/receipt for replicas, profiles, rebuild double-buffer, N−1 redistribution and scratch. |
| `sdks/java/hydracache-client-hc2/` | Java semantic types/client and cross-language goldens. |
| `sdks/python/hydracache-client-hc2/` | Python async semantic client, typed models, wheel generation and conformance. |
| `tests/semantic-reference/` | Independent exact cosine oracle, seeded corpora, hard negatives and cross-product differential driver. |
| `tests/semantic-process/` | Multi-daemon routing, failover, rebuild, rolling-upgrade, fairness and security tests. |
| `tests/semantic-migration/` | Dual-write/backfill, active-shadow-previous cutover/rollback and interrupted profile retirement tests. |
| `tests/semantic-security/` | Producer-policy poisoning, score-oracle probing, feedback forgery, deletion/reclamation and corrupt-live-index tests. |
| `tests/semantic-benchmark/` | Common HydraCache/Redis/Hazelcast VectorCollection/no-cache adapters with precomputed embeddings and open-loop load. |
| `docs/architecture/SEMANTIC_CACHE_076.md` | Authority/index/query/rebuild/failure ADR-level architecture. |
| `docs/testing/semantic/0.76/` | Frozen profiles, corpora, statistics, attempts, manifests and exact-candidate receipts. |
| `docs/security/SEMANTIC_CACHE_076.md` | Threat model, privacy, deletion, tenant isolation, side-channel and operator guidance. |
| `docs/operations/SEMANTIC_CACHE_076.md` | Admission/eviction, quality circuit, emergency disable, profile rollback, maintenance, deletion SLO, scrub and capacity runbooks. |

W0 replaces expected paths with exact post-0.75 paths before implementation. A move cannot remove
the semantic owner, tests or release gate.

### Per-work-item change and verification map

| Work | Primary implementation locations | First executable proof |
| --- | --- | --- |
| W0 | `docs/architecture/SEMANTIC_CACHE_076.md`, `docs/security/SEMANTIC_CACHE_076.md`, `docs/testing/semantic/0.76/`, `docs/GATES.md` | Contract checker rejects every deliberately mutated profile/bound/identity. |
| W1 | `hydracache-core::semantic{,_config}`, HC/2 proto/types, embedded facade skeleton | API/proto snapshots plus old-client decode, stricter-only decision and no-raw-hint compile tests. |
| W2 | core canonical codec and `semantic-index::kernel` | Cross-language byte/digest goldens and scalar/SIMD differential. |
| W3 | 0.75 value-plane record codecs/owner mutations, server batch/backfill/admission/eviction handlers | Authority/provenance/eviction reference model, per-item retry/ambiguity and index-deletion rebuild proof. |
| W4 | `semantic-index::flat` and generation interface | Independent exact oracle, deterministic ordering and exact no-truncation proof. |
| W5 | `semantic-index::hnsw`, kernel adapter and dependency manifest | Flat differential/recall/resource matrix for every named class and dedup mode. |
| W6 | delta journal/checkpoint plus `server::semantic_maintenance` | Crash/cancel/resume at every phase, deletion receipt and corrupt-live-index scrub with unchanged authority. |
| W7 | `server::semantic_coordinator` and cluster semantic frames | Three/five-member candidate-boundary, incompleteness and deterministic-merge suite. |
| W8 | embedded/client semantic-cache facade and 0.75 lease integration | Exact-only loader-collapse model under holder death and profile cutover. |
| W9 | authentication/authorization hooks, producer/diagnostic/feedback policies, deletion path and threat-model tests | Cross-tenant/profile/provenance/score-oracle/feedback/maintenance hostile suite, erasure proof and secret scan. |
| W10 | semantic admission/eviction/config, observability and `xtask::semantic_capacity` | Pollution/eviction and boundary matrix plus predicted/observed normal, N−1, rebuild and combined receipt. |
| W11 | Rust, Java and Python HC/2 clients/examples/packages | Live daemon cross-language batch/profile/job interop and clean consumers. |
| W12 | `semantic-reference`, `semantic-migration` and `server::semantic_quality` | Untouched shadow receipt, score/margin calibration, active verification/demotion, promotion rejection canaries and rollback rehearsal. |
| W13 | `semantic-benchmark` Hydra/Redis/Hazelcast/no-cache harness/adapters | Same-semantics lower-layer and end-to-end counterbalanced cells with stage/resource/quality attribution. |
| W14 | Management/readiness, deletion/scrub/quality runbooks, upgrade/backup/release tooling | Full old/new matrix, emergency demotion, reclamation SLO, restore/rebuild, evidence verification and final ship gate. |

### Safe implementation and enablement order

1. Land W0/W1 schema and capability negotiation with the feature unavailable; old clients and
   daemons remain the control.
2. Land W2/W3 authority and canonical records without ANN serving. Verify delete/rebuild and batch
   idempotency before retaining production-like data.
3. Land W4 exact local mode, then W5 ANN behind `ShadowOnly`; the flat oracle remains selectable for
   every later test.
4. Land W6/W7 distributed delta, maintenance and coordinator paths in diagnostic mode. Prove
   completeness/failover before any hit can bypass the loader.
5. Land W8-W10 leases, security, migration roles and capacity admission. Exercise active/shadow/
   previous and N−1+rebuild while auto-reuse is still disabled.
6. Land W11 clients, then execute W12 untouched shadow admission. Only a specific immutable profile
   revision may transition to `AutoReuse`; there is no cluster-wide enable switch without profile
   receipts.
7. Run W13 exact-candidate comparisons and W14 operational/upgrade/backup gates. Packaging may expose
   manual/shadow APIs if admitted, but failed active gates leave automatic reuse disabled.

## W0. Freeze scope, identities, claims and evidence contracts

**Changes.** Create the architecture decision, capability manifest, scenario matrix, quality
contract, statistics plan, threat model and release-evidence registry before product mutation.
Freeze the published 0.75 source/tag/artifacts and exact Redis, Hazelcast and no-cache comparison
identities/configurations.
Record Hazelcast 5.7 VectorCollection only as a cited design input: named spaces, staged candidate
limits, ANN knobs, bulk ingestion, maintenance, backups/capacity and visited-node telemetry. There is
no Hazelcast wire/API compatibility claim. Freeze an optional but first-class Hazelcast comparison
identity, Enterprise license provenance and legal artifact-retention boundary; if the pinned runtime
is unavailable, the cells are `not-run/not-comparable` and no Hazelcast advantage claim is allowed.

**Implementation.** Record supported cosine-only profiles, maximum dimensions/top-k/members,
local versus member modes, storage profile, payload limit, default `ShadowOnly`, availability
semantics, corpus provenance and every public non-claim. Freeze provider-independent precomputed
vectors so repeated measurements never call a live embedding API. Define D0 baseline → D1 flat
oracle → D2 ANN candidate → D3 shadow → D4 active/exact-candidate promotion states.
Also freeze: maximum batch entries/bytes/partitions; maximum three vector slots; profile transition
and rollback windows; exact `Latency/Balanced/Recall/Exact` revisions; partition/member/global
candidate budgets; maintenance CPU/extra-byte/deadline limits; scalar/SIMD identity; tombstone and
fragmentation triggers; capacity formula inputs; N−1 failure topology; every graph-specific metric
name/unit/cardinality; writer/provenance policy; score/margin/age/validation-depth decision policy;
score disclosure and diagnostic rate limit; authority admission/eviction weight; active verification
and feedback circuit; emergency kill-switch propagation SLO; logical/physical deletion SLO and backup
retention; scrub sampling/resource budget; and the distinction between operational completeness and
ANN recall.

**Tests and evidence.** Add `cargo xtask semantic-contract-check --release 0.76`, release-scoped
dynamic canaries and a coverage manifest. Negative fixtures must reject a changed embedding model,
dimension, metric, scope digest, corpus label, threshold, index/search/capacity profile, batch bound,
maintenance budget, SIMD kernel identity or comparison cell. Mutation canaries loosen each bound,
promote a shadow profile early, omit rebuild double-buffer, hide a missing partition and replace a
failed attempt; lower score/margin from the client, forge provenance/feedback, suppress quality
demotion, underweight vectors, claim backup erasure or disable scrub; every canary must turn an
independent gate red.

**Exit.** No implementation starts until the contract distinguishes semantic response caching,
RAG retrieval, provider prompt caching and generic vector storage, and until every later numerical
claim has a frozen estimator and practical threshold.

## W1. Define the stable semantic API and explicit outcomes

**Changes.** Add `SemanticIndexFamilyId`, `SemanticIndexId`, `EmbeddingProfile`, `EmbeddingVector`,
`ReuseScope`, `EmbeddingProfileRole/Set`, `SemanticSearchClass`, `DistributedSearchBudget`,
`SemanticDecisionPolicy`, `SemanticWriteProvenance`, `ProducerProvenance`, `SearchCompleteness`,
`ScoreDisclosure`, `SemanticFeedback`, `SemanticDeletionReceipt`, `SemanticEntry`, `SemanticQuery`,
batch item/outcome,
maintenance job, `SemanticLookup`, `SemanticHitProof`, availability and status types to core. Add
local/embedded and HC/2 facades without exposing index-library types.

**Implementation.** Use builders/validated constructors so invalid dimension, NaN/infinity,
zero-norm, oversized top-k and inconsistent TTL never cross the API boundary. Keep `put`, `query`,
`put_batch`, `backfill_profile_batch`, `remove`, `invalidate_tag`, `await_indexed`, `status`,
`submit_feedback`, `get_deletion_receipt`, `start_maintenance`, `get_maintenance_status`,
`cancel_maintenance` and `resume_maintenance` distinct. Data write/read, exact diagnostic,
feedback/evaluator and maintenance authorization are distinct. Normal queries carry a named search
class, not raw ANN knobs; the server returns the resolved immutable search/decision revisions and
completeness proof. Client score/margin requests are optional stricter lower bounds, never policy
replacement. No boolean result may conflate below-score, ambiguous margin, quality circuit,
integrity degradation, miss, incomplete search, truncation, overload, uncertain authority, partial
batch, cancelled maintenance or invalid input.

Define stable HC/2 messages in additive field ranges. Batch responses correlate by client item
ordinal plus `EntryId`, never by unordered map iteration. Maintenance status is paged/bounded and
contains identifiers, counters and digests but no prompt/value/vector. Unknown search class/profile
revision is `Rejected::UnsupportedProfile`, not a silent default.

**Tests.** Unit/property tests cover every constructor and outcome. SemVer snapshots, rustdoc
examples and compile-fail tests prevent accidental generic/index leakage. Retained 0.75 clients
must ignore the new capability and remain byte compatible. Proto golden tests cover old decoders,
unknown enum values, reordered batch replies, maximum proof size and job-state transitions. An API
negative test attempts to submit `efSearch`, `M` and raw partition limits and must find no stable
field through which to do so. Additional negative tests lower score/margin, request exact score with
ordinary cache credentials, submit free-form feedback and treat a pending physical purge as erased.

**Exit.** Rust signatures, Java/Python mappings, retry advice and unsupported operations are frozen
before HC/2 field numbers or product storage are added.

## W2. Canonicalize exact request identity and hard reuse scope

**Changes.** Define a versioned canonical exact request fingerprint and fixed hard scope. The exact
fingerprint covers normalized application-owned request bytes plus every field whose change can
change a safe answer; the semantic scope controls candidate eligibility.

**Implementation.** Include tenant/namespace generation, generation model/revision, prompt-template
digest, system-prompt digest, tool-schema digest, retrieval-snapshot digest, safety-policy digest,
decoding profile, application scope and producer-policy digest. Do not infer these from prompt text.
Encode with a stable length-framed codec and domain-separated digest. Tenant and authenticated
writer class come only from authentication; application pipeline/source digests are separately
validated provenance and cannot substitute for writer identity.

Embedding identity separately includes model/revision, dimensions, canonicalization, public cosine
metric and vector slot version. Normalize once in `f64`, persist canonical normalized `f32`, and use
DOT internally. Add a `DistanceKernel` interface in the semantic-index crate with scalar reference,
x86_64 and aarch64 implementations behind runtime feature detection. ANN traversal may use the fast
kernel; the bounded final candidates are rescored by the canonical scalar routine before merge and
authoritative validation. Kernel selection is telemetry, never part of cache identity.

**Tests.** Rust/Java/Python golden vectors cover empty/binary/Unicode/max-size fields, omitted versus
empty optionals, reordered input maps, old/new codec versions and tenant forgery. Property tests
prove any semantic field change changes the appropriate identity. Collision fixtures and unknown
versions fail loudly. Kernel differential tests cover random/adversarial vectors, unaligned buffers,
dimension tails around SIMD widths, subnormal/signed-zero policy, unsupported CPU fallback and
x86_64/aarch64. Candidate rescoring/order must match scalar goldens even when ANN traversal differs;
recall changes are charged to W12 rather than hidden as score equivalence.

Decision-policy goldens prove effective client score/margin equals the stricter server/requested
value, a lower request is rejected or ignored fail-closed, producer-policy changes split eligibility,
and score disclosure never changes ranking or the decision itself.

**Exit.** One canonical implementation feeds storage, routing, single-flight and receipts; SDKs do
not construct language-specific hashes.

## W3. Store authoritative semantic records in the distributed IMap plane

**Changes.** Add versioned `SemanticRecord` encoding and keys to the 0.75 backend. The record owns
the bounded active/shadow/previous vector slots, hard scope, payload, tags, TTL, namespace
generation, server-derived producer provenance and policy version. Add partition-grouped semantic
batch/backfill mutations plus weighted admission/eviction using the 0.75 authoritative
bulk/idempotency/cache-policy primitives.

**Implementation.** Route mutations through one owner-authoritative operation and inherited backup
acknowledgement. Emit an index delta only after the authoritative commit point. `put` returns record
version, vector digest and `Pending/Visible/Unavailable` index visibility. Delete, expiry, tag
invalidation, replacement and namespace deletion produce versioned removal deltas/tombstones.

Each vector slot has profile ID, vector digest and monotonically increasing `vector_version`;
payload changes increment `entry_version`. Adding a shadow slot requires expected `entry_version`
and does not change payload/TTL. Profile roles live only in `SemanticIndexFamilyMetadata`; cutover
atomically advances its generation/epoch under inherited authority and backup acknowledgement, so
records are not mass-rewritten. Coordinators and deltas bind that metadata generation. Batch
admission validates and reserves the whole encoded request, groups items by authority partition,
then returns per-item `Stored`, `Rejected` or `Ambiguous`; it never claims cross-partition atomicity.
Server and internal frames cap item count,
encoded/vector bytes, distinct partitions and parallel owners.

The authenticated writer class is captured by the server; application pipeline/source digests are
admitted against family producer policy and included in hard scope. Weight reserves payload,
metadata, every vector slot, configured derived charge and replicas before commit. Authority
admission or eviction uses existing bounded policy hooks; an eviction emits the same removal delta
as delete and is acknowledged before a stale payload can be returned. Claimed provider cost never
influences victim selection.

**Tests.** Reference-model sequences cover put/replace/remove/expire/tag invalidate/namespace reuse,
add/replace/remove shadow slot, cutover/rollback/retire, duplicate requests and owner failure
before/after ACK. Batch model tests compare every interleaving with individual operations, including
one invalid item, partial owner loss, retry with identical/different idempotency key and concurrent
payload replacement during backfill. Cross-surface tests ensure semantic records cannot be corrupted
by ordinary map APIs or exposed through RESP as an undocumented encoding. Add write-principal/
pipeline-policy matrix, poisoned low-trust writer, weight under/overflow, scan pollution, hot/cold
victim, shadow-slot admission rejection and crash/failover before/after eviction ACK.

**Exit.** Deleting every derived index loses no authoritative data, and rebuilding records produces
the same live record digest/cardinality.

## W4. Implement the bounded exact flat index as oracle and small-index mode

**Changes.** Introduce `SemanticIndex` with `insert`, `remove`, `search`, `snapshot`, `restore`,
`watermark`, `stats`, `begin_generation`, `validate_generation`, `activate_generation` and `clear`.
Implement `FlatReferenceIndex` and the scalar normalized-DOT kernel first.

**Implementation.** Keep vectors in canonical contiguous storage with explicit byte accounting.
Search computes cosine as scalar DOT over normalized vectors in a deterministic reference path,
orders by score then `EntryId`, applies scope at index selection and enforces candidate budget,
top-k/deadline/cancellation. Its stats include examined vectors and stage truncation. Small
namespaces may permanently use flat mode below a frozen cardinality threshold. `Exact` refuses a
query that cannot complete within admitted cardinality/CPU bounds instead of returning partial
results labelled exact.

**Tests.** Compare against an independent scalar `f64` oracle, including ties, near-threshold
values, extreme finite components, cancellation and boundary cardinalities. Fuzz vector decoder and
query/budget limits. Compare one large batch with the equivalent ordered single inserts. Miri covers
unsafe-free storage code; no SIMD/unsafe is admitted in the oracle. Generation tests prove failed or
cancelled builds cannot replace the active flat generation.

**Exit.** The flat index is correct, bounded and retained permanently as the ANN differential
oracle, rebuild fallback and deterministic test implementation.

## W5. Add an admitted partition-local HNSW index

**Changes.** Select or implement an HNSW backend only after a dependency/SBOM/MSRV/license/security
review. Keep it behind `SemanticIndex`; no third-party type reaches public or wire APIs.

**Implementation.** Freeze `M`, `ef_construction`, query `ef`, allocation/accounting behavior,
delete/tombstone policy, checkpoint schema and rebuild trigger per profile. The authoritative vector
is never quantized in 0.76. HNSW background work uses bounded CPU, memory, tasks and queues and is
cancellable during drain/rebuild/shutdown.

Map backend knobs only from immutable `Latency/Balanced/Recall` revisions. Identical-vector
deduplication is an optional internal proposal: records remain distinct, and a shared graph node must
expand to a bounded deterministic list of `(EntryId, entry_version, vector_version)` without hiding
top-k candidates. It ships enabled only if duplicate-heavy corpus tests prove recall/order and its
indirection/accounting beat ordinary nodes. Otherwise dedup remains disabled with an evidence
receipt. Runtime SIMD accelerates backend distance calls only through W2's reviewed kernel boundary.

**Tests.** Differential every seeded operation trace against flat top-k; measure recall@k against
the exact oracle; exercise insertion order permutations, delete/reinsert, duplicate vectors, ties,
checkpoint corruption and repeated rebuild. Fuzz checkpoints and operations. Dependency negative
canary rejects a backend that cannot report/account retained bytes or bound construction work.
Run every named profile over identical corpora, scalar/SIMD modes and low/high duplicate ratios;
verify the resolved config digest, visited nodes, candidate counts and retained bytes. Dedup tests
cover more IDs sharing a vector than every stage limit, deletion of one/all aliases and deterministic
expansion so a plausible candidate cannot disappear solely because its vector was shared.

**Exit.** Recall, build cost, memory/vector and query tail meet the W0-frozen profile limits without
semantic/security divergence; otherwise flat mode ships alone and no scale claim is made.

## W6. Couple TTL, tags, replacement and deletion to index freshness

**Changes.** Add a versioned bounded index-delta journal per authoritative partition and a
watermark/lag model. Integrate every 0.75 removal cause and namespace reclamation. Implement the
copy-on-write maintenance job state machine, deletion receipt/purge tracker and bounded live
integrity scrub in `semantic_maintenance.rs`.

**Implementation.** Deltas contain index/profile ID, entry ID, authoritative version, operation,
vector digest and required index data but no response payload. Apply is idempotent and ignores stale
versions. Candidate final reads protect correctness while lag affects completeness/readiness.
Compact journal only after all required index owners/checkpoints pass the safe watermark.

Tombstone ratio, fragmentation, degraded recall canary or an operator request may enqueue
maintenance, but never perform unaccounted work inline with mutation. `Reserving` obtains target
generation, replay and scratch capacity from W10; snapshot and delta replay persist bounded
phase/unit checkpoints; validation compares cardinality/digest/watermark plus a frozen exact sample;
cutover commits one query epoch; old cleanup remains charged until a deletion receipt. A restart
resumes only an identity-matching job, cancellation leaves the active generation untouched, and
repeated failure opens a circuit rather than rebuilding forever. Deletes remain semantically final
through authority validation even while physical graph nodes await cleanup.

Logical deletion creates a receipt rooted in authoritative delete version; purge workers advance
live/retired generation, checkpoint and backup-retention states under bounded retry. The scrub
samples graph nodes/checkpoints, compares profile epoch plus entry/vector version/digest with
authority and immediately degrades the affected shard on mismatch. Repair uses the normal reserved
maintenance path; scrub cannot rewrite authority or bypass admission.

**Tests.** Delay/reorder/duplicate/drop deltas; expire during query; replace vector and scope;
invalidate tag during rebuild; delete/recreate namespace; crash around checkpoint/journal compaction.
Assert no old response is returned, lag is visible, journals/tombstones remain bounded and eventual
rebuild digests equal a clean rebuild. Kill the process in every maintenance phase, resume on the
same/different owner, cancel before and during cutover, exhaust reserved disk/memory and mutate the
source authority epoch. Assert the old generation stays usable before cutover, half-built state is
never selected, reservations are released exactly once and maintenance status remains bounded.
Inject wrong-version/digest nodes, edge corruption, deleted entries in active/old generations and
purge/checkpoint crashes; verify immediate non-reuse, exact degraded scope, bounded repair and
logical/physical/backup receipt transitions within their separate SLOs.

**Exit.** Every authoritative lifecycle cause has a tested derived-index consequence, deletion has
honest logical/physical/backup evidence, runtime corruption is detected within the frozen scrub
coverage/SLO, and stale/corrupt index state can cause only explicit unavailability/miss.

## W7. Implement distributed fan-out, deterministic merge and failover

**Changes.** Add authenticated internal semantic-query frames and a server coordinator. Member-local
aggregators search all locally owned index shards and return bounded candidates plus completeness,
epoch, watermark, lag, visited-node and stage-truncation proofs. Encode three explicit limits:
partition candidates, member candidates and coordinator global top-k.

**Implementation.** Fan out once per admitted live member, not once per value partition. Bound
member count, candidate bytes, in-flight queries and deadline. Each shard truncates only after
deterministic local ordering to `partition_candidate_limit`; each member merges/rescores its shards
and truncates to `member_candidate_limit`; the coordinator enforces `max_candidate_bytes` and
`global_top_k_limit`. Merge canonical scalar score then `EntryId`, dedup old/new owners during
rebalance, enforce query/profile/cutover epoch and validate final candidates through 0.75
authoritative routing. `RequireComplete` rejects any missing/incompatible shard. A proof distinguishes
normal admitted truncation from missing/stale partitions; neither is silently described as exact.

Search budgets are resolved from server configuration. `top_k` may be lower than the profile maximum
but may not raise any stage limit. `BestEffortDiagnostic` may report partial candidates and exact
missing partitions but cannot feed `SemanticCache` auto-reuse. Backpressure/cancellation is
propagated to shard tasks so a timed-out coordinator does not leave unbounded CPU work.

**Tests.** Three/five-node real-process suites cover non-owner ingress, owner failure, backup
promotion, concurrent rebalance, stale owner, network partition, duplicate responses, slow member,
deadline/cancel and rolling upgrade. A canary drops one shard and must make strict auto-reuse fail
closed rather than improve apparent latency. Boundary tests place the true nearest candidate at
positions immediately below/above partition and member cutoffs, vary partition ownership, and prove
deterministic merge. Property tests verify candidate bytes never exceed the profile, cancellation
drains tasks, and `operationally_complete` is true iff every required shard/epoch/watermark response
is present. Exact mode must reject any truncation.

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

Profile cutover is included in the exact fingerprint/recheck receipt. A holder that acquired under
an old active profile must re-query under the committed active profile before publication; it may
store application-supplied old/new vectors according to the dual-write policy but cannot promote a
shadow result. Batch warming never creates generation leases and therefore cannot accidentally
coalesce unrelated application work.

After promotion, the helper applies the immutable verification sampler. A sampled hit may run the
loader under the exact lease to avoid duplicate verification work, then submit an opaque typed
feedback receipt according to application policy. Verification output never overwrites the cached
value implicitly and cannot change score/margin. If the server quality circuit or emergency kill
switch opens between lookup and return/publication, the helper fails closed and follows the declared
generate/fallback path.

**Tests.** Hundreds of identical requests collapse as configured; paraphrases never share the
in-flight result; leader/holder/client death, timeout, cancellation, lease expiry and ambiguous put
preserve fencing. Loom/model tests cover acquire/recheck/publish/release interleavings. Add sampled
verification races, duplicate feedback, circuit opening during loader, evaluator cancellation and
kill-switch propagation; no sampled path may create a semantic rather than exact single-flight key.

**Exit.** No loader result can be published by a stale generation and no non-identical fingerprint
is coalesced, even when its vector score is 1.0.

## W9. Enforce tenant isolation, privacy and deletion

**Changes.** Define physical logical index selection by authenticated tenant, namespace generation
and embedding profile. Add a threat model for content leakage, timing/cardinality side channels,
malicious vectors, cache poisoning, score-oracle probing, forged provenance/feedback, filter forgery,
batch amplification, migration-role forgery, maintenance abuse and deletion obligations.

**Implementation.** Never search a cross-tenant graph and filter afterward. Do not log prompt,
response, vector coordinates or tenant-controlled scope fields. Raw prompt retention is off by
default; optional retention is a separate encrypted payload field with explicit policy. Namespace
delete fences new operations, removes authoritative records, drains indexes/checkpoints and proves
reclamation before name reuse.

Classify canonical vectors, vector digests where linkable, candidate IDs and hit proofs at least as
sensitive as their source namespace. Read-only principals cannot write; write principals are mapped
to bounded writer classes, and family producer policy decides which classes/pipeline revisions may
be reused. High-level cache lookups hide exact score and other candidates by default. Exact scores,
vectors and diagnostic candidate lists require separate rate-limited permission and audit receipt;
threshold probing receives coarse stable outcomes and normal abuse controls. Feedback requires an
evaluator class, opaque live hit proof, namespace/profile match, dedup and quota and cannot carry
free-form content or affect another tenant.

`backfill_profile_batch` is a separate permission scoped to tenant, namespace generation and target
profile; it requires expected payload version and cannot update value/scope/TTL/tags. Profile
propose/cutover/rollback/retire and maintenance start/cancel/resume require administrative
permissions plus audit receipts. Candidate proofs expose aggregate counts and opaque IDs only to
authorized callers. Capacity/profile names are allowlisted configuration values, not attacker-
controlled metric labels.

Document immediate logical non-reuse separately from physical removal across primary/backups,
retired generations, checkpoints and backup archives. Where per-namespace encryption exists,
supported crypto-erasure may close retained-backup state; otherwise the receipt reports configured
backup expiry and must not claim erasure. Name reuse waits for the required logical and physical
scope defined by policy.

**Tests.** Hostile HC/2/internal frames forge tenant/generation/profile, oversize vectors, NaN/
infinity, decompression/codec bombs and replay epochs. Timing/cardinality tests verify bounded
coarse diagnostics. Add hostile batches with mixed tenants, forged expected version, repeated item
IDs, maximum partitions and cancellation; unauthorized profile cutover/maintenance; stale rollback;
delete during backfill/rebuild; low-trust producer insertion near a popular query; exact-score
enumeration; forged/duplicated feedback and cross-tenant circuit attempts. Scan primary/backup test
images, active/retired generations, checkpoints, temporary files, logs, traces, maintenance/deletion
receipts, crash artifacts and evidence packages according to retention state.

**Exit.** Security review and real-process isolation gates prove no cross-tenant candidate access,
not merely no cross-tenant response after application filtering.

## W10. Bound memory, CPU, queues, fan-out and noisy tenants

**Changes.** Add `SemanticIndexConfig` and per-tenant/index accounting: dimensions, entries,
authoritative bytes, index bytes, indexes, top-k, concurrent queries, queue bytes, build/rebuild
work, delta lag, checkpoints, profile slots, batch bytes/partitions, candidate-stage bytes,
maintenance/scrub/purge reservations, feedback/diagnostic rates, SIMD scratch and generation leases.
Add weighted semantic admission/eviction, `xtask semantic-capacity-plan` and a machine-readable
admission receipt.

**Implementation.** Admission reserves bytes before mutation. Foreground query, authoritative
mutation, index delta, rebuild and transfer use separate bounded lanes; safety/control traffic
cannot be starved. Eviction remains an authoritative cache decision; index-only eviction may cause
a miss but never pretend the entry was deleted. Tombstone ratio and fragmentation trigger a
budgeted rebuild with hysteresis/circuit breaker.

Compute record weight from actual encoded authority bytes, all vector slots, replica multiplier and
the measured configured derived charge. Reserve before commit and reconcile after index mutation;
large discrepancy stops further admission for that family. Reuse existing bounded cache policy for
victim selection, excluding client-claimed provider/token cost. Eviction, purge, scrub, verification
feedback and exact diagnostics have separate queues/quotas; none may starve safety/control or cold
tenants. A quality or integrity circuit sheds auto-reuse immediately without deleting authority.

Account authority primary/backups, every active/shadow/previous ANN generation, journals/tombstones,
checkpoint/disk bytes, query/build scratch, target rebuild generation and redistribution after one
member loss. Reserve before profile proposal, batch mutation or maintenance. The planner uses
measured exact-candidate bytes/vector and high-water marks, outputs normal/N−1/N−1+rebuild envelopes
per member and cluster, and rejects configurations that cross memory, disk, CPU, task or FD budgets.
Admission cannot borrow the safety/control reserve. Reconciliation compares reservations with
actual allocator/RSS/disk observations and opens a bounded discrepancy alert.

**Tests.** Boundary−1/boundary/boundary+1 for every count/byte/time limit; hot index beside cold
tenant; slow queries; rebuild storms; namespace churn; memory pressure; disk full; FD/task limits;
cancellation leaks. Assert cold admitted work progresses, process memory is explained and recovery
returns queues/owners to steady bounds. Run normal, rebuild, one-member-loss and combined
`N-1 + rebuild + foreground load` scenarios, including active+shadow+previous profile maximum and
largest legal batch. Mutation tests omit each capacity component and must cause the planner/gate to
underestimate then fail. Compare reserved, component-measured and process-level high-water values
within a frozen reconciliation tolerance. Add unique-vector pollution, adversarial weights, hot
producer versus cold tenant, eviction during query/rebuild/cutover, feedback flood, diagnostic score
scan and scrub/purge storms; prove bounded work, coherent tombstones and cold/safety progress.

**Exit.** No client-controlled input creates unbounded work or retained state, and overload has a
typed retryable/non-retryable result rather than timeout-only behavior.

## W11. Ship coherent Rust, Java and Python clients

**Changes.** Extend Rust HC/2, Java HC/2 and Python HC/2 with the same semantic types and methods.
Add high-level response/tool-result helpers that accept application embedding/loader callbacks but
no provider SDK dependency. Add bounded batch streaming/backpressure, profile migration inspection
typed verification/feedback, deletion receipt inspection and authorized maintenance clients without
exposing HNSW-specific types.

**Implementation.** Generated HC/2 messages remain internal; hand-written SDK models expose stable
types. All SDKs default to `ShadowOnly` until an admitted reuse profile is supplied. Examples show
OpenAI-compatible embeddings only as application code with environment-provided credentials; tests
use deterministic local fixture embeddings and never call external services.

Expose named `SemanticSearchClass` and typed completeness/profile proofs. SDK batch iterators chunk
by both item count and encoded bytes, retain stable per-item idempotency keys across retry, stop on
caller cancellation and never retry `Ambiguous` with a new key. Migration helpers accept separate
application callbacks for active/shadow embeddings and make external provider cost/concurrency
visible; they do not synthesize one embedding from another. Administrative APIs expose status and
bounded progress by default; destructive cutover/retire calls remain explicit and are not hidden in
convenience helpers.

Cache helpers expose the effective decision-policy revision and typed miss reason but hide exact
score by default. Exact-score/vector/candidate diagnostics require an explicit diagnostic client and
permission. Verification callbacks are application-owned, use the existing exact lease and submit
only typed feedback bound to an opaque proof; provider calls/cost remain visible to the application.
SDKs expose quality-circuit/kill-switch outcomes and logical-versus-physical deletion status without
silently retrying into weaker semantics.

**Tests.** Cross-language request/response goldens, live daemon interop, cancellation/deadline,
unknown-field compatibility, wheel/JAR/crate clean-consumer tests and retained 0.75 client matrix.
Python covers async cancellation and binary vector buffers; Java covers little-endian `FloatBuffer`
and lifecycle; Rust covers typed payload codecs. Cross-language batch tests vary chunk boundaries,
partial/ambiguous responses and retry; migration tests dual-write, backfill and observe cutover;
maintenance tests enforce authorization and job-state mapping. Add stricter-only score/margin,
score-disclosure permission, verification sampling, feedback dedup/expiry, circuit demotion and
deletion receipt goldens. No ordinary SDK method may expose raw `ef`, degree, partition/member
candidate limits, backend string hints or an exact score oracle.

**Exit.** The same seeded query yields identical identity, scope, ordering, outcome and proof in all
three SDKs, and old clients continue to operate without semantic capability.

## W12. Build shadow-mode quality admission and adversarial corpora

**Changes.** Add `ShadowOnly`, `ManualReuse` and `AutoReuse` profile states. Create versioned labelled
corpora of exact duplicates, paraphrases, acceptable variants, hard negatives, negation, temporal
changes, locale boundaries, tool-schema changes, retrieval updates and safety-policy changes. Add a
paired active-versus-shadow migration corpus, duplicate-heavy ANN corpus, poisoning/provenance cases,
near-tie ambiguity cases and a post-promotion drift/feedback campaign.

**Implementation.** Shadow mode performs lookup but always calls the loader, recording only
privacy-safe candidate ID/score, expected class and comparison outcome. W0 freezes train/tune/test
separation, minimum score, top-two margin, maximum age, validation depth, thresholds and estimator
before active candidate results. A profile promotes only from an untouched evaluation split and
binds corpus, embedding model, producer policy, decision policy and reuse scope digests. Auto-reuse
validates enough final candidates to evaluate ambiguity; a high winner score is insufficient when
the admitted top-two margin fails.

For migration, shadow queries run beside active under identical request/scope and frozen search
class. Receipts report authoritative coverage, eligible query coverage, recall@k, score/decision
disagreement, false reuse, additional bytes/CPU and backfill/provider cost. Promotion requires the
minimum authoritative-vector coverage, zero hard-boundary reuse, admitted false-reuse bound,
resource envelope and rollback-ready previous profile. Cutover does not tune thresholds. Previous
profile retirement requires a post-cutover observation window and successful rollback rehearsal.
ANN dedup, SIMD and search-class revisions each get a separate paired cell so combined changes do
not hide which mechanism changed recall or ordering.

After promotion, run a frozen bounded verification sample and accept authenticated typed application
feedback. Report evaluator coverage, accepted/rejected/corrected counts, dedup/replay rejection and
confidence separately from offline corpus evidence. Feedback never retrains/tunes. A verified hard
boundary/provenance failure or preregistered sustained quality/resource breach opens the profile
circuit and demotes it to `ShadowOnly`; reopening requires operator/root-cause evidence and a fresh
admission receipt. Test the emergency kill switch independently of statistical feedback and measure
its cluster propagation/fail-closed SLO.

**Tests.** Require zero security/scope boundary reuse, publish confusion matrix and score
distribution, and calculate false-reuse confidence bounds. ANN recall is compared with flat exact
search separately from semantic answer quality. Threshold mutations and label leakage canaries must
turn the gate red. Add canaries that count ANN nodes instead of authoritative coverage, promote a
shadow with missing partitions, omit duplicate-vector aliases, lower candidate budgets after seeing
results, lower score/margin from a client, admit a poisoned producer, count forged feedback, suppress
demotion or retire previous before rollback rehearsal; all must fail.

**Exit.** Auto-reuse remains disabled for any profile without a reproducible receipt. Hit rate alone
cannot promote a profile; false reuse, ambiguity, provenance and incomplete-search behavior are
primary release gates. Every promoted profile demonstrates feedback/circuit/kill-switch demotion and
cannot automatically re-enable itself.

## W13. Measure latency, avoided calls, quality and cost honestly

**Changes.** Build a common benchmark with HydraCache flat/HNSW, exact-key HydraCache, pinned Redis
semantic cache, pinned Hazelcast Enterprise VectorCollection and no semantic cache adapters.
Precompute identical embeddings and separate index latency from optional end-to-end embedding
latency. Add scalar/SIMD, named search class, single/batch ingestion, active-only/migration,
maintenance and failure-capacity cohorts.

**Implementation.** Run same-host counterbalanced pairs over fixed cardinality, dimensions, scope
selectivity, hit/miss/near-threshold mix, payload sizes, top-k, clients and cluster topology. Use
open-loop offered load and retain all errors, rejections, timeouts and incomplete results. Report
goodput, p50/p95/p99/p99.9, CPU/query, bytes/vector, rebuild time, recall@k, semantic precision,
avoided loader calls and avoided input/output tokens. Currency cost is timestamped illustrative
metadata, never the stable gate.

Keep two Hazelcast cohorts. The lower-layer ANN cohort uses the same normalized vectors, dimensions,
DOT metric, corpus, top-k, one/three-member topology, backup count and as-close-as-documented graph/
candidate budgets; pin Hazelcast/JDK/container/license identity, partition count, Vector API flag,
max-degree, `ef-construction`, `efSearch`, partition/member limits and all defaults. It compares
vector ingestion/search only and makes no semantic-cache claim. The end-to-end cohort uses a common
application adapter that adds the same hard scope, TTL, producer policy, decision score/margin and
authoritative final validation around VectorCollection, accounts for any companion metadata store
and rejects a cell as not comparable when equivalent semantics cannot be expressed. Do not weaken
HydraCache or Hazelcast backups/durability, hide Enterprise licensing, or infer universal superiority
from one cardinality/topology.

Report per-stage query work: partitions/members contacted, visited graph nodes, candidates emitted
at partition/member/coordinator, truncations, candidate bytes, authoritative rechecks and invalidated
candidates. Ingestion reports entries/s, bytes/s, partial/ambiguous rate and client/server peak
memory for single versus bounded batch. Maintenance reports reservation delay, build/replay/
validation/cutover/cleanup time, foreground latency/goodput impact and retained old-generation bytes.
Capacity cells compare predicted versus observed component/process peaks in steady, rebuild, N−1 and
combined N−1+rebuild states. SIMD claims require identical semantic gates and publish CPU feature/
kernel identity; profile migration reports dual-write/backfill cost rather than hiding it.
Admission/eviction cells publish accepted/rejected/evicted records, pollution resistance and bytes
per live reusable entry. Quality-circuit cells report verification overhead and demotion latency;
deletion cells report logical non-reuse and physical-reclamation latency separately.

**Tests/evidence.** Semantic equivalence and corpus admission run before timing. Include one/three
members, replication, failover, rebuild and noisy-tenant cells. Publish wins, losses, equivalent,
inconclusive and not-comparable cells; no composite score. Long-run exact-candidate tests prove
resource/lag stability. Counterbalance maintenance/no-maintenance and scalar/SIMD order; freeze batch
size and search budgets before timing. A benchmark adapter that uses a raw backend hint, different
candidate budget, incomplete fan-out, weaker backup policy or asynchronous durability is rejected as
not comparable. Hazelcast tests first run semantic/data parity, document every unavoidable mismatch
and retain only artifacts permitted by the pinned license; unavailable licensed runtime yields an
honest not-run receipt rather than substituting a mock.

**Exit.** Any published advantage names exact profile, topology, corpus, workload, product/tooling
SHA, host, estimator and uncertainty. A faster result with worse false reuse, recall, errors,
incompleteness or resource bounds is rejected.

## W14. Complete operations, upgrades, evidence and ship decision

**Changes.** Add management/readiness views, alerts, runbooks, rolling-upgrade/index-schema
migration, embedding-profile migration/cutover/rollback, capacity planning, maintenance jobs,
backup/restore treatment, SBOM/provenance and release evidence. The console remains read-only for
semantic data and never displays prompts/responses/vectors; maintenance actions remain authenticated
HC/2/CLI operations rather than browser buttons in 0.76.

**Implementation.** Expose bounded metrics for records/indexes, bytes, readiness state, build
generation, lag, query outcomes, validation rejects, rebuild/circuit state, recall diagnostic and
quality profile status. Rolling upgrade supports old readers plus new semantic-capable nodes; an
unsupported index schema rebuilds from authority. Backup owns authoritative semantic records; ANN
checkpoints are optional accelerators and are digest-validated/rebuildable.

Add fixed-cardinality metrics for `visited_nodes`, partition/member/coordinator candidates,
stage truncations, scalar/SIMD/flat fallback, authoritative recheck rejects, duplicate-vector ratio,
tombstone/fragmentation ratio, maintenance phase/progress/reservation, profile role/coverage/lag and
capacity predicted/observed bytes. Add provenance rejects, below-score/ambiguous-margin decisions,
score-disclosure denials, verification samples/feedback rejects, quality-circuit/kill-switch state,
authority admission/eviction weight, purge receipt age/backlog and scrub coverage/mismatch/repair.
Histograms have frozen buckets; tenant/index identifiers are hashed/bounded or available only through
authorized paged views, never labels.

The upgrade order is: deploy readers that understand new record fields → enable semantic capability
without active profiles → build/shadow index generations → admit profile → enable auto-reuse.
Downgrade first disables auto-reuse and new profile/batch/maintenance mutations, waits for bounded
drain, retains authoritative records readable by the supported version or executes the documented
rollback transform, and discards rebuildable index generations. Backup manifests include record and
profile/cutover metadata plus capacity/config digests; restored ANN checkpoints are accepted only
after epoch/schema/digest validation, otherwise a reserved rebuild runs.

Runbooks cover: poisoning response and writer revocation; exact-score diagnostic abuse; manual and
automatic quality demotion; emergency namespace/profile disable; admission/eviction pressure;
logical deletion versus backup-aware physical purge; scrub mismatch and rebuild; feedback outage;
profile rollback; maintenance cancellation; and restoration without a trusted ANN checkpoint. Each
action names required permission, observable receipt, safe retry/rollback and condition for returning
to active reuse.

**Tests.** Old/new daemon and SDK matrix, interrupted rebuild across restart, checkpoint downgrade,
backup/restore followed by full rebuild, full-cluster restart, cert rotation, clean workspace,
package consumers, docs examples, dynamic canaries and evidence archive verification. Add rolling
upgrade during dual-write/backfill and maintenance, active cutover with old coordinator present,
rollback after one-member loss, restore with/without optional ANN checkpoint, metric-cardinality and
no-secret assertions, capacity receipt reproduction on the admitted host, producer revocation,
score-oracle rate limiting, quality demotion during rolling upgrade, kill-switch propagation, purge
across retained backup expiry and live scrub corruption/repair.

**Exit.** Exact-candidate release evidence binds product/tooling/SDK/proto/index/corpus/profile/
comparison identities and every nested artifact hash. Release notes state admitted profiles and
non-claims. Missing quality/security/distributed evidence blocks the feature rather than weakening
the gate.

## Required test matrix

| Tier | Mandatory proof |
| --- | --- |
| Unit/model | Vector validation/canonicalization, normalized-DOT/cosine equivalence, scalar/SIMD differential, ordering, scope/provenance identity, stricter-only score/margin/age decision, disclosure policy, named-profile resolution, staged budgets/completeness, admission/eviction, batch outcomes, profile/quality/deletion/maintenance transitions, TTL/tag and exact lease fencing. |
| Property/fuzz | Wire/record/checkpoint/job/feedback/deletion decoders, arbitrary finite vectors and SIMD tails, operation/batch/profile/eviction/circuit/purge sequences, delta ordering, per-stage bounds and corrupt/truncated inputs. |
| Concurrency | Query vs replace/delete/evict/expire/backfill/cutover, lease holder death, circuit/kill-switch, rebuild delta cutover, scrub/purge/maintenance cancel-resume, batch retry, cancellation and shutdown; loom where state is bounded. |
| Cross-language | Rust/Java/Python canonical vectors, scope/provenance/fingerprint/config digests, decision/disclosure, named classes, completeness, feedback/deletion and batch/job HC/2 messages, error/retry mapping and live daemon interop. |
| Real process | One/three/five members, non-owner ingress, per-stage candidate cutoffs, failover, partition, rebalance, rolling upgrade, restart, profile cutover/rollback, quality demotion, maintenance/scrub/purge resume and namespace reuse. |
| Security | Tenant/backfill/profile-role/provenance/feedback forgery, poisoning and score-oracle abuse, diagnostic/maintenance authorization, internal-route auth/replay, batch amplification, malicious vectors, content-free telemetry, deletion/reclamation and evidence secret scan. |
| Quality | Labelled untouched corpus, hard boundaries/negatives, score/margin ambiguity, producer trust, flat-vs-ANN recall, active-vs-shadow migration, duplicate vectors, scalar-vs-SIMD, named search classes, post-promotion verification/demotion, confidence and mutation canaries. |
| Resource | Entry/index/vector-slot/payload/top-k/partition/member/concurrency/queue/batch/maintenance/scrub/purge/feedback/diagnostic limits, weighted eviction, active+shadow+previous, noisy tenant, cleanup and predicted-vs-observed normal/N−1/rebuild/N−1+rebuild envelopes. |
| Performance | Same-semantics open-loop exact/flat/HNSW/Redis/Hazelcast/no-cache controls, fixed embeddings, scalar/SIMD, single/batch ingest, named search classes, admission pollution, verification, deletion and foreground-under-maintenance long-run stability. |
| Release | Compatibility, producer revocation, profile migration/rollback, quality/kill-switch demotion, resumable maintenance/scrub/purge, deletion SLO, capacity receipt, metrics cardinality, SBOM/license/advisory/MSRV, clean packages, backup/restore, downgrade and immutable evidence. |

## Fast, scheduled and protected gates

**Fast PR gates:** formatting, clippy, doc-check, semantic contract validation, flat oracle,
canonical/golden vectors, scalar/SIMD differential, model/property tests, batch/profile/maintenance
state models, decision/provenance/feedback/deletion/eviction models, capacity-plan fixture, protocol
compatibility, Rust/Java/Python unit tests, bounded small-scope authority/index model, hostile input
tests and dynamic canaries.

**Scheduled gates:** fuzz corpora, Miri/loom, full cross-language daemon interop, three/five-member
fault/rebalance/rebuild suites, candidate-budget boundaries, active/shadow cutover/rollback,
maintenance crash/resume/cancel, rolling upgrade, namespace deletion, batch retry, noisy-tenant and
N−1+rebuild resource matrix, poisoning/score-oracle, scrub/purge, active verification/circuit,
quality corpus and dependency/SBOM audit.

**Protected exact-candidate gates:** admitted host, pinned corpus/profile/index/Redis/Hazelcast
identities and license receipt, same-host comparison, long-run quality-circuit/resource/lag/scrub/
deletion proof, full compatibility/package/backup/restore, independent artifact verification and
immutable release archive. Retries are append-only and cannot replace failed quality/security/
deletion evidence; unavailable licensed comparison is retained as `not-run`, never fabricated.

Representative commands are frozen in W0 and added to `docs/GATES.md`; at minimum they include:

```text
cargo xtask semantic-contract-check --release 0.76
cargo xtask semantic-capacity-plan --config tests/semantic-reference/admitted-capacity.toml --verify
cargo nextest run -p hydracache-semantic-index
cargo nextest run -p hydracache --test semantic_cache
cargo nextest run -p hydracache-server --test semantic_process
cargo nextest run -p hydracache-server --test semantic_profile_migration
cargo nextest run -p hydracache-server --test semantic_maintenance
cargo nextest run -p hydracache-server --test semantic_security
cargo nextest run -p hydracache-server --test semantic_quality_circuit
cargo test -p hydracache-client-hc2 --test semantic_interop
mvn -pl sdks/java/hydracache-client-hc2 test
python -m pytest sdks/python/hydracache-client-hc2/tests
cargo xtask release-evidence --release 0.76 --require-ship
```

## Release-level risks and mitigations

| Risk | Required mitigation |
| --- | --- |
| Similarity serves a plausible but wrong or ambiguous answer | Default shadow mode, hard scope, labelled hard negatives, immutable score plus top-two-margin policy and false-reuse gate. |
| Tenant data reaches another tenant's ANN graph | Physically/logically select index by authenticated tenant+namespace generation before search; hostile proof. |
| Index lag resurrects expired/deleted content | Versioned deltas plus mandatory authoritative final read; stale candidate becomes miss. |
| Rebuild marks ready too early | Snapshot+delta watermark parity, committed query epoch and falsifying early-ready canary. |
| HNSW improves latency by losing recall | Permanent flat differential, frozen recall floor and published misses. |
| Benchmark hides embedding cost | Separate precomputed-index latency and end-to-end embedding cohorts; never blend them. |
| Similar prompts collapse into one in-flight result | Exact canonical fingerprint only; model/loom tests with score-1.0 distinct prompts. |
| One hot tenant consumes CPU/memory | Per-tenant/index quotas and separate query/build/rebuild lanes with cold-progress gate. |
| New embedding model causes a flag-day rebuild or wrong-vector reuse | Bounded active/shadow/previous slots, expected-version backfill, coverage/quality gate, epoch-fenced cutover and rehearsed rollback. |
| Candidate limits silently remove the true neighbour | Three explicit frozen stages, truncation proof/metrics, flat differential and profile-specific recall gate. |
| Raw ANN tuning makes evidence irreproducible | Named immutable search-profile revisions; no public backend hints; configuration digest in every receipt. |
| Interrupted optimization damages the index | Copy-on-write target generation, resumable phase checkpoints, active-until-validated cutover and cancellation proof. |
| Duplicate-vector compaction hides valid records | Optional measured dedup only, bounded deterministic alias expansion and duplicate-heavy differential corpus. |
| SIMD changes score/order across hosts | Scalar reference and final candidate rescore, runtime fallback, architecture goldens and separate recall accounting. |
| Bulk ingestion amplifies memory or claims false atomicity | Pre-allocation bounds, partition grouping, per-item idempotency/outcomes, backpressure and explicit partial/ambiguous results. |
| Rebuild plus failover exhausts the cluster | Capacity receipt includes replicas, all profile generations, double-buffer, scratch and N−1 redistribution; combined failure gate. |
| Low-trust writer poisons a popular semantic neighbourhood | Server-derived writer class, admitted producer/pipeline policy in hard scope, separate write permission, quotas and poisoning corpus. |
| Exact scores become a membership/enumeration oracle | Hidden/coarse default disclosure, separate rate-limited diagnostic permission, audit receipt and abuse tests. |
| Offline-admitted quality drifts after deployment | Bounded authenticated verification/feedback, preregistered circuit, fast kill switch, automatic demotion only and operator-controlled re-admission. |
| Unique-vector floods defeat memory bounds | Full semantic weight, pre-commit reservation, bounded authority admission/eviction, pollution workload and coherent removal deltas. |
| Deletion is logically safe but sensitive vectors remain untracked | Separate logical/physical/backup SLOs, purge receipts, retention-aware wording and disk/backup/checkpoint scans. |
| Live index corruption survives until restart | Bounded digest/version scrub, immediate shard degradation and normal reserved rebuild with fault injection. |
| Hazelcast comparison weakens one side or violates licensing | Two scoped cohorts, pinned legal/runtime/config identities, semantic parity precheck and honest not-comparable/not-run cells. |
| Vector dependency expands supply-chain risk | Trait isolation, pinned source/checksum, SBOM/license/MSRV/advisory review and clean rebuild. |
| Product drifts into vector database/RAG scope | Capability manifest and release note non-goals; no document/query-language/reranker surface. |

## Deferred work

- Quantized vectors, product quantization, GPU indexes and disk-tier ANN require separate recall,
  portability, resource and recovery proposals.
- More than one active/one shadow/one previous vector slot, arbitrary user-named vector spaces and
  raw per-query ANN tuning are deferred; 0.76 supports migration, not a general multi-vector DB.
- Hybrid sparse+dense search, arbitrary metadata predicates, full-text search and reranking belong to
  a retrieval product, not the 0.76 response cache.
- Durable ordered agent memory, conversation history and episodic summarization require consent,
  retention, deletion, ordering and compaction contracts.
- A distributed token bucket and priority lease queue are useful AI gateway structures but are
  independent semantics and should receive their own release plan.
- Cross-region semantic indexes and active-active ANN convergence are not inferred from existing
  multiregion metadata features.
- Online learning, automatic threshold/margin tuning and automatic quality-circuit re-enablement are
  deferred; 0.76 can only demote automatically and requires a new immutable admission to promote.

## Final release decision

Release 0.76 is eligible only when:

- the published 0.75 distributed value plane is the sole semantic-record authority;
- public/wire/SDK types expose every miss, unavailable, overload and ambiguous outcome explicitly;
- Rust, Java and Python agree on vector bytes, identity, scope, ordering and outcomes;
- hard tenant/namespace/profile selection happens before similarity search;
- every returned hit passes authoritative version, TTL, age, generation, hard-scope and admitted
  producer/writer/pipeline provenance validation;
- automatic reuse applies an immutable server-owned minimum score, top-two margin and validation
  depth; client requests can only make the decision stricter and normal helpers do not expose an
  exact score oracle;
- stale/corrupt/missing/rebuilding index state can cause only explicit unavailability or miss;
- flat exact search remains a permanent oracle and all ANN recall/resource thresholds pass;
- cosine is represented by canonical normalized vectors, every admitted SIMD kernel passes scalar
  differential tests, and bounded final candidates are canonically rescored before reuse;
- named `Latency/Balanced/Recall/Exact` revisions bind construction/search and partition/member/
  global candidate limits; clients cannot supply raw backend hints;
- every strict result carries an operational completeness/truncation proof and exact mode never
  succeeds after budget truncation;
- TTL, tags, replacement, deletion, eviction and namespace reclamation update or safely invalidate
  derived state without resurrection;
- strict distributed queries prove bounded fan-out, completeness and deterministic merge through
  owner failure, partition, rebalance and rolling upgrade;
- single-flight/generation leases use exact fingerprints only and stale holders cannot publish;
- bounded batch and backfill operations enforce pre-allocation limits, partition grouping, per-item
  idempotency and typed partial/ambiguous outcomes without cross-partition atomicity claims;
- active/shadow/previous migration proves authoritative coverage, quality, resource budget,
  epoch-fenced cutover and rollback before the previous profile is retired;
- optimize/rebuild jobs reserve a double-buffer, checkpoint/resume/cancel safely, validate before
  cutover and never make deletion correctness depend on graph cleanup;
- authority admission/eviction accounts payload, metadata, every vector slot, replicas and measured
  derived charge, survives pollution/churn and emits coherent acknowledged removal deltas;
- logical deletion is immediately non-reusable, while authority/index/checkpoint/backup physical
  reclamation follows separately declared SLOs and produces bounded retention-aware receipts;
- bounded live integrity scrub detects version/digest/graph/checkpoint corruption, degrades only the
  affected scope and repairs through the normal reserved rebuild path;
- all per-entry/index/tenant/process memory, CPU, queue, task, fan-out and rebuild limits pass at
  boundary−1/boundary/boundary+1 and under noisy tenants;
- the exact-candidate capacity receipt reconciles predicted and observed component/process peaks and
  passes normal, N−1, rebuild and combined `N−1 + rebuild + foreground load` gates;
- automatic reuse is enabled only for exact admitted corpus/model/policy profiles, with zero hard
  boundary violations and frozen false-reuse confidence gates;
- active profiles retain authenticated bounded verification/feedback and a tested emergency kill
  switch; frozen breaches demote to `ShadowOnly`, and no profile can automatically promote/re-enable;
- shadow and active evidence publishes precision, false reuse, recall, misses, incompleteness,
  errors and resource trade-offs beside hit rate and latency;
- Redis/Hazelcast/no-cache comparisons use pinned same-semantics/runtime/config/license identities,
  separate lower-layer ANN from end-to-end semantic reuse and publish losses, not-run and
  not-comparable cells without a composite score;
- logs, metrics, traces, profiles, crash reports and public artifacts contain no prompt, response,
  vector coordinates, credentials or tenant-controlled identifiers;
- graph/search/decision/provenance/quality/eviction/deletion/scrub/profile/maintenance/capacity
  metrics have frozen units and bounded cardinality and explain visited nodes, stage candidates/
  truncation, fallbacks, fragmentation, demotion, purge and rebuild progress;
- old clients/daemons, backup/restore, full restart, downgrade/rebuild and rollback gates pass;
- exact product/tooling/proto/SDK/index/dependency/corpus/profile/workload/host identities and nested
  artifact checksums are archived immutably;
- release documentation says semantic response cache, not general vector database, RAG platform,
  model correctness guarantee or universal performance advantage.

Anything less ships with semantic reuse disabled or does not ship as 0.76.
