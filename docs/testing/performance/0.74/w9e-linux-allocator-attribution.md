# 0.74 W9e dedicated-Linux allocator attribution

Status: **complete owner attribution; no allocator candidate admitted**.

This run answers a narrow question: can allocator replacement or explicit purge be justified as a
single 0.74 product hypothesis? It is not a RESP/native end-to-end result and it is not release
qualification.

## Frozen method

Source `707abde46b23f24d03d1fe745e56576b964b99ad` ran on the serialized self-hosted Linux/x86_64
runner. The workflow built `system`, `mimalloc` and `jemalloc` binaries from the same source with
only the mutually exclusive allocator feature changed. Five counterbalanced no-purge repeats per
allocator plus five separate mimalloc purge repeats produced 20/20 valid attempts. Preflight and
postflight host, toolchain, source and lease identities were byte-identical; no attempt was silently
retried.

Every process executed the same deterministic trace: seed 74009, 16,384 keys, 4,096-byte payloads,
65,536 steady reads, then delete, refill and a 2-second idle. The profiler recorded cumulative CPU,
RSS/PSS/private pages, page faults, context switches, thread count, executable size and
provider-native allocator fields at every phase. RSS/PSS was never substituted for a missing
allocator-native concept. The exact contract is
[`w9e-linux-allocator-profile-contract.toml`](w9e-linux-allocator-profile-contract.toml), and the
repeat-level data used below is retained in
[`local-runs/w9e-linux-allocator-attribution-707abde4.json`](local-runs/w9e-linux-allocator-attribution-707abde4.json).

## Results

Percentages compare medians with the system allocator. Lower is better for every row.

| Metric | system median | mimalloc vs system | favorable repeats | jemalloc vs system | favorable repeats |
| --- | ---: | ---: | ---: | ---: | ---: |
| Whole trace elapsed, including fixed 2 s idle | 2.3817 s | -3.08% | 5/5 | -0.23% | 5/5 |
| Fill CPU/op | 6,595 ns | -51.26% | 5/5 | +3.33% | 0/5 |
| Steady-read CPU/op | 2,887 ns | -0.54% | 4/5 | -2.46% | 5/5 |
| Delete CPU/op | 1,530 ns | -41.54% | 5/5 | -29.31% | 5/5 |
| Refill CPU/op | 3,306 ns | -8.55% | 5/5 | +5.10% | 0/5 |
| Post-idle PSS | 144,759,808 B | +14.24% | 0/5 | +1.69% | 0/5 |
| Post-idle RSS / peak RSS | 146,870,272 B | +14.04% | 0/5 | +1.66% | 0/5 |
| Executable size | 2,159,688 B | +9.92% | 0/5 | +34.12% | 0/5 |

The CPU reductions under mimalloc are real owner observations, especially for fill, delete and
refill. They are not an admissible product win. Total elapsed improved only 3.08%, below the frozen
5% materiality floor for that aggregate, while PSS/RSS and binary size all regressed beyond the 5%
guard. The isolated CPU owners themselves clear the owner floor, but the unchanged regression
guard still forbids admission. This is a cost transfer from CPU-heavy churn into memory and
distribution size.

Jemalloc reduced delete CPU consistently but did not improve total elapsed materially. Refill CPU
regressed 5.10% and executable size grew 34.12%, so it also fails the frozen guard.

Provider-native values remain intentionally non-equivalent. At post-idle, glibc reported roughly
142.94 MB `uordblks` and about 0.66 MB free heap bytes; jemalloc reported about 141.32 MB allocated,
141.60 MB active, 147.6 MB resident and 10.0 MB retained; mimalloc reported about 168.0 MB committed
and 1.076 GB virtually reserved but no trustworthy live-allocation or allocator-owned resident
counter. Those values preserve provider semantics and are not used as direct cross-provider
percentages.

Neither glibc `mallinfo2` nor the reviewed mimalloc/jemalloc APIs expose a comparable scalar
thread-cache count; each receipt records that field as unavailable with a provider-specific reason.
That limitation would block acceptance, but it cannot rescue candidates that already fail the
independent PSS, binary-size or CPU guards. The evidence is therefore complete enough for rejection,
not for allocator admission.

Explicit mimalloc purge was also too small to become a separate policy candidate. Every repeat
performed three provider purge calls and reported 655,360 purged bytes, but process PSS/RSS fell by
only 339,968 bytes (about 0.22% of pre-purge PSS). The second refill completed in every attempt.

## Decision

No allocator replacement and no purge policy is admitted. The system allocator remains the 0.74
default, and there is no reason to spend a 6-hour or 24-hour qualification slot on either candidate.
If a later accepted pipeline implementation changes the allocation/churn shape materially, W9e may
be rerun against that exact candidate; this evidence cannot be reinterpreted as its result.

The successful workflow run is
[GitHub Actions run 37300775917](https://github.com/javaquasar/hydracache/actions/runs/37300775917).
Its sealed packet binds 20 attempts, the exact three binaries, contract/tool/lock digests and the
unchanged host identity. The packet is explicitly non-promotable and contains no candidate data.
