# TD-0011: Static raft voter set and address-derived node identity

## Status

Open, with the `0.60.0` identity, ConfChange, quorum, and graceful-drain
sub-items resolved on 2026-07-06.

Owner: cluster raft runtime / server grid host.

Partial target: `0.60.0` Networked Grid Hardening (W3/W4). The remaining
late-start daemon join bootstrap is deferred to a follow-up quality slice.

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

## Remaining Gap

A fourth daemon started after an already-formed 3-member cluster is still not a
release claim. The current startup path waits for a networked raft leader before
the member cache has completed admission, and a late-start node needs a
non-voter/join bootstrap path rather than seeding its local durable raft log as
if it were part of the original voter set. The leader-side drive loop can promote
an already admitted member to a voter, but the full new-daemon bootstrap needs a
separate falsifiable E2E before operator scale-up can be claimed end-to-end.

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

- **Late-start daemon join is still incomplete.** A fourth daemon started after
  the initial cluster needs a real join bootstrap path before it can become a
  voter end-to-end.

## Risk While Open

- Operator-driven scale-up of member pods is not yet an end-to-end release
  claim.
- Rolling infrastructure that changes pod IPs is covered only when the same
  `storage_dir` and persisted `node-identity.json` are retained.

## Revisit Triggers

- the operator asserts member-count changes through the deployed daemon;
- any soak/E2E needs to add or remove a daemon at runtime.

## Future Definition Of Done

- A fourth daemon can start after a 3-member cluster has already formed, join
  through the networked daemon path, and appear as an admitted raft voter on all
  survivors.
- The E2E proves both directions: graceful drain shrinks the quorum denominator,
  while a crash does not silently remove a voter.

## Related

- `docs/plans/V0_60_NETWORKED_GRID_HARDENING_PLAN.md` (W3/W4)
- `docs/plans/V0_59_NETWORKED_DAEMON_GRID_HOSTING_PLAN.md` (W1b scoping)
- `crates/hydracache-cluster-raft/src/lib.rs`
- `crates/hydracache-cluster-raft/src/log_store.rs`
- `crates/hydracache-server/src/grid_host.rs`
- `crates/hydracache-server/src/bootstrap.rs`
