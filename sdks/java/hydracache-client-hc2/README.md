# HydraCache HC/2 Java Client (preview)

This Java 17 SDK is a preview artifact for the generated HC/2 client plane. It
is buildable and consumable as a normal Maven dependency and is tested against
the production HydraCache daemon, but it is not yet published to Maven Central.

```xml
<dependency>
  <groupId>io.hydracache</groupId>
  <artifactId>hydracache-client-hc2</artifactId>
  <version>0.68.0-alpha.1-SNAPSHOT</version>
</dependency>
```

The API has no plaintext mode. Supply a trusted CA, client certificate, client
private key, and the expected TLS server name:

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

The preview exposes data operations, CAS and ordered batches, subscriptions,
topology, fenced sessions, deadlines/cancellation, stable errors and retry
advice, and pull-based metrics. Generated protobuf/gRPC types live under
`io.hydracache.client.hc2.internal.wire` and never appear in public signatures.

Run the complete package and independent-consumer proof from the repository
root:

```text
cargo xtask client-plane-java-sdk-check
```

`HydraCacheClient.connect` remains a single-connection primitive. Applications
that want SDK-owned recovery use `RecoveringHydraCacheClient.connect` with an
ordered endpoint list plus separate bounded reconnect and invocation-replay
policies. Reads may replay; mutations and mutating batches require a nonempty
idempotency key. Reconnect emits an explicit subscription gap that the caller
must repair, deduplicates watermarks, and permanently loses fenced sessions
instead of silently reacquiring them. TLS/certificate, protocol, capability,
and cluster-identity failures are terminal and never fall through to another
endpoint.
