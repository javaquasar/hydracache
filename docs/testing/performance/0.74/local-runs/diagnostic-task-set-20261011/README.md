# Local original task set evidence

This packet covers the separate read-only original-task primitive, not signed
worker enrollment or a production start. The [design](../../diagnostic-task-set-local-design.md)
and [contract](../../diagnostic-task-set-local-contract.toml) freeze bounded scans
before implementation. Exact capture byte sizes and SHA-256 values are listed in
`manifest.json`; `python verify-packet.py` performs read-only offline verification.

Linux scoped tests pass 382 with one existing ignored systemd test. Eleven new
unit tests pass three serial repetitions; one external API compile test passes.
The Windows full supervisor package passes 189 existing portable tests; Linux
task tests execute zero cases there. Root performance/evidence tests pass 100
and 13; governance passes 23 after the new cfg target is registered under the
existing ordinary Linux supervisor lane. Both platforms pass all-target checks,
strict lint and Rust 1.88 supervisor compilation; Linux strict rustdoc passes.
These are local working-tree checks, not an immutable release qualification.

Retained negatives include absent API E0432, an unchanged-byte directory mutation
fixture mistake, strict lint, the initial parallel membership-error expectation,
and missing cfg-target registration. The initial membership failure contains no
actual error variant; its exact kernel cause is not established. The fixture now
acknowledges removal only after procfs task absence. It adds no observer retry,
wall-clock acceptance threshold or automatic qualification restart.

The first capture loop also lost its shell variable at the PowerShell boundary:
all three runs wrote `repeat-.log`, retaining only the last. That closed capture
is preserved; three fresh repetitions use explicit distinct filenames. Only
those complete retained repetitions supply the repeated-check claim.

The modeled seeds are `0x7612026` for 256 first-error schedules and `0x7622026`
for 512 directory-record mutations per final repetition. They are not live
namespace/credential races. Real helper cases cover stable single/multi-thread
operation, addition/removal/restoration, original process exit and independent
readers. Actual foreign namespace changes, TID reuse and nonleader credential
mutation are not performed; private original-object/policy mutations cover their
refusal predicates. A new task disappearing entirely between scans may be missed.

The guard retains at most 128 tasks. Directory scans require EOF within 16 calls
and 65536 bytes, including dot records; no truncation or refresh is accepted.
Each task retains six descriptors (directory, stat, status, namespace directory,
user and mount; the original task-set directory is shared),
plus the fixed observer user/mount objects and original process borrow. Actual
FD exhaustion refuses without dropping tasks to meet the budget.

Original signed composition, durable issuer/epoch/refusal authority, fixed-host
positive enrollment, root-owned preparation, authenticated start and autonomous
writer cleanup remain outside this packet. No server, Actions, workload, product
candidate or numerical performance run is launched. The frozen qualification
manifest and its eight input files remain unchanged. The previous Windows
workspace readiness packet is not relabeled as a new full-workspace run.
