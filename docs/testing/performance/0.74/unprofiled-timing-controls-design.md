# Unprofiled timing executable: instrumentation, not full D3

The baseline is clean `cd72e5e16f21b9f41757ca4597146840250bb5d8`, with 73
observer semantic checks per variant. The new contract was written before any
new timing implementation or numerical attempt. Four initial integration tests
first failed to compile with E0432 because the timing module did not exist.
This is test-first instrumentation, not a failed numerical candidate.

## Scope and identities

`timing-controls-074` is a separate executable with no counting allocator. It
refuses `--run` when built with `allocation-diagnostics`, including all-feature
builds. Tests still compile in that feature combination and check the refusal.
The separate allocation executable and all historical diagnostic packets remain
unchanged. No product code, allocator default or get-owner default is changed.

Eight independent boundaries support scalar GET/PUT: public embedded HydraCache,
direct ClientSurfaceState, HC1 HTTP, HC2 required-client-certificate TLS, RESP2/3
plaintext and RESP2/3 required-client-certificate TLS with AUTH. PUT is the same
fixed preloaded value; it is not a conflicting-write or batch atomicity claim.

Input JSON must supply every field, including the entire original scheduled
calendar, queue/concurrency/deadlines, SLO/histogram bounds, dataset/seed,
warmup and CPU-quality floors. Unknown fields, even inside the schedule, fail.
The declared dataset digest is checked before constructing a listener. Typed
canonical JSON hashes every workload field; whitespace/key-order formatting
changes do not change that digest. Source/features/binary identity are separate
from workload identity, so an off/on pair can have the same workload digest.

Before any fixture, the executable checks its own raw SHA256 against the supplied
binary seal. It reports both lock digests compiled into that binary. The supplied
source SHA is **not independently verified by the executable**: a future cohort
coordinator must bind it to clean Git source, the reviewed toolchain, build mode,
features, placement and binary hash. A syntactically valid source SHA is not that
proof. `--validate CONFIG` parses/validates without constructing any fixture.

## Calendar, warmup and CPU

The executable creates one current-thread runtime with client and server colocated.
It preserves the existing non-skipping scheduled driver, its 10,000-offer bound
and 15-second maximum original offer window. It does not raise a cap or reuse the
invalidated B0 closed-loop million-operation runner to obtain a longer CPU sample.

Warmup is explicitly 0–64 sequential calls of the same operation and corpus.
RESP warmup allocates descending setup IDs, never the measured 0..N offer IDs.
It is single-use, drains wire owners and cannot run after measured offers. The
warmup/measurement phase guard refuses overlap before reserving any IDs or
consuming either single-use flag; callers do not wait/rebase a busy phase. The
bounded history leaves room for setup, warmup, all offers and final read oracles
on one socket. Native/embedded warmup uses the same operation adapter. Final
corpus verification follows warmup and measured completion. This is not a claim
that every one of 128 clients received an equal warmup allocation.

The measured wall and whole-process CPU bracket `scheduled::run` or
`RespControl::run`: driver/task work, byte oracles, actual wire-owner drain and
observation/histogram projection are included. Preload, warmup, before/after
corpus verification, explicit shutdown, JSON serialization and executable hashing
are excluded. Goodput's denominator remains the driver's actual offer-to-drain
elapsed interval, not CPU's slightly wider projection interval. Both intervals
are reported; they must not be silently substituted.

Windows uses GetProcessTimes user+kernel; Linux uses CLOCK_PROCESS_CPUTIME_ID for
all threads. API units/getres are reported, **not asserted clock accuracy or CPU
update granularity**. CPU per offer and per successful operation are separate;
zero successes has no invented success denominator. A backwards clock fails.

The preregistered minimum usable CPU and measurement wall are each one second;
inputs may strengthen but not lower them. Too short, zero CPU, non-success,
undrained or histogram-overflow observations are not usable for CPU ratios.
Usability alone is never an A/A noise pass or candidate admission. No acceptance
floor in the existing proposal/D3 contracts changes.

The driver retains all original samples and completed RESP wire observations,
including censored operations and tombstones where projection succeeds. A fatal
observer/provider/deadline error may yield only a partial receipt; it must not be
called a complete trace. Measurement errors still trigger an explicit owner
shutdown attempt. A process deadline cannot certify completed shutdown and is
reported as failure. A future coordinator must own the child process tree,
enforce a hard deadline and retain stdout/stderr even in that failure case.

## Validation versus the next numerical cohort

The new tests are bounded semantic correctness fixtures, not A/A-A/B attempts.
They cover all eight boundaries with GET/PUT, warmup/original sequence separation,
value verification, owner shutdown, input/binary drift, CPU-quality formulas,
and executable feature refusal. They assert no real-clock speedup threshold.
The hosted semantic workflow also exercises the one-offer executable entry point
in truly unprofiled off/on builds, separately from all-feature refusal tests.
Explicit negative fixtures delete the expected corpus before warmup/verification
and retain an extra native owner: failed work still attempts shutdown, and an
unjoined owner cannot produce a successful shutdown receipt.

The next numerical cohort is **not sealed or executed** by this instrumentation
contract. Preparation still needs:

- Exact clean off/on source/lock/toolchain/binaries, final offered rate/work
  count/duration/warmup/placement and finite five-pair A/A and A/B order per cell,
  preregistered before the first measured child. Every invalid/noisy attempt is
  retained without retry or best-pair selection.
- CPU feasibility at the unchanged floors. The bounded 10k observer may be too
  short for native controls. A larger bounded/streaming driver would need its
  own preregistered instrumentation contract and tests, not a post-observation
  relaxation of quality floors or a B0 retry.
- Fixed private PKI input and fingerprint binding across fresh secure processes.
  This executable currently generates ephemeral material per process and retains
  only certificate fingerprints; it cannot assert secure A/A or A/B material
  parity. Private keys/tokens must not appear in evidence.
- Miss/error/slow-reader and size-transition cells, the entire RESP/security
  matrix, four independent native floors, supported allocator active/resident/
  retained lane and the inherited idle/delete/refill/post-close checks.

No new numerical result, allocation improvement, native nonregression or full-D3
pass follows from this implementation. W10 remains closed, C74 unresolved. Old
D3a/B0 packets and the qualification manifest are unchanged. Rented-host use,
same-box Redis and six-hour/24-hour qualification are not launched or authorized.

## Exact-source verification receipt

Clean source `9e79b012a6046db9946da1bb972750d83b966d5b` passed 88 serialized
observer checks per feature variant and two separate unprofiled CLI checks per
variant on Windows/Rust 1.94.0. Tool check/strict lint/fmt, 63 contract tests,
13 release-evidence tests, xtask check/strict lint, doc-check and 17 governance
checks passed. The local non-promotable contract check passed; `--require-ship`
returned expected exit 1 for incomplete candidate identity/qualification.

Hosted semantic run `37742392078` attempt 1 passed on the same exact source:
Ubuntu 24.04/Rust 1.94.0, 88 checks per variant and two independent unprofiled CLI
checks per variant, plus formatting/check/strict lint. The packet at
`local-runs/timing-instrumentation-9e79b012/` retains local/hosted logs and hashes.
An empty successful local fmt output has no generated Tee log; it is recorded
explicitly instead of inventing nonempty output. Source was checked clean before
and after the complete local sequence; only then was the evidence packet created.
These receipts do not certify full workspace verification, noise, a native floor,
allocator retention, full D3 or a new evidence/documentation HEAD.

The subsequent receipt regression guard verifies all 23 local/hosted raw file
hashes, sizes, actual test counts and closed admission flags. It raises the local
contract suite to 64 tests without relabeling the older exact-source 63-test log
or the hosted receipt as verification of this later evidence HEAD.
