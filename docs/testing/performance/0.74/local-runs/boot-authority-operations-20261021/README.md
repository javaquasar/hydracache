# Boot operations and short authority directory names

One narrow runtime change removes repeated short directory-name ownership in
the supervisor's shared authority engine. GNU 1.94 and MSRV 1.88 reproduce the
preregistered request reduction. No Boot runtime change, CPU speedup, native API
gain or server admission is claimed. The [plan](../../boot-authority-operations-plan-20261021.md)
freezes operation windows, vectors and the allocation target before profiles.

Instrumentation/baseline source is `a0cb7fe3ebf199d01e067adb389e3fb06557c152`;
candidate is `3d3a2180c6b54ea9156ae2bac609f36966359908`. The candidate changes
only the short-name predicate in `open_at` and adds one directory differential
test. It reuses the existing 24-byte stack buffer for directories as well as
files; names of at least 24 bytes retain CString. Every syscall, flag, errno,
fresh metadata/read check, returned owner and original deadline remains.

## Boot operation windows

The new test-only walk preserves the actual four fresh proc opens, filesystem
checks, bounded positional read, decode and descriptor lifetime/order. Its
open expression is token-compared with the runtime helper; values, identities,
FD flags, fresh errors and original Boot/Document trace counts agree. Selected
clocks preserve errno and returned owners. Actual and uncounted walks remain
separate controls; production Boot functions stay byte-identical.

Each mode has sixteen 1024-Boot batch observations, balanced over forward and
reverse rotations after 32 warmups. Values below are baseline medians in
microseconds per selected operation window, including clock/observer work.
They are not per-syscall kernel CPU or an additive partition. The empty
callback has a different body/value type and is never subtracted.

| Selected operation | Calls per Boot | GNU | MSRV |
| --- | ---: | ---: | ---: |
| Open | 4 | 1.391 | 1.293 |
| Filesystem check | 4 | 0.412 | 0.618 |
| Bounded read | 1 | 0.762 | 0.691 |
| Decode and compare | 1 | 0.333 | 0.305 |
| Close | 4 | 0.525 | 0.553 |
| Empty callback | 1 | 0.207 | 0.205 |

Open is a larger selected window than decode in these observations; decode
ranges overlap the empty control. This does not justify a decode optimization
or removing opens, filesystem checks or closes. All outliers remain: GNU's
read-window batch means reach 5.971 us and empty reaches 2.054 us. Actual Boot
batch means span 5.319–15.191 us GNU and 5.221–12.022 us MSRV. No calibrated
observer-free latency is inferred.

## Authority operations and the accepted request reduction

The unchanged authority probes separately measure actual full/directory/
inventory/record/marker/bind methods and selected leaf expressions. At depth
seven, baseline median phase CPU in microseconds is GNU/MSRV
105.262/105.358 for full inspect, 5.404/5.663 for directory,
11.908/13.532 for inventory, 30.969/31.889 for records and
31.464/31.869 for markers. Independently measured phases cannot be summed
into an exact CPU partition or substituted for original ActiveAuthority time.

For depth-seven records, selected median nanoseconds per leaf are:

| Selected expression | GNU | MSRV |
| --- | ---: | ---: |
| Held metadata | 835 | 818 |
| Fresh open | 1368 | 1345 |
| Before metadata | 785 | 722 |
| Parent metadata | 864 | 745 |
| Initial buffer | 284 | 304 |
| Bounded read | 939 | 981 |
| After metadata | 862 | 911 |
| Empty callback | 236 | 218 |

These include clock-window overhead and are not an allocator CPU partition.
The narrow candidate targets the demonstrable redundant CString request for
short directory components, rather than claiming buffers dominate CPU.

Both compilers match every one of three 1024-call request batches:

| Full synthetic authority inspect | Allocations before to after | Requested bytes before to after |
| --- | ---: | ---: |
| Depth one | 21 to 19 | 1616 to 1608 |
| Depth seven | 85 to 83 | 8122 to 8114 |

Reallocations stay zero. Isolated directory revalidation removes one four-byte
request; full inspect invokes it twice. The `sub` helper changes one four-byte
request to zero. All five other helper cases, cold/warm file reads, traced
site counts, payload/name hashes, policy epochs and fresh read/kernel counts
remain equal. Full authority retains 4/28 read requests at depths one/seven;
the six kernel/non-authority request controls remain unchanged.

This measures ordinary-user /tmp fixtures using the shared engine, not a full
root-owned fixed ancestry or production trust enrollment. Requested bytes are
allocator requests, not physical copied bytes, retained memory, RSS or PSS.
The runtime has no new retained field, cached guard, extra namespace/clock
observation, weakened check or changed descriptor lifetime.

## CPU limits and boot continuity

Each stage runs all twelve bounded diagnostic children in one ordinary-user
WSL parent. Source, images, boot, parent PID/start ticks, UID, configuration
hashes and fixture membership agree before/after each stage. Baseline boot is
`9c2bf668-c834-4585-9d09-36a3338b7ed1`; candidate boot is
`0ad3d48b-dd37-4bc8-a75f-56f1e002fa0a`. They are distinct diagnostic lifetimes,
not a continuous attempt, resumed process or same-boot CPU pair. No clock,
service or idle-policy change was made and no stage was retried.

Twenty directory helper signed CPU pairs (actual minus reference) retain both
signs in every compiler/stage. GNU baseline/candidate medians are
28.891/-26.391 ns per call, with ranges -7368.617–3190.166 and
-1933.949–935.921. MSRV medians are 8.576/2.955 ns with ranges
-8581.691–1817.479 and -3614.971–3308.613. Neither speedup nor CPU
nonregression is established. Even unmodified Boot control medians move
between stages; no subtraction or cross-stage CPU acceleration is accepted.
The confirmed result is the repeated deterministic request reduction only.

## Checks and retained evidence

Boot/authority targeted tests, affected all-target checks and strict
all-feature clippy pass on GNU and MSRV. The new directory test first passes
on unchanged runtime before implementation; it covers valid UTF-8, 23/24-byte
boundaries, 64 seeded directory names and four fixed components created only
under its private /tmp fixture. Existing NUL, symlink, FIFO, retained-parent,
UID/GID, budget, sticky refusal and publication/lifecycle controls remain.
Candidate authority selection passes 214 tests per compiler, versus 213 on
baseline; ignored manual tests are not relabeled as ordinary passes.

Thirteen reader/runner controls pass on Windows and Linux, including complete
matrices, wrong/missing/duplicate rows, numeric gates, changed read/request
targets, child refusal/postflight and exclusive output creation. Earlier
11/13-test authoring logs remain distinct; final controls are `reader-v3-*`.
Authoring refusals, including the failed first xtask preflight, remain with
their corrected separate prefixes. None is a substituted timing observation.
The broader historical source scan may overlap later read-only diagnostics;
desktop/shared-host CPU remains uncontrolled, not qualification evidence.

The predecessor source review is retained; all twenty-five earlier inventories
and member bytes remain verified independently. Their old HEAD-specific
readiness scripts remain frozen historical checks, not current-candidate gates.
The new exact source canary admits only the directory ownership predicate and
the registered opt-in Boot hook, leaving flags and unrelated drift visible.
After sealing, use `final-review.ps1 -Mode Review`, never capture or reseal.

All raw data are retained in [summary.json](summary.json) and the stage logs;
[baseline-facts.json](baseline-facts.json) was reviewed before implementation.
Original runtime windows remain 300/60/10, qualification digest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`,
and the earlier original ten-second refusals are not repaired or erased.
No original delivery/start, server run, installation, supervisor operation,
clock/service change, product workload, qualification or push occurs.
Native/server admission remains closed. Next investigate another separately
measured owner or repeat full-frame preparation under unchanged live checks;
this request result alone cannot establish a healthy original attempt.
