# Macros

HydraCache macros remove repetition around cache boundaries without making the boundary implicit.

They are useful after the key, tags, TTL, and loader shape are already clear. If those choices are still being designed, start with the explicit runtime APIs first.

## Function Wrappers

Use `cacheable_loader!` when one call site has a fallible async loader and the cache metadata belongs next to the call.

Use `cacheable_infallible!` when the loader returns a value directly. The cache operation can still fail because serialization, storage, or runtime boundaries can fail.

```rust
{{#include ../../examples/src/bin/macros.rs:function-wrapper}}
```

Use `#[cacheable]` when the cached operation is naturally a reusable async function.

```rust
{{#include ../../examples/src/bin/macros.rs:cacheable-attribute}}
```

The cache remains an explicit argument. HydraCache does not discover a global cache and does not derive keys from every function argument.

## Entity Metadata

Use `HydraCacheEntity` when repository code repeatedly caches entity-shaped values.

```rust
{{#include ../../examples/src/bin/macros.rs:entity-derive}}
```

The derive produces the `CacheEntity` metadata used by `DbCache`, `QueryCachePolicy`, prepared policies, and invalidation plans:

- entity key: `user:42`;
- entity tag: `user:42`;
- optional collection tag: `users`.

Use `#[hydracache(id = Type)]` on the struct when the id type is generated or not represented by one named field.

## Query Policies

Use `query_cache_policy!` when one query call site should declare the whole key/tag/freshness contract in one expression.

```rust
{{#include ../../examples/src/bin/macros.rs:query-policy}}
```

The macro does not inspect SQL. It only builds the same `QueryCachePolicy` that could be written with builder calls.

Use `prepared_query_policy!` when most metadata is stable and only the entity id changes at call time.

```rust
{{#include ../../examples/src/bin/macros.rs:prepared-policy}}
```

Prepared policies keep hot repository methods compact while preserving an explicit dynamic id boundary.

## Write-side Invalidation

The entity derive also keeps write-side invalidation small.

```rust
{{#include ../../examples/src/bin/macros.rs:invalidation-plan}}
```

Stage invalidations while preparing repository work, then execute the plan only after the database transaction commits.

## Choosing A Macro

| Macro | Use when |
| --- | --- |
| `cacheable_loader!` | One fallible async loader needs local cache metadata. |
| `cacheable_infallible!` | One infallible async loader should avoid `Ok::<_, E>(value)` ceremony. |
| `#[cacheable]` | A reusable async function has stable key/tag metadata. |
| `HydraCacheEntity` | Entity keys and invalidation tags repeat across repository code. |
| `query_cache_policy!` | One query call site should declare key, tags, TTL, and refresh policy compactly. |
| `prepared_query_policy!` | A repository method reuses stable policy metadata and binds only ids or segments per call. |

Prefer macro inputs that read like the cache contract a reviewer should approve. If a macro invocation becomes hard to review, move some metadata into named builders or prepared policies.
