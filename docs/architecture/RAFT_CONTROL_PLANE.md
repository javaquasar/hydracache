# Raft Control Plane and the Write Path

This is the **single source of truth** for what Raft does in HydraCache, what it does
not do, and how the deterministic simulator (`hydracache-sim`) and the browser demo
(`demo/`) relate to it.

- ADRs record *why* a decision was made (ADR-0001, ADR-0002, ADR-0016). This file
  records *what the code does now*, mapped to the symbols that do it.
- When this file and the code disagree, the code is right and this file is a bug.
  Fix it in the same change that touched the code.
- Every simulator or demo behaviour that differs from the product is listed in
  [Simulator and demo fidelity](#simulator-and-demo-fidelity), with a gap id and the
  technical-debt record that owns it. A new difference must not ship without a row.

Paths are relative to the repository root. Symbols are cited by name rather than by
line number, because line numbers drift.

## In One Paragraph

Raft (`raft-rs`, wrapped by `hydracache-cluster-raft`) is the **metadata control
plane only**. It commits who is in the cluster (members and clients, with their
process generations), the topology epoch, and the voter set. It never carries cache
values, invalidations, or reads. A cache write is handled by the member that
receives it, without contacting the Raft leader. When there is no leader, admission,
leave, topology and voter changes fail loud and are retried. Cache reads, cache
writes, and invalidation publishing keep working on the metadata already applied
locally.

## Code Map

| Concern | Symbol | File |
| --- | --- | --- |
| Control-plane contract (transport-neutral) | `ClusterControlPlane` | `crates/hydracache/src/cluster/control_plane.rs` |
| Metadata commands (state-machine input) | `RaftMetadataCommand` | `crates/hydracache/src/cluster/control_plane.rs` |
| Materialized membership (the applied state) | `InMemoryCluster` | `crates/hydracache/src/cluster/control_plane.rs` |
| Raft runtime behind the contract | `RaftMetadataRuntime`, `RaftMetadataRuntimeConfig` | `crates/hydracache-cluster-raft/src/lib.rs` |
| Command envelope and idempotency key | `RaftMetadataCommandEnvelope`, `command_id_for` | `crates/hydracache-cluster-raft/src/lib.rs` |
| Proposal result | `RaftCommandResult`, `RaftCommandStatus` (`Committed` / `Forwarded` / `Duplicate`) | `crates/hydracache-cluster-raft/src/lib.rs` |
| Raft wire frame | `RaftWireMessage` | `crates/hydracache-cluster-raft/src/lib.rs` |
| Log stores | `InMemoryRaftLogStore`, `DurableRaftLogStore`, `SledRaftLogStore` (`sled-log-store`) | `crates/hydracache-cluster-raft/src/log_store.rs` |
| Discovery to admission | `ClusterAdmissionBridge` | `crates/hydracache/src/cluster/admission.rs` |
| Discovery (liveness only) | `ChitchatDiscovery` | `crates/hydracache-cluster-chitchat` |
| Topology epoch fence | `TopologyFence` | `crates/hydracache/src/cluster/topology.rs` |
| Key ownership | `RendezvousClusterOwnership` (`ClusterOwnershipResolver`) | `crates/hydracache/src/cluster/ownership.rs` |
| Replica placement types (built for every target, incl. wasm32) | `Replicas`, `EffectiveReplicationMap`, `ReplicationConfig`, `RegionId` | `crates/hydracache/src/replication_model/mod.rs` |
| Replica ranking (native only) | `ClusterReplicationStrategy` for `RendezvousClusterOwnership` | `crates/hydracache/src/grid/mod.rs` |
| Consistency levels (built for every target) | `ConsistencyLevel`, `AckRequirement`, `ConsistencyUnsatisfiable` | `crates/hydracache/src/replication_model/consistency_level.rs` |
| Golden key placement (cross-version contract) | `rendezvous_placement_matches_recorded_owners` | `crates/hydracache/tests/placement_golden.rs` |
| Hinted handoff (opt-in) | `Hint`, `HintStore`, `replay_hints` | `crates/hydracache/src/grid/hinted_handoff.rs` |
| Invalidation publish | `HydraCache::publish_invalidation` | `crates/hydracache/src/cache.rs` |
| Daemon hosting of the stack | `networked_member_stack`, `drive_grid_once`, `metadata_authority_matches` | `crates/hydracache-server/src/grid_host.rs` |
| Sans-IO data-plane node (simulator and tests only) | `ClusterNode`, `ClusterNodeMessage` | `crates/hydracache/src/cluster/node.rs` |
| Simulator election (model) | `ElectionDriver`, `step_with_partitions` | `crates/hydracache-sim/src/election.rs` |
| Simulator election (real raft-rs, native only) | `SimRaft` | `crates/hydracache-sim/src/sim_raft.rs` |
| Simulator world | `SimWorld` | `crates/hydracache-sim/src/world.rs` |

## Metadata State Machine

### Commands

`RaftMetadataCommand` has exactly four variants:

| Command | Proposed by | Effect when applied |
| --- | --- | --- |
| `MemberUpsert { node_id, generation, epoch }` | `RaftMetadataRuntime::join_member` | admits or updates a member and advances the epoch for new membership |
| `ClientUpsert { node_id, generation, epoch }` | `RaftMetadataRuntime::join_client` | admits or updates a client; the epoch is the current one (clients do not move it) |
| `NodeLeft { node_id, role, epoch }` | `RaftMetadataRuntime::leave` | removes the node when its generation still matches |
| `CommitTopology { epoch, members }` | only `RaftStyleMetadataControlPlane::commit_topology` (the in-process model); `RaftMetadataRuntime` decodes and applies it but never proposes it | makes an owner table authoritative at `epoch` |

Voter changes are not metadata commands. They are raft-rs `ConfChange` / `ConfChangeV2`
entries (`propose_add_voter`, `propose_remove_voter`, `request_remove_voter`).

### Idempotency

Each command is wrapped in a `RaftMetadataCommandEnvelope` whose `command_id` comes
from `command_id_for`, for example `client-upsert:<node>:<generation>` or
`node-left:<node>:<epoch>`. When a command id has already been applied, the proposal
returns `RaftCommandStatus::Duplicate` without proposing again. This is why retrying
an admission after a timeout or a failover is safe.

### Apply

`apply_committed_entries` decodes every committed normal entry. It inserts the
`command_id` into `applied_command_ids` and calls `materialize_committed_command`
against the inner `InMemoryCluster`. Conf-change entries are applied to the
`RawNode`, then `save_conf_state` persists them. Membership is materialized **only
after commit**. A failed proposal never mutates materialized state
(`failed_proposal_does_not_mutate_materialized_metadata`).

### Durability

ADR-0002 is the durability contract. The `Ready` loop persists the snapshot first,
then entries, then hard state, before outbound messages are sent. Compaction never
moves past applied progress. The daemon uses `SledRaftLogStore`. The embedded default
can run in memory.

## Proposal Path: Leader, Follower, No Leader

`RaftRuntimeState::commit_command`:

1. A command id that was already applied returns `Duplicate`.
2. If this node is not the leader **and** knows no leader (`known_leader_id` is
   `None`), it returns
   `CacheError::Backend("no raft leader; retry metadata proposal after election")`.
   This is a fail-loud refusal, not a silent fallback.
3. Otherwise it calls `raw_node.propose`.
   - On the leader, the entry is appended and replicated. With a single voter it is
     committed and applied inside the same `drain_ready`, so the result is
     `Committed`.
   - On a follower, raft-rs forwards the proposal to the leader as `MsgPropose`. The
     result is `Forwarded`, and `wait_for_forwarded_apply` polls `command_applied`
     (500 attempts, 10 ms apart) until the leader's commit reaches this node. If the
     commit has not arrived by then, the call fails with "was forwarded but not
     applied locally before timeout".

The daemon (`networked_member_cache`) treats both error strings as retryable
(`is_retryable_member_admission_error`). It retries the idempotent admission within
the configured join timeout and fails loud after it.

## Admission

```text
process start
  -> discovery announces a ClusterCandidate (role, node_id, generation)
  -> ClusterAdmissionBridge::run_once polls candidates
  -> reject_stale_candidate     (an older generation is refused: StaleGeneration)
  -> join_member / join_client  (the command id makes retries idempotent)
  -> commit_command             (leader commits, or follower forwards)
  -> materialize on apply       (InMemoryCluster, membership event published)
  -> persist_metadata           (metadata store snapshot)
```

Discovery is a liveness input only (ADR-0001). A candidate is not a member until its
upsert is committed. A client's admission commits the same way as a member's; the
difference is that a client does not advance the epoch.

## Leave and Restarts

`leave` checks the generation first, then commits `NodeLeft`. A restarted process
gets a new generation. Its upsert supersedes the old one, and anything still stamped
with the old generation is refused:

- `validate_generation` before every invalidation publish;
- `TopologyFence::admit` for epoch-stamped decisions.

## Voters and Elections

- Constructors: `RaftMetadataRuntimeConfig::single_node`, `multi_voter` (bootstrapped
  voter set) and `try_joining`. A joining node becomes a voter only after the leader
  commits its `ConfChange`.
- Defaults: `election_tick = 10`, `heartbeat_tick = 3`. `pre_vote = true` has been the
  production default since 0.62.0.
- Draining a leader transfers leadership first (`transfer_leadership`, used in the
  drain path of `grid_host.rs`).
- Formal model: `docs/specs/RaftElection.tla` with the canary specification.

## Leader Loss: What Stops and What Keeps Working

| Operation | Needs a leader? | Behaviour while there is none |
| --- | --- | --- |
| Member or client admission | yes | fails with the no-leader error; the daemon retries within the join timeout |
| Leave, topology commit, voter change | yes | fails loud; voter change without a known leader is refused (`voter_change_without_a_known_leader_fails_loud`) |
| Cache `put` / `get` / `invalidate` on a member | no | served locally from applied metadata |
| Invalidation publish | no | `validate_generation` reads the local `InMemoryCluster` and does not propose |
| Advertising metadata as authoritative (management and overview) | fresh authority | `metadata_authority_matches` requires all three conditions below |

The three conditions for authoritative metadata are:

- leader commit traffic seen within `GRID_RAFT_AUTHORITY_FRESHNESS` (200 ms) on a
  follower, or a voter acknowledgement seen within that window on the leader
  (`RaftAuthorityObservation`);
- committed metadata fully applied (`applied_index == commit_index`);
- a matching metadata snapshot.

Single-voter clusters are exempt from the freshness condition.

A new leader is elected by raft-rs (pre-vote, check-quorum semantics). Clients do not
re-register when the leader changes: their `ClientUpsert` is already committed state.
A client re-registers only after a process restart, which gives it a new generation.

## The Data Plane: What Raft Does Not Do

### Default: local-first with invalidation

1. A client or member writes into its **local** cache.
2. If the write changes freshness, `publish_invalidation` stamps the source
   generation, validates it, and publishes a `CacheInvalidationMessage` on the
   `CacheInvalidationBus`.
3. Other participants drop their stale copy, and their next read refills it (read
   through, or a peer fetch from the rendezvous owner).

Two details follow from this composition:

- The bus implementation is chosen when the stack is composed: in-memory, framed, or
  the NATS and Redis transports.
- `RaftMetadataRuntime::invalidation_bus` returns the inner cluster's bus. Raft only
  hosts the reference; it never carries an invalidation.

The cluster surface intentionally does not replicate cached values by default
(`docs-site/src/guides/client-member-cluster.md`).

### Ownership

`RendezvousClusterOwnership` scores every admitted **member** (clients are excluded)
with FNV-1a over `key 0xff node_id` and picks the highest score. Ties go to the
greater node id. Every member computes the same owner from the same committed
membership. `RoutingMode::Direct` (smart client) contacts the owner. With
`SingleEndpoint`, owner traffic goes through a gateway.

### Opt-in value replication: primitives, not a wired write path

`ReplicationConfig { replicate_values, replication_factor, write_quorum, sync_backups,
async_backups, max_replicated_entry_bytes, .. }` is validated
(`ReplicationConfig::validate`), and these primitives exist and are tested:

- `replicas_for_key`: the primary plus `replication_factor - 1` backups, by the same
  rendezvous ranking;
- `EffectiveReplicationMap`: natural owners plus pending owners while a rebalance
  is moving data;
- `ConsistencyLevel::required_acks` / `validate`: `One`, `LocalQuorum`, `Quorum`,
  `EachQuorum`, `All`, with `ConsistencyUnsatisfiable` as the loud refusal;
- hinted handoff, the merkle repair and anti-entropy tasks, backup promotion, and the
  replicated value stores.

**As of this document, no runtime code path sends a replicated put to its owners and
collects acknowledgements.** Checked by searching for non-test callers of
`prepare_replicated_payload`, the replicated-send counters, and `replicas_for_key`
outside placement code. A diagram that shows "coordinator → primary → backup → acks"
describes the *target* of these primitives, not shipped behaviour. Label it that way.

## Simulator and Demo Fidelity

### What the simulator runs

| Layer | Simulator | Product equivalent | Same code? |
| --- | --- | --- | --- |
| Election (server and sandbox, native) | `SimRaft`: real raft-rs `RawNode` over `SimNetwork`, `election_source: "raft"` | `RaftMetadataRuntime`'s `RawNode` | same library, own harness |
| Election (browser, wasm) | `ElectionDriver` model, partition-aware, `election_source: "sim-model"` | none | model; ADR-0016 forbids claiming raft fidelity |
| Metadata commands | not simulated; `SimRaft` proposes only conf changes | `RaftMetadataCommand` | no |
| Data plane | `ClusterNode`: write locally, then broadcast `ReplicatePut` / `ReplicateInvalidate` / `ReplicateFlush` to **all** peers | default: local write plus invalidation bus; opt-in: owner placement (primitives) | `ClusterNode` is product code, but only the simulator and `tests/sans_io_seam.rs` use it |
| Consistency panel | `SimWorld::consistency_views` calls `ConsistencyLevel::{required_acks, validate}` with every node a replica, one region | `ConsistencyLevel` | yes, in both builds (TD-0014, resolved) |

### Gap register

Each row is a place where the simulator or the demo shows something the product does
not do. Close a gap by changing the engine, then the demo, then this table. A closed
gap moves to the history list below, with its commit.

| Id | Gap | Simulator / demo today | Product | Owner |
| --- | --- | --- | --- | --- |
| G1 | Write entry node | `SimWorld::push_event` writes on `preferred_write_node`: the leader, or **silently the first live node** when there is none. The smoke workload (`issue_smoke_workload`) writes on a random live node. | The member the client is connected to, with or without a leader. | TD-0016 |
| G2 | Leader on the data path | `demo/graph.js` `pushPlan` and `playDataFlowPulse` draw client → attached node → leader → replicas. | The leader is not involved in a cache write. | TD-0016 |
| G3 | Replica set | Every live node receives every write (`ClusterNode::broadcast`). | Default: nobody receives the value, others invalidate. Opt-in: the primary and `RF - 1` backups only. | TD-0016, TD-0014 |
| G4 | Acknowledgement and quorum | `handle_client` returns `Accepted` as soon as the node applies locally. Peer `Ack` messages are ignored. | Opt-in: the write is acknowledged per `ConsistencyLevel`, or refused with `ConsistencyUnsatisfiable`. | TD-0016 |
| G5 | Subscriber delivery | An event is delivered only once **every** live node holds the version (`deliver_subscriber_events`). | Events come from the cache event or invalidation stream of the node the subscriber is attached to. | TD-0016 |
| G6 | Client attachment | `connected_node_for` hashes the id over live nodes, so a client re-pins whenever the live set changes. | The client connects through bootstrap and discovery, and stays until its connection fails. | TD-0016 |
| G7 | Client admission | `add_client` / the first push insert the client directly. | `ClientUpsert` is committed through Raft and fails while there is no leader. | TD-0016 |

Closed gaps:

- TD-0015: per-node applied writes, drops at send, push delivery timing (snapshot
  schema 9).
- G8 / TD-0014: the simulator used a mirror of the consistency rules. The placement
  and consistency types now live in `replication_model` and build for wasm32; both
  simulator builds call the product rules, and the snapshot is unchanged.

### Target engine shape

This is the order for closing TD-0016 and TD-0014. Each step is a separate commit
with its own tests.

1. **Done: make the placement and consistency types available in wasm** (TD-0014,
   closes G8). The rendezvous *ranking* (`replicas_for_key`) still lives in `grid`,
   because it takes `ClusterMember`, which is native-only. Step 3 has to make it
   available to wasm32, and must keep `tests/placement_golden.rs` green while doing
   it.
2. **Write through the client's coordinator** (G1, G6, G7).
   - `push_event` enters on the client's attached node. Attachment is sticky and
     stored in the world, not re-hashed.
   - Admission becomes a simulated `ClientUpsert` that fails loud without a leader.
   - The smoke workload uses the same rule.
   - The silent first-live-node fallback is removed.
3. **Owner placement and acknowledgements** (G3, G4).
   - A `SimConfig` replication mode chooses one of two models:
     - `local_first`: the coordinator writes locally and broadcasts invalidations;
     - `replicated { rf, level }`: the coordinator forwards to the primary, which
       replicates to its backups. Acknowledgements are counted with
       `ConsistencyLevel::validate`, an unreachable replica gets a hint, and the
       write is refused when the level cannot be met.
   - The snapshot gains the owners of each key and an acknowledgement record per
     write. This bumps the snapshot schema and needs a `docs/COMPAT.md` entry.
4. **Subscriber events from the attached node** (G5). An event is delivered when the
   subscriber's node applies the write or receives the invalidation.
5. **Demo** (G2). The demo draws:
   - client → coordinator → primary → backups, then acknowledgements back;
   - invalidations to the holders of the key;
   - the Raft leader on a separate metadata lane, shown only for admission, leave
     and topology.

Rules that apply to every step: `docs/RULES.md` (fail loud, deterministic fault
model, boolean gates, tests for every new path). Native and wasm snapshots must stay
byte-identical (`same_seed_native_and_wasm_match`).

## Keeping This File True

Update this file in the same change when you:

- add, remove or change a `RaftMetadataCommand` variant, a `command_id_for` format, or
  `RaftCommandStatus`;
- change when a proposal fails, forwards or retries (`commit_command`,
  `wait_for_forwarded_apply`, `is_retryable_member_admission_error`);
- change the authority fence (`metadata_authority_matches`,
  `GRID_RAFT_AUTHORITY_FRESHNESS`);
- wire a runtime replicated-write path (rewrite "Opt-in value replication" and close
  the rows it affects);
- change how the simulator picks an entry node, replicates, acknowledges, attaches
  clients or delivers subscriber events (update the gap register).
