# Release 0.67.1 local Docker orchestration preflight

Date: 2026-08-04

Base main commit: `331d5a1c34f9db02568e48bb67453c708e0e7266`

Evaluated fix commit: `e250e163eb5d9a0f8807b2d12f698dfdebd647f9`

Result: **all containerizable orchestration checks passed after one host-preflight defect was fixed**.

This is a pre-rental engineering check. It is not machine qualification,
full-dress admission, bootstrap evidence, or release evidence. No local output
from this exercise may be promoted into the five-sample bootstrap chain.

## Purpose

The check exercises the parts of the 0.67.1 reference-runner procedure that do
not require physical ownership of CPUs, IRQs, storage, firmware, or systemd.
Its purpose is to catch shell portability, immutable identity, GitHub context,
container lifecycle, cgroup, affinity, toolchain, and fail-closed errors before
paid hardware is provisioned.

## Frozen local environment

| Item | Observed identity |
| --- | --- |
| Docker client/server | `28.3.2` / `28.3.2`, Linux `amd64` engine |
| Docker engine | Docker Desktop, 8 CPUs, 16,412,532,736 bytes RAM, cgroup v2 |
| Docker kernel | `6.6.87.2-microsoft-standard-WSL2` |
| Ubuntu userland | `ubuntu@sha256:561618e2c15bf2397621dd04f96926663a3b5616c189cf7e38db7e82f5c538ea` |
| Rust lane | `rust@sha256:365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f`, Rust/Cargo `1.94.0` |
| Redis server | `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e` |
| Redis load tool | checksum-pinned Redis `7.2.5` source archive; built binary SHA-256 `4dc1a5a2fec211dc47524b7249f662889f70352b4f85264543547c64037c4552` |

The repository was mounted read-only for Linux Rust checks. The real Git common
directory and isolated-worktree metadata were mounted separately, with
`GIT_DIR` and `GIT_WORK_TREE` pointing at those read-only mounts. Cargo registry
and build output used disposable Docker volumes outside the checkout.

## Result matrix

| Surface | Exercise | Expected result | Observed result |
| --- | --- | --- | --- |
| Shell parse | `bash -n` over all 19 `scripts/perf/*.sh` files in Ubuntu | Pass | Pass |
| Host audit | `audit-reference-host.sh --mode provisioned` in Docker/WSL | Reject virtualization | Rejected `wsl` |
| Host preparation | Exact Ubuntu 24.04 profile | Reject non-matching kernel | Rejected WSL2 `6.6.87.2` versus `^6\\.8\\.0-[0-9]+-generic$` |
| Host preparation canary | Temporary profile copy relaxing only the kernel regex | Reject non-bare-metal host | Rejected `wsl` |
| Host tuning | Parse and validate the committed profile | Pass profile validation, then reject local host | Passed profile validation; rejected kernel before mutation |
| Host tuning canary | Temporary profile with one duplicate mutable unit | Reject duplicate policy | Rejected `service policy contains duplicate mutable units` |
| Evidence tmpfs | Prepare, verify, materialize twice | Preserve marker and remain idempotent | Pass |
| Evidence tmpfs canary | Alter tmpfs `source-commit` | Reject identity drift | Rejected |
| Rootless Docker | Invoke as the wrong user | Reject | Rejected |
| Rootless Docker canary | Simulated rootful Unix Docker socket for `github-runner` | Reject rootful socket | Rejected |
| Runtime IRQ guard | Run against Docker/WSL topology | Reject ineligible topology | Rejected, exit 1 |
| cgroup/affinity | BusyBox with `--cpuset-cpus 1-4`, no CPU quota | Effective CPUs `1-4`, `cpu.max=max` | Pass |
| CPU quota canary | BusyBox with `--cpus 1` | Detect finite quota | Observed finite `100000` quota |
| Redis lifecycle | Pinned image, SET/GET round trip | Pass | Pass |
| Redis workload | Pinned `redis-benchmark 7.2.5`, 1,000 SET and GET requests | Both commands complete | Pass; supplemental smoke only |
| Redis isolation | Server process cpuset and cgroup quota | `1-4`, unlimited quota | `1-4`, `cpu.max=max 100000` |
| Trusted context | Qualification context on exact clean commit | Pass | Pass |
| Trusted context | Full-dress context on exact clean commit | Pass | Pass |
| Trusted context | Bootstrap context on exact clean commit | Pass | Pass |
| Trusted context canary | Replace `GITHUB_SHA` with 40 zeroes | Reject checkout identity | Rejected `qualification checkout does not match the dispatched commit` |
| Rust model/governance | Nine focused `xtask` test binaries, 67 tests total | Pass | 67/67 on Windows and 67/67 on Linux/Rust 1.94 |
| Static Rust checks | `cargo clippy -p xtask --all-targets --locked -- -D warnings` | Pass | Pass on Windows |
| Formatting | `cargo fmt --all -- --check` | Pass | Pass in Linux/Rust 1.94 |
| Diff hygiene | `git diff --check` | Pass | Pass |

The Redis request rate printed by this short smoke is deliberately omitted from
claims. A 1,000-request Docker Desktop run is only a protocol/lifecycle test and
is neither stable nor comparable reference performance evidence.

## Defect found before rental

`scripts/perf/reference-host-tuning.sh` used GNU-looking long option
`uniq --duplicates`. Ubuntu 24.04 GNU coreutils does not implement that option,
so a real host would have failed while validating the service policy, before
any meaningful bare-metal check.

The producer was changed to the POSIX/GNU-supported short form `uniq -d`. The
regression requires the portable pipeline and rejects reintroduction of the
unsupported spelling. A live Ubuntu negative canary also proved that duplicate
mutable units are still rejected; the safety check was not weakened.

## What Docker cannot qualify

The following remain mandatory on the rented Ubuntu 24.04 bare-metal host:

1. exact provider CPU model, physical-core count, SMT state, governor, clock
   stability, thermal behavior, and BIOS/firmware effects;
2. the exact `6.8.0-*-generic` boot plus `isolcpus`, `nohz_full`, `rcu_nocbs`,
   housekeeping, and online-CPU contract;
3. physical NVMe devices, MSI-X queue topology, effective IRQ affinity, and
   zero pre/post measurement IRQ deltas on CPUs `1-4`;
4. root-owned systemd service policy, protected SSH/time-sync services,
   allowlisted service changes, freeze/restore receipts, and drift detection;
5. the real `github-runner` user, linger/session state, rootless Docker daemon,
   runner service affinity, and absence of a rootful Docker socket;
6. calibration spread, workload stability, zero errors, SLOs, memory pressure,
   filesystem and network noise, and full measurement duration;
7. prebuild, attestation, provisioning, receipt, admission, and sample
   identities generated for the **new exact main commit** on that host;
8. two serialized identical full-dress receipts followed by five serialized,
   successful, same-fingerprint bootstrap samples.

## Rental decision

The code-side and container-side orchestration is ready to try on a rented
host once this portability fix is merged and the new exact main commit is used.
Docker materially reduces rental risk, but it cannot make an `EM-B220E-NVMe`
eligible or predict whether its immutable NVMe IRQ mapping will satisfy the
reference profile. The first paid action must therefore still be the bounded
host audit and qualification sequence; any red host gate ends the attempt
without launching full-dress or bootstrap workloads.
