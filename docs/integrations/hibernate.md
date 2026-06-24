# Hibernate L2 Provider Contract

HydraCache 0.49 supports Hibernate second-level cache integration as a provider
contract over the stable external client protocol. The Java artifact is
`hydracache-hibernate` and implements Hibernate's `RegionFactory` /
`DomainDataRegion` SPI outside the Cargo workspace. HydraCache stays a Rust cache
server; it does not clone Hibernate and does not join Hibernate/JVM
transactions.

## Supported Matrix

The first supported provider line is Hibernate ORM 6.x. Hibernate 5.6, if added
later, must use a separate compatibility module and pass the same conformance
suite before it is claimed.

## Region Mapping

A Hibernate region maps to one HydraCache `Namespace`.

| Hibernate concept | HydraCache protocol shape |
| --- | --- |
| Entity region | `Namespace("hibernate:<region>")`, structured keys starting with `entity` |
| Collection region | `Namespace("hibernate:<region>")`, structured keys starting with `collection` |
| Natural-id region | `Namespace("hibernate:<region>")`, structured keys starting with `natural-id` |
| Query result region | `Namespace("hibernate:<region>")`, structured keys starting with `query` |
| Update timestamps region | `Namespace("hibernate:<region>")`, structured keys starting with `timestamps` |

The Rust-side contract is in `hydracache_client_protocol::hibernate`.
`RegionMapping` builds the `Get`, `Put`, `Invalidate`, and `EvictRegion`
requests the Java provider must send over protocol v1.

## Access Mode Mapping

The provider maps Hibernate access strategies to explicit HydraCache consistency
semantics:

| Hibernate access strategy | HydraCache contract label | Protocol context | Invalidation boundary |
| --- | --- | --- | --- |
| read-only -> strong-immutable | `strong-immutable` | `ReadConsistency::Strong`, no write consistency | immutable; no write-driven invalidation |
| nonstrict-read-write -> best-effort-invalidate | `best-effort-invalidate` | `ReadConsistency::Eventual`, `WriteConsistency::Local` | provider invalidates when it observes a write |
| read-write / transactional -> invalidate-on-commit | `invalidate-on-commit` | `ReadConsistency::Session`, `WriteConsistency::Quorum` | provider invalidates from transaction-completion callbacks |

HydraCache does not join the JVM transaction. For `read-write` and
`transactional`, the Java provider observes Hibernate transaction completion and
then publishes invalidation intent through the W1 client protocol. Failed,
rolled-back, or unobserved transactions must not be represented as successful
HydraCache writes.

## Query Cache

Query cache behavior is explicit. The supported mode uses a query-result
namespace plus a Hibernate update-timestamps namespace. Bulk updates evict both
the query-result region and the timestamps region with `EvictRegion`.

If a provider build does not implement timestamp/bulk invalidation, query cache
support must fail loud at bootstrap. It may not silently treat query regions like
ordinary entity regions.

## Compatibility

This mapping is versioned as Hibernate provider contract `1` and registered in
`docs/COMPAT.md`. The contract is intentionally small:

- region-to-namespace mapping is deterministic;
- access-mode consistency labels are stable;
- `EvictRegion` clears the whole namespace;
- query cache support is timestamp/bulk invalidation or explicit bootstrap
  refusal;
- the provider never claims distributed transaction ownership.

## Required Tests

The Rust contract gate is:

```powershell
cargo test -p hydracache-client-protocol --locked hibernate_contract
```

The future Java provider gate is run in the nightly Docker tier:

```powershell
mvn -pl hibernate-provider test
```

The Java conformance suite must cover read-only immutability, non-strict
best-effort invalidation, read-write invalidation on commit, query cache
timestamp behavior or loud refusal, Hibernate version matrix checks, and
failover against a two-node HydraCache grid.
