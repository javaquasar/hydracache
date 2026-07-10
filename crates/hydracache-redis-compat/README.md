# HydraCache Redis RESP Compatibility

`hydracache-redis-compat` is the optional RESP edge facade for the Redis cache
subset. It is protocol-compatible for selected cache commands; it is not a Redis
server clone.

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

The executable source of truth is
`docs/integrations/redis_compat_conformance.json`; the user-facing explanation is
`docs/integrations/redis-compat.md`.

## Test Anchors

The release plan and conformance manifest pin this contract to executable tests:

- `info_returns_minimal_honest_facade_state`
- `info_section_argument_does_not_fabricate_redis_keyspace_state`
- `resp_listener_info_probe_does_not_fabricate_keyspace_or_cluster_state`
- `type_reports_string_or_none_through_client_surface`
- `resp_listener_type_reports_string_and_none`
- `mainstream_redis_client_can_talk_to_the_facade`
- `nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset`
