# HydraCache 0.65.0 Redis Debt Safety Net - Codex Execution Plan

> **At a glance**
> - **What:** a **backend-agnostic characterization + contract test net** around the `0.63`
>   `hydracache-redis-compat` facade so the deferred **distributed RESP backend** (the `0.63` Plan A
>   debt) can be built later with mechanical proof that (1) the translation layer, (2) the
>   client-surface execution contract, (3) core invariants (tenancy/limits/audit), (4) cross-subsystem
>   isolation, and (5) protocol compatibility are all preserved. It also adds **executable
>   flip-sentinels** that fail loud the moment node-local divergence is paid down, forcing docs/claims
>   to update in lockstep.
> - **Why:** `0.63` shipped a **single-endpoint, node-local** RESP facade backed by a per-daemon
>   `Mutex<BTreeMap>` inside `ClientSurfaceState`. The deferred debt (cross-node visibility, distributed
>   `SET NX`, atomic `MSET` across ownership boundaries, EXAT/PXAT, more commands) will **replace that
>   backend**. Today the tests are welded to the concrete in-memory store, so a backend swap has no
>   reusable conformance suite to satisfy and no guardrail proving the rest of the system still holds.
>   This release builds that net **before** the risky refactor, not during it.
> - **After (depends on):** `0.63.0` Redis RESP Edge Facade (the facade under test) and `0.64.0`
>   test-expansion discipline (canary/flip-sentinel + `doc-check` enforcement patterns).
> - **Unblocks:** a safe future distributed-RESP-backend release (`0.63` Plan A) and stronger `1.0`
>   correctness evidence for the outward Redis surface.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md) -
> conformance manifest: [`../integrations/redis_compat_conformance.json`](../integrations/redis_compat_conformance.json)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md)
first. This is a **test-expansion** release. It may include narrow production fixes **only** if a new
test exposes a real bug; no distributed backend, no new Redis command, no product/Redis/Hazelcast
protocol feature, and no core change belong here. The win condition is a reusable safety net, not new
surface area.

## Design Principle: three test layers, each with a defined behavior when the debt is paid

The debt payment (distributed RESP backend) will keep the wire protocol and translation identical while
replacing the execution backend and widening from single-endpoint to cross-node. The net is therefore
split into three layers with **explicitly different** future behavior:

1. **Contract suite (backend-agnostic).** Survives the backend swap unchanged. Any backend that claims
   to serve the RESP facade must pass it. Debt payment = "make the distributed backend pass this suite."
2. **Characterization goldens (byte-level).** Freeze the current observable wire output. Stay green for
   the single-endpoint case after the refactor; any drift is caught immediately.
3. **Flip-sentinels (executable TODOs).** Assert today's node-local divergence (`kind=deployment_scope`
   in the conformance manifest). They **must fail** when the debt is paid, forcing the sentinel and the
   public claim to change together. This is the mechanism that keeps docs honest across the refactor.

No test in this release asserts a distributed guarantee as if it already exists. Aspirational behavior
is encoded only as a flip-sentinel of the current documented divergence, never as a passing
distributed-consistency assertion.

## Non-Goals

- **No distributed RESP backend.** This release does not replicate the RESP store, add cross-node
  visibility, or make `SET NX` cluster-wide. It builds the proof harness the future backend must pass.
- **No new Redis commands or option families.** `EXAT`/`PXAT`, `SET XX/GET/KEEPTTL`, transactions,
  general Lua, and new data structures stay out of scope; they may be added as **documented-divergence
  manifest rows with a flip-sentinel**, never as new supported behavior.
- **No core, raft, consensus, or native fenced-lock change.** The `SingleKeyConditionalStore` /
  `FencedLock` engine is only *observed* for non-interference, never modified.
- **No weakening or muting of existing behavior** to make a test green; no lowering of loud errors.
- **No product API, ownership routing, or Hazelcast protocol work.**

## Preflight

Re-grep the current repo before writing tests; do not assume the seams from this plan:

- translation seam: `translate_redis_command`, `RedisTranslatedCommand` (`Execute`/`Immediate`/
  `Extension`), `RedisExecutionPlan`, `RedisResponseReducer`, `RedisCommand`.
- execution seam: `ClientSurfaceState`, `handle_conditional_put`, `handle_compare_value_invalidate`,
  `handle_compare_value_expire`, `handle_put`, `handle_batch_put`, the store
  `Mutex<BTreeMap<StoreKey, StoredValue>>`, `admit_put`/`admit_request`, audit recorder.
- protocol seam: `ConditionalPutCondition`, `CompareValueExpireMode`, `PROTOCOL_VERSION` (=4),
  `TTL_PROTOCOL_VERSION` (=3), `protocol_version_supported`.
- conformance/doc-check seam: `docs/integrations/redis_compat_conformance.json` command rows and their
  `kind`/`oracle`/`tests`; `check_redis_compat_conformance` in `crates/xtask/src/doc_check.rs`
  (already enforces `kind=deployment_scope` rows name a real `fn ...(` in
  `crates/hydracache-server/tests/redis_resp_multinode.rs`).
- native fenced-lock seam (for non-interference only): `SingleKeyConditionalStore`, `current_fence`,
  `FenceToken`, `expire_due`.
- existing tests not to duplicate: `conditional_put_if_absent_is_atomic_under_contention`,
  `del_and_exists_return_redis_integer_counts`, `mget_preserves_order_and_represents_misses_as_nil_bulk`,
  `setex_psetex_expire_pexpire_persist_and_ttl_pttl_match_redis_semantics`,
  `eval_redis_py_extend_adds_to_remaining_ttl_and_rejects_persistent_keys`,
  `sha1_hex_matches_known_answer_vectors`,
  `lock_script_sha_fingerprints_are_frozen_for_reviewed_client_versions`,
  `multinode_resp_facade_documents_node_local_state`,
  `multinode_resp_lock_subset_is_single_endpoint_only`.

Audit question:

```text
For each supported RESP command, is its observable behavior (wire bytes, integer counts, nil shape,
atomicity, error class, TTL semantics, tenancy/limit/audit path) pinned by a test that does NOT depend
on the concrete in-memory store, so a distributed backend can be validated against the same assertions?
```

Where the answer is "no", that command is a hole this release must close.

## W1. Backend-agnostic ClientSurface conformance suite

Goal: extract the execution-contract assertions from the concrete `ClientSurfaceState` into a suite that
runs against **any** backend, so the future distributed backend is validated by re-running it.

Design:

- Define a minimal backend seam (a trait or an enum of constructors) that yields "a thing that answers
  `ClientRequest` envelopes with `ClientResponse` envelopes". The current node-local
  `ClientSurfaceState` is one implementation; the seam exists so a second implementation can be dropped
  in later without editing the suite. If a trait would touch shipped types, keep it a **test-only**
  seam in `hydracache-cluster-testkit` or a `#[cfg(test)]` harness module - no product API change.
- Move/duplicate the behavioral asserts into a parameterized module that takes the backend under test.

Required tests (each runs against the node-local backend now; reusable later):

- `conformance_conditional_put_if_absent_is_atomic_under_n_concurrent_acquirers` (exactly one winner,
  losers observe the winner's token).
- `conformance_conditional_put_treats_expired_key_as_absent`.
- `conformance_compare_value_invalidate_is_token_safe_and_returns_applied_count`.
- `conformance_compare_value_expire_add_to_remaining_and_replace_if_expiring_and_persistent_guard`.
- `conformance_batch_put_is_all_or_nothing_under_injected_item_failure`.
- `conformance_ttl_states_missing_persistent_expiring_round_trip`.
- `conformance_expired_key_absent_for_get_mget_exists_del`.
- `conformance_enforces_value_bytes_batch_and_tenant_quota_limits`.

Definition of Done:

```powershell
cargo test -p hydracache-client-transport-axum client_surface_conformance --locked
cargo test -p hydracache-redis-compat --locked
```

## W2. Exhaustive translation contract table (manifest-linked)

Goal: freeze the pure translation mapping - every supported command -> `(ClientRequest` shape,
`RedisResponseReducer)` - so a backend refactor cannot silently alter what a command means, and no new
command can be added outside the table.

Design:

- One table-driven test enumerating every `status=supported` (and `supported_with_caveat`) row in
  `redis_compat_conformance.json` and asserting the exact translated `RedisTranslatedCommand`
  (`Execute` plan initial requests + reducer, `Immediate` value, or `Extension` kind).
- Assert the table is **complete**: every supported manifest command has a row, and every row maps to a
  real translator arm (fail loud on drift in either direction).

Required tests:

- `every_supported_command_maps_to_frozen_client_request_and_reducer`.
- `translation_table_has_no_supported_manifest_command_missing_and_no_extra_row`.
- `lock_script_kinds_map_to_frozen_conditional_or_compare_value_shapes` (simple/redlock/redis-py
  release, extend, reacquire -> exact `ConditionalPut`/`CompareValueAndExpire{mode}`).

Definition of Done:

```powershell
cargo test -p hydracache-redis-compat translation_contract --locked
cargo run -p xtask --locked -- doc-check
```

## W3. Deferred-capability flip sentinels

Goal: encode each deferred distributed capability as a documented-divergence sentinel that **fails loud
when the debt is paid**, extending the two `0.63` node-local sentinels into a full set.

Design:

- Add `kind=deployment_scope`, `oracle=documented_divergence` manifest rows, each naming a network-gated
  sentinel in `crates/hydracache-server/tests/redis_resp_multinode.rs` (already `doc-check`-enforced to
  exist).
- Each sentinel asserts today's node-local reality and carries a comment: "flip when the distributed
  RESP backend lands."

Required tests (network-gated, `skip_unless_daemon_process_e2e`):

- `cross_node_mget_del_exists_are_node_local` (write on A, `MGET`/`EXISTS` on B report miss/zero, `DEL`
  on B reports 0).
- `cross_node_lock_release_is_node_local` (acquire on A, redis-py/redlock release script on B returns 0
  and does not free A's lock).
- `cross_node_lock_extend_is_node_local` (extend on B does not affect A's TTL).
- `cross_node_mset_is_node_local` (`MSET` on A invisible on B).

Acceptance standard: no sentinel may pass by asserting a distributed guarantee; each is explicitly a
documented-divergence proof that must be rewritten (not deleted) when Plan A ships.

Definition of Done:

```powershell
cargo run -p xtask --locked -- doc-check
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test redis_resp_multinode --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

## W4. RESP3 response-encoding re-certification

Goal: close the gap the `0.63` accuracy pass explicitly left open - RESP3 response forms were not
re-certified. Freeze RESP3 encoding as byte goldens so future command/backend work cannot regress the
RESP3 dialect.

Required tests:

- `resp3_null_uses_underscore_encoding_and_resp2_uses_dash_one` (contention `SET NX`, `GET` miss,
  `MGET` miss element under both dialects).
- `resp3_integer_array_bulk_and_error_frames_match_golden_bytes`.
- `resp3_unsupported_aggregate_inputs_fail_loud_before_mutation` (Map/Set/Push/attributes/nested
  non-string args).
- `resp3_lock_and_ttl_subset_round_trips_after_hello3` (PING/SET/GET/MSET/MGET/TTL/PTTL/QUIT +
  `SET NX PX` contention null).

Definition of Done:

```powershell
cargo test -p hydracache-redis-compat resp3_ --locked
cargo test -p hydracache-redis-compat --test resp_boundaries --locked
```

## W5. Core-invariant tests: the RESP path never bypasses the core

Goal: prove the facade drives the client surface (tenancy/limits/accounting/audit) rather than the cache
directly - especially for the newer lock-conditional paths - and never leaks secrets. These invariants
must hold regardless of backend.

Required tests:

- `resp_lock_ops_go_through_admit_and_emit_audit` (`ConditionalPut`/`CompareValue*` call `admit_*`,
  respect limits, and record audit events like ordinary writes).
- `oversized_lock_token_or_value_is_rejected_loud_not_truncated`.
- `resp_lock_and_extension_bytes_never_appear_in_logs_metrics_stats_or_diagnostics` (keys, tokens,
  script bodies, script args).
- `translate_redis_command_is_total_and_never_panics_on_arbitrary_commands` (proptest: always
  `Execute`/`Immediate`/`Extension` or a loud error).
- `mget_len_equals_keys_len_and_exists_count_is_bounded_for_arbitrary_inputs` (proptest).

Definition of Done:

```powershell
cargo test -p hydracache-redis-compat core_invariants --locked
cargo test -p hydracache-redis-compat --test resp_resource_smoke --locked
```

## W6. Cross-subsystem non-interference and protocol-version regression

Goal: prove that the RESP facade and its `v4` lock additions do not disturb the rest of the system, and
that older clients remain compatible - the "other parts still work" evidence.

Required tests:

- `resp_lock_state_is_independent_of_native_fenced_lock_engine` (RESP `SET NX` on a key and the native
  `SingleKeyConditionalStore`/`FencedLock` on the same logical name do not read or mutate each other).
- `enabling_redis_listener_does_not_change_core_cache_or_client_surface_behavior` (identical core
  behavior with the RESP listener enabled vs disabled; the fast path is untouched, R-10).
- `protocol_v2_and_v3_clients_are_still_accepted_after_v4_lock_shapes`.
- `protocol_v2_v3_clients_never_receive_v4_conditional_shapes` (version-gated request/response).
- `redis_facade_does_not_register_in_release_dependency_graph_beyond_declared_crates` (reuse the
  `verify-no-test-features` discipline so the safety-net seam stays test-only).

Definition of Done:

```powershell
cargo test -p hydracache-redis-compat non_interference --locked
cargo test -p hydracache-client-protocol --locked
cargo run -p xtask --locked -- verify-no-test-features
```

## W7. Docs, gates, and release ledger

Goal: record the safety net as the contract for the future debt payment, and enforce it in `doc-check`.

Design:

- `docs/integrations/redis_compat_conformance.json`: add the new `deployment_scope` sentinel rows (W3)
  and a short `test_layers` note distinguishing contract-suite / characterization / flip-sentinel rows.
- `docs/TESTING.md`: add a "Redis debt safety net" section describing the three layers and, critically,
  the **payment protocol**: paying the debt = (1) implement the distributed backend against the W1
  suite, (2) keep W2/W4 goldens green for the single-endpoint case, (3) **flip** each W3 sentinel and
  rewrite its manifest row + public claim in the same change.
- `docs/GATES.md`: fast-tier rows for W1/W2/W4/W5/W6; network-gated row for the W3 sentinels; wire the
  `doc-check` extension.
- Extend `check_redis_compat_conformance` (`crates/xtask/src/doc_check.rs`) if any new manifest field
  (`test_layers`) is added, so a documented layer without its named test fails `doc-check` - the same
  anti-dangling rule that already guards `deployment_scope` rows.
- `docs/releases/0.65.0.md`: state this is a test-expansion release that builds the net for the deferred
  distributed RESP backend and changes no product surface.
- Reconcile `releases.toml`, `INDEX.md`, plan header, `docs/COMPAT.md` (only if any wire/byte golden is
  newly declared), `docs/GATES.md`, `docs/TESTING.md`, and `docs/releases/0.65.0.md`.

Definition of Done:

```powershell
cargo run -p xtask --locked -- doc-check
cargo test -p xtask --locked
```

## Gates (Definition of Done for the release)

- The backend-agnostic client-surface conformance suite (W1) passes against the node-local backend and
  is structured so a second backend implementation runs it unchanged.
- Every `supported`/`supported_with_caveat` manifest command has a frozen translation-table row (W2);
  the table fails loud on any missing or extra command; `doc-check` green.
- Each deferred distributed capability has a network-gated `deployment_scope` flip-sentinel (W3) that
  asserts current node-local divergence and is named in the manifest and implemented in
  `redis_resp_multinode.rs`; no sentinel asserts a distributed guarantee.
- RESP3 response encoding is re-certified by byte goldens including dialect-correct null and loud
  rejection of unsupported aggregate frames (W4).
- The RESP path (including `v4` lock-conditional ops) is proven to go through tenancy/limits/audit and
  to leak no keys/tokens/script bytes; the translator is total and never panics (W5).
- RESP lock state is proven independent of the native fenced-lock engine; enabling the listener does not
  change core behavior; `v2`/`v3` clients remain accepted and never receive `v4` shapes; no test-only
  seam leaks into a release dependency graph (W6).
- `TESTING.md` documents the three layers and the **debt-payment protocol**; `GATES.md`/`COMPAT.md`/
  release notes/`releases.toml`/`INDEX.md`/plan header are reconciled; `doc-check` green.
- No product/backend/consensus/native-lock change; any production fix is narrow, test-driven, and named.

## Final Release Decision

Ship `0.65.0` only when the three layers exist and are self-consistent: the **contract suite** is
backend-agnostic and green, the **characterization goldens** freeze the current wire behavior, and every
**flip-sentinel** truthfully asserts the node-local divergence it will later invert. The release adds no
distributed behavior and no product surface - its entire value is that the future distributed-RESP-backend
work (the `0.63` Plan A debt) can proceed with a reusable pass/fail contract, a byte-level regression
net, and executable TODOs that force the public compatibility claims to change in lockstep with the
implementation. If any test would have to assert a distributed guarantee that does not yet exist, it is
written as a flip-sentinel of the documented divergence instead. The core, the consensus engine, and the
native fenced-lock stay untouched; the win condition is sharper, reusable proof - not new surface.
