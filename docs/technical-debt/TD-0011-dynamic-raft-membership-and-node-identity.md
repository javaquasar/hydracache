# TD-0011: Static raft voter set and address-derived node identity

## Status

Open pending the live operator kind scale proof. The `0.60.0` identity,
ConfChange, quorum, and graceful-drain sub-items were resolved on 2026-07-06.
The `0.61.0` daemon late-start join sub-item was resolved on 2026-07-07.

Owner: cluster raft runtime / server grid host.

Partial target: `0.60.0` Networked Grid Hardening (W3/W4), plus `0.61.0`
Cluster Elasticity Completion for the daemon join path. The remaining evidence
gap is the live operator kind proof that `spec.replicas` changes move deployed
daemon voters end-to-end.

## 0.60 Resolution Scope

`0.60.0` resolves the unsafe identity and drain/quorum parts:

- member identity is persisted as `node-identity.json` in `storage_dir`, with
  fail-loud future-format, configured-id mismatch, and raft-id collision paths;
- `RaftMetadataRuntime` exposes raft-rs `ConfChange` voter add/remove and
  persists `ConfState` through the raft log store;
- follower metadata proposals report `Forwarded` and wait for real apply
  instead of claiming `Committed` early;
- graceful daemon drain commits metadata leave before requesting voter removal,
  and the E2E covers both follower drain and leader drain/re-election;
- `/cluster/overview` quorum is computed from reachable raft voters rather than
  metadata members.

Verification:

```powershell
cargo test -p hydracache-cluster-raft --test networked_raft --locked
cargo test -p hydracache-server --lib --locked grid_host::tests
cargo test -p hydracache-server --test grid_host --locked
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
cargo test -p hydracache-server --test grid_host multi_node --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue
```

## 0.61 Daemon Join Resolution Scope

`0.61.0` resolves the daemon late-start join bootstrap:

- member startup has an explicit `cluster_start = "bootstrap" | "join"` mode;
- a joiner starts from the existing voter seeds and does not seed itself into a
  divergent local `ConfState`;
- the joiner announces its routable cluster endpoint before waiting for join
  completion, so the leader can admit and promote it;
- peers fold gossip candidate endpoints into raft routing hints, so followers
  can route to the new daemon;
- `wait_for_join_complete` succeeds only after the joiner sees a leader and its
  own raft id in the voter set; an unreachable cluster fails loud before any
  self-bootstrap;
- graceful drain of a joined daemon removes it from the voter set.

Verification:

```powershell
cargo test -p hydracache-server --test server_lifecycle --locked
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
cargo test -p hydracache-server --test grid_host multi_node --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue
```

## Remaining Gap

The daemon late-start join path is covered by the loopback networked E2E. The
remaining TD-0011 gap is the live operator scale claim: with a real kind cluster
and the current `hydracache-server` image, changing
`HydraCacheCluster.spec.replicas` must move deployed daemon voters end-to-end
(`3 -> 4 -> 3`), and a pod crash must not silently shrink voters.

Verification still required before resolving the whole debt:

```powershell
$env:HYDRACACHE_OPERATOR_KIND='1'
$env:HYDRACACHE_OPERATOR_IMAGE='<current hydracache-server image>'
cargo test -p hydracache-operator --test e2e kind_ --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_OPERATOR_KIND,Env:\HYDRACACHE_OPERATOR_IMAGE -ErrorAction SilentlyContinue
```

## Context

`0.59` shipped multi-voter raft with a voter set computed **once, at startup,
from the local `seeds` list** (`raft_topology`,
`crates/hydracache-server/src/grid_host.rs:375-418`). There is no raft
`ConfChange` anywhere in `hydracache-cluster-raft` (the `0.59` plan's W1b step 3
named a voter-change path and the test
`conf_change_adds_and_removes_raft_voter_loudly`; neither was implemented —
the release shipped with the static-bootstrap subset).

Node identity is derived from the listen address: `member_node_id_for_addr`
(grid_host.rs:537-550) turns `cluster_addr` into the `ClusterNodeId`, and
`raft_node_id` is an FNV-1a hash of that string (grid_host.rs:552-572) with no
collision handling.

## Why It Is A Debt

- **Live operator scale proof is still pending.** The daemon join path works in
  the loopback E2E, but the operator replica-to-voter claim needs the kind live
  tier with the current server image before this TD can be marked resolved.

## Risk While Open

- Operator-driven scale-up of member pods is not yet an end-to-end release claim
  until the live kind gate above is run with the current server image.
- Rolling infrastructure that changes pod IPs is covered only when the same
  `storage_dir` and persisted `node-identity.json` are retained.

## Revisit Triggers

- the operator live kind gate is available with the current server image;
- any soak/E2E needs to add or remove a daemon at runtime.

## Future Definition Of Done

- The live kind gate proves `spec.replicas` changes move raft voters through
  deployed daemons.
- The live kind gate proves both directions: graceful drain shrinks the quorum
  denominator, while a crash does not silently remove a voter.

## Related

- `docs/plans/V0_60_NETWORKED_GRID_HARDENING_PLAN.md` (W3/W4)
- `docs/plans/V0_59_NETWORKED_DAEMON_GRID_HOSTING_PLAN.md` (W1b scoping)
- `crates/hydracache-cluster-raft/src/lib.rs`
- `crates/hydracache-cluster-raft/src/log_store.rs`
- `crates/hydracache-server/src/grid_host.rs`
- `crates/hydracache-server/src/bootstrap.rs`
