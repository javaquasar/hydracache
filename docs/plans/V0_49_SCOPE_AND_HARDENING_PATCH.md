# HydraCache 0.49 Scope & Hardening Patch — Codex Execution Plan

> **At a glance**
> - **What:** five corrections/additions to [`V0_49_ECOSYSTEM_AND_EXTERNAL_CONSUMERS_PLAN.md`](V0_49_ECOSYSTEM_AND_EXTERNAL_CONSUMERS_PLAN.md): (P5) split the oversized scope into a shippable core + a follow-on migration release, (P6) fix the non-JVM SDK language now, (P7) make the wire-framing choice an ADR deliverable, (P8) route the multi-node faults through the `0.44` deterministic simulator, and (P9) add a region-scoped cache-change subscription to the W1 protocol contract.
> - **Why:** the mechanical fixes (test names, dependency-graph dedup, R-id references) made `0.49` *internally consistent*, but did not address the four risks that can still make the release ship red or diverge in implementation: it is too large to land on one green gate, leaves two forever-decisions open (SDK language, protocol framing), and validates its hardest correctness claims (residency-under-failover, tenant fair-share) only as `#[ignore]` chaos instead of reproducibly.
> - **After (depends on):** none new — this is a documentation/scoping patch over the existing `0.49` plan; it does not change `0.37`–`0.48`. It is a **supporting plan, not a release version** (not tracked in `releases.toml`).
> - **Unblocks:** a `0.49` that can ship on a boolean gate (R-7) and an unambiguous Codex execution path.
> - **Status:** proposal — apply after the user confirms the P5 split option.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · gates: [`../GATES.md`](../GATES.md)

This is a patch plan: each item names the **target section/anchor in the `0.49` plan**, the
**change**, the **recommended decision**, and the **acceptance check**. One item = one
commit/PR over the `0.49` plan document (and, where noted, `releases.toml`/`INDEX.md`).
It introduces no code; it makes `0.49` smaller, more decided, and more verifiable.

## Why a patch, not a rewrite

`0.49` is well-structured and its Final Decision is correctly boolean. The only changes
needed are to its **shape**, not its content: what ships together, which choices are
pinned, and how the multi-node claims are proven. Keeping this as a separate, referenced
patch (a) preserves the `0.49` plan's review history, (b) lets the P5 split be decided
without churning the whole document, and (c) records the rationale for the two pinned
decisions (P6/P7) in one place.

## Patch items

```
P5 scope split ──► defines which work items stay in 0.49 vs move to a follow-on
        │
        ├──► P6 pin non-JVM SDK language (affects W3, and the follow-on if split)
        ├──► P7 pin wire framing via ADR (affects W1)
        ├──► P8 route multi-node faults through 0.44 DST (affects W4, W5 fault model)
        └──► P9 region-scoped SubscribeInvalidations (affects W1 + storage-doc §3.1)
```

---

## P5. Split the scope: shippable core (`0.49`) + migration follow-on

**Target.** `0.49` "Scope note" (the paragraph that keeps *all* of W0–W7 in scope and
explicitly defers nothing), the "Dependency Graph", and the "Final Release Decision".

**Problem.** `0.49` as written is two to three releases of work in one: a stable external
protocol + multi-tenancy + residency + audit (already large), **plus** a complete
Java/Spring migration toolkit (Boot 2/3/4 starters, JCache, Hibernate L2, listener
annotations, Micrometer, Actuator probe, smart routing). The "Scope note" actively
refuses to defer anything. Under R-7 (ship without a claim rather than on a red gate),
bundling a JVM ecosystem toolkit into the same all-or-nothing decision is the single
biggest risk to ever shipping `0.49`.

**Change.** Re-cut the scope along the natural seam already present in the body grouping:

- **`0.49` core (external-consumer-ready grid):** W0 surface, W1 protocol, W2 Hibernate
  L2 provider (the highest-value, smallest JVM surface), W4 isolation, W5 residency, W6
  observability/audit. This is a coherent, gate-able "the grid is a safe, governed,
  external backend" claim.
- **Follow-on release (Java/Spring migration ecosystem):** W3 the non-JVM SDK +
  conformance suite **and** W7 the full Spring/Boot/JCache migration toolkit, which
  together are the "make legacy Java migration a config change" claim. Keep W2 in core
  (Hibernate L2 is the one JVM piece with direct DB-loop value and a small SPI surface);
  move the broad client-ergonomics surface out.

**Recommended decision.** Adopt the split. Slot the follow-on as a **new release `0.52`**
(after `0.51`), `depends_on = ["0.49.0"]`, numbered to avoid renumbering the in-flight
line — the same convention used for `0.50`/`0.51`. Do **not** renumber `0.46`–`0.51`.

> Alternative if the user prefers a single release: keep W3/W7 in `0.49` but mark the
> Boot 4 starter, JCache binding, and smart-routing as **explicit stretch** with their
> own sub-gates, so a red stretch sub-gate ships `0.49` *without those sub-claims*
> instead of blocking the whole release (still R-7, lower confidence than the split).

**Steps (doc-only).**
1. Rewrite the `0.49` "Scope note" to state the core/follow-on seam and reference this
   patch.
2. Move the W3 and W7 sections (or, in the alternative, their stretch sub-items) into a
   new `docs/plans/V0_52_JAVA_SPRING_MIGRATION_ECOSYSTEM_PLAN.md`, carrying their
   problem/design/tests verbatim; leave a one-line pointer in `0.49`.
3. Trim the `0.49` Dependency Graph and Final Decision to the core items; add the
   corresponding W-items to the `0.52` plan's own Final Decision.
4. If split: add the `0.52` entry to `releases.toml` (`depends_on = ["0.49.0"]`) and a
   row + DAG side-branch to `INDEX.md`; run `cargo xtask doc-check`.

**Acceptance.** The `0.49` Final Decision lists only core W-items; `doc-check` is green;
if split, `0.52` exists with W3/W7 and resolves. No work item is silently dropped — each
appears in exactly one plan's Final Decision.

---

## P6. Pin the non-JVM SDK language now (Python)

**Target.** `0.49` W3 "Design / contract" (the sentence that defers the Python-vs-Node
choice to "W3 planning").

**Problem.** A Codex execution plan that leaves the first SDK language undecided stalls
the agent at the first concrete step and makes the conformance harness un-buildable.

**Change.** Replace the deferred choice with a pinned one and its rationale.

**Recommended decision.** **Python.** Rationale to record in the plan: (a) it is the
fastest path to a working conformance runner; (b) it is the dominant language for the
data-platform / ML consumers that the storage roadmap targets
(`STORAGE_AND_DATA_PLATFORM_EVOLUTION.md` §5 vectors); (c) async client maturity
(`asyncio`/`httpx`) matches the W1 HTTP/2 surface. Node remains a *later* SDK, not a
fork in this plan. (If the primary target were browser/edge consumers, Node would win —
record that as the explicit trigger to revisit.)

**Steps (doc-only).**
1. In W3, change "pick Python or Node" to "Python (rationale: …); Node deferred".
2. Make the W3 packaging step concrete: `pyproject.toml`, semver tied to the W1 protocol
   support window, schema-generated client surface where possible.
3. Keep the conformance manifest language-agnostic (already required by
   `conformance_manifest_is_language_agnostic`) so Node can be added later without a
   manifest change.

**Acceptance.** W3 names exactly one non-JVM SDK with packaging metadata; the
language-agnostic conformance manifest test still holds.

---

## P7. Pin the wire framing via an ADR deliverable

**Target.** `0.49` W1 "Design / contract" (the "length-prefixed binary … or gRPC-shaped
is acceptable" sentence) and W1 step list.

**Problem.** The protocol framing is a **forever-compatibility** decision (R-4), and the
plan leaves it as a fork. Two implementers (or the agent across sessions) could diverge,
and a public protocol cannot quietly switch framing later.

**Change.** Require the choice to be made and recorded as an ADR before the first
supported version is published; the COMPAT entry references the ADR.

**Recommended decision.** Choose **custom length-prefixed binary frames over the existing
axum HTTP/2 transport** (not full gRPC/tonic), because: (a) it keeps the dependency
surface and build complexity low (consistent with the lean-core positioning); (b) it lets
the frame carry HydraCache's own `protocol_version`, request envelope, and B1 watermark
fields without bending them to protobuf service semantics; (c) it avoids a second IDL/
codegen toolchain competing with the Rust types that are the compatibility source of
truth. Record gRPC/tonic as the considered alternative and the reason it was not chosen
(SDK ubiquity vs. control over framing/versioning).

> If broad off-the-shelf client tooling is judged more important than framing control,
> gRPC/tonic is the defensible opposite choice — but it must then be the *single* choice,
> with the protobuf schema registered in COMPAT as the wire artifact.

**Reversibility — choosing custom binary now is not a one-way door.** Because the
operation set (`Get`/`Put`/`Invalidate`/batch/`SubscribeInvalidations`), the request/error
envelopes, structured key segments, and the B1 watermark are defined by the Rust types —
**independent of framing** — gRPC can be adopted later as a *second protocol major* (`v2`
over gRPC/tonic) introduced **alongside** `v1`, not as an in-place swap: the server speaks
both during the support window, version negotiation (W1) picks one, old clients keep `v1`,
and the COMPAT register tracks both wire artifacts (R-4, forward-only). The cost is a
`.proto` generated from the same operations + a transport adapter + running the W3
conformance suite against both encodings — integration work, not a redesign. ADR `0007`
must record this reversibility explicitly so the `v1` choice stays low-risk. The migration
stays cheap **only if** `v1` never leaks framing details into semantics (no app logic
depending on exact byte layout) — which the structured-key / typed-operation contract
already enforces.

**Steps (doc-only).**
1. Replace the W1 "either/or" sentence with the pinned framing + a one-line rationale.
2. Add an ADR deliverable to W1 steps: `docs/adr/0007-client-wire-framing.md` (decision,
   alternatives, compatibility implications), referenced from the W1 `docs/COMPAT.md`
   entry.
3. Add it to the W1 Final-Decision condition ("framing ADR exists and COMPAT references
   it").

**Acceptance.** W1 names one framing; ADR `0007` exists and is linked from COMPAT; no
"either/or" remains in the protocol contract.

---

## P8. Route the multi-node faults through the `0.44` deterministic simulator

**Target.** `0.49` "Fault Model and Test Tiering" and the W4/W5 testing blocks
(`fair_share_prevents_one_tenant_starving_replication`,
`residency_holds_under_region_failover`).

**Problem.** The `0.49` preamble promises "any multi-node behavior gets coverage in the
`0.44` `hydracache-sim` deterministic harness", but the two hardest *multi-node*
correctness claims are currently only `#[ignore]` chaos / property tests:
residency-under-failover (W5) and tenant fair-share starvation (W4). Chaos tests find
bugs but are not reproducible release evidence; `0.48` W8 already sets the precedent of
adding new fault types to the `0.44` sim and asserting invariants there (R-5).

**Change.** Add the consumer-surface *multi-node* invariants to the `0.44` deterministic
harness with seeded, replayable runs in the fast budget, keeping the `#[ignore]` chaos as
an additional soak tier (not the only evidence).

**Recommended decision.** Adopt. Single-node protocol faults (malformed frame, version
mismatch, oversized payload) stay as fast unit/property tests — they do not need the
multi-node sim. The sim gains: **residency-under-failover** (a `0.45` W4 failover must
never promote a home outside the allowed set; if none survives in policy, report degraded
— assert as a seeded invariant) and **tenant fair-share under multi-tenant load** (no
tenant starves another past the fair-share bound across the seed matrix).

**Steps (doc-only over `0.49`; the code lands in the `0.49` W4/W5 PRs).**
1. In the Fault Model section, move residency-under-failover and fair-share from "chaos
   only" to "modeled in the `0.44` sim (fast budget) + chaos soak".
2. Add to W5 testing: `residency_under_failover_holds_in_sim` (deterministic, fast
   budget) alongside the existing `#[ignore]` chaos variant.
3. Add to W4 testing: `fair_share_holds_in_sim` (deterministic seed matrix) alongside the
   existing property test.
4. Add a focused gate line `cargo test -p hydracache-sim --locked consumer_invariants`
   and a Final-Decision condition that these invariants hold across the seed matrix
   (mirrors `0.48` W8).

**Acceptance.** Residency-under-failover and fair-share each have a seeded, replayable sim
test in the fast budget (R-5), the focused gates include the sim suite, and the chaos
variants remain as soak. No multi-node claim rests on `#[ignore]`-only evidence.

---

## P9. Region-scoped cache-change subscription in the W1 contract

**Target.** `0.49` W1 "Design / contract" + Rust sketch (`SubscribeInvalidations`,
`InvalidationEvent`, `ClientContext`); the `STORAGE_AND_DATA_PLATFORM_EVOLUTION.md` §3.1
change-stream note.

**Problem.** W1 lets a client subscribe to cache-change events, but only
**namespace-scoped** (`SubscribeInvalidations { ns, from }`). There is no way to subscribe
to changes for a **specific region**, even though `0.45` made regions first-class and
operators of active-active/geo deployments routinely want "stream me what changed in
region X" (per-region near-cache, regional projections, regional audit).
`ClientContext.preferred_region` only steers read routing, not subscription scope.
Against etcd watch / Hazelcast listeners this is a real gap for geo consumers.

**Change.** Add an optional region scope to the subscription with explicit,
correctness-safe semantics.

**Recommended decision.** Extend the contract to
`SubscribeInvalidations { ns, region: Option<RegionId>, from: Option<Watermark>, include_value: bool }`:

- **`region = None`** → current behavior (namespace-wide, all regions).
- **`region = Some(r)`** → "**applied-in-region r**": events for keys as they are
  applied/committed in r's replica, gated by **r's resolved-timestamp watermark**
  (storage-doc §3.1), so the per-region stream is consistent and replayable — the
  etcd-revision model, region-local.
- **Correctness invariant (R-1):** a region filter must **never hide a cross-region
  invalidation that affects the subscriber's view**. If a key the subscriber tracks is
  invalidated under a newer epoch elsewhere, the region-scoped stream still emits it (or
  advances the watermark and signals a gap that forces a conservative repair). Region is a
  **dissemination** dimension; authority stays epoch/version. The filter narrows
  *delivery*, never *correctness*.
- **`include_value` + residency (R-3 / W5):** value-in-event is opt-in and must pass W5
  residency — a pinned value is never shipped to a subscriber whose region/connection is
  outside the allowed set; the event then degrades to invalidation-only (key + watermark),
  counted, never silently dropped.

Record region as a **dissemination filter, not an authority source** in the W1 contract
and storage-doc §3.1.

**Steps (doc-only over `0.49` + storage doc; code lands in the W1 PR).**
1. Update the W1 `SubscribeInvalidations` contract + Rust sketch with
   `region`/`include_value` and the applied-in-region semantics.
2. Add the R-1 "filter narrows delivery, not correctness" invariant and the W5 residency
   gate for `include_value` to the W1 design text.
3. Cross-link `STORAGE_AND_DATA_PLATFORM_EVOLUTION.md` §3.1: the region-scoped stream is
   the resolved-ts change-stream restricted to one region.
4. Add the W1 Final-Decision condition + the tests below.

**Testing.** add to `crates/hydracache-client-protocol/tests/protocol.rs`:
- `region_scoped_subscription_streams_only_that_regions_applied_events` (integration,
  2-region sim).
- `region_filter_does_not_hide_cross_region_invalidation_affecting_subscriber`
  (integration) — the R-1 hard case; run in the `0.44` simulator (ties P8).
- `include_value_is_residency_gated_and_degrades_to_invalidation` (integration) — ties W5.
- `region_subscription_resume_and_gap_trigger_repair` (property): per-region watermark
  resume + gap → `RepairAction`, exactly like the namespace stream.
- Run: `cargo test -p hydracache-client-protocol --locked protocol`.

**Acceptance.** A client can subscribe to changes for one region; the region filter never
suppresses a correctness-relevant cross-region invalidation; `include_value` honors
residency; per-region resume/gap behaves like the namespace stream; the region semantics
are documented in W1 and storage-doc §3.1 as dissemination-only.

---

## Order of application

1. **P6, P7, P9** first (W1/W3 contract, no roadmap impact): pin SDK + framing + ADR
   `0007`, and add the region-scoped subscription to the W1 contract.
2. **P8** next: re-tier the multi-node faults (touches Fault Model + W4/W5 testing); the
   P9 cross-region invariant test lands in the same `0.44` sim work.
3. **P5** last, **after user confirms the split option**: it is the only item that may
   create a new release (`0.52`) and edit `releases.toml`/`INDEX.md`; do it once the
   other three have settled the content that moves.

## Acceptance for the whole patch

- `0.49` contains no undecided fork (SDK language, wire framing) and no copy-pasted rule
  (already done in the mechanical pass).
- Every multi-node correctness claim has reproducible (seeded) evidence, not only chaos.
- `0.49` Final Decision is gate-able as a single coherent claim (core), with the
  Java/Spring migration either split out (`0.52`) or explicitly stretch-gated.
- `cargo xtask doc-check` stays green; if `0.52` is created, its `depends_on` resolves and
  it appears in `INDEX.md`.
