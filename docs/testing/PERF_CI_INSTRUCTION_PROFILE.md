# CI instruction-count performance profile

`ci-instruction-v1` is HydraCache's GitHub-hosted, deterministic work-regression
tripwire. It runs the same benchmark harness against the exact pull-request base
and head commits in one job and compares Valgrind Callgrind instruction counts.
It complements wall-clock testing; it does not replace `reference-v1` and it
does not replace `memory-only-v1`.

## Claim boundary

This profile answers one narrow question: did a selected local operation require
materially more machine instructions than it required at the base commit? Its
result is relative work-regression evidence. It is not latency, not throughput,
not capacity, not sizing guidance, and not release qualification or bootstrap
evidence. Results from different runners or unrelated runs must not be pooled.

The machine-readable contract is
[`perf-policies/ci-instruction-v1.json`](perf-policies/ci-instruction-v1.json).
The contract permits only `relative_work_regression`; every release, latency,
throughput, and capacity claim is explicitly false.

## Workloads and threshold

The initial harness measures 64 serialized operations per Callgrind sample:

| Benchmark | Setup outside measured function | Measured work |
| --- | --- | --- |
| `cache_get_hit` | Build a local cache and seed one `u64` value | 64 typed cache hits |
| `cache_get_miss` | Build an empty local cache | 64 typed cache misses |

Only Callgrind `Ir` (retired/instrumented instruction count) blocks the job. A
head benchmark is rejected when it exceeds its paired base result by more than
5%. Cache and branch simulation are intentionally not gates: Valgrind's cache
model is useful for investigation but is not a model of the runner's modern CPU.
ASLR remains enabled because GitHub-hosted/container security commonly forbids
the `setarch -R` personality change. The setting is explicit and identical for
base and head; address placement is not interpreted as wall-clock evidence.

The harness and dependency lock live under
`scripts/perf/ci-instruction-harness/`. Gungraun is pinned to 0.19.4. The
instrumentation harness is pinned to Rust 1.94.0 independently of HydraCache's
product MSRV (which remains tested by its existing lane), and `gungraun-runner` is installed separately at the
same exact 0.19.4 version. The artifact records the effective Rust, Cargo,
Gungraun runner, Valgrind, kernel, base SHA, and head SHA.

## Pairing and fail-closed behavior

`run-ci-instruction-pair.sh` exports both commits with `git archive`, points one
unchanged harness first at base and then at head, uses one Cargo target directory,
saves base as a named Gungraun baseline, and runs head against that baseline.
Benchmarks are serialized. There is no rolling baseline, downloaded historical
baseline, or cross-runner comparison. Pull requests explicitly check out the
pull-request head SHA instead of GitHub's synthetic merge commit; both exact
commit identities are embedded in `report.json`.

The job fails when a commit cannot be resolved, Valgrind is absent, the harness
cannot build, benchmark identities differ, Gungraun detects a regression, or the
machine-readable report cannot be produced. Raw Callgrind files, Gungraun JSON,
stderr logs, tool receipts, and the stable `report.json` envelope are uploaded
even on failure.

## Local Linux/Docker reproduction

The runner requires Linux and Valgrind. From a full Git checkout:

```bash
sudo apt-get update
sudo apt-get install --yes valgrind
cargo install --locked gungraun-runner --version 0.19.4
scripts/perf/run-ci-instruction-pair.sh \
  --base origin/main \
  --head HEAD \
  --output target/test-evidence/ci-instruction-v1
```

On Windows, use the project's Linux Docker validation command documented in the
pull request or run the script inside an Ubuntu 24.04 container with the Git
history mounted read-only and a writable output/target volume.

## Relationship to the other profiles

| Profile | Environment | Valid conclusion |
| --- | --- | --- |
| `ci-instruction-v1` | GitHub-hosted Linux, paired commits | Relative deterministic work regression |
| `memory-only-v1` | Guarded dedicated host, diskless diagnostic window | Non-ship memory-only diagnostic behavior |
| `reference-v1` | Qualified dedicated bare metal | Authoritative latency/capacity evidence after bootstrap and activation |

Passing this CI profile never makes a commit qualified for either dedicated-host
profile. Failing it is a code-work regression signal to investigate; it does not
invalidate previously retained dedicated-host evidence for another exact SHA.
