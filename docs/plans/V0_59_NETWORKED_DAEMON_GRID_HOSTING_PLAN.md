# HydraCache 0.59.0 Networked Daemon Grid Hosting — Codex Execution Plan

> **At a glance**
> - **What:** make the deployable `hydracache-server` **actually host the real networked grid** in
>   member role — closing `0.57` W6b / **TD-0008**. Wire the already-shipped networked adapters
>   (`hydracache-cluster-raft::RaftMetadataRuntime`, `hydracache-cluster-chitchat::ChitchatDiscovery`,
>   `hydracache-cluster-transport-axum`) into `grid_host.rs` so multiple daemons **join over
>   `cluster_addr`/`seeds`, elect a real raft leader, and report it** — turning the `0.57`
>   `source:"live"` tag from *single-in-process-member* into *true multi-node*. `/cluster/overview`
>   `leader:null` (grid_host.rs:154) becomes a real elected leader id.
> - **Why:** this is the **#1 maturity gap** to a defensible 1.0. `0.42`–`0.56` proved the production
>   grid in the **library + DST + networked-transport tests**, but the **deployable artifact** still
>   runs `HydraCache::local()` — `0.57` W6a wired only an **in-process** member. The networked pieces
>   already interoperate in tests (`chitchat_admission_bridge.rs`, `networked_raft.rs`); they are
>   simply **not wired into the daemon**. So this is **integration, not new consensus** (no new
>   algorithm, R-1).
> - **After (depends on):** `0.57.0` (the `grid_host.rs` / `GridControlPlaneHandle` /
>   `LiveClusterStatus` seam), the shipped `hydracache-cluster-*` adapters. Sequenced **after `0.58`**;
>   it retroactively upgrades `0.58` W4's "real multi-node soak" from an operator fixture to a true
>   daemon cluster, and is a prerequisite for a `1.0` "production-ready cluster out of the box" claim.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> debt tracked: [`../technical-debt/TD-0008-networked-daemon-grid-hosting.md`](../technical-debt/TD-0008-networked-daemon-grid-hosting.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its Definition of
Done **and** `cargo xtask verify`; never push red. Networked multi-daemon tests are network-gated and
**skip-graceful** without a loopback cluster.

## Preflight (verified against the repo)

- **The seam to fill exists (0.57 W6a).** `crates/hydracache-server/src/grid_host.rs`: `build_member`
  (:18) builds `HydraCache::member()` (cache.rs:137) with an **in-process**
  `RaftStyleMetadataControlPlane` (:22); `InProcessGridHandle` (:118) implements
  `GridControlPlaneHandle` but **`raft_leader_id() -> None`** (:154), `has_quorum` = "members
  non-empty" (:158), `reshard_phase` = `Idle` (:175), `is_draining` = `false` (:179). These are the
  exact placeholders W2/W3 replace with networked reality.
- **The networked adapters are shipped and interoperate in tests.**
  - `hydracache-cluster-raft::RaftMetadataRuntime` (lib.rs:439) — real raft-rs metadata control
    plane; `::single_node` (lib.rs:127), `::durable` (storage-backed), `snapshot() ->
    RaftMetadataRuntimeSnapshot { raft_node_id, term, commit_index, applied_index, role, … }`
    (lib.rs:622/172), `RaftRuntimeRole` (lib.rs:393, `StateRole::Leader → Leader` :407),
    `leader() -> Option<u64>` on the log store (log_store.rs:582). It is already used as an
    `Arc<…>` control plane in `tests/chitchat_admission_bridge.rs`.
  - `hydracache-cluster-chitchat::ChitchatDiscovery` **impl `ClusterDiscovery`** (lib.rs:404) — the
    discovery adapter `HydraCache::member().discovery(...)` (runtime.rs:207) expects.
  - `hydracache-cluster-transport-axum` — the cluster message transport, exercised end-to-end in
    `crates/hydracache-cluster-raft/tests/networked_raft.rs`.
- **Config already models member mode.** `crates/hydracache-server/src/config.rs`: `role = Member`,
  `cluster_addr`, `seeds`, `storage_dir`, `tls`; `validate()` already **fails loud** if a member has
  no `storage_dir`/`seeds` (config.rs:218-223).
- **The ignored sentinel is waiting.** `crates/hydracache-server/tests/grid_host.rs::
  multi_node_members_form_a_cluster_and_elect_one_leader` is `#[ignore]` (per TD-0008) — W5 enables
  it (or replaces it with a loopback 3-daemon gate).
- **Consumers that light up:** the `0.57` Management Center (`/cluster/overview` leader/quorum), the
  `0.56` operator (pods run member-mode `hydracache-server`), and `0.58` W4 (real multi-node soak).

## Release Theme

Wire the shipped networked cluster adapters into the deployable daemon so a real multi-node grid forms
from `cluster_addr`/`seeds`, elects a true raft leader, survives leader loss, and reports honest live
status — **integration over shipped consensus**, no new algorithm, `local`/`client` roles unchanged.

## Non-Goals

- **No new consensus / no new algorithm (R-1).** Reuse `RaftMetadataRuntime`; do not fork raft or add
  a consistency level. Authority stays epoch/version.
- **Not a data-plane rewrite.** This wires the **metadata control plane + membership + leader**; the
  value replication path (`0.42`/`0.43`) is unchanged.
- **`local`/`client` roles stay `modeled`.** Only `member` gains the networked stack; embedded fast
  path byte-for-byte unchanged (R-10).
- **Not the operator's job.** The operator (`0.56`) already schedules pods; `0.59` makes the pod's
  process actually join a cluster. No CRD changes required.

## Dependency Graph

```
0.57 grid_host seam (GridControlPlaneHandle) ─┐
hydracache-cluster-raft (RaftMetadataRuntime) ─┼─► W1 expose leader/role ─► W2 networked grid stack ─► W3 live status + lifecycle ─┐
hydracache-cluster-chitchat (ClusterDiscovery)─┤                                                                                   ├─► W5 multi-daemon E2E ─► W6 docs + gates + TD-0008 close
hydracache-cluster-transport-axum ────────────┘                          W4 TLS-bound cluster listener ──────────────────────────┘
```

## W1. Expose leader identity + role from the raft runtime

**Goal.** Give `RaftMetadataRuntime` a public accessor for the **current leader id** (from raft-rs
soft-state) and role, so a networked `GridControlPlaneHandle` can report a true leader instead of
`None` (grid_host.rs:154).

**Files.** `crates/hydracache-cluster-raft/src/lib.rs` (add `leader_id() -> Option<u64>` reading
`RawNode.raft.leader_id`, exposed alongside `snapshot()` at lib.rs:622; surface `RaftRuntimeRole`).

**Steps.**
1. Add `pub fn leader_id(&self) -> Option<u64>` returning the raft-rs `leader_id` (0 = unknown → `None`
   mid-election). Keep it consistent with `snapshot().role` (lib.rs:172/393).
2. Map the raft `u64` node id back to the `ClusterNodeId` used by membership (a small id↔node map the
   runtime already needs for transport).

**Tests & requirements.** `crates/hydracache-cluster-raft/tests/` (extend `persistent_log.rs` /
`networked_raft.rs`)
- `single_node_runtime_reports_itself_as_leader` (after election, `leader_id == raft_node_id`).
- `no_leader_id_during_election_is_none` (falsifiable: before commit, `None`).
- Run: `cargo test -p hydracache-cluster-raft --locked`.

**Risk & rollback.** Additive accessor; revert leaves `leader_id` unexposed and the handle at `None`.

## W2. Networked grid stack in `grid_host.rs` (member role)

**Goal.** Build the **networked** member stack when `role == Member`: a durable `RaftMetadataRuntime`
as the `ClusterControlPlane`, `ChitchatDiscovery` over `seeds`, and the cluster transport on
`cluster_addr` — replacing the in-process `RaftStyleMetadataControlPlane` (grid_host.rs:22).

**Files.** `crates/hydracache-server/src/grid_host.rs` (new networked path + `NetworkedGridHandle`),
`crates/hydracache-server/Cargo.toml` (+ `hydracache-cluster-raft`, `hydracache-cluster-chitchat`,
`hydracache-cluster-transport-axum`).

**Code sketch (mirrors the existing test wiring in `chitchat_admission_bridge.rs`).**
```rust
// grid_host.rs — networked member path (replaces the in-process control plane).
fn build_networked_member(config: &ServerConfig)
    -> Result<(HydraCache, Arc<dyn GridControlPlaneHandle>), ServerConfigError>
{
    let raft = Arc::new(
        RaftMetadataRuntime::durable(cluster_name(config), raft_node_id(config), log_dir(config))
            .map_err(host_err)?,                                 // storage_dir-backed (config.rs)
    );
    let discovery = Arc::new(ChitchatDiscovery::from_seeds(&config.seeds, config.cluster_addr)); // ClusterDiscovery (chitchat:404)
    let cache = HydraCache::member()                             // cache.rs:137
        .cluster(cluster_name(config))
        .control_plane(raft.clone())                            // runtime.rs:191 — RaftMetadataRuntime as ClusterControlPlane
        .discovery(discovery)                                    // runtime.rs:207
        .bind(config.cluster_addr.to_string())
        .start().await?;
    Ok((cache, Arc::new(NetworkedGridHandle::new(raft))))
}
```

**Steps.**
1. `build_member` dispatches: networked stack for `Member` (this WI), in-process stays a documented
   test/dev fallback (`HYDRACACHE_GRID_INPROC=1`) so W6a's tests keep passing.
2. Construct the raft runtime **durable** against `storage_dir` (config.rs); derive `raft_node_id`
   deterministically from the node identity.
3. Spawn the raft drive loop (tick/step/ready) on the grid-host tokio runtime (grid_host.rs already
   owns one, :57-68); carry raft messages over the cluster transport.

**Tests & requirements.** `crates/hydracache-server/tests/grid_host.rs`
- `member_builds_networked_stack_with_durable_raft` (constructs without panic; durable dir created).
- `member_without_storage_or_seeds_is_rejected_loud` (kept — config.rs:218-223).
- `inproc_fallback_still_builds_under_env_flag` (W6a regression guard).
- Run: `cargo test -p hydracache-server --locked grid_host`.

**Risk & rollback.** The heaviest WI. Revert restores the in-process path; `source:"live"` reverts to
single-member and `leader:null` (still honest via the `0.57` tag).

## W3. Live status + lifecycle wiring (`NetworkedGridHandle`)

**Goal.** Implement `GridControlPlaneHandle` over the networked runtime so **every** field is real:
`raft_leader_id` (W1), `snapshot`/`members`, `has_quorum`, `reachability`, `reshard_phase`,
`is_draining` — and integrate graceful drain with `0.56`.

**Files.** `crates/hydracache-server/src/grid_host.rs` (`NetworkedGridHandle`),
`crates/hydracache-server/src/bootstrap.rs` (drain path → raft leave).

**Steps.**
1. `raft_leader_id()` = W1's `leader_id` mapped to `ClusterNodeId` (real leader, `None` mid-election).
2. `has_quorum()` = raft view membership majority reachable; `reachability(node)` from chitchat
   liveness; `reshard_phase()` from the grid runtime; `is_draining()` from the server state.
3. Graceful drain (`ServerRuntime::graceful_shutdown`, bootstrap.rs:242): a draining leader steps down
   / a member leaves the raft config before the process exits (no stuck quorum) — ties `0.56` W4.

**Tests & requirements (network-gated where multi-node).** `crates/hydracache-server/tests/grid_host.rs`
- `leader_status_reflects_raft_soft_state` (single-node: leader = self; overview `leader != null`).
- `draining_member_leaves_raft_config_cleanly`.
- `overview_reports_live_source_with_networked_handle`.
- Run: `cargo test -p hydracache-server --locked grid_host`.

**Risk & rollback.** Correctness of drain-vs-quorum is load-bearing; gate it. Revert leaves the W2
stack but with placeholder status fields.

## W4. TLS-bound cluster listener + fail-loud config

**Goal.** Bind the cluster transport with `0.48` mTLS when configured, and fail loud on incomplete
cluster material — a member must not silently form an unauthenticated cluster in a non-loopback
deployment.

**Files.** `crates/hydracache-server/src/grid_host.rs` (pass `config.tls` to the cluster listener),
`config.rs` (reuse the existing `exposes_non_loopback` + TLS validation, config.rs:235-263).

**Steps.**
1. Wire `config.tls` (cert/key/ca) into the cluster transport listener on `cluster_addr`.
2. Reuse the shipped guard: a non-loopback cluster listener without TLS and without
   `acknowledge_insecure` is already rejected (config.rs:238-240) — assert it also covers
   `cluster_addr`.

**Tests & requirements.** `crates/hydracache-server/tests/grid_host.rs`
- `non_loopback_member_without_tls_is_rejected_loud`.
- `member_cluster_listener_uses_configured_tls`.
- Run: `cargo test -p hydracache-server --locked grid_host`.

**Risk & rollback.** Security boundary; tested explicitly. Revert removes TLS binding (loopback-only
dev still works).

## W5. Multi-daemon E2E (loopback, network-gated, skip-graceful)

**Goal.** Prove the whole point: **three daemons form one cluster, elect one leader, survive leader
loss** — enabling the ignored sentinel and adding the falsifiable failure paths.

**Files.** `crates/hydracache-server/tests/grid_host.rs` (enable/replace
`multi_node_members_form_a_cluster_and_elect_one_leader`), a loopback harness spawning three
`ServerRuntime` member processes/tasks over loopback `cluster_addr`s + shared `seeds`.

**Steps.**
1. Start three members on loopback; assert they **converge to one leader** and three members in
   `/cluster/overview`.
2. **Kill the leader**; assert `/cluster/overview` `leader` transitions **null → new elected id**
   (no stale leader — ties `0.57` W3 corner case, now against a *real* election).
3. Skip **graceful** without the network harness (no loopback cluster), keeping `cargo xtask verify`
   green; run in a named network-gated tier in `docs/GATES.md`.

**Tests & requirements.**
- `three_members_form_a_cluster_and_elect_one_leader` (network-gated; the un-ignored sentinel).
- `killing_the_leader_triggers_reelection_without_stale_leader` (falsifiable: a stale-leader report
  fails).
- `no_lost_committed_metadata_across_leader_change`.
- `grid_host_skips_gracefully_without_a_network_cluster` (always runs; keeps PR gate green).
- Run (gated): the network command in `docs/GATES.md`; excluded from the fast PR gate.

**Risk & rollback.** Real elections are timing-sensitive; keep the harness deterministic where
possible and gated to nightly. Revert leaves the sentinel `#[ignore]` and TD-0008 open.

## W6. Docs, gates, and TD-0008 closure

**Goal.** Document the member-mode deployment, close TD-0008, and keep the ledger/manifest honest.

**Files.** `docs/deployment` member-mode runbook (seeds/cluster_addr/storage_dir/TLS),
`docs/management-center.md` (leader is now live in `member` mode), `docs/GATES.md` (network-gated E2E
command), `docs/technical-debt/TD-0008-…` → Resolved, `releases.toml` + `INDEX.md`.

**Steps.**
1. Runbook: how to bring up a 3-node cluster of daemons (or via the `0.56` operator).
2. Update the `0.57` `source:live|modeled` note: `member` now reports live multi-node; `local`/`client`
   still `modeled`.
3. Mark **TD-0008 Resolved** (the sentinel is enabled); note the `0.58` W4 upgrade.

**Tests & requirements.**
- `cargo xtask verify` green (incl. `doc-check` header-status; keep `0.59.0` header/manifest/INDEX in
  sync).
- Run: `cargo xtask verify`.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green throughout; multi-daemon E2E network-gated + **skip-graceful** (PR gate
  green without a cluster).
- A `member` daemon builds the **networked** stack (durable raft + chitchat + cluster transport) and
  reports a **real elected leader** — `/cluster/overview` `leader` is no longer `null` (W1-W3).
- Three daemons form one cluster; **killing the leader re-elects without a stale leader**; no lost
  committed metadata (W5, falsifiable).
- Cluster listener is **TLS-bound** when configured; non-loopback without TLS is rejected loud (W4).
- `local`/`client` roles unchanged and `modeled`; embedded fast path byte-for-byte unchanged (R-10);
  no new consensus/consistency level (R-1).
- **TD-0008 marked Resolved**; the `0.57` `source` honesty note updated; `0.58` W4 re-pointed to the
  real daemon cluster.
- `releases.toml` + `INDEX.md` updated; `0.59.0` header matches the manifest (doc-check).
