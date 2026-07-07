# TD-0011: Static raft voter set and address-derived node identity

## Status

Resolved on 2026-07-07. The `0.60.0` identity, ConfChange, quorum, and
graceful-drain sub-items were resolved on 2026-07-06. The `0.61.0` daemon
late-start join and live operator kind scale proof sub-items were resolved on
2026-07-07.

Owner: cluster raft runtime / server grid host.

Partial target: `0.60.0` Networked Grid Hardening (W3/W4), plus `0.61.0`
Cluster Elasticity Completion for the daemon join path and live operator scale
proof.

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

## 0.61 Live Operator Scale Proof

`0.61.0` also resolves the live operator scale claim with a real kind cluster:
changing `HydraCacheCluster.spec.replicas` moves deployed daemon voters
end-to-end (`3 -> 4 -> 3`), and a pod crash does not silently shrink voters.

Implementation notes:

- the operator runs namespace-scoped watches when configured with
  `HYDRACACHE_OPERATOR_NAMESPACE`, matching its namespace-scoped RBAC;
- StatefulSet pods derive node identity, advertise address, bootstrap/join mode,
  and seeds from Kubernetes env without a shell wrapper;
- `/admin/drain` is available as POST and Kubernetes preStop-compatible GET;
- admin drain marks the daemon draining, leaves metadata membership, requests
  raft voter removal, and does not stop the process from inside the admin Tokio
  runtime;
- scale-down status is two-phase: `DrainRequested` is sticky until a survivor
  admin status reports committed member/voter removal, then `DrainComplete`
  allows the StatefulSet replica count to shrink.

Verification performed on 2026-07-07:

```powershell
$env:PATH="$env:USERPROFILE\go\bin;$env:PATH"
$env:CARGO_TARGET_DIR='target\td0011-cargo-target'
$env:HYDRACACHE_OPERATOR_KIND='1'
$env:HYDRACACHE_OPERATOR_IMAGE='hydracache-server:td0011-0.61'
$env:HYDRACACHE_OPERATOR_NAMESPACE='default'
$env:HYDRACACHE_OPERATOR_CLUSTER='hydracache-td0011'
cargo test -p hydracache-operator --test e2e kind_ --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_OPERATOR_KIND,Env:\HYDRACACHE_OPERATOR_IMAGE,Env:\HYDRACACHE_OPERATOR_NAMESPACE,Env:\HYDRACACHE_OPERATOR_CLUSTER,Env:\CARGO_TARGET_DIR -ErrorAction SilentlyContinue
```

Result: `3 passed; 0 failed; finished in 41.08s`.

Images used in the proof:

- `hydracache-server:td0011-0.61`
  (`sha256:3498bc05d7aaf7de2d3c632be8ad1aab421ace501d522c6bc3a52fd1494ce7e4`)
- `hydracache-operator:td0011-0.61`
  (`sha256:b28938af69480e4290ff7462d05d22bdf76a60e912fd442921e22523898dc3d0`)

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

Resolved. The daemon join path is covered by the loopback networked E2E, and the
operator replica-to-voter claim is covered by the kind live tier with the current
server image.

## Risk While Open

None for this debt item after the 2026-07-07 live proof. Rolling infrastructure
that changes pod IPs still depends on retaining the same `storage_dir` and
persisted `node-identity.json`, which is the supported identity model.

## Revisit Triggers

- any soak/E2E needs to add or remove a daemon at runtime.

## Definition Of Done

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
