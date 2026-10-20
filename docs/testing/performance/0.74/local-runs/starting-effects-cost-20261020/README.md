# Starting durable effects and validation attribution

This local control separates collector Starting's filesystem effects from the
full live validation rounds around them. It changes only test observations;
the production function bodies, nine stages, three fsync invocations, checkpoint
order, error paths and original 300/60/10 bounds remain identical to baseline
`4d0f461c638d158ff62dbd3e0f8f9914d523581e` after exact marker removal.

Measured source is `9471afb4f2ce297c5c2d9ece77603c7e6a3e5bfe`.
Each compiler-selected release test image has its own SHA256 receipt. The
original synthetic material bundle, qualification digest and five preceding
sealed packets remain unchanged. Nothing is installed or run on the server;
there is no supervisor, clock/service or product operation.

## Measurement boundaries

The intended cohort has three serial healthy observations per GNU 1.94 and
MSRV 1.88 lane. Only GNU completes its three profiles; the first MSRV profile
refuses and its two remaining profiles are not run. Each complete GNU profile
retains ten preparation phases. Starting has 91 ordered events in the outputs
phase: encoding and begin, nine repeated groups, final round and acknowledgement,
then the original pipe/sink boundaries. Each stage separates its opening full
round, storage opening inspection, phase checks, effect, storage closing
inspection, checkpoint and closing full round. There are 19 surrounding full
rounds including the final one, in addition to storage checks in begin and ack.

`effect` includes the existing stage arm and instrumentation overhead. Its three
fsync arms select the held directory/file FD and call the unchanged `sync_all`;
no storage inspection or full live round is inside those intervals. Write/seal
arms also retain their original metadata/content checks, so their interval is
not a bare write syscall. The two no-op arms provide an observation-floor
control, not an overhead subtraction. Process CPU excludes children, and nested
kernel/live spans overlap. These instrumented observations are not functional
admission gates, native permit ages, disk qualification or cache API benchmarks.

For the three GNU cost observations, `facts.ps1` requires exact boundary order, original healthy results and all
kernel/live/authentication/signature frequencies in all ten phases to equal the
preceding frozen profiles. Failed or incomplete invocations stay failures and
are never retried into the healthy cohort. `run.ps1` refuses to overwrite any
recorded invocation, checks selected source/image/recipe before and after use,
and captures the native exit immediately without a redirection pipeline.

## Reproduction and independent gates

`run.ps1` supplies Build, Checks, Lint, Profile and Matrix stages for each lane.
Complete own builds/checks/lints before serial profiles. Matrix removes both
profiling and case overrides and executes the original five cases in order.
Use a new evidence directory to repeat a cohort; never overwrite this packet.
Keep the original bundle unchanged rather than rebuilding its source receipt.

`review-checks.ps1` runs formatting, strict xtask lint, all 359 historical/current
source and contract assertions, documentation, registry and local performance
contracts. The two current-source tests require all new marks and restore both
complete preceding bodies; the shared source adapter remains idempotent and
preserves all old assertions. `verify.ps1` re-derives the summary from raw logs,
binds receipts/inputs and rehashes old packets; `seal.ps1` adds a sorted complete
SHA256 inventory. Hash integrity is not production trust or enrollment.

## Local result and incomplete gates

Across the three complete GNU profiles, Starting is 1263.504–1349.378 ms.
Its three fsync intervals together occupy 4.707–22.226 ms, while all nine effect
arms together occupy 4.824–22.349 ms. The 19 surrounding full validation rounds
occupy 1256.868–1342.783 ms: 98.10–99.51% of each profile's Starting span. Fsync
occupies 0.35–1.72%. Both storage inspections together occupy 0.978–1.242 ms;
encoding occupies 0.529–0.670 ms. These are local instrumented observations,
not pure syscall latency or a cross-compiler benchmark. The remaining owner is
full fresh validation, not an avoidable duplicate fsync demonstrated by this
control. No runtime optimization candidate is adopted; no required check,
durable effect or original deadline is removed or batched.

The MSRV profile returns 101 in the unchanged kernel `clock-order` check during
outputs. Boottime advances 4,113,714,415 ns but realtime moves backwards
3,003,892,153 ns on the same boot. Its causal deadline does not refuse; the
partial Starting observations do not enter the healthy cost cohort. The
independent GNU unprofiled matrix returns 101 at healthy with the same original
predicate: boottime advances 5,355,547,947 ns and realtime moves backwards
1,719,003,928 ns. No later GNU case or profile retry runs. The independent MSRV
matrix passes all five original cases once. The actor changing time is not
attributed, and no clock or time service is changed.

Initial environment inspection records boot `9360d8a9-50f9-4818-9509-21aca8f02f71`
and local ext4. Profiles and final fixture postflight use boot
`cf619cb0-6679-4f52-a5b8-f6a9802d9ec4`; the before/after environment continuity
requirement therefore fails. This is distinct from same-boot realtime reversal
inside the refused calls. No cause for the boot change is established. Final
postflight finds no matching temporary root-fixture directory/cgroup.

The first analysis verifier refuses this boot mismatch. Its empty redirected
log and missing wrapper exit receipt remain as observed; the exception and tool
exit 1 are disclosed in `analysis-review-refusal.json`, and the exact refused
verifier is retained with its hash. The final integrity verifier checks that
these failures remain failures. It never calls the six-profile cohort or both
matrices passed, and source/lint passes do not replace those missing gates.

All 359 source/contract assertions, both compiler lanes' focused checks and
strict lint, formatting, docs, registry and local performance contracts pass.
The normal Starting selection runs storage/record/protocol regressions; its
manual process-crash tests remain ignored. This work does not renew the separate
joined SIGKILL/lost-ACK gate. Native composition, original authenticated delivery,
all-writer cessation, later lifecycle cells, reconnect and real enrollment also
remain independent closed requirements.
