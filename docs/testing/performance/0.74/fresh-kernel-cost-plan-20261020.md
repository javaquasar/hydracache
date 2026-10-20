# Local fresh kernel validation cost

The baseline is `94e53f29bab80734862673063bb3e9e114838ef0`. Attribute the retained
fresh origin, sender, pidfd, proc document and metadata operations locally, then
evaluate one hypothesis: use the existing fixed C string opener for the two
literal `status` call sites instead of constructing a `CString` on each call.
Do not combine this with buffer reuse, syscall removal, status parsing changes,
cached kernel observations or changes to any clock or deadline.

## Controls and acceptance

Run three serial samples of each actual validator and leaf, 64 iterations per
sample, without product execution or privileged start authority. Nested numeric
spans are inclusive wall observations, not exclusive CPU or syscall tracing.
Record current-process CPU separately. Preserve each result and first refusal;
do not retry failures into successful cohorts.

Before changing runtime code, compare the existing dynamic and fixed openers on
the same retained proc directory, with 32 warmup iterations and three alternating
1024-iteration allocation samples. Timing uses a separate uncounted loop. The
acceptance condition is fewer Rust allocation requests and requested bytes, not a
latency threshold. Do not infer libc allocation, RSS, product API throughput or
whole-attempt speedup from this leaf control.

Check both GNU 1.94 and MSRV 1.88. Differential checks cover the same object and
descriptor flags, fresh changed bytes, missing and non-directory parents,
non-proc filesystems and symlinks. Keep exact historical source reconstruction:
only two call substitutions, one helper visibility change and exact test hooks
may be reversed; unrelated validation changes must still fail.

Run targeted sender, origin, namespace, clock, custody and held-join regressions,
formatting, strict lint, historical source guards and local performance
contracts. Preserve original packet, request and placement time bounds. A
passing cost probe does not replace the full functional matrix or reopen the
failed native execution gate. Do not install a release, access the server, stop
the supervisor, change clock services or run product workloads.

## Evidence

Use a new `local-runs/fresh-kernel-cost-20261020` packet. Prior inventories and
qualification manifest remain untouched. Record exact source and executable
hashes for each compiler and every invocation, retain failures, then seal a
strict inventory. Final acceptance and remaining expensive observations will be
recorded here after the controls complete.
