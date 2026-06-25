# Java/Spring Migration Contract

Release 0.49 targets the painful migration path first: legacy Spring and
Hibernate services that already use Hazelcast concepts but only need cache
behavior. The Java toolkit is cache-focused and sits on the W1 client protocol,
the W2 Hibernate L2 contract, W3 conformance behavior, W4 tenant isolation, W5
residency, and W6 observability/audit.

It is not a Hazelcast wire-compatible client and it is not a Hazelcast API
clone. Unsupported Hazelcast APIs fail loud with a migration hint from the
checked-in unsupported-API manifest.

HydraCache 0.52 documents the Java lock facade contract and Rust-side protocol
mapping only. It does not publish a Maven/Gradle Java SDK artifact from this
workspace; buildable Java artifacts remain a separate delivery item.

## Artifacts

- `hydracache-java-client`: typed Java client over the protocol-v1/v2 HTTP/2
  length-prefixed binary frame. Protocol v1 covers cache operations; protocol v2
  is required for the 0.52 IMap/Fenced Lock surface.
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
`evictRegion`. These cache operations map to the protocol-v1/v2 compatibility
window and never expose server-side code execution. Lock/CAS extensions require
protocol v2 negotiation before dispatch.

## Fenced Lock Release Contract

The 0.52 lock facade follows the same explicit-release shape that Java users
already expect from `Lock.unlock()`: the successful lock path returns a guard or
handle that exposes the fence token, and the guaranteed release path is an
explicit `unlock().await` / `unlock()` call through the client.

Client-side `LockGuard::Drop` must not attempt an async network unlock. Rust
`Drop` is synchronous, cannot await a transport round trip, and may run after the
runtime/channel is already gone. A guard may record a non-blocking abandon hint
for diagnostics, but that hint is not a correctness guarantee. The server-side
logical lease/session expiry is the safety net for crashed clients or forgotten
explicit unlocks, and stale owners are rejected by fence/session checks.

## Hazelcast Lock Mapping

This is source-level migration ergonomics, not Hazelcast wire compatibility.

| Hazelcast concept | HydraCache equivalent | Notes |
| --- | --- | --- |
| `IMap.lock(key)` | `HydraFencedLock.lock(key)` | Protocol-v2 `TryLock` with client-side wait/retry semantics. |
| `IMap.tryLock(key)` | `HydraFencedLock.tryLock(key)` | Immediate protocol-v2 `TryLock`; returns busy without blocking. |
| `FencedLock.lockAndGetFence()` | `HydraFencedLock.lockAndGetFence()` | Returns the fence token; callers must pass it to the external system of record. |
| `FencedLock.getFence()` / `isLocked()` | `GetLockOwnership` | Reads ownership metadata from the partition leader. |
| `forceUnlock()` | privileged `ForceUnlock` | Requires admin authorization, writes an audit event, and advances the fence before the next owner. |
| `getCPSubsystem().getLock(...)` | lock-only mapping | Other CP structures remain unsupported. |

The GC-pause story is fence-first: a paused client may resume after its logical
lease expired, but its old fence is rejected after another owner advances the
fence. This is a single-key linearizable lock surface. It is not cross-region
linearizable, not a distributed transaction primitive, and not reentrant across
processes unless callers preserve the same session identity.

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
The refused group still includes non-lock CP structures, executor service,
Hazelcast SQL, entry processors, server-side interceptors, general pub/sub,
ringbuffer, replicated map, and CRDT object APIs.

This keeps the migration honest: HydraCache supports cache migration, not a
transparent replacement for all Hazelcast runtime features.

The list reflects what the toolkit refuses **today**, not a permanent ceiling for
every entry. `RULES.md` R-2 fixes only the permanent non-goals (distributed
transactions, cross-region linearizability, remote code execution); distributed
locks are **not** among them. The **lock-by-key subset** (`IMap.lock` /
`IMap.tryLock` / CP `FencedLock`) has a planned supported path — a single-key
linearizable HydraCache fenced lock with a returned fencing token — tracked in
[`V0_52_IMAP_AND_FENCED_LOCK_JAVA_SURFACE_PLAN.md`](../plans/V0_52_IMAP_AND_FENCED_LOCK_JAVA_SURFACE_PLAN.md)
(status: planned; not yet shipped). The remaining refused APIs — entry processors /
`executeOnKey`, Hazelcast SQL, executor service, `ReplicatedMap`, ringbuffer /
reliable topic as an event log, and CRDT object APIs — stay unsupported: they are
either permanent R-2 non-goals or out of scope for the cache wedge.
