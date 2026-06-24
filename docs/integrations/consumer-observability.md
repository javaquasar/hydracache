# Consumer Observability And Audit

HydraCache 0.49 W6 adds a consumer-facing observability surface for external,
multi-tenant clients. It is intentionally read-only and tenant-scoped: callers
see their own usage, quota, rate/fair-share state, and near-cache/subscription
health, but never another tenant's counters or namespace detail.

## Tenant Status

`GET /client/v1/status` validates the same client id and tenant binding used by
the data route. A caller cannot switch the tenant header to inspect a neighbor.
The response embeds `TenantStatus` schema version `1`.

The status payload contains:

- namespace bytes/entries and configured quotas for the caller tenant;
- request and fair-share counters for the modeled window;
- admission rejection count;
- active subscription count, subscription limit, and near-cache repair count.

Per-key detail is not exported in status. It belongs in audit snapshots only
after redaction.

## Audit

`AuditEvent` schema version `1` covers governance/security/admin events:

- auth/authz failures;
- quota/rate/fair-share rejections;
- residency refusals;
- region failover decisions;
- policy changes;
- optional advisory events.

Mandatory governance/security events fail closed when the configured sink is
unavailable. Optional advisory events may be dropped, but the drop is counted in
`AuditHealth`.

Audit payloads are redacted by default. Keys are either omitted or represented by
a stable hash/dimensions chosen by the operator policy. Values are never logged.

## Dashboards

Consumer dashboards and alerts live under
`docs/cluster/dashboards/consumer/`. The release gate checks that alert rules
reference registered metrics only.
