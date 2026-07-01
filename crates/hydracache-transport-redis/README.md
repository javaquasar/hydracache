# hydracache-transport-redis

Redis pub/sub external invalidation transport for HydraCache.

This crate is opt-in. The base `hydracache` crate does not depend on Redis, and
applications only add this package when invalidation frames must cross process,
host, or region boundaries through an operator-owned Redis fabric.

## Configuration

Create a normal `hydracache::TransportConfig` and wrap it in
`RedisTransportConfig`:

```rust,no_run
use hydracache::TransportConfig;
use hydracache_transport_redis::{RedisInvalidationTransport, RedisTransportConfig};

# async fn build() -> Result<(), hydracache::TransportError> {
let core = TransportConfig::new("orders-prod", "node-a")
    .channel("hydracache:inval:orders-prod");
let config = RedisTransportConfig::new("redis://127.0.0.1:6379/", core);
let transport = RedisInvalidationTransport::connect(config).await?;
# let _ = transport;
# Ok(())
# }
```

The Redis URL owns authentication and TLS selection. Use standard Redis URL
forms such as `redis://:password@host:6379/` or `rediss://host:6380/` when the
selected Redis client features support the deployment. Keep channel names stable
per HydraCache cluster and do not share a channel between unrelated clusters.

## Security Boundary

Redis is a transport, not an authorization boundary. Any producer allowed to
publish to the configured channel can request key, tag, or cache-wide flush
invalidations. Deploy Redis ACLs, network policy, TLS, and credentials so only
trusted HydraCache nodes can publish to the invalidation channel.

## Correctness And Over-Invalidation

HydraCache invalidation frames carry no cached values. Redis delivery is treated
as at-least-once: fencing is the correctness mechanism, while message-id
deduplication is an optimization. Malformed frames and unsupported future frame
versions are reported loudly and skipped.

Cache-wide flush frames are valid and intentionally conservative. A compromised
or misconfigured publisher can cause over-invalidation and elevated origin load,
so isolate channels per cluster and monitor relay drop/error counters.
