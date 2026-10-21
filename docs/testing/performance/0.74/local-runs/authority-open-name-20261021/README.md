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
performance. The 64 clock-pair samples are retained, not subtracted.

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
`final-review.ps1` additionally checks final documentation/format/registry/
performance receipts, baseline ancillary checks, the read-only environment
capture and mutation audit before delegating to the byte-pinned original
reviewer. It does not change request targets, estimators or measured samples.
Use `final-review.ps1 -Mode Review` for the completed packet.

No live matrix, clock/service change, server action, install, supervisor stop,
product workload, qualification or push is part of this comparison. Existing
time refusals, delivery/writer cessation, lifecycle and server gates remain.

## Local request and CPU observations

Baseline source is `275cd253875dc32eda5c6d2ba3c3b1c2290e9da2`; runtime
candidate is `b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d`. Both compiler
lanes reproduce all three request batches. Depth one/seven full inspections
change from 23/99 allocations and 1662/8444 requested bytes to 21/85 and
1616/8122, with zero reallocations and unchanged 4/28 read requests. Actual/
traced phases conserve requests and fresh call counts. Directory/inventory
and the six non-authority full controls remain equal.

Short record/marker and multibyte helper calls each remove one request of
22/24/24 bytes; 24-byte and 255-byte names and the short directory preserve
their original one request of 25/256/4 bytes. Separate 65-byte cold record/
marker opens change 3 allocations/108 bytes and 3/112 to 2/86 and 2/88;
warm revalidations change 2/87 and 2/89 to 1/65 each. No new retained field
is introduced, and cold name cloning stays inside the measurement.

The CPU result is ambiguous, not a gain or nonregression result. Candidate
record/marker median signed pairs per call are GNU 9.57/-9.87 ns and MSRV
2.15/-3.06 ns. Their ranges are respectively -600.58 to 697.59,
-1306.38 to 50.63, -182.70 to 81.42 and -960.95 to 1025.85 ns. Every case
has both signs. Baseline record/marker pair medians are GNU 8.29/-6.00 and
MSRV 8.34/-8.42 ns despite equivalent helpers. Full six-case ranges and
positive/negative pair counts are reconstructed in the summary; no baseline
bias, clock cost or outlier is subtracted.

The four helper processes record boot
`211b1e59-b352-4b49-a762-2c07581c705b`, 1 ns clock resolution and empty
clock pairs spanning 132–524 ns. This is local context, not proof of remote
reconnect or a repair of old clock refusals. All captures finish once without
failure or retry. Both stages pass 213 authority tests per compiler, ancillary
controls, affected checks and strict lint. The independent admission gates
remain closed; this candidate is retained only for its confirmed request effect.

Historical/source checks pass 388 assertions across 23 targets. The raw-log
review reconstructs the full comparison and verifies all nineteen earlier
packets without changing them. Four in-memory request/scope/difference/order
mutations are refused. Final repository receipts and byte-preserving Git
storage are checked separately before the evidence handoff.
