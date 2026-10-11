# Original worker task observations

The next W11 prerequisite observes every task of the already retained original
process. Leader-only status and namespace checks cannot establish the same
assertions for another thread. This local slice adds a separate bounded,
read-only guard; it does not change the existing leader or signed APIs.

## Preregistered scope

Baseline: `5efc0cfbe3e1defbefffdfa3dd0ddb012db04a36`, tree
`49bed06faf8258571d89309a7ccb10259d79a0ee`. The frozen qualification manifest
remains SHA-256 `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The [server checklist](production-readiness-and-server-launch-20261011.md)
continues to require original signed composition and real authority enrollment.

The public opener accepts only the original `ProcessRead` and explicit nonroot
numeric credential assertions. It selects no PID, path, namespace or supplied
task set. It retains `/proc/<original>/task`, a sorted inventory of at most 128
members, each original task directory and its stat/status/user/mount objects.
Enumeration uses bounded descriptor-relative Linux directory reads with no
retry. A new enumeration descriptor resets the directory offset without sharing
an offset with another reader. At most 16 calls and 65536 enumerated bytes are
allowed, including dot entries; overflow refuses rather than truncates.

Opening and later observations bracket inventory and two complete task passes
with original process generation checks. Every task keeps its original TID,
start ticks, directory/stat/status identities and bounding capability set. Status
requires the original process TGID, all four UID/GID slots, exact sorted groups,
zero active capabilities and NoNewPrivs. Each user and mount namespace must
remain the original observer-thread namespace, with nsfs type checks. The
observer TID is fixed and the guard is neither Send nor Sync.

Task additions/removals, TID reuse, namespace/credential drift, descriptor
substitution, uncertain reads or any budget failure permanently refuse the new
guard. Healthy drop leaves the original process readable. This first primitive
does not latch a signed reader: that separately tested exclusive composition is
the next slice. It never adopts a refreshed task set.

## Local checks

Tests first require the absent API. Real owned unprivileged helpers cover one
task, multiple tasks, membership churn, process exit and independent readers.
Private seams cover retained-object replacement, credential fields, namespace
type/mapping, generation drift, malformed directory records, exact first-error
order and sticky refusal. Seeded mutation results are model evidence, not live
kernel race counts. Linux test/check/strict lint and Windows portable checks
remain separate, with Rust 1.88 MSRV, documentation and contract checks.

## Proof boundaries

This is sequential point-in-time observation, not an atomic task snapshot or
continuous confinement. Tasks that appear and disappear entirely between scans
may be missed. Same observer namespace does not attest initial-host authority,
mount contents or proc file-opener credentials. Numeric assertions do not enroll
the fixed account or an issuer. Durable epochs/refusal, original signed binding,
root-owned output preparation, authenticated start, watchdog/writer revocation
and positive dedicated-host rehearsal remain required. No helper/IPC mode,
production route, server mutation, workload, performance claim or release
admission is enabled.

## Reviewed local result

Implementation: `8c8e55aa68115f7633b9ecfe12367384b37474d3`, tree
`283e6a48abbeba57a8f4c0e7836a10212610fcc8`; preregistration:
`a2c91dc0de343262e7accddb4928db1fbf205989`. The
[packet](local-runs/diagnostic-task-set-20261011/README.md) retains 30 exact raw
captures, including initial failures. Linux scoped checks pass 382 with one
existing ignored, eleven new unit tests repeat three times, and the external
API test passes. Windows full supervisor package passes 189 portable cases with
zero new Linux cases. Root contracts/evidence pass 100/13 and final governance 23.
Strict lint, all-target checks, both-platform Rust 1.88 compilation, Linux
rustdoc, formatting, documentation and local performance contracts pass.

The initial removal-error expectation fails in a parallel suite without recording
the actual variant. Its exact cause is unestablished; the fixture now acknowledges
procfs task absence explicitly. The guard still never retries. Missing cfg-target
registration is fixed under the existing Linux fast lane, without changing its
workflow or timeout. The first capture loop overwrites its repeat filename at
the PowerShell boundary; the preserved final capture is supplemental and three
fresh explicitly named captures supply the repetition result. None of these
negative results becomes a performance or production pass. Frozen qualification
inputs remain unchanged, and ship admission still refuses.
