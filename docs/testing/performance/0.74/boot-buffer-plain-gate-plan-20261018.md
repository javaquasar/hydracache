# Boot buffer plain worker and preparation gate

## Frozen comparison

Continue the allocation candidate `dc00ebf9`, retained in the
[local packet](local-runs/boot-buffer-20261018/README.md). Add one test-only
plain worker harness to current source. Build two release libtest images per
GNU compiler (1.94.0 and 1.88.0), at the same source path and with identical
manifests, locks, inputs and harness. Baseline substitutes only the old `boot`
function from `3b8e73fe`; all additional reader helpers and tests remain compiled
in both images. They are not invoked by the baseline worker route. Candidate
restores the exact committed boot function. Retain the exact baseline overlay,
source hashes, build output and copied image identities before another build.
Do not switch branches, modify another worktree, or leave the overlay installed.

## Plain measurement

Each exact test invocation creates one fresh original fixture, validates its
full context, snapshots the request/worker/OIDC inputs, and times 128 calls to
the actual retained worker validator. It performs no capture, allocator
instrumentation, per-iteration clock sample or output inside the timed loop.
Existing test-only entry hooks remain inactive and identical in both images;
this is uncaptured libtest timing, not a production binary or product workload.
Closing full validation and exact unchanged inputs are mandatory. No reservation,
start, send, completion or replacement process is admitted by this harness.

Run five fresh pairs per compiler on the unchanged dedicated host, with orders
AB, BA, AB, BA, AB (A old buffer, B candidate). No warmup/iteration extension,
best-sample selection or retries. Report whole-batch wall nanoseconds divided
by 128 and paired candidate/baseline ratios separately per compiler. Use the
two-sided 95 percent Student interval on five paired log ratios, df=4,
critical value 2.7764451051977987. A lower-cost observation requires interval
upper bound <=0.99 in both compiler cohorts. Runtime non-regression requires
upper bound <=1.03 in both; an inconclusive result does not authorize promotion
or another acceptance cohort. These local validator criteria are not native,
RESP, CPU/op, p99 or release qualification thresholds.

## Preparation correctness and safety

After the plain cohort completes without unexpected errors, run the existing
three exact addressed-frame tests once per candidate compiler: healthy controls,
twenty live refusals and nonroot fixed-channel refusal. The inventory is six
healthy fixtures/twelve rounds, forty sticky refusals, six same-process rights
transfers and two real channel refusals. Keep all original 46 effect brackets,
live checks and 300/60/10-second origins/deadlines. Reuse the exact nine-file
nonexecuted start-material bundle, not a newly generated product executable.

Stop at the first unexpected host failure, retain the failing cohort and skip
remaining cases. A runtime failure withdraws only the boot-buffer substitution;
do not retry, renew an origin or widen a window. Plain numerical inconclusiveness
and successful correctness remain distinct results. Any host identity/marker
drift blocks execution rather than replacing expected pins.

Use only owned nonroot correctness fixtures. Pre/postflight must retain the
existing boot, supervisor PID/restart count, installed binary/receipt hashes
and absent campaign/lifecycle/authority markers; scan registered fixture
executables. No service action, install, production trust enrollment, product
candidate, expensive workload or qualification. The 0700 runner is not a
filesystem sandbox: existing strict direct-/tmp authority leaves and RAII
temporary directories remain unchanged. No claim of single-parent write
containment or complete per-path write census is made.

## Acceptance boundary

Local format/check/strict Clippy, focused tests, exact source adapters, registry,
docs, non-promotable performance contracts and independent packet verification
must pass before commit/push. An accepted preparation-only candidate still
does not close authenticated original-start, negotiated bus FD delivery,
all-writer cessation, completion/later cells, SIGKILL/lost-ack or same-attempt
reconnect/enrollment/installation gates. Preserve the frozen qualification
manifest digest and all older negative packets.

## Result

The [retained packet](local-runs/boot-buffer-plain-20261018/README.md) at
`8e7e5da13b4cf4219c8f9cd15d4fd72e2899dc45` passes all twenty plain and six
preparation invocations without retry. Candidate/baseline geometric mean wall
ratios are 0.95153 (GNU 1.94.0) and 0.95820 (MSRV 1.88.0); both paired intervals
meet the frozen local criteria. Twelve healthy rounds, forty sticky refusals,
six owned rights transfers and two fixed-channel refusals pass. Preparation-only
acceptance does not open any execution, installation or product measurement gate.
