# TD-0008: Networked daemon grid hosting

## Status

Resolved in `0.59.0`.

Owner: server / cluster runtime integration.

## Resolution

`hydracache-server` member mode now hosts the networked grid stack:

- durable `hydracache-cluster-raft::RaftMetadataRuntime`
- `hydracache-cluster-chitchat::ChitchatDiscovery`
- `hydracache-cluster-transport-axum` raft message routes
- one shared raft-backed membership/status authority for the cache and
  `/cluster/overview`

The old in-process member grid remains available only as the explicit
`HYDRACACHE_GRID_INPROC=1` test/development fallback. `local` and `client`
roles continue to report `source:"modeled"`.

## Verification

Fast gate:

```powershell
cargo test -p hydracache-server --test grid_host --locked
```

Network-gated loopback daemon E2E:

```powershell
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
cargo test -p hydracache-server --test grid_host multi_node_members_form_a_cluster_and_elect_one_leader --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue
```

The E2E starts three real `ServerRuntime` member daemons on loopback, waits for
one elected raft leader and three committed members, drops the leader, and then
asserts that the remaining daemons re-elect a new leader without reporting the
stale one.

## Follow-Ups

`0.59.0` closes the daemon hosting debt. Production soak mileage remains a
separate `0.60`/`1.0` evidence track, and the kind chaos soak still documents
external injectors for network-partition and slow-disk faults.

## Related Plans

- `docs/plans/V0_57_MANAGEMENT_CENTER_AND_OBSERVABILITY_PLAN.md` (W6b)
- `docs/plans/V0_58_ENDURANCE_SOAK_AND_OVERLOAD_HARDENING_PLAN.md`
- `docs/plans/V0_59_NETWORKED_DAEMON_GRID_HOSTING_PLAN.md`
- `crates/hydracache-server/tests/grid_host.rs`
- `crates/hydracache-cluster-raft/tests/networked_raft.rs`
