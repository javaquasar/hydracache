# Short file name encoding local comparison

The baseline and candidate use the same committed reference helper, actual
helper probe, payloads, recipes and estimators. Only the candidate may change
short non-directory name ownership inside `open_at`. This packet is local WSL
instrumentation, not product/API throughput or server admission evidence.

`check.ps1` retains the original preflight. Its GNU lint rejected manual
divisibility, MSRV lint rejected a non-inlined format argument, and the source
canary rejected extra registry whitespace. `check-repair.ps1` preserves those
failures and checks the corrected test code and exact registry block. No
measured invocation had started. These are authoring failures, not a cohort
that may be retried or silently replaced.

Run repaired Baseline checks on both compiler lanes and xtask, commit only the
instrumentation, then invoke `profile.ps1 -Stage Baseline` on GNU and MSRV
sequentially. The source commit must be clean. Each profile runs three separate
processes: new helper pairs/cold-warm requests, frozen owners requests, and the
frozen eight-case full control. Review the baseline before changing runtime.
Repeat the same source-bound process for Candidate after its focused gate.
Run historical and repository checks before accepting the comparison.

The helper has six successful cases: generation record, generation marker,
24-byte fallback, 23-byte multibyte UTF-8, 255-byte fallback and short directory.
It warms each arm 32 times, captures three alternating 1024-call allocation
batches and 20 alternating paired 1024-call thread-CPU batches per case.
Reference always precedes actual in even samples and follows it in odd samples.
CPU includes open, CString destruction where applicable, close and loop work;
the counting allocator is inactive. Raw pair differences are actual minus
reference, without subtraction of clock or loop overhead. They compare helpers
within one source-local process, not different compiler speeds or whole API
performance. An empty clock-pair distribution is retained, not subtracted.

Cold leaf open includes cloning its owned name, fresh open, metadata, bytes
and destruction; warm leaf revalidation starts with a retained leaf. Both use
the same 65-byte payload, three 1024-call samples and 32 warmups. Neither is
whole-registry initialization. Owners/full controls separately cover retained
journal fixtures at depths one/seven, with unchanged payload hashes and epochs.
Their frozen `runtime_changed=false` fields describe the old instrumentation
contract and are not comparative assertions about the new source commit;
stage receipts and this comparison determine runtime differences.

The 24-byte candidate buffer has bounded source-level storage. Zeroing it
and copying the complete short name are logical writes, not a measurement of
physical copied bytes, stack-frame size, allocator CPU, retained RSS or memory
residency. Cold/warm request changes must agree and may not be hidden in setup.
Directory and longer names retain the allocating fallback.

`review.ps1` reconstructs request targets, identities and CPU statistics from
raw logs, checks source/image/recipe receipts, preserves all failed preflights,
and verifies the previous 19 packets through their sealed inventory chain.
Summary and inventory are written once. No old evidence is rewritten.

No live matrix, clock/service change, server action, install, supervisor stop,
product workload, qualification or push is part of this comparison. Existing
time refusals, delivery/writer cessation, lifecycle and server gates remain.
