# HydraCache Reference Reread Index

> Purpose: second-pass reference map for HydraCache's current product direction.
> Focus: local-first cache core, DB adapters, invalidation, and cluster `Local / Client / Member` roles.
> All links are relative so the index remains usable if the workspace moves.

---

## 1. Current HydraCache Direction

Primary product docs:

- [Unified architecture](./HYDRACACHE_UNIFIED_ARCHITECTURE.md)
- [Architecture review](./HYDRACACHE_ARCHITECTURE_REVIEW.md)

HydraCache's current direction is:

- a convenient embedded local cache first
- DB query/result caching as an adapter layer, not the whole product
- distributed synchronization later
- explicit cluster roles in later phases: `Local`, `Client`, `Member`

---

## 2. Highest-Priority Rereads

### Local cache core

- [Moka reread](../moka/MOKA_HYDRACACHE_REREAD.md)
- [Caffeine reread](../caffeine/CAFFEINE_HYDRACACHE_REREAD.md)
- [HikariCP reread](../hikaricp/HIKARICP_HYDRACACHE_REREAD.md)

### Distributed coordination

- [Groupcache reread](../groupcache/GROUPCACHE_HYDRACACHE_REREAD.md)
- [Hazelcast reread](../hazelcast/HAZELCAST_HYDRACACHE_REREAD.md)
- [Olric reread](../olric/OLRIC_HYDRACACHE_REREAD.md)
- [Coerce-rs reread](../coerce-rs/COERCE_RS_HYDRACACHE_REREAD.md)

### DB adapter boundary

- [SQLx reread](../sqlx/SQLX_HYDRACACHE_REREAD.md)
- [ReadySet reread](../readyset/READYSET_HYDRACACHE_REREAD.md)

---

## 3. What Each Project Is Best For

| Project | Best use for HydraCache now | Local repo |
|---|---|---|
| `groupcache` | ownership, peer fetch, cross-node dedup, hot-cache patterns | [groupcache](../groupcache) |
| `hazelcast` | `member` / `client` model, service layering, cluster lifecycle | [hazelcast](../hazelcast) |
| `moka` | default local backend constraints and extension seams | [moka](../moka) |
| `caffeine` | policy design, deferred maintenance, ergonomic local cache API | [caffeine](../caffeine) |
| `olric` | embedded + standalone duality, clustering, pub/sub invalidation | [olric](../olric) |
| `coerce-rs` | actor-style runtime ownership, message-driven coordination, supervision ideas; older project, latest public release observed `2023-10-16` | [coerce-rs](../coerce-rs) / [GitHub](https://github.com/LeonHartley/Coerce-rs) |
| `sqlx` | first DB adapter seam, compile-time SQL boundary, pool/statement cache split | [sqlx](../sqlx) |
| `hikaricp` | hot-path discipline, practical defaults, low-overhead maintenance | [hikaricp](../hikaricp) |
| `readyset` | freshness/invalidation boundary, anti-scope reference, connector ideas | [readyset](../readyset) |

---

## 4. Recommended Return Path

If revisiting the references later, use this order:

1. `groupcache`
2. `hazelcast`
3. `olric`
4. `coerce-rs`
5. `moka`
6. `caffeine`
7. `sqlx`
8. `hikaricp`
9. `readyset`

Reasoning:

- `groupcache` and `hazelcast` most directly affect the future cluster model
- `olric` and `coerce-rs` help refine embedded/clustered runtime boundaries
- `moka` and `caffeine` most directly affect the quality of the local product
- `sqlx` determines whether DB adapters stay clean instead of leaking into the core
- `readyset` is more valuable as a boundary marker than as a direct implementation template

---

## 5. Evidence Base

Each reread doc builds on the project's deeper knowledge base and points back to it:

- [Groupcache KB](../groupcache/GROUPCACHE_KNOWLEDGE_BASE.md)
- [Hazelcast KB](../hazelcast/HAZELCAST_KNOWLEDGE_BASE.md)
- [Moka KB](../moka/MOKA_KNOWLEDGE_BASE.md)
- [Caffeine KB](../caffeine/CAFFEINE_KNOWLEDGE_BASE.md)
- [Olric KB](../olric/OLRIC_KNOWLEDGE_BASE.md)
- [Coerce-rs KB](../coerce-rs/COERCE_RS_KNOWLEDGE_BASE.md)
- [SQLx KB](../sqlx/SQLX_KNOWLEDGE_BASE.md)
- [HikariCP KB](../hikaricp/HIKARICP_KNOWLEDGE_BASE.md)
- [ReadySet KB](../readyset/READYSET_KNOWLEDGE_BASE.md)
