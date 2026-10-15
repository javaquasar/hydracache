# Private original owner loss at two stopped child boundaries

Source `ce573dcc3aadd7b02e0f23527311116ceb10a7a7`, tree
`0f4282d97995c597b33ded983adcf5356a8690ad`, preregistration `32098fef`.
The source commit adds only tests, contract and documentation; the normal owner,
child policy and previous library fixtures are unchanged from `ce5fd37d`.
These are unsigned development captures, not executions attested against a clean
checkout of that SHA. Offline hashes establish packet/source consistency only.

Five current Linux repeats each pass 12 unit tests (including the no-role entry
helper), one portable CLI test and five normal CLI process tests. The unit matrix
has two actual owner SIGKILL cases and two healthy acknowledgement/drop controls.
Each loss case requires original child pidfd readiness, adopted CLD_KILLED/SIGKILL,
reap and ECHILD without monitor signaling the child. An unrelated original stays
alive until explicit cleanup. Both stops are in the unchanged private constructor
using a fixed /bin/true fixture, not the complete normal main-thread CLI path.
The initial stop precedes EXITKILL installation; after exec both protections are
present and no individual mechanism is attributed. Root/runtime, pre-arm/running
boundaries, credentials, signing, worker original-start and all-writer revocation
remain open. There are no cache-performance measurements or server/GitHub changes.

Linux Rust 1.88 passes all-target/all-feature check and 12/1/5 tests; current Linux
passes check and strict Clippy. Windows current check/strict Clippy (including
xtask) and current/MSRV CLI refusal pass. Root contracts pass 158, registry 174,
format/doc/local contract checks pass; require-ship explicitly refuses with exit1.
The old first-exec/constructor-loss and child/policy groups pass 11/7/5/7. A
mistyped creator filter ran zero tests in regressions.log; it is not coverage.

The expanded old creator/thread-exit group is NOT green under its default parallel
command. Both retained default runs fail 13/1 at
signer_thread_exit_no_exitkill_is_not_positive; serial execution passes all14.
No old fixture source was changed. Serial success is diagnostic, not a waiver or
repair of the parallel failure. Root cause remains unassigned and this regression
must be resolved before server or release admission. All-regressions-passed stays
false in the manifest and verifier. Do not describe this packet as a full green gate.

The first new harness also failed: self-SIGSTOP was treated as a logical owner
acknowledgement, but the exec case already had a zombie before fault injection.
The corrected private bidirectional socket only grants healthy controls, preventing
loss cases from reaching normal cleanup. The failures and status diagnostic are
retained. Temporary image ownership now belongs to the monitor; test role parent
loss cleanup is armed. Initial E0308 si_pid signedness was corrected before these
successful runs; its compiler output is not part of the raw packet.

Reproduce the focused tests using Rust1.94 or Rust1.88 on nonroot Linux x86_64:

```text
cargo test -p hydracache-long-run-supervisor-074 --bin diagnostic_signer_owned_probe --test diagnostic_signer_owned_probe --test diagnostic_signer_owned_cli --locked --offline -- --nocapture
cargo test -p xtask --test performance_contract_074 --locked --offline
```

The old group must be reproduced without removing parallelism or changing deadlines:

```text
cargo test -p hydracache-long-run-supervisor-074 --lib diagnostic_signer_root_fixture::rooted_exec::handoff::loss::creator_loss:: --locked --offline -- --nocapture
```

Run `python -B verify_packet.py` and `python -B -m unittest test_verify_packet.py`
from this directory. The verifier rejects missing/rehashed result markers, changed
sources/oracles, false admission flips and lost negative runs. It cannot attest
execution provenance, turn a failed regression green or authorize a launch.
