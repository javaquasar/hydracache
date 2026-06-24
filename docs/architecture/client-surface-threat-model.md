# HydraCache 0.49 Client Surface Threat Model

The external client surface is a new trust boundary. It is opt-in, lives under
`/client/v1/*`, and is owned by `hydracache-client-transport-axum` rather than the
internal member transport.

## Threats And W0 Controls

| Threat | Control |
| --- | --- |
| Downgrade attempts | W1 version negotiation refuses out-of-window versions loud; W0 reserves `/client/v1/*` as the only v1 route boundary. |
| Malformed or truncated frames | Frames are decoded after identity and size checks; decode failures become explicit client errors, never panics. |
| Oversized payloads | `max_frame_bytes`, `max_value_bytes`, `max_batch_entries`, and `max_batch_bytes` reject abuse before state mutation. |
| Tenant spoofing | W0 requires identity headers before dispatch; W4 replaces this with authoritative tenant resolution and roster validation. |
| Replay / idempotency abuse | W1 request envelopes must carry request ids and idempotency keys for retry-safe writes. |
| Subscription floods | W0 models `max_streams_per_connection`, idle timeout, heartbeat interval, and graceful subscription drain. |
| Batch abuse | W0 reserves batch limits; W1/W4 enforce batch admission with the same quota/rate budget as single-key operations. |
| Metric-label cardinality attacks | W4 validates tenant ids against a bounded roster before tenant labels are emitted. |
| Audit redaction failures | W6 audit payloads never include values and keep per-key detail out of metrics. |
| Governance bypass attempts | W5 residency decisions fail closed and W6 audits refusals. |
| Route confusion | `/client/v1/*` public routes and `/cluster/*` member routes are owned by different crates and tested as disjoint. |

## Non-Goals

The client surface does not add distributed transactions, remote code execution,
server-side expression evaluation, or Hazelcast wire compatibility. It is a
HydraCache protocol boundary with fail-loud compatibility rules.
