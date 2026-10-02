# 0.74 local semantic-risk coverage

This matrix records the local tests used while screening W2/W3/W7/W8 candidates. It is not release
qualification and does not replace Redis-oracle, HC/1, HC/2, persistence, long-run, or expensive
host gates.

| Risk | Local coverage | Current result |
| --- | --- | --- |
| Fragmented RESP frames | `resp_boundaries::partial_resp_frames_decode_like_complete_frames`; RESP2/RESP3 partial-pipeline boundary tests | Passed |
| Multiple pipelined frames | `resp_boundaries::multiple_resp_frames_in_one_read_are_all_processed`; `resp_pipeline_attribution_074` | Passed |
| Partial writes and `Pending` | `resp_transport_adversarial_074::fragmented_frames_short_writes_and_pending_flush_preserve_pipeline`; retained scripted writer fixture | Passed on Windows and WSL2 |
| Disconnects and half-open frames | `connection_chaos::half_open_and_reset_connections_free_resources_without_leaking_inflight_work`; churn/limit recovery | Passed |
| Slow readers/backpressure | `resp_transport_adversarial_074::closed_write_gate_backpressures_one_connection_without_starving_another`; subscription lag tests | Passed on Windows and WSL2 |
| Large legal payload/reply | `resp_transport_adversarial_074::large_reply_survives_repeated_pending_and_one_byte_writes`; W8 ownership tests at 1 MiB | Passed |
| Oversized or malformed input | `resp_resource_smoke`, `resp_boundaries`, and W3 malformed-middle-frame fixture | Passed focused non-ignored tests; resource smoke remains an explicit opt-in gate |
| Expiration races | `native_semantic_concurrency_074::expiration_race_removes_each_batch_entry_once_and_returns_only_misses`; existing expiry/overwrite conformance | Passed; one sweep removes each due entry once |
| Multi-key atomicity | `native_semantic_concurrency_074::concurrent_batch_get_observes_all_old_or_all_new_values`; batch rejection and duplicate-key corpus | Passed; readers observe no torn batch |
| Exact response/order | every local profiler receipt plus RESP pipeline byte-identity fixtures | Passed |
| Tenant isolation/quota | client-surface conformance, W7 candidate tests, W8 isolated profiler | Passed; no policy or quota relaxation |
| Verified-session revocation | W8a policy-generation replacement, namespace fallback, result/metric parity and `Send + Sync` fixtures | Passed before targeted W8a revert |
| Concurrent tenant authorization | `native_semantic_concurrency_074::concurrent_callers_never_reuse_authorization_across_tenant_bindings` | Passed; 32/32 forged calls denied and audited |

Focused commands used in this work include:

```text
cargo test -p hydracache-redis-compat --test resp_boundaries
cargo test -p hydracache-redis-compat --test connection_chaos
cargo test -p hydracache-redis-compat --test resp_pipeline_attribution_074
cargo test -p hydracache-redis-compat --test resp_transport_adversarial_074
cargo test -p hydracache-client-transport-axum --test client_surface_conformance
cargo test -p hydracache-client-transport-axum --test client_surface
cargo test -p hydracache-client-transport-axum --test native_semantic_concurrency_074
```

The W3 candidate-specific file was removed with the rejected product candidate, but its green test
execution and exact candidate receipts are retained in `w1-local-attribution.md`. Reintroducing a
different W3 implementation requires equivalent scripted-I/O coverage; the old result cannot be
silently inherited.

The same rule applies to W8a: its candidate-only generation and parity tests passed before the
targeted rollback, but a future policy/session design must carry equivalent revocation coverage.
