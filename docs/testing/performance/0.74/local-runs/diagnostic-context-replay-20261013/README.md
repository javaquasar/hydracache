# Local fresh-reader context replay evidence

This packet combines actual owned SIGKILL with modeled changes to one asserted
host/trust component. It does **not** reboot a host, switch namespaces or enroll
production authority. Fresh reader results remain observation data, never a
recovered runtime acknowledgement, retry, start or cleanup capability.

## Exact source and reproduction

Baseline: `bd72707189db21fe5ac67111462c3c91b71f049e`.
Preregistration: `d6ee09ce37aad58f429ea075e0d80ca20579b566`.
Implementation: `4302202b12d90f0708c029929b6f7f51c6f38aea`.

Only the existing test-only process harness, its new test-only context helper
and manual gated-test registration change in the implementation commit.
Production authority, storage boundaries, record formats and product code
remain byte-identical to baseline after LF normalization. The packet pins
source hashes, frozen inputs, command lines and raw capture hashes.

Run `C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-context-replay-20261013/verify_packet.py`
for read-only verification. The local Linux manual invocation is
`cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --lib
diagnostic_process_crash_context_replay_refuses_after_loss --
--ignored --nocapture --test-threads=1`.
The registered exact test retains the 600-second budget and seed 0x074d.
Do not relink libtest while its owned child processes are active.

## What the reader proves

Eight profiles cover unchanged inputs and one changed boot ID, machine ID,
user-namespace inode, mount-namespace inode, authority digest, epoch or fixture
issuer. Fast seeded checks cover 32 unchanged and 224 changed input groups;
unknown profile names refuse rather than falling back to unchanged context.

Six actual signal boundaries are BeforeRefusal, RecordCreated, RecordSynced,
MarkerCreated, DurableCommit and AfterReturn. One uninterrupted AfterReturn
control distinguishes process loss from ordinary completion. Each retained
case gets two independent fresh reader processes per profile.

Complete records with changed claims return exact `AuthorityError::Binding`.
Partial append tails return exact `AuthorityError::Invalid` before binding:
they are not evidence that a boot/issuer check was reached. Unchanged complete
records expose active authority before refusal and sealed refusal after commit.
Every profile still refuses a fresh reservation; bytes and metadata remain
unchanged after every read. The original lock and exact owned helper are
checked independently, without promoting absence into completion proof.

The fixture parent supplies its original policy digest and known test issuer.
Each child samples its current local host context then applies one modeled
mutation. These are controlled test inputs, not independent production pins,
authentic reboot observations or original-peer/time credentials.

## Development and closed admission

The nine baseline authority-fixture tests pass. The preregistered missing-helper
red test fails compilation with E0433/E0282 and executes no process matrix.
Three final fast repeats pass three tests each, with the same seeded counts.

Three final-source process repeats each pass six SIGKILL cases, one control and
112 fresh inspections: 18 kills, three controls and 336 inspections. Per repeat,
56 complete mismatches return Binding, 48 partial-tail observations return
Invalid, and eight unchanged complete observations retain their exact state.
The separate nine-proof regression covers 78 kills, eight controls and 270
fresh inspections, including all eight pre-existing proofs.

The initial three admission logs are retained with a null observed exit: the
command-group tool receipt was lost to an orchestration store error, not a source
test failure. Separate final captures retain explicit group exit verification;
no unreceipted initial invocation is used as a passed gate.

Scoped Linux regression passes 676 tests with ten manual/host tests ignored;
the nine process proofs run separately and the real-host bus test remains
unrun. Windows passes 204, root contract tests 166 and Linux doctests 52.
Current/MSRV checks, warnings-denied Clippy/Rustdoc, scoped format, documentation,
book, artifact and local performance-contract checks are retained.

Qualification dry-run and require-ship are expected exit-one refusals. The
qualification output is absent; the manifest and eight frozen inputs are
unchanged. No performance sample, server operation, product workload or
qualification run occurs.

P1 still lacks genuine reboot semantics, later fixed cells and authenticated
completion. P2 trusted peer/time and production enrollment, P3 execution and
writer cessation, P4 signed installation and P5 host rehearsal remain separate
dependencies. Whole-host rollback requires external authority design; this
modeled reader matrix does not settle it.
