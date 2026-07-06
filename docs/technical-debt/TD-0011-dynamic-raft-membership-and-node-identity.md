# TD-0011: Static raft voter set and address-derived node identity

## Status

Open.

Owner: cluster raft runtime / server grid host.

Candidate target: `0.60.0` Networked Grid Hardening (W3/W4).

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

- **The cluster cannot be resized at runtime.** A fourth daemon started later
  computes its own topology, but the running members' raft `ConfState` is fixed
  — the new node never becomes a voter. This breaks the `0.56` operator's
  scale-up story for member pods (the StatefulSet grows, the raft quorum does
  not).
- **Graceful drain leaks voters.** Shutdown removes the member from *metadata*
  (`NodeLeft` via `leave_cluster_for_shutdown`,
  `crates/hydracache-server/src/bootstrap.rs:373-377`) but never from the raft
  voter set. After two of three members drain away, the survivor can never win
  an election (raft still requires 2/3), while `has_quorum()`
  (grid_host.rs:720-730) counts *metadata members* — the two quorum planes
  disagree, which is exactly the split the `0.59` plan was corrected to avoid.
- **Identity is coupled to the address.** Restarting a member on a different
  port/IP with the same `storage_dir` produces a new node id and raft id over a
  durable raft log whose recorded voter set names the old id — the node comes
  back outside its own cluster. An FNV-64 collision between two seed-derived
  ids would silently merge two identities.

## Risk While Open

- Operator-driven scale-up/scale-down of member pods silently fails to change
  the raft quorum.
- Rolling infrastructure that changes pod IPs (without stable DNS names in
  `seeds`) orphans durable raft logs.
- Quorum reporting on `/cluster/overview` can read `true` while raft can no
  longer commit (or elect), and vice versa.

## Revisit Triggers

- `0.60.0` starts implementation;
- the operator asserts member-count changes through the deployed daemon;
- any soak/E2E needs to add or remove a daemon at runtime.

## Future Definition Of Done

- `RaftMetadataRuntime` exposes a fail-loud voter-change path (raft
  `ConfChange` AddNode/RemoveNode) with the `ConfState` persisted by the log
  stores.
- The daemon promotes an admitted member to voter (leader-side) and removes a
  draining member from the voter set before process exit; a drained node's
  departure shrinks the quorum denominator.
- Node identity is persisted in `storage_dir` (or configured explicitly),
  survives address changes, and identity/raft-id mismatches or hash collisions
  fail loud at startup.
- `has_quorum()` counts reachable raft **voters** against the raft `ConfState`
  majority, not metadata members.

## Related

- `docs/plans/V0_60_NETWORKED_GRID_HARDENING_PLAN.md` (W3/W4)
- `docs/plans/V0_59_NETWORKED_DAEMON_GRID_HOSTING_PLAN.md` (W1b scoping)
- `crates/hydracache-cluster-raft/src/lib.rs`
- `crates/hydracache-cluster-raft/src/log_store.rs`
- `crates/hydracache-server/src/grid_host.rs`
- `crates/hydracache-server/src/bootstrap.rs`
