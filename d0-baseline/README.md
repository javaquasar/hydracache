# HydraCache 0.71 AX42 D0 memory baseline

This directory preserves the complete accepted D0 baseline for the 0.71 memory-footprint and
retention-efficiency program. It contains all eight preregistered M0-M7 rows and all 105 successful
measurement jobs.

## Provenance

- Measured source SHA: `906aa24cc22ad6b50b824120ed6364208484203a`
- Trusted workflow/controller SHA: `9c533d1de5a25b83bcb294e5b939064737b7d9fc`
- Host class: Hetzner AX42 dedicated bare metal
- Result: eight accepted campaigns; 105/105 jobs successful
- Acquisition dates: 2026-09-12 through 2026-09-13

Each row keeps two immutable server-side archives in `raw/` and the corresponding extracted
GitHub Actions artifact in `github/`. The compressed archives are Git LFS objects. JSON,
JSONL and logs remain ordinary Git objects for review and offline analysis.

The original M6 campaign contained 30 generated per-job mTLS private keys. They were not host or
GitHub credentials, but private keys are not published. The M6 `raw/` directory therefore contains
a content-preserving sanitized campaign archive and a `redaction-receipt.json` recording the exact
removed member paths and hashes, the original archive hash, and the published archive hash. The
original archive remains in the external local evidence store and is never added to Git.

| Row | Scenario | Campaign | Run | Jobs |
| --- | --- | --- | ---: | ---: |
| M0 | Cold process floor and instrumentation overhead | `hc071-d0-ax42-20260912-m0-07` | 34718869848 | 6 |
| M1 | Cardinality/payload shape grid | `hc071-d0-ax42-20260912-m1-01` | 34721063612 | 48 |
| M2 | Rewrite churn and reuse | `hc071-d0-ax42-20260913-m2-01` | 34742078073 | 6 |
| M3 | TTL cleanup and recovery | `hc071-d0-ax42-20260913-m3-01` | 34745085320 | 3 |
| M4 | Delete and namespace-reset reclamation | `hc071-d0-ax42-20260913-m4-01` | 34746314692 | 3 |
| M5 | Tag and index amplification | `hc071-d0-ax42-20260913-m5-01` | 34748154680 | 18 |
| M6 | HC/2 connection and slow-consumer floor | `hc071-d0-ax42-20260913-m6-01` | 34755585783 | 15 |
| M7 | Persistence and file/slab attribution | `hc071-d0-ax42-20260913-m7-01` | 34763224825 | 6 |

## Integrity

[`manifest.json`](manifest.json) records immutable identities, run IDs, expected job counts, archive
sizes and archive hashes. [`SHA256SUMS`](SHA256SUMS) covers every preserved evidence file. Verify a
fully materialized checkout from this directory with:

```powershell
./verify.ps1
```

The deliberately short row paths keep a fully materialized checkout portable on Windows. The
verifier rejects missing or unexpected evidence files, hash mismatches, unreadable gzip/tar
archives, malformed JSON/JSONL, mixed campaign identities, non-success receipts and incorrect job
counts.

## Claim boundary

These measurements describe the exact source, workflow, scenario definitions and admitted host
recorded above. They are suitable for D1 classification and future same-contract comparisons. They
must not be pooled silently with a different source, host fingerprint, instrumentation contract or
workload definition.

The repository is public. The evidence comes from a dedicated synthetic test server rather than a
production system. It contains detailed operational metadata needed for reproducibility; it must
not be treated as production or user data.
