# hydracache-transport-nats

NATS external invalidation transport for HydraCache.

This crate is opt-in. The base `hydracache` crate does not depend on NATS, and
applications add this package only when invalidation frames must cross process,
host, or region boundaries through an operator-owned NATS subject fabric.

## Configuration

Use `NatsTransportConfig::for_cluster` for the standard subject shape:

```rust,no_run
use hydracache_transport_nats::{NatsInvalidationTransport, NatsTransportConfig};

# async fn build() -> Result<(), hydracache::TransportError> {
let config = NatsTransportConfig::for_cluster(
    "nats://127.0.0.1:4222",
    "orders-prod",
    "node-a",
);
let transport = NatsInvalidationTransport::connect(config).await?;
# let _ = transport;
# Ok(())
# }
```

The default subject is `hydracache.inval.{cluster_name}`. Override the embedded
`hydracache::TransportConfig` channel if your operators require a different
subject. Credentials and TLS are deployment concerns of the NATS URL and client
configuration; use authenticated or TLS-enabled NATS endpoints where the fabric
is not fully private.

## Security Boundary

NATS is a transport, not an authorization boundary. Any producer allowed to
publish to the configured subject can request key, tag, or cache-wide flush
invalidations. Use NATS accounts, subject permissions, network policy, TLS, and
credential rotation so only trusted HydraCache nodes can publish to the
invalidation subject.

## Correctness And Over-Invalidation

HydraCache invalidation frames carry no cached values. NATS delivery is treated
as at-least-once: fencing is the correctness mechanism, while message-id
deduplication is an optimization. Malformed frames and unsupported future frame
versions are reported loudly and skipped.

Cache-wide flush frames are valid and intentionally conservative. A compromised
or misconfigured publisher can cause over-invalidation and elevated origin load,
so isolate subjects per cluster and monitor relay drop/error counters.
