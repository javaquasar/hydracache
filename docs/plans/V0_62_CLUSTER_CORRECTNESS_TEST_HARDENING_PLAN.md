# HydraCache 0.62.0 Cluster Correctness Test Hardening — Codex Execution Plan

> **At a glance**
> - **What:** close the **test-infrastructure gap** between the DST simulator (core-only) and the
>   happy-path daemon E2E (one kill). Build the four harnesses the reference distributed systems in
>   this workspace all rely on, and the correctness tests they unlock: (**W1**) a deterministic
>   **message-filter transport** on the shipped `RaftMessageSink` seam (blueprint: raft-rs
>   `harness/src/network.rs`, TiKV `test_raftstore/src/transport_simulate.rs`); (**W1b**) a
>   deterministic **gossip fault transport** for chitchat discovery; (**W2**) **failpoint**
>   injection at process-crash boundaries (blueprint: TiKV `tests/failpoints/`, `fail` crate);
>   (**W3**) a **real-process multi-daemon harness** that actually `kill -9`s (blueprint: qdrant
>   `tests/consensus_tests`, curvine `MiniCluster`); (**W4**) the **membership-history linearizability
>   check** reusing the shipped `0.44` checker; (**W5**) **property/fuzz** on id-mapping + wire decode;
>   (**W6**) **golden wire/durable vectors** for rolling-upgrade compatibility; plus two code
>   **fixes** the harnesses expose (**F1** `pre_vote`, **F2** the `raft_wire_node_id` mapping bug).
> - **Why:** the grid's *algorithms* are proven in `hydracache-sim`, and formation/re-election is
>   proven once in `tests/grid_host.rs`, but everything between — asymmetric partitions, torn writes
>   at crash boundaries, stale/zombie peers, duplicate/reordered raft messages, real process death,
>   wire/log format drift — is untested. Every reference cluster in `C:\Workspace\prj\jq\cashe`
>   (TiKV, raft-rs, ScyllaDB, qdrant, curvine, tigerbeetle) invests in exactly these harnesses; the
>   `CROSS_PROJECT_IDEA_BACKLOG.md` "Cluster Load Test Suite As A First-Class Gate" item (#3) names
>   this. This release is **all tests + two small fixes they surface** — no new features (R-1/R-10).
> - **After (depends on):** `0.61.0` (the join/drain machinery under test), the shipped
>   `RaftMessageSink`/`RaftWireMessage` seam (`0.59` W1b), the `0.44` `InvariantChecker` /
>   `LinearizabilityChecker`, the `0.60` `ConfChange`/`ConfState` path.
> - **Unblocks:** confidence for the `1.0` "production-ready cluster" claim (mileage + these
>   correctness gates are the evidence; no grid mechanics remain).
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> backlog: [`CROSS_PROJECT_IDEA_BACKLOG.md`](CROSS_PROJECT_IDEA_BACKLOG.md) ·
> debt: [`../technical-debt/TD-0009-coverage-ratchet-and-coverage-run-stability.md`](../technical-debt/TD-0009-coverage-ratchet-and-coverage-run-stability.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its Definition
of Done **and** `cargo xtask verify`; never push red. Deterministic filter/failpoint/property tests
are PR-tier; real-process and randomized-topology tests are network-gated and skip-graceful.

## Reference blueprint map (verified — read these before implementing)

Every harness below is modeled on code that exists in this workspace. Paths and the exact seams are
quoted so an implementer (or reviewer) can open the original and compare behavior.

| Harness (WI) | Reference project | Exact file + seam | What to copy |
| --- | --- | --- | --- |
| Message filter (W1) | **raft-rs** (`cluster_libs/raft-rs`) | `harness/src/network.rs`: `Network` with `dropm: HashMap<Connection, f64>`, `ignorem: HashMap<MessageType,bool>`, methods `cut(one,other)` (:205), `isolate(id)` (:211), `recover()` (:222), `filter()` (:123) | the drop-map + `cut`/`isolate`/`recover` vocabulary for a per-peer, per-type filter |
| Message filter (W1) | **TiKV** (`tikv`) | `components/test_raftstore/src/transport_simulate.rs`: `trait Filter { fn before(&self, msgs: &mut Vec<RaftMessage>) -> Result<()>; fn after(...) }` (:37), `RegionPacketFilter` with `direction`/`drop_type`/`skip`/`allow(n)`/`when(cond)`/`reserve_dropped` builder (:379-508), `DropPacketFilter` (:108) | the composable `Filter` trait with `before`/`after` hooks and the builder that drops **by direction + message type + count/condition** — the shape our `FilteredRaftMessageSink` copies |
| Failpoints (W2) | **TiKV** | `tests/failpoints/cases/test_conf_change.rs`: `fail::cfg("apply_on_conf_change_1_3_1", "pause")` (:106), `fail::cfg(fp, "return")` (:168); crate `fail` | named injection points as `"panic"`/`"pause"`/`"return"` around conf-change apply — directly analogous to our torn-`ConfState` risk |
| Real-process (W3) | **qdrant** | `tests/consensus_tests/fixtures.py` + `utils.py`: `class PeerProcess(proc: Popen…)` with `kill()` (`proc.kill(); proc.wait()`), `start_first_peer`/`start_peer` spawning real binaries with peer URIs; `test_cluster_rejoin_data/`, `downscale_cluster.py` | spawn **real OS processes**, `kill()` (SIGKILL) the leader, restart against the same data dir, assert rejoin |
| Real-process (W3) | **curvine** | `curvine-tests/src/testing.rs`: `MiniCluster::with_num(&conf, master_num, worker_num)` (:92), `start_cluster()` (:65) | a typed Rust in-repo multi-process cluster builder with per-node temp dirs, as the Rust-native alternative to a Python harness |
| Membership linearizability (W4) | **(in-repo)** `hydracache-sim` | `crates/hydracache-sim/src/linearizability.rs` `LinearizabilityChecker`; `src/invariants.rs` `InvariantChecker` | feed a **recorded daemon history** into the already-shipped checker instead of only the simulated one |
| Golden vectors (W6) | **(in-repo)** + **arroyo** | `crates/hydracache-sim/tests/snapshot_schema.rs` (existing golden pattern); arroyo binary-protocol versioning (`ARROYO_KNOWLEDGE_BASE.md`) | committed byte-corpus that new code must still decode (R-4 rolling-upgrade) |
| Randomized topology soak (W3 ext) | **qdrant** / **tigerbeetle** | qdrant `downscale_cluster.py`; tigerbeetle `src/testing/` VOPR discipline | seeded random join/drain/crash/restart sequence with per-step invariant checks |
| Stale/drained peer rejection (W1/W3 ext) | **TiKV** + **Chitchat** | TiKV stale-peer tests; Chitchat tombstone/reset vocabulary in `ALGORITHM.md` | explicit tests that old messages or restarted drained storage cannot resurrect stale membership |
| Process lifecycle discipline (W3/W7) | **Pingora** | server lifecycle/readiness/drain style | bounded readiness, bounded shutdown, child reaping, and captured logs for real-process tests |
| Gossip fault filter (W1b) | **Chitchat** | `chitchat::transport::ChannelTransport`; `crates/hydracache-cluster-chitchat/src/lib.rs` `spawn_with_transport` | deterministic drop/delay/liveness-flap injection for the discovery plane, not only raft |
| Falsifiability canaries (W2/W7) | **TiKV** + in-repo R-7 rule | `fail` crate plus test-only canary toggles | prove each harness test fails against a deliberately seeded broken behavior |
| Resource sampler reuse (W3 ext) | **in-repo 0.58** | `BoundedGrowthChecker` + real RSS/fd sampler from the endurance plan | make process soaks assert bounded resource growth instead of only correctness |
| CI/platform/flake policy (W7) | **0.59-0.61 lessons** | `.github/workflows/ci.yml`, `docs/GATES.md`, `docs/TESTING.md` | every documented gate has a CI step, platform scope, and no-silent-retry policy |

## Preflight (verified against the repo at `0.61.0`)

- **The filter seam already exists and is trivially interceptable.** The daemon sends every raft
  message through `trait RaftMessageSink { async fn send(&self, message: RaftWireMessage) }`
  (`crates/hydracache-cluster-raft/src/lib.rs:257`). Production impl is `HttpRaftMessageSink`
  (`crates/hydracache-server/src/grid_host.rs:771-873`); a `NoopRaftMessageSink` (:622-630) already
  proves a drop-in alternative sink is expected. **W1 wraps a sink** — no production code changes to
  route messages through it. `RaftWireMessage { from, to, term, payload }` (raft lib.rs:224) already
  carries the routing tuple a filter needs; the raft message *type* is inside `payload` and is
  recoverable via `RaftWireMessage::decode()` (raft lib.rs:249) → `raft::eraftpb::Message::msg_type`.
- **The runtime is single-threaded/lock-driven and already deterministically steppable.** `tick`,
  `step`, `drain_ready`, `take_outbound_messages`, `campaign` (raft lib.rs:712-757) each take the
  internal `Mutex` and return outbound `Vec<RaftWireMessage>`. The existing `NetworkedRuntimeCluster`
  in `crates/hydracache-cluster-raft/tests/networked_raft.rs` already hand-drives three runtimes over
  an in-test message bus — **W1's deterministic tests extend this harness**, they do not invent one.
- **No message-filtering test harness exists.** `networked_raft.rs` delivers every message; there is
  no drop/delay/dup/reorder/asymmetric-partition capability anywhere. So none of these are tested:
  asymmetric partition (A→B dropped, B→A delivered), duplicate `ConfChange` delivery, stale-term
  injection, message reordering. TiKV's `test_prevote.rs`, `test_stale_peer.rs`, `test_transfer_leader.rs`
  are entirely built on this capability.
- **No process-crash failpoints.** There is no `fail` crate in the workspace (verified: no
  `fail`/`fail-rs` in any `Cargo.toml`). The torn-state windows are real and new in `0.60`/`0.61`:
  in `RaftRuntimeState::drain_ready` (raft lib.rs, the ready loop) the order is save-snapshot →
  save-entries → save-hard-state → apply → **advance**; a crash between `save_hard_state` and the
  send, or between committing a `ConfChange` entry and `save_conf_state`, is untested. `save_conf_state`
  is the newest durable write (`0.60` W4).
- **The E2E "kill" is not a real kill.** `multi_node_members_form_a_cluster_and_elect_one_leader`
  (`crates/hydracache-server/tests/grid_host.rs`) drops the `ServerRuntime` (its `NetworkedGridHandle`
  `Drop` sends `shutdown`, grid_host.rs:1216-1220) — a **graceful, in-process** teardown sharing one
  tokio runtime and address space with the survivors. It cannot exercise: SIGKILL mid-write, a
  persisted-vote/term double-vote after restart, or OS-level socket death. qdrant's `PeerProcess.kill()`
  (`proc.kill(); proc.wait()`) is a real SIGKILL.
- **`0.57` "no stale leader" has no falsifiable test.** The claim that `/cluster/overview` never shows
  a stale leader mid-election has never been tested against an actual partitioned-but-still-"leader"
  node — impossible without W1.
- **No gossip-plane fault harness exists.** W1 covers raft messages only. The chitchat discovery plane
  still has no deterministic way to drop graceful-leave markers, flap liveness
  (`Live -> Suspect -> Dead -> Live`), or replay a stale generation after tombstone GC, even though
  `ChitchatDiscovery::spawn_with_transport` already accepts a caller-provided transport for tests.
- **R-7 falsifiability is stated but not mechanized.** The final gate says a test must fail against a
  seeded-broken build, but no work item yet defines how to seed that broken behavior. W2's
  `test-failpoints` feature must also carry test-only bug-injection canaries.
- **Drain-vs-promotion has an untested race window.** `sync_raft_voters` promotes committed metadata
  members missing from `voter_ids()`. If `RemoveNode` commits before the corresponding `NodeLeft`
  metadata is visible, the leader's promotion cycle can try to add the draining node back unless the
  drain state is explicitly excluded.
- **Pause is not partition.** The real-process harness kills/restarts daemons, but it does not yet model
  a frozen process that wakes with expired timers (`SIGSTOP`/`SIGCONT`, Linux-only). This is a separate
  GC-pause class from packet loss.
- **Inbound snapshot apply is a cold path.** The plan covers snapshot persist/apply crash windows and
  golden bytes, but the runtime path for an inbound raft `MsgSnapshot` should either apply correctly or
  reject loudly; it must not silently corrupt if snapshot generation remains rare.
- **Two concrete code bugs the harnesses will expose (become F1/F2 below):**
  - **F1 — no `pre_vote`.** `RaftMetadataRuntimeConfig::raft_config` (raft lib.rs:233-243) leaves
    `pre_vote` at its `false` default. A node returning from a partition with an inflated term forces
    the stable leader to step down (term explosion). raft-rs supports `Config::pre_vote = true`.
  - **F2 — `raft_wire_node_id` mismatch.** `raft_wire_node_id` (grid_host.rs:1263-1267) does
    `node_id.parse::<u64>().unwrap_or_else(|_| stable_nonzero_hash(node_id))` — a node literally named
    `"42"` maps to `42`, while `raft_node_id()` (:1259-1261) always hashes. Inbound `step` (raft handler,
    grid_host.rs:840-886 area) would attribute the message to the wrong raft id. A property test makes
    this deterministic and loud.

## Release Theme

Give the cluster the **test infrastructure a distributed system needs to be trusted**: deterministic
adversarial message scheduling, crash-boundary injection, real process death, history linearizability,
format-drift golden vectors. Copy the harness shapes from the reference systems already in this
workspace. Fix only what the harnesses prove broken. **No new product features (R-1/R-10).**

## Cross-Project Strengthening Pass

The cross-project reread plan confirms that `0.62` is aimed at the right seam:
TiKV-style raft correctness, qdrant/Curvine real-process harnesses, and
TigerBeetle-style deterministic replay. It should be strengthened in four
places, without pulling future Redis/Hazelcast compatibility work into this
release.

1. **Snapshot crash windows.** W2 already covers `ConfState` and `HardState`,
   but the TiKV lesson also calls out snapshot persist/install/apply edges.
   Add failpoints around snapshot save/install/apply and a named recovery test.
2. **Stale or drained peers.** The plan says "stale/zombie peers", but the test
   list should explicitly cover old messages from a removed member and restart
   from a drained node's old data directory. This is the TiKV stale-peer and
   Chitchat tombstone/reset lesson in HydraCache terms.
3. **Replayable soak artifacts.** W3 logs the seed, but TigerBeetle discipline
   wants the full replay input: seed, operation list, node ids, ports, data dirs,
   and failing history. A failed nightly should leave enough artifacts to replay
   locally without guessing.
4. **Process lifecycle hygiene.** Pingora is not a feature dependency here, but
   its server-operability lesson applies to W3/W7: bounded readiness, bounded
   shutdown, process-group cleanup, and captured child logs are part of the
   harness contract, not test niceties.
5. **Falsifiability as a mechanism.** R-7 cannot stay a sentence in Final
   Decision. Each major harness test needs a test-only canary that deliberately
   reintroduces the bug and proves the test turns red.
6. **Gossip fault injection.** Membership dissemination has a raft plane and a
   chitchat plane. `0.62` must cover both enough to prove join/drain does not
   depend on a perfectly reliable gossip announce path.
7. **Drain-vs-promotion race.** The leader promotion loop must not resurrect a
   draining member when `RemoveNode` and `NodeLeft` visibility are reordered.
8. **Frozen-process class.** Real SIGKILL is necessary but not sufficient. A
   suspended former leader that resumes after timers expire is a distinct
   split-brain risk and belongs in the nightly tier.
9. **Harness determinism and feature hygiene.** A seed must replay to the same
   delivered/dropped trace, and release builds must mechanically prove that
   `test-support`/`fail` features did not leak through feature unification.
10. **Reuse shipped endurance samplers.** The `0.58` RSS/fd and bounded-growth
    samplers should be attached to W3 so correctness soaks also catch process
    growth and unexplained drive errors.

### Execution Scope / Corrections

- **Stage the release deliberately.** PR/MVP slice: `F1`, `F2`, `W1`, `W2`, `W5`, `W6`. Nightly/extended
  slice: `W3`, `W1b`, `W4`, frozen-process tests, and RSS/fd sampler reuse. This is one release plan,
  but implementation should not block the deterministic slice on nightly infrastructure.
- **Current code anchors.** `F1` currently lives at `crates/hydracache-cluster-raft/src/lib.rs:233`.
  `F2` currently lives at `crates/hydracache-server/src/grid_host.rs:1259-1267`. Re-grep before editing;
  exact file:line references are review aids, not authority.
- **Testkit decision.** Use a dev-only shared `hydracache-cluster-testkit` crate (`publish = false`,
  consumed only through `[dev-dependencies]`) for W1/W1b/W3 helper types, traces, replay manifests,
  and process harness utilities. Do not let each crate grow a private copy of the same harness
  vocabulary.
- **Testkit boundaries.** Keep W2 failpoints inside the raft crate behind `test-failpoints`; those
  injection points must live in production code paths to guard `drain_ready`/log-store boundaries.
  Keep server `test-support` as a minimal private seam only (`build_member_with_sink` or equivalent),
  not a home for harness types.
- **Dev-cycle fallback.** Preferred shape: `hydracache-cluster-testkit` owns raft/chitchat filters and
  `DaemonCluster`. If `testkit -> hydracache-server` plus server tests `-> testkit` causes Cargo,
  rust-analyzer, or CI friction, move only `DaemonCluster` into `crates/hydracache-server/tests/support/`
  and keep `hydracache-cluster-testkit` server-free.
- **W4 scope guard.** Ship a thin membership-history adapter/checker first: epoch monotonicity,
  member-set equality per epoch, and same-term leader uniqueness. Reuse `LinearizabilityChecker` only
  if the adapter stays small; otherwise record it as stretch work rather than expanding `0.62`.
- **F1 extra gate.** Because `pre_vote = true` is a production consensus behavior change, it needs both
  deterministic W1 coverage and a W3/nightly mixed restart/topology soak before release.

Redis RESP and Hazelcast client compatibility stay out of `0.62`. They are
future edge facades that depend on the cluster-correctness evidence this release
is meant to produce. The only compatibility lesson to pull forward now is W6's
byte-level golden-vector discipline.

## Non-Goals

- **No new consensus, no product behavior change.** W1-W6 are tests + harnesses; F1/F2 are minimal,
  each independently revertible and separately gated.
- **Not a Jepsen/Antithesis dependency.** W4 reuses the shipped `0.44` checker; we do not vendor an
  external model checker (the DST simulator is our TigerBeetle-class engine already).
- **No `fail` failpoints in production builds.** W2 injection points compile only under a
  `test-failpoints` feature (TiKV keeps them behind `failpoints`); release binaries never carry them.
- **Not a chaos-mesh/kind expansion.** That is `0.61` W3. This release is process- and message-level
  determinism, complementary to the kind soak.
- **No Redis or Hazelcast edge facade in `0.62`.** The RESP and Hazelcast-compatible protocol surfaces
  remain future migration accelerators. `0.62` only builds the correctness gates they will rely on.
- **The real-process and randomized-topology tiers stay nightly.** They are timing-sensitive; the PR
  gate keeps only deterministic filter/failpoint/property tests.

## Dependency Graph

```
F1 pre_vote ─┐
F2 wire-id ──┤ (small fixes, land early; W1 tests exercise both)
             ▼
W1 message-filter harness ─► W1-tests (partition/stale/reorder/dup/prevote) ─┐
W2 failpoints (torn ConfState / hard-state) ─────────────────────────────────┤
W3 real-process harness ─► W3-tests (SIGKILL/restart/double-vote) + soak ─────┼─► W7 docs + gates + backlog #3 closure
W4 membership linearizability over recorded history ─────────────────────────┤
W5 property/fuzz (id-map, wire decode) ──────────────────────────────────────┤
W6 golden wire/durable vectors ──────────────────────────────────────────────┘
```

## F1. Enable raft `pre_vote` (small fix, exposed by W1)

**Why.** Without pre-vote, a node isolated by a partition keeps incrementing its term while
campaigning; when the partition heals it injects a higher term that forces the healthy leader to
step down and re-elect — a needless availability blip (the "term explosion" TiKV's `test_prevote.rs`
guards). raft-rs implements pre-vote; we only need to switch it on.

**Change.** `crates/hydracache-cluster-raft/src/lib.rs:233-243` — add `pre_vote: true` to the
`Config` built by `raft_config`. Verify the existing `networked_raft.rs` runtime tests still elect
(pre-vote adds a round trip; the deterministic harness must still converge). This is R-4-safe: pre-vote
is a wire-behavior change **within** one raft group upgraded together, but a mixed pre-vote/no-pre-vote
cluster during a rolling upgrade must be checked — **document in `docs/COMPAT.md`** that pre-vote is
enabled at `0.62` and note the one-release mixed window (raft-rs pre-vote interoperates with non-pre-vote
peers by falling back, but assert it in a W1 test: `mixed_prevote_cluster_still_elects`).

**Tests.** `prevote_isolated_node_rejoin_does_not_depose_leader` (W1 filter harness: isolate a
follower, let it campaign, heal, assert the original leader's term is unchanged and it stays leader —
**falsifiable**: without F1 the leader's term jumps). Also run a W3/nightly mixed restart/topology soak
with `pre_vote = true`, because this is a production consensus behavior change. Run:
`cargo test -p hydracache-cluster-raft --locked`.

## F2. Fix `raft_wire_node_id` mapping (small fix, exposed by W5)

**Why.** `raft_wire_node_id` (grid_host.rs:1263-1267) and `raft_node_id` (:1259-1261) can disagree for a
node whose `ClusterNodeId` string parses as a `u64`. Inbound raft `step` then attributes the message
to the wrong sender id, corrupting raft's per-peer progress tracking. The only reason it has not bitten
is that identities are currently `member-<addr>` strings that never parse as bare integers — but
`node-identity.json` and the `0.61` configurable `node_id` make integer-like ids reachable.

**Change.** Make one mapping the single source of truth: inbound handler should map the wire `from`
field through **the same `raft_node_id(&ClusterNodeId)`** used everywhere else, not a parse-first
shortcut. Delete `raft_wire_node_id`; where the handler has only the wire `from` string, resolve it
through the routing table (`SharedRaftPeers` reverse lookup) or hash consistently. Fail loud if a wire
`from` cannot be resolved to a known peer (do not silently hash an unknown string into a fabricated id).

**Tests.** W5's property test `wire_id_mapping_is_consistent_across_sink_and_handler`. Run:
`cargo test -p hydracache-server --locked grid_host`.

## W1. Deterministic message-filter transport harness

**Goal.** A test-only `RaftMessageSink` decorator that can **drop, delay, duplicate, reorder, and
mutate** raft messages per (from, to, message-type), giving deterministic adversarial scheduling —
the single most valuable missing harness. This is what unlocks partition/stale-peer/prevote/reorder
tests without a real network.

**Design — copy the TiKV `Filter` trait shape (transport_simulate.rs:37-45).** TiKV:
```rust
pub trait Filter: Send + Sync {
    fn before(&self, msgs: &mut Vec<RaftMessage>) -> Result<()>; // mutate/drop the batch
    fn after(&self, res: Result<()>) -> Result<()> { res }
}
```
HydraCache analogue (new, in a dev-only shared `hydracache-cluster-testkit` crate so W1, W1b, W3,
replay manifests, and daemon tests share one vocabulary):
```rust
pub trait RaftMessageFilter: Send + Sync {
    /// Decide the fate of one outbound message; return the (possibly empty,
    /// possibly duplicated/reordered) set actually delivered.
    fn filter(&self, msg: &RaftWireMessage) -> FilterVerdict; // Pass | Drop | Delay(n) | Duplicate
}

pub struct FilteredRaftMessageSink {
    inner: Arc<dyn RaftMessageSink>,           // wraps the real or in-test bus sink
    filters: Arc<RwLock<Vec<Box<dyn RaftMessageFilter>>>>,
    delayed: Arc<Mutex<Vec<(u64 /*deliver_at_tick*/, RaftWireMessage)>>>,
    dropped: Arc<Mutex<Vec<RaftWireMessage>>>, // reserve_dropped analogue for assertions
}
```
The **vocabulary** copies raft-rs `harness/network.rs`: `cut(a, b)` (drop both directions),
`isolate(id)` (cut a node from all others), `recover()` (clear all filters). The **builder** copies
TiKV `RegionPacketFilter`: `.direction(Send|Recv|Both)`, `.msg_type(MsgRequestVote)`, `.allow(n)`
(let n through then drop), `.when(Arc<AtomicBool>)` (conditional), `.reserve_dropped(buf)` (capture for
assertions). Decode the raft message type from the payload once
(`RaftWireMessage::decode()?.get_msg_type()`) so filters can target `MsgRequestVote`/`MsgAppend`/etc.
Use `BTreeMap`/ordered vectors for filter maps and trace emission even though raft-rs uses `HashMap`;
the HydraCache harness must prefer deterministic iteration over literal copying.

**Where it plugs in.** Two levels, both already have the seam:
1. **Runtime level (PR-tier, deterministic):** extend `NetworkedRuntimeCluster` in
   `crates/hydracache-cluster-raft/tests/networked_raft.rs` so its in-test message bus routes through
   `FilteredRaftMessageSink`. No wall-clock; "delay" is measured in the harness's manual `tick` count
   (mirrors raft-rs `Network` which is purely logical — "no actual network calls are made",
   network.rs:41).
2. **Daemon level (network-gated):** the server's `networked_member_stack` builds the sink at
   grid_host.rs:153-163; a test-support constructor injects a `FilteredRaftMessageSink` wrapping the
   real `HttpRaftMessageSink`, exercising the actual HTTP path under partition.

**Steps.**
1. Add the `RaftMessageFilter` trait + `FilteredRaftMessageSink` + `cut/isolate/recover` + the
   builder, with `reserve_dropped` capture, in `hydracache-cluster-testkit`.
2. Extend `NetworkedRuntimeCluster` to route through it; add a `deliver_delayed(up_to_tick)` pump.
3. Add a server-side test-support injection point (a `#[cfg(feature="test-support")]`
   `build_member_with_sink` seam or equivalent) so daemon tests can wrap the real sink. This feature
   exposes only the private constructor seam; all filter types stay in `hydracache-cluster-testkit`.
4. Emit a deterministic trace of delivered/dropped/delayed messages and add a meta-test that runs the
   same seeded filter scenario twice and compares traces byte-for-byte.

**Tests (PR-tier deterministic unless noted).** New file
`crates/hydracache-cluster-raft/tests/raft_message_filter.rs`:
- `asymmetric_partition_leader_keeps_leadership_when_only_one_direction_drops` — cut A→B but not
  B→A; assert no spurious re-election (the classic asymmetric bug; TiKV covers this class in
  `test_transfer_leader.rs`). **Falsifiable**: a naive impl re-elects.
- `minority_partition_cannot_commit_but_majority_can` — isolate one of three; majority commits a
  `MemberUpsert`, isolated node does not advance `applied_index`; heal → it catches up.
- `duplicate_confchange_delivery_is_idempotent` — `.duplicate()` an `AddNode` append; assert
  `voter_ids()` contains the node exactly once, no panic (guards `0.60` conf apply).
- `reordered_appends_do_not_corrupt_committed_prefix` — reorder two appends; consensus-prefix
  invariant holds (reuse `hydracache-sim` `InvariantChecker` consensus-prefix check).
- `stale_leader_not_reported_during_partition` (daemon-level, network-gated) — partition the leader
  into the minority; assert `/cluster/overview` on the majority side elects a **new** leader and
  never shows the partitioned one as live (the missing `0.57` falsifiable test).
- `retired_peer_messages_are_rejected_after_drain_epoch_advances` - drain/remove a member, reserve
  an old `MsgAppend`/`MsgRequestVote` from that peer, deliver it after the membership epoch advances,
  and assert the old peer cannot reappear in `voter_ids()` or depose the leader. This is the explicit
  stale-peer/tombstone test missing from the first draft.
- `leader_promotion_does_not_resurrect_draining_member` - delay the `NodeLeft` append, allow the
  leader's promotion cycle to tick after `RemoveNode`, and assert `sync_raft_voters` does not propose
  `AddNode` for the draining member.
- `message_filter_replays_identically_for_same_seed` - run one filter scenario twice with the same
  seed and assert the delivered/dropped trace is identical; this guards the harness itself.
- `inbound_snapshot_message_is_applied_or_rejected_loud` - feed a synthetic raft `MsgSnapshot` through
  `step` and prove the state machine either installs it safely or rejects it loudly; no silent
  corruption on the cold snapshot path.
- `prevote_isolated_node_rejoin_does_not_depose_leader` (F1) and
  `mixed_prevote_cluster_still_elects` (F1 compat).
- Run: `cargo test -p hydracache-cluster-raft --locked`,
  `cargo test -p hydracache-server --locked grid_host` (+ network-gated tier).

**Risk & rollback.** Purely additive test infrastructure. The one design risk is determinism of
"delay/reorder" — keep it tick-counted, never wall-clock (R-5), exactly as raft-rs `Network` does.

## W1b. Deterministic gossip fault transport harness

**Goal.** Add the same deterministic fault vocabulary to the chitchat discovery plane. Raft filters
prove authoritative membership behavior under bad raft transport; they do not prove the join/drain
path when gossip announcements, liveness transitions, or graceful-leave markers are lost or delayed.

**Design.** Add a test-only `FilteredChitchatTransport` wrapping `chitchat::transport::Transport`.
Reuse chitchat's in-memory `ChannelTransport` first, because `ChitchatDiscovery::spawn_with_transport`
already exists for tests. The filter vocabulary mirrors W1 where possible:

- `cut(a, b)`, `isolate(id)`, `recover()`;
- drop/delay by source, target, key prefix, or message class where the chitchat API exposes it;
- scripted liveness flaps (`Live -> Suspect -> Dead -> Live`) without wall-clock sleeps;
- ordered trace emission for replay, again using ordered maps/vectors rather than `HashMap` iteration.

**Steps.**
1. Add `FilteredChitchatTransport` in `hydracache-cluster-testkit`.
2. Wire it into `chitchat_admission_bridge`-style tests through `spawn_with_transport`.
3. Record gossip delivered/dropped/liveness-flap traces alongside raft traces in replay manifests.
4. Cross-link this work to backlog #8 (gossip reset/tombstone semantics) instead of pretending W1 alone
   closes the discovery plane.

**Tests.** New/extended chitchat tests:

- `gossip_flap_does_not_flap_quorum` - liveness flaps must not make raft voter quorum bounce if the
  authoritative raft view remains healthy.
- `lost_leave_marker_behavior_is_named` - drop the graceful-leave marker and assert the documented
  behavior: either leader timeout removal is explicit, or the gap is recorded as a known follow-up.
- `stale_generation_candidate_resurrection_is_rejected_under_flap` - after tombstone/reset pressure,
  a stale generation candidate must not be admitted as a fresh member.

**Risk & rollback.** Additive test infrastructure. Keep it PR-tier only if it uses `ChannelTransport`
and logical time; UDP/process versions stay network-gated.

## W2. Failpoints at crash/persist boundaries

**Goal.** Prove the durable-write ordering is crash-safe by injecting a fault **between** persist steps
— the windows `0.60`/`0.61` opened (`save_conf_state`, `save_hard_state`) and never tested.

**Design — copy TiKV `fail` usage (`tests/failpoints/cases/test_conf_change.rs:106/168`).** Add the
`fail` crate as a **dev-dependency + optional `test-failpoints` feature** on `hydracache-cluster-raft`.
Insert named points in `RaftRuntimeState::drain_ready` and the log-store writers
(`crates/hydracache-cluster-raft/src/log_store.rs` `save_hard_state`, `save_conf_state`, `append`):
```rust
fail::fail_point!("raft_after_save_snapshot_before_entries", |_| Err(injected_crash()));
fail::fail_point!("raft_before_save_conf_state", |_| Err(injected_crash()));
fail::fail_point!("raft_after_save_hard_state_before_send", |_| Err(injected_crash()));
fail::fail_point!("raft_after_install_snapshot_before_apply", |_| Err(injected_crash()));
fail::fail_point!("sled_append_disk_full", |_| Err(disk_full()));
fail::fail_point!("canary_raft_skip_save_conf_state", |_| Ok(()));
fail::fail_point!("canary_raft_disable_prevote", |_| Ok(()));
fail::fail_point!("canary_raft_disable_confchange_dedup", |_| Ok(()));
```
Points are **inert unless the feature is on** — release builds carry nothing (verify with a
mechanical W7 feature-leak gate). TiKV configures them as `fail::cfg(name, "return")`/`"panic"`/`"pause"`
per test. The `canary_*` points are not crash failpoints; they deliberately reintroduce known broken
behavior so the corresponding harness test can prove it turns red.

**Steps.**
1. Add `fail` dev-dep + `test-failpoints` feature; gate every `fail_point!` behind it.
2. Place points: after snapshot save before entry persistence, before/after `save_conf_state`, after
   `save_hard_state` before outbound send, after snapshot install before apply, and on `append`
   (disk-full). Names namespaced `raft_*`.
3. Tests use `fail::cfg` to arm, drive a scenario, disarm, then reopen the durable log and assert
   integrity.
4. Add a falsifiability canary mapping for each major guarantee: `canary_raft_skip_save_conf_state`
   for W2, `canary_raft_disable_prevote` for F1, and `canary_raft_disable_confchange_dedup` for W1.
   Each canary run must fail before the production test counts toward R-7.

**Tests.** New `crates/hydracache-cluster-raft/tests/failpoints_crash_safety.rs`
(`#[cfg(feature="test-failpoints")]`, run serially — `fail` is process-global, use
`fail::FailScenario` per TiKV):
- `crash_between_confchange_commit_and_save_conf_state_recovers_consistent_voters` — arm
  `raft_before_save_conf_state` to error after the entry commits; drop + reopen the durable runtime;
  assert the recovered `voter_ids()` equals the committed conf (either fully applied or safely replayed
  — never a torn half-state). **The single most valuable failpoint test**: it guards the newest `0.60`
  durable write.
- `crash_after_hard_state_before_send_does_not_lose_committed_entry` — the entry is durable even though
  the outbound message never left; a re-driven runtime re-sends.
- `crash_after_snapshot_persist_before_apply_replays_or_installs_once` - persist a snapshot, inject a
  crash before apply/advance, reopen, and assert the snapshot is either replayed once or cleanly
  ignored in favor of a newer log prefix; never double-apply, never lose committed membership.
- `disk_full_on_append_fails_loud_not_silent` — `append` disk-full surfaces a loud error, no partial
  commit (R-3).
- `falsifiability_canaries_turn_their_guard_tests_red` - arm each `canary_*` point once and assert the
  matching guard test fails; this is the mechanical version of the Final Decision's R-7 rule.
- Run (gated): `cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1`.

**Risk & rollback.** Feature-gated; PR gate runs it as a **separate** invocation (single-threaded).
Revert removes the feature; production untouched.

## W3. Real-process multi-daemon harness

**Goal.** Replace the in-process "kill" with **actual OS process death**, so restart/rejoin, persisted
vote/term honesty, and socket-level failure are real. Then a seeded randomized-topology soak on top.

**Design — copy qdrant `consensus_tests` + curvine `MiniCluster`.** qdrant
(`tests/consensus_tests/utils.py`) spawns real binaries and holds `PeerProcess(proc: Popen)` with
`kill()` = `proc.kill(); proc.wait()`; curvine (`curvine-tests/src/testing.rs`) does the Rust-native
equivalent via `MiniCluster::with_num(&conf, master_num, worker_num)` with per-node temp dirs and a
`start_cluster()` that cleans meta/journal dirs first. **Choose Rust-native** (curvine shape) to stay
in-workspace and reuse `ServerConfig`: a `DaemonCluster` helper in `hydracache-cluster-testkit` by
default, with fallback to `crates/hydracache-server/tests/support/daemon_cluster.rs` if the server
dev-dependency cycle causes tooling friction. It:
- builds N `ServerConfig`s over loopback (reuse `reserve_loopback_addrs`, tests/grid_host.rs), each
  with its own temp `storage_dir`;
- spawns each as a **child process** running the real `hydracache-server` binary
  (`std::process::Command`, `env!("CARGO_BIN_EXE_hydracache-server")`), not an in-process runtime;
- exposes `kill(idx)` (SIGKILL via `Child::kill`), `restart(idx)` (re-spawn same `storage_dir`),
  `overview(idx)` (HTTP GET `/cluster/overview` on that daemon's admin port), and `Drop` that reaps all
  children (qdrant reaps in a fixture teardown).

**Steps.**
1. Add the `DaemonCluster` support module (child-process spawn, admin-port polling with a bounded
   deadline like qdrant's `WAIT_TIME_SEC`, temp-dir lifecycle, guaranteed child reaping on drop,
   process-group cleanup, and captured stdout/stderr per node).
2. Port the existing `multi_node_*` E2E to also run under `DaemonCluster` (real processes) as a
   network-gated sibling.
3. Add the randomized-topology soak driver (seeded `SimRng` from `hydracache-sim` for the operation
   sequence; each step: join/drain/kill/restart; after each, poll every live daemon's overview and
   assert invariants).
4. Persist a replay manifest for every failed soak: seed, operation list, node ids, ports, storage
   dirs, overview history, and child logs. This is the TigerBeetle-style "make the failure replayable"
   rule applied to real processes.
5. Add `suspend(idx)` / `resume(idx)` for Linux nightly (`SIGSTOP`/`SIGCONT`) and skip-gracefully on
   Windows unless a Job Objects equivalent is implemented.
6. Reuse the `0.58` real-process RSS/fd sampler and `BoundedGrowthChecker` in the daemon soak. Bound
   `GridDriveDiagnostics.drive_errors` to known, explained failures after the run.
7. Document the Windows/Linux matrix explicitly: `Child::kill` is `TerminateProcess` on Windows,
   process-group cleanup differs from Job Objects, and sled lock behavior after hard termination must
   be covered or the nightly tier is declared Linux-CI-only.

**Tests (network-gated, nightly).** New `crates/hydracache-server/tests/daemon_process_cluster.rs`:
- `sigkill_leader_reelects_and_restarted_node_rejoins_same_storage` — real SIGKILL of the leader;
  survivors elect; restart the killed process on its `storage_dir`; it rejoins as a **returning
  member** (not a new voter). **Falsifiable**: an in-process drop can't distinguish this.
- `restarted_node_does_not_double_vote_in_same_term` — kill a follower mid-election, restart; assert it
  never grants two votes in one term (persisted `HardState.vote`/`term` honesty — the classic raft
  durability bug; only a real crash+restart exercises the durable read path).
- `randomized_topology_soak_preserves_invariants` — seeded N-step join/drain/kill/restart over 4-5 real
  daemons; after each step assert: one leader (or a bounded election window), voter set matches live
  membership, no lost committed `MemberUpsert`, epoch monotonic per observer. Logs its seed (R-5),
  replayable. This is the membership-plane analogue of the `0.58` VOPR.
- `drained_node_restart_does_not_silently_resurrect_voter` - drain a node, kill it, restart from the
  same `storage_dir`, and assert it follows the `0.61` drain contract instead of rejoining as an old
  voter without a fresh join path.
- `suspended_leader_resumes_as_follower_without_split_brain` - Linux-only nightly: suspend the leader,
  allow a new leader to be elected, resume the old process, and assert it steps down rather than
  reporting a concurrent leader view.
- `daemon_process_soak_bounds_rss_fds_and_drive_errors` - attach the `0.58` sampler during the
  randomized soak and assert process growth plateaus and drive errors are explained.
- Run (gated): the network command in GATES.md, extended with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1`.

**Risk & rollback.** Child-process orphans are the real hazard — the `Drop` reaper + a process-group
kill (qdrant tracks a global `processes` list and reaps in teardown) must be robust on Windows and
Linux. Keep strictly nightly; never in the PR gate.

## W4. Membership-history linearizability check

**Goal.** Reuse the shipped `0.44` checker to validate a **recorded daemon history**, not only the
simulated one — a lightweight Jepsen-style consistency check on the real membership plane.

**Design.** During a W3 randomized soak (or a W1 daemon-level filter run), record a history of
`(observer_node, wall_or_logical_ts, operation, observed_epoch, observed_member_set)` from each
daemon's `/cluster/overview`. Feed it to `hydracache-sim`'s `LinearizabilityChecker`
(`crates/hydracache-sim/src/linearizability.rs`) / `InvariantChecker`
(`src/invariants.rs`) — the same checkers the DST gate uses (per `demo/README.md`, "the actual
invariant checker"). Assert: per-observer epoch monotonicity, no committed-`MemberUpsert` lost across a
leader change, no two distinct member-sets reported at the same epoch (split-membership), and no two
observable leaders with the same `status.term`.

Scope guard: implement this adapter-first. The first shipped artifact may be a small
`MembershipHistoryChecker` purpose-built for epoch/member-set/leader-term observations. Reuse the
generic `LinearizabilityChecker` only if the mapping remains thin and obvious; otherwise keep full
linearizability integration as stretch work.

**Steps.**
1. Add a `MembershipHistoryRecorder` in the server test-support module that snapshots overviews.
2. Add the adapter-first `MembershipHistoryChecker`; only then evaluate whether the generic
   `LinearizabilityChecker` accepts the same history without a large translation layer.
3. Include leader id + `status.term` in the recorded history so same-term split-leader observations are
   first-class failures.
4. Wire it as an assertion at the end of the W3 soak and a dedicated W1 partition-heal test.

**Tests.**
- `membership_history_is_epoch_monotone_under_partition_heal` (network-gated) — partition, mutate
  membership on the majority, heal, record all observers' histories, run the checker; **falsifiable**:
  an injected out-of-order epoch fails the check.
- `membership_history_rejects_two_leaders_in_same_term` - inject or replay a history containing two
  distinct leaders at the same raft term and assert the checker rejects it.
- Run (gated): with the W3 tier.

**Risk & rollback.** The checker is shipped and gate-proven; the only new code is the recorder +
adapter. Additive.

## W5. Property / fuzz on id-mapping and wire decode

**Goal.** Make F2 (and the whole id-mapping surface) deterministically loud, and prove the wire decoder
rejects malformed input without panicking.

**Design.** Use `proptest` (already a workspace dev-dep pattern in the DB crates) for:
- **id-mapping consistency:** ∀ `ClusterNodeId` strings (incl. integer-like `"42"`, unicode, `|`/`:`),
  the id used by the outbound sink == the id used by the inbound handler == `raft_node_id(&id)`
  (this is exactly F2's guard).
- **wire decode robustness:** ∀ arbitrary byte payloads, `RaftWireMessage::decode()` and
  `ClusterOpaqueMessage::decode_payload()` (transport-axum) either return a value or a loud `Err` —
  never panic, never `unwrap` (R-3). Feed truncated, oversized, and random protobuf.

**Steps.**
1. Add `proptest` dev-dep to `hydracache-cluster-raft` and `hydracache-server` where missing.
2. `wire_id_mapping_is_consistent_across_sink_and_handler` (drives F2).
3. `raft_wire_message_decode_never_panics` + `cluster_opaque_message_decode_rejects_malformed_loud`.

**Tests.** New `crates/hydracache-cluster-raft/tests/wire_properties.rs`,
`crates/hydracache-server/tests/id_mapping_properties.rs`. Run:
`cargo test -p hydracache-cluster-raft --locked`, `cargo test -p hydracache-server --locked`.

**Risk & rollback.** Property tests are deterministic given a seed; `proptest` records failing seeds in
`proptest-regressions/` (commit them, per the tool's convention). Additive.

## W6. Golden wire/durable format vectors (rolling-upgrade compat)

**Goal.** Guard the `0.56` mixed-version rolling-upgrade claim with **byte-level** golden vectors, not
only a `RAFT_LOG_FORMAT_VERSION` integer check.

**Design — copy the in-repo golden pattern** (`crates/hydracache-sim/tests/snapshot_schema.rs`) and
arroyo's versioned binary protocol discipline. Commit a corpus of encoded bytes for each wire/durable
artifact: `RaftMetadataCommandEnvelope` (each `RaftMetadataCommand` variant: `MemberUpsert`,
`ClientUpsert`, `NodeLeft`, `CommitTopology`), `RaftWireMessage`, and a minimal durable raft-log
segment, plus one durable snapshot/`ConfState` vector. A test decodes each committed vector and
asserts the materialized value equals the expected struct — so a future encoding change that breaks
old-reader compatibility fails loudly and forces a `RAFT_LOG_FORMAT_VERSION` bump + `docs/COMPAT.md`
entry (R-4).

**Steps.**
1. Add `crates/hydracache-cluster-raft/tests/vectors/` with committed `.bin` files (generate once from
   the current encoders, review the bytes into git).
2. `golden_command_envelopes_decode_to_expected`, `golden_wire_messages_decode_to_expected`, and
   `golden_snapshot_conf_state_decodes_to_expected`.
3. A regenerate helper behind `--ignored` (like snapshot updates) so intentional format changes are a
   deliberate, reviewed commit.

**Tests.** New `crates/hydracache-cluster-raft/tests/golden_vectors.rs`. Run:
`cargo test -p hydracache-cluster-raft --locked`.

**Risk & rollback.** Additive; the only discipline is that changing the vectors is a reviewed act.

## W7. Docs, gates, and backlog closure

**Goal.** Wire the new tiers into GATES.md, document the harnesses so they become the standing home for
cluster correctness tests (backlog #3), and reconcile the ledger.

**Files.** `docs/GATES.md` (failpoint job, real-process nightly tier, property/golden in the fast
gate, release-feature-leak check), `docs/TESTING.md` (a "cluster correctness harnesses" section
pointing at each harness and its reference blueprint, plus flake policy), `.github/workflows/ci.yml`
(named steps for the failpoint and daemon-process tiers), `docs/plans/CROSS_PROJECT_IDEA_BACKLOG.md`
(mark #3 "Cluster Load Test Suite As A First-Class Gate" delivered, with pointers), `docs/COMPAT.md`
(F1 pre-vote note + W6 vector policy), `releases.toml` + `INDEX.md` + this header.

**Steps.**
1. GATES.md rows: `test-failpoints` gate (separate single-threaded invocation), `daemon_process_cluster`
   nightly tier, `wire_properties`/`golden_vectors` in the fast tier, and the required replay-artifact
   path for failed randomized soaks.
2. Add an xtask or script-backed release feature boundary check to `cargo xtask verify`: inspect
   `cargo tree -p hydracache-server --edges features` and fail if `fail`, `test-failpoints`, or the
   server's narrow `test-support` seam leaks into the default/release graph. `hydracache-cluster-testkit`
   itself must be present only as a `[dev-dependencies]` consumer, never in publishable/package graphs.
3. Add explicit `.github/workflows/ci.yml` steps for the failpoint gate and `daemon_process_cluster`
   nightly tier; do not rely on docs-only GATES rows.
4. TESTING.md: document how to add a new filter/failpoint/process test and which reference file each
   harness was modeled on (this plan's blueprint table).
5. TESTING.md: add the flake policy. A failed nightly creates an issue with seed + replay manifest,
   may be quarantined for at most one day, and must not be silently retried into green.
6. Document the W3 platform matrix: Linux is required for SIGSTOP/SIGCONT; Windows support must use Job
   Objects or the test is Linux-CI-only and skip-graceful elsewhere.
7. Add the falsifiability checklist: each new harness test maps to a `canary_*` or fixture-level
   broken behavior and is proven red once before counting toward R-7.
8. Backlog #3 and the "sandbox as regression lab" conclusion: mark delivered.
9. Flip the manifest triple green only when every gate passes.

## Test coverage matrix (every new artifact has a named test)

| New code / harness | Source | Covering test(s) | Tier |
| --- | --- | --- | --- |
| `pre_vote` (F1) | raft lib.rs:233 | `prevote_isolated_node_rejoin_does_not_depose_leader`, `mixed_prevote_cluster_still_elects`, W3/nightly mixed restart/topology soak with `pre_vote = true` | PR + nightly |
| wire-id fix (F2) | grid_host.rs:1259-1267 | `wire_id_mapping_is_consistent_across_sink_and_handler` | PR |
| `hydracache-cluster-testkit` crate | dev-only `publish = false` crate | package/publishability checks prove it is dev-only; no harness types exposed through production crates | PR |
| `RaftMessageFilter` + `FilteredRaftMessageSink` (W1) | `hydracache-cluster-testkit` | `asymmetric_partition_leader_keeps_leadership_when_only_one_direction_drops`, `minority_partition_cannot_commit_but_majority_can`, `duplicate_confchange_delivery_is_idempotent`, `reordered_appends_do_not_corrupt_committed_prefix`, `retired_peer_messages_are_rejected_after_drain_epoch_advances`, `leader_promotion_does_not_resurrect_draining_member`, `message_filter_replays_identically_for_same_seed`, `inbound_snapshot_message_is_applied_or_rejected_loud` | PR |
| `FilteredChitchatTransport` (W1b) | `hydracache-cluster-testkit` | `gossip_flap_does_not_flap_quorum`, `lost_leave_marker_behavior_is_named`, `stale_generation_candidate_resurrection_is_rejected_under_flap` | PR or network-gated depending on transport |
| daemon-level filter (W1) | grid_host.rs sink seam | `stale_leader_not_reported_during_partition` | network-gated |
| failpoints + falsifiability canaries (W2/W7) | raft log_store.rs / drain_ready / test-failpoints | `crash_between_confchange_commit_and_save_conf_state_recovers_consistent_voters`, `crash_after_hard_state_before_send_does_not_lose_committed_entry`, `crash_after_snapshot_persist_before_apply_replays_or_installs_once`, `disk_full_on_append_fails_loud_not_silent`, `falsifiability_canaries_turn_their_guard_tests_red` | PR (gated feature, serial) |
| `DaemonCluster` real-process harness (W3) | `hydracache-cluster-testkit` + server tests | `sigkill_leader_reelects_and_restarted_node_rejoins_same_storage`, `restarted_node_does_not_double_vote_in_same_term`, `drained_node_restart_does_not_silently_resurrect_voter`, `suspended_leader_resumes_as_follower_without_split_brain`, `daemon_process_soak_bounds_rss_fds_and_drive_errors` | network-gated / nightly |
| randomized topology soak (W3) | server test | `randomized_topology_soak_preserves_invariants` | nightly |
| membership linearizability (W4) | sim checker reuse | `membership_history_is_epoch_monotone_under_partition_heal`, `membership_history_rejects_two_leaders_in_same_term` | network-gated |
| id/wire property+fuzz (W5) | proptest | `raft_wire_message_decode_never_panics`, `cluster_opaque_message_decode_rejects_malformed_loud` | PR |
| golden vectors (W6) | committed corpus | `golden_command_envelopes_decode_to_expected`, `golden_wire_messages_decode_to_expected`, `golden_snapshot_conf_state_decodes_to_expected` | PR |
| docs/CI/release gates (W7) | GATES.md / TESTING.md / ci.yml / xtask | `cargo xtask verify-no-test-features`, named CI steps for failpoints + daemon process tier, documented flake quarantine policy | PR + CI/nightly |

**Coverage rule (DoD):** no new harness lands without a row here; PR-tier tests are deterministic and
inside `cargo xtask verify` (failpoints as a separate serial invocation); network/nightly rows are
env-gated and skip-graceful.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green; the failpoint suite runs as a separate serial gate; real-process and
  randomized-soak tiers are nightly and skip-graceful without their env flags.
- Falsifiability is mechanical: each major harness guarantee has a `canary_*` or fixture-level broken
  behavior, and the mapped test is proven red once before it counts toward R-7.
- Release feature graph proves `fail`/`test-failpoints`/`test-support` do not leak into default
  release builds; `hydracache-cluster-testkit` is dev-only and `publish = false`; CI has named steps
  for failpoints and daemon-process nightly; flake policy requires issue + seed + replay manifest, no
  silent retries.
- The message-filter harness exists and proves: asymmetric partition does not spuriously re-elect;
  minority cannot commit; duplicate/reordered raft messages are idempotent/safe; stale messages from a
  retired peer cannot resurrect membership; a partitioned leader is never reported live (the missing
  `0.57` falsifiable test) — all deterministic (R-5).
- W1 additionally proves drain-vs-promotion cannot resurrect a draining member, inbound `MsgSnapshot`
  applies or rejects loudly, and the same seed emits the same delivered/dropped trace.
- The chitchat/gossip plane has a deterministic fault harness and proves liveness flaps, lost
  graceful-leave markers, and stale-generation candidates cannot silently break admission/quorum
  semantics (W1b).
- A crash injected **between** `ConfChange` commit and `save_conf_state` recovers a consistent voter
  set; a crash after `save_hard_state` loses no committed entry; a crash around snapshot persist/apply
  recovers without double-apply or lost committed membership; disk-full on append fails loud (W2).
- A **real SIGKILL** of the leader re-elects; the restarted process rejoins on its `storage_dir` as a
  returning member and never double-votes in a term; a drained node restart cannot silently resurrect
  an old voter; failed randomized soaks write replay manifests and child logs (W3).
- W3 additionally covers Linux suspended-leader resume, records the platform matrix, and reuses the
  `0.58` RSS/fd bounded-growth sampler with explained `GridDriveDiagnostics.drive_errors`.
- Membership history under partition-heal passes the shipped `0.44` linearizability/invariant checker
  (W4); id-mapping and wire-decode property tests pass, `raft_wire_node_id` bug fixed (F2/W5); golden
  vectors decode (W6).
- W4 additionally rejects two observable leaders in the same `status.term`.
- `pre_vote` enabled with a documented one-release mixed window (F1, COMPAT); no production behavior
  change beyond F1/F2; embedded fast path unchanged (R-10); no new consensus/consistency level (R-1).
- Backlog #3 marked delivered, backlog #8 cross-linked for gossip reset semantics;
  `releases.toml` + `INDEX.md` + header flipped; `cargo xtask doc-check`
  green.

```powershell
# fast (PR) tier
cargo xtask verify
cargo test -p hydracache-cluster-raft --locked
cargo test -p hydracache-cluster-chitchat --locked
cargo test -p hydracache-server --locked grid_host
cargo xtask verify-no-test-features

# failpoint gate (separate, serial)
cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1

# network-gated + nightly tiers
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test daemon_process_cluster --locked -- --nocapture
cargo test -p hydracache-server --test grid_host --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E,Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

## Final Release Decision

`0.62.0` ships **only** if every gate above is green. Because this release is test infrastructure, the
bar is inverted from a feature release: a harness that exists but proves nothing (a filter test that
would pass even with the bug present) is worse than no test. Every W1/W2/W3 test names its
**falsifiable** failure mode; a test that cannot fail on its canary/seeded-broken behavior does not
count toward the gate (R-7). F1/F2 are the only production changes and each reverts independently.
