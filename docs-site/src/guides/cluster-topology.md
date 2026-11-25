# Cluster Topology

HydraCache grows in steps. Each step is opt-in, adds crates rather than changing the ones you already use, and leaves the embedded cache's behavior unchanged. Pick the smallest shape that solves your problem.

| Shape | What is shared | Values cross the network? | Extra infrastructure |
| --- | --- | --- | --- |
| **1. Embedded** | nothing | no | none |
| **2. Embedded + invalidation transport** | invalidation intent | no | Redis or NATS (often already present) |
| **3. Client/member cluster** | membership, ownership, invalidation; peer fetch from owners | on peer fetch only | discovery + raft control plane inside your processes |
| **4. Server grid** | replicated values, consistency levels | yes | `hydracache-server` members |
| **5. Multi-region** | replicated values across regions | yes | members in each region |

## 1. Embedded

One process, one cache. Single-flight, tags, TTL, and post-commit invalidation all work here. Most services should start — and many can stay — at this step.

**Move on when** you run more than one instance and a write on one must invalidate the others.

## 2. Embedded + Invalidation Transport

Each instance still owns its cache and its loaders. Invalidations are published through Redis or NATS, so a write on any instance removes stale copies everywhere. Values never travel; every instance reloads from the database on its next read. See [Distributed Invalidation](distributed-invalidation.md) and the [tutorial](../tutorial.md).

**Move on when** many instances miss on the same hot keys and the database load from N independent reloads matters, or you need a single owner per key.

## 3. Client/Member Cluster (Embedded)

Processes join a cluster as **members** (participants that can own keys) or **clients** (application-side near-caches). The cluster adds:

- generation-safe membership with chitchat discovery;
- a raft-backed metadata control plane with a durable log;
- deterministic rendezvous **ownership**: every key has one owner among admitted members;
- **peer fetch / owner load**: a member can read from, or ask the owner to load, instead of every member hitting the database.

See [Client and Member Cluster](client-member-cluster.md).

**Move on when** you need values to survive the restart of the process that loaded them, or a cache tier shared by several services or languages.

## 4. Server Grid

`hydracache-server` members hold replicated values. Replication is configured explicitly:

| Setting | Meaning |
| --- | --- |
| `replication_factor` | total copies, including the primary |
| `read_quorum` / `write_quorum` | replicas that must answer a read / acknowledge a write |
| `sync_backups` / `async_backups` | backups acknowledged before returning, and best-effort backups |
| `max_replicated_entry_bytes` | largest value accepted by replication |

Each operation can also choose a [consistency level](../concepts/consistency-model.md#grid-consistency-levels) — from `One` to `All` — and fails loudly when the level cannot be met. Repair (read repair, hinted handoff, Merkle repair) and versioned tombstones keep replicas converging without resurrecting deletes. Online resharding and checkpoints support scaling and recovery.

Applications reach the grid through [Client SDKs](client-sdks.md) or the [Redis API](redis-api.md). Deployment is covered in [Server and Deployment](server-and-deployment.md).

## 5. Multi-region

Members in several regions replicate across region links. Two write-authority modes exist:

| Mode | Writes accepted in | Cross-region consistency |
| --- | --- | --- |
| Home region only (default) | the partition's home region | reads elsewhere may be stale, bounded |
| Active-active | any region; ordering still converges through the home region | bounded staleness / causal+ |

> [!IMPORTANT]
> Active-active must be enabled with an explicit acknowledgement that cross-region consistency is bounded staleness — the configuration is rejected without it. Cross-region linearizability is a permanent non-goal.

Region failover is conservative: a region that merely shows symptoms is *suspected*, and promotion away from it requires an explicit *down* declaration.

## Choosing

```text
One instance?                               → 1. Embedded
Several instances, writes must propagate?   → 2. + Redis/NATS invalidation
Hot keys reloaded by every instance?        → 3. Client/member cluster with peer fetch
Shared tier for several services/languages? → 4. Server grid
Users in several regions?                   → 5. Multi-region (accept bounded staleness)
```

Each step's correctness claims are checked by deterministic simulation and real-process tests; see [How We Test Correctness](../concepts/how-we-test-correctness.md). The cluster layer is staging-ready but young in production — plan runbooks accordingly.

## Next Steps

- [Consistency Model](../concepts/consistency-model.md) — the guarantees behind each step
- [Server and Deployment](server-and-deployment.md) — running members
- [Interactive simulator](https://javaquasar.github.io/hydracache/demo/) — watch elections, partitions, and recovery
