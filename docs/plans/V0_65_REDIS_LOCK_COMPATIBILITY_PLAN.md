# HydraCache 0.65.0 Redis Lock Compatibility Subset - Codex Execution Plan

> **At a glance**
> - **What:** a dedicated Redis RESP lock-compatibility subset for single-endpoint Redis lock
>   migrations: `SET NX PX/EX`, token-safe release, token-safe extension, and real lock-library matrix
>   proof.
> - **Why:** `0.63.0` deliberately ships the Redis cache subset without Redis lock support. Lock
>   migration needs new atomic client-surface operations, script compatibility decisions, protocol
>   versioning, and a heavier ecosystem proof than the 0.63 release should absorb.
> - **After (depends on):** `0.63.0` Redis RESP Edge Facade and the currently planned `0.64.0` raft
>   snapshot proof release. If the roadmap owner renumbers `0.64.0`, this plan can move earlier without
>   changing the technical contract.
> - **Unblocks:** migration of selected Redis lock-library users to HydraCache's RESP facade while
>   preserving the honesty bar: no fake Redis Cluster, no general Lua, no Redlock quorum claim, and no
>   unsafe read-then-write lock shim.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - Redis facade docs:
> [`../integrations/redis-compat.md`](../integrations/redis-compat.md) - gates:
> [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. This is a dedicated compatibility release, not an expansion of
`0.63.0`. Until this plan is implemented and its heavy gates are green, `SET key value NX PX ttl`
remains the documented `0.63` unsupported-loud divergence.

## Design Decisions

### D1. Protocol Version

Lock conditional operations use `hydracache-client-protocol` **v4**. Protocol v3 remains the Redis TTL
metadata/expiry extension registered by `0.63.0`; it must not silently grow lock-only request/response
shapes after the 0.63 compatibility claim.

Required behavior:

- Add v4 request/response shapes for conditional lock mutation and compare-value expiry/invalidation.
- Keep v1/v2/v3 clients accepted for their existing surfaces, but reject v4 lock shapes before mutation
  unless protocol v4 is negotiated.
- Update `docs/COMPAT.md` so v3 is explicitly TTL-only and v4 is explicitly lock-conditional.
- Cover with `client_protocol_v4_registers_lock_conditional_operations`,
  `protocol_v2_v3_clients_do_not_receive_lock_conditional_shapes`, and
  `compat_register_mentions_client_protocol_v4_lock_extension`.

### D2. Release Boundary

The Redis lock subset is not a `0.63` ship gate. It is a full follow-up release because the minimum
honest scope includes:

- `ConditionalPut`/if-absent acquire.
- Compare-value invalidate and compare-value expire primitives.
- Redis `SET NX PX/EX` response semantics including nil/null on contention.
- A lock-script allowlist or equivalent native mapping for safe release/extend.
- Pinned real Redis oracle rows.
- Real lock-library matrix rows, not handcrafted command sequences.
- Protocol v4 compatibility tests.

If any of those rows are missing, the release does not claim Redis lock-library migration support.

### D3. Lua Allowlist Stability

This release must not ship a general Lua runtime. Any `EVAL`/`EVALSHA` support is an allowlisted lock
script compatibility shim bound to pinned client-library versions and pinned script bodies.

Compatibility rule:

- Each supported library/version records the exact script SHA1 and a reviewed canonical form.
- A library bump that changes a script body, argument order, or command trace is a reviewed
  compatibility change and must update the manifest, docs, oracle rows, and client matrix together.
- Unknown scripts, changed scripts, multi-key scripts, and scripts that call unsupported commands fail
  loud before mutation.
- "Whitespace-insensitive" canonicalization is a convenience only after SHA/script-version review; it
  is not a promise that arbitrary equivalent Lua will be accepted.

### D4. KEYS/ARGV Mapping

The script shim must prove argument mapping explicitly. It is not enough to test invalid `numkeys` or
wrong arity.

Required tests:

- `eval_unlock_script_maps_keys1_to_lock_key_and_argv1_to_token`.
- `eval_extend_script_maps_keys1_to_lock_key_argv1_to_token_and_argv2_to_ttl`.
- `eval_extend_script_rejects_missing_swapped_or_non_integer_argv2_without_mutation`.
- `eval_script_rejects_multi_key_or_extra_key_shapes_without_partial_mutation`.

## Goals And Non-Goals

**Goals.**

- `SET lock_key token NX PX ttl_ms` and `SET lock_key token NX EX ttl_seconds`.
- Existing-key acquire failure returns Redis nil/null, not an error.
- Token-safe release deletes only if the stored value still equals the caller's token.
- Token-safe extension updates expiry only if the stored value still equals the caller's token.
- TTL expiry makes an expired lock immediately acquirable.
- Behavior is proven through real Redis client libraries and pinned real Redis oracle rows.

**Non-goals.**

- No Redis Cluster lock routing, hash slots, `MOVED`, `ASK`, or topology.
- No Redlock quorum claim.
- No general Lua runtime or arbitrary server-side scripting.
- No transactions (`MULTI`/`EXEC`/`WATCH`), modules, pub/sub, hashes, or Redisson full-lock claim unless
  their command traces are separately implemented and tested.
- No claim that Redis-compatible locks are fencing locks or CP locks. HydraCache native fenced-lock APIs
  remain the correctness path for systems that need fencing tokens.
- No `SET GET`, `SET KEEPTTL`, `SET XX`, `SET EXAT`, or `SET PXAT` support unless each is promoted with
  its own contract and tests.

## Compatibility Tiers

| Tier | Target | 0.65 claim if implemented | Required proof |
| --- | --- | --- | --- |
| L0 | Raw Redis lock idiom | `SET k token NX PX/EX ttl` works like Redis for single-key string locks | Fast translator tests, protocol v4 client-surface tests, pinned Redis oracle |
| L1 | redis-py `Lock`-style clients | Acquire, release, extend/reacquire work through the library API | L0 plus pinned redis-py script trace, real `.acquire()`/release/extend rows |
| L2 | Node redlock-style clients in single-endpoint mode | Single-node acquire/release/extend work; no quorum claim | L0/L1 plus pinned Node library version and API-level matrix |
| L3 | Go Redis lock libraries | Selected `go-redis` based lock library acquire/release/refresh works | L0/L1 plus pinned Go library version and API-level matrix |
| L4 | Redisson full lock | Only if command analysis proves every required command is in scope | Likely hashes, Lua, pub/sub, watchdog behavior; out of scope by default |

## Workstreams

### W0. Contract And Manifest

- Split `SET NX PX/EX` out of the current unsupported `SET NX/XX/GET/KEEPTTL` row only when the
  implementation is ready.
- Add explicit manifest rows for `SET NX PX/EX`, known lock release script, known lock extend script,
  `SCRIPT LOAD/EXISTS` allowlist behavior, unknown Lua divergence, and non-goal Redis lock surfaces.
- Keep `SET XX`, `GET`, `KEEPTTL`, `EXAT`, and `PXAT` as unsupported unless separately promoted.
- Update `COMMAND` metadata so it does not advertise general Lua or broader Redis lock capabilities.

### W1. Protocol v4 Lock Operations

- Add protocol v4 operations:
  - `ConditionalPut { ns, key, value, condition: IfAbsent, ttl_ms }`.
  - `CompareValueAndInvalidate { ns, key, expected_value }`.
  - `CompareValueAndExpire { ns, key, expected_value, ttl_ms }`.
- Add result shapes that preserve Redis mapping:
  - `ConditionalStored { stored: bool }`.
  - `CompareValueApplied { applied: bool }`.
- Version-gate all v4 lock shapes. v2/v3 clients must not receive or send them successfully.
- Preserve v3 TTL behavior byte-for-byte outside the new v4 negotiation path.

### W2. Client-Surface Atomicity

- Implement conditional acquire under the same store lock as the existing cache mutation path.
- Treat expired entries as absent before evaluating `NX`.
- Ensure contention returns `stored=false` without changing value or TTL.
- Implement compare-value release/extend as single-store-lock mutations.
- Reuse tenant limits, max key/value bytes, deadlines, auth identity, idempotency, metrics, and audit
  behavior.
- Never log lock keys, values/tokens, script bodies, or script arguments.

### W3. Redis SET Option Parsing

- Accept Redis-compatible option order for the supported subset: `NX` plus exactly one relative TTL
  (`EX seconds` or `PX milliseconds`).
- Reject duplicate options, missing TTL values, non-integer TTL, zero/negative TTL, overflow, `XX`,
  `GET`, `KEEPTTL`, `EXAT`, `PXAT`, and unknown options with Redis-shaped errors.
- Map successful acquire to `OK`.
- Map contention to RESP2 null bulk (`$-1`) and RESP3 null (`_`).
- Keep bare `SET`, `SET EX/PX`, `SETEX`, and `PSETEX` on their current 0.63 paths.

### W4. Lock Script Shim

- Support only reviewed release/extend script forms required by pinned library versions.
- Parse `EVAL`, `EVALSHA`, `SCRIPT LOAD`, and `SCRIPT EXISTS` only for the allowlisted lock scripts.
- Verify exact `numkeys`, `KEYS[1]`, `ARGV[1]`, and `ARGV[2]` mapping before mutation.
- Reject unknown SHA, unknown script body, multi-key script, unsupported command inside script, wrong
  arity, non-string token, and invalid TTL before mutation.
- Cache allowlisted `SCRIPT LOAD` SHA metadata per listener/process; do not claim a general Redis script
  cache.

### W5. Client Library And Oracle Proof

- Pin client library versions and record command traces before implementation:
  - redis-py `Lock`.
  - one Node redlock-style library in single-endpoint mode.
  - one maintained Go `go-redis` based lock library.
  - one JVM Jedis/Lettuce lock-script example; Redisson only after trace review.
- Compare raw Redis lock semantics against `redis:6.2.14` and `redis:7.2.5`.
- Use API-level lock tests for real libraries: `.acquire()`/release/extend/refresh, bounded wait on
  contention, stale-token release safety, expiry/reacquire, reconnect behavior, auth-required startup,
  and `rediss://` if claimed.

### W6. Docs, Gates, And Release Notes

- Add a dedicated `docs/releases/0.65.0.md` release note before shipping.
- Update `docs/integrations/redis-compat.md` only after the manifest rows are implemented and tested.
- Keep the `0.63.0` release note explicit that Redis locks are not supported until this release lands.
- Add gate docs for protocol v4, pinned library versions, Docker/pinned Redis oracle, and required
  ecosystem rows.

## Test Plan

Fast tests:

- `client_protocol_v4_registers_lock_conditional_operations`.
- `protocol_v2_v3_clients_do_not_receive_lock_conditional_shapes`.
- `conditional_put_if_absent_is_atomic_under_contention`.
- `conditional_put_treats_expired_key_as_absent`.
- `compare_value_invalidate_removes_only_matching_token`.
- `compare_value_expire_extends_only_matching_token`.
- `set_nx_px_acquires_missing_key_and_returns_ok`.
- `set_nx_px_existing_key_returns_null_without_mutation`.
- `set_nx_contention_uses_resp2_null_and_resp3_null`.
- `set_nx_rejects_get_keepttl_exat_pxat_xx_and_unknown_options`.
- `eval_known_unlock_script_deletes_only_matching_token`.
- `eval_known_extend_script_updates_ttl_only_for_matching_token`.
- `eval_unlock_script_maps_keys1_to_lock_key_and_argv1_to_token`.
- `eval_extend_script_maps_keys1_to_lock_key_argv1_to_token_and_argv2_to_ttl`.
- `eval_extend_script_rejects_missing_swapped_or_non_integer_argv2_without_mutation`.
- `unknown_or_changed_lock_script_fails_loud_before_mutation`.
- `script_load_and_exists_are_allowlist_scoped_not_general_lua_cache`.
- `lock_keys_tokens_and_script_args_are_redacted_from_logs_metrics_and_diagnostics`.

Pinned Redis oracle tests:

- `redis_oracle_set_nx_px_lock_acquire_matches_real_redis`.
- `redis_oracle_set_nx_px_contention_and_expiry_match_real_redis`.
- `redis_oracle_lock_unlock_script_matches_real_redis`.
- `redis_oracle_lock_extend_script_matches_real_redis_with_ttl_tolerance`.
- `redis_oracle_unknown_lua_is_documented_divergence`.

Mainstream client matrix:

- `client_matrix_python_redis_py_lock_acquire_release_extend`.
- `client_matrix_node_redlock_single_endpoint_acquire_release_extend`.
- `client_matrix_go_redis_lock_library_acquire_release_refresh`.
- `client_matrix_jvm_jedis_or_lettuce_lock_script_acquire_release_extend`.
- `client_matrix_redisson_lock_is_supported_or_fails_loud_by_contract`.

## Gates

- `cargo test -p hydracache-client-protocol --locked lock`
- `cargo test -p hydracache-client-transport-axum --locked lock`
- `cargo test -p hydracache-redis-compat --locked lock`
- `HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 HYDRACACHE_REQUIRE_REDIS_ORACLE=1 cargo test -p hydracache-redis-compat --test redis_clients --locked -- --ignored --nocapture`

If Python/Node/Go/JVM rows are release claims, set their matching `HYDRACACHE_REQUIRE_REDIS_CLIENT_*`
flags so skipped ecosystem rows fail the gate. Skip-only green is not acceptable for a lock
compatibility claim.

## Release Decision

Ship only when L0/L1 are implemented and the selected API-level library rows are green against pinned
real Redis and HydraCache. If any selected lock-library row is not green, the release must keep the
0.63 posture: `SET NX PX` remains unsupported-loud, Redis lock libraries are named as not supported,
and no release note implies lock compatibility.
