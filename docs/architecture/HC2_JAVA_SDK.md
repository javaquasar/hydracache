# HC/2 Java SDK Preview

## Status

H16 now has a publishable-shape Java 17 SDK at
`sdks/java/hydracache-client-hc2`, preview coordinates
`io.hydracache:hydracache-client-hc2:0.68.0-alpha.1-SNAPSHOT`, and a consumer
project that resolves only the installed JAR and POM. H16 is not complete:

- H01 has not mounted HC/2 on the production daemon, so the current independent
  Rust process is a conformance peer, not the daemon;
- H11 has not specified reconnect and subscription/session repair;
- H18 cannot retain a previous Java HC/2 artifact before the first preview is
  released.

The SDK therefore makes no stable-API, Maven Central, production-listener, or
automatic-repair claim.

## Public boundary

The public `io.hydracache.client.hc2` package provides:

- fail-closed gRPC+mTLS connection configuration;
- asynchronous get, put, delete, compare-and-set, and ordered batch operations;
- per-request deadlines, explicit cancellation, idempotency bytes, tenant and
  topology metadata;
- subscriptions with watermark events/gaps and explicit unsubscribe;
- immutable topology snapshots;
- fenced session open, heartbeat, loss handling, and explicit close;
- stable error/retry enums and bounded pull-based metrics.

Generated messages and stubs use
`io.hydracache.client.hc2.internal.wire`. Reflection tests reject any public
signature that leaks those types. The authoritative proto remains in
`crates/hydracache-client-hc2/proto`; H16 added explicit unsubscribe and
session-close messages because listener/session ownership otherwise had no
wire-level release operation.

## Runtime invariants

- Only `https://host:port` is accepted; credentials, query, fragment, path, and
  plaintext have no fallback.
- CA, client certificate, client key, and expected server name are mandatory.
- Handshake generation, connection generation, correlation, and requested
  capabilities are validated before the client is returned.
- Invocation, subscription, session, topology, message, key/value, TTL, and
  idempotency bounds fail before unbounded retention.
- Deadlines emit HC/2 cancellation and late responses cannot recover a removed
  correlation.
- Listener exceptions and executor rejection are counted without terminating
  the transport callback.
- Connection failure completes every pending owner, releases permits, stops
  heartbeat/deadline work, and shuts down the channel. It never retries
  implicitly while H11 remains open.

## Packaging

The Maven build pins Java 17, gRPC/protobuf/plugin versions, dependency
convergence, source and public-API Javadoc JARs, automatic module name, protocol
generation, and preview-stability manifest entries. The separate
`tests/java-hc2-consumer` project imports only the Maven coordinate. Its tests
verify that the resolved code source is an installed JAR and inspect the JAR
manifest rather than reading repository classes or proto files.

## Executable evidence

`cargo xtask client-plane-java-sdk-check`:

1. builds `hc2_java_interop_server` as a separate Rust executable;
2. starts each process with ephemeral CA-signed server/client identities;
3. tests positive mTLS and rejects hostname, expiry, not-yet-valid, EKU, and
   untrusted-client profiles before application dispatch;
4. exercises data/CAS/batch, event delivery, topology, fenced session lifecycle,
   deadline cancellation, stable metrics, and public API isolation;
5. requires a terminal server receipt with zero subscriptions and sessions;
6. installs SDK/POM/source/Javadoc artifacts locally;
7. builds and runs the external consumer against the installed coordinate.

The combined `cargo xtask client-plane-spike-check` includes this gate alongside
the existing Rust transport and Java/Python generation evidence.

## Completion path

After H01 and H11, replace the conformance peer row with a real-daemon matrix,
prove reconnect and deterministic listener/session repair, and retain the first
published preview for H18 old/new compatibility. H18 now retains the first H17
JAR/POM and runs it from an isolated Maven repository as a `baseline-smoke`;
the production old/new rows remain blocked in `HC2_COMPATIBILITY_ARTIFACTS.md`.
Only then may H16 move from
`in progress` to `complete`.
