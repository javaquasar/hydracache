# Hazelcast migration evidence (0.69)

HydraCache 0.69 validates the narrow Java migration facade with expectations adapted from
Hazelcast `v5.6.0` at commit `a9ce2a02ac17f88fcd38869ac698e56e613dc40c`. The executable ledger is
[`hazelcast_borrowed_suite.json`](hazelcast_borrowed_suite.json); each row names the upstream source
symbol, the adapted expectation, the HydraCache runner case, and its exact expected outcome.

This is source-level migration evidence, not Hazelcast wire compatibility and not a claim that
Hazelcast's test classes run unchanged. `BasicMapTest` and `MapLockTest` own real Hazelcast clusters
and use Hazelcast internals. `HydraMap` and `HydraFencedLock` intentionally do not implement the
Hazelcast interfaces or depend on the Hazelcast runtime. The runner therefore ports only expectations
inside the already-shipped facade surface.

The final 0.69 slice has 16 rows: 13 expected passes, two documented divergences, and one documented
unsupported operation. The divergences are default `FencedLock` reentrancy and session binding:
Hazelcast is reentrant and session-bound by default, while `HydraFencedLock` is deliberately
non-reentrant and lease/fence-bound. `IMap.executeOnKey` remains loud-unsupported. Neither boundary
was widened to make the suite green.

Manifest schema v2 gives every row typed proof references. Every pass has a non-double proof from
the production daemon, deterministic lock state machine, or recovery interop. The live daemon test
uses two client certificates signed by the fixture CA, so the held-lock row represents two verified
owners rather than two facades sharing one mTLS identity. The TTL row polls until real expiry rather
than proving only that a TTL argument was accepted.

Run the structural check on every change:

```text
cargo run -p xtask --locked -- migration-conformance-check --structural
```

Resolve every cited source path and selector from the exact immutable upstream commits:

```text
cargo run -p xtask --locked -- migration-conformance-check --upstream
```

The upstream check fails closed when a commit/path cannot be downloaded or a selector is absent;
the fast exact-candidate CI lane executes it before accepting borrowed-suite evidence.

Run the Java expectations with Java 17 and Maven available:

```text
HYDRACACHE_RUN_JVM_COMPAT=1 cargo run -p xtask --locked -- borrowed-suite-check --suite hazelcast
```

That gated command is not test-double-only: it starts the production `hydracache-server` behind
the Rust mTLS fixture and executes `HydraMap`/`HydraFencedLock` through the real Java HC/2 client.
It also runs the deterministic production-server lease-expiry test and the Java recovery test that
proves an old fenced session becomes `SESSION_LOST`. Facade locks remain lease/fence-bound rather
than being bound to the Java SDK's separate `FencedSession` lifecycle.

An unexpected pass is a failure just like an unexpected failure: it means the public claim and the
reviewed ledger no longer agree. Changes to the source pin, row set, outcome, divergence reason, or
facade surface require the manifest and this evidence statement to change together.
