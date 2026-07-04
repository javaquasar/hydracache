# HydraCache 0.57.1 Technical Debt Closure — Codex Execution Plan

> **At a glance**
> - **What:** a focused **maintenance release** that closes the *actually-closeable* technical debt in
>   `docs/technical-debt/` before the `0.58` soak work — **dependency hygiene** (TD-0003 buckets A/B),
>   a **supply-chain advisory re-affirmation** (TD-0002), a **driven operator lifecycle E2E**
>   (TD-0007), and a **TD-ledger reconciliation** (mark closed items, honestly re-scope the
>   feature-sized ones). It does **not** pretend to close debts that are really future *features*.
> - **Why now:** `0.57` shipped and left several tracked debts. `0.58` (endurance/soak) reuses the
>   operator kind harness and a healthy dependency graph, so hardening those first is the natural
>   pre-`0.58` step. Develop-**downward** housekeeping: no new product surface, no new algorithms.
> - **After (depends on):** `0.57.0`. **Sequenced before `0.58.0`** (which lists `0.57.1` in its
>   `depends_on`: `0.58` W4 reuses the operator E2E harness hardened here).
> - **Status:** shipped.
>
> Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> debt ledger: [`../technical-debt/README.md`](../technical-debt/README.md) ·
> gates: [`../GATES.md`](../GATES.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its Definition of
Done **and** `cargo xtask verify`; never push red.

## Preflight (verified against the repo — file:line the work touches)

- **Workspace deps live in one place:** `Cargo.toml` `[workspace.dependencies]` (Cargo.toml:47-115),
  MSRV `rust-version = "1.88"` (Cargo.toml:35). Bucket-B crates and their exact current versions:
  `criterion = "0.5"` (Cargo.toml:54), `reqwest = "0.12"` (Cargo.toml:93), `sha2 = "0.10.9"`
  (Cargo.toml:98), `sqlx = "0.8"` (Cargo.toml:101). Blocked bucket-C: `sea-orm = "1.1.17"`
  (Cargo.toml:94), `protobuf = "2.28.0"` (Cargo.toml:87), `sled = "0.34.7"` (Cargo.toml:99).
- **Consumers of the bucket-B crates (which suites to run):**
  - `sha2`: `hydracache-sql-lint` (`Cargo.toml:15`), `hydracache-db` (`Cargo.toml:22`).
  - `reqwest`: `hydracache-operator` (`Cargo.toml:18`), `hydracache-client` (`Cargo.toml:18`),
    `hydracache-cluster-transport-axum` (`Cargo.toml:19`).
  - `sqlx`: `hydracache-sqlx` (`Cargo.toml:18/22`), `hydracache-db` (`sqlx-outbox` feature,
    `Cargo.toml:14/23`), `hydracache-sandbox` (`Cargo.toml:27`).
- **Supply-chain state:** `deny.toml` `[advisories].ignore` already lists **four** IDs, each pointing
  at TD-0002: `RUSTSEC-2024-0437` (protobuf 2.x via raft), `RUSTSEC-2023-0089` (postcard/heapless),
  `RUSTSEC-2025-0057`, `RUSTSEC-2026-0173` (deny.toml:3-6). `raft = "0.7.0"` (Cargo.toml:89) pulls
  `protobuf 2.28.0` unconditionally via the `protobuf-codec` feature.
- **Operator is plan-based (drives the W4 E2E):** pure planners already unit-tested —
  `plan_scale(cluster, obs) -> ScalePlan` (`crates/hydracache-operator/src/scale.rs:141`,
  `quorum_for` :297, `ScaleAdminClient` :337, `admin_base_url` :309), `plan_upgrade`
  (`upgrade.rs:109`, `version_skew_supported` :233, `PodObservation::from_pod` :35),
  `plan_tls_rotation` (`tls.rs:211`), `plan_backup` (`backup.rs:57`),
  `plan_pitr_restore_into_fresh_cluster` (`backup.rs:118`). The only live-cluster test today is
  `crates/hydracache-operator/tests/e2e.rs::full_lifecycle_…` — a **prepared-state snapshot** (gets
  the StatefulSet, checks `ready_replicas >= quorum` + Service selector + `unavailable <= 1`), gated
  by `HYDRACACHE_OPERATOR_KIND=1` (e2e.rs:10-16). Object-shape is in `tests/reconcile.rs`.
- **doc-check now validates plan header status** (`crates/xtask/src/doc_check.rs::check_plan_header_status`)
  — keep this plan's `**Status:**` header, `releases.toml`, and `INDEX.md` in lockstep.

## Debt Triage (verified against `docs/technical-debt/`)

| TD | Title | Disposition in 0.57.1 |
| --- | --- | --- |
| TD-0001 | MSRV-pinned SQLx/testcontainers | **Already closed** (0.7.0) — reconcile ledger only (W5). |
| TD-0002 | raft-rs 0.7 protobuf advisory (`RUSTSEC-2024-0437` + 3 transitives) | **Re-affirm** (W3): can't remove `protobuf 2.x`; refresh `deny.toml` reasons/dates, re-check for a fixed `raft`, clear transitives where a compatible refresh exists. |
| TD-0003 | Dependency upgrade backlog (A/B/C) | **Act** (W1 bucket A, W2 bucket B). Bucket C stays blocked. |
| TD-0004 | Deferred placement/autoscaling | **Out of scope** — intentional *feature* deferral. |
| TD-0005 | Release-claim evidence gap | Wording branch **already applied**; **artifact branch out of scope** (future Java toolkit). Reconcile status only (W5). |
| TD-0006 | Release-plan status validation | **Already resolved** — reconcile ledger only (W5). |
| TD-0007 | Operator lifecycle E2E coverage | **Close** (W4): a **driven** kind E2E, not a prepared-state snapshot. |
| TD-0008 | Networked daemon grid hosting (W6b) | **Out of scope** — feature-sized; feeds `0.59.0`. |

## Non-Goals / Explicitly Out of Scope (named, not hidden)

- **TD-0004 placement/autoscaling** — changes placement authority; a feature.
- **TD-0005 artifact branch** — a real `hydracache-hibernate` + Java toolkit is a JVM deliverable; the
  wording overclaim is already fixed (no honesty debt remains), only the optional future artifact.
- **TD-0008 networked daemon grid (0.57 W6b)** — feature-sized; feeds `0.59.0`.
- **TD-0003 bucket C** (`sled 1.0-alpha`, `sea-orm 2.0-rc`, `protobuf 4.x`) — pre-release /
  transitively pinned; **do not migrate**.
- No new product features, no new consistency level (R-1), no MSRV raise unless a bump forces it.

## Dependency Graph

```
0.57.0 shipped ─► 0.57.1 debt closure ─► 0.58.0 soak (reuses W4 operator E2E harness + healthy deps)
                    W1 lockfile hygiene ─┐
                    W2 scheduled majors ─┼─► W5 ledger reconcile + gates
                    W3 advisory re-affirm┤
                    W4 driven operator E2E ┘
```

---

## W1. Dependency lockfile hygiene (TD-0003 bucket A)

**Goal.** Refresh `Cargo.lock` to current semver-compatible versions (security patches) under the
gates — routine hygiene, not a migration.

**Files.** `Cargo.lock` only (bucket-A crates already sit within the caret requirements at
Cargo.toml:47-115: `axum`, `bytes`, `syn`, `serde`/`serde_json`, `tokio`/`tokio-stream`, `tower`,
`utoipa`/`utoipa-swagger-ui`, `proc-macro-crate`, `quote`, `trybuild`, `proptest`, `toml`).

**Steps.**
1. `cargo update` (no `Cargo.toml` edits — caret requirements already admit the newer minors/patches).
2. `cargo xtask verify` green; `cargo deny check` clean (licenses/advisories/bans/sources).
3. Commit the refreshed `Cargo.lock` alone.

**Tests & requirements.**
- **No new test code.** The requirement is the **full gate suite stays green** on the refreshed lock:
  `cargo xtask verify` (fmt, clippy `-D warnings`, workspace tests, doc-check, deny). This is the
  regression proof for a lockfile bump.
- **Requirement:** the commit contains **only** `Cargo.lock` (no manifest/source drift) — a bucket-A
  refresh that needs a `Cargo.toml` edit is a *major bump* and belongs in W2, not here.
- Windows note: an `LNK1104` during the test build is a Defender linker lock, **not** a failure — retry.

**Risk & rollback.** Pure lockfile refresh; revert restores the prior `Cargo.lock`.

## W2. Scheduled major bumps (TD-0003 bucket B — one crate per commit)

**Goal.** Land the safe major bumps as **dedicated** commits with the named suites. Never bundle with
feature work; each is independently revertible.

**Files.** `Cargo.toml` `[workspace.dependencies]` (the single version line per crate) + `Cargo.lock`,
one crate per commit.

**Steps (each its own green commit).**
1. `sha2 "0.10.9" → "0.11"` (Cargo.toml:98). Consumers: `hydracache-sql-lint`, `hydracache-db`.
   RustCrypto bump; may cascade with sibling RustCrypto crates in the lock.
2. `criterion "0.5" → "0.8"` (Cargo.toml:54). Dev-dependency/benches only — no runtime impact;
   confirm benches still compile (`cargo build --benches` where present).
3. `reqwest "0.12" → "0.13"` (Cargo.toml:93). Consumers: `hydracache-operator`, `hydracache-client`,
   `hydracache-cluster-transport-axum`. Keep `default-features = false, features = ["json"]`.
4. `sqlx "0.8" → "0.9"` (Cargo.toml:101) — **evaluate**; DB-adapter core with breaking API + MSRV
   interaction (TD-0001). If the migration is non-trivial, **defer it as its own follow-up and record
   the reason in TD-0003** (honest deferral, not a silent skip or a half-done port).

**Tests & requirements (per bump).**
- **Must run the consumer suites, not just `verify`:**
  - `sha2`: `cargo test -p hydracache-sql-lint -p hydracache-db --locked`.
  - `reqwest`: `cargo test -p hydracache-operator -p hydracache-client -p hydracache-cluster-transport-axum --locked`.
  - `sqlx`: `cargo test -p hydracache-sqlx -p hydracache-db --locked` (Postgres integration is
    testcontainers-gated — skip-graceful without Docker, per the existing suite).
- **Requirements:** one crate/group per commit; `cargo deny check` clean; `cargo xtask verify` green.
  If a bump clears an advisory, the matching `deny.toml` ignore is removed **in the same commit**.
- **Falsifiable-deferral requirement (sqlx):** if deferred, TD-0003 §Bucket B must gain a dated line
  stating *why* (breaking API / MSRV), so the deferral is auditable — not an empty checkbox.

**Risk & rollback.** Each bump reverts independently; the hard migration (sqlx) is deferred with a
written reason, never half-applied.

## W3. Supply-chain advisory re-affirmation (TD-0002)

**Goal.** Re-verify the four ignored advisories are still unavoidable, keep every `deny.toml` ignore
dated + reasoned + TD-linked, and clear the unmaintained transitives where a compatible refresh now
exists.

**Files.** `deny.toml` (the `[advisories].ignore` block, deny.toml:2-7),
`docs/technical-debt/TD-0002-raft-protobuf-advisory.md`; reads `Cargo.toml:87,89`
(`protobuf`/`raft`) and `crates/hydracache-cluster-raft/Cargo.toml`.

**Steps.**
1. Check crates.io for a `raft` release that removes `protobuf 2.x` or supports `prost-codec` without
   a local `protoc`. If none, keep `protobuf-codec` (Cargo.toml:89) and **refresh** the four ignore
   reasons in `deny.toml` with the re-check date.
2. Re-check the transitives behind `RUSTSEC-2023-0089` / `-2025-0057` / `-2026-0173`
   (`atomic-polyfill` via `postcard`/`heapless`; `fxhash` / `proc-macro-error2` via `raft 0.7`). If a
   semver-compatible refresh (from W1) drops any from the graph, **remove that ignore**; otherwise
   record that it remains upstream-bound.
3. `cargo deny check advisories` passes with every remaining ignore carrying a reason + TD link.

**Tests & requirements.**
- **Gate as test:** `cargo deny check` (advisories + licenses + bans + sources) is the executable
  proof and must be green.
- **Requirement:** **no ignore without a dated reason + a `docs/technical-debt/TD-0002` reference**
  (RULES: `deny` stays authoritative). An ignore whose advisory no longer appears in the graph must be
  **removed**, not left dangling.
- **Requirement:** TD-0002 status/"Related Warnings" updated with the re-check date and outcome
  (found-fix / still-blocked) so the debt does not silently rot.

**Risk & rollback.** Docs + `deny.toml` hygiene; no code-path changes.

## W4. Driven operator lifecycle E2E (TD-0007)

**Goal.** Replace the prepared-state snapshot with a kind E2E that **drives** the lifecycle chain and
asserts an invariant **at each transition** — the evidence the `0.56` plan promised — nightly/gated and
skip-graceful, sharing the harness `0.58` W4 will reuse.

**Files.** `crates/hydracache-operator/tests/e2e.rs` (extend/replace `full_lifecycle_…` so it *acts*),
a kind provisioning helper (e.g. `tests/support/kind.rs`), `docs/GATES.md` (name the gated command).
Drives the shipped, already-unit-tested planners: `plan_scale` (scale.rs:141) via `ScaleAdminClient`
(scale.rs:337) + `admin_base_url` (scale.rs:309); `plan_upgrade` (upgrade.rs:109); `plan_tls_rotation`
(tls.rs:211); `plan_backup` (backup.rs:57) + `plan_pitr_restore_into_fresh_cluster` (backup.rs:118);
observes via `ScaleObservation::from_statefulset` (scale.rs:39), `PodObservation::from_pod`
(upgrade.rs:35), `TlsRotationObservation`, `BackupObservation`; asserts via the emitted `Condition`s
(`scale_condition` scale.rs:318, `backup_completed_condition` backup.rs:157, upgrade/tls conditions).

**Code sketch (driven chain, replacing the snapshot).**
```rust
// crates/hydracache-operator/tests/e2e.rs — DRIVE the chain, assert each transition.
#[tokio::test]
async fn full_lifecycle_drives_install_scale_upgrade_rotate_backup_restore() {
    let Some(kind) = KindHarness::try_start() else { return log_skip(); }; // skip-graceful (e2e.rs:14)
    let cr = apply_cluster(&kind, sample_spec()).await;          // install
    kind.wait_ready(&cr, quorum_for(3)).await;                   // scale.rs:297 quorum

    scale(&kind, &cr, 5).await;                                  // drives plan_scale -> ScaleAdminClient
    assert!(kind.ready_replicas(&cr).await >= quorum_for(5), "quorum preserved during scale");

    rolling_upgrade(&kind, &cr, NEXT_IMAGE).await;               // drives plan_upgrade
    assert!(kind.max_unavailable(&cr).await <= 1, "one pod at a time");
    assert!(kind.has_leader(&cr).await, "leader re-elected during upgrade");

    rotate_tls_secret(&kind, &cr).await;                         // drives plan_tls_rotation
    assert!(kind.connections_uninterrupted(&cr).await, "no dropped mTLS conns");

    let backup = run_backup(&kind, &cr).await;                   // drives plan_backup
    let restored = restore_pitr(&kind, backup).await;           // plan_pitr_restore_into_fresh_cluster
    assert_eq!(restored.committed_writes, cr_committed_writes(), "no lost committed write");
}
```

**Test descriptions & requirements.**
- `full_lifecycle_drives_install_scale_upgrade_rotate_backup_restore` (kind-gated, driven): must
  **act** at each stage and assert the stage invariant (quorum preserved on scale; ≤1 unavailable +
  leader re-elected on upgrade; no dropped connection on rotation; no lost committed write on
  restore). **Requirement:** each assertion checks a *post-transition* observation, not a static
  fixture.
- `deliberate_two_pods_down_during_upgrade_fails_loud` (**falsifiability requirement**): force two
  pods unavailable and assert the test **fails** the quorum invariant — proves the E2E is not vacuous.
- `e2e_skips_gracefully_without_a_cluster` (kept, e2e.rs:135): without `HYDRACACHE_OPERATOR_KIND=1`
  the whole suite skips **green**; **requirement:** `cargo xtask verify` stays green with no kind
  cluster and no Node.
- **Gating requirement:** the driven test runs only in a named nightly/gated tier documented in
  `docs/GATES.md` (kind is heavy) — it is **excluded from the fast PR gate**, mirroring the `0.56`
  Docker/kind rows. `HYDRACACHE_OPERATOR_KIND` / `_NAMESPACE` / `_CLUSTER` env contract preserved
  (e2e.rs:10-24).
- **Determinism requirement:** the harness provisions a fresh cluster/namespace per run and tears it
  down, so reruns are independent.

**Risk & rollback.** Real-cluster driven E2E is heavy → nightly/gated, off the PR gate. Revert keeps
the prepared-state snapshot; TD-0007 stays open. Shares provisioning with `0.58` W4.

## W5. TD-ledger reconciliation + gates (cross-cutting)

**Goal.** Make `docs/technical-debt/` tell the truth after this release: closed items marked closed,
re-scoped items pointed at their future homes, index consistent.

**Files.** `docs/technical-debt/README.md`, the individual TD files, `docs/plans/releases.toml`,
`docs/plans/INDEX.md`.

**Steps.**
1. Confirm/annotate **closed**: TD-0001 (0.7.0), TD-0006 (resolved). Set TD-0007 → **Resolved** if W4
   lands (else note partial). Refresh TD-0002 (W3 outcome + date) and TD-0003 (W1/W2 outcome, incl. a
   dated sqlx-deferral line if applicable).
2. Re-affirm **out-of-scope-here** with pointers: TD-0004 (feature), TD-0005 artifact branch (future
  Java toolkit; wording already fixed), TD-0008 (feeds `0.59.0`).
3. `cargo xtask verify` green — including `doc-check`, which now also validates the plan `**Status:**`
   header vs the manifest (`check_plan_header_status`). Keep `0.57.1` header/manifest/INDEX in sync.

**Tests & requirements.**
- **Executable check:** `cargo xtask doc-check` must be green — it enforces manifest↔INDEX↔header
  consistency, so a stale `0.57.1` status anywhere fails the gate.
- **Requirement (honesty, R-11):** no TD is marked "closed" unless its Definition-of-Done in the TD
  file is actually met; feature-sized debts are labelled "out of scope here → <future home>", **not**
  quietly closed.
- **Requirement:** `docs/technical-debt/README.md` open-items list matches the real statuses.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green (fmt, clippy, tests, doc-check, COMPAT, deny) throughout.
- **TD-0003 bucket A** applied (lockfile-only commit, gates green); **bucket B** safe bumps landed as
  dedicated green commits with their **consumer suites** run; `sqlx` migrated **or** deferred with a
  dated written reason in TD-0003 (W1/W2).
- **TD-0002** re-affirmed: `cargo deny check` clean, every ignore dated + reasoned + TD-linked, a fixed
  `raft` re-checked, transitives cleared where possible (W3).
- **TD-0007** closed by a **driven** kind E2E that asserts each transition and is **falsifiable**
  (two-pods-down fails loud), skip-graceful locally, gated to nightly (W4).
- **Ledger honest:** closed debts marked closed; feature-sized debts (TD-0004, TD-0005 artifact,
  TD-0008) named out-of-scope-here with pointers, **not** silently closed (R-11, W5).
- No new features, no new consistency level (R-1); embedded fast path unchanged (R-10); no numeric
  self-score (R-7).
- `releases.toml` + `INDEX.md` updated; `0.58.0` `depends_on` includes `0.57.1`; this plan's
  `**Status:**` header matches the manifest (doc-check `check_plan_header_status`).
