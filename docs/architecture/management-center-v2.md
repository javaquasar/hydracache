# Management Center 2.0 architecture

This document is the normative read-side architecture for HydraCache 0.72. It inherits
`docs/RULES.md`; in particular, authority is Raft plus a monotonic epoch, missing evidence fails
loud, and every collection is bounded. The browser is an operator presentation client, never a
control-plane participant.

## Scope and non-goals

Management Center 2.0 exposes same-origin, read-only `/management/v1` observations for dashboard,
members, formation, partitions, placement, protocol clients, namespaces/caches, health,
persistence/recovery, operations, audit, consensus progress, and optional fixed-query history.
The browser contains no mutation client. It cannot drain, reshard, repair, compact, back up,
restore, edit configuration, execute code, or mutate cached data. It does not introduce a storage
engine, consensus authority, generic proxy, arbitrary PromQL, service worker, or persistent browser
database.

## Information architecture

Navigation is capability-driven and consists of Dashboard, Members, Partitions, Clients,
Namespaces, Caches, Health, Persistence, Operations, Metrics, and Audit. A capability that is
absent is presented as unavailable and is not polled. All pages retain the persistent cluster
selector and a global truth banner. Summary links carry filters into bounded read-only detail
views; the browser never recomputes health, placement, or cluster totals.

The complete field-group-to-producer map is machine readable in
`docs/testing/management-center/0.72/source-map.toml`. Bounds are frozen in
`docs/testing/management-center/0.72/bounds.toml`. A rendered route or field group without a
producer, privacy class, consistency boundary, fallback, bound, test owner, claim, and canary is a
release error.

## Authority and observation state machine

Every response carries schema version, observation sequence, optional authority epoch, capture
time, source, completeness, staleness policy, bounded warnings, and data. Wall time is only a
presentation/freshness input. It never decides membership, ordering, placement, completion, or
recovery.

The browser accepts observations by the following state machine:

```text
empty -> live|modeled|unavailable
live(sequence=N, epoch=E) -> live(sequence>N, epoch=E)
live(sequence=N, epoch=E) -> live(sequence=*, epoch>E) + clear incompatible history
any -> stale after stale_after_ms (presentation only)
any -> unavailable on explicit source failure; retained data is labelled previous, not current
older sequence or epoch -> rejected, never merged
unknown schema or enum -> incompatible/unknown, never optimistic fallback
```

`Live + fresh + complete` may use normal status styling. Partial is always labelled PARTIAL.
Stale is always labelled STALE and cannot produce green health. Modeled is neutral and never PASS.
Unavailable contains a stable reason and no value masquerading as current. Formation keeps
discovered, transport-reachable, authenticated, admitted, consensus role, caught-up, and serving
as independent dimensions. Commit and apply, accepted and completed, and live and recovered remain
distinct.

## Permissions and data classification

`management.read` authorizes cluster-safe and pseudonymous member diagnostics on the internal
admin listener. Existing write-admin identities imply read for compatibility but management
readers cannot invoke write routes. Tenant-scoped namespace/cache data is authorized server-side
before lookup, counting, pagination, or serialization. Hidden and nonexistent scoped objects share
the configured anti-enumeration response.

Allowed cluster-safe data includes bounded counts, roles, versions, health IDs, and resource
summaries. Member diagnostics use pseudonymous node IDs and an allowlisted configuration digest.
Tenant data contains only authorized names and aggregate usage. Keys, values, SQL, tags, raw
payloads, authentication tokens, certificate subjects, remote addresses, peer URLs, lock/session
tokens, file paths, raw errors, credentials, and actor identities are forbidden from management
DTOs, logs, URLs, DOM attributes, screenshots, and evidence.

## Trust boundaries and threat model

The browser is untrusted input. It may choose only documented filters, limits, opaque cursors, and
opaque trace IDs. It cannot supply a peer address, upstream URL, PromQL, credential, header set, or
cluster identity. The receiving daemon derives fan-out targets exclusively from authenticated
committed membership and uses the existing authenticated cluster channel.

The threat model covers:

- stored/reflected XSS: text-only DOM construction, typed JSON, restrictive CSP, `nosniff`, no
  inline script, no CDN, no raw HTML;
- confused-deputy fan-out and SSRF: committed-roster targets only; one configured Prometheus
  origin; scheme/host/port/address allowlist, DNS-rebinding check, no redirects;
- enumeration and cross-tenant disclosure: authorization before lookup/totals plus opaque bounded
  IDs and consistent 403/404 policy;
- stale replay and mixed epochs: monotonic epoch/sequence validation and immutable publication;
- response amplification and denial of service: page, candidate, string, byte, deadline,
  concurrency, retained-record, history, and DOM limits; cancellation releases owned resources;
- dependency compromise: exact lockfile pins, registry/integrity/license validation, SBOM, and no
  runtime third-party fetch;
- data-plane starvation: independently owned fail-fast management semaphore and bounded peer
  aggregation; management reads cannot propose Raft work.

Remote exposure is disabled by topology: the listener is internal/loopback. A trusted TLS or mTLS
reverse proxy must remove inbound HydraCache identity/capability headers and install verified
replacements.

## Request and aggregation sequences

```text
browser GET -> admin auth -> management.read/scope check -> read limiter
  -> query/limit/cursor/schema validation -> local immutable source snapshot
  -> bounded DTO/redaction -> byte ceiling -> typed JSON response
```

```text
browser GET -> receiving daemon -> committed roster at epoch E
  -> bounded authenticated peer RPC fan-out (concurrency 8, per-peer 500 ms)
  -> validate schema/node/generation/epoch/sequence/bytes
  -> deterministic deduplicate/reduce -> one immutable complete or explicit partial snapshot
  -> cancel late tasks and release buffers/permit -> response
```

```text
history query ID -> configured adapter -> resolve configured origin
  -> validate every resolved address -> fixed query template -> deadline/series/point/byte bounds
  -> discard labels and redact errors -> typed history envelope
```

No sequence contains a browser-originated POST, peer URL, placement decision, or recovery action.

## Endpoint budgets

The authoritative numeric values live in `bounds.toml`; this table records ownership and behavior.

| Surface | Principal frozen bounds | Over-budget behavior |
| --- | --- | --- |
| HTTP pages | 100 items, 256 KiB response, 32 warnings | clamp/reject as specified and mark truncated |
| Cursors | 256 encoded bytes, 30 s TTL, 1,024 records | invalid/expired cursor fails loud |
| Peer aggregation | 100 peers, concurrency 8, 500 ms peer/1,500 ms whole request | explicit partial/timeout; late work cancelled |
| Management reads | concurrency 16 | fail-fast 429; write/data paths use separate admission |
| Browser history | 24 series, 360 points/series, 4,320 points, 256 KiB | oldest-first eviction; epoch clears ring |
| Placement | 512 candidates, 64 selected, 16 reasons/candidate | stable truncation with preserved outcome |
| Health | 64 checks, 16 evidence references/check | stable truncation; missing evidence is UNKNOWN |
| Prometheus | 24 series, 1,000 points, 256 KiB, concurrency 2, 2 s | explicit timeout/partial/oversize; no fallback splice |
| Operations/audit | 128/256 current-generation records | oldest-first eviction with visible count |

Counts saturate and expose overflow/truncation; they never wrap. Strings and opaque identifiers
have explicit byte limits. All retained owners are registered in the 0.71 memory ownership model.

## Responsive interaction specification

Desktop uses persistent navigation and dense tables with detail drawers. Tablet and 200% zoom
collapse navigation while preserving every truth badge and accessible name. Narrow mobile renders
the same semantic facts as stacked labelled rows; horizontal scrolling may expose columns but
cannot hide source, freshness, completeness, UNKNOWN, truncation, or remediation. Tables and
drawers are keyboard operable, focus is visible and restored on close, status never relies on
color, landmarks/headings are semantic, reduced motion disables decorative transitions, and
forced-colors retains borders/focus/status text. Large result sets remain server-paginated and DOM
bounded.

## Baselines and evidence boundary

The pre-feature branch point is commit
`8d205fa302d81a07c19147cb4431e16390d256c3`. The machine-readable baseline declaration is
`docs/testing/management-center/0.72/baselines.toml`. It deliberately records the published
`v0.71.0` baseline as unavailable because neither that tag nor a retained artifact exists in the
repository/remote. No development branch may impersonate it.

Structural/unit/process evidence may be produced before candidate freeze. Numerical baseline,
mixed-binary, six-hour candidate, 24-hour ship-confirmation, Linux FD/RSS, full LLVM coverage, and
per-claim exact-candidate receipts are valid only for the declared immutable inputs. A missing,
dirty, stale, skipped, timed-out, rebuilt, or mixed-SHA receipt blocks promotion and is never
converted into success by this document.

