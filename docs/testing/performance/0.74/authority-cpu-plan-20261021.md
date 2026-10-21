# CPU attribution of fresh authority phases

Baseline is `3d227add18d68bea62686ea85f5c225c51ba3f1b`. Attribute CPU consumed
by the six actual authority methods already exposed by the owners fixture;
do not time its traced expression copies. This is local diagnostic work, not
a runtime candidate or server admission. All original live checks, error order,
fresh reads, retained owners and 300/60/10 windows remain unchanged.

Use Linux `CLOCK_THREAD_CPUTIME_ID`, recording its resolution and 64 adjacent
clock-pair deltas. Reject syscall errors, negative seconds, non-normalized
nanoseconds, overflow and backwards readings; never substitute wall-clock,
clamp negative differences or repair a failed measured invocation. Test these
refusals with synthetic readings. This CPU source does not validate civil time,
boot continuity or the earlier WSL live failures.

Use the original one/seven-generation fixture, its exact journal bytes and
policy epochs. Time only batches, with the allocation observer inactive.
For each depth, warm up each of six actual phases and an empty dispatch loop
32 times, then record 14 samples of 2048 operations per case. Samples 0 through
6 rotate seven cases forward; 7 through 13 rotate them in reverse. Each case
occupies every position once in each direction. The empty case retains batch
dispatch and black-box barriers, not runtime validation. Record raw batch CPU
nanoseconds, CPU per invocation, empty costs and the signed residual of
full minus twice directory minus twice inventory minus records minus markers
minus bind. Do not force noisy timing sums to conserve, subtract/clamp overhead
into a positive result, choose an acceptance threshold or claim an improvement.
The original full inspection includes a different cache/branch schedule from
separate phases; CPU residuals and sample ranges must remain visible.

The existing opt-in test binary still installs its counting allocator even when
the thread-local observer is inactive. Its inactive TLS bookkeeping, fixture
dispatch, assertions and the boundary clock reads remain part of these CPU
observations. The empty control does not estimate per-allocation bookkeeping,
so these samples cannot establish production CPU costs or isolate allocator,
kernel and payload-copy CPU within a phase. A separate candidate must use
matching instrumentation on both sides and retain those limits.

Use one capture per compiler, GNU 1.94 and MSRV 1.88, sequentially without
concurrent local builds or measured lanes. Separately run the unchanged
eight-case allocation/read control, comparing all observations with the sealed
owners packet; require the same allocation/read counts and fixture payloads.
This does not count all syscalls or measure API throughput, wall latency, RSS,
retained memory or copied bytes. No measured failure may be retried as healthy.

Add only an opt-in test hook and registered manual diagnostic, with an exact
source adapter that preserves every other byte and rejects partial/duplicate
hooks. Test runtime byte restoration, clock failures, rotation coverage and
inactive allocator observations. Before capture, freeze non-overwrite check,
capture and review recipes; bind source inputs and test image hashes. Run
focused tests, affected all-target checks and strict all-feature lint on both
compilers, historical source contracts and repository format/docs/registry/local
performance checks. Review structural JSON facts, all seventeen prior immutable
inventories and the unchanged qualification digest before sealing new evidence.

Only after CPU attribution can a separate hypothesis propose changing ownership
of names or buffers. Allocation frequency alone is insufficient justification.
No new live matrix, clock/service action, server run, installation, supervisor
operation, product workload, expensive qualification or push belongs here.
Delivery, writer cessation, durable lifecycle, SIGKILL/lost-ACK, reconnect and
trust enrollment remain independent closed gates.
