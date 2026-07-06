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

[admin_api]
enabled = true
listen_addr = "127.0.0.1:19091"
```

Use the same `seeds` set on every member. For the 0.59 loopback gate, concrete
socket seeds are required so the daemon can derive the raft voter set and route
outbound raft messages to peers.

## Status

`GET /cluster/overview` on the admin listener reports:

- `source: "live"` for member mode
- `leader` as the elected daemon member id, or `null` during election
- committed member count and quorum status from the same raft-backed authority
  used by the cache

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
cargo test -p hydracache-server --test grid_host multi_node_members_form_a_cluster_and_elect_one_leader --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue
```
