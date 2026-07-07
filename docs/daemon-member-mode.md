# HydraCache Daemon Member Mode

`hydracache-server` with `role = "member"` hosts the networked daemon grid. The
member stack is durable raft metadata, chitchat discovery, and the cluster raft
transport on `cluster_addr`. `local` and `client` roles remain modeled and do not
join the raft-backed member grid.

## Minimal Three-Member Shape

Each member needs:

- a unique `cluster_addr`
- the same concrete `seeds` list
- a persistent `storage_dir`
- TLS enabled for any non-loopback listener, or an explicit insecure
  acknowledgement for loopback/dev only
- `[cluster_auth]` when TLS is enabled

Example TOML shape:

```toml
role = "member"
listen_addr = "127.0.0.1:18080"
cluster_addr = "127.0.0.1:17000"
seeds = [
  "127.0.0.1:17000",
  "127.0.0.1:17001",
  "127.0.0.1:17002",
]
storage_dir = "data/hydracache-0"
drain_timeout_ms = 1000
join_timeout_ms = 15000

[admin_api]
enabled = true
listen_addr = "127.0.0.1:19091"

# Required when [tls].enabled = true.
[cluster_auth]
key_id = "cluster-key-1"
token_file = "secrets/cluster-token"
```

Use the same concrete `seeds` set on the initial members. Concrete socket seeds
are required so the daemon can derive the initial raft voter set and route
outbound raft messages to peers.

`cluster_start` defaults to `bootstrap`. Use that default for the initial voter
cohort. `join_timeout_ms` bounds an explicit late join; a joiner that cannot
reach or be admitted by an existing cluster fails loud instead of
self-bootstrapping.

In `0.60.0`, `tls.enabled = true` means the cluster listener terminates rustls
with the configured cert/key, outbound raft messages use `https://` with the
configured CA, and `[cluster_auth]` credentials are required on the raft route.
TLS without `[cluster_auth]` fails loud at startup. Plaintext member transport is
only for loopback/dev or an explicitly acknowledged insecure staging boundary.

The first startup writes `node-identity.json` in `storage_dir`. Keep that file
with the raft log when a member moves addresses; a configured `node_id` must
match the persisted identity on later starts.

## Growing The Cluster

To add a member after the initial cluster has formed:

1. Start the existing voter cohort with `cluster_start = "bootstrap"` (or omit
   it) and a shared concrete `seeds` list.
2. Give the new member a unique `cluster_addr`, persistent `storage_dir`, and
   `cluster_start = "join"`.
3. Point the joiner's `seeds` at the existing voter cohort, not at itself.
4. Set `cluster_advertise_addr` when `cluster_addr` is bound to a non-routable
   address such as `0.0.0.0`.
5. Wait for `/cluster/overview` to show the new member and voter count.

Example late-join TOML fragment:

```toml
role = "member"
listen_addr = "127.0.0.1:18083"
cluster_addr = "0.0.0.0:17003"
cluster_advertise_addr = "127.0.0.1:17003"
cluster_start = "join"
seeds = [
  "127.0.0.1:17000",
  "127.0.0.1:17001",
  "127.0.0.1:17002",
]
storage_dir = "data/hydracache-3"
join_timeout_ms = 15000
```

The operator renders the same idea through stable pod identity: the original
bootstrap cohort keeps bootstrap mode, later ordinals use join mode and
advertise their headless-Service DNS endpoint. A scale-up pod may crash-loop
until the existing cluster is reachable and can admit it; that is honest
backpressure. It must not self-bootstrap into a second raft cluster.

## Shrinking The Cluster

Use graceful drain for planned shrink:

1. Identify the target member in `/cluster/overview`.
2. If the target is leader, wait for or trigger re-election before deleting it.
3. Call the admin drain endpoint on the target member.
4. Wait for `/cluster/overview` on survivors to show both the member count and
   voter count reduced.
5. Remove the pod/process only after drain completes.

In Kubernetes, the operator scale-down planner sends the admin drain action
before removing the highest ordinal. A crash is different from a drain: a
crashed pod does not silently shrink the voter set; it is expected to return
with the same PVC and `node-identity.json`.

## Status

`GET /cluster/overview` on the admin listener reports:

- `source: "live"` for member mode
- `leader` as the elected daemon member id, or `null` during election
- committed member count and quorum status from the same raft-backed authority
  used by the cache
- `quorum_ok` from reachable raft voters, not merely metadata members

Graceful shutdown/drain commits the member leave before requesting raft voter
removal, so follower drain and leader drain shrink the committed member set and
voter denominator. A fourth daemon can now join an already-formed loopback
cluster as a counted voter. The operator replica-to-voter claim is still gated
by the live kind tier described in
[`TD-0011`](technical-debt/TD-0011-dynamic-raft-membership-and-node-identity.md).

The old in-process member host is only the explicit development fallback:

```powershell
$env:HYDRACACHE_GRID_INPROC='1'
```

## Verification

Fast member-hosting tests:

```powershell
cargo test -p hydracache-server --test grid_host --locked
```

Networked loopback E2E:

```powershell
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
cargo test -p hydracache-server --test grid_host multi_node --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue
```

Operator live kind scale proof:

```powershell
$env:HYDRACACHE_OPERATOR_KIND='1'
$env:HYDRACACHE_OPERATOR_IMAGE='<current hydracache-server image>'
cargo test -p hydracache-operator --test e2e kind_ --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_OPERATOR_KIND,Env:\HYDRACACHE_OPERATOR_IMAGE -ErrorAction SilentlyContinue
```
