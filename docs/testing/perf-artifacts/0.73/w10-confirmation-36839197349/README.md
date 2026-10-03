# HydraCache 0.73 W10 confirmation archive

This append-only archive preserves the evidence used to admit the frozen
HydraCache 0.73 candidate after GitHub Actions run
[`36839197349`](https://github.com/javaquasar/hydracache/actions/runs/36839197349).
It is the durable W11 copy; GitHub Actions retention is only a staging source.

## Decision

The split 24-hour confirmation passed. I73 and C73 each completed
1,036,800,000 operations with 1,442 checkpoints, zero errors, zero timeouts,
zero rejections, an exact final reconciliation, and the same admitted host and
lease. The negative canary was rejected for the intended missing post-idle
reconciled checkpoint. The I73 continuation manifest was byte-identical in the
stage and final provider archives.

All frozen comparison guards passed:

| Guard | I73 | C73 | Relative change | Budget |
| --- | ---: | ---: | ---: | ---: |
| Goodput, operations/s | 12000.043024603427 | 12000.043023226612 | -0.0000000115% | no more than 2% regression |
| CPU seconds/operation | 0.000025959230324 | 0.000026026668596 | +0.2598% | no more than 3% regression |
| p99, microseconds | 7311 | 7287 | -0.3283% | no more than 3% regression |
| RSS slope 95% upper bound, bytes/s | 30.144750809500 | 20.318058662033 | -32.60% | C73 must not exceed I73 |
| Anonymous-PSS slope 95% upper bound, bytes/s | 30.144750809500 | 20.318058662033 | -32.60% | C73 must not exceed I73 |

The result admits final C73 and closes W10. It does not turn the single long
pair into a portable performance, capacity, allocation, Redis, or Hazelcast
claim; those boundaries remain unchanged.

## Layout

- `original-artifacts/` contains all eight provider ZIPs byte-for-byte: every
  rejected, failed, interrupted, qualification, continuation, and accepted
  attempt named by the release archive contract.
- `accepted/final-packet/` is the traversal-checked extraction of the accepted
  final provider ZIP. It contains both roles, all minute checkpoints, the four
  calibrations, canary evidence, the continuation manifest, and the sealed
  campaign receipt.
- `attempt-records/` mirrors the reviewed decision receipts for the earlier
  attempts. Nothing is spliced into the accepted pair.
- `verification/verification-receipt.toml` records the independent download,
  nested-hash, continuation, identity, workload, duration, estimator,
  threshold, secret-marker, and guard checks.
- `artifact-manifest.tsv` binds provider identities to exact archive bytes.
- `SHA256SUMS` binds every archived file except `SHA256SUMS` itself.

The local `.gitattributes` disables text conversion for the whole archive so a
fresh Windows checkout retains the provider bytes and verifies the same
checksums. The follow-up verifier correction reads runner and scenario identity
from the frozen Git commit rather than from platform-converted worktree bytes.

The branch `evidence/0.73/w10-confirmation` must never be force-pushed or
rewritten. Any correction is a new append-only commit identifying what it
supersedes.
