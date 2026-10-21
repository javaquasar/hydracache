# Collector peer primary capacity candidate

Test one allocation hypothesis against sealed baseline
`da44628a85779977d5d1cb2f32e324239165eeff`. Replace supplementary.to_vec with
Vec::with_capacity(supplementary.len()+1) followed by extend_from_slice, then
keep the original push/sort_unstable/dedup and owned DiagnosticPeer unchanged.
Do not skip primary membership work, add a bound, change representation or
reuse a previous peer. The u32 slice length plus one fits usize for valid Rust
slices; ordinary Vec allocation/layout failure remains infallible allocation,
not a new recoverable admission result.

Keep the entire collector-input wrapper, live observation counts/order, original
300/60/10-second windows and first Causal(Time) failures. Add only an exact
private test adapter to actual peer and a pure allocator-test hook. Whole-file
reversal must restore the two changed old supervisor files; historical canaries
must compare the full earlier collector source, not exempt its filename.

Compare actual candidate requests with the unchanged baseline at the same 31
cases, 256 constructions each. Keep one-construction stage controls separate.
Use the baseline's canonical-union seed 0x074c0ee1 and 1024 cases, empty/present/
absent/duplicate primary shapes, integer extremes, 32-to-33 union, generic larger
inputs and owned-output controls. Compare complete old/new synthetic wrapper
first-error/latch/reentry schedules over the same 32 fault combinations; no
actual KernelPacket or clock is constructed.

Run pure tests/source canaries, affected all-target check and strict all-feature
lint on host/GNU/MSRV, formatting and local structural gates. Preserve first
failures and use distinct cohorts for any repairs. Verify ten prior raw seals
and 442 members without altering historical source-pinned reviewers. No live
profile, ignored/manual fixture, clock/service change, server/SSH/Actions,
installation, supervisor or product workload. A failed hypothesis is retained
and this leaf alone reverted. Pure allocation success cannot establish CPU/RSS,
full preparation timing, material/all-writer/fault/reconnect or server admission.
