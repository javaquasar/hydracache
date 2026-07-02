# HydraCache Management Center

HydraCache 0.57 adds a read-only Management Center for operating a running
daemon. It is served from the internal admin surface at `/console/` and reads the
same-origin endpoints `/cluster/overview` and `/metrics`.

The console is an observe-only surface. It does not call the authz-gated write
API. Operational actions still flow through the Kubernetes operator or the admin
write endpoints:

- `POST /admin/drain`
- `POST /admin/reshard`
- `POST /admin/backup`

## Trust Boundary

The admin listener defaults to `127.0.0.1:9091` and is intended for local
operators, Kubernetes probes, Prometheus, and port-forwarded console sessions.
Expose it as an internal service only. The public client surface uses
`/client/v1/*`; `/metrics`, `/cluster/overview`, and `/console/` are not mounted
there.

When the console is served from `/console/`, browser reads to `/cluster/overview`
and `/metrics` are same-origin and need no CORS policy. If an operator hosts the
bundle elsewhere, allow only read-only `GET` requests from a narrow origin list.
Never use browser CORS as the authorization boundary for admin writes.

## Source Semantics

Every cluster view carries `source`:

- `live` means the daemon has a real grid/control-plane status source.
- `modeled` means the daemon is exposing a local model because the real grid host
  is not attached yet.

Console readers must treat missing or unknown `source` as `modeled`. Modeled
views are useful, but they are not evidence of a live cluster. In particular,
`/cluster/overview` renders modeled leader as `null`, even if older operator
status still has a local placeholder.

G9 remains the named prerequisite for production-live views: member-role daemon
hosting of the real grid is required before a deployable server can consistently
emit `source:"live"`. Until then, the console must show `modeled` plainly.

## `/cluster/overview`

`GET /cluster/overview` returns one point-in-time JSON document:

- `source`
- `members` with role, reachability, and generation
- `leader` with node id, term, and epoch, or `null` while electing/unknown
- `partitions` with `under_replicated` and effective `count`
- `consistency` with `configured_default` plus `op_counts_by_level`
- `backup_age_seconds`, `null` when no snapshot exists
- `lifecycle` with reshard and upgrade phases

It is a view, not a linearizable read. Consumers should poll it and replace the
whole view. They should not infer hidden members, a current consistency level, or
backup freshness from absent fields.

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

## Day-2 Observe Flow

1. Port-forward the admin listener, for example
   `kubectl port-forward statefulset/hydracache 9091:9091`.
2. Open `http://127.0.0.1:9091/console/`.
3. Check the `source` badge first. Treat `modeled` as a constrained local view.
4. Check degraded state. If the console cannot reach `/cluster/overview`, it must
   show an explicit unreachable state rather than a stale healthy view.
5. Correlate `/cluster/overview` lifecycle and partition data with `/metrics`
   counters before running any write action through the operator/admin API.

## Verification

Local W5 verification:

```powershell
npm --prefix console test
cargo test -p hydracache-server --locked deploy_smoke
cargo xtask verify
```

`cargo xtask verify` skips the console specs only when Node or npm is missing. If
Node is available, the console static check and Playwright specs are part of the
gate.
