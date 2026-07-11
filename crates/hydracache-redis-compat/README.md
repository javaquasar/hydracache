# HydraCache Redis RESP Compatibility

`hydracache-redis-compat` is the optional RESP edge facade for the Redis cache
subset. It is protocol-compatible for selected cache commands; it is not a Redis
server clone.

## SET Option Scope

HydraCache 0.63 supports bare `SET` plus TTL-bearing `SET EX/PX`, `SETEX`, and
`PSETEX`. Redis write-conditional and retention options (`NX`, `XX`, `GET`,
`KEEPTTL`) stay unsupported-loud and return `ERR syntax error` before dispatch.
Those options include Redis lock/conditional-write semantics and must not be
faked with a read-then-write path.

`SET EXAT` and `SET PXAT` also stay unsupported-loud in 0.63, but they are
absolute-expiry candidates rather than lock primitives. Supporting them later
requires an explicit server-clock, past-timestamp, overflow, TTL-tolerance, and
oracle/client test contract. The current gated proof covers raw `SET NX PX`
fail-loud/no-mutation behavior; it does not claim redis-py `Lock`, redlock, or
Redisson lock-library API compatibility.

## Health And Probe Commands

The facade supports only probe commands whose replies can be stated honestly:

- `PING` is the primary liveness probe.
- `COMMAND` advertises only the supported subset.
- `HELLO 2` and `HELLO 3` negotiate RESP dialects.
- `CLIENT SETNAME` and `CLIENT SETINFO` are bounded startup no-ops.
- `INFO` returns a minimal bulk-string snapshot with HydraCache RESP facade facts:
  standalone mode, RESP dialect support, package version, accepted connection
  count, processed command count, and RESP error count. It does not expose fake
  Redis memory, keyspace, replication, or cluster sections.
- `TYPE key` returns `string` for an existing cache value and `none` for a miss.

`ROLE`, `DBSIZE`, and `SCAN` stay unsupported. Returning Redis-like replication
roles, exact keyspace sizes, or iterable keyspace state would either fabricate
Redis server state or create unsafe/expensive tenant-visible behavior.

## Admin Commands

`CONFIG`, `FLUSHDB`, and `FLUSHALL` are recognized but disabled by default:

- `CONFIG` is a Redis server administration surface for reading or changing
  runtime server configuration. The HydraCache RESP facade must not return fake
  Redis configuration or pretend that Redis memory, persistence, TLS, ACL, or
  replication settings were changed.
- `FLUSHDB` deletes every key in the selected Redis database. HydraCache exposes
  only one Redis-compatible logical database, so mapping this command would be a
  broad tenant/namespace destructive operation rather than a normal cache-subset
  command.
- `FLUSHALL` deletes every key in every Redis database. HydraCache does not expose
  a Redis-global server keyspace through this facade, and a Redis client must not
  be able to wipe broader HydraCache state by accident.

All three commands return stable `NOPERM ... is disabled by the HydraCache Redis
facade` errors before dispatching to `ClientSurfaceState`, so they do not mutate
keys or fabricate Redis server state. A future destructive/admin capability
should be a HydraCache-native admin API with explicit scope, authorization,
audit, and rollout gates rather than a Redis-compatible default.

## HydraCache Tag Extensions

`HC.NAMESPACE`, `HC.TAG`, `HC.SETTAGS`, and `HC.INVALIDATE_TAG` are
HydraCache-only RESP extension commands:

- `HC.NAMESPACE` reports the listener namespace; `HC.NAMESPACE <same>` returns
  `OK`; any other namespace fails loud. This is not Redis multi-db support.
- `HC.TAG key tag [tag ...]` attaches non-empty UTF-8 tags to an existing live
  key in this listener's local tag index and returns the number of newly added
  tags.
- `HC.SETTAGS key tag [tag ...]` replaces the listener-local tag set for an
  existing live key and returns the number of unique tags stored.
- `HC.INVALIDATE_TAG tag` looks up only keys explicitly tagged through this
  listener and invalidates live matches through `ClientSurfaceState`.

The tag index is edge-local and in-memory. It is not a Redis Cluster topology,
not a persisted global HydraCache tag index, and not a scan over visible keys.
Missing or expired keys return/count as `0`; stale tag entries are pruned during
tag invalidation.

The executable source of truth is
`docs/integrations/redis_compat_conformance.json`; the user-facing explanation is
`docs/integrations/redis-compat.md`.

## Test Anchors

The release plan and conformance manifest pin this contract to executable tests:

- `info_returns_minimal_honest_facade_state`
- `set_write_conditional_options_follow_conformance_contract`
- `set_nx_px_lock_idiom_has_declared_behavior_and_redis_shaped_error`
- `client_matrix_raw_set_nx_px_fails_loud_promptly_without_mutation`
- `expire_zero_or_negative_deletes_key_and_returns_one`
- `expired_by_nonpositive_expire_is_absent_for_get_mget_exists_ttl`
- `expire_pexpire_and_persist_on_missing_key_return_zero`
- `rejected_set_and_expire_shapes_use_redis_error_class_or_documented_normalization`
- `info_section_argument_does_not_fabricate_redis_keyspace_state`
- `resp_listener_info_probe_does_not_fabricate_keyspace_or_cluster_state`
- `type_reports_string_or_none_through_client_surface`
- `resp_listener_type_reports_string_and_none`
- `admin_commands_are_disabled_by_default_without_config_or_flush_mutation`
- `resp_listener_admin_commands_are_disabled_before_mutation`
- `hc_namespace_is_listener_scoped_not_redis_multidb`
- `hc_tag_settags_and_invalidate_tag_use_edge_local_index_and_client_surface`
- `hc_tag_missing_key_does_not_create_metadata_or_mutate`
- `hc_invalidate_tag_prunes_expired_keys_without_counting_them`
- `client_matrix_runs_hydracache_tag_extension_scenario`
- `mainstream_redis_client_can_talk_to_the_facade`
- `nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset`
