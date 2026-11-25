# Server and Deployment

**API:** [`hydracache-server`](https://docs.rs/hydracache-server/latest/hydracache_server/)

Most services never need a HydraCache server: the cache lives inside your process. Run `hydracache-server` when you want a **shared cache tier** — remote clients in Rust, Python, or Java, a Redis-compatible endpoint, or a raft-backed member grid that outlives individual application processes.

| You want | Run |
| --- | --- |
| A cache inside one Rust service | nothing — embed `hydracache` |
| Several Rust instances sharing invalidation | nothing — embed `hydracache` + a Redis or NATS transport |
| A shared cache for several services or languages | `hydracache-server` members + client SDKs |
| Existing Redis clients against HydraCache | `hydracache-server` with the RESP listener |

## Get the Binary

- **Release download.** Each release publishes a signed Linux x86-64 binary on [GitHub Releases](https://github.com/javaquasar/hydracache/releases) (requires glibc 2.38 or newer). Verify the signature and SHA-256 before use.
- **Container.** The repository `Dockerfile` builds a distroless, non-root image that runs as a `member` by default.
- **From source.** `cargo build --release --locked -p hydracache-server`.

## Roles

| `HYDRACACHE_ROLE` | Meaning |
| --- | --- |
| `local` (default) | single process, no cluster |
| `member` | joins the networked grid: durable raft metadata, chitchat discovery, and the raft transport on `cluster_addr` |
| `client` | application-side role; modeled for diagnostics, does not join the raft voter set |

## Listeners

Every listener is opt-in and has its own address:

| Listener | Variables | Default | Purpose |
| --- | --- | --- | --- |
| Client API | `HYDRACACHE_LISTEN_ADDR`, `HYDRACACHE_CLIENT_API_ENABLED` | `127.0.0.1:8080` | HC/1 client protocol |
| HC/2 client plane | `HYDRACACHE_HC2_ENABLED`, `HYDRACACHE_HC2_ADDR`, `HYDRACACHE_HC2_CLUSTER_ID` | off | gRPC streaming plane used by the preview SDKs; **mTLS is mandatory** |
| Cluster | `HYDRACACHE_CLUSTER_ADDR`, `HYDRACACHE_SEEDS` | `127.0.0.1:7000` | member-to-member raft and discovery |
| Admin | `HYDRACACHE_ADMIN_API_ENABLED`, `HYDRACACHE_ADMIN_ADDR` | `127.0.0.1:9091` | health, readiness, status, drain |
| Redis RESP | `HYDRACACHE_REDIS_API_ENABLED`, `HYDRACACHE_REDIS_ADDR` | off | Redis-compatible facade, with optional AUTH and `rediss://` |

The server is configured through environment variables, which map one-to-one onto Kubernetes manifests.

## A Three-Member Cluster

Each member needs a unique cluster address, the same concrete seed list, and a persistent storage directory:

```powershell
$env:HYDRACACHE_ROLE = "member"
$env:HYDRACACHE_LISTEN_ADDR = "127.0.0.1:18080"
$env:HYDRACACHE_CLUSTER_ADDR = "127.0.0.1:17000"
$env:HYDRACACHE_SEEDS = "127.0.0.1:17000,127.0.0.1:17001,127.0.0.1:17002"
$env:HYDRACACHE_STORAGE_DIR = "data/hydracache-0"
$env:HYDRACACHE_ADMIN_API_ENABLED = "true"
$env:HYDRACACHE_ADMIN_ADDR = "127.0.0.1:19091"
$env:HYDRACACHE_TLS_ACK_INSECURE = "true"   # loopback/dev only
hydracache-server
```

Repeat with `17001`/`17002`, ports `18081`/`18082`, and separate storage directories. The initial members bootstrap together; later members set `HYDRACACHE_CLUSTER_START=join` and point their seeds at the existing cohort.

> [!WARNING]
> Keep `storage_dir` — including `node-identity.json` and the raft log — with the member across restarts and moves. A member that loses its identity is a new member, and a joiner that cannot be admitted fails loud instead of bootstrapping a second cluster.

## Security

- **TLS for anything non-loopback.** Set `HYDRACACHE_TLS_ENABLED=true` with `HYDRACACHE_TLS_CERT_PATH`, `HYDRACACHE_TLS_KEY_PATH`, and `HYDRACACHE_TLS_CA_PATH`. Plaintext requires an explicit `HYDRACACHE_TLS_ACK_INSECURE=true`.
- **Cluster authentication is required with TLS.** `HYDRACACHE_CLUSTER_AUTH_KEY_ID` and `HYDRACACHE_CLUSTER_AUTH_TOKEN_FILE` protect the raft routes; a previous key id and token file can be configured during rotation. TLS without cluster auth fails at startup.
- **HC/2 always uses mutual TLS.** Clients present a certificate; there is no plaintext mode outside explicit loopback tests.
- **Redis AUTH.** Enable `HYDRACACHE_REDIS_AUTH_REQUIRED` with a token file, and `HYDRACACHE_REDIS_REDISS_ENABLED` for TLS.

## Operating Members

- `GET /healthz` and `/readyz` on the admin listener for probes.
- `GET /cluster/overview` shows the leader, committed members, voter count, and whether the quorum is reachable.
- **Planned shrink:** call the admin drain endpoint on the target, wait until survivors show the reduced member and voter counts, then remove the process.
- **Crash ≠ shrink:** a crashed member does not silently leave the voter set; bring it back with the same storage.
- The [Management Center](management-center.md) gives a read-only operational view across members.

## Kubernetes

| Option | Use when |
| --- | --- |
| Static manifests (`deploy/k8s`: StatefulSet, Service, PodDisruptionBudget) | Kubernetes only needs to place and restart pods |
| Helm chart (`deploy/helm/hydracache`) | the same, with values-driven configuration |
| Operator (`deploy/operator`, `HydraCacheCluster` CRD) | lifecycle must respect cluster correctness: drain-before-remove, quorum-guarded scale, one-at-a-time upgrades, mTLS secret rotation, PVC retention |

Grafana dashboards and Prometheus alerts live in `deploy/dashboards`.

> [!NOTE]
> The operator is shipped as manifests and a container image, not as a crates.io library. Its live replica-to-voter scaling is still validated in a dedicated Kubernetes test tier.

## Next Steps

- [Client SDKs](client-sdks.md) — connect from Rust, Python, or Java
- [Redis API](redis-api.md) — the RESP facade and its supported subset
- [Cluster Topology](cluster-topology.md) — from embedded to multi-region, step by step
- [Production Checklist](../production-checklist.md)
