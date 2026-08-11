# API Links

Use docs.rs for exact Rust API signatures. This mdBook explains how the pieces fit together.

## Core Runtime

| API | Link |
| --- | --- |
| `HydraCache` | <https://docs.rs/hydracache/latest/hydracache/struct.HydraCache.html> |
| `CacheOptions` | <https://docs.rs/hydracache/latest/hydracache/struct.CacheOptions.html> |
| `CacheKeyBuilder` | <https://docs.rs/hydracache/latest/hydracache/struct.CacheKeyBuilder.html> |
| `TagSet` | <https://docs.rs/hydracache/latest/hydracache/struct.TagSet.html> |
| `TypedCache` | <https://docs.rs/hydracache/latest/hydracache/struct.TypedCache.html> |
| `RefreshOptions` | <https://docs.rs/hydracache/latest/hydracache/struct.RefreshOptions.html> |

## Function Caching

| API | Link |
| --- | --- |
| `cacheable_loader!` | <https://docs.rs/hydracache/latest/hydracache/macro.cacheable_loader.html> |
| `cacheable_infallible!` | <https://docs.rs/hydracache/latest/hydracache/macro.cacheable_infallible.html> |
| `#[cacheable]` | <https://docs.rs/hydracache/latest/hydracache/attr.cacheable.html> |

## Database Caching

| API | Link |
| --- | --- |
| `DbCache` | <https://docs.rs/hydracache-db/latest/hydracache_db/struct.DbCache.html> |
| `QueryCachePolicy` | <https://docs.rs/hydracache-db/latest/hydracache_db/struct.QueryCachePolicy.html> |
| `PreparedQueryPolicy` | <https://docs.rs/hydracache-db/latest/hydracache_db/struct.PreparedQueryPolicy.html> |
| `query_cache_policy!` | <https://docs.rs/hydracache-db/latest/hydracache_db/macro.query_cache_policy.html> |
| `HydraCacheEntity` | <https://docs.rs/hydracache-db/latest/hydracache_db/derive.HydraCacheEntity.html> |
| `prepared_query_policy!` | <https://docs.rs/hydracache-db/latest/hydracache_db/macro.prepared_query_policy.html> |

## Redis Compatibility

| API | Link |
| --- | --- |
| `RedisRespServer` | <https://docs.rs/hydracache-redis-compat/latest/hydracache_redis_compat/struct.RedisRespServer.html> |
| `RedisListenerConfig` | <https://docs.rs/hydracache-redis-compat/latest/hydracache_redis_compat/struct.RedisListenerConfig.html> |
| `RedisCommand` | <https://docs.rs/hydracache-redis-compat/latest/hydracache_redis_compat/enum.RedisCommand.html> |

## Adapter Crates

- <https://docs.rs/hydracache-sqlx/latest/hydracache_sqlx/>
- <https://docs.rs/hydracache-diesel/latest/hydracache_diesel/>
- <https://docs.rs/hydracache-seaorm/latest/hydracache_seaorm/>
- <https://docs.rs/hydracache-observability/latest/hydracache_observability/>
- <https://docs.rs/hydracache-actuator-axum/latest/hydracache_actuator_axum/>
- <https://docs.rs/hydracache-redis-compat/latest/hydracache_redis_compat/>
- <https://docs.rs/hydracache-server/latest/hydracache_server/>
