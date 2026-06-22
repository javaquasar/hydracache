# Distributed Cache Coordination

## Concept

Distributed cache coordination lets multiple application instances share invalidation events, reduce duplicate loads, and eventually route cache fills through owner nodes.

## Why It Matters For HydraCache

HydraCache should support a path from local cache to cluster-aware cache without becoming a full distributed database.

## Current Direction

Cluster roles:

- `Local`: no distributed dependencies
- `Client`: local near-cache plus synchronization with cluster members
- `Member`: participates in ownership and serves peer fetches

Phase 2 adds invalidation synchronization.
Phase 3 adds member/client ownership and distributed fill coordination.

Coerce-rs adds one extra angle to this topic: actor-style internal ownership for distributed control-plane components. It is useful as a design reference for message-driven tasks such as membership watchers, invalidation consumers, and peer fetch services, but not as a dependency candidate without a maintenance review.

## Reference Projects

- [Groupcache reread](../../../groupcache/GROUPCACHE_HYDRACACHE_REREAD.md)
- [Hazelcast reread](../../../hazelcast/HAZELCAST_HYDRACACHE_REREAD.md)
- [Olric reread](../../../olric/OLRIC_HYDRACACHE_REREAD.md)
- [Coerce-rs reread](../../../coerce-rs/COERCE_RS_HYDRACACHE_REREAD.md)

## Open Questions

- Should standalone member mode be a separate binary or just an embedding pattern?
- What discovery mechanism should Phase 3 start with: static peers, DNS, or service discovery trait?
- How should `Client` mode behave when no members are reachable?
