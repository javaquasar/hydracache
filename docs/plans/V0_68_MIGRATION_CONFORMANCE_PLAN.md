# HydraCache 0.68.0 Migration Conformance & Borrowed Test Suites - Codex Execution Plan

> **At a glance**
> - **What:** prove HydraCache's migration and compatibility claims with **other projects' own
>   evidence**: (W1) execute a curated subset of **Hazelcast's own IMap/FencedLock test suite**
>   against the shipped `0.52` Java facade - the borrowed-conformance pattern Caffeine uses to run
>   Guava's cache testlib against itself and Scylla uses for DynamoDB (alternator); (W2) an
>   embedded-cache semantics conformance set borrowed from the moka/caffeine expectations for the
>   in-process API; (W3) run **real previously published HydraCache client binaries** (built from
>   the shipped tags) against the current server - live artifacts, not byte fixtures; (W4) a
>   readyset/noria-style **cached-result vs direct-query differential** for the DB track under
>   concurrent writes, retrofitting `0.64`-era proof discipline onto the oldest shipped surface.
> - **Why:** the project's core positioning is Hazelcast migration, but every compatibility proof so
>   far was written by us (mined rows, hand-built oracles). A predecessor's own test suite encodes
>   thousands of behavioral expectations nobody re-derives by hand; passing it is the strongest
>   possible migration evidence, and each failure is either a real gap or a documented divergence.
>   Likewise `0.64` W32 proves old **bytes** decode, but never runs an old **client binary**; and
>   the `0.37`/`0.38` DB track predates the canary/falsifiability discipline entirely.
> - **After (depends on):** `0.67.0` (release chain); consumes `0.52` (Java facade), `0.49`
>   (client protocol/SDK), `0.37`/`0.38` (DB track), and the `0.64` governance machinery.
> - **Unblocks:** a defensible "Hazelcast-migration ready for the claimed subset" statement backed
>   by Hazelcast's own tests, and client-upgrade guidance backed by executed old binaries.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md) -
> governance: `0.64` W33 (registries, receipts, `release-evidence --require-ship`).

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md)
first. This is a **conformance-evidence** release: it executes borrowed suites and live artifacts and
records honest pass/divergence ledgers. It does **not** widen the supported surface to make a
borrowed test pass; a red borrowed test becomes either (a) a narrow fix with its own commit and
regression test, (b) a documented-divergence ledger row with a reason, or (c) a named future-work
item - never a silent skip and never a quiet feature addition (`R-11`).

## Source Reflection (verified blueprints)

- `cashe/caffeine/guava/src/compatibilityTest/` - Caffeine executes **Guava's** cache test
  library against the Caffeine adapter. Principle: *the predecessor already wrote your conformance
  suite; run it.*
- `cashe/scylladb/test/alternator/` - Scylla proves DynamoDB compatibility by running
  DynamoDB-shaped expectations against alternator, keeping an explicit list of intentional
  divergences. Principle: *borrowed suite + divergence ledger, not cherry-picked examples.*
- `cashe/hazelcast/` - the source of the borrowed IMap/FencedLock tests for W1 and the
  old-client compatibility practice (old clients against new members) for W3.
- readyset/noria (workspace) - the cached-view vs base-table consistency discipline for W4.
- `0.63` conformance-manifest discipline - every borrowed/derived row lives in a versioned
  manifest with per-row status and a covering test; no ad hoc lists.

## Non-Goals

- **No new product surface.** The `0.52` lock/IMap subset, the `R-2` unsupported-manifest stance,
  and the client protocol stay as shipped. A borrowed test for an unimplemented feature is recorded
  as `unsupported-documented`, not implemented to make the suite green.
- **No full-Hazelcast claim.** W1 curates the subset matching shipped semantics (IMap CAS ops,
  entry listeners, FencedLock lease/reentrancy); CP-subsystem, WAN, SQL, Jet, and other Hazelcast
  suites are explicitly out of scope with a named ledger.
- **No wire-protocol compatibility with Hazelcast clients.** W1 drives the **Java facade API**,
  not Hazelcast's binary protocol.
- **No DB feature work.** W4 measures/verifies the shipped `0.37`/`0.38` semantics; reconciliation
  and outbox mechanics are not redesigned.
- **No benchmark claims.** Conformance only; performance stays `0.67`.

## Preflight

Re-grep before implementing:

- `0.52` Java facade: the `hydracache-java`/facade artifacts, lock lease/session/reentrancy tests,
  IMap CAS (`replace(k,old,new)`, `remove(k,val)`), entry-listener bus wiring, and the reversed
  unsupported-manifest lock subset rows.
- `0.49` client protocol/SDK conformance harness (Rust/Python SDK conformance), the published tags
  `v0.62.0`/`v0.62.1`/`v0.63.0` and what client crates/bins each tag can build.
- `0.64` W32 `compat_matrix.rs` + `docs/testing/compat/` manifest (byte fixtures; W3 extends, must
  not duplicate) and the governance seams (`release-evidence`, gated/canary registries, quarantine).
- DB track: `crates/hydracache-db` (hooks/CDC, named consistency modes, outbox, reconciliation
  drift reports in `reconcile.rs`), which invariants are already asserted vs merely reported.
- JVM availability in CI (`0.63` used maven/temurin images for the JVM client row - reuse that
  gate pattern).

Audit question:

```text
For each compatibility claim (Hazelcast-shaped Java facade, embedded cache semantics, old client
compatibility, DB cache-vs-source consistency), is the evidence generated by an INDEPENDENT party's
suite or a LIVE prior artifact - or only by tests we wrote ourselves against our own understanding?
```

## Implementation Map For Audits

Populate as W-items land: item -> where implemented -> required command -> boundary/gate.

| Item | Implemented where | Required command | Boundary |
| --- | --- | --- | --- |
| _(populate during implementation; W1-W5 below define the targets)_ | | | |

## W1. Borrowed Hazelcast IMap/FencedLock Suite Against The Java Facade (blueprint: `caffeine/guava/src/compatibilityTest/`, `scylladb/test/alternator/`)

**Principle.** Passing the predecessor's own tests is the strongest migration proof and the cheapest
source of thousands of expectations. Every red result is signal: a real gap, a divergence to
document, or future work to name.

**Files to change.** New JVM module (e.g., `java/hazelcast-compat-suite/` or alongside the `0.52`
facade module) with a pinned Hazelcast source/test-jar version; a **borrowed-suite manifest**
`docs/integrations/hazelcast_borrowed_suite.json` in the `0.63` conformance style: every borrowed
test class/method -> `expected: pass | divergence-documented | unsupported-documented | skipped(reason)`;
a runner that executes the curated subset against the facade and diffs actual vs manifest.

**Design.**
- Curate by shipped surface: IMap get/put/CAS (`replace(k,old,new)`, `remove(k,val)`),
  entry listeners, FencedLock acquire/release/reentrancy/lease-expiry/session-loss.
- The runner fails on **any** unmanifested outcome: an unexpected pass (claim widened silently) is
  as red as an unexpected failure - the `0.63` no-silent-drift rule in both directions.
- Divergence rows carry a reason and, where applicable, the `R-2`/`0.52` manifest reference.
- Pin the Hazelcast version; upgrading it is a reviewed compatibility change (`0.63` oracle-pinning
  discipline).

**Required tests/gates:**
- `borrowed_hazelcast_suite_outcomes_match_the_manifest_exactly`;
- `manifest_has_no_unreviewed_skip_and_every_divergence_has_a_reason`;
- `unexpected_pass_or_fail_versus_manifest_is_red`.

**Canary.** `canary_borrowed_suite_runner_treats_an_unlisted_failure_as_skip` - a fixture failure
absent from the manifest must fail the runner, proving it cannot silently swallow outcomes.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_JVM_COMPAT='1'
cargo run --manifest-path crates\xtask\Cargo.toml -- borrowed-suite-check --suite hazelcast
Remove-Item Env:\HYDRACACHE_RUN_JVM_COMPAT -ErrorAction SilentlyContinue
```
**CI.** JVM-gated lane (reuse the `0.63` maven/temurin gate pattern), scheduled + release-proof;
manifest/structural checks run fast on every PR.

## W2. Embedded Cache Semantics Conformance Set (blueprint: `caffeine/guava` adapter pattern applied to moka/caffeine expectations)

**Principle.** The embedded API is the oldest surface with the least borrowed scrutiny. Port the
*semantic expectations* encoded in moka/caffeine test suites (present/absent/loading semantics,
listener ordering guarantees, eviction-notification contracts, weigher/capacity edge behavior,
expiry variants) into a manifest-driven Rust conformance set for `hydracache`'s cache API - each row
citing the source test it was derived from.

**Files to change.** `crates/hydracache/tests/borrowed_cache_semantics.rs` + manifest
`docs/integrations/cache_semantics_borrowed.json` (row: source project/test -> our expectation ->
status). Rows for semantics HydraCache intentionally does not have (e.g., weighted eviction if
unclaimed) are `unsupported-documented`, mirroring W1.

**Required tests:**
- `borrowed_cache_semantics_rows_all_execute_and_match_manifest`;
- `no_row_is_silently_absent_from_execution` (count check, W19-style).

**Canary.** `canary_cache_semantics_runner_skips_a_listed_row`.

**DoD.**
```powershell
cargo test -p hydracache --test borrowed_cache_semantics --locked -j 2
```
**CI.** Fast `rust` job (pure in-process).

## W3. Live Previous-Client Binaries Against The Current Server (blueprint: Hazelcast old-client/new-member practice; extends `0.64` W32 beyond byte fixtures)

**Principle.** `0.64` W32 proves old **bytes** decode; it never proves an old **client binary**
completes a session. Handshake negotiation, retry behavior, and error mapping only surface with the
real artifact.

**Files to change.** `crates/hydracache-server/tests/legacy_client_matrix.rs` + an xtask helper that
builds pinned client artifacts from the shipped tags (`v0.62.x`, `v0.63.0`) into a cache directory
(recorded commit + toolchain, `0.64` W32 provenance discipline); a matrix manifest
`docs/testing/compat/legacy-clients.toml` (tag -> surface -> expected outcome).

**Design.**
- Each legacy client runs its supported subset (handshake, get/put, TTL where its protocol version
  allows, lock ops for `v0.63`) against a current daemon; per the protocol contract, `v2`/`v3`
  clients must succeed on their surface and **never** receive `v4` shapes.
- A legacy client offered an unsupported operation fails loud with the documented error, not a hang.
- Skip-loud when a tag cannot be built reproducibly; the row stays visibly non-green (`R-11`), the
  same rule as W32's baseline decision.

**Required tests:**
- `v062_and_v063_client_binaries_complete_their_supported_surface_against_current_server`;
- `legacy_clients_never_receive_v4_shapes_and_fail_loud_beyond_their_surface`.

**Canary.** `canary_legacy_matrix_marks_an_unbuilt_tag_green`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_LEGACY_CLIENTS='1'
cargo test -p hydracache-server --test legacy_client_matrix --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_LEGACY_CLIENTS -ErrorAction SilentlyContinue
```
**CI.** Gated lane in the compatibility job (tag builds are slow); registry rows + fast structural
manifest check on PR.

## W4. DB-Track Differential: Cached Result Versus Direct Query Under Concurrent Writes (blueprint: readyset/noria view-maintenance discipline; retrofits `0.64`-era proof onto `0.37`/`0.38`)

**Principle.** A query cache is correct only if the cached answer equals the direct answer under
the declared consistency mode - especially while writes race. The shipped reconciliation (`0.38`)
detects *outbox drift*; it does not differentially prove *result equality* under load. The DB track
predates canaries, seeds, and falsifiability entirely.

**Files to change.** `crates/hydracache-db/tests/cached_vs_direct_differential.rs`: a seeded
generator interleaves writes (insert/update/delete via the hooked paths) with reads through (a) the
cache and (b) a direct DB query, then compares per the named consistency mode's contract
(read-after-write barrier rows must match immediately; bounded modes must match within the
documented bound; convergence must be exact after quiescence). SQLite runs fast; Postgres joins the
existing Docker gate (W35 adapter-corpus pattern).

**Required tests:**
- `cached_reads_match_direct_queries_per_consistency_mode_under_concurrent_writes`;
- `post_quiescence_cache_and_source_are_exactly_equal`;
- `stale_read_beyond_the_documented_bound_is_red_not_tolerated`.

**Canary.** `canary_db_differential_accepts_a_dropped_invalidation` - a fixture that swallows one
invalidation must produce a detected mismatch.

**DoD.**
```powershell
cargo test -p hydracache-db --test cached_vs_direct_differential --locked -j 2
```
**CI.** SQLite row in the fast `rust` job; Postgres row in the existing Docker-gated lane.

## W5. Governance, CI, And Docs

- `docs/testing/release-evidence/0.68.toml` work items for W1-W4 with receipts;
  `release-evidence --release 0.68 --require-ship` is the ship gate. Register every gated lane
  (JVM, legacy-tag builds, Postgres) in the gated-test registry with tier/timeout/owner; canary
  pairs in the canary registry; quarantine rules unchanged.
- Extend `release-governance-check --release 0.68` coverage (structural manifest checks for the
  three new manifests: borrowed-suite, cache-semantics, legacy-clients).
- Docs: `docs/integrations/hazelcast-migration-evidence.md` - what the borrowed suite proves, the
  divergence ledger, and the standing rule that the migration claim never exceeds the manifest;
  reconcile `GATES.md`/`TESTING.md`/`COMPAT.md`/`releases.toml`/`INDEX.md`/plan header/
  `docs/releases/0.68.0.md`; `doc-check` green.

**DoD.**
```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- release-governance-check --release 0.68
cargo run --manifest-path crates\xtask\Cargo.toml -- release-evidence --release 0.68
cargo run --manifest-path crates\xtask\Cargo.toml -- doc-check
```

## Gates (Definition of Done for the release)

- The borrowed Hazelcast subset executes against the `0.52` Java facade with **every** outcome
  matching the versioned manifest - unexpected passes are as red as unexpected failures; every
  divergence/unsupported row carries a reason; the pinned Hazelcast version is a reviewed input;
  the swallow-canary is caught.
- The embedded cache semantics set executes every manifest row (count-checked) and matches; rows
  our API intentionally lacks are `unsupported-documented`, never silently green.
- Real `v0.62.x`/`v0.63.0` client binaries complete their supported surface against the current
  server, never receive `v4` shapes, fail loud beyond their surface, and an unbuildable tag is
  visibly non-green rather than substituted.
- The DB differential holds per declared consistency mode under seeded concurrent writes, is exact
  after quiescence, and the dropped-invalidation canary is detected.
- Every suite/canary/gated lane is registered in the `0.64` governance machinery; a green
  `release-evidence --release 0.68 --require-ship` on the candidate commit is the ship gate; all
  lanes run locally and in GitHub CI with skip-loud discipline.
- No product surface was widened to satisfy a borrowed test; every red result became a narrow fix
  with regression, a reasoned divergence row, or named future work (`R-11`).

## Final Release Decision

Ship `0.68.0` only when the compatibility story is proven by evidence we did not author: Hazelcast's
own tests pass (or are reasoned) against the Java facade under an exact-outcome manifest; borrowed
embedded-cache expectations execute completely; real previously shipped client binaries talk to the
current server within their protocol contract; and the oldest shipped surface - the DB query cache -
differentially matches its source of truth under racing writes with a canary proving the check can
fail. The migration claim then rests on executed third-party expectations and live artifacts, with
divergences documented rather than hidden, and the claim never exceeds the manifests that encode it.
