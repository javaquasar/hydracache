# Client SDKs

**API:** [`hydracache-client`](https://docs.rs/hydracache-client/latest/hydracache_client/) · [`hydracache-client-hc2`](https://docs.rs/hydracache-client-hc2/latest/hydracache_client_hc2/)

> [!NOTE]
> **At a glance**
>
> - **Use when:** a service talks to `hydracache-server` instead of embedding the cache.
> - **Crate:** `hydracache-client` (HC/1), `hydracache-client-hc2`, Python and Java SDKs.
> - **Guarantees:** versioned protocols; unknown versions refused before mutation; reconnects report gaps and lost sessions explicitly.
> - **Watch:** SDK retry advice and pull-based metrics.
> - **Pitfalls:** replaying mutations without idempotency keys; ignoring subscription gaps.

Client SDKs talk to a running [`hydracache-server`](server-and-deployment.md). If your service is written in Rust and can embed the cache, you usually do not need one — the embedded cache has no network hop at all.

## Two Protocols

| | HC/1 (external client protocol) | HC/2 (client plane) |
| --- | --- | --- |
| Shape | request/reply binary frames over HTTP/2 | one long-lived bidirectional gRPC stream |
| Security | follows the server's TLS configuration | **mutual TLS, always** |
| Features | data operations, stable errors and retry advice, `IMap`-style locks and CAS, TTL operations, entry-event subscriptions, near-cache repair | data operations, CAS and ordered batches, subscriptions, topology, fenced sessions, deadlines and cancellation, bounded reconnect |
| Versioning | protocol versions 1–4, negotiated; newer operations are gated on the negotiated version | generated contract with a cross-SDK conformance manifest |
| Status | stable | preview |

Both are versioned wire contracts registered in the compatibility register. Unknown future versions and operations the peer did not negotiate are refused before any mutation, and the SDKs are checked against shared, language-agnostic conformance manifests.

## SDKs at a Glance

| Language | Package | Protocol | Status |
| --- | --- | --- | --- |
| Rust | `hydracache-client` | HC/1 | stable reference implementation |
| Rust | `hydracache-client-hc2` | HC/2 | preview |
| Python | `hydracache-client-hc2` (import `hydracache_hc2`) | HC/2 (asyncio) | preview |
| Python | `hydracache-client` | HC/1 | small: protocol constants, retryability, conformance runner |
| Java 17 | `io.hydracache:hydracache-client-hc2` | HC/2 | preview, not yet on Maven Central |
| Java | `hydracache-hazelcast-facade` | — | `HydraMap` / `HydraFencedLock` surface shaped like Hazelcast `IMap` and `FencedLock` |
| Any Redis client | — | RESP | see [Redis API](redis-api.md) |

## Python (HC/2)

```python
from hydracache_hc2 import AsyncHydraCacheClient, ClientConfig

client = await AsyncHydraCacheClient.connect(ClientConfig(
    endpoint="cache.example:9444",
    client_id="orders-api",
    tenant="orders",
    root_certificate="ca.pem",
    client_certificate="client.pem",
    client_private_key="client.key",
    server_name="cache.example",
))
await client.put(b"key", b"value")
value = await client.get(b"key")
await client.close()
```

Keys and values are opaque bytes. TLS is mandatory unless `insecure=True` is set explicitly for loopback tests.

## Java (HC/2)

```java
HydraCacheClientConfig config = HydraCacheClientConfig.builder()
    .endpoint(URI.create("https://cache.example.com:7443"))
    .serverName("cache.example.com")
    .trustCertificate(Path.of("pki/ca.pem"))
    .clientCertificate(Path.of("pki/client.pem"))
    .clientPrivateKey(Path.of("pki/client.key"))
    .clientId("orders-service")
    .tenant("orders")
    .build();

try (HydraCacheClient client = HydraCacheClient.connect(config)) {
  RequestOptions request = RequestOptions.withTimeout(Duration.ofSeconds(2));
  client.put(key, value, Duration.ofMinutes(5), request).join();
  Optional<CacheValue> loaded = client.get(key, request).join();
}
```

`HydraCacheClient.connect` is a single connection. Use `RecoveringHydraCacheClient.connect` with an ordered endpoint list for SDK-owned reconnect.

## Reconnect Semantics

The HC/2 SDKs share the same fail-loud recovery rules:

- **reads may be replayed** after a reconnect; **mutations require an idempotency key** to be replayed;
- a reconnect emits an explicit **subscription gap** that the caller must repair — events are never silently assumed continuous;
- **fenced sessions are lost** on reconnect and must be re-acquired deliberately, never silently;
- TLS, protocol, capability, and cluster-identity failures are terminal and do not fall through to another endpoint.

> [!NOTE]
> HC/2 SDKs are previews: the API can still change, and the Java artifact is built from the repository rather than Maven Central. The Rust HC/1 client is the stable path today.

## Next Steps

- [Server and Deployment](server-and-deployment.md) — run the server the SDKs connect to
- [Redis API](redis-api.md) — use existing Redis clients
- [Consistency Model](../concepts/consistency-model.md) — what the server guarantees
