# HC/2 Client-Plane Observability Contract and Runbook

Status: H21 complete for the non-production client-plane spike. Production
listener wiring remains owned by H01; this document does not promote an HC/2
adapter or install a public metrics endpoint.

Schema: `hydracache.hc2.client_plane.v1`

## Purpose and boundary

H21 replaces object-local, unnamed counters with a stable typed export from
`ClientPlaneDiagnostics`. A telemetry adapter can consume one bounded snapshot
through `DiagnosticSink`, serialize it as JSON, or map it to an existing
Prometheus/OTLP registry. The recorder has no network, global registry, or
telemetry-SDK dependency, so a blocked exporter cannot enter the client fast
path.

The authoritative inputs are:

- `SpikeConnection::{state, connection_generation, resources, metrics}`;
- the closed `SecurityRejection`, `DeadlineOutcome`, and `DrainReason`
  taxonomies;
- explicit retry, repair, session-heartbeat, and session-loss events from
  their owning services.

H01 must connect the selected real adapter to this contract. It must not copy
the spike into the daemon or claim that H21 alone proves a production listener.

## Cardinality and privacy invariants

Every exported label has a finite domain:

| Label | Domain | Rule |
| --- | --- | --- |
| `transport` | 3 enum values | never endpoint, URI, authority, or peer address |
| `state` | 6 enum values | only on the current-state gauge and trace records; never on monotonic counters |
| `tenant_bucket` | integers `0..63` | SHA-256 over a per-process 256-bit salt, tenant length, and tenant bytes; raw tenant is discarded immediately |
| `reason` | reviewed enum values | no error text or peer-provided string |

Connection generation is a gauge, not a label. Correlation IDs, session IDs,
cache names, keys, values, credentials, certificate DER/subjects/SANs, client
IDs, raw tenant IDs, endpoints, and exception messages are not accepted by the
export types.

The salt must be random per process and all-zero configuration fails closed.
It is redacted by `Debug`. Tenant buckets deliberately trade identity for a
hard cardinality ceiling; they are useful for coarse hotspot correlation, not
tenant billing or audit. Across restarts, bucket identity is not stable.

One recorder retains at most 128 trace records. The configured capacity must
be within `1..=128`; the oldest record is dropped and
`hydracache_hc2_trace_dropped_total` increments. One snapshot emits no more
than 48 series.

## Metric catalog

All metrics carry `transport` and `tenant_bucket`. The connection-state gauge
alone carries `state`; this prevents a lifecycle transition from splitting a
monotonic counter into new time series. Only the three reason families carry
`reason`.

### Gauges

| Metric | Meaning |
| --- | --- |
| `hydracache_hc2_connection_state` | exactly one current-state series with value `1` |
| `hydracache_hc2_connection_generation` | monotonic generation value; never a label |
| `hydracache_hc2_pending_invocations` | calls awaiting one terminal completion |
| `hydracache_hc2_subscriptions` | connection-owned subscriptions |
| `hydracache_hc2_reply_queue_frames` / `_bytes` | retained reply queue |
| `hydracache_hc2_event_queue_frames` / `_bytes` | retained event/gap queue |
| `hydracache_hc2_control_queue_frames` / `_bytes` | retained control queue |
| `hydracache_hc2_retry_slots` | retained retry records |
| `hydracache_hc2_reconnect_slots` | retained reconnect records |
| `hydracache_hc2_deadlines` | retained deadline registrations |
| `hydracache_hc2_topology_nodes` / `_bytes` | retained topology view |
| `hydracache_hc2_sessions` / `_bytes` | retained generation-fenced sessions |

After `SpikeConnection::disconnect`, every resource gauge above is zero. The
state series remains `1` with `state=closed`, and generation remains available
for incident correlation; those two are not resource-leak gauges.

### Counters

| Metric | Meaning |
| --- | --- |
| `hydracache_hc2_dispatched_frames_total` | validated frames reaching semantic dispatch |
| `hydracache_hc2_rejected_frames_total` | frames rejected before dispatch |
| `hydracache_hc2_resource_rejections_total` | bounded-resource admission failures |
| `hydracache_hc2_cancellations_total` | invocation cancellation wins |
| `hydracache_hc2_event_gaps_total` | event continuity losses requiring conservative repair |
| `hydracache_hc2_stale_generation_frames_total` | late or cross-generation frames rejected |
| `hydracache_hc2_reconnects_total` | strictly advancing connection generations |
| `hydracache_hc2_retries_scheduled_total` / `_exhausted_total` | retry lifecycle outcomes |
| `hydracache_hc2_repairs_total` | conservative cache repairs performed |
| `hydracache_hc2_session_heartbeats_total` / `_losses_total` | session liveness and terminal loss |
| `hydracache_hc2_trace_dropped_total` | bounded trace-ring overwrites |
| `hydracache_hc2_tls_rejections_total{reason}` | fixed TLS verification reasons |
| `hydracache_hc2_auth_rejections_total{reason}` | missing client certificate or authorization denial |
| `hydracache_hc2_deadline_outcomes_total{reason}` | `completed`, `timed_out`, or `cancelled` |
| `hydracache_hc2_drains_total{reason}` | `graceful`, `deadline_expired`, or `peer_reset` |

Counter observation is delta-based. A production integration must either feed
an outcome explicitly or let `observe_connection` derive it from
`ConnectionMetrics`; it must not count the same outcome through both paths.

## Trace contract

Trace records contain only sequence, event enum label, fixed labels, numeric
connection generation, and an optional closed reason label. Events are:

- `connection_created`, `connection_reconnect`, and `state_changed`;
- `invocation_cancelled`, `event_gap`, and `cache_repair`;
- `retry_scheduled` and `retry_exhausted`;
- `security_rejected` and `deadline_outcome`;
- `session_heartbeat` and `session_loss`;
- `connection_drain`.

Traces explain transitions; metrics remain the aggregation source. The ring is
diagnostic context, not an audit log and not a durable event stream.

## Integration sequence

For each logical authenticated client identity:

1. Generate a random 32-byte process salt before accepting traffic.
2. Construct the recorder with transport, first generation, raw tenant, salt,
   and a reviewed trace capacity. The recorder hashes the tenant immediately.
3. Call `observe_connection` after lifecycle/resource changes. A different
   transport or non-advancing generation fails closed.
4. Record retry, repair, security, deadline, session, and drain outcomes once
   in their owning service.
5. Export a snapshot through a non-blocking adapter. If the backend is down,
   drop or buffer outside the client-plane owner under a separate hard bound.
6. Observe the closed connection after disconnect and verify all resource
   gauges are zero.

## Operator diagnosis

| Symptom | Inspect | Interpretation / action |
| --- | --- | --- |
| connection never reaches `ready` | state plus TLS/auth reason counters | fix trust roots, hostname/time/EKU, client certificate, or authorization policy; do not enable fallback for security failures |
| reconnect loop | reconnects, generation, retry scheduled/exhausted | correlate with transport availability; generation must strictly increase |
| stale replies or events | stale-generation frames | identify delayed old transport owner; never relax generation fencing |
| near-cache uncertainty | gaps followed by repairs | repair is required after every gap; a gap without repair is a correctness incident |
| growing memory | pending and every queue/session/topology byte gauge | compare with configured limits; after close all resource gauges must reach zero |
| lost lock/session | session losses and heartbeats | treat loss as terminal until H11 defines and proves recovery semantics |
| shutdown stalls | drains by `deadline_expired` or `peer_reset` | inspect H20 drain traces and peer behavior; do not extend the deadline without evidence |
| missing trace context | trace dropped counter | increase capacity only within the 128-record ceiling or export more often |

Do not aggregate a salted tenant bucket across process restarts or interpret it
as a unique tenant. Do not alert on a single shared-CI timing sample.

## Evidence and reproduction

The contract tests prove metric naming, fixed cardinality, JSON privacy,
bounded trace retention, reconnect and failure increments, sink export, and
zero resource gauges after close:

```powershell
cargo test -p hydracache-client-plane-spike --test observability_contract --locked
cargo test -p hydracache-client-plane-spike --locked
cargo clippy -p hydracache-client-plane-spike --all-targets --locked -- -D warnings
cargo run --manifest-path crates/xtask/Cargo.toml -- doc-check
git diff --check
```

Passing H21 does not close H01, H03, or H11 and is not performance or
production-readiness evidence.
