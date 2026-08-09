# Performance evidence strategy

Status: accepted

Decision date: 2026-08-09

Applies to: HydraCache 0.67.1 and later performance work

## Purpose

HydraCache uses several performance test profiles because no single measurement
environment can answer every performance question honestly. This document is
the canonical guide for selecting a profile, interpreting its output, and
preventing evidence from a weaker profile from being promoted into a stronger
claim.

The central rule is:

> Each profile may reject a change within its own claim boundary, but passing a
> weaker profile never grants the claims reserved for a stronger profile.

This strategy adds fast feedback without weakening the qualification,
bootstrap, SLO, repetition, zero-error, spread, calibration, affinity, quota,
privacy, or fail-closed requirements of `reference-v1`.

## Problem statement

Wall-clock measurements on shared runners are affected by host contention,
scheduling, frequency control, virtualization, and other activity outside the
job. They are useful as tripwires but are not a sound basis for a portable
latency or capacity claim.

A qualified dedicated host can control substantially more of that environment,
including CPU affinity, IRQ placement, storage activity, quota, and measurement
window admission. It is therefore the correct foundation for authoritative
tail-latency and capacity evidence. That foundation is also expensive, depends
on suitable hardware being available, and cannot provide feedback on every pull
request.

HydraCache resolves this tension by separating evidence classes instead of
pretending that one class can substitute for another.

## Decision

HydraCache maintains four explicit performance profiles. Three are diagnostic
or regression-oriented and non-ship; only `reference-v1` can produce
authoritative release performance evidence.

| Profile | Environment | Primary question | Permitted conclusion |
| --- | --- | --- | --- |
| `ci-instruction-v1` | GitHub-hosted Linux; exact base and head in one job | Did a selected local operation require materially more machine instructions? | Paired relative work regression |
| `indicative-exploratory-v1` | Named same-host VM or server with recorded provenance | What behavior is useful for investigation and optimization planning on this host? | Same-host exploratory characterization |
| `memory-only-v1` | Guarded dedicated host with a verified diskless measurement window | Does a result change when ordinary measured-window storage activity is excluded? | Non-ship memory-only diagnostic behavior |
| `reference-v1` | Qualified dedicated bare metal | Does the exact candidate satisfy the release latency and capacity contract? | Authoritative latency and capacity evidence after all gates pass |

Results from different profiles must not be pooled. A result also remains bound
to its exact workload, commit, toolchain, environment, and contract identity.

## Profile selection

Use `ci-instruction-v1` when reviewing a pull request or continuously protecting
a small deterministic hot path. It is the default choice for cheap, frequent
work-regression feedback.

Use `indicative-exploratory-v1` when comparing implementations, forming an
optimization hypothesis, collecting resource telemetry, or producing same-host
numbers before authoritative hardware is available. The host must be named and
its provenance retained, but the result remains exploratory.

Use `memory-only-v1` when investigating whether storage traffic, page faults,
or NVMe interrupts explain a rejected or unstable measurement. It is a
diagnostic control, not an easier qualification profile.

Use `reference-v1` when qualifying a release candidate, acquiring bootstrap
samples, activating a performance baseline, or publishing a latency/capacity
claim.

If a question spans more than one class, run more than one profile and report
each verdict separately. Do not construct a synthetic stronger verdict from
several weaker results.

## Claim boundaries

### `ci-instruction-v1`

The profile compares Callgrind instruction count (`Ir`) for exact base and head
commits with one unchanged harness in one CI job. It may say that measured work
did or did not regress beyond its policy threshold.

It must not be interpreted as:

- latency or tail-latency evidence;
- operations-per-second throughput;
- capacity or sizing guidance;
- host qualification;
- bootstrap, frozen-candidate, or ship evidence.

Instruction count does not directly model cache locality, branch prediction,
allocator and memory-bandwidth behavior, scheduler contention, locking, system
calls, network or storage I/O, queueing, or coordinated omission. An unchanged
instruction count therefore does not imply unchanged wall-clock performance.

The detailed contract and reproduction procedure are in
[`PERF_CI_INSTRUCTION_PROFILE.md`](PERF_CI_INSTRUCTION_PROFILE.md). The
machine-readable claim boundary is
[`perf-policies/ci-instruction-v1.json`](perf-policies/ci-instruction-v1.json).

### `indicative-exploratory-v1`

This profile may report reproducible same-host observations and comparative
telemetry when the exact host, image, target, workload, ordering, and artifacts
are retained. Such results are useful for prioritizing engineering work.

They must not be presented as:

- a capacity floor or production sizing recommendation;
- a portable ranking across unrelated hosts;
- qualification or bootstrap evidence;
- proof that the reference profile is activated;
- release or ship evidence.

The detailed rules are in
[`PERF_INDICATIVE_0_67_1.md`](PERF_INDICATIVE_0_67_1.md), and the
machine-readable boundary is
[`perf-policies/indicative-exploratory-v1.json`](perf-policies/indicative-exploratory-v1.json).

### `memory-only-v1`

This profile requires the measured executable, working directory, and output
below a verified `tmpfs` runtime root. Its guarded window rejects storage,
cgroup I/O, NVMe IRQ, major-page-fault, affinity, containment, or command
failures defined by the profile.

It may isolate a storage-related source of noise. It must not be used as:

- a replacement for full-I/O-isolation qualification;
- qualification, bootstrap, release, or ship evidence;
- permission to pool diskless and `reference-v1` samples;
- proof of production behavior with persistent storage enabled.

The detailed contract is in
[`PERF_MEMORY_ONLY_HOST_PROFILE.md`](PERF_MEMORY_ONLY_HOST_PROFILE.md), and its
host profile is
[`perf-host-profiles/ubuntu-24.04-memory-only-v1.json`](perf-host-profiles/ubuntu-24.04-memory-only-v1.json).

### `reference-v1`

This is the authoritative release performance profile. A result is eligible
only when the exact host, candidate SHA, workload, runner, prebuild, receipt,
attestation, admission checks, repetitions, zero-error requirement, spread
limit, and evidence chain satisfy the frozen contract.

A qualification belongs to one exact hardware and service fingerprint. A code
change creates a new candidate SHA and requires the evidence prescribed for
that SHA. Failed or unstable samples do not count, and a successful result from
another profile cannot rescue them.

The operating contract is documented in
[`PERF_RUNNER_0_67_1.md`](PERF_RUNNER_0_67_1.md). The workload profile,
baseline, budget, and host contract are respectively:

- [`perf-profiles/reference-v1.toml`](perf-profiles/reference-v1.toml);
- [`perf-baselines/0.67/reference-v1.toml`](perf-baselines/0.67/reference-v1.toml);
- [`perf-budgets/0.67/reference-v1.toml`](perf-budgets/0.67/reference-v1.toml);
- [`perf-host-profiles/ubuntu-24.04-reference-v1.json`](perf-host-profiles/ubuntu-24.04-reference-v1.json).

## Evidence identity and retention

Every profile must preserve enough information to identify what actually ran.
At minimum, its applicable artifact or receipt records:

- exact source SHA, and exact base/head SHA for a paired comparison;
- profile and policy identity, including contract digest where defined;
- workload and benchmark identity;
- toolchain, harness, and measurement-tool versions;
- runner or host provenance appropriate to the profile;
- raw measurements and command status;
- machine-readable verdict and claim boundary;
- failure evidence when the profile is fail-closed.

Original artifacts must remain unchanged. Derived tables and prose summaries
must identify their source artifacts and must not broaden the source claim
boundary.

## Why the profiles remain separate

### CI instruction count does not replace wall-clock evidence

Callgrind provides a comparatively noise-resistant work metric and makes paired
CI feedback practical. It cannot establish p99 latency, capacity under an
open-loop arrival process, or system behavior under real contention and I/O.

### A named VM does not become a capacity floor

Naming the instance and retaining provenance makes an exploratory result easier
to reproduce and audit. It does not remove noisy neighbors, virtualization
effects, host drift, or differences between instance allocations. The result
therefore remains indicative unless it passes a separately defined stronger
contract.

### A diskless control does not replace production-path qualification

The memory-only window is valuable precisely because it changes an important
part of the environment. That makes it a strong diagnostic experiment and an
invalid substitute for the full reference workload.

### Dedicated-host evidence is intentionally expensive

Tail latency and capacity are emergent properties of the full system and host.
CPU placement, NUMA topology, IRQ routing, NVMe activity, frequency behavior,
background services, quotas, calibration, and repeated-sample stability all
matter. The `reference-v1` cost follows from the strength of the claim rather
than from a requirement that every performance test use bare metal.

## Alternatives considered

### Use only `reference-v1`

Rejected as the sole feedback mechanism. It provides the strongest evidence but
cannot run economically on every pull request and can be blocked by hardware
availability. It remains mandatory for its authoritative claim class.

### Treat shared-runner wall-clock results as capacity evidence

Rejected. Same-runner relative wall-clock tripwires can be useful, but host
variance prevents them from supporting a portable capacity floor.

### Compare a pull request with a historical rolling instruction baseline

Rejected for the blocking instruction profile. Toolchain and runner drift can
be confused with product changes. `ci-instruction-v1` instead runs exact base
and head commits in one job with one pinned harness.

### Replace qualification with Callgrind

Rejected because instruction work and system performance are different
quantities. This would silently weaken the release claim instead of solving the
measurement problem.

### Pool diagnostic and reference samples

Rejected. Different profiles intentionally change environment and admission
rules; pooling would erase evidence identity and invalidate interpretation.

## Adoption record

The initial `ci-instruction-v1` implementation was merged by
[PR #82](https://github.com/javaquasar/hydracache/pull/82) as merge commit
`984e4f91ddbce7413df40f8ce82cccb93cb1c402`.

The exact post-merge
[CI run 31303444599](https://github.com/javaquasar/hydracache/actions/runs/31303444599)
completed successfully. Its accepted artifact reported:

| Benchmark | Base `Ir` | Head `Ir` | Change |
| --- | ---: | ---: | ---: |
| `cache_get_hit` | 238145 | 238145 | 0.0% |
| `cache_get_miss` | 196617 | 196617 | 0.0% |

That result demonstrates the new CI profile and its artifact chain. It does not
qualify the merge commit for `memory-only-v1` or `reference-v1`, and it does not
close the remaining 0.67.1 dedicated-host campaign.

## Maintenance rules

Changes to a profile's permitted claims, admission rules, or promotion path
require an explicit review of this strategy and the corresponding
machine-readable contract. A documentation-only rewording must not broaden a
claim.

New workloads may extend coverage within a profile, but their benchmark
identity and setup/measured-work boundary must be documented. A new measurement
class should receive a new profile rather than overloading an existing verdict.

When presenting results, name the profile before quoting numbers. If the reader
cannot determine whether a result is regression, exploratory, diagnostic, or
authoritative evidence, the report is incomplete.
