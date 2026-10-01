# 0.74 local semantic-risk coverage

This matrix records the local tests used while screening W2/W3/W7/W8 candidates. It is not release
qualification and does not replace Redis-oracle, HC/1, HC/2, persistence, long-run, or expensive
host gates.

| Risk | Local coverage | Current result |
| --- | --- | --- |
| Fragmented RESP frames | `resp_boundaries::partial_resp_frames_decode_like_complete_frames`; RESP2/RESP3 partial-pipeline boundary tests | Passed |
| Multiple pipelined frames | `resp_boundaries::multiple_resp_frames_in_one_read_are_all_processed`; `resp_pipeline_attribution_074` | Passed |
| Partial writes and `Pending` | W3 candidate `resp_pipeline_write_074::partial_socket_writes_do_not_split_high_level_batch_or_reorder_replies` and scripted writer fixtures | Passed before targeted candidate revert |
| Disconnects and half-open frames | `connection_chaos::half_open_and_reset_connections_free_resources_without_leaking_inflight_work`; churn/limit recovery | Passed |
| Slow readers/backpressure | W3 scripted small-buffer slow-reader test; subscription lag disconnect tests remain in the standard suite | Passed before targeted candidate revert |
| Large legal payload/reply | W3 oversized legal reply one-item progress fixture; W8 ownership tests at 1 MiB | Passed |
| Oversized or malformed input | `resp_resource_smoke`, `resp_boundaries`, and W3 malformed-middle-frame fixture | Passed focused non-ignored tests; resource smoke remains an explicit opt-in gate |
| Expiration races | client-surface concurrent expiry/overwrite conformance and local-cache expiry tests | Passed |
| Multi-key atomicity | client-surface batch-put rejection, RESP oversized DEL rejection, duplicate-key/last-write-wins corpus | Passed; documented concurrent DEL writer debt is unchanged |
| Exact response/order | every local profiler receipt plus RESP pipeline byte-identity fixtures | Passed |
| Tenant isolation/quota | client-surface conformance, W7 candidate tests, W8 isolated profiler | Passed; no policy or quota relaxation |
| Verified-session revocation | W8a policy-generation replacement, namespace fallback, result/metric parity and `Send + Sync` fixtures | Passed before targeted W8a revert |

Focused commands used in this work include:

```text
cargo test -p hydracache-redis-compat --test resp_boundaries
cargo test -p hydracache-redis-compat --test connection_chaos
cargo test -p hydracache-redis-compat --test resp_pipeline_attribution_074
cargo test -p hydracache-client-transport-axum --test client_surface_conformance
cargo test -p hydracache-client-transport-axum --test client_surface
```

The W3 candidate-specific file was removed with the rejected product candidate, but its green test
execution and exact candidate receipts are retained in `w1-local-attribution.md`. Reintroducing a
different W3 implementation requires equivalent scripted-I/O coverage; the old result cannot be
silently inherited.

The same rule applies to W8a: its candidate-only generation and parity tests passed before the
targeted rollback, but a future policy/session design must carry equivalent revocation coverage.
