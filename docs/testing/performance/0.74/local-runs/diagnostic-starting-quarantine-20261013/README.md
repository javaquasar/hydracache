# Local evidence for owned Starting quarantine

This packet covers a consuming quarantine of the original acknowledged Starting
guard. It preserves the original runtime borrow, objects and common fence while
declining the original authority. It proves neither a completed cell nor safe
resource cleanup. Server execution and qualification remain closed.

## Source and reproduction

Baseline is `7f18ae34e2de2c4163bba305f443ced367c25cb6`; preregistration is
`a5d3d9c410233261cc82ebe09087d0a291ea0e21`. The manifest pins the implementation
commit `9875fa44a58b45f25fde249c9c8038bf946c6425` and exact LF-normalized runtime/test sources. Only those two Rust files
and the preregistered design differ between baseline and implementation.
Commands and original capture hashes are recorded individually. Run
`C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-starting-quarantine-20261013/verify_packet.py`
from the repository to inspect this packet without writing or running workloads.

The final functional command is `cargo +1.94.0 test -p
hydracache-long-run-supervisor-074 --locked --lib starting_quarantine_ --
--nocapture --test-threads=8`, locally on Linux through WSL. Three repeats use
the same final source. These are correctness fixtures, not numerical benchmarks.

## Functional coverage and uncertainty

Eight cases cover acknowledged original authority refusal, an already durable
refusal without another append, four logical authority append faults, unsafe
storage or advanced epoch, changed/replaced Starting objects, declining without
fresh launch consent, no I/O after prior refusal, and independent concurrent
chains. Every applicable case checks the original common fence and retained
attempt objects; fresh reservation still rejects retained context after drop.
Compile-fail checks deny cloning, launch, resource release and consumed reuse.

At DurableCommit the injected acknowledgement failure remains Uncertain despite
the durable tail. Repeated disposition reads do not retry or upgrade it. A
PreviouslyRefused outcome also makes no durable claim. No new storage format or
write boundary is added, and no missing PID/inode is treated as cleanup evidence.

## Retained development results

All three final repeats pass eight cases. Scoped Linux regression passes 668
tests with eight manual/host cases ignored; those ignored proofs are not rerun
or claimed by this slice. Windows regression passes 204, root contracts 166 and
Linux compile-fail doctests 52. Current/MSRV all-target checks, warnings-denied
lint/Rustdoc, format, performance-contract and documentation/book checks pass.

Baseline passes sixteen Starting tests. The missing-entry red test fails with
E0599/E0433. The first implementation compile stops at an internal Rust 1.94
diagnostic-rendering panic while borrow-checking a macro-expanded test; its raw
log is retained with expected exit zero and observed exit 101. MSRV compilation
then reports an unused mutable test binding. Removing that unnecessary binding
allows current-toolchain compilation without disabling warnings or changing
assertions. The subsequent seven-case development run passes; the additional
already-durable-refusal case is included in all three final eight-case repeats.

The compiler sequence is an observation, not a claim to have proved the compiler
bug's root cause. No failed capture is relabeled as a passing test. Logical faults
do not establish SIGKILL during authority append, host reboot or power loss.
Those distinct proofs remain required follow-up work.

## Admission boundary

Qualification dry-run and require-ship continue to refuse with exit one; the
requested qualification artifact is absent. The exact qualification manifest
and eight frozen input files remain unchanged. No product candidate, service,
host operation, expensive workload or performance claim is introduced.
P1 remains open for actual authority process-loss proof, reboot semantics,
authentic cell completion and later fixed cells; P2 through P5 remain open.
