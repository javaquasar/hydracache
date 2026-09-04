# HydraCache Management Center

HydraCache 0.57 adds a read-only Management Center for operating a running
daemon. It is served from the internal admin surface at `/console/` and reads the
same-origin endpoints `/cluster/overview` and `/metrics`. Granular per-cache
diagnostics are served on the same internal listener under
`/actuator/hydracache/*`.

The console is an observe-only surface. It does not call the authz-gated write
API. Operational actions still flow through the Kubernetes operator or the admin
write endpoints:

- `POST /admin/drain`
- `POST /admin/reshard`
- `POST /admin/backup` (request acceptance only; it does not create a durable
  backup artifact or restore point)

The separate `POST /admin/diagnostics/reset` route is not an operator data API. It is disabled by
default and configuration accepts `HYDRACACHE_DIAGNOSTIC_RESET_ENABLED=true` only for `role=local`
with a loopback admin listener. The route refuses active HC/2 connections, pending invocations,
subscriptions and sessions; after reset it returns aggregate before/after owner counts and fails
unless embedded and shared client data owners are logically zero. It preserves audit history and
monotonic fencing/version counters. This endpoint exists only to synchronize non-ship local memory
measurements and must never be enabled in member/client deployments or exposed on a network
interface.

## Trust Boundary

The admin listener defaults to `127.0.0.1:9091` and is intended for local
operators, Kubernetes probes, Prometheus, and port-forwarded console sessions.
Expose it as an internal service only. The public client surface uses
`/client/v1/*`; `/metrics`, `/cluster/overview`, `/actuator/hydracache/*`, and
`/console/` are not mounted there.

When the console is served from `/console/`, browser reads to `/cluster/overview`
and `/metrics` are same-origin and need no CORS policy. If an operator hosts the
bundle elsewhere, allow only read-only `GET` requests from a narrow origin list.
Never use browser CORS as the authorization boundary for admin writes.

## Source Semantics

Every cluster view carries `source`:

- `live` means the daemon has a real grid/control-plane status source.
- `modeled` means the daemon is exposing a local model because the real grid host
  is not attached for that role.

Console readers must treat missing or unknown `source` as `modeled`. Modeled
views are useful, but they are not evidence of a live cluster. In particular,
`/cluster/overview` renders modeled leader as `null`, even if older operator
status still has a local placeholder.

For `role = "member"`, the daemon hosts the networked grid stack: durable
`RaftMetadataRuntime`, chitchat discovery, and the cluster raft transport. It
emits `source:"live"` from the same raft-backed membership authority used by the
cache, so `/cluster/overview` can report a real elected leader and quorum from
reachable raft voters. `local` and `client` roles stay `modeled`.

The historical W6b follow-up is closed as
[`TD-0008`](technical-debt/TD-0008-networked-daemon-grid-hosting.md). The
`HYDRACACHE_GRID_INPROC=1` path remains only as an explicit test/development
fallback.

## `/cluster/overview`

`GET /cluster/overview` returns one point-in-time JSON document:

- `source`
- `members` with role, reachability, and generation
- `leader` with node id, term, and epoch, or `null` while electing/unknown
- `partitions` with `under_replicated` and effective `count`
- `consistency` with `configured_default` plus `op_counts_by_level`
- `backup_age_seconds`, supplied only by an attached observability source and
  `null` when no authoritative backup-age observation exists. An accepted
  `/admin/backup` response does not populate this field.
- `lifecycle` with reshard and upgrade phases

It is a view, not a linearizable read. Consumers should poll it and replace the
whole view. They should not infer hidden members, a current consistency level, or
backup freshness from absent fields.

`POST /admin/backup` currently validates authorization, readiness, and backup
configuration, then returns an explicit request-only response. In 0.66,
`outcome: "accepted"` is paired with `durable_artifact_created: false` and
`restore_point_available: false`; the console and operators must not translate
that response into backup completion.

For the networked member grid, `quorum_ok` is live voter-majority state. The
`lifecycle.reshard_phase` field remains an honest lifecycle label; it is `idle`
unless a real reshard runtime has supplied a non-idle phase.

## Actuator JSON

`/cluster/overview` is the aggregated console view. `/actuator/hydracache/*` is
the granular per-cache read-only actuator mounted on the same admin listener:

- `GET /actuator/hydracache/health`
- `GET /actuator/hydracache/caches`
- `GET /actuator/hydracache/caches/{name}/diagnostics`
- `GET /actuator/hydracache/caches/{name}/stats`
- `GET /actuator/hydracache/cluster/staging-health`
- `GET /actuator/hydracache/cluster/pilot-report`
- `GET /actuator/hydracache/correctness`

The standalone daemon registers its cache as `server`. Unknown cache names
return `404`. These routes are read-only and remain available during drain, like
`/metrics`.

## Prometheus

Scrape `/metrics` on the same admin listener:

```yaml
scrape_configs:
  - job_name: hydracache
    metrics_path: /metrics
    static_configs:
      - targets:
          - 127.0.0.1:9091
```

The metric catalog is registered in `docs/COMPAT.md`. Topology metrics carry a
bounded `source="live|modeled"` label, and the exporter emits cache, admission,
cluster-grid, topology, and backup-age series.

## Grafana Dashboard

Import
[`docs/observability/dashboards/hydracache-overview.json`](observability/dashboards/hydracache-overview.json)
into Grafana with Prometheus as the datasource. The dashboard covers hit ratio,
cache traffic, admission pressure, topology, replication/repair, and backup age.
`cargo xtask verify` includes a drift guard that parses every PromQL `expr` in
the dashboard and rejects references to metrics not emitted by
`registered_metric_names()`.

## Day-2 Observe Flow

1. Port-forward the admin listener, for example
   `kubectl port-forward statefulset/hydracache 9091:9091`.
2. Open `http://127.0.0.1:9091/console/`.
3. Check the `source` badge first. Treat `modeled` as a constrained local view.
   Treat member-role `live` as the daemon's raft-backed membership/status view;
   `leader:null` means an election is in progress or no leader is currently
   known.
4. Check degraded state. If the console cannot reach `/cluster/overview`, it must
   show an explicit unreachable state rather than a stale healthy view.
5. Correlate `/cluster/overview` lifecycle and partition data with `/metrics`
   counters before running any write action through the operator/admin API.
6. Use `/actuator/hydracache/caches/server/diagnostics` for per-cache stats when
   the aggregate overview is not detailed enough.

## Persistence, Operations, and Audit

The 0.72 console reads three additional admin-only resources:

- `GET /management/v1/persistence` separates configured backup support, observed backup age, and
  verified artifact evidence. Missing verification is displayed as unavailable.
- `GET /management/v1/operations?limit=...` returns a cursor-bound, newest-first journal for the
  current process generation. Accepted backup and reshard requests stay accepted until a real
  owner publishes later transitions.
- `GET /management/v1/audit?limit=...` returns redacted metadata for those transitions only. It is
  not presented as a complete security audit stream.

The browser contains no controls for drain, backup, reshard, repair, restore, delete, or retry and
never calls a write method. Tests assert that all management requests made by the console are GET,
that accepted is not completed, terminal records are immutable, cursor snapshots invalidate on a
new transition, bounded eviction is reported, recovery without a retained report is unknown, and
destination/identity/credential strings do not enter JSON or the DOM. Four W10 falsifiability
canaries deliberately violate those rules and must fail with their registered marker.

Local W10 verification:

```powershell
cargo test -p hydracache-server --test management_operations_072 --locked
cargo test -p hydracache-server --lib --locked management_operations
npm --prefix console test
```

## Management read security and accessibility

All `/management/v1/**` routes require a verified internal identity plus the dedicated
`management.read` capability. Tenant-scoped read alone is insufficient and cannot enumerate the
management surface. A write-admin identity implies management read, while a management reader
cannot invoke `/admin/*` mutations. The management reader has its own 16-permit fail-fast budget;
overload returns 429, cancellation releases the permit, and admin recovery admission remains
independent.

The administration listener remains the default trust boundary. Normal access is loopback or an
internal port-forward. Remote deployments must place a TLS (preferably mTLS) reverse proxy in
front, reject public access to the raw listener, strip every client-supplied `x-hydracache-*`
identity/capability header, and install only identity headers derived from authenticated proxy
state. Forwarded headers are not a substitute for that boundary.

Console assets are same-origin and locally bundled. Responses set a restrictive CSP, deny framing,
disable MIME sniffing and referrers, and disable caching. The browser issues GET only and inserts
diagnostic values with text nodes. Static checks reject raw-HTML/eval/write markers, external or
inline runtime dependencies and credential-like strings. The npm lock uses exact direct versions;
the supply-chain gate validates registry provenance, integrity and reviewed licenses and writes a
CycloneDX 1.5 SBOM to `target/management-center-0.72-sbom.cdx.json`.

Status is conveyed by labels as well as color. Semantic landmarks/tables, accessible names,
focus-visible navigation, focusable overflow regions, reduced motion and forced-color behavior are
tested at desktop, tablet/200%-zoom and narrow-mobile sizes. Automated axe checks run against the
fully populated production page; forced-colors and keyboard workflows have separate behavioral
oracles so OS color substitution cannot mask structural accessibility failures.

Local W11 verification:

```powershell
cargo test -p hydracache-server --test management_security_072 --locked
cargo test -p hydracache-server --lib --locked management_security
npm --prefix console run build
npm --prefix console test
npm --prefix console run supply-chain
npm --prefix console audit --audit-level=high
```

## Real-process and fault evidence

W12 composes existing storage/Raft source proofs with a dedicated management projection matrix at
`docs/testing/management-center/0.72/fault-matrix.toml`. Its 13 rows cover clean reopen, missing
derivatives, corruption, torn artifacts, ENOSPC, uncommitted WAL, authoritative snapshot failure,
commit/apply separation, stale-peer deletion, foreign identity, interrupted reconciliation,
concurrent aggregation and bounded pressure. Every source and projection reference is resolved to
an actual test function by the fast suite.

The process gate starts the production server in one- and three-daemon shapes. It verifies the
embedded console and every management section, then runs follower kill → explicit partial truth →
32-reader pressure → same-disk restart → recovered membership. The receipt binds the binary hash,
fixed seed, length-framed schedule/event digests, endpoint p95 and process resources. Retry attempts
are append-only and linked; failures cannot be overwritten. Long scheduled/candidate/ship receipts
remain exact-candidate environment evidence and cannot be replaced by this short local proof.

Local W12 verification:

```powershell
cargo test -p hydracache-server --test management_process_072 --locked
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test management_process_072 --locked one_daemon_production_management_surface_is_typed_and_honest -- --nocapture
cargo test -p hydracache-server --test management_process_072 --locked three_daemon_fault_recovery_retains_partial_truth_and_resource_bounds -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E
```

## Verification

Local W5 verification:

```powershell
npm --prefix console test
cargo test -p hydracache-server --locked deploy_smoke
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
cargo test -p hydracache-server --test grid_host multi_node_members_form_a_cluster_and_elect_one_leader --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue
cargo xtask verify
```

`cargo xtask verify` skips the console specs only when Node or npm is missing. If
Node is available, the console static check and Playwright specs are part of the
gate.
