# HydraCache 0.69.0 Migration Conformance & Borrowed Test Suites - Codex Execution Plan

> **At a glance**
> - **What:** prove HydraCache's migration and compatibility claims with **other projects' own
>   evidence**: (W1) execute a provenance-bound adaptation of curated
>   **Hazelcast IMap/FencedLock expectations** against the buildable `0.68` Java source-preview
>   facade implementing the `0.52` surface contract - the facade does not implement Hazelcast's
>   Java interfaces and Hazelcast's cluster-owning tests therefore cannot run verbatim - the
>   borrowed-conformance pattern Caffeine uses to run
>   Guava's cache testlib against itself and Scylla uses for DynamoDB (alternator); (W2) an
>   embedded-cache semantics conformance set borrowed from the moka/caffeine expectations for the
>   in-process API; (W3) run **real previously published HC/1 client consumers** (built from
>   the shipped tags) against the current server - live artifacts, not byte fixtures, while the
>   retained HC/2 generation matrix from `0.68` remains a prerequisite; (W4) a
>   readyset/noria-style **cached-result vs direct-query differential** for the DB track under
>   concurrent writes, retrofitting `0.64`-era proof discipline onto the oldest shipped surface.
> - **Why:** the project's core positioning is Hazelcast migration, but every compatibility proof so
>   far was written by us (mined rows, hand-built oracles). A predecessor's own test suite encodes
>   thousands of behavioral expectations nobody re-derives by hand; passing it is the strongest
>   possible migration evidence, and each failure is either a real gap or a documented divergence.
>   Likewise `0.64` W32 proves old **bytes** decode, but never runs an old **client binary**; and
>   the `0.37`/`0.38` DB track predates the canary/falsifiability discipline entirely.
> - **After (depends on):** `0.68.0` (generated client plane, published Rust HC/2 client, live
>   remote subscriptions, and the first buildable Java SDK/facade source preview); public Java
>   migration claims additionally depend on full client-promotion admission and Maven publication.
>   Also consumes `0.52` (surface contract), `0.49` (legacy
>   client protocol/SDK), `0.37`/`0.38` (DB track), and the `0.64` governance machinery.
> - **Unblocks:** a defensible "Hazelcast-migration ready for the claimed subset" statement backed
>   by Hazelcast's own tests, client-upgrade guidance backed by executed old binaries, and the
>   stable post-HC/2 surface required by the `0.70` memory-efficiency release.
> - **Status:** shipped; the exact-candidate migration, compatibility, database, canary, and
>   fail-closed admission evidence is complete.
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
- **No new DB integration surface.** W4 keeps the existing outbox API family, but the final
  hardening pass makes `InvalidationWait` honor the receipt's exact commit position so unrelated
  later rows cannot create false degraded outcomes. SQLite, PostgreSQL, in-memory, and SQLx
  implementations must agree on that correction.
- **No benchmark claims.** Conformance only; performance stays `0.67`.

## Final Hardening Amendments

The release-review pass adds seven mandatory improvements. They are part of plan 69 and must be
covered by tests and release evidence rather than treated as optional follow-up:

1. **Per-row proof layers.** Hazelcast manifest schema v2 records `adapted-unit`, `live-daemon`,
   `server-state-machine`, and `recovery-interop` proofs with source, selector, and language.
   Every expected-pass Hazelcast row requires at least one non-double proof. TTL proves expiry,
   lock contention uses two independently signed client certificates, and lease-expiry/session-loss
   are explicit rows rather than prose-only supplements.
2. **Semantic canaries.** W0-W5 use distinct defects: abbreviated source commit, swallowed
   Hazelcast row, skipped embedded-cache row, false-green legacy tag, dropped invalidation, and an
   unwired PostgreSQL aggregate dependency. Generic work-item omission remains a closure test but
   is no longer the release canary for every item.
3. **PostgreSQL expected-red coverage.** The real PostgreSQL target contains a dropped-invalidation
   sentinel in addition to its happy path, is registered as an ignored external test, and has a
   dedicated CI step that only succeeds when the injected defect fails with `HC-CANARY-RED:W4-PG`.
4. **Real concurrency and multiple seeds.** SQLite and PostgreSQL each execute 12 concurrent
   transactional writers for seeds `0x69_2026`, `0x69_2027`, and `0x69_2028`, then require exact
   cached/direct convergence through the production outbox worker.
5. **Multi-language evidence selectors.** `release-evidence` validates Rust, JUnit Java, and Python
   test selectors. W1 names its live facade and recovery tests directly, so deleting a Java test
   moves the work item back to planned before Maven runs.
6. **Reproducible and diagnosable CI.** The PostgreSQL service image is digest-pinned; Java/legacy
   and PostgreSQL jobs have explicit timeouts; Surefire reports, daemon close receipts, PostgreSQL
   version, image, seed set, and full differential logs are retained with `if: always()`. The new
   SQLx fast-suite has a reviewed 120-second budget, raising the aggregate PR ceiling from 1560 to
   1680 seconds instead of borrowing evidence or budget from an unrelated suite. Java Surefire
   reports are copied to canonical files under `target/test-evidence/0.69`, allowing the release
   harness to digest them without accepting artifacts outside the repository target boundary.
   CI runs fast SQLx, Java, legacy, and both PostgreSQL proofs through `evidence-run`, then the
   `migration-conformance-admission-069` job downloads their exact-SHA receipts plus canary and
   workspace receipts and enforces `release-evidence --require-ship`.
7. **Commit-scoped invalidation waits.** `InvalidationOutbox::status_for_commit` is implemented by
   in-memory, SQLite, and PostgreSQL adapters. `InvalidationWait` uses it, and a later pending row in
   the same namespace is proven not to delay an already-published receipt.

**Hardening tests/gates:**

```powershell
cargo test -p hydracache-db --features sqlx-outbox --test outbox_barrier --locked
cargo test -p hydracache-db --features sqlx-outbox --test cached_vs_direct_differential --locked
cargo test -p xtask release_evidence::language_selector_tests --locked
cargo run -p xtask --locked -- canary-check --release 0.69
$env:HYDRACACHE_RUN_JVM_COMPAT='1'; cargo run -p xtask --locked -- borrowed-suite-check --suite hazelcast
$env:HYDRACACHE_CANARY_DEFECT='W4_PG_DROP'; cargo test -p hydracache-db --features sqlx-outbox --test cached_vs_direct_postgres canary_postgres_differential_rejects_a_dropped_invalidation -- --ignored --exact --nocapture
```

### CI-derived release strengthening

The first exact-candidate PR run exposed a deterministic process-fixture contract mismatch and a
diagnostic weakness: the producer added a second mTLS identity, but one Rust consumer still parsed
the old field count; the resulting Rust failure caused the final 0.69 admission job to be skipped.
The release therefore includes these additional mandatory controls:

1. `READY_DAEMON_V1` is a versioned receipt. The Rust producer and current consumer share a typed
   parser; both Java process consumers and the retained/external compatibility harnesses assert the
   same exact version and eight-field shape.
2. `migration-conformance-admission-069` uses `if: always()` and materializes a structured upstream
   status before downloading evidence. Failed or skipped dependencies are explicit red outcomes.
3. Fast 0.69 canaries, SQLx evidence, and `fast.workspace-nextest` run in the independent
   `migration-conformance-fast-evidence-069` job instead of depending on the monolithic Rust lane.
4. Every 0.69 lane finalizes a JSON lane status under `target/release-evidence/lanes`; ordinary
   `evidence-run` failures continue to write full gate receipts. Artifact upload therefore retains
   the original outcome without relying on a later successful step.
5. CI provenance records `HYDRACACHE_EVIDENCE_HEAD_SHA`, `HYDRACACHE_EVIDENCE_BASE_SHA`, and
   `HYDRACACHE_EVIDENCE_TESTED_SHA`; the tested SHA must equal the checkout used by `evidence-run`.
   The HC/2 client-plane workflow exports the same exact bindings for pull requests, branch and tag
   pushes, schedules, and manual dispatches; `client-plane-ci-check` rejects any missing or weakened
   binding before hosted or release admission can run.
6. PostgreSQL happy-path evidence is a digest-pinned matrix covering the declared 16.4 floor and
   current major 18. The test queries `server_version_num` and rejects a service that does not match
   its declared matrix series.
7. A downstream-style `CustomInvalidationOutbox` integration fixture compiles the public trait and
   proves exact namespace+commit filtering. The PostgreSQL 16 lane additionally runs a bounded
   24-seed, 12-writer-per-seed soak with a 120-second in-test budget.

The follow-up exact-candidate run also made the isolated W1 canary build its Maven reactor
dependencies with `-am`, and made Java reconnect treat cleanup of a session on an already-dead
transport as best effort. A cleanup exception can no longer abort endpoint replacement; the logical
session remains permanently lost and the loss metric remains observable. If the gRPC stream closes
before the handshake can be sent, that narrowly identified transport failure is classified as
`RECONNECT_IDEMPOTENT`, allowing ordered endpoint fallback; authentication and protocol-policy
failures remain terminal. Both decisions have deterministic Java regression tests in addition to
the live Rust-process recovery proof. The independently provisioned fast-evidence job installs every
tool exercised transitively by workspace Nextest, restores and verifies full tag/branch history
before its exact-candidate evidence step, and strips outer receipt provenance from registered child
processes after capturing it for the parent receipt. A nested-repository regression test proves
that a child cannot accidentally claim the outer candidate's SHA identity. Receipt bundles are
self-contained and retain their `target/`-relative layout: admission receives the JUnit, Java, and
PostgreSQL artifacts named by each receipt and re-hashes them instead of trusting receipt metadata
alone. The admission checkout also restores full history so the published `v0.68.0` compatibility
baseline tag remains a fail-closed release prerequisite rather than appearing absent in a shallow
checkout.

Tag qualification additionally treats ephemeral listener ownership and cluster status as concurrent
observations. The listener-reservation regression proves addresses are distinct and held together;
it does not attempt to reclaim released ports after another parallel test may legitimately acquire
them. Daemon replay evidence requires every observed node to retain the expected membership and
voter counts, plus a majority of authoritative statuses agreeing on one term and leader. A single
transient follower status without a leader or local quorum is retained as diagnostic evidence and
does not invalidate the live majority; split or minority-only authority remains release-blocking.
Scheduler/demotion proof samples also bind the separately fetched `/admin/status` and
`/cluster/overview` projections: both must expose non-zero epoch and term, the same leader, exact
epoch/term/leader equality, and the expected member/voter shape before the overview enters
authoritative history. Bootstrap sentinels and cross-snapshot stale views remain diagnostics until
the endpoints align within the bounded convergence deadline.

The final review adds four release-blocking closures. First, a commit status containing any
dead-lettered invalidation is terminal degraded evidence and `InvalidationWait` must never return
`satisfied`; in-memory, SQLite, and PostgreSQL paths cover that contract. Second,
`migration-conformance-check --upstream` downloads every unique borrowed source file from its exact
40-character GitHub commit and resolves every `path#selector`; the two Hazelcast lock rows now cite
real `FencedLock` symbols. Third, W3's only executable command is the implemented
`legacy-client-check --matrix hc1` runner, with complete manifest-row execution enforced. Fourth,
final admission parses the four downloaded lane-status JSON files and rejects an invalid schema,
non-pass or internally inconsistent outcome, missing upstream results, or any release/head/base/
tested-SHA mismatch before release receipts can satisfy shipping.

## Preflight

Re-grep before implementing:

- `0.68` Java SDK/facade: the buildable `hydracache-java-client` and Hazelcast-shaped facade
  artifacts, live connection/listener/session tests, lock lease/session/reentrancy tests,
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
| W0 | `docs/integrations/*.json`, `docs/testing/compat/legacy-clients.toml`, `xtask migration-conformance-check` | `cargo run -p xtask -- migration-conformance-check --upstream` | Structural validation plus exact pinned-commit source/selector resolution |
| W1 | Java facade borrowed-expectation runner + Hazelcast manifest | `cargo run -p xtask -- borrowed-suite-check --suite hazelcast` | Source-level facade only; no Hazelcast wire/interface claim |
| W2 | `crates/hydracache/tests/borrowed_cache_semantics.rs` | `cargo test -p hydracache --test borrowed_cache_semantics --locked -j 2` | Embedded cache surface only |
| W3 | `xtask legacy-client-check` + HC/1 manifest/current daemon harness | `$env:HYDRACACHE_RUN_LEGACY_CLIENTS='1'; cargo run -p xtask --locked -- legacy-client-check --matrix hc1` | HC/1 tags only; HC/2 evidence is reused from 0.68 |
| W4 | `crates/hydracache-db/tests/cached_vs_direct_{differential,postgres}.rs`, `outbox_{barrier,sqlite}.rs` | `cargo test -p hydracache-db --features sqlx-outbox --test cached_vs_direct_differential --locked -j 2` | Commit-position oracle; SQLite fast, PostgreSQL happy-path and expected-red gates |
| W5 | release registries/evidence/docs/CI | `cargo run -p xtask -- release-governance-check --release 0.69` | Exact-candidate ship proof |

## W0. Executable Manifest Contracts And Provenance

Define and validate the three manifests before implementing their runners. Every executable row has
a stable ID, an exact upstream repository/version/commit and source test, an expected outcome, and a
HydraCache test mapping. `divergence-documented`, `unsupported-documented`, and `skipped` rows require
a non-empty reason; skips are never green. Duplicate IDs, missing sources, unknown outcomes, or a
runner/manifest count mismatch fail closed. The pinned Hazelcast input is `v5.6.0` at
`a9ce2a02ac17f88fcd38869ac698e56e613dc40c`.

**DoD.**
```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- migration-conformance-check --structural
cargo run --manifest-path crates\xtask\Cargo.toml -- migration-conformance-check --upstream
```

## W1. Adapted Hazelcast IMap/FencedLock Expectations Against The Java Facade (blueprint: `caffeine/guava/src/compatibilityTest/`, `scylladb/test/alternator/`)

**Principle.** The predecessor's tests are an independent source of behavioral expectations. This
is an adaptation with per-row source provenance, not a claim that Hazelcast's cluster-owning test
classes execute unchanged: those tests construct real Hazelcast members and depend on Hazelcast
internals, while the shipped HydraCache facade deliberately implements neither `IMap` nor
`FencedLock` and has no Hazelcast runtime dependency. Every red result is signal: a real gap, a
divergence to document, or future work to name.

**Files to change.** A test-only runner in the existing Java facade module; a **borrowed-suite manifest**
`docs/integrations/hazelcast_borrowed_suite.json` in the `0.63` conformance style: every borrowed
test class/method -> `expected: pass | divergence-documented | unsupported-documented | skipped(reason)`;
a runner that executes the adapted expectation against the facade and diffs actual vs manifest.

**Design.**
- Start with a 10-20-row feasibility slice, then expand only after the runner proves exact outcome
  accounting. Curate by shipped surface: IMap get/put/CAS (`replace(k,old,new)`, `remove(k,val)`),
  entry listeners, FencedLock acquire/release/lease-expiry/session-loss. Hazelcast reentrancy is an
  explicit `divergence-documented` row because `HydraFencedLock` is intentionally non-reentrant;
  this release does not widen that contract.
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

## W3. Live Previous HC/1 Client Consumers Against The Current Server (blueprint: Hazelcast old-client/new-member practice; extends `0.64` W32 beyond byte fixtures)

**Principle.** `0.64` W32 proves old **bytes** decode; it never proves an old **client binary**
completes a session. Handshake negotiation, retry behavior, and error mapping only surface with the
real artifact.

**Files changed.** `crates/xtask/src/migration_conformance.rs` provides the executable matrix runner
that builds pinned HC/1 consumer fixtures against the library artifacts from shipped tags
(`v0.62.0`, `v0.62.1`, `v0.63.0`) into a cache directory
(recorded commit + toolchain, `0.64` W32 provenance discipline); a matrix manifest
`docs/testing/compat/legacy-clients.toml` (tag -> surface -> expected outcome).

**Design.**
- Each legacy HC/1 consumer runs its supported subset (handshake, get/put, TTL where its protocol
  version allows, lock ops for `v0.63`) against a current daemon. The tags publish a library rather
  than a runnable binary, so the versioned consumer fixture is the executable artifact and records
  the tag commit plus toolchain. Per the protocol contract, `v2`/`v3` clients must succeed on their surface and **never**
  receive `v4` or generation-2 shapes.
- Do not duplicate `0.68` W9/W10: HC/2 generations 5/6 and the nine-row client-plane compatibility
  artifact remain mandatory prerequisites and are revalidated with
  `client-plane-compat-check --manifest-only`.
- A legacy client offered an unsupported operation fails loud with the documented error, not a hang.
- Skip-loud when a tag cannot be built reproducibly; the row stays visibly non-green (`R-11`), the
  same rule as W32's baseline decision.

**Required tests:**
- `legacy-client-check --matrix hc1` executes every manifest consumer against the current daemon;
- `legacy_execution_must_cover_every_manifest_row` rejects a falsely green partial execution;
- `client-plane-compat-check --manifest-only` retains the nine-row HC/2 prerequisite.

**Canary.** `canary_legacy_matrix_marks_an_unbuilt_tag_green`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_LEGACY_CLIENTS='1'
cargo run -p xtask --locked -- legacy-client-check --matrix hc1
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
cache and (b) a direct DB query. The oracle records a logical commit position for each write and
compares both reads at that position; it never compares an older cached snapshot with a direct read
that may already include a later commit. `NoWait` may be stale before the captured invalidation is
drained; `Local` and `BestEffort` assertions are tied to the shipped outbox-wait contract rather
than being relabelled as arbitrary read consistency. Convergence must be exact after quiescence.
SQLite runs fast; PostgreSQL joins the existing Docker gate (W35 adapter-corpus pattern) as a
digest-pinned 16.4/18 support matrix. The 16.4 floor also executes a bounded 24-seed soak.

**Required tests:**
- `cached_reads_match_direct_queries_per_consistency_mode_under_concurrent_writes`;
- `post_quiescence_cache_and_source_are_exactly_equal`;
- `stale_read_beyond_the_documented_bound_is_red_not_tolerated`.

**Canary.** `canary_db_differential_accepts_a_dropped_invalidation` - a fixture that swallows one
invalidation must produce a detected mismatch.

**DoD.**
```powershell
cargo test -p hydracache-db --features sqlx-outbox --test cached_vs_direct_differential --locked -j 2
$env:HYDRACACHE_TEST_POSTGRES_URL='postgres://hydracache:hydracache@127.0.0.1:5432/hydracache'
cargo test -p hydracache-db --features sqlx-outbox --test cached_vs_direct_postgres --locked -- --ignored --nocapture
cargo test -p hydracache-db --test custom_outbox_contract --locked
```
**CI.** SQLite and workspace receipts run in `migration-conformance-fast-evidence-069`; PostgreSQL
16.4 and 18 rows run in the matrix-backed `migration-conformance-postgres-069` service lane. The
final admission always runs, records upstream outcomes, and is required by `hc2-linux-required`.

## W5. Governance, CI, And Docs

- `docs/testing/release-evidence/0.69.toml` work items for W0-W4 with receipts;
  `release-evidence --release 0.69 --require-ship` is the ship gate. Register every gated lane
  (JVM, legacy-tag builds, Postgres) in the gated-test registry with tier/timeout/owner; canary
  pairs in the canary registry; quarantine rules unchanged.
- Extend `release-governance-check --release 0.69` coverage (structural manifest checks for the
  three new manifests: borrowed-suite, cache-semantics, legacy-clients).
- Docs: `docs/integrations/hazelcast-migration-evidence.md` - what the borrowed suite proves, the
  divergence ledger, and the standing rule that the migration claim never exceeds the manifest;
  reconcile `GATES.md`/`TESTING.md`/`COMPAT.md`/`releases.toml`/`INDEX.md`/plan header/
  `docs/releases/0.69.0.md`; `doc-check` green.

**DoD.**
```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- release-governance-check --release 0.69
cargo run --manifest-path crates\xtask\Cargo.toml -- release-evidence --release 0.69
cargo run --manifest-path crates\xtask\Cargo.toml -- doc-check
```

## Gates (Definition of Done for the release)

- The borrowed Hazelcast subset executes against the buildable `0.68` Java facade implementing
  the `0.52` surface contract with **every** outcome
  matching the versioned manifest - unexpected passes are as red as unexpected failures; every
  divergence/unsupported row carries a reason; the pinned Hazelcast version is a reviewed input;
  the swallow-canary is caught.
- The embedded cache semantics set executes every manifest row (count-checked) and matches; rows
  our API intentionally lacks are `unsupported-documented`, never silently green.
- Real `v0.62.x`/`v0.63.0` HC/1 consumer fixtures compiled against the shipped libraries complete
  their supported surface against the current server; legacy clients never receive `v4` or HC/2-only shapes,
  the retained `0.68` HC/2 compatibility matrix remains green,
  all clients fail loud beyond their surface, and an unbuildable tag is visibly non-green rather
  than substituted.
- The retained HC/2 clean-package consumer derives the Rust archive version from
  `workspace.package.version`, requires the frozen API manifest to match it, and rejects a stale
  release-cut archive name before admission; both the workspace and docs-example lock files bind
  their local HydraCache packages to the same release version.
- The real-daemon HC/1+HC/2 harness reserves all loopback listeners simultaneously before spawn,
  proving that independently requested ephemeral ports cannot collapse onto one config address.
- The DB differential holds per declared consistency mode under seeded concurrent writes on the
  PostgreSQL 16.4/18 matrix, is exact after quiescence, detects the dropped-invalidation canary,
  and completes the bounded 24-seed floor-version soak.
- Every suite/canary/gated lane is registered in the `0.64` governance machinery; a green
  `release-evidence --release 0.69 --require-ship` on the candidate commit is the ship gate; all
  lanes run locally and in GitHub CI with skip-loud discipline.
- No product surface was widened to satisfy a borrowed test; every red result became a narrow fix
  with regression, a reasoned divergence row, or named future work (`R-11`).

## Final Release Decision

Ship `0.69.0` only when the compatibility story is proven by evidence we did not author: adapted,
source-pinned Hazelcast expectations pass (or are reasoned) against the Java facade under an exact-outcome manifest; borrowed
embedded-cache expectations execute completely; real previously shipped client binaries talk to the
current server within their protocol contract; and the oldest shipped surface - the DB query cache -
differentially matches its source of truth under racing writes with a canary proving the check can
fail. The migration claim then rests on executed third-party expectations and live artifacts, with
divergences documented rather than hidden, and the claim never exceeds the manifests that encode it.
