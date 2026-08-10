# Client and Member Cluster

`HydraCache::client()` and `HydraCache::member()` are the embedded cluster shape.

A client is an application-side near-cache. A member is a cluster participant. Both can join an `InMemoryCluster`, share invalidation, and expose role, node id, generation, bootstrap, lifecycle, and participant diagnostics.

The cluster vocabulary is useful today even before production remote-value distribution:

- roles distinguish application clients from member nodes;
- generations protect against stale processes reusing node ids;
- local-first invalidation keeps value ownership explicit;
- diagnostics make runtime state visible to health and actuator surfaces.

The current cluster surface intentionally does not replicate cached values. It gives applications a stable model for membership, invalidation, ownership, and future peer-fetch routing while keeping local cache semantics intact.

## Support Boundary

Current support includes:

- local, client, and member cache roles;
- generation-safe admission, leave, and invalidation publishing;
- in-memory cluster control plane for tests, demos, and local embedding;
- chitchat-backed discovery candidate adapter;
- raft-rs-backed metadata/control-plane adapter;
- deterministic ownership resolution over admitted members;
- transport-neutral peer-fetch over encoded cache bytes;
- read-only diagnostics and observability surfaces.

It intentionally does not yet include:

- production multi-node Raft networking or full durable Raft log storage;
- transparent remote closures or arbitrary executable code;
- value replication, backup ownership, or failover repair;
- external invalidation transports such as Redis, NATS, or Postgres LISTEN/NOTIFY;
- TLS/certificate management, external identity, or write-enabled admin APIs.

See the repository guide `docs/PRODUCTION_CLUSTER_READINESS.md` for the current staging checklist and non-goals.

## Optional Adapters

The cluster APIs are split into optional crates so applications can adopt only the pieces they need:

- use `hydracache-cluster-chitchat` for chitchat-backed candidate discovery;
- use `hydracache-cluster-raft` for the raft-rs metadata control-plane runtime;
- use `hydracache-cluster` for the standard chitchat plus raft composition;
- use `hydracache-cluster-transport-axum` when members expose HTTP peer-fetch over encoded cache bytes.

Start with `InMemoryCluster` for tests and demos. Add real adapters only when the deployment has a real discovery and metadata story.
