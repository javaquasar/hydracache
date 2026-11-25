# Client and Member Cluster

**API:** [`hydracache-cluster`](https://docs.rs/hydracache-cluster/latest/hydracache_cluster/)

`HydraCache::client()` and `HydraCache::member()` are the embedded cluster shape.

A client is an application-side near-cache. A member is a cluster participant. Both can join an `InMemoryCluster`, share invalidation, and expose role, node id, generation, bootstrap, lifecycle, and participant diagnostics.

The cluster vocabulary is the foundation the rest of the cluster surface builds on:

- roles distinguish application clients from member nodes;
- generations protect against stale processes reusing node ids;
- local-first invalidation keeps value ownership explicit;
- diagnostics make runtime state visible to health and actuator surfaces.

Embedded client and member caches do not replicate values by default: each process keeps its own entries, and invalidation intent is what travels. Value replication is an explicit opt-in of the grid layer (see below).

## Support Boundary

Current support includes:

- local, client, and member cache roles;
- generation-safe admission, leave, and invalidation publishing;
- in-memory cluster control plane for tests, demos, and local embedding;
- chitchat-backed discovery;
- raft-rs metadata control plane with a durable (sled-backed) log, restart recovery, and authenticated HTTP raft append, vote, and snapshot routes;
- deterministic rendezvous ownership over admitted members;
- HTTP peer fetch and owner load over encoded cache bytes, with token authentication and wire-version checks;
- external invalidation transports over Redis and NATS;
- grid features: replicated values with backups, per-operation consistency levels, online resharding, cluster checkpoints, and multi-region links;
- a standalone server (`hydracache-server`) with TLS and mTLS listeners;
- read-only diagnostics and observability surfaces.

It intentionally does not include:

- distributed transactions (a permanent non-goal);
- transparent remote closures or arbitrary executable code;
- certificate issuance or rotation for the embedded HTTP transport (bring your own TLS termination, or use the server's TLS listeners);
- write-enabled admin APIs on the embedded actuator.

> [!WARNING]
> The cluster surface is exercised by a deterministic fault simulator, networked integration tests, and soak runs, but it has limited production operating history. Treat it as staging-ready and plan topology, storage, authentication, supervision, and runbooks before a production rollout.

The repository guide `docs/PRODUCTION_CLUSTER_READINESS.md` has the staging and pilot checklists.

## Optional Adapters

The cluster APIs are split into optional crates so applications can adopt only the pieces they need:

- use `hydracache-cluster-chitchat` for chitchat-backed candidate discovery;
- use `hydracache-cluster-raft` for the raft-rs metadata control-plane runtime;
- use `hydracache-cluster` for the standard chitchat plus raft composition;
- use `hydracache-cluster-transport-axum` when members expose HTTP peer fetch or raft routes over encoded cache bytes;
- use `hydracache-transport-redis` or `hydracache-transport-nats` to carry invalidation across processes.

Start with `InMemoryCluster` for tests and demos. Add real adapters only when the deployment has a real discovery and metadata story.

## Next Steps

- [Cluster Topology](cluster-topology.md) — where this shape fits between embedded and a server grid
- [Consistency Model](../concepts/consistency-model.md) — consistency levels and limits
- [How We Test Correctness](../concepts/how-we-test-correctness.md) — how the cluster is verified
