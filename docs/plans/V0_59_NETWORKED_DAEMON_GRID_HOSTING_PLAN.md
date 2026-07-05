# HydraCache 0.59.0 Networked Daemon Grid Hosting — Codex Execution Plan

> **At a glance**
> - **What:** make the deployable `hydracache-server` **actually host the real networked grid** in
>   member role — closing `0.57` W6b / **TD-0008**. Wire the already-shipped networked adapters
>   (`hydracache-cluster-raft::RaftMetadataRuntime`, `hydracache-cluster-chitchat::ChitchatDiscovery`,
>   `hydracache-cluster-transport-axum`) into `grid_host.rs` so multiple daemons **join over
>   `cluster_addr`/`seeds`, elect a real raft leader, and report it** — plus the one enabling
>   slice that makes it possible (**W1b**: a network-drivable, **multi-voter** `RaftMetadataRuntime`;
>   today the runtime is single-node-only). Turns the `0.57` `source:"live"` tag from
>   *single-in-process-member* into *true multi-node*. `/cluster/overview` `leader:null`
>   (grid_host.rs:154) becomes a real elected leader id.
> - **Why:** this is the **#1 maturity gap** to a defensible 1.0. `0.42`–`0.56` proved the production
>   grid in the **library + DST + networked-transport tests**, but the deployable daemon's member
>   role is still **in-process only**: `0.57` W6a builds `HydraCache::member()` over an in-process
>   `RaftStyleMetadataControlPlane` (grid_host.rs:22) — a standalone daemon member path, not a
>   networked multi-daemon grid. The networked pieces interoperate in tests
>   (`chitchat_admission_bridge.rs` at runtime level, single-node; `networked_raft.rs` at
>   raw-`RawNode` level) but are **not wired into the daemon**, and the runtime itself is not yet
>   network-drivable (see G4/W1b). Mostly **integration, not new consensus** (no new algorithm,
>   R-1) — W1b adds runtime plumbing around shipped raft-rs, not a new protocol.
> - **After (depends on):** `0.57.0` (the `grid_host.rs` / `GridControlPlaneHandle` /
>   `LiveClusterStatus` seam) **and `0.58.0`** (W6 discharges `0.58` W4's downstream obligation and
>   re-points its soak, so `0.58` must be shipped first — `releases.toml` records both), plus the
>   shipped `hydracache-cluster-*` adapters. It retroactively upgrades `0.58` W4's "real multi-node
>   soak" from an operator fixture to a true daemon cluster, and is a prerequisite for a `1.0`
>   "production-ready cluster out of the box" claim.
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
    `leader() -> Option<u64>` on the log store (log_store.rs:582). It **implements
    `ClusterControlPlane`** (lib.rs:1027) and is already used as an `Arc<…>` control plane in
    `tests/chitchat_admission_bridge.rs` — but only as a **single-node** runtime; it is not yet
    network-drivable (G4/W1b).
  - `hydracache-cluster-chitchat::ChitchatDiscovery` **impl `ClusterDiscovery`** (lib.rs:404) — the
    discovery adapter `HydraCache::member().discovery(...)` (runtime.rs:207) expects.
  - `hydracache-cluster-transport-axum` — the cluster message transport, exercised end-to-end in
    `crates/hydracache-cluster-raft/tests/networked_raft.rs` — against **raw `RawNode`s**, not the
    runtime; runtime-level coverage arrives with W1b.
- **Config already models member mode.** `crates/hydracache-server/src/config.rs`: `role = Member`,
  `cluster_addr`, `seeds`, `storage_dir`, `tls`; `validate()` already **fails loud** if a member has
  no `storage_dir`/`seeds` (config.rs:218-223).
- **The ignored sentinel is waiting.** `crates/hydracache-server/tests/grid_host.rs::
  multi_node_members_form_a_cluster_and_elect_one_leader` is `#[ignore]` (per TD-0008) — W5 enables
  it (or replaces it with a loopback 3-daemon gate).
- **Consumers that light up:** the `0.57` Management Center (`/cluster/overview` leader/quorum), the
  `0.56` operator (pods run member-mode `hydracache-server`), and `0.58` W4 (real multi-node soak).

## Gap Analysis (post-audit — the real networked stack, corrected twice)

A third code-grounded pass corrected the second draft's central assumption (the second pass had
"corrected" the first draft in the wrong direction: it claimed `RaftMetadataRuntime` is not a
`ClusterControlPlane` — it is). Each is verified:

- **G1 — `RaftMetadataRuntime` IS a `ClusterControlPlane`; one runtime is the single source of
  truth.** `impl<S> ClusterControlPlane for RaftMetadataRuntime<S>` exists (lib.rs:1027):
  `join_member`/`join_client` propose `RaftMetadataCommand::{MemberUpsert,ClientUpsert}` through
  raft commit and materialize into the runtime's internal membership view, and
  `chitchat_admission_bridge.rs` already passes an `Arc<RaftMetadataRuntime>` as the bridge's
  control plane. So `HydraCache::member().control_plane(raft.clone())` compiles and is the right
  design: W2 wires **one** `Arc<RaftMetadataRuntime>` everywhere —
  `HydraCache::member().control_plane(raft.clone()).discovery(discovery.clone())`
  (runtime.rs:455/471) **and** `NetworkedGridHandle` reads the same runtime. No second
  `InMemoryCluster` next to raft: that would fork the membership authority into a status plane and
  a cache plane that can disagree. The **triad** proven in
  `hydracache-cluster-raft/tests/chitchat_admission_bridge.rs` still stands:
  **`ChitchatDiscovery` (gossip) + `RaftMetadataRuntime` (raft authority + control plane) +
  `ClusterAdmissionBridge`** that admits gossip candidates into raft via `bridge.run_once().await`
  (rejects stale generations).
- **G2 — discovery constructor is `spawn_udp`, not `from_seeds`.** Real API:
  `ChitchatDiscovery::spawn_udp(ChitchatDiscoveryConfig::new(cluster, node, generation, bind_addr)
  .seed_nodes(seeds))` (chitchat lib.rs:238/98/130). Tests use `ChannelTransport`; the daemon uses
  `spawn_udp` (`UdpTransport`).
- **G3 — leader is read from the raft `RawNode`.** `RaftMetadataRuntimeSnapshot` (lib.rs:622-630)
  exposes `term`/`role` (from `raw_node.raft.state`) but **not** the leader id. W1 adds
  `leader_id()` reading `raw_node.raft.leader_id` (0 → `None`), mapping the `u64` back to a
  `ClusterNodeId`.
- **G4 — the runtime is single-node-only and NOT network-drivable today; W1b makes it so.**
  `::durable` builds a **single-node** voter set (`RaftMetadataRuntimeConfig::single_node`,
  lib.rs:531/540). The ready cycle is **internal** (`RaftRuntimeState::drain_ready`, lib.rs:977 —
  not public API) and **drops outbound raft messages** (`ready.messages()` /
  `light_ready.messages()` are never taken or sent). There is no public `step()` for inbound peer
  messages, no `tick()`, and **no conf-change path** (no `ConfChange` anywhere in the crate) — so
  today three daemons cannot elect one raft leader no matter how `grid_host.rs` wires them. The
  constructor also unconditionally `campaign()`s (lib.rs:567), which is wrong for a multi-voter
  cold start (every node would campaign at once). `RaftWireMessage`/`RaftMessageSink`
  (lib.rs:224/257) exist as seams, but the runtime never uses them; `networked_raft.rs` proves the
  transport against **raw `RawNode`s**, not the runtime. **W1b** closes exactly this gap before W2
  can exist; W2/W3 then spawn the drive loop over the W1b seams and run `bridge.run_once()`
  periodically.
- **G5 — one runtime serves both the value/invalidation path and the status handle.** Because of
  G1, the cache's control plane and `NetworkedGridHandle`'s status source are the **same**
  `Arc<RaftMetadataRuntime>`: member admission goes through raft commit, and the handle reads
  leader/term/members from the identical state. What stays out of scope is re-routing the **value
  replication / partition-ownership** path through raft-committed placement — the grid value path
  (`0.42`/`0.43`) is unchanged (named scoping boundary below).

**Corrected W2 stack sketch (grounded in `chitchat_admission_bridge.rs`, after W1b).**
```rust
// crates/hydracache-server/src/grid_host.rs — networked member stack (replaces the in-process CP).
let discovery = Arc::new(
    ChitchatDiscovery::spawn_udp(                                   // chitchat lib.rs:238
        ChitchatDiscoveryConfig::new(cluster_name(config), node_id(config), generation(config),
                                     config.cluster_addr)
            .seed_nodes(config.seeds.clone()),                     // :130
    ).await.map_err(host_err)?,
);
let raft = Arc::new(
    RaftMetadataRuntime::durable(cluster_name(config), raft_node_id(config), log_dir(config))
        .map_err(host_err)?,                                       // storage_dir-backed
);
let bridge = ClusterAdmissionBridge::new(discovery.clone(), raft.clone());  // hydracache re-export
// drive loop (G4): tick raft + step transport inbound + process ready + admit gossip candidates
spawn_grid_drive(raft.clone(), bridge, cluster_transport_on(config.cluster_addr, &config.tls));
let cache = HydraCache::member().cluster(cluster_name(config))
    .control_plane(raft.clone())                              // same authoritative control plane
    .discovery(discovery.clone())
    .node_id(node_id(config))
    .generation(generation(config))
    .bind(config.cluster_addr.to_string()).start().await?;
let handle: Arc<dyn GridControlPlaneHandle> = Arc::new(NetworkedGridHandle::new(raft, discovery));
```

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

## Technical-debt scope & downstream obligations (do not lose)

| TD / obligation | In `0.59`? | Detail |
| --- | --- | --- |
| **TD-0008** networked daemon grid | **Closed here** | This release *is* TD-0008. W5's loopback multi-daemon E2E enables the ignored sentinel; W6 marks it **Resolved**. |
| **Discharge `0.58` W4's downstream obligation** | **Yes (W6)** | `0.58` shipped W4 as **honest-partial**: its `soak_kind.rs` ran against a kind fixture whose pods host the **in-process** member grid (W6a), *not* a true multi-daemon raft cluster. Once `0.59` lands, the pods host the **networked** grid (W2), so `0.58`'s soak now exercises real multi-daemon raft. **W6 re-points `0.58` `soak_kind.rs` at the real daemon cluster and lifts the `0.58` TD-scope "blocked" caveat.** |
| **TD-0009 coverage ratchet / coverage-run stability** | **Revisited, not closed here** | `0.59` adds server/operator-facing surface that may lower the clean coverage baseline. Keep the ratchet/targeted-test slice post-`0.59`; do not add a coverage gate in this release. |
| **Production soak mileage** | **Stays for `0.60`/`1.0`** | `0.59` makes a real daemon cluster *soakable*; multi-day field mileage still accrues later (R-11) — `0.59` does not claim "battle-tested". |
| TD-0002 raft/protobuf, TD-0003 bucket C, TD-0004 placement, TD-0005 Java artifact | **Out of scope** | Untouched. |

**Forward note for `1.0`:** a real daemon cluster (`0.59`) + soak harness (`0.58`) + operability
(`0.57`) are the three legs of a defensible `1.0` "production-ready cluster out of the box"; the
remaining `1.0` work is API-freeze/semver + mileage, not new consensus.

## Dependency Graph

```
0.57 grid_host seam (GridControlPlaneHandle) ─┐
hydracache-cluster-raft (RaftMetadataRuntime) ─┼─► W1 leader/role ─► W1b network-drivable multi-voter runtime ─► W2 daemon stack ─► W3 live status + lifecycle ─┐
hydracache-cluster-chitchat (ClusterDiscovery)─┤                                                                                                                            ├─► W5 daemon E2E ─► W6 docs + gates + TD-0008 close
hydracache-cluster-transport-axum ────────────┘                                                                 W4 TLS-bound cluster listener ───────────────────────────┘
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
`networked_raft.rs`) — note the auto-campaign correction in "Test completeness" below:
- `single_node_reports_itself_as_leader_after_startup` (post construction; `build_with_storage`
  currently campaigns and drains ready before returning, lib.rs:567/578),
  `leader_id() == raft_node_id`).
- `leader_id_maps_zero_soft_state_to_none` (unit-test the 0 → `None` mapping through a tiny helper or
  a W1b no-leader runtime fixture; do **not** pretend the existing public single-node constructor has
  a before-ready window).
- `follower_reports_the_elected_leader_not_itself` (covered by the W1b multi-voter runtime fixture:
  follower's `leader_id()` equals the elected leader's id).
- Run: `cargo test -p hydracache-cluster-raft --locked`.

**Risk & rollback.** Additive accessor; revert leaves `leader_id` unexposed and the handle at `None`.

## W1b. Make `RaftMetadataRuntime` network-drivable and multi-voter

**Goal.** Close the load-bearing gap behind the daemon claim: turn the shipped single-node
`RaftMetadataRuntime` into a runtime that can participate in a multi-voter raft cluster over the
existing HTTP cluster transport. This is runtime plumbing around raft-rs, not a new consensus
algorithm (R-1).

**Files.** `crates/hydracache-cluster-raft/src/lib.rs`,
`crates/hydracache-cluster-raft/src/log_store.rs`,
`crates/hydracache-cluster-raft/tests/networked_raft.rs`,
`crates/hydracache-cluster-transport-axum/src/lib.rs` if an adapter wrapper is needed.

**Steps.**
1. Add a multi-voter runtime configuration path: explicit voters/peers, stable
   `ClusterNodeId` ↔ raft `u64` mapping, and a persisted or deterministic mapping source. Keep
   `::single_node`/`::durable` behavior intact for current tests, but add a networked constructor that
   does **not** blindly auto-campaign every daemon at construction.
2. Expose a drive seam: `tick`, inbound `step(RaftWireMessage)`, and a `drain_ready`/outbound-message
   API that persists entries, applies committed metadata, and returns/sends `RaftWireMessage`s instead
   of dropping raft-rs outbound messages inside the private `RaftRuntimeState::drain_ready`.
3. Add a voter-change path. A `MemberUpsert` updates HydraCache metadata membership; it is not
   automatically a raft `ConfChange`. Define when a discovered member becomes a raft voter, how a
   leaving member is removed, and fail loud if the node-id mapping/conf-change cannot be committed.
4. Bridge HTTP transport to the runtime: inbound `AxumClusterMessageService` routes decode opaque raft
   payloads into `RaftWireMessage` and call `step`; outbound messages POST through a sink over
   `hydracache-cluster-transport-axum` with TLS/auth from W4.
5. Keep all new drive APIs deterministic and testable without wall-clock assertions; networked daemon
   timing belongs in bounded poll loops and the network-gated tier (R-5).

**Tests & requirements.**
- `runtime_three_voters_elect_one_leader` (runtime-level, not raw `RawNode` harness).
- `runtime_replicates_member_upsert_to_all_voters`.
- `runtime_outbound_messages_are_emitted_not_dropped`.
- `runtime_steps_inbound_wire_message_and_updates_soft_state`.
- `conf_change_adds_and_removes_raft_voter_loudly`.
- `raft_node_id_mapping_is_stable_across_restart`.
- Run: `cargo test -p hydracache-cluster-raft --locked networked_raft`.

**Risk & rollback.** This is the real enabling slice. Revert leaves the daemon able to construct
single-node raft-backed membership only; W2/W5 must not claim a multi-daemon elected leader without
W1b.

## W2. Networked grid stack in `grid_host.rs` (member role)

**Goal.** Build the **networked triad** when `role == Member` (G1): `ChitchatDiscovery::spawn_udp`
over `seeds`, a durable/networked `RaftMetadataRuntime`, and a `ClusterAdmissionBridge` connecting
them — driven by a spawned loop — replacing the in-process `RaftStyleMetadataControlPlane`
(grid_host.rs:22). The cache and the status handle share the same raft runtime.

**Files.** `crates/hydracache-server/src/grid_host.rs` (networked path + `NetworkedGridHandle` +
`spawn_grid_drive`), `crates/hydracache-server/Cargo.toml` (+ `hydracache-cluster-raft`,
`hydracache-cluster-chitchat`, `hydracache-cluster-transport-axum`). See the corrected stack sketch in
**Gap Analysis** above (grounded in `chitchat_admission_bridge.rs`).

**Steps.**
1. `build_member` dispatches: networked triad for `Member` (this WI), in-process stays a documented
   test/dev fallback (`HYDRACACHE_GRID_INPROC=1`) so W6a's tests keep passing.
2. Construct `ChitchatDiscovery::spawn_udp(ChitchatDiscoveryConfig::new(cluster, node, generation,
   cluster_addr).seed_nodes(seeds))` (G2) and the W1b networked/durable `RaftMetadataRuntime` against
   `storage_dir`; derive `raft_node_id` deterministically from node identity.
3. Start `HydraCache::member()` with the same `Arc<RaftMetadataRuntime>` as its
   `ClusterControlPlane` and the same `Arc<ChitchatDiscovery>` as its discovery adapter:
   `.control_plane(raft.clone()).discovery(discovery.clone())`.
4. Wire `ClusterAdmissionBridge::new(discovery.clone(), raft.clone())` and **spawn the drive loop** on
   the grid-host tokio runtime (grid_host.rs already owns one, :57-68): tick raft, step inbound
   cluster-transport messages, drain ready/outbound messages through the W1b seam, and run
   `bridge.run_once()` periodically to admit gossip candidates into raft.

**Tests & requirements.** `crates/hydracache-server/tests/grid_host.rs`
- `member_builds_networked_triad_with_shared_raft_control_plane` (cache + handle + bridge share one
  `Arc<RaftMetadataRuntime>`; durable dir created; no panic).
- `drive_loop_admits_a_gossip_candidate_into_the_shared_raft_runtime` (seeded candidate →
  `bridge.run_once` commits a `MemberUpsert`; cache/status membership views agree).
- `member_without_storage_or_seeds_is_rejected_loud` (kept — config.rs:218-223).
- `inproc_fallback_still_builds_under_env_flag` (W6a regression guard).
- Run: `cargo test -p hydracache-server --locked grid_host`.

**Scoping boundary (named).** `0.59` makes daemon **membership/status authority** raft-backed. It does
not rewrite value replication, partition ownership, or placement to a new data plane; the shipped
`0.42`/`0.43` value/invalidation path stays unchanged unless a later release explicitly adds a
raft-committed placement/data-plane item.

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
- `has_quorum_reflects_membership_majority` (N members → quorum `⌊N/2⌋+1`; **falsifiable**: a majority
  unreachable → `has_quorum()` false).
- `reachability_maps_chitchat_liveness` (suspect/absent → `Suspect`/`Unreachable`; live → `Reachable`).
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

**Goal.** Prove the whole point at daemon level: **three `ServerRuntime` member daemons use the W1b
networked `RaftMetadataRuntime`, form one cluster, elect one leader, survive leader loss** — enabling
the ignored sentinel and adding the falsifiable failure paths. Raw `RawNode` harnesses and HTTP
route round-trips are useful lower-level tests, but they are not sufficient for this gate.

**Files.** `crates/hydracache-server/tests/grid_host.rs` (enable/replace
`multi_node_members_form_a_cluster_and_elect_one_leader`), a loopback harness spawning three
`ServerRuntime` member processes/tasks over loopback `cluster_addr`s + shared `seeds`.

**Steps.**
1. Start three members on loopback with distinct `cluster_addr`s, shared `seeds`, durable
   `storage_dir`s, and the W2 daemon stack; assert they **converge to one leader** and three members
   in `/cluster/overview`.
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
command), `docs/technical-debt/TD-0008-…` → Resolved, `docs/plans/V0_58_…` (lift the W4 caveat),
`crates/hydracache-operator/tests/soak_kind.rs` (re-point), `releases.toml` + `INDEX.md`.

**Steps.**
1. Runbook: how to bring up a 3-node cluster of daemons (or via the `0.56` operator).
2. Update the `0.57` `source:live|modeled` note: `member` now reports live multi-node; `local`/`client`
   still `modeled`.
3. Mark **TD-0008 Resolved** (the sentinel is enabled).
4. **Discharge the `0.58` downstream obligation** (three concrete sub-items verified in the shipped
   `0.58` code — do not lose them):
   a. Re-point `crates/hydracache-operator/tests/soak_kind.rs` so its "no lost committed write /
      leader re-elected" assertions run against pods hosting the **networked** grid (W2), not the
      `0.57.1` in-process fixture.
   b. **Update the `SCOPE_DISCLOSURE` constant + its test assertion** (soak_kind.rs:17 +
      `soak_skips_gracefully_without_a_cluster` asserts `SCOPE_DISCLOSURE.contains("0.59 / TD-0008")`,
      soak_kind.rs:352) — once `0.59` lands, the disclosure "honest partial … lands in 0.59" is no
      longer true and the assertion must be flipped/removed, or CI stays wrong-but-green.
   c. **Wire the external chaos injector for partition / slow-disk faults** — today `inject()` only
      deletes a pod; `NetworkPartition`/`SlowDisk` are *observe-only* (soak_kind.rs:159-168,
      `requires_external_injector`). With a real daemon cluster, actually inject them (or keep them
      external + documented) so the chaos soak drives all three fault classes, not just pod-crash.
   d. **Edit `docs/plans/V0_58_…` Technical-debt-scope row for TD-0008** from "blocked → stays for
      0.59" to "unblocked by 0.59" so the roadmap stays honest.

**Tests & requirements.**
- `cargo xtask verify` green (incl. `doc-check` header-status; keep `0.59.0` header/manifest/INDEX in
  sync).
- `soak_kind.rs` (from `0.58`) now asserts against a real daemon cluster (network/kind-gated,
  skip-graceful — no PR-gate regression).
- Run: `cargo xtask verify`.

## Test completeness — refinements & additions (deepening audit)

A code-grounded pass on the *test descriptions* corrected one and added several missing ones:

- **W1 leader_id — the "None mid-election" test as first drafted is unreachable and is corrected.**
  Every `RaftMetadataRuntime` **auto-campaigns on construction** (`raw_node.campaign()`, lib.rs:567),
  so a single node becomes leader immediately — there is no single-node "mid-election" window. The
  honest tests are:
  - `single_node_reports_itself_as_leader_after_startup` — after construction, because the existing
    public single-node constructor campaigns and drains ready before returning.
  - `leader_id_maps_zero_soft_state_to_none` — unit-test the 0 → `None` mapping through a helper or
    W1b no-leader fixture; do not rely on a nonexistent before-ready public window.
  - `follower_reports_the_elected_leader_not_itself` — the W1b runtime-level multi-voter fixture,
    where a follower's `leader_id()` equals the elected leader's id.
- **W1b — add runtime-level network tests before daemon wiring** (raw `RawNode` tests are not enough):
  - `runtime_three_voters_elect_one_leader`.
  - `runtime_replicates_member_upsert_to_all_voters`.
  - `runtime_outbound_messages_are_emitted_not_dropped`.
  - `runtime_steps_inbound_wire_message_and_updates_soft_state`.
  - `conf_change_adds_and_removes_raft_voter_loudly`.
- **W2 — add durable-recovery, stable-identity, and shared-runtime tests** (the daemon uses durable
  raft over `storage_dir`, and raft needs a stable node id across restarts):
  - `durable_raft_recovers_committed_membership_after_restart` — build `::durable`, admit a member via
    the bridge, drop + reopen the runtime against the same `storage_dir`, assert the committed
    membership/commands survive (mirrors `durable_runtime.rs`/`with_config_and_metadata_store`
    recovery).
  - `raft_node_id_is_stable_and_deterministic_for_a_node` — the same node identity → the same
    `raft_node_id` across process restarts (a durable raft log keyed on a changing id would corrupt).
  - `member_builds_networked_triad_with_shared_raft_control_plane` — cache, bridge, and
    `NetworkedGridHandle` all point at the same `Arc<RaftMetadataRuntime>`.
- **W3 — add explicit quorum and reachability tests** (not just leader):
  - `has_quorum_reflects_membership_majority` — with N committed members, quorum is `⌊N/2⌋+1` reachable;
    below that, `has_quorum()` is false (falsifiable: mark a majority unreachable → false).
  - `reachability_maps_chitchat_liveness` — a chitchat-suspect/absent node maps to
    `Reachability::{Suspect,Unreachable}`, a live one to `Reachable`.
- **W5 — pin down election determinism + startup resilience** (real elections are timing-sensitive):
  - Determinism strategy for the loopback 3-daemon test: short `ticks(election, heartbeat)`
    (`RaftMetadataRuntimeConfig::ticks`, lib.rs:139), a **generous** convergence timeout with a
    bounded poll loop (like `chitchat_admission_bridge.rs::wait_until`), and **network-gated +
    skip-graceful** so flakiness never reaches the PR gate.
  - `seed_unreachable_at_startup_retries_not_crashes` — a member whose `seeds` are initially
    unreachable **retries discovery/join, does not panic**, and converges once a seed appears
    (fail-loud only on config-invalid, not on transient unreachability).
- **Cross-cutting (ties `TD-0009`):** `0.59` adds `grid_host.rs` + `NetworkedGridHandle` +
  `config.rs` cluster-TLS surface — exactly the low-coverage areas TD-0009 names. Every new public fn
  must land with a matrix row so the new surface does **not** drag workspace coverage down (TD-0009
  revisit trigger: "0.59 adds server/operator surface").

## Test coverage matrix (every new artifact has a named test)

| New code | Source file | Covering test(s) | Tier |
| --- | --- | --- | --- |
| `RaftMetadataRuntime::leader_id()` (W1) | `hydracache-cluster-raft/src/lib.rs` | `single_node_reports_itself_as_leader_after_startup`, `leader_id_maps_zero_soft_state_to_none`, `follower_reports_the_elected_leader_not_itself` | PR |
| network-drivable multi-voter runtime (W1b) | `hydracache-cluster-raft/src/lib.rs`, `log_store.rs` | `runtime_three_voters_elect_one_leader`, `runtime_replicates_member_upsert_to_all_voters`, `runtime_outbound_messages_are_emitted_not_dropped`, `runtime_steps_inbound_wire_message_and_updates_soft_state`, `conf_change_adds_and_removes_raft_voter_loudly` | PR |
| networked triad + `spawn_grid_drive` (W2) | `hydracache-server/src/grid_host.rs` | `member_builds_networked_triad_with_shared_raft_control_plane`, `drive_loop_admits_a_gossip_candidate_into_the_shared_raft_runtime`, `inproc_fallback_still_builds_under_env_flag` | PR |
| durable recovery + stable id (W1b/W2) | `grid_host.rs`, `hydracache-cluster-raft` | `durable_raft_recovers_committed_membership_after_restart`, `raft_node_id_is_stable_and_deterministic_for_a_node` | PR |
| member config guard (W2) | `config.rs` (reuse) | `member_without_storage_or_seeds_is_rejected_loud`, `seed_unreachable_at_startup_retries_not_crashes` | PR |
| `NetworkedGridHandle` (W3) | `grid_host.rs` | `leader_status_reflects_raft_soft_state`, `has_quorum_reflects_membership_majority` (falsifiable), `reachability_maps_chitchat_liveness`, `draining_member_leaves_raft_config_cleanly`, `overview_reports_live_source_with_networked_handle` | PR |
| TLS cluster listener (W4) | `grid_host.rs`, `config.rs:235-263` | `non_loopback_member_without_tls_is_rejected_loud`, `member_cluster_listener_uses_configured_tls` | PR |
| loopback 3-daemon E2E (W5) | `hydracache-server/tests/grid_host.rs` | `three_members_form_a_cluster_and_elect_one_leader`, `killing_the_leader_triggers_reelection_without_stale_leader` (falsifiable), `no_lost_committed_metadata_across_leader_change` against real `ServerRuntime` daemons | network-gated / nightly |
| skip-graceful guard (W5) | `tests/grid_host.rs` | `grid_host_skips_gracefully_without_a_network_cluster` | PR (always runs) |
| 0.58 soak re-point (W6) | `hydracache-operator/tests/soak_kind.rs` | `multi_node_chaos_soak_loses_no_committed_write` (now real daemon cluster), `SCOPE_DISCLOSURE` assertion updated | kind / nightly |

**Coverage rule (DoD):** no new public type or file lands without a row here; PR-tier tests are
deterministic and in `cargo xtask verify`; network/kind rows are env-gated and **skip-graceful** so the
fast gate stays green without a loopback cluster.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green throughout; multi-daemon E2E network-gated + **skip-graceful** (PR gate
  green without a cluster).
- `RaftMetadataRuntime` is network-drivable and multi-voter at runtime level before daemon wiring
  claims a multi-daemon elected leader (W1b).
- A `member` daemon builds the **networked** stack (durable raft + chitchat + cluster transport) and
  reports a **real elected leader** — `/cluster/overview` `leader` is no longer `null` (W1-W3,
  including W1b).
- Three daemons form one cluster; **killing the leader re-elects without a stale leader**; no lost
  committed metadata (W5, falsifiable, real `ServerRuntime` daemons).
- Cluster listener is **TLS-bound** when configured; non-loopback without TLS is rejected loud (W4).
- `local`/`client` roles unchanged and `modeled`; embedded fast path byte-for-byte unchanged (R-10);
  no new consensus/consistency level (R-1).
- **TD-0008 marked Resolved**; the `0.57` `source` honesty note updated; `0.58` W4 re-pointed to the
  real daemon cluster.
- `releases.toml` + `INDEX.md` updated; `0.59.0` header matches the manifest (doc-check).
