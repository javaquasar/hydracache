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

In `0.60.0`, `tls.enabled = true` means the cluster listener terminates rustls
with the configured cert/key, outbound raft messages use `https://` with the
configured CA, and `[cluster_auth]` credentials are required on the raft route.
TLS without `[cluster_auth]` fails loud at startup. Plaintext member transport is
only for loopback/dev or an explicitly acknowledged insecure staging boundary.

The first startup writes `node-identity.json` in `storage_dir`. Keep that file
with the raft log when a member moves addresses; a configured `node_id` must
match the persisted identity on later starts.

## Status

`GET /cluster/overview` on the admin listener reports:

- `source: "live"` for member mode
- `leader` as the elected daemon member id, or `null` during election
- committed member count and quorum status from the same raft-backed authority
  used by the cache
- `quorum_ok` from reachable raft voters, not merely metadata members

Graceful shutdown/drain commits the member leave before requesting raft voter
removal, so follower drain and leader drain shrink the committed member set and
voter denominator. A full fourth-daemon late join after an already-formed
cluster remains tracked in
[`TD-0011`](technical-debt/TD-0011-dynamic-raft-membership-and-node-identity.md)
and should not be claimed as an operator scale-up guarantee yet.

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
