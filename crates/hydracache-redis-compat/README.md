# HydraCache Redis RESP Compatibility

`hydracache-redis-compat` is the optional RESP edge facade for the Redis cache
subset. It is protocol-compatible for selected cache commands; it is not a Redis
server clone.

## SET Option Scope

HydraCache 0.63 supports bare `SET`, TTL-bearing `SET EX/PX`, `SETEX`,
`PSETEX`, and the narrow expiring Redis lock-acquire subset `SET NX PX/EX`.
The lock-acquire path is backed by `hydracache-client-protocol` v4
`ConditionalPut IfAbsent`, so success returns `OK`, contention returns Redis
nil/null, expired keys are treated as absent, and the write is atomic inside the
client surface.

Redis conditional and retention shapes outside that lock-acquire subset remain
unsupported-loud. `SET NX` without TTL, `SET XX`, `SET GET`, and `SET KEEPTTL`
return Redis-shaped errors before dispatch and must not be faked with a
read-then-write path.

Lock release and extension are supported only through reviewed Lua-script
fingerprints. The reviewed client-library surface for 0.63 is pinned to
`redis-py==5.2.1`, `redis@4.7.0`, and `redlock@5.0.0-beta.2`. redis-py
`Lock.release`, `Lock.extend`, and `Lock.reacquire` are accepted by exact SHA1
fingerprint and by the conservative reviewed canonical form; any client-library
upgrade that changes a script body is a compatibility change, not an automatic
extension of support.

redis-py `Lock.extend` is handled with its real `replace_ttl` semantics: the
default `replace_ttl=False` adds the requested extension to the current remaining
TTL, `replace_ttl=True` replaces the TTL only when the key is already expiring,
and persistent or missing keys return `0` without mutation. This behavior is
safety-critical because replacing instead of adding can expire a lock before the
owning client believes it has ended.

`SET EXAT` and `SET PXAT` also stay unsupported-loud in 0.63, but they are
absolute-expiry candidates rather than lock primitives. Supporting them later
requires an explicit server-clock, past-timestamp, overflow, TTL-tolerance, and
oracle/client test contract.

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
`docs/integrations/redis-compat.md`, and implementation-level notes live in
`docs/integrations/redis-api-implementation-notes.md`.

## Test Anchors

The release plan and conformance manifest pin this contract to executable tests:

- `sha1_hex_matches_known_answer_vectors`
- `lock_script_sha_fingerprints_are_frozen_for_reviewed_client_versions`
- `redis_auth_uses_hardened_credential_comparison_contract`
- `info_returns_minimal_honest_facade_state`
- `set_write_conditional_options_follow_conformance_contract`
- `set_nx_px_acquires_missing_key_and_contention_returns_null`
- `set_nx_ex_ttl_uses_seconds_and_expires`
- `client_matrix_set_nx_px_lock_idiom_acquires_contends_and_releases`
- `eval_redis_py_release_and_reacquire_scripts_are_exact_allowlisted`
- `eval_redis_py_extend_adds_to_remaining_ttl_and_rejects_persistent_keys`
- `compare_value_expire_adds_to_remaining_ttl_for_redis_py_extend`
- `compare_value_expire_expiring_only_rejects_persistent_or_missing_keys`
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

The lock-library compatibility claim is single-endpoint only and is not complete
until the Docker/client matrix is run with both
`HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1` and
`HYDRACACHE_REQUIRE_REDIS_ORACLE=1`; skip-only green is not enough for the
redis-py/redlock lock subset. The multi-daemon release proof must also keep the
planned node-local sentinels wired:
- `multinode_resp_facade_documents_node_local_state`
- `multinode_resp_lock_subset_is_single_endpoint_only`
- `hc_namespace_is_listener_scoped_not_redis_multidb`
- `hc_tag_settags_and_invalidate_tag_use_edge_local_index_and_client_surface`
- `hc_tag_missing_key_does_not_create_metadata_or_mutate`
- `hc_invalidate_tag_prunes_expired_keys_without_counting_them`
- `client_matrix_runs_hydracache_tag_extension_scenario`
- `mainstream_redis_client_can_talk_to_the_facade`
- `nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset`
