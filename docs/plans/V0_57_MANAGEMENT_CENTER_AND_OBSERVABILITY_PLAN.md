# HydraCache 0.57.0 Management Center & Observability Console — Codex Execution Plan

> **At a glance**
> - **What:** turn the shipped-but-incomplete observability pieces into a **real, honest,
>   operate-in-prod surface**. In order: (**W0, preflight, load-bearing**) replace the stub
>   `ServerRuntime::admin_status()` with a real **`ClusterStatusProvider`** seam so the daemon
>   reports *actual* cluster status (members / leader / term / epoch / reshard phase) sourced from the
>   grid control plane — with an explicit `source: "live" | "modeled"` tag so the console can never
>   paint modeled data as live (R-11); (**W1**) **complete the Prometheus exporter** so it emits the
>   admission + cluster-grid series it currently only *reserves*; (**W2**) **serve `/metrics`** on the
>   already-separate internal admin port; (**W3**) a read-only **`ClusterOverview`** read-model over
>   W0 + the observability registry, served on the admin surface; (**W4**) a read-only **Management
>   Center web console** built on the existing
>   `demo/` Playwright front-end; (**W5**) wire-through, docs, gates; (**W6**) **host the real grid in
>   the member-role daemon** (`HydraCache::member()` + the existing `hydracache-cluster-*` adapters) so
>   `source:"live"` is real, not modeled — staged (W6a in-process, W6b networked/split-able); (**W7**)
>   **mount the already-shipped read-only actuator** (`hydracache-actuator-axum`) on the admin surface
>   (the daemon mounts none today — G1); (**W8**) ship a **drift-guarded Grafana dashboard** over the
>   exposed metrics (no bundled TSDB). Closes the named positioning gap *"a thin operability surface
>   (metrics/actuator/admin API, no Management Center-style UI)."*
> - **Why (honest, verified against the code):** the primitives exist but are **incomplete, unserved,
>   or modeled**. `hydracache-server`'s `admin_status()` returns **hardcoded stubs**
>   (`leader:"local"`, `members: 0|1`, `reshard_phase:"idle"` — bootstrap.rs:267) and the runtime
>   holds a `HydraCache::local()` with **no grid at all**, so there is currently **no real cluster
>   data source** behind a Management Center. The Prometheus exporter renders only cache series while
>   `registered_metric_names()` reserves cluster + admission names. Nothing serves `/metrics`. So this
>   release is **completion + honest plumbing + serving + a UI over existing seams** — the direct
>   sibling of the `0.56` operator on the **develop-downward** thread, and the headline feature for
>   teams leaving Hazelcast Management Center.
> - **After (depends on):** `0.56` (admin HTTP surface + separate admin port), `0.48`
>   (observability + actuator + server daemon), `0.42`/`0.43`/`0.46` (the real grid control plane,
>   membership, reshard, consistency levels that W0 reads), `0.53` (the `demo/` Playwright console
>   tech), `0.51` (backup age). Independent of `0.54`/`0.55`.
> - **Blueprint:** Hazelcast **Management Center** (read-only cluster view, not a control plane);
>   TigerBeetle-style "correctness is visible" discipline already used in `0.50`/`0.53`.
> - **Status:** shipped. W0-W8 landed; W6b networked multi-daemon grid hosting is deferred as
>   [`TD-0008`](../technical-debt/TD-0008-networked-daemon-grid-hosting.md).
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> positioning: [`../POSITIONING.md`](../POSITIONING.md) ·
> competitive: [`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its
Definition of Done **and** `cargo xtask verify`; never push red. **W0 before W1** — the console/
metrics have no real data source until W0 lands.

## Preflight Audit (Codex, 0.57 start — read before touching code)

Verified by reading the code; each is a *load-bearing* fact this plan is built on:

1. **The deployable server does not run the grid.** `crates/hydracache-server/src/bootstrap.rs`:
   `ServerRuntime::new` builds `cache: HydraCache::local()` (bootstrap.rs:119) and `start()`
   (bootstrap.rs:132-146) merely flips booleans — `cluster_ready = matches!(role, Local|Member|
   Client)` is `true` for **any** role, including `Local`. There is **no** membership table, raft
   handle, or partition map in `ServerRuntime`.
2. **`admin_status()` is a placeholder.** bootstrap.rs:267-277 returns `leader:
   cluster_ready.then(|| "local".to_owned())`, `term: u64::from(cluster_ready)` (0/1), `members:
   u32::from(cluster_ready)` (0/1), `reshard_phase: "idle".to_owned()` — **hardcoded**, not observed.
   A Management Center built naïvely on this would display *fake* numbers → an R-11 violation. **This
   is why W0 exists and comes first.**
3. **The real status source exists — in the `hydracache` crate, not the server.**
   `crates/hydracache/src/cluster/control_plane.rs`: `RaftStyleMetadataControlPlane::snapshot()
   -> RaftMetadataSnapshot { term, commit_index, epoch, member_count, client_count, last_command }`
   (control_plane.rs:206-216) and `members() -> Vec<ClusterMember>` (control_plane.rs:467).
   `ClusterMember { node_id, generation, role, epoch, endpoints, metadata }`
   (`cluster/membership.rs:136-149`). The **networked** authority is
   `hydracache-cluster-raft::RaftMetadataRuntime` (used by `0.42`/`0.53.1`). W0 plumbs these; it does
   **not** invent a status model.
4. **The exporter is real but incomplete.** `crates/hydracache-observability/src/exporter.rs`
   renders only cache series; `registered_metric_names()` (exporter.rs:96-113) already reserves
   `hydracache_admission_*` and every `cluster_grid_metric_descriptors()` name
   (`crates/hydracache/src/grid/mod.rs:1032`). The grid counters are reachable via
   `HydraCache::cluster_grid_counters() -> ClusterGridCounters` (`crates/hydracache/src/cache.rs:352`,
   struct at grid/mod.rs:887) and admission via `AdmissionSnapshot` (`admission.rs:122`).
5. **Nothing serves `/metrics`.** `crates/hydracache-actuator-axum/src/lib.rs` serves read-only JSON
   only. The admin surface (`crates/hydracache-server/src/admin_http.rs`) serves
   `/healthz`/`/readyz`/`/admin/*` on a **separate internal port**: `AdminApiConfig.listen_addr`
   defaults to `127.0.0.1:9091` and `validate()` **rejects** admin == client `listen_addr`
   (`config.rs:74-83, 247-249` → `AdminAddressConflicts`). So W2's "internal, not client port" is a
   *guaranteed* boundary, not an aspiration.
6. **The console front-end + test harness already exist and use Playwright.** `demo/` ships
   `app.js`/`index.html`/`style.css`/`scenarios.js`/`share.js`, a `playwright.config.mjs`, and
   `demo/tests/{ui_smoke,seed_share}.spec.js` run by `npm test` (`package.json` → `playwright test`).
   **It is NOT wired into `cargo xtask verify`** (no xtask reference). So W4/W5 write Playwright specs
   and W5 optionally adds a **Node-gated, skip-gracefully** xtask step — the plan must not claim these
   run inside the Rust `verify` today.

## Gap Analysis (post-pause audit — holes found and closed)

A second code-grounded pass (after a work pause) found nine holes in the first draft. Each is
listed with the verified fact and the correction now folded into the work items:

- **G1 — the server does not mount the actuator.** `hydracache-server` serves **only** the
  `admin_http.rs` `Router` (grep: the sole `Router::new()` is admin_http.rs:59); it never nests
  `HydraCacheActuator`. **Fix:** W2/W3 add `/metrics` and `/cluster/overview` **directly to
  `AdminHttpSurface::routes()`** (admin_http.rs:58-67), using the exporter + read-model as
  *libraries*. The earlier "add a route to the actuator, wired into admin_http" was contradictory and
  is removed.
- **G2 — `ServerRuntime` derives `Clone` + `Debug` (bootstrap.rs:90).** A `Box<dyn
  ClusterStatusProvider>` field breaks both derives. **Fix:** the field is
  `Arc<dyn ClusterStatusProvider>` (`Arc` is `Clone`; the trait is `: Debug + Send + Sync`). W0
  sketch updated.
- **G3 — `leader` has no source in any in-memory snapshot.** `RaftMetadataSnapshot`
  (control_plane.rs:206) carries `term`/`commit_index`/`epoch`/`member_count`/`client_count` but **no
  leader**; `ClusterStagingHealth`/`ClusterPilotReport` (diagnostics.rs:329/273) have no leader
  either; `RaftStyleMetadataControlPlane` is explicitly a *simulated* metadata plane. The real leader
  lives in the networked raft runtime (raft-rs `SoftState.leader_id`; a `leader() -> Option<u64>`
  exists at `hydracache-cluster-raft/src/log_store.rs:582`). **Fix:** W0's live path sources leader
  from the raft runtime; the in-memory/modeled path returns `leader: None` (which W3's "no leader
  mid-election → null" already handles). The invented `control_plane.leader_id()` is removed from the
  in-memory impl.
- **G4 — consistency level is per-operation, not a single cluster value.** `ConsistencyLevel`
  (`grid/consistency_level.rs:15`) is chosen *per request*; the grid exposes per-level op **counters**
  (`consistency_level_operations_total`, `hydracache_op_consistency_level_total`, grid/mod.rs:953/1223)
  — there is **no** single "current CL". **Fix:** W3 shows the **configured default** CL (if any) plus
  the per-level op-count distribution, never a fabricated single "current" value (R-11).
- **G5 — no partition owner map in the diagnostics reports.** `under_replicated` is a real grid
  counter (`hydracache_under_replicated_keys`, grid/mod.rs) and `ClusterPilotReport.stamp`
  (diagnostics.rs:283) is the partition-table drift stamp, but neither report enumerates partition
  ownership. **Fix:** W3's `PartitionSummary` sources `under_replicated` from grid counters and
  partition **count** from the replication map (`EffectiveReplicationMap`, grid/mod.rs:98); owner-map
  detail is out of scope for the read-model (named, not faked).
- **G6 — backup age is per-namespace.** The real source is `snapshot_age_ms(namespace, now) ->
  Option<u64>` (`grid/durability.rs:212`). **Fix:** W3 aggregates to `backup_age_seconds` = the
  **oldest** namespace age (worst case), `None` if no namespace has a snapshot.
- **G7 — `/cluster/overview` trust tier was unspecified.** **Fix:** it is served on the **internal
  admin port** at the **same liveness tier as `/metrics`/`/healthz`** (read-only, not behind
  `require_admin`), never on the client port. Stated in W3.
- **G8 — console CORS/origin unspecified.** A browser SPA polling the admin port (9091) from another
  origin fails CORS. **Fix:** W4 serves the console **from the admin surface itself (same-origin)**;
  if served statically, W5 documents a read-only CORS allow-list for the admin port. Also: the
  console is a **distinct bundle/mode**, not the `0.53` sim lab re-pointed — only the Playwright
  *harness* and graph widgets are reused, so the teaching lab and the ops console never entangle.
- **G9 (the big one) — the deployable server does not host the grid, so live status has no real
  source in practice.** `ServerRuntime` holds `HydraCache::local()` and `start()` only flips booleans
  (bootstrap.rs:119, 132-146) — even operator-deployed pods would report `source:"modeled"`. **Fix:
  now IN scope as W6** (was a named prerequisite): the member-role daemon builds `HydraCache::member()`
  (cache.rs:137) with the control-plane + discovery adapters and feeds W0's `LiveClusterStatus`, so
  `source:"live"` is real. **Staged** — W6a (in-process member) is a self-contained win; W6b (networked
  multi-node via `hydracache-cluster-raft`/`-transport-axum`/`-chitchat`) is larger and may split into
  its own release. The `source` tag keeps the console honest whether or not W6b finishes here.

### Grid hosting is now a work item (W6), not a hidden assumption

The Management Center is only as "live" as its data source. `HydraCache::member()` (cache.rs:137) and
the networked adapters (`hydracache-cluster-raft::RaftMetadataRuntime` lib.rs:439,
`-transport-axum`, `-chitchat`, validated over live transport in
[`V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md`](V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md) Phase F + `0.44`
DST) already exist — the daemon simply never wired them (it calls `local()`). **W6 wires them.**
Because W6b (networked multi-node) is substantial, it is explicitly split-able: ship W6a + the seam and
defer W6b if it grows past the release. `source:"live|modeled"` makes that split honest at all times.

## Justification (why this, why now)

The algorithms and wire surface ship; what is missing is an **honest, served, human-facing operate
surface**. This release completes the exporter, serves it on the existing internal port, plumbs a
**real** cluster-status source into the daemon (replacing the stub), aggregates a read-model, and
renders a read-only console on the existing Playwright front-end — all over existing seams, off the
embedded fast path (R-10). It is the operability half of the `0.56` operator: the operator *acts*, the
Management Center *shows* — truthfully.

## Release Theme

A read-only, **honest** operate-in-prod surface: real cluster status behind a labelled live/modeled
seam, complete + served Prometheus metrics, one aggregated read-model, and a Management-Center-style
console — **without** becoming a control plane (writes stay on the `0.56` authz-gated admin API),
**without** a bundled TSDB, and **without** touching the embedded fast path or adding a consistency
level.

## Non-Goals

- **Not a control plane / not a write UI.** The console is **read-only**. Drain/reshard/backup/scale
  stay on the `0.56` authz-gated admin API + the operator; the UI may *deep-link* to them, never
  perform them. No new authz model.
- **Not a metrics store.** We **expose** OpenMetrics text for Prometheus/Grafana; no bundled TSDB,
  alerting, retention, or query engine.
- **Grid hosting is IN scope, but staged (W6).** `0.57` includes making the member-role daemon host
  the grid so `source:"live"` is real: **W6a** (in-process member) is firmly in scope; **W6b**
  (networked multi-node via the existing `hydracache-cluster-*` adapters) is in scope but **split-able**
  if it grows past the release. What stays out of scope is doing it "in one shot" as a hard blocker —
  the `source: live|modeled` tag keeps the console honest whether W6b lands here or in a follow-up, and
  `local`/`client` roles stay `modeled`.
- **Not on the fast path (R-10).** Metrics/console/status live in `hydracache-observability`,
  `hydracache-actuator-axum`, `hydracache-server`, and a console asset; base `hydracache` embedded
  caching is byte-for-byte unchanged. Scrape/console reads never block a cache op.
- **No new consistency level (R-1).** The console *displays* the `0.46` level; never changes it.
- **No numeric self-score (R-7).** Real counters/states and honest verdicts only — no fabricated
  "health %".

## Inherited Boundary (assumes 0.56 + 0.48 + 0.42/0.43/0.46 + 0.53 + 0.51)

- **From `0.56`:** `AdminHttpSurface` (`admin_http.rs`) on the separate admin port; `admin_status()`
  (the stub W0 replaces); admin-gated write actions (unchanged).
- **From `0.48`:** `hydracache-observability` (`HydraCacheRegistry`, `HydraCacheOverview`,
  `PrometheusExporter`, `registered_metric_names`), `hydracache-actuator-axum` read-only JSON.
- **From `0.42`/`0.43`/`0.46`:** the real control plane + membership (`RaftStyleMetadataControlPlane`,
  `RaftMetadataRuntime`, `ClusterMember`), online reshard phase, consistency levels — W0/W3 read
  these.
- **From `0.53`:** the `demo/` Playwright front-end reused for the console.
- **From `0.51`:** durable snapshot manifest → backup/checkpoint age (displayed).

## Dependency Graph

```
0.56 admin_http + admin port ─┐
0.42/0.43/0.46 grid control plane, membership, reshard, CL ─┼─► W0 ClusterStatusProvider (seam + live/modeled tag)
                                                            │        │
0.48 observability/actuator ────────────────────────────────┘        ├─► W1 complete exporter ─► W2 serve /metrics ─┐
                                                                     ├─► W3 ClusterOverview read-model ─────────────┼─► W4 read-only console ─► W5 wire-through + docs + gates
                                                                     ├─► W6 host the grid (member role) ── fills the seam so source:"live" is real (W6a in-proc; W6b networked, split-able)
                                                                     ├─► W7 mount the shipped actuator on the admin surface (closes G1 further)
                                                                     └─► W8 drift-guarded Grafana dashboard over W1 metrics (no TSDB)
0.53 demo/ Playwright front-end ─────────────────────────────────────────────────────────────────────────────────┘
HydraCache::member() (cache.rs:137) + hydracache-cluster-raft/-transport-axum/-chitchat ─► W6
hydracache-actuator-axum (already shipped, 0.48) ─► W7
```

---

## W0. Real cluster-status seam (`ClusterStatusProvider`) — the preflight that unblocks everything

**Goal.** Replace the hardcoded `admin_status()` with a **provider seam** that returns *observed*
cluster status from the grid control plane, tagged `source: live|modeled` so no consumer ever
presents modeled data as live (R-11). Default stays modeled (today's behaviour, honestly labelled);
`member` role backed by the real control-plane snapshot when a grid handle is present.

**Files.**
- new `crates/hydracache-server/src/cluster_status.rs` (the seam + two impls),
- `crates/hydracache-server/src/bootstrap.rs` (hold `Box<dyn ClusterStatusProvider>`; delegate
  `admin_status()`; add `source` to `ServerAdminStatus`),
- reads `crates/hydracache/src/cluster/control_plane.rs:206,467` +
  `crates/hydracache/src/cluster/membership.rs:136`.

**Code sketch (grounded in the real types above).**
```rust
// crates/hydracache-server/src/cluster_status.rs  (new)
use serde::Serialize;

/// Where a status reading came from. Surfaced everywhere so a modeled value is
/// never rendered as live (R-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusSource { Live, Modeled }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemberStatus {
    pub node_id: String,
    pub role: MemberRole,            // Member | Client (from ClusterRole)
    pub reachable: Reachability,     // Reachable | Suspect | Unreachable
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClusterStatus {
    pub source: StatusSource,
    pub leader: Option<String>,
    pub term: u64,
    pub epoch: u64,
    pub quorum_ok: bool,
    pub members: Vec<MemberStatus>,
    pub reshard_phase: ReshardPhase, // Idle | Planning | Moving | Finalizing
    pub draining: bool,
}

/// Read-only provider of authoritative cluster status.
pub trait ClusterStatusProvider: std::fmt::Debug + Send + Sync {
    fn cluster_status(&self) -> ClusterStatus;
}

/// Today's behaviour, honestly tagged. Used when the daemon hosts no grid.
#[derive(Debug, Clone)]
pub struct ModeledClusterStatus { pub ready: bool, pub draining: bool }

impl ClusterStatusProvider for ModeledClusterStatus {
    fn cluster_status(&self) -> ClusterStatus {
        ClusterStatus {
            source: StatusSource::Modeled,
            leader: self.ready.then(|| "local".to_owned()),
            term: u64::from(self.ready),
            epoch: 0,
            quorum_ok: self.ready && !self.draining,
            members: Vec::new(),
            reshard_phase: ReshardPhase::Idle,
            draining: self.draining,
        }
    }
}

/// Backed by the real grid control plane + membership + the raft runtime.
/// NOTE (G3): `leader` comes from the raft runtime's soft-state, NOT the in-memory
/// control-plane snapshot (which has term/epoch but no leader). NOTE (G2): held as
/// `Arc<dyn ...>` in ServerRuntime so `#[derive(Clone)]` still holds.
#[derive(Debug)]
pub struct LiveClusterStatus<C> { grid: C /* : GridControlPlaneHandle */ }

impl<C: GridControlPlaneHandle> ClusterStatusProvider for LiveClusterStatus<C> {
    fn cluster_status(&self) -> ClusterStatus {
        let snap = self.grid.snapshot();            // RaftMetadataSnapshot (control_plane.rs:206) — term/epoch/member_count
        let members = self.grid.members()           // Vec<ClusterMember> (control_plane.rs:467)
            .into_iter()
            .map(|m| MemberStatus {
                node_id: m.node_id.to_string(),
                role: MemberRole::from(m.role),
                reachable: self.grid.reachability(&m.node_id),
                generation: m.generation.into(),
            })
            .collect::<Vec<_>>();
        ClusterStatus {
            source: StatusSource::Live,
            leader: self.grid.raft_leader_id(),     // raft-rs SoftState.leader_id (hydracache-cluster-raft) — None mid-election
            term: snap.term,
            epoch: snap.epoch.into(),
            quorum_ok: self.grid.has_quorum(),
            members,
            reshard_phase: self.grid.reshard_phase(),
            draining: self.grid.is_draining(),
        }
    }
}

// GridControlPlaneHandle adapts the real types. `raft_leader_id` is the ONLY member
// whose value cannot come from the in-memory control plane (G3) — it needs the raft
// runtime; a modeled/handle-less deployment returns None there and stays `modeled`.
pub trait GridControlPlaneHandle: std::fmt::Debug + Send + Sync {
    fn snapshot(&self) -> RaftMetadataSnapshot;         // control_plane.rs:206
    fn members(&self) -> Vec<ClusterMember>;            // control_plane.rs:467
    fn raft_leader_id(&self) -> Option<String>;         // raft runtime soft-state (G3)
    fn has_quorum(&self) -> bool;
    fn reachability(&self, node: &ClusterNodeId) -> Reachability;
    fn reshard_phase(&self) -> ReshardPhase;
    fn is_draining(&self) -> bool;
}
```
```rust
// crates/hydracache-server/src/bootstrap.rs  (delegate, add `source`)
pub struct ServerRuntime {
    // ...existing fields...
    // Arc, NOT Box (G2): ServerRuntime derives Clone + Debug (bootstrap.rs:90).
    cluster_status: Arc<dyn ClusterStatusProvider>,  // Modeled by default; Live only with a real grid handle
}

pub fn admin_status(&self) -> ServerAdminStatus {
    let s = self.cluster_status.cluster_status();
    ServerAdminStatus {
        leader: s.leader,
        term: s.term,
        quorum_ok: s.quorum_ok,
        members: s.members.len() as u32,
        reshard_phase: s.reshard_phase.to_string(),
        draining: s.draining,
        source: s.source,           // NEW field — honesty travels with the data
    }
}
```

**Steps.**
1. Add `cluster_status.rs` with `ClusterStatusProvider`, `ClusterStatus`, `MemberStatus`,
   `StatusSource`, `ReshardPhase`, `MemberRole`, `Reachability`; a `GridControlPlaneHandle` adapter
   trait over the real `RaftStyleMetadataControlPlane`/`RaftMetadataRuntime` (snapshot/members/leader/
   quorum/reshard/reachability).
2. `ServerRuntime` holds `Arc<dyn ClusterStatusProvider>` (G2 — Arc keeps `#[derive(Clone)]`/`Debug`
   at bootstrap.rs:90), defaulting to `ModeledClusterStatus`; `start()` installs `LiveClusterStatus`
   **only** when a real grid handle is available for the role (G9 — absent one, the daemon stays
   modeled), else stays modeled. `admin_status()` delegates and carries `source`. `raft_leader_id`
   (G3) is the one field the in-memory control plane cannot supply — it comes from the raft runtime.
3. Add `source: StatusSource` to `ServerAdminStatus` (bootstrap.rs:47-62) — a COMPAT change to the
   admin status JSON; register in `docs/COMPAT.md` (R-4).

**Corner-case scenarios (each an explicit test).**
- **Modeled honesty:** a `Local`-role daemon reports `source:"modeled"`, `members:[]`, `leader:
  "local"` — and the value is *never* labelled live. (Prevents the R-11 violation the audit found.)
- **Live members:** a member-role runtime with a 3-node control plane reports `source:"live"`, three
  `MemberStatus`, `term`/`epoch` from `snapshot()`.
- **No leader mid-election:** control plane with no leader → `leader: None` (not a stale id).
- **Unreachable member shown not omitted:** a suspect/partitioned node appears with
  `reachable: Unreachable`, still counted in `members`.
- **Draining:** `is_draining()` reflects in both `draining` and `quorum_ok` exactly as today.

**DoD.** `crates/hydracache-server/tests/cluster_status.rs`
- `modeled_status_is_tagged_modeled_and_never_live`.
- `live_status_reports_real_members_term_and_epoch`.
- `no_leader_during_election_is_none_not_stale`.
- `unreachable_member_is_present_with_unreachable_flag`.
- `admin_status_json_includes_source_field` (COMPAT shape).
- Run: `cargo test -p hydracache-server --locked cluster_status`.

**Risk & rollback.** The load-bearing property is **live/modeled honesty** — gated by
`modeled_status_is_tagged_modeled_and_never_live`. Full networked-grid-in-daemon is *not* required:
absent a grid handle the provider stays modeled and the console shows it as such. Revert restores the
stub `admin_status()`; nothing downstream can be built truthfully, which is exactly why W0 is first.

## W1. Complete the Prometheus exporter (emit what it reserves + topology gauges)

**Goal.** Make `PrometheusExporter` emit the **admission** + **cluster-grid** series it reserves, plus
bounded-label **topology gauges** sourced from W0 — so `registered_metric_names()` is the single
source of truth and the reserve-vs-render gap is closed.

**Files.** `crates/hydracache-observability/src/exporter.rs` (extend `render_overview`), input types
in `src/lib.rs`; sources: `HydraCache::cluster_grid_counters()` (`cache.rs:352`, `ClusterGridCounters`
grid/mod.rs:887), `cluster_grid_metric_descriptors()` (grid/mod.rs:1032), `AdmissionSnapshot`
(`admission.rs:122`), and the W0 `ClusterStatus`.

**Code sketch.**
```rust
// exporter.rs — drive names + types from the descriptor table so they can't drift.
for d in cluster_grid_metric_descriptors() {           // grid/mod.rs:1032
    write_header(&mut out, d.name, counter_or_gauge(d.name), d.help());
    push_labeled(&mut out, d.name, d.labels, counters.value_for(d.name));  // ClusterGridCounters
}
// admission (names already in registered_metric_names())
push_gauge(&mut out, "hydracache_admission_in_flight",   admission.in_flight);
push_gauge(&mut out, "hydracache_admission_queue_depth", admission.queue_depth);
push_counter(&mut out, "hydracache_admission_rejected_total", admission.rejected_total);
// topology gauges from W0 ClusterStatus (bounded labels only — R-6)
push_gauge(&mut out, "hydracache_cluster_members",   status.members.len() as u64);
push_gauge_labeled(&mut out, "hydracache_cluster_leader", &[("node", leader_label(&status))], 1);
push_gauge(&mut out, "hydracache_cluster_epoch",     status.epoch);
```

**Steps.**
1. Emit admission series from `AdmissionSnapshot`.
2. Emit cluster-grid series **driven by the descriptor table** (name/type/labels from the descriptor,
   value from `ClusterGridCounters`) so exporter and descriptors cannot drift.
3. Add topology gauges from W0 (`hydracache_cluster_members`, `_leader{node}`, `_epoch`,
   `_reshard_phase`, `hydracache_backup_age_seconds`) — labelled only by **bounded** dimensions
   (node id / phase enum), never unbounded keys (R-6).
4. Keep valid OpenMetrics (one HELP/TYPE per name; reuse `escape_label`).

**Corner-case scenarios.**
- **Reserve-vs-render:** every name in `registered_metric_names()` appears in the output (the exact
  gap the audit found — a missing emitter fails the test).
- **Bounded labels:** a would-be per-key label is rejected/bucketed (R-6).
- **Zero state:** zero caches / zero members render valid, panic-free, empty-value exposition.
- **Modeled source:** when W0 reports `modeled`, topology gauges carry a `source="modeled"` label so
  scrapes are self-describing.

**DoD.** `crates/hydracache-observability/tests/exporter.rs` (extend)
- `every_registered_metric_name_is_emitted` (iterates `registered_metric_names()`).
- `admission_and_cluster_series_render_with_type_headers`.
- `exporter_labels_are_bounded` (R-6).
- `empty_registry_and_zero_members_render_valid_exposition`.
- `topology_gauges_carry_source_label`.
- Run: `cargo test -p hydracache-observability --locked exporter`.

**Risk & rollback.** Metric names are a compat surface → register the set in `docs/COMPAT.md` (R-4).
Revert drops the new series; cache metrics unchanged.

## W2. Serve `/metrics` (OpenMetrics) on the internal admin port

**Goal.** Expose the completed exporter as `GET /metrics` on the **separate internal admin port**
(`AdminApiConfig.listen_addr`, default `127.0.0.1:9091`) — read-only, unauthenticated on that internal
port (same trust tier as `/healthz`), stable content-type, served even while draining.

**Files (G1 — the server does not mount the actuator; add the route to the served admin surface).**
`crates/hydracache-server/src/admin_http.rs` — add `/metrics` **directly to
`AdminHttpSurface::routes()`** (admin_http.rs:58-67) next to `/healthz`/`/readyz`, using
`PrometheusExporter` (`hydracache-observability`) as a **library**. Do **not** try to nest
`hydracache-actuator-axum` into the daemon — it is not wired in and is out of scope here.

**Code sketch.**
```rust
// admin_http.rs — /metrics sits with the liveness-tier routes, NOT behind require_admin.
.route("/metrics", get(metrics))
// ...
async fn metrics(State(rt): State<SharedServerRuntime>) -> Response {
    let text = rt.lock().expect("server runtime mutex").render_metrics(); // PrometheusExporter::render_overview
    ([(CONTENT_TYPE, "text/plain; version=0.0.4")], text).into_response()
}
```

**Steps.**
1. Add `GET /metrics` returning exporter text with `Content-Type: text/plain; version=0.0.4`.
2. Mount on the admin `routes()` (admin_http.rs:58-67), a **sibling of `/healthz`** (not behind
   `require_admin`, since scrapers send no admin identity); write actions stay gated (unchanged).
3. It cannot reach the client port: `config.rs:247-249` already rejects admin == client
   `listen_addr`; add a test asserting the client surface exposes no `/metrics`.

**Corner-case scenarios.**
- **During drain:** `/readyz` is 503 but `/metrics` is **200** (observability during drain required).
- **Trust boundary:** `/metrics` present on admin port, absent on client port.
- **Content-type:** stable `text/plain; version=0.0.4`; odd `Accept` still returns valid text.
- **No fast-path coupling:** render is a snapshot; a large registry does not block cache ops.

**DoD.** `crates/hydracache-server/tests/admin_http.rs` (extend)
- `metrics_endpoint_serves_prometheus_text_with_stable_content_type`.
- `metrics_endpoint_is_served_during_drain`.
- `metrics_endpoint_is_not_on_the_client_port`.
- Run: `cargo test -p hydracache-server --locked admin_http`.

**Risk & rollback.** Endpoint placement (internal vs client) is the load-bearing decision, tested
explicitly. Revert removes the route; the exporter library stays.

## W3. Cluster read-model (`ClusterOverview` — one aggregated read-only snapshot)

**Goal.** One read-only JSON the console renders in a single poll: members (from W0), leader/term/
epoch (from `RaftMetadataSnapshot`), consistency level, backup/checkpoint age, reshard/upgrade phase —
plus the `source` tag. A **view**, not a new source of truth.

**Files (G1/G7).** `crates/hydracache-observability/src/lib.rs` (`ClusterOverview` + assembler),
served as `GET /cluster/overview` **on the admin surface** (`admin_http.rs` `routes()`, same internal
port + liveness tier as `/metrics` — read-only, **not** behind `require_admin`, never on the client
port), fed by the W0 provider + `HydraCacheRegistry`. (Not the actuator crate — the daemon doesn't
mount it, G1.)

**Code sketch (stable serde shape — a display contract, COMPAT-registered; sources corrected per G3-G6).**
```rust
#[derive(Debug, Clone, Serialize)]
pub struct ClusterOverview {
    pub source: StatusSource,                     // live | modeled — travels to the UI (W0)
    pub members: Vec<MemberView>,                 // id, role, reachable (W0 provider)
    pub leader: Option<LeaderView>,               // id (from raft soft-state, G3), term, epoch — None mid-election
    pub partitions: PartitionSummary,             // under_replicated (grid counter), count (EffectiveReplicationMap) — G5
    pub consistency: ConsistencyView,             // configured default + per-level op counts, NOT a single "current" (G4)
    pub backup_age_seconds: Option<u64>,          // oldest namespace snapshot_age_ms (durability.rs:212), None if none (G6)
    pub lifecycle: LifecycleView,                 // reshard_phase, upgrade_phase
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsistencyView {                      // G4: CL is per-op, so show honest aggregates
    pub configured_default: Option<String>,       // the configured default level, if any
    pub op_counts_by_level: Vec<(String, u64)>,   // from consistency_level_operations_total
}
```

**Steps.**
1. Define `ClusterOverview` (stable field names; register in `docs/COMPAT.md`, R-4).
2. Assemble read-only from: W0 `ClusterStatus` (members/leader/term/epoch/reshard/source); grid
   counters for `under_replicated` + per-level op counts (`cluster_grid_counters()`, cache.rs:352,
   G4/G5); `EffectiveReplicationMap` for partition count (grid/mod.rs:98, G5); `snapshot_age_ms`
   aggregated to the **oldest** namespace for `backup_age_seconds` (durability.rs:212, G6); the
   `registry` staging/pilot snapshots for auxiliary counters. **No** new cluster RPC, **no** mutation.
3. Serve `GET /cluster/overview` on the admin surface; document it as a **point-in-time view** (R-11),
   not a linearizable read; the `source` tag rides along.

**Corner-case scenarios.**
- **Unreachable member** shown (`reachable:false`), not omitted.
- **No leader mid-election** → `leader: null` + a phase, never a stale leader (G3 makes this the
  *normal* case for the in-memory/modeled path).
- **Consistency (G4):** the view shows a per-level op-count distribution + configured default, and
  **never** a single fabricated "current CL".
- **Mid-reshard/upgrade** → the phase is surfaced.
- **Embedded / modeled** cache → `source:"modeled"`, `members:[]`, `leader:null`, no panic.
- **Backup never taken** → `backup_age_seconds: null` (not `0`); with multiple namespaces, the
  **oldest** age is reported (worst case) (G6).

**DoD.** `crates/hydracache-server/tests/cluster_overview.rs` (served on the admin surface, G1)
- `cluster_overview_aggregates_members_leader_partitions_consistency_backup_and_lifecycle`.
- `unreachable_member_is_shown_not_omitted`; `no_leader_during_election_is_null`.
- `consistency_is_distribution_not_single_current` (G4).
- `backup_age_is_oldest_namespace_or_null` (G6).
- `modeled_source_is_carried_through_to_overview`.
- `cluster_overview_is_on_admin_port_not_client_port` (G7).
- Snapshot-test the JSON shape (COMPAT stability).
- Run: `cargo test -p hydracache-server --locked cluster_overview`.

**Risk & rollback.** JSON shape is a compat surface (the console binds to it) → snapshot-tested +
COMPAT-registered. Revert removes the view; underlying snapshots stay.

## W4. Read-only Management Center web console (on the existing Playwright front-end)

**Goal.** A read-only SPA that polls `/cluster/overview` and `/metrics` (both on the admin surface,
G1) and renders a live cluster view — topology graph (members + roles + reachability), leader/partition panel,
consistency level, backup age, reshard/upgrade phase, a metrics strip — with an explicit **read-only**
stance and a visible **`source: live|modeled`** banner (never paint modeled as live).

**Files (G8 — distinct bundle, same-origin).** a **separate** `console/` bundle (or a distinct
`console` mode in the `demo/` project — **not** the `0.53` sim lab re-pointed; only the Playwright
harness + graph/polling widgets are reused, so the teaching lab and the ops console never entangle),
served **from the admin surface itself** so the SPA is **same-origin** with `/cluster/overview` +
`/metrics` (avoids CORS on port 9091, G8). New Playwright specs in `console/tests/` (harness pattern
from `demo/`). A `console/README.md` fidelity note (like `demo/README.md`).

**Same-origin note (G8).** The console reads the admin port (default `127.0.0.1:9091`). Serve
`GET /console` (static bundle) from the admin `routes()` so browser fetches to `/cluster/overview` and
`/metrics` are same-origin; if operators instead host the bundle elsewhere, W5 documents a read-only
CORS allow-list for the admin port. Operators typically reach the loopback admin port via
`kubectl port-forward` (or the operator's admin Service).

**Code sketch (Playwright spec, matching the existing `demo/tests/*.spec.js` style).**
```js
// console/tests/console_readonly.spec.js
import { test, expect } from '@playwright/test';

test('console renders live cluster overview and is read-only', async ({ page }) => {
  await page.route('**/cluster/overview', (r) => r.fulfill({ json: liveOverviewFixture }));
  await page.goto('/console.html');
  await expect(page.getByTestId('source-badge')).toHaveText(/live/i);
  await expect(page.getByTestId('member')).toHaveCount(3);
  await expect(page.getByTestId('leader')).toContainText('node-1');
  // read-only: no mutate controls exist
  await expect(page.getByRole('button', { name: /drain|reshard|delete/i })).toHaveCount(0);
});

test('modeled source is shown as modeled, never live', async ({ page }) => {
  await page.route('**/cluster/overview', (r) => r.fulfill({ json: modeledOverviewFixture }));
  await page.goto('/console.html');
  await expect(page.getByTestId('source-badge')).toHaveText(/modeled/i);
});
```

**Steps.**
1. Reuse the `0.53` force-directed graph + snapshot-polling loop, pointed at the **real** server read
   endpoints (config: admin base URL), not the sim.
2. Panels: members (role/reachability colours as in the lab), leader/partitions, consistency level,
   backup age, reshard/upgrade phase; a `/metrics` strip (hit ratio, admission rejects, queue depth).
3. **Read-only by construction:** no mutate controls; where an action is relevant, show the exact
   `0.56` authz-gated admin API call an operator would run — never perform it. A visible banner states
   read-only; the `source` badge shows live/modeled.
4. Degrade honestly: server unreachable → "cannot reach cluster", never a stale-green view.

**Corner-case scenarios (Playwright specs).**
- **Read-only:** zero mutate controls in the DOM.
- **Live vs modeled:** the `source` badge matches the payload; modeled is never shown as live (the
  end-to-end guard of the W0/R-11 property).
- **Unreachable server:** explicit degraded state, not fake healthy.
- **One node / no leader:** renders correctly (leader panel shows "electing").
- **Large cluster:** bounded render (cap rendered members/partitions like the lab's
  `MAX_IN_FLIGHT_RENDERED`); no unbounded DOM growth.

**DoD.** `console/tests/console_readonly.spec.js` (+ fixtures)
- `console_renders_live_cluster_overview_from_endpoints`.
- `console_is_read_only_no_mutate_controls`.
- `modeled_source_is_shown_as_modeled_never_live`.
- `console_shows_degraded_state_when_server_unreachable`.
- `console_render_is_bounded_for_large_clusters`.
- Run: `cd console && npm test` (Playwright). **Note:** this runs under Node/Playwright, *not* inside
  `cargo xtask verify` today (audit item 6); W5 decides the gate wiring.

**Risk & rollback.** Front-end honesty (don't paint green when blind; don't show modeled as live) is
load-bearing → the degraded-state + source specs. Revert removes the console asset; metrics +
read-model remain usable by Prometheus/Grafana directly.

## W5. Wire-through, docs, and gates (cross-cutting)

**Goal.** End-to-end: a running daemon serves `/metrics` + `/cluster/overview` on its internal port
with a real (or honestly-modeled) `source`, and the console renders it; document the surface + trust
boundary; register compat; decide the console-test gate; pass release gates.

**Files.** `docs/management-center.md` (console + metrics reference, trust boundary, `source`
semantics incl. the **G9 named prerequisite** — when the console shows `modeled` vs `live` and why,
"observe" day-2 runbook, Prometheus scrape config, read-only CORS note (G8), deep-links to the `0.56`
admin write API), `docs/COMPAT.md` (metric-name set + `ServerAdminStatus.source` + `ClusterOverview`
shape), `README.md` (one line + screenshot), `console/README.md` fidelity note, and a **Node-gated,
skip-gracefully** console-test step in `crates/xtask` (runs `npm test` in `console/` only if Node is
present; skips loud-but-green otherwise — mirrors the `0.56` Docker/kind skip pattern).

**Steps.**
1. Server integration test: boot the daemon, scrape `/metrics`, GET `/cluster/overview`, assert both
   on the internal port and absent on the client port, and that `source` is present.
2. Docs: live vs point-in-time; where writes live (authz-gated admin API/operator); how to scrape;
   the read-only + live/modeled honesty stance (R-11).
3. xtask: add the optional Node-gated console-test step; `cargo xtask verify` stays green without Node
   (skips), runs the Playwright specs when Node is available.

**Corner-case scenarios.**
- **No Node:** `cargo xtask verify` **skips** the console specs cleanly (green, logged), never fails
  for a missing toolchain.
- **With Node:** the Playwright specs run and gate.
- **Compat drift:** a change to `ServerAdminStatus`/`ClusterOverview`/metric-name set without a
  `docs/COMPAT.md` update fails `doc-check`/COMPAT (R-4).

**DoD.**
- `crates/hydracache-server/tests/deploy_smoke.rs` (extend):
  `daemon_serves_metrics_and_cluster_overview_with_source_on_internal_port`.
- `docs/management-center.md` present; `docs/COMPAT.md` updated; `cargo xtask verify` green (console
  specs skip without Node).
- Run: `cargo xtask verify` (+ `cd console && npm test` where Node is present).

**Risk & rollback.** Cross-cutting; each prior W is independently revertible. The whole feature is
opt-in and off the fast path.

## W6. Host the real grid in `hydracache-server` (member role) — close G9

**Goal.** Make `source:"live"` *real in production*: when `role == Member`, the daemon builds a
**grid-mode** `HydraCache` bound to the networked control-plane + discovery adapters (instead of
`HydraCache::local()`), and exposes that handle to W0's `LiveClusterStatus` so members / leader / term
/ epoch / reshard are **observed**, not modeled. This closes G9 — the reason the console could only
ever show `modeled` for the deployable server.

**Honest scope & risk (read first).** This is the **largest** item in `0.57` and is **staged** so it
lands safely and can be split if it grows:
- **W6a (in-process member — always in scope):** for `role == Member`, build via `HydraCache::member()`
  (cache.rs:137) with the **in-process** control plane (`RaftStyleMetadataControlPlane`,
  control_plane.rs) and hand its `snapshot()`/`members()` to `LiveClusterStatus`. This makes a
  single member's status **live and real** (real member table/epoch/term), replacing the boolean stub
  — a self-contained, testable win.
- **W6b (networked member — in scope, may split if it dwarfs the release):** wire the **networked**
  adapters — `hydracache-cluster-raft::RaftMetadataRuntime` (lib.rs:439) as the
  `Arc<dyn ClusterControlPlane>` (runtime.rs:191) + `hydracache-cluster-transport-axum` listener on
  `config.cluster_addr` + `hydracache-cluster-chitchat` discovery over `config.seeds` — so a real
  multi-node cluster forms and `raft_leader_id` (G3) is a true elected leader. These adapters exist
  and were validated over live networked transport in
  [`V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md`](V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md) (Phase F) and
  the `0.44` DST — but wiring them end-to-end into the daemon (with `0.48` TLS, `storage_dir`, seed
  discovery, graceful drain) is substantial. **If W6b grows past the release, ship W6a + the seam and
  split W6b into its own release; the `source` tag keeps that honest.**

**Files.**
- `crates/hydracache-server/src/bootstrap.rs` (`ServerRuntime::new`/`start`: build `member()` for
  `role == Member` instead of `local()` at bootstrap.rs:119/133; hold the grid handle),
- new `crates/hydracache-server/src/grid_host.rs` (constructs the control-plane + discovery adapters
  from `ServerConfig` and implements the W0 `GridControlPlaneHandle`),
- `crates/hydracache-server/Cargo.toml` (add `hydracache-cluster-raft`,
  `hydracache-cluster-transport-axum`, `hydracache-cluster-chitchat` — member-only deps),
- reads `crates/hydracache/src/cluster/runtime.rs:191,207` (builder seams),
  `crates/hydracache-cluster-raft/src/lib.rs:116,439`.

**Code sketch (grounded in the real builder seams).**
```rust
// crates/hydracache-server/src/grid_host.rs  (new)
use std::sync::Arc;
use hydracache::{HydraCache, cluster::{ClusterControlPlane, ClusterDiscovery}};

/// Build the grid-mode cache + the status handle for a member-role daemon.
pub fn build_member(config: &ServerConfig) -> (HydraCache, Arc<dyn GridControlPlaneHandle>) {
    // W6a: in-process control plane — real member table/epoch/term, single process.
    // W6b: swap in the networked adapters (feature/roll-forward) without changing this shape.
    let control_plane: Arc<dyn ClusterControlPlane> = networked_or_in_process(config);
    let discovery:     Arc<dyn ClusterDiscovery>    = discovery_for(config);   // chitchat over seeds (W6b)

    let cache = HydraCache::member()                 // cache.rs:137
        .cluster(config.cluster_name())
        .control_plane(Arc::clone(&control_plane))   // runtime.rs:191
        .discovery(discovery)                        // runtime.rs:207
        .build();

    let handle = Arc::new(GridHandle::new(control_plane /*, raft runtime for leader (G3) */));
    (cache, handle)
}
```
```rust
// bootstrap.rs — member role hosts the grid; local/client unchanged.
let (cache, status): (HydraCache, Arc<dyn ClusterStatusProvider>) = match config.role {
    ServerRole::Member => {
        let (cache, grid) = grid_host::build_member(&config);
        (cache, Arc::new(LiveClusterStatus::new(grid)))     // source: Live (W0)
    }
    _ => (HydraCache::local().build(), Arc::new(ModeledClusterStatus::default())), // source: Modeled
};
```

**Steps.**
1. **W6a:** for `role == Member`, build `HydraCache::member()` with the in-process control plane;
   implement `GridControlPlaneHandle` over `RaftStyleMetadataControlPlane::snapshot()`/`members()`;
   install `LiveClusterStatus`. `admin_status()`/`/cluster/overview` now report `source:"live"` for a
   member.
2. **W6b:** construct the networked `RaftMetadataRuntime` (raft control plane) + cluster transport on
   `cluster_addr` + chitchat discovery over `seeds`; `raft_leader_id` comes from the raft soft-state
   (G3). Respect `0.48` TLS + `storage_dir`; drain via the existing `graceful_shutdown` (0.56).
3. `local`/`client` roles are **unchanged** and stay `modeled` (honest).

**Corner-case scenarios.**
- **Member single-node (W6a):** reports `source:"live"`, one real member, `leader` may be self or
  `None` (in-process) — never the stub `"local"`.
- **Member multi-node (W6b):** three daemons over `cluster_addr`/`seeds` form a cluster; overview
  shows three members + one elected leader; a killed leader → `leader` transitions to `None` then the
  new elected id (ties W3's no-stale-leader case, now against a *real* election).
- **Local/client role:** stays `modeled`, byte-for-byte as before (no regression).
- **Member without `storage_dir`/`seeds`:** already rejected loud by `config.validate()`
  (config.rs:218-223) — surfaced, not silently degraded.
- **Grid start failure (W6b):** fails loud at `start()`, never a fake-ready member (R-3).

**DoD.** `crates/hydracache-server/tests/grid_host.rs`
- `member_role_reports_live_source_with_real_member_table` (W6a).
- `local_and_client_roles_stay_modeled` (no regression).
- `member_without_storage_or_seeds_is_rejected_loud`.
- `multi_node_members_form_a_cluster_and_elect_one_leader` (W6b; may be `#[ignore]`/network-gated,
  skip-gracefully like the `0.56` kind rows if the networked wiring is deferred).
- Run: `cargo test -p hydracache-server --locked grid_host`.

**Risk & rollback.** W6b is the load-bearing risk — if it grows past the release, ship **W6a + the
seam** and split W6b; the `source` tag keeps the console honest either way. Revert leaves the daemon
`modeled` (the pre-0.57 behaviour) with the rest of the Management Center still serving truthfully.

## W7. Mount the already-shipped read-only actuator on the admin surface

**Goal.** Expose the `0.48` read-only actuator JSON (`hydracache-actuator-axum`) on the internal admin
surface so operators and the console get per-cache diagnostics the daemon does **not** expose today
(G1: `hydracache-server` mounts no actuator — only `admin_http.rs`). This is a cheap, purely-additive
win that complements the W3 aggregated `ClusterOverview` with granular per-cache detail.

**Files.**
- `crates/hydracache-server/src/admin_http.rs` (nest the actuator router under the admin surface),
- `crates/hydracache-server/src/bootstrap.rs` (build a `HydraCacheRegistry` from the runtime cache;
  add a cache accessor if needed),
- `crates/hydracache-server/Cargo.toml` (+ `hydracache-actuator-axum`, `hydracache-observability`),
- reuses the shipped routes in `crates/hydracache-actuator-axum/src/lib.rs`
  (`/health`, `/caches`, `/caches/{name}/diagnostics`, `/caches/{name}/stats`, `/correctness`, …).

**Code sketch.**
```rust
// admin_http.rs — nest the read-only actuator on the same internal port + liveness tier as /metrics.
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_observability::HydraCacheRegistry;

pub fn routes(&self) -> Router {
    let registry = HydraCacheRegistry::new().with_cache("main", self.cache()); // from ServerRuntime
    Router::new()
        .route(ADMIN_HEALTHZ_PATH, get(healthz))
        // … existing admin routes + /metrics (W2) + /cluster/overview (W3) …
        .nest("/actuator/hydracache", HydraCacheActuator::routes_for(registry)) // read-only, not require_admin
        .with_state(Arc::clone(&self.runtime))
}
```

**Steps.**
1. Build a `HydraCacheRegistry` from the runtime cache (`HydraCacheRegistry::new().with_cache("main",
   cache)`); add a `ServerRuntime::cache()` accessor if one is not already present.
2. `.nest("/actuator/hydracache", HydraCacheActuator::routes_for(registry))` on the admin router —
   read-only, **same internal port + liveness tier** as `/metrics` (not behind `require_admin`),
   never on the client port.
3. Document the split: `/cluster/overview` (W3) is the **aggregated console view**; the actuator
   routes are the **granular per-cache diagnostics**. No mutation on either (Non-Goals).

**Corner-case scenarios.**
- **During drain:** actuator JSON still served (like `/metrics`, W2).
- **Trust boundary:** present on the admin port, absent on the client port.
- **Embedded / no cluster:** `/cluster/staging-health` and `/cluster/pilot-report` return clean empty
  results, no panic (the actuator already handles this).
- **Unknown cache name:** `404` (existing actuator behaviour, `actuator_routes_return_not_found_for_unknown_cache`).

**DoD.** `crates/hydracache-server/tests/actuator_mount.rs`
- `actuator_json_is_served_on_the_admin_port`.
- `actuator_is_absent_on_the_client_port`.
- `actuator_is_served_during_drain`.
- Run: `cargo test -p hydracache-server --locked actuator_mount`.

**Risk & rollback.** Purely additive read-only surface reusing a shipped crate. Revert removes the
`.nest(...)`; `/metrics` + `/cluster/overview` remain.

## W8. Ship a Grafana dashboard artifact over the exposed metrics (drift-guarded)

**Goal.** Ship a versioned **Grafana dashboard** (and optional Prometheus alert rules) over the W1
metrics so consumers get a Management-Center-grade view in their existing Grafana without hand-building
panels — **no bundled TSDB** (Non-Goal preserved: we expose metrics, Grafana/Prometheus store them).
A guard test keeps the dashboard from drifting away from the exporter.

**Files.**
- `docs/observability/dashboards/hydracache-overview.json` (Grafana dashboard),
- optional `docs/observability/alerts.yml` (Prometheus rules: under-replicated, no-leader,
  backup-age, admission-reject-rate),
- guard test `crates/hydracache-observability/tests/dashboard_metrics.rs`,
- `docs/management-center.md` (import instructions; link from W5).

**Steps.**
1. Author panels using **only** metric names emitted by W1 (`registered_metric_names()` +
   topology gauges): hit ratio, admission rejects / queue depth, cluster members / leader / epoch,
   `under_replicated`, backup age.
2. **Drift guard (ties W8 ↔ W1):** the test parses the dashboard JSON, extracts every metric name
   referenced in panel `expr` (PromQL), and asserts each is in
   `hydracache_observability::registered_metric_names()` — a dashboard panel referencing a
   non-existent series **fails loud**. (A registered metric with no panel is allowed; the dashboard
   need not cover everything, but everything it references must exist.)
3. Docs: `docs/management-center.md` links the dashboard + how to import; alert rules optional.

**Code sketch (the drift guard).**
```rust
// crates/hydracache-observability/tests/dashboard_metrics.rs
use hydracache_observability::registered_metric_names;

#[test]
fn dashboard_only_references_registered_metrics() {
    let json = std::fs::read_to_string("../../docs/observability/dashboards/hydracache-overview.json").unwrap();
    let registered = registered_metric_names();
    for metric in extract_promql_metric_names(&json) {
        assert!(
            registered.contains(metric.as_str()),
            "dashboard references '{metric}' which the exporter does not emit (W1)"
        );
    }
}
```

**Corner-case scenarios.**
- **Panel references an unknown metric** → guard fails loud (the whole point).
- **A new metric added in W1 with no panel** → allowed (guard is one-directional: referenced ⇒ exists).
- **Empty/zero cluster** → dashboard panels render "no data", not fabricated values (Grafana-side).

**DoD.** `crates/hydracache-observability/tests/dashboard_metrics.rs`
- `dashboard_only_references_registered_metrics`.
- Run: `cargo test -p hydracache-observability --locked dashboard_metrics`.

**Risk & rollback.** Docs/artifact plus a guard test; no runtime code path. Revert removes the
dashboard + guard.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green (fmt, clippy, tests, doc-check, COMPAT, deny); console Playwright specs
  run under Node and **skip-gracefully** without it.
- **Honesty (R-11):** every status/metric/overview carries a `source: live|modeled` tag; the console
  never renders modeled data as live (W0 + W3 + W4 gates). The stub `admin_status()` is gone.
- **Grid hosting (W6, closes G9):** the **member-role** daemon reports `source:"live"` with a **real**
  member table (W6a); `local`/`client` stay `modeled` with no regression. W6b (networked multi-node
  election) either lands (real leader across nodes) or is split with the seam shipped and the `source`
  tag keeping it honest — never a faked `live`.
- **Actuator + dashboard (W7/W8):** the shipped read-only actuator is mounted on the admin surface
  (further closes G1), served during drain, absent on the client port; a Grafana dashboard ships as an
  artifact whose panels are **drift-guarded** against `registered_metric_names()` (a panel referencing
  a non-emitted series fails the gate). No bundled TSDB; no mutation.
- The exporter **emits every name it reserves** — the reserve-vs-render gap is closed and gated (W1).
- `/metrics` is served on the **internal admin port** (not the client port), including **during
  drain** (W2); metric names + `ServerAdminStatus.source` + `ClusterOverview` shape registered in
  `docs/COMPAT.md` (R-4).
- `ClusterOverview` is an honest read-only **view** — unreachable members shown not omitted, no stale
  leader mid-election, `null` backup age when never taken (W3).
- The console is **read-only** (no mutate controls), degrades honestly when unreachable, renders
  bounded for large clusters (W4). Writes stay on the `0.56` authz-gated admin API / operator.
- **Gap-analysis fixes landed (G1-G9):** routes on the admin surface not the un-mounted actuator
  (G1); `Arc<dyn>` keeps `ServerRuntime: Clone` (G2); `leader` from raft soft-state, `None` on the
  modeled path (G3); consistency shown as a distribution not a single value (G4); partitions from grid
  counters + replication map (G5); backup age = oldest namespace, `null` if none (G6);
  `/cluster/overview` on the internal port at liveness tier (G7); same-origin console / documented
  CORS (G8); and the **G9 named prerequisite** (server-hosts-grid) documented, not faked.
- Embedded fast path byte-for-byte unchanged (R-10); no new consistency level (R-1); metrics
  bounded-label (R-6); no numeric self-score (R-7).
- `releases.toml` + `INDEX.md` updated; `docs/management-center.md` added.
