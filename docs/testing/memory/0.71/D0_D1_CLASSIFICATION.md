# 0.71 D0 baseline and D1 classification

The accepted AX42 baseline contains all eight preregistered rows and 105/105 successful jobs. Its
immutable identity and checksums are preserved under
`docs/testing/perf-artifacts/0.71/ax42/d0-baseline/`. These are pre-change B1 measurements, not
candidate or ship evidence.

## Decisions

| Surface | Direct evidence | D1 classification | D2 disposition |
| --- | --- | --- | --- |
| Client-surface entry accounting | M1 owner snapshots contain entries and key/value bytes while the original report-level logical fields were zero | Live owner; report aggregation omitted the separate client-surface owner | Mandatory W1/W2a correction authorized; no capacity or eviction semantic change |
| Client-surface TTL | M3 snapshots retain 10,000 expired entries after the expiry checkpoint and 5,000 after refill/post-idle | Confirmed logical retention, not RSS or page cache | W4 safety fix authorized: bounded active expiry plus quota-ledger cleanup |
| Rewrite churn | Six and sixty rewrite cycles have effectively equal steady RSS (about 15.24 and 15.33 MiB) | Bounded plateau | No W5 optimization authorized |
| Namespace reset | Logical ownership returns to zero while RSS remains near the steady high-water mark | Allocator high-water after correct logical reclamation | No representation or allocator change authorized |
| Histories and queues | Idempotency, audit, mutation replay and conditional owners have explicit count bounds and overflow semantics in the source inventory | Bounded and correct | No W3 product change; add the required long plateau/release tests in point 4 |
| Connections | 1,000 HC/2 connections settle near 62 MiB RSS, about 51.5 MiB anonymous memory and 1,013 file descriptors | Resource-correlated connection cost | No W10 change authorized without paired proposal evidence |
| Persistence | Supported persistence settles near 30.29 MiB versus about 15.35 MiB with persistence off | Expected enabled-service/file-cache cost | No W11 change authorized without paired proposal evidence |

M1 also establishes expected payload scaling: at 250,000 entries, corrected logical owner bytes rise
from about 32.42 MiB for 64-byte values to 993.73 MiB for 4,096-byte values, while RSS rises from
about 113.50 MiB to 1,074.79 MiB. This confirms that direct client-surface owner snapshots are the
right source for logical attribution.

## Implemented foundation and safety fix

The report aggregator now combines independent embedded-cache and client-surface owners and never
replaces a real observed zero with a workload-derived value. The client surface now performs a
rate-limited, fixed-scan-budget expiry sweep on request paths and a forced sweep at quiescent diagnostic snapshots.
For isolated tenants, the same removal also releases the quota ledger entry. The legacy value-only
capacity and eviction semantics are unchanged.

Candidate acceptance remains open: these changes require point 4 release tests and later D3/D4
same-host comparison evidence before any numerical improvement or ship claim is made.
