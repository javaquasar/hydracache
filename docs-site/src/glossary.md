# Glossary

**Authority**
The source of truth for ownership, topology, and which version is newer. In HydraCache it is a raft-backed control plane with monotonic epochs, never wall-clock time. See [Consistency Model](concepts/consistency-model.md).

**Cache key**
The identity of exactly one cached value. It must contain every dimension that changes the result (tenant, filters, page, permission scope). See [Keys and Tags](concepts/keys-and-tags.md).

**Client (role)**
An application-side cache that joins a cluster as a near-cache without being a member that owns data. See [Client and Member Cluster](guides/client-member-cluster.md).

**Collection**
A group of entities of the same kind, such as `users`. Collection tags invalidate every cached list or query over that kind.

**Consistency level**
The number of replica acknowledgements a grid operation requires: `One`, `LocalQuorum`, `Quorum`, `EachQuorum`, or `All`. An unsatisfiable level fails instead of downgrading.

**Entity**
A domain object with a stable id, such as `user:42`. `HydraCacheEntity` derives its key and tags.

**Epoch**
A monotonically increasing number attached to authority decisions. When a newer epoch disagrees with an older hint, the epoch wins.

**Generation**
A per-process counter that prevents a restarted process from being confused with its previous incarnation under the same node id.

**Invalidation intent**
A message saying "remove this key / these tags". Intent travels between processes; values do not (outside the grid layer).

**InvalidationPlan**
A list of invalidations staged while preparing a write and executed only after the transaction commits. Dropping it on rollback leaves the cache untouched.

**Invariant checker**
Code that inspects a simulated or recorded run and fails if a safety property is violated, such as two leaders in one term. See [How We Test Correctness](concepts/how-we-test-correctness.md).

**Loader**
The async function that produces a value on a cache miss — usually a database query or an API call.

**Member (role)**
A cluster participant that can own data and serve peers.

**Near cache**
A local copy of remotely owned data, kept close to the application for fast reads.

**Outbox (transactional)**
A table written in the same transaction as the business data, holding invalidation intent. It is published only if the transaction commits, so the intent survives a crash between commit and publish.

**Owner**
The member responsible for a key under deterministic rendezvous ownership. Other members can fetch from it (peer fetch).

**Peer fetch / owner load**
Reading a value from the member that owns it, or asking the owner to load it, instead of every member hitting the database.

**Refresh-ahead**
Reloading a hot entry shortly before it expires so readers never see a miss.

**Single-flight**
Coalescing concurrent misses for the same key into one loader call. See [Single-flight](concepts/single-flight.md).

**Stale-if-error / stale-on-loader-error**
Serving the last known value for a bounded window when the loader fails.

**Stale-while-revalidate**
Serving an expired value for a bounded window while one background load refreshes it.

**Tag**
A label describing which writes can invalidate a group of entries, such as `user:42`, `users`, or `tenant:7:users`.

**Tombstone**
A versioned marker left by a delete in the grid so that an older replica cannot resurrect the value. Tombstones are garbage-collected only after repair confirms it is safe.

**TTL**
Time to live: an upper bound on how long an entry can be served. In HydraCache it is a safety net for missed invalidations, not the main freshness mechanism.
