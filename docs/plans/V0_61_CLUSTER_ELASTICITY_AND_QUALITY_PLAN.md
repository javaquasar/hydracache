# HydraCache 0.61.0 Cluster Elasticity Completion & Quality — Codex Execution Plan

> **At a glance**
> - **What:** finish what `0.60` deliberately left named-open: (**W1**) the **late-start daemon
>   join bootstrap** — a fourth daemon started after a formed cluster joins over the network and
>   becomes a raft voter end-to-end (the TD-0011 residual); (**W2**) the **operator scale claim** —
>   the StatefulSet template gets deterministic pod identity, advertised cluster endpoints, and an
>   ordinal-aware bootstrap-vs-join startup path, with kind E2E proving
>   `HydraCacheCluster.spec.replicas` changes flow through to raft voters via the deployed daemon;
>   (**W3**) the **kind chaos injector** for `NetworkPartition`/`SlowDisk` (today
>   observe-only, soak_kind.rs:159-168 — the `0.58` W4 residual); (**W4**) the **coverage ratchet**
>   + the targeted fast tests TD-0009 names (post-`0.60` clean baseline: 87.77% lines,
>   2026-07-06); (**W5**) docs/gates/ledger closure — TD-0011 Resolved, TD-0009 Resolved.
> - **Why:** `0.60` made the grid securable and shrinkable, but **growable is still not a claim**:
>   a late daemon fabricates a divergent voter set in its fresh durable log, deadlocks on
>   `wait_for_raft_leader` before it can even be admitted, and is unroutable from followers
>   because the replicated `MemberUpsert` carries no endpoints. The operator also currently renders
>   every pod with a uniform bind address (`0.0.0.0:7000`) and DNS seeds, while the daemon topology
>   code still expects routable per-node endpoints. Until W1/W2 land, the `0.56` operator can scale
>   **pods** but not the **quorum** — the last gap in the `1.0`
>   "production-ready cluster out of the box" claim. W3/W4 close the two remaining named
>   residuals so the ledger reaches `1.0` with only the permanent TDs open.
> - **After (depends on):** `0.60.0` (ConfChange voter add/remove + persisted `ConfState`,
>   leader-side `sync_raft_voters` promotion, `SharedRaftPeers`, persistent `node-identity.json`,
>   `Forwarded` proposal honesty, drive diagnostics, the nightly networked-E2E CI tier).
> - **Unblocks:** `1.0` (API freeze/semver + soak mileage remain; no new grid mechanics after
>   this).
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> debt tracked: [`../technical-debt/TD-0011-dynamic-raft-membership-and-node-identity.md`](../technical-debt/TD-0011-dynamic-raft-membership-and-node-identity.md),
> [`../technical-debt/TD-0009-coverage-ratchet-and-coverage-run-stability.md`](../technical-debt/TD-0009-coverage-ratchet-and-coverage-run-stability.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its Definition
of Done **and** `cargo xtask verify`; never push red. Multi-daemon tests stay network-gated and
skip-graceful; kind tests stay kind-gated and skip-graceful.

## Preflight (verified against the repo at `0.60.0`)

Every claim below was re-read from the post-`0.60` code (commits `deeb2c9`…`df37c21`):

- **The `0.60` machinery W1 builds on is shipped and works for the bootstrap cohort.**
  `crates/hydracache-server/src/grid_host.rs`: leader-side voter promotion `sync_raft_voters`
  (:560-591) proposes `propose_add_voter` for any committed metadata member whose raft id is
  missing from `voter_ids()` and present in the peer map — **the leader side of late-join already
  exists**; a live routing table `SharedRaftPeers` (:43, seeded at :152, refreshed each drive tick
  by `refresh_raft_peers` :524-558); persistent identity `node-identity.json`
  (`resolve_member_identity` :980-1022, fail-loud future-format/mismatch/collision);
  drain-removes-voter (`begin_drain` :1236-1239 → `try_remove_local_voter_for_drain` :1330-1350
  over `request_remove_voter`, raft lib.rs:804 → `request_voter_change` :1216-1237, which requires
  a known leader and proposes single-step `ConfChange`); quorum over reachable raft voters
  (`has_quorum` :1264-1279); `Forwarded` proposal status (raft lib.rs:273) with apply-wait in
  `join_member`.
- **Why a late joiner still cannot start — four concrete blockers.**
  1. **It fabricates a divergent voter set.** `raft_topology` (grid_host.rs:698-738) always
     inserts *self* into peers, and `normalize_voters` in the raft crate **always chains
     `raft_node_id` into the voter list** (lib.rs:203-215) — so a 4th daemon with
     `seeds = [three existing]` initializes its fresh durable log with a **4-voter** `ConfState`
     while the running cluster's committed `ConfState` has 3. Its log is born divergent from the
     cluster it wants to join.
  2. **Admission deadlock at startup.** The gossip candidate announce happens only inside
     `HydraCache::member().start()` (`networked_member_cache`, :217-236), which runs **after**
     `wait_for_raft_leader` (:185-190, 5s deadline :38/:913-926). The leader can only admit (and
     then promote) a node it has seen as a **candidate** — so the joiner times out waiting for a
     leader it can never learn about. This is the exact TD-0011 "Remaining Gap" sentence.
  3. **Followers cannot route to the joiner.** The replicated
     `RaftMetadataCommand::MemberUpsert` carries **no endpoints** (`hydracache` control_plane.rs:
     73-79), and `refresh_raft_peers` (:538-557) reads addresses only from
     `member.endpoints.control` — populated on the **leader** (it materializes the gossip
     candidate, which carries endpoints) but `None` on followers (they materialize the replicated
     command). Chitchat already disseminates the control endpoint as gossip KV
     (`hydracache.endpoint.control`, chitchat lib.rs:67, written by announce :464, read into
     candidates :630) — the address is on every node's gossip view, just not folded into the
     routing table.
  4. **Election-tick rank is wrong for a non-voter.** `election_tick_for` (:760-767) uses the
     node's position in the voter list and `unwrap_or(0)` — a joiner (absent from the list) gets
     the **most aggressive** tick (5), the opposite of what a catching-up node should have.
- **Restart vs join is already distinguishable.** `durable_with_config` initializes the
  `ConfState` **only when the durable log is empty** (raft lib.rs, `voters.is_empty()` guard) — a
  restarting member with a non-empty log ignores the configured voter set entirely. So "join
  mode" only needs to govern **first boot**; restart semantics stay untouched (R-10).
- **No raft log compaction exists anywhere** — the metadata log is replayed in full to a new
  follower via ordinary appends; the leader never generates snapshots. This makes full-log
  catch-up the correct v1 join transfer and makes snapshot-based catch-up a **named boundary**,
  not silent scope.
- **Operator gap (W2).** The `0.56` operator scales the StatefulSet, and the `0.57.1` kind E2E
  asserts pod lifecycle — but nothing asserts that a scale-up pod becomes a raft **voter** through
  the deployed daemon. TD-0011's revisit trigger "the operator asserts member-count changes
  through the deployed daemon" is exactly this. There are two concrete wiring gaps, not just a
  missing test: `resources.rs::server_env` currently renders the same bind address for every pod
  (`HYDRACACHE_CLUSTER_ADDR=0.0.0.0:7000`) and no explicit `HYDRACACHE_NODE_ID`, while `seed_list`
  emits headless-Service DNS names. The daemon's `RaftPeer`/`raft_topology` path currently stores
  `SocketAddr` and only parses socket-address seeds. W2 must therefore make the operator-managed
  pod identity and advertised cluster endpoint deterministic before it can honestly claim
  `replicas => voters`. The pod config template must also select bootstrap-vs-join mode per pod
  using a Kubernetes-feasible mechanism: StatefulSet has one pod template, so per-ordinal behavior
  needs a `HOSTNAME`/ordinal-aware entrypoint or equivalent, not a static env var that pretends
  each pod has a different template.
- **Chaos injector residual (W3).** `soak_kind.rs::inject` deletes a pod for `PodCrash` but
  merely prints "requires the external kind chaos injector" for `NetworkPartition`/`SlowDisk`
  (:151-179, `requires_external_injector` :47-49); `SCOPE_DISCLOSURE` (:17) documents it. Default
  kind CNI (kindnet) does **not** enforce `NetworkPolicy` — a partition injector must probe
  enforcement or it will be wrong-but-green.
- **Coverage state (W4).** TD-0009: run-stability closed; post-`0.60` clean baseline re-measured
  2026-07-06 — **Regions 86.69% / Functions 84.88% / Lines 87.77%** — recorded "for ratchet
  planning only"; the ratchet + the named targeted tests (operator `controller.rs`, NATS/Redis
  transports, server `config.rs`, `sqlx_outbox.rs`, `vopr.rs` CLI) are deferred to exactly this
  slice ("post-`0.60.0` quality-hardening slice", TD-0009:10-13).

## Release Theme

Make the networked grid **elastic and finished**: a daemon can join a running cluster and become a
voter, the operator's replica count is the quorum's size, the kind soak injects all three fault
classes for real, and the coverage floor is enforced by CI instead of by habit. Completion over
shipped mechanics — **no new consensus, no new consistency level (R-1)**.

## Non-Goals

- **No joint consensus / no learner stage.** Voter changes stay single-step `ConfChange`
  (AddNode/RemoveNode), one in flight at a time, leader-proposed — the `0.60` discipline. A
  raft-rs learner (`AddLearnerNode`) staging phase is a named future refinement, not `0.61`.
- **No raft log compaction / snapshot catch-up.** The metadata log is small (membership
  commands); a joiner catches up by full log replay over ordinary appends. Compaction + snapshot
  transfer is a named boundary for when the metadata log grows real weight.
- **No autoscaling policy.** W2 makes `spec.replicas` changes *truthful*, not automatic; deciding
  replica counts stays with humans/the operator's user.
- **No new fault classes in the soak.** W3 implements the two already-named observe-only faults;
  it does not add new ones.
- **Fast path, `local`/`client` roles, embedded behavior — byte-for-byte unchanged (R-10).** The
  join path activates only via the new explicit config switch; every existing config keeps the
  `0.60` bootstrap behavior.
- **The coverage ratchet is a mechanical floor, not a score (R-7).** No numeric self-assessment
  anywhere; the gate is boolean (`--fail-under-lines`).

## Technical-debt scope & downstream obligations (do not lose)

| TD / obligation | In `0.61`? | Detail |
| --- | --- | --- |
| **TD-0011** late-start join bootstrap (the residual) | **Closed here** | W1 implements the join path; W2 proves it through the operator. W5 marks TD-0011 **Resolved** — the `0.60` sub-items are already resolved in place. |
| **TD-0009** coverage ratchet + targeted tests | **Closed here** | W4 lands the named targeted tests, documents the thin-binary policy, and enables the `--fail-under-lines` ratchet as a scheduled CI gate. W5 marks TD-0009 **Resolved** with the ratchet-raise ladder recorded. |
| **`0.58` W4 residual** — external chaos injector | **Partition closed here; slow-disk executable when present** | W3 implements `NetworkPartition` via enforced `NetworkPolicy` (with an enforcement probe so a non-enforcing CNI skips loud, never wrong-but-green). `SlowDisk` is injected via chaos-mesh `IOChaos` **when its CRDs are present**; otherwise the residual is narrowed to a named external dependency and `SCOPE_DISCLOSURE` says exactly that. |
| TD-0002 raft/protobuf, TD-0003 bucket C, TD-0004 placement, TD-0005 Java artifact | **Out of scope** | Untouched. W1 uses the already-shipped `ConfChange` path — no new protobuf surface. |

**Forward note for `1.0`:** after `0.61` the open ledger should contain only the permanent/blocked
entries (TD-0002 upstream, TD-0003 bucket C, TD-0004, TD-0005). The remaining `1.0` work is API
freeze/semver + soak mileage — no grid mechanics.

## Dependency Graph

```
W1 late-start join bootstrap (daemon + raft crate) ─► W2 operator scale E2E (kind) ─┐
W3 kind chaos injector (partition / slow-disk) ─────────────────────────────────────┼─► W5 docs + gates + TD-0009/0011 closure
W4 coverage ratchet + targeted fast tests ──────────────────────────────────────────┘
```

## W1. Late-start daemon join bootstrap

**Goal.** A daemon started **after** a cluster has formed joins it over the network: it announces
itself via gossip, is admitted into metadata by the leader's bridge, is promoted to a raft voter by
the already-shipped `sync_raft_voters`, catches up the metadata log, and only then reports ready —
closing the four preflight blockers and the TD-0011 residual. Everything below is plumbing around
shipped mechanics; the only *new* moving part is the joiner's startup mode.

### W1a. Explicit start mode in config (no heuristics)

**Files.** `crates/hydracache-server/src/config.rs`.

**Design.** `ServerConfig` gains `cluster_start: ClusterStartMode` and `join_timeout_ms: u64` with
`#[derive(Default)] enum ClusterStartMode { #[default] Bootstrap, Join }` (TOML: `cluster_start =
"bootstrap" | "join"`, env `HYDRACACHE_CLUSTER_START`; timeout env
`HYDRACACHE_JOIN_TIMEOUT_MS`, default `15_000`). **Explicit beats heuristics**: an auto-detect
rule like "self ∉ seeds ⇒ joiner" would silently flip existing deployments where each node lists
only its *peers* as seeds (all three would become joiners and nobody would campaign — a formation
deadlock). Default `Bootstrap` keeps every existing config byte-identical (R-10). `validate()`
gains fail-loud rules: `Join` with empty `seeds` → new `ServerConfigError::JoinRequiresSeeds`;
`Join` with `role != Member` → rejected (join mode is a member-grid concept); zero
`join_timeout_ms` → rejected rather than spinning forever or failing immediately without context.

**Advertised endpoint rule.** Operator-managed pods cannot advertise the bind address
`0.0.0.0:7000`. If W2 cannot express a routable per-pod endpoint using the existing
`cluster_addr`, this release adds an explicit advertised cluster endpoint field/env (for example
`cluster_advertise_addr` / `HYDRACACHE_CLUSTER_ADVERTISE_ADDR`) whose default is the existing
`cluster_addr` string. The advertised endpoint is used for gossip candidates and raft peer routing;
the bind address remains the listener socket. If this field is persisted, transmitted, or exposed
through a compatibility-sensitive shape, register it under R-4 in `docs/COMPAT.md`.

**Precedence rule (restart beats mode).** The mode governs **first boot only**. If the durable
raft log in `storage_dir/raft-log` is non-empty, the daemon is a *returning member* regardless of
`cluster_start` — `durable_with_config` already ignores the configured voter set when the stored
`ConfState` is non-empty (raft lib.rs `voters.is_empty()` guard), and `node-identity.json`
(grid_host.rs:980-1022) pins its identity. Document this precedence in the config rustdoc so a
scale-up pod that restarts does not need its config edited.

### W1b. Joiner raft configuration — do not fabricate a voter set

**Files.** `crates/hydracache-cluster-raft/src/lib.rs`,
`crates/hydracache-server/src/grid_host.rs`.

**Design (raft crate).** `normalize_voters` (lib.rs:203-215) unconditionally chains
`raft_node_id` into the voter list — correct for a bootstrap cohort, wrong for a joiner. Add a
fallible constructor variant instead of a flag on the existing one:

```rust
/// Build a runtime configuration for a node JOINING an existing voter set.
/// `remote_voters` is the seeds-derived view of the running cluster; self is
/// deliberately NOT added — the leader admits this node via ConfChange.
pub fn try_joining<I>(
    cluster_name: impl Into<String>,
    raft_node_id: u64,
    remote_voters: I,
) -> CacheResult<Self>
where I: IntoIterator<Item = u64> {
    let raft_node_id = raft_node_id.max(1);
    let voters = normalize_remote_voters(remote_voters); // sort/dedup/max(1), NO self-chain
    // fail loud instead of quietly bootstrapping or panicking:
    ensure: voters is non-empty;
    ensure: !voters.contains(&raft_node_id);
    Ok(Self { cluster_name, raft_node_id, voters, auto_campaign: false, …defaults })
}
```

raft-rs accepts a `RawNode` whose id is absent from the storage `ConfState`: the node runs as a
plain follower — it appends what the leader sends and cannot vote or be counted — and becomes a
voter the moment the leader's `ConfChange AddNode` entry commits and is applied
(`request_voter_change` → `apply_conf_change` → `save_conf_state`, all shipped in `0.60`). Add a
loud error (not a panic/debug-assert) if `try_joining()` is constructed with itself in
`remote_voters` — that is a config lie.

**Load-bearing proof first.** The raft-rs "node outside ConfState can receive appends and later
become a voter" assumption must be proved before server wiring lands. The first W1b commit adds
`runtime_outside_conf_receives_appends_and_becomes_voter`; if that fails, stop and re-scope rather
than building a server path around an invalid assumption.

**Remote-voter contract.** `remote_voters` must represent the already-formed cluster's current
voter set, not an arbitrary or desired future replica set. For operator scale-up this means seeds
and bootstrap metadata come from `status.bootstrap_replicas`/the original cohort, not from the
post-scale desired replica count that includes the joiner. If the implementation cannot derive a
non-empty remote voter set without self, join mode fails loud.

**Design (grid_host).** `networked_member_stack` (:110-190) branches on the resolved mode:

```rust
let mode = resolved_start_mode(config, &raft_log_dir); // Join only if config says Join AND log is empty (W1a precedence)
let raft_config = match mode {
    Bootstrap => RaftMetadataRuntimeConfig::multi_voter(…, topology.voters.clone())
        .auto_campaign(!topology.multi_voter)
        .ticks(topology.election_tick_for(raft_node_id), 1),          // unchanged (:127-133)
    Join => RaftMetadataRuntimeConfig::try_joining(…, raft_node_id, topology.remote_voters())?
        .ticks(topology.joiner_election_tick(), 1),                   // W1e
};
```

`RaftTopology` gains `remote_voters()` (peers-derived voter ids **excluding** the local id) and
`raft_topology` keeps inserting self into `peers` (the routing table legitimately contains self,
:531-537) — only the **voter list** handed to raft changes. In `Join` mode, `multi_voter`
short-circuits to `true` (a joiner always has peers), so the `HttpRaftMessageSink` (not the noop
sink) is selected at :153-163 unchanged.

### W1c. Pre-cache candidate announce — break the admission deadlock

**Files.** `crates/hydracache-server/src/grid_host.rs`,
`crates/hydracache-cluster-chitchat/src/lib.rs` (only if `announce` needs a pre-cache-safe
variant; expected: no change).

**Design.** The leader admits gossip **candidates**; today the joiner only becomes a candidate
inside `HydraCache::member().start()` — after the wait it can never pass (blocker 2). In `Join`
mode, immediately after `ChitchatDiscovery::spawn_udp` (:138-149) and **before** any wait:

```rust
discovery
    .announce(
        ClusterCandidate::member(node_id.clone())
            .generation(generation)
            .endpoints(ClusterEndpoints::new().control(config.cluster_addr.to_string())),
    )
    .await?;
```

This writes the candidate + `hydracache.endpoint.control` into gossip KV (chitchat lib.rs:464),
which the running members' admission drive (`spawn_admission_drive`, :481-497) turns into a
committed `MemberUpsert`, which the leader's `sync_raft_voters` (:560-591) turns into
`ConfChange AddNode` — the entire admission→promotion pipeline is shipped; this one announce is
the missing first domino. The later announce inside the cache start is idempotent (same node id +
generation → `Duplicate`, dedup shipped in `0.59`). Do **not** pre-announce in `Bootstrap` mode —
keep the formed-cluster path byte-identical (R-10).

### W1d. Fold gossip addresses into the routing table — make the joiner routable from followers

**Files.** `crates/hydracache-server/src/grid_host.rs`.

**Design.** Blocker 3: followers materialize the replicated `MemberUpsert` **without** endpoints
(control_plane.rs:73-79 has no endpoint field), so `refresh_raft_peers` (:524-558) never learns
the joiner's address on non-leader nodes — vote requests/responses and post-re-election appends
to the joiner would fail with "no HTTP raft peer endpoint" (:829-840). Fix at the dissemination
layer, **not** by widening the replicated command:

```rust
fn refresh_raft_peers(
    raft_peers: &SharedRaftPeers,
    local_node_id: &ClusterNodeId,
    local_addr: SocketAddr,
    members: &[ClusterMember],
    candidates: &[ClusterCandidate],   // NEW: discovery.candidates() from the drive loop
) {
    // 1) self + raft-committed members (authority) — unchanged (:531-557)
    // 2) gossip candidates FILL ADDRESS GAPS ONLY: entry(raft_id).or_insert(…)
    //    never overwrite, never remove — addresses are routing hints (R-1:
    //    authority = raft/epoch; gossip = dissemination).
}
```

`drive_grid_once` (:499-522) passes `discovery.candidates()` in — the drive loop already owns the
discovery handle's sibling objects; thread `Arc<ChitchatDiscovery>` into `spawn_grid_drive`
(:445-479). Extending `MemberUpsert` with endpoints is explicitly **rejected** for `0.61`: the
command envelopes are encoded into the durable raft log (R-4 wire+durable surface, old-log
decode compatibility, format-version dance) for information gossip already carries.

**Conservative candidate filter.** Candidate addresses are never authority. The fold-in only fills
missing routing hints for member-role candidates at the local generation, ignores candidates with
empty/unparseable endpoints, never overwrites member-sourced addresses, and never removes peers.
Prefer filling only after the node is present in raft-committed metadata; if the implementation
needs a pre-metadata hint to let the leader send `AddNode`, keep that path leader-local and cover
the stale-candidate rejection separately. This keeps R-1 crisp: raft membership decides *who* is a
member; gossip only tells us *where* to try sending bytes.

### W1e. Join-complete wait + joiner election tick

**Files.** `crates/hydracache-server/src/grid_host.rs`, `crates/hydracache-server/src/config.rs`.

**Design.** Replace the joiner's use of `wait_for_raft_leader` (:913-926) with:

```rust
async fn wait_for_join_complete(
    raft: &Arc<NetworkedRaftRuntime>,
    raft_node_id: u64,
    deadline: Duration,               // config.join_timeout_ms, default 15_000
) -> CacheResult<()> {
    // ready ⇔ leader known AND self ∈ voter_ids() — i.e. the AddNode committed
    // and was applied locally (which also implies metadata catch-up has begun,
    // since conf and metadata ride the same log).
    // On timeout: fail LOUD with a joiner-specific message naming the three
    // usual causes — seeds unreachable, auth/TLS posture mismatch with the
    // cluster, or no live leader — never fall back to bootstrapping alone.
}
```

`GRID_LEADER_WAIT_TIMEOUT` (5s, :38) stays for the bootstrap cohort; the joiner deadline is
config-surfaced (`join_timeout_ms`) because operator environments legitimately take longer than
loopback tests (image pull, gossip convergence). A joiner that times out **exits with an error**
— it must never campaign or degrade into a 1-node cluster (R-3).

Fix blocker 4 alongside: `RaftTopology::joiner_election_tick()` returns
`5 + 2 * (voters.len() + 1)` — strictly lazier than every bootstrap member
(`election_tick_for`, :760-767), so a freshly promoted joiner does not immediately contest the
incumbent leader; drop the `unwrap_or(0)`.

### W1f. Drain/restart symmetry (verification, expected no code)

Once a joiner is a voter, the `0.60` paths must apply to it unmodified: graceful drain removes it
(`try_remove_local_voter_for_drain`, :1330-1350), crash keeps it in the voter set, restart with
the same `storage_dir` re-enters as a returning member (W1a precedence). W1's E2E asserts all
three on the *joined* node specifically — if any needs code, that is a `0.60` regression to fix,
not new design.

**Step-by-step (implementation order).**
1. W1a config + validation (+ unit tests) — no behavior change yet.
2. Raft crate `try_joining()` + `normalize_remote_voters` + loud self-in-remote-voters error (+ crate
   tests) — additive.
3. W1d routing fold-in (+ unit test) — additive and independently useful (heals any
   address gap, not only joins).
4. W1b/W1c/W1e grid_host branch: mode resolution, joiner raft config, pre-announce,
   `wait_for_join_complete`, joiner tick.
5. E2E + negative tests; wire the new tests into the existing nightly tier (ci.yml:159-162 —
   the `multi_node` name filter already matches tests named `multi_node_*`; name the new E2Es
   accordingly or widen the filter in the same commit).

**Tests & requirements.**
- Raft crate (`crates/hydracache-cluster-raft`):
  - `joining_config_excludes_self_from_voters` (unit; falsifiable: `multi_voter` would include
    it).
  - `joining_config_with_self_in_remote_voters_fails_loud`.
  - `runtime_outside_conf_receives_appends_and_becomes_voter` (runtime-level, extends the
    `NetworkedRuntimeCluster` harness: 3-voter cluster + a `try_joining()` runtime → leader
    `propose_add_voter` → joiner applies the conf change, `voter_ids()` contains it, replicated
    `MemberUpsert`s materialized on it — full-log catch-up proven at runtime level).
- Server unit (`grid_host.rs` tests module):
  - `join_mode_requires_seeds_and_member_role` (config validation).
  - `refresh_raft_peers_folds_gossip_candidate_addresses` (falsifiable: without the fold-in the
    peer map lacks the candidate; with it, `or_insert` fills the gap and never overwrites a
    member-sourced address).
  - `refresh_raft_peers_ignores_stale_or_non_member_candidates`.
  - `restart_with_nonempty_log_ignores_join_mode` (W1a precedence).
- E2E (`crates/hydracache-server/tests/grid_host.rs`, network-gated, same env flag +
  `grid_env_lock` + `reserve_loopback_addrs` harness):
  - `multi_node_fourth_daemon_joins_running_cluster_as_voter` — start 3 (bootstrap), converge;
    start a 4th with `cluster_start = "join"`, `seeds` = the three → all four report
    `members == 4`, **4 voters**, one leader, `quorum_ok`; then **kill one original member** and
    assert re-election succeeds among the remaining 3 of 4 — provable only if the joiner is a
    real counted voter (falsifiable: a metadata-only "member" would leave 2/3 and stall).
  - `multi_node_joiner_with_unreachable_cluster_fails_loud` — `join` mode, dead seeds → startup
    error within `join_timeout_ms` naming the cause; **no** self-bootstrap (falsifiable: the
    `0.60` code would form a 1..4-voter cluster of its own).
  - `multi_node_drained_joiner_leaves_voter_set` — drain the 4th; survivors report 3 voters
    (W1f symmetry).
- Run: `cargo test -p hydracache-cluster-raft --locked`,
  `cargo test -p hydracache-server --locked grid_host`, and the network-gated tier from GATES.md.

**Risk & rollback.** The joiner path is strictly additive behind `cluster_start = "join"`; revert
returns every config to `0.60` bootstrap semantics. The known sharp edge is a joiner configured
against a *partially* formed cluster (no leader yet): `wait_for_join_complete` fails loud and the
process exits — the operator retries the pod; document this as the expected crash-loop-until-
cluster-ready behavior in the runbook (it is honest backpressure, not a bug).

## W2. Operator scale claim: replicas ⇒ voters, end-to-end (kind)

**Goal.** Discharge TD-0011's operator trigger: changing `HydraCacheCluster.spec.replicas` on a
running cluster changes the **raft voter count** through the deployed daemons — scale-up pods
join via W1, scale-down pods drain via `0.60`.

**Files.** `crates/hydracache-operator/src/crd.rs` (`status.bootstrap_replicas`),
`crates/hydracache-operator/src/controller.rs` (record/preserve status),
`crates/hydracache-operator/src/resources.rs` (pod template/env/command rendering),
`crates/hydracache-operator/tests/e2e.rs` / `soak_kind.rs` (assertions),
`docs/daemon-member-mode.md` (operator section), and `docs/COMPAT.md` if the CRD/status schema
entry is added or revised.

**Steps.**
1. **Status field + upgrade semantics.** Add `status.bootstrap_replicas`, set once when the
   cluster first has an owned StatefulSet. For new installs it is the install replica count. For
   pre-`0.61` clusters where the field is absent, initialize it from the existing StatefulSet's
   current/spec replicas, not the requested post-upgrade scale target. `observed_status` and every
   status patch path must preserve the field; losing it would make pod start mode non-deterministic.
   Register the CRD/status schema compatibility rule in `docs/COMPAT.md` or explicitly document why
   this status-only additive field is outside the compatibility table.
2. **Deterministic pod identity and advertised endpoint.** The current template renders
   `HYDRACACHE_CLUSTER_ADDR=0.0.0.0:7000`, no `HYDRACACHE_NODE_ID`, and DNS seeds. That is a bind
   address, not a raft peer endpoint. W2 must render or compute per-pod `node_id` (normally the
   pod name) and a routable advertised cluster endpoint (normally
   `<pod>.<headless-service>:7000`). If the server still requires `SocketAddr`, W2/W1 must either
   make `RaftPeer` endpoint storage DNS-capable or resolve DNS before inserting peers; do not claim
   the operator path while routing depends on `0.0.0.0` or socket-only seed parsing.
3. **Ordinal-aware start mode in a single StatefulSet template.** Pods with ordinal
   `< status.bootstrap_replicas` get bootstrap mode; pods with ordinal `>= status.bootstrap_replicas`
   get join mode. Because StatefulSet has one pod template, implement this through a real
   `HOSTNAME`/ordinal-aware startup command, init wrapper, or equivalent deterministic mechanism.
   A single static `HYDRACACHE_CLUSTER_START` env var is not sufficient. W1a's restart precedence
   makes pod restarts safe even if the wrapper recomputes the same mode.
4. **Seed rule.** Bootstrap pods use the original bootstrap cohort as the initial voter set.
   Joiner pods seed from the bootstrap/live cohort, not from the desired post-scale replica count
   that includes themselves. This prevents a scale-up pod from deriving a voter set that already
   contains the joiner.
5. **Scale-down already drains** (0.56 drain-before-remove + `0.60` voter removal); assert it now
   reaches the voter set.
6. **E2E assertions** (kind-gated, skip-graceful — same `KindHarness` pattern as soak_kind.rs
   :116-149): apply replicas=3 → converge (leader, 3 voters via `/cluster/overview`); patch
   replicas=4 → the new pod joins, **4 voters**, quorum true; patch replicas=3 → drained pod
   leaves, **3 voters**; falsifiable contrast: **delete** a pod (crash, not drain) → voter count
   stays until the pod returns (kubelet restarts it; same `storage_dir` PVC → returning member).
7. Update the operator runbook section: scale-up latency expectations (join wait), the
   crash-loop-until-ready note from W1.

**Tests & requirements.**
- `kind_scale_up_adds_raft_voter_through_daemon_join` (kind-gated).
- `kind_scale_down_drains_voter_through_daemon` (kind-gated).
- `kind_pod_crash_does_not_shrink_voters` (kind-gated, falsifiable contrast).
- PR-tier: `bootstrap_replicas_is_recorded_once_and_pods_get_deterministic_start_mode`
  (controller unit test over the rendered pod template — no cluster needed).
- PR-tier: `operator_template_renders_routable_cluster_identity_and_endpoint`
  (asserts no raft peer advertises `0.0.0.0`, pod identity is stable, and DNS/socket endpoint
  handling matches the server contract).
- Run: `cargo test -p hydracache-operator --locked` (fast tier) + the kind commands in GATES.md.

**Risk & rollback.** Confined to the operator's config rendering + tests; revert restores
scale-pods-without-quorum (the honest `0.60` state, still documented in TD-0011).

## W3. Kind chaos injector: partition for real, slow-disk honestly

**Goal.** Close the `NetworkPartition` part of the `0.58` W4 residual: `inject()`
(soak_kind.rs:151-170) stops printing disclaimers for partitions and injects them when the CNI
actually enforces `NetworkPolicy`. `SlowDisk` becomes executable when a chaos-mesh installation is
present and stays a named external dependency otherwise. Never wrong-but-green.

**Steps.**
1. **Partition = `NetworkPolicy` + enforcement probe.** Injector applies a deny-all ingress+egress
   `NetworkPolicy` selecting the target pod. Because default kind CNI (kindnet) does **not**
   enforce NetworkPolicy, the injector first runs a **probe**: apply the policy to a probe target
   and verify connectivity actually drops within a bounded deadline; if it does not, **skip the
   partition leg loud** ("CNI does not enforce NetworkPolicy — install calico/cilium in the kind
   config") — the same skip-graceful discipline as every other gate (R-5). `heal()` deletes the
   policy and waits for recovery.
2. **Slow-disk = chaos-mesh `IOChaos` when present.** Detect the chaos-mesh CRD
   (`iochaos.chaos-mesh.org`) via the discovery API: present → apply a latency `IOChaos` against
   the pod's volume for the fault window and remove it in `heal()`; absent → keep the current
   observe-only disclosure line. No chaos-mesh dependency is vendored; the test consumes the CRD
   dynamically (`kube::api::DynamicObject`) so the operator crate gains no new build dependency.
3. **`SCOPE_DISCLOSURE` (:17) rewrite** to state the new truth: partition is injected when the
   CNI enforces policy; slow-disk is injected when chaos-mesh is installed; each leg names its
   skip condition. Update the assertion that pins the disclosure text.
4. **Soak assertions unchanged** — quorum kept, leader present, no lost committed writes
   (:63-107) now run against *actually injected* partitions.

**Tests & requirements.**
- `kind_partition_injection_isolates_and_heals` (kind+calico-gated; falsifiable: during the
  partition the isolated pod's reachability degrades in `/cluster/overview` and quorum holds on
  the majority side; after heal it recovers).
- `partition_probe_skips_loud_on_non_enforcing_cni` (kind-gated: with kindnet the leg skips with
  the named message — asserting the *skip path* is exercised, the wrong-but-green trap).
- `slow_disk_uses_iochaos_only_when_crd_present` (unit-testable detection logic + kind-gated
  when chaos-mesh is installed).
- Run: the kind tier commands in GATES.md (extended with the calico/chaos-mesh notes).

**Risk & rollback.** All inside the gated soak; revert restores observe-only disclosures. The
enforcement probe is the load-bearing honesty device — it must land in the same commit as the
injector.

## W4. Coverage ratchet + targeted fast tests (TD-0009)

**Goal.** Enforce the coverage floor mechanically and close TD-0009: land the named targeted
tests, document the thin-binary policy, then switch the ratchet on — in that order, so the first
enforced floor already reflects the improved denominator.

**Steps.**
1. **Targeted fast tests** (TD-0009 step 2 list, one commit per surface):
   `hydracache-operator/src/controller.rs` (reconcile branches, failed status, finalizer/error
   transitions, status patch paths), `hydracache-transport-nats` + `hydracache-transport-redis`
   (mocked publish/subscribe, malformed frames, reconnect/resume, queue bounds,
   backpressure/error accounting), `hydracache-server/src/config.rs` (remaining invalid combos —
   `0.60`/W1a added many; cover what is still red in the HTML report),
   `hydracache-db/src/sqlx_outbox.rs` (idempotency, retry, malformed row, lag, tx/error paths),
   `hydracache-sim/src/bin/vopr.rs` (CLI argument errors, JSON report shape, failure exit codes,
   report-writing paths). Every test deterministic, PR-tier, named per file in the PR.
2. **Thin-entrypoint policy** (TD-0009 step 3): document in `docs/TESTING.md` which `main.rs`
   wrappers are excluded from coverage-chasing and why; add CLI smoke tests where a wrapper
   carries real behavior.
3. **Re-measure**, then **enable the ratchet as a scheduled CI job** (not inside
   `cargo xtask verify` — a full instrumented workspace build does not belong in the fast gate):
   `cargo llvm-cov --workspace --all-targets --locked --summary-only --fail-under-lines <N>`.
   First `N` = `max(88, floor(post-step-1 clean line baseline))` unless the targeted tests expose a
   real coverage-accounting regression that is documented in TD-0009. The current post-`0.60`
   baseline is 87.77%, so a floor of `87` would permit avoidable drift; do not use it as the first
   ratchet unless the re-measure honestly falls below 88 for a documented reason. Record the raise ladder
   (`88 → 89 → 90` as surfaces land) in TD-0009. The job runs in the nightly workflow beside the
   soak tier and fails the nightly loudly. Upload or retain inspectable summary/LCOV/HTML artifacts
   for failures so the nightly is actionable, not just red.
4. GATES.md gains the ratchet row (Where = "CI nightly"); RULES R-7 note restated: mechanical
   floor, not a self-score.

**Tests & requirements.** Step 1's tests are the deliverable; the gate for W4 itself is: the
scheduled job exists in `.github/workflows/ci.yml`, fails on an artificial drop (verified once by
running with `--fail-under-lines 99` locally — falsifiability check, not committed), and passes
at the chosen floor. Run: `cargo llvm-cov --workspace --all-targets --locked --summary-only`.

**Risk & rollback.** A flaky-under-instrumentation test would block nightly — precedent and fix
pattern are already recorded in TD-0009 (assert semantics, not race winners). Revert disables the
job; the baseline record stays.

## W5. Docs, gates, and ledger closure

**Goal.** The ledger after `0.61`: TD-0011 **Resolved** only if W1+W2 are both green end-to-end;
TD-0009 **Resolved** only if the targeted tests and scheduled ratchet land; the `0.58` W4 residual
is either closed for partition and narrowed to the named slow-disk-without-chaos-mesh case, or left
open with an updated target. Roadmap updated.

**Files.** `docs/technical-debt/TD-0011-…` (Resolved: join path + operator claim + verification
commands), `docs/technical-debt/TD-0009-…` (Resolved: ratchet enabled, ladder recorded),
`docs/daemon-member-mode.md` (join mode: config, timeout, crash-loop-until-ready semantics,
operator scale walkthrough), `docs/GATES.md` (kind chaos + coverage-ratchet rows, widened
networked-E2E command if the name filter changed), `docs/plans/V0_58_…` (W4 residual note
updated), `docs/COMPAT.md` (CRD/status and any advertised-endpoint compatibility note if changed),
`releases.toml` + `INDEX.md` + this plan's header — flipped together, `cargo xtask doc-check`
green.

**Steps.**
1. Runbook: "growing the cluster" (join config + operator scale) and "shrinking the cluster"
   (drain) as two symmetric walkthroughs with the observable `/cluster/overview` transitions at
   each step.
2. TD closures with their verification command blocks (the TD discipline: a Resolved entry names
   how to re-verify).
3. Ship-flip of the manifest triple only when every gate below is green; anything missed is
   re-scoped **in writing** here (R-7/R-11).

## Test coverage matrix (every new artifact has a named test)

| New code | Source file | Covering test(s) | Tier |
| --- | --- | --- | --- |
| `ClusterStartMode` config + validation (W1a) | `hydracache-server/src/config.rs` | `join_mode_requires_seeds_and_member_role`, `restart_with_nonempty_log_ignores_join_mode` | PR |
| `RaftMetadataRuntimeConfig::try_joining` (W1b) | `hydracache-cluster-raft/src/lib.rs:203` area | `joining_config_excludes_self_from_voters` (falsifiable), `joining_config_with_self_in_remote_voters_fails_loud` | PR |
| joiner runtime catch-up (W1b) | `hydracache-cluster-raft/tests/networked_raft.rs` | `runtime_outside_conf_receives_appends_and_becomes_voter` | PR |
| gossip address fold-in (W1d) | `grid_host.rs:524-558` | `refresh_raft_peers_folds_gossip_candidate_addresses` (falsifiable both directions) | PR |
| pre-announce + join wait + joiner tick (W1c/W1e) | `grid_host.rs:110-190/:913-926` | `multi_node_fourth_daemon_joins_running_cluster_as_voter` (falsifiable via kill-after-join), `multi_node_joiner_with_unreachable_cluster_fails_loud`, `multi_node_drained_joiner_leaves_voter_set` | network-gated / nightly |
| operator start-mode rendering (W2) | `hydracache-operator/src/crd.rs`, `controller.rs`, `resources.rs` | `bootstrap_replicas_is_recorded_once_and_pods_get_deterministic_start_mode`, `operator_template_renders_routable_cluster_identity_and_endpoint` | PR |
| operator scale claim (W2) | operator kind E2E | `kind_scale_up_adds_raft_voter_through_daemon_join`, `kind_scale_down_drains_voter_through_daemon`, `kind_pod_crash_does_not_shrink_voters` (falsifiable contrast) | kind / nightly |
| partition injector + probe (W3) | `hydracache-operator/tests/soak_kind.rs:151-179` | `kind_partition_injection_isolates_and_heals`, `partition_probe_skips_loud_on_non_enforcing_cni` | kind / nightly |
| slow-disk IOChaos detection (W3) | `soak_kind.rs` | `slow_disk_uses_iochaos_only_when_crd_present` | PR (detection) + kind |
| targeted coverage tests (W4) | per TD-0009 list | named per surface in each commit | PR |
| coverage ratchet job (W4) | `.github/workflows/ci.yml` | job green at floor `N`; one-off local falsifiability run at `--fail-under-lines 99` | CI nightly |

**Coverage rule (DoD):** no new public type or file lands without a row here; PR-tier tests are
deterministic and inside `cargo xtask verify`; network/kind rows are env-gated and skip-graceful.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green throughout; every gated tier skip-graceful without its env.
- A fourth daemon with `cluster_start = "join"` joins a running 3-node cluster and becomes a
  **counted raft voter** (proven by surviving a subsequent member kill); a joiner that cannot
  reach a cluster **fails loud within its deadline and never self-bootstraps** (W1, falsifiable
  both directions).
- Existing configs are byte-for-byte unchanged: `Bootstrap` is the default mode, restart
  precedence overrides the mode, no pre-announce on the bootstrap path (R-10).
- `HydraCacheCluster.spec.replicas` 3→4→3 moves the raft voter count 3→4→3 through the deployed
  daemons; a pod **crash** does not shrink the voter set (W2, kind-gated, falsifiable contrast).
  The operator path uses stable pod identity and routable advertised cluster endpoints; no raft peer
  advertises `0.0.0.0`, and DNS/socket endpoint handling is covered by PR-tier tests.
- The kind soak **injects** network partitions (with a CNI-enforcement probe that skips loud, so
  the gate can never be wrong-but-green) and injects slow-disk when chaos-mesh is present;
  `SCOPE_DISCLOSURE` states exactly that (W3).
- The coverage floor is enforced by a scheduled CI job at the recorded post-W4 baseline; the
  TD-0009 targeted tests and the thin-binary policy are landed; no numeric self-score anywhere
  (W4, R-7).
- **TD-0011 and TD-0009 marked Resolved**; the `0.58` W4 residual note updated; `local`/`client`
  roles unchanged and `modeled`; no new consensus/consistency level (R-1); embedded fast path
  unchanged (R-10).
- `releases.toml` + `INDEX.md` + plan header flipped together; `cargo xtask doc-check` green.

```powershell
# fast (PR) tier
cargo xtask verify

# focused suites
cargo test -p hydracache-cluster-raft --locked
cargo test -p hydracache-server --locked grid_host
cargo test -p hydracache-operator --locked

# network-gated tier (nightly; also runnable locally)
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
cargo test -p hydracache-server --test grid_host multi_node --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue

# kind tier (nightly / pre-release; needs kind + calico for the partition leg)
$env:HYDRACACHE_OPERATOR_KIND='1'
cargo test -p hydracache-operator --locked --test e2e -- --nocapture
cargo test -p hydracache-operator --locked --test soak_kind -- --ignored --nocapture
Remove-Item Env:\HYDRACACHE_OPERATOR_KIND -ErrorAction SilentlyContinue

# coverage floor (scheduled CI job; record-only run locally)
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

## Final Release Decision

`0.61.0` ships **only** if every gate above is green, or if the plan is re-scoped in writing before
the ship-flip:

| Missed item | Allowed release claim | TD status |
| --- | --- | --- |
| W1 late-start daemon join | **No 0.61 elasticity claim.** Do not ship a partial join path as done. | TD-0011 stays Open. |
| W2 operator replicas⇒voters | W1 may ship as daemon-only join if the plan, gates, and runbook say operator scale remains unclaimed. | TD-0011 stays Open or partial; not Resolved. |
| W3 chaos injector | W1/W2/W4 may ship; W3 residual stays named with exact skip/external dependency disclosure. | `0.58` W4 residual remains Open/narrowed. |
| W4 coverage ratchet | W1/W2/W3 may ship; TD-0009 remains Open with a fresh baseline and next target. | TD-0009 stays Open. |

W1+W2 together are the hard blocker for resolving TD-0011. A join path that exists but is unproven
end-to-end must not flip TD-0011 to Resolved (R-7/R-11).
