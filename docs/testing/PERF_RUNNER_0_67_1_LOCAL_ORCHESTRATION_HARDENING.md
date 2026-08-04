# HydraCache 0.67.1 local orchestration hardening

Status: implemented local preflight. This suite is deliberately **not**
qualification, bootstrap, or ship evidence.

## Purpose

The dedicated server should be rented only after deterministic orchestration
defects have already been found locally. The suite exercises the state machine,
service lifecycle, recovery, input rejection, offline behavior, and static
quality gates in disposable Docker resources. It cannot prove bare-metal
latency, IRQ isolation, calibration stability, NVMe topology, or an SLO.

Run from an exactly clean checkout on Windows with Docker Desktop:

```powershell
pwsh -File scripts/perf/local-orchestration-preflight.ps1 \
  -OutputDirectory C:\hydracache-local-preflight
```

The output directory must be outside the repository. The command writes one
JSON receipt with the exact source commit, pinned image identities, helper image
ID, result of every scenario, any terminal failure, and explicit
`bootstrap_eligible: false` / `ship_evidence_eligible: false` markers. Docker
objects have unique names derived from the source SHA and process ID. Exact
containers, network, and cache volumes are removed in `finally`; pass
`-KeepDockerState` only for local diagnosis.

## The six scenarios

| # | Scenario | Positive path | Fail-closed canaries |
|---:|---|---|---|
| 1 | Receipt state machine | Two distinct full-dress receipts → validated admission → serialized five-sample predecessor chain → sample set | Truncated JSON, unknown fields, provisioning identity drift, predecessor digest drift, and missing fifth sample |
| 2 | systemd lifecycle | Runner contract; online/offline; tuning `plan`, `apply`, `verify`, `freeze`, frozen check, and exact `restore` | Apply while runner is online, repeated apply, frozen service drift, repeated restore, and incomplete restore state |
| 3 | Fault injection | Provisioning receipt import and tmpfs prepare/verify/materialize | Wrong source commit, wrong mode, overwrite, truncated receipt, wrong tmpfs commit, wrong symlink target, and interrupted materialization staging |
| 4 | Offline replay | The warmed Rust test is rerun with Docker networking disabled and Cargo `--offline` | A fresh empty Cargo registry must fail while offline |
| 5 | Static analysis | ShellCheck error-level analysis for all performance shell scripts, full ShellCheck for the new harness helpers, actionlint for all workflows, and Python byte-compilation | Malformed telemetry JSON must be rejected |
| 6 | Cleanup/recovery | A pinned Redis fixture is started on a uniquely named network, probed, forcibly stopped, and removed | Post-cleanup inspect must prove that the exact container and network no longer exist |

## Container identities

The driver refuses floating tags. Current immutable inputs are:

- Ubuntu 24.04 helper base:
  `ubuntu@sha256:561618e2c15bf2397621dd04f96926663a3b5616c189cf7e38db7e82f5c538ea`;
- Rust 1.94 toolchain image:
  `rust@sha256:365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f`;
- Redis fixture:
  `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e`;
- actionlint 1.7.7 Linux amd64 archive SHA-256:
  `023070a287cd8cccd71515fedc843f1985bf96c436b7effaecce67290e7e0757`.

The helper image is built before the offline phase. Its content-addressed image
ID is recorded because Ubuntu package repositories are not a release-evidence
source and can change even when the base digest is fixed.

The actionlint invocation retains four narrow compatibility exclusions: the
platform's `background`/`cancel` step extensions (including the associated
missing-`run` diagnostic), the reviewed `hydracache-perf-v1` self-hosted label,
and existing SC2129 style-only findings. It does not suppress other syntax,
expression, runner, or embedded-shell findings. Legacy performance scripts are
checked at ShellCheck error severity; the two new orchestration helpers have no
ShellCheck warning or info baseline and are checked at full severity.

## Fixture boundary for systemd

The systemd scenario boots a privileged Ubuntu container and runs the real
`reference-host-tuning.sh`, `check-reference-host-freeze.sh`,
`runner-service.sh`, and `verify-runner-service.sh` entry points. A disposable
copy of the repository receives a small service profile and discovery shims for
kernel, core count, RAM, NVMe, and selected sysctls. Only in that copy, the
hardware isolation verifier and provisioned-host audit bodies are replaced by
fixture stubs. The production scripts and committed Ubuntu profile remain
read-only and unchanged.

This division is intentional:

- local Docker proves orchestration transitions, pre-state capture, drift
  rejection, and recovery;
- the full-dress gate on the rented Ubuntu 24.04 bare-metal host proves actual
  CPU isolation, cgroup quota, NVMe/IRQ behavior, calibration, workload, and
  evidence identity;
- neither result may be substituted for the other.

## Offline and mutation guarantees

The initial online Rust run populates uniquely named Cargo cache volumes. The
same state-machine test then runs with `--network none --offline`. An empty
registry is separately required to fail, proving that the offline success used
only the warmed immutable cache instead of silently reaching the network.
The Cargo target volume is mounted at `/cargo-target`, outside the read-only
`/repo` bind, so the harness also works from a pristine checkout where a host
`target` directory has never been created.

The source checkout is mounted read-only in every helper container. All tmpfs,
receipt, service, and repository mutations occur in a disposable copy or an
exactly named Docker object. The suite never reads or writes committed release
evidence paths in the host checkout.

## Interpretation

A green receipt means the orchestration is ready to be attempted on rented
hardware. It does not predict that a particular server will satisfy the
full-dress hardware gate. A red receipt is actionable and should block rental:
the JSON identifies the first failed scenario, while the console preserves the
underlying command output. No threshold, repetition count, zero-error rule,
spread limit, calibration rule, affinity rule, quota rule, privacy rule, shell
safety rule, or W4B/W5C identity check is weakened by this suite.
