# ADR-0006: Hibernate Provider Instead Of Hibernate Or HikariCP Clone

## Status

Accepted for HydraCache 0.49 W2.

## Context

Java applications already have strong ownership boundaries for database access:
Hibernate owns ORM state, dirty checking, transactions, region SPI, and query
cache semantics; HikariCP owns JDBC connection pooling. HydraCache should help
those applications move their cache backend to the HydraCache grid without
pretending to own the ORM or the JDBC pool.

The 0.49 ecosystem release adds a stable external client protocol. That makes a
thin Hibernate second-level cache provider possible without pulling Java runtime
concerns into the Rust workspace.

## Options Considered

- Reimplement or clone Hibernate cache behavior in Rust.
- Reimplement or embed HikariCP-style JDBC pooling ideas in HydraCache.
- Implement Hibernate's supported provider SPI as a thin Java adapter over the
  HydraCache client protocol.

## Decision

HydraCache uses the provider approach. The Java artifact
`hydracache-hibernate` is to implement Hibernate's `RegionFactory` /
`DomainDataRegion` SPI and talk to HydraCache through protocol v1 (the artifact is
**planned**; today only the Rust-side `hydracache_client_protocol::hibernate`
contract ships — see `technical-debt/TD-0005-…`). A Hibernate region maps to a
HydraCache namespace; Hibernate access strategies map to the stable
`hydracache_client_protocol::hibernate` consistency labels.

HikariCP remains the application's JDBC connection pool. HydraCache borrows the
operational discipline that made pooling successful: explicit limits, fail-fast
bootstrap checks, clear lifecycle, and observable health. It does not pool JDBC
connections or join database transactions.

## Consequences

This keeps the Rust workspace focused on the cache/grid and keeps Hibernate
version churn inside a small Java adapter. It also makes the migration path
boring in the useful sense: Java applications configure a provider rather than
rewriting ORM internals.

The tradeoff is a new JVM conformance surface. The provider must pin its
Hibernate ORM 6.x support window, map every access mode to a documented
HydraCache consistency label, and prove query-cache timestamp behavior or refuse
query-cache support loud at bootstrap.

## Revisit When

Revisit this decision if Hibernate removes the provider SPI, if a standard cache
provider API becomes a better fit than Hibernate-specific SPI, or if a future
protocol version offers a schema-generated client surface that can reduce the
Java adapter code without hiding the same semantics.
