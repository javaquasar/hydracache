# Original preparation clock facts and cost attribution

Six fresh local preparations completed: three with GNU Rust 1.94 and three with
MSRV 1.88. No causal time refusal occurred in this series. This is not a fix or
an explanation of the earlier GNU `Causal(Time)` refusal, whose original tuple
was not captured. The closed [negative retirement packet](../root-retirement-20261019/README.md)
and its separate positive functional gate are unchanged.

## Scope and identities

Selected source: `7c523942f0ecba1011a09d63ec1e2531923b7538`.
Recipe SHA256: `3b5a503ec8ca968df00001ffe3a55a1677847847667983951b319238ae1cf8eb`.
GNU image: `60a114158989e84db95a5a77ecc1b5c37031ed54f5d45d0b784ef9c05733b649`.
MSRV image: `0a5c681925cce90f12fed9c028702401d6c5baa976ade957501b62a325cf0c5e`.

Both selected cohorts use the same unchanged signed-bundle fixture data and
nonroot UID1000. Bundle hashes and executable hashes match before and after.
The copied bundle is never executed or installed. Root controls used for normal
owner retirement are simulated; no original privileged root constructor,
manager send, held-unit start, all-writer cessation, completion or unlock is
proved by this attribution. `functional_gate_passed=false` is intentional.

`gnu194/` is a preliminary build at `42fcb5e5`, not a measurement cohort.
`gnu188/` retains the preliminary MSRV formatting-lint refusal. A subsequent
inline-format correction is test-only. Only `gnu194-confirmed/` and
`gnu188-confirmed/` supply the six preparation measurements.

`source-full.log` retains the first full source suite: 292 passed and three
independent historical adapters refused the new test hooks. The correction at
`2b984ff2` routes those comparisons through the same exact normalizer and tests
deletion, duplication and body mutation for the adapter additions themselves.
The three regressions and the three new focused controls pass. This correction
changes xtask source contracts only; Linux source and both measured executable
images are unchanged. `source-full-confirmed.log` is the separate final suite,
not an overwrite or waiver of the earlier failure.

## Clock observations

The test copies the already sampled issued/now BOOTTIME values and both boot
identities at the original `window` and `final_clocks` comparisons. It performs
no additional kernel clock read. First/last observations and first refusal
are bounded diagnostic data, never authority or a cached validation result.
The original comparison still decides independently, including refusal exactly
at ten seconds. Unit controls cover zero issuance, backward BOOTTIME, boot
mismatch, exact expiry, healthy observations, nested capture, unwind and threads.

Both local preflights settled for 65 seconds with zero backward REALTIME steps,
then passed a separate 65-second strict REALTIME probe before creating any
attempt. The exact previously authorized local timesyncd service is restored
and verified active after each cohort. No clock was manually changed.
The six attempts observed the same boot
`4a7ec037-91ad-47d0-9999-a4aa2357e588`; no causal observation recorded rollback,
boot mismatch or expiry. This says nothing about the unrecorded historical tuple.

Last observed challenge ages after enrollment are 7.732039312, 8.037277738 and
8.234251158 seconds on GNU 1.94; 7.571231067, 7.971775545 and 8.258887769 seconds
on MSRV. These ages start at the original challenge issuance, not test entry.
The original 300/60/10 second offsets, origins, predicates and 46 effect brackets
are preserved. Finishing a test in more than ten seconds is therefore not by
itself evidence that its later-issued challenge expired.

## Cost of unchanged preparation

Ranges below are the three selected repetitions per compiler. Counts are
identical in all six fresh attempts; durations are instrumented wall time,
not CPU time, allocations or product throughput.

| Phase | GNU 1.94 seconds | MSRV 1.88 seconds | Authentication and signature calls each |
| --- | --- | --- | --- |
| Retain through consent acceptance | 4.376–4.556 | 4.347–4.922 | 640 |
| Starting and output preparation | 5.903–6.624 | 5.851–6.460 | 782 |
| Enrollment preparation | 0.281–0.400 | 0.272–0.392 | 34 |

GNU output preparation spends 79–91 ms inside authentication, including
59–63 ms of signature verification. Authentication is about 1.2–1.4 percent
of the phase, not its dominant cost. Existing kernel spans cover 3.905–4.333
seconds, about 65–66 percent of output wall time. In each output preparation,
the unchanged path performs 4,232 worker revalidations, 8,464 kernel-context
checks, 153,088 Boot spans and 306,176 Namespace spans. Counts are observations
of this fixture schedule, not permission to omit any check.

Boot and Namespace are the largest individual kernel categories outside their
instrumented direct children. The broader worker live walk also includes
policy, source, authority and local-context work. Authentication, live and
kernel spans overlap: they cannot be added as independent costs. The sum of
kernel outside-children spans covers a forest of existing nested spans; it is
wall attribution, not exclusive CPU or a syscall census. Instrumentation itself
has overhead and can affect code generation. The apparent improvement over
older failing runs is not attributed to this diagnostic patch or a compiler.

`summary.json` is derived from raw JSON lines by `summarize.ps1`. Preserve the
raw logs for individual issued/now/boot tuples and full span counts. A future
hypothesis should target measured leaf work without reducing live observations,
changing check order or extending the original deadlines. There is no product
optimization candidate or speedup claim in this packet.

## Reproduction and acceptance boundary

At the selected source, invoke `run.ps1` with `-Compiler 1.94.0` or `1.88.0`,
an unused `-Label`, first `-Stage build`, then `-Stage frame`. Build both images
before measuring; frame cohorts run sequentially without concurrent compilation
or source-contract tests. Every repetition constructs a fresh original chain;
no refused attempt is retried or renewed. Existing labels cannot be overwritten.
The selected runner assumes the retained local bundle path and the prior local
WSL time-service authorization. It does not authorize another environment.

`verify.ps1 -SelfTest` checks raw inventory, source/image/recipe and bundle
identity, nonzero test selection, scope, clock restoration and eight in-memory
mutation controls. `seal.ps1` first verifies without writing, then creates the
inventory once. Closed packets must never be resealed.

Original GNU retirement-frame acceptance still requires its own dynamic
receipt; this attribution deliberately does not substitute preparation success
for that callback proof. Protected host enrollment, the combined privileged
original constructor, confinement/cessation of every writer, durable lifecycle
and reconnect rehearsal remain separate requirements. Server operations,
supervisor maintenance, product workload and qualification remain unauthorized.

## Completed local checks

The final full source suite passes 295 controls in 308.96 seconds at
`2b984ff2a3e74ec3855af39af065bd996790a2a9`; four threads are used only for
read-only source contracts, after all six timed preparations finish. Both GNU
compilers pass strict all-target/all-feature clippy and two new causal-facts
units each. Windows passes 54 ordinary units and 18 compile-fail doctests.
Strict xtask clippy, formatting, doc-check, gated-test-check and the local
non-promotable performance contract pass. The frozen qualification manifest
retains SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The creator baseline and negative retirement inventories verify unchanged.
