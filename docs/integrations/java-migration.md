# Java/Spring Migration Contract

Release 0.49 targets the painful migration path first: legacy Spring and
Hibernate services that already use Hazelcast concepts but only need cache
behavior. The Java toolkit is cache-focused and sits on the W1 client protocol,
the W2 Hibernate L2 contract, W3 conformance behavior, W4 tenant isolation, W5
residency, and W6 observability/audit.

It is not a Hazelcast wire-compatible client and it is not a Hazelcast API
clone. Unsupported Hazelcast APIs fail loud with a migration hint from the
checked-in unsupported-API manifest.

## Artifacts

- `hydracache-java-client`: typed Java client over the protocol-v1 HTTP/2
  length-prefixed binary frame.
- `hydracache-spring-boot2-starter`, `hydracache-spring-boot3-starter`, and
  `hydracache-spring-boot4-starter`: Boot-generation-specific
  auto-configuration with one shared runtime model.
- `hydracache-spring-cache`: Spring Cache integration with `native`, `jcache`,
  and `none` modes.
- `hydracache-jcache`: optional JCache binding for applications already wired
  through `javax.cache` or `jakarta.cache`.
- `hydracache-hibernate`: Hibernate L2 provider that follows
  `docs/integrations/hibernate.md`.

## Client Configuration

The default application topology is client-first. Application JVMs do not become
HydraCache members in 0.49.

```yaml
hydracache:
  client:
    endpoints:
      - https://cache-a.internal:8443
      - https://cache-b.internal:8443
    tenant: core
    client-name: gameservice
    smart-routing: true
    deadline-ms: 5000
    identity:
      token: ${HYDRACACHE_TOKEN}
    retry:
      max-attempts: 3
      initial-backoff-ms: 25
      max-backoff-ms: 1000
```

The same runtime model is used by the Boot 2, Boot 3, and Boot 4 starters.
Customizer hooks may adjust transport settings, but they must not bypass tenant
identity, request deadlines, frame limits, or protocol version negotiation.

## Map Facade

Hazelcast-style direct map usage:

```java
HazelcastInstance hz = HazelcastClient.newHazelcastClient(config);
IMap<String, UserProfile> users = hz.getMap("users");
users.put(userId, profile);
UserProfile cached = users.get(userId);
```

HydraCache migration target:

```java
HydraCacheClient client = HydraCacheClient.create(config);
HydraCacheMap<String, UserProfile> users =
    client.getMap("users", Codecs.string(), UserProfileCodec.INSTANCE);
users.put(userId, profile);
UserProfile cached = users.get(userId);
```

The facade is deliberately narrow: `get`, `put`, `putIfAbsent`, `remove`,
`containsKey`, `getAll`, `putAll`, `invalidate`, `clearNamespace`, and
`evictRegion`. These operations map to protocol-v1 cache operations and never
expose server-side code execution.

## Codec And Schema Registration

Java native serialization and reflective fallback serializers are disabled by
default. Production code should register explicit codecs or schemas:

```java
@HydraCacheCodec(id = "user-profile-v1", type = UserProfile.class, schemaVersion = 1)
public final class UserProfileCodec implements HydraCacheCodec<UserProfile> {
    public static final UserProfileCodec INSTANCE = new UserProfileCodec();
}
```

Package scanning may register `@HydraCacheCodec` or `@HydraCacheSchema`
descriptors. Duplicate codec ids, schema-version mismatches, reflective
fallbacks, and Java native serialization fail fast. A legacy serializer bridge
can only be enabled explicitly and should be treated as migration-only risk.

## Spring Cache Modes

`native` mode preserves the most important legacy behavior: dynamic Spring cache
names are lazily resolved to named HydraCache maps.

```yaml
hydracache:
  toolkit:
    spring-cache:
      mode: native
```

`jcache` mode binds Spring Cache through JCache when `hydracache-jcache` is on
the classpath. If the provider is absent, startup fails with a clear dependency
message.

`none` mode does not auto-configure a Spring `CacheManager`; an application bean
wins.

## Hibernate L2

```yaml
hydracache:
  toolkit:
    hibernate:
      l2:
        enabled: true
        extended-config: true
        region-factory: HYDRACACHE_LOCAL
        use-query-cache: false
        use-statistics: true
```

The starter delegates to the W2 provider contract. It uses put-if-absent
property customization so existing `spring.jpa.properties.*` always wins.

## Listeners

```java
@Component
@HydraCacheMapListener(map = "users", includeValue = false)
public class UserCacheListener implements HydraCacheEntryInvalidatedListener<String> {
    @Override
    public void entryInvalidated(HydraCacheEntryEvent<String> event) {
        // refresh local projections or invalidate application-local state
    }
}
```

Listeners register after Spring singletons are ready and deregister on context
shutdown. Stream resume uses W1 watermarks. `includeValue` is always subject to
W5 residency checks, and unsupported listener interfaces fail loud at startup.

## Micrometer And Actuator Probe

The Java toolkit exposes bounded Micrometer meters and an Actuator near-cache
probe. The probe loads an entity, reloads it to verify a near-cache hit,
evicts/invalidates, reloads cold, and returns structured status. It mirrors the
operational shape of the existing Hazelcast toolkit without exposing Hazelcast
internals.

## Unsupported Hazelcast APIs

The Java toolkit refuses unsupported APIs from
`crates/hydracache-client-protocol/manifests/unsupported_hazelcast_apis.txt`.
The first refused group includes CP structures, locks, executor service,
Hazelcast SQL, entry processors, server-side interceptors, general pub/sub,
ringbuffer, replicated map, and CRDT object APIs.

This keeps the migration honest: HydraCache supports cache migration, not a
transparent replacement for all Hazelcast runtime features.
