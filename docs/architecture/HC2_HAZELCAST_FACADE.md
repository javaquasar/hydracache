# HC/2 Hazelcast-shaped Java Facade

## Status and purpose

HydraCache 0.68 provides a narrow source-level migration facade at
`sdks/java/hydracache-hazelcast-facade`. Its preview Maven coordinate is
`io.hydracache:hydracache-hazelcast-facade:0.68.0-alpha.1-SNAPSHOT`.

The artifact is **not** Hazelcast wire compatible, does not publish classes in
`com.hazelcast.*`, and does not claim full `IMap` or CP-subsystem semantics. It
lets an application preserve familiar map and fenced-lock operation shapes
while changing configuration, imports, and dependency coordinates explicitly.

## Supported surface

`io.hydracache.hazelcast.HydraCacheInstance` creates typed `HydraMap<K,V>` and
`HydraFencedLock<K>` objects over an existing native `HydraCacheClient`.

The map subset contains:

- `get` and `put`, including an optional TTL;
- exact-old-value `replace` mapped to HC/2 compare-and-set;
- `remove(key, expectedValue)` mapped to HC/2 conditional remove;
- prefix-backed entry listeners with event watermarks and explicit gap calls;
- `getLock(key)` for the same logical map key.

The lock subset contains:

- immediate and bounded client-side-wait `tryLock`;
- `lockAndGetFence`, `renew`, `unlock`, `isLocked`, and `getFence`;
- a local held-fence view and best-effort explicit close.

The facade is deliberately non-reentrant. Callers must persist the returned
fence beside every protected source-of-truth write and reject stale fences.
Losing a transport response to acquire, unlock, or renew is ambiguous, so the
native recovery client never replays those operations automatically. A caller
may inspect ownership and make an application-specific recovery decision.

## Encoding and isolation

Every key and value uses an explicit `HydraCodec<T>`. UTF-8 and defensive-copy
byte codecs are supplied; Java native serialization and remote class loading
are not. Physical keys carry a versioned binary prefix, object kind, and
bounded UTF-8 map name before the application key bytes. This makes equal
logical keys in different maps distinct and gives a listener a precise prefix
to subscribe to and validate before decoding.

Listener events are cache repair signals, not a durable business event log.
Values are optional on removals, gaps are surfaced rather than concealed, and
callbacks retain the native HC/2 executor and bounded-queue behavior.

## Server mapping

The unified HC/2 protobuf adds `RemoveIfValue`, `TryLock`, `Unlock`,
`RenewLock`, and `LockOwnership`. The production server converts these messages
to the existing `ClientRequest` variants and dispatches them through the same
quorum, lease, and monotonic-fence implementation used by the existing client
protocol. There is no facade-specific lock table or alternate consistency
path.

## Unsupported surface

The packaged
`META-INF/hydracache/hazelcast-capabilities.properties` manifest lists the
supported and unsupported operation families. Server-side entry processors,
interceptors, SQL, executor services, replicated maps, topics, ring buffers,
CRDT counters, and non-lock CP structures remain unsupported. Exposed
unsupported helper methods throw immediately with a pointer to that manifest;
the facade never approximates those semantics silently.

## Build and test

Run the fast local reactor tests without the external Rust-process interop
class:

```powershell
mvn -B -ntp -f sdks/java/pom.xml `
  "-Dtest=HydraCacheClientConfigTest,RecoveringHydraCacheClientTest,HydraCacheFacadeTest" `
  "-Dsurefire.failIfNoSpecifiedTests=false" test
```

The facade tests prove codec and map namespace isolation, get/put/CAS/remove,
listener decoding/watermarks, server-fence propagation, lease renewal,
non-reentrancy, unlock, and loud unsupported behavior. The full native SDK
process/PKI suite remains owned by `client-plane-java-sdk-check`.
`client-package-check` adds the clean external JAR consumer, and the HC/2 CI
workflow runs the reactor and consumer on Java 17/21 before publication.
