# Pure proc document buffer baseline

## Scope and prior work

Starting source: `5310113d`. The existing custody preparation experiment already
compared complete Sender validators and adopted one invocation-local document
buffer at `1f371549`. Its three removed allocations per healthy invocation are
not a new result. Preserve that packet and its negative timing interpretation.

The new baseline isolates the remaining constructor and reuse operation only.
Do not rerun either complete validator, the original owner or a Linux fixture.
Use a separate dependency-free test crate and synthetic callbacks on the installed
Windows MSVC toolchains 1.94.0 and 1.88.0. This is not GNU/Linux evidence.

Copy the current complete DocumentBuffer declaration, constructor and read_with
body into a private model, omitting only the exact File/read_at adapter. A new
source canary must recover the complete original block after that one explicit
omission, including the trace scope, one-read contract and error site. Fake trace
and error receivers provide no live kernel capabilities. Keep runtime files and
the full Sender body unchanged, with exact source and mutation controls.

## Fixed cohort and acceptance

Use 256 complete buffer instances, four synthetic reads each with lengths
16, 37, 4096 and 128. Observe constructor/reuse/drop allocator requests, including
alloc_zeroed and deallocation, and keep a separate one-instance stage control
out of the main corpus. Do not measure elapsed time or claim physical zeroing,
CPU, RSS, retention, admission headroom or new savings.

Controls must cover zeroed first storage, repeated long-to-short reads, visible
prefixes only, independent ownership, empty/maximum/over-budget counts, one
callback per read, propagated Interrupted/I/O errors with no retry, and seeded
reuse using seed `0x074d5a18`. Counter controls must cover nested capture, unwind,
thread isolation, zeroed allocation, realloc and drop. Output every first result;
repairs need separately named cohorts rather than replacement.

Run standalone tests/check/strict clippy on both installed Windows compilers;
run focused xtask source canaries, affected-package check/clippy, direct rustfmt,
doc-check, gate registration and the local non-promotable performance contract.
Verify the twelve preceding raw seals plus the original custody packet without
relaxing any frozen reviewer. Keep all 300/60/10-second windows and first Causal
refusals. No runtime candidate, validation change, WSL/live profile, clock/service
change, server action, qualification or product workload is included.

Next decisions require the baseline and lifetime/error contract first. A large
stack replacement, cached observation or changed allocation-failure behavior is
not authorized by this baseline, and no server gate is promoted.
