# HC/2 Java SDK Preview

## Status

H16 now has a publishable-shape Java 17 SDK at
`sdks/java/hydracache-client-hc2`, preview coordinates
`io.hydracache:hydracache-client-hc2:0.68.0-alpha.1-SNAPSHOT`, and a consumer
project that resolves only the installed JAR and POM. H16 is complete for the
preview contract. The executable gate covers both an independent Rust
conformance peer and the production daemon, Java owns the H11 reconnect/repair
policy, and H18 retains an immutable first-preview JAR/POM baseline.

This completion does not make a stable-API, Maven Central, or full old/new
rolling-compatibility claim. Those promotion decisions remain release and H18
concerns rather than being implied by the preview SDK.

ADR-0020 keeps this exact SNAPSHOT coordinate in the repository for 0.68.0.
CI still builds, installs, and consumes it from an isolated Maven repository,
but no remote Maven deployment is authorized. External distribution requires
the later full Linux/Docker/fuzz/fixed-host client-promotion admission.

## Public boundary

The public `io.hydracache.client.hc2` package provides:

- fail-closed gRPC+mTLS connection configuration;
- asynchronous get, put, delete, compare-and-set, conditional remove, fenced
  lock acquire/release/renew/ownership, and ordered batch operations;
- per-request deadlines, explicit cancellation, idempotency bytes, tenant and
  topology metadata;
- subscriptions with watermark events/gaps and explicit unsubscribe;
- immutable topology snapshots;
- fenced session open, heartbeat, loss handling, and explicit close;
- ordered bounded reconnect endpoints, explicit endpoint preference, and a
  monotonic logical connection generation;
- separate reconnect and invocation-replay policies: reads may replay, while
  mutations and mutating batches require a nonempty idempotency key;
- explicit subscription gap/repair ownership with watermark deduplication and
  permanent fail-loud fenced-session loss on reconnect;
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
  heartbeat/deadline work, and shuts down the channel.
- TCP refusal/reset is reconnectable, but certificate, TLS identity,
  authentication, protocol-generation, capability, and cluster-identity
  failures are terminal and cannot fall through to another endpoint.
- Reconnect is serialized and bounded. A stale completion from a replaced
  logical generation is rejected; only replay-safe work may be submitted again.
- Every reconnect emits one explicit repair boundary for each subscription.
  Events are suppressed until caller repair, and duplicate watermarks are
  counted and discarded. Fenced sessions are never silently reacquired.

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
5. replaces a killed Rust process through an ordered endpoint list, proves
   bounded fallback, cluster pinning, explicit listener repair, duplicate
   suppression, safe replay rules, and permanent fenced-session loss;
6. starts the actual `hydracache-server` binary with off-by-default HC/2 enabled,
   executes Java data/listener/session operations, requests production admin
   drain, and requires a successful zero-resource process exit;
7. requires terminal conformance-server receipts with zero subscriptions and
   sessions and rejects retained non-daemon SDK threads;
8. installs SDK/POM/source/Javadoc artifacts locally;
9. builds and runs the external consumer against the installed coordinate.

The combined `cargo xtask client-plane-spike-check` includes this gate alongside
the existing Rust transport and Java/Python generation evidence.

## Compatibility boundary

H18 retains the first Java JAR/POM and runs it from an isolated Maven repository
as `baseline-smoke`. That proves artifact independence and prevents the first
preview from being silently replaced. The production old-client/new-daemon,
new-client/old-daemon, and rolling-upgrade rows remain explicitly blocked in
`HC2_COMPATIBILITY_ARTIFACTS.md` until a genuinely later preview exists. H16 is
therefore complete without misrepresenting same-contract smoke evidence as a
future rolling-compatibility result.
