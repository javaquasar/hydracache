# Local kernel sender and clock evidence

The diagnostic input producer obtains per-message kernel credentials/pidfd and
original-scope time. Production issuer/host enrollment, durable replay/fence
admission and original start remain open; this packet cannot admit a server run.

Baseline: `5483ae1c0d37dc925c464662c1f88c8db83e150a`. Preregistration: `b485fd18`.
Implementation: `f2dbbe64c0fbf0a0eeca3bf6e8e2177188bc7f67`.
The [contract](../../diagnostic-kernel-admission-design.md) requires a distinct
diagnostic receiver, never a fallback or reinterpretation of campaign transport.

## What the tests establish

An original receiver enables SO_PASSCRED/SO_PASSPIDFD and uses one bounded recvmsg
to join payload, credentials and message sender pidfd. Every control FD is owned
and closed on failure. Actual SCM_RIGHTS tests pass 1, 32 and 253 copies of an
owned pipe writer; after refusal no copy survives and the owned reader sees EOF.
Private synthetic ancillary-layout tests are a separate parser/ownership family,
not kernel sender authentication. Empty/oversized packets, disconnects and socket
option drift refuse; a refused receiver cannot continue.

The retained sender's proc directory and credential/group projection are checked
between original pidfd readiness/correlation and receiver origin checks. After
sender exit no fresh numeric PID can be substituted. Kernel clock sampling
retains procfs boot plus user/mount/PID/time namespace handles; CLOCK_BOOTTIME
supplies elapsed time including suspend, while CLOCK_REALTIME gates signature
freshness only. Boot identity, reversal and exclusive 60-second hold boundaries
have separate refusal tests. Scope changes are modeled; no namespace switch or
host reboot is performed.

Three reviewed fast repetitions pass 14 tests each with one manual proof skipped.
Three reviewed manual repetitions each observe SO_PEERCRED identifying the
parent, an inherited-socket message identifying the child and its original
pidfd, then actual owned SIGKILL and refusal of that child's retained packet.
The parent's separately received packet remains live. The child is an owned
bounded Python helper with RAII cleanup, not an admitted product worker.

Supplied fixture-key authentication exercises signature, principal, boot and
freshness refusal using kernel peer/time, with no caller peer/time argument.
Fixture keys are compiled only in tests and never enrolled as production trust.
Successful verification returns request data, not execution/completion authority.

## Negatives and checks

The missing-method red test fails E0599. The first compile retains a Rust 1.94
diagnostic-renderer panic during borrow checking, with unused imports/mutability
in the harness. Removing those unused declarations allows compilation without
changing toolchain or policy. Strict Clippy separately rejects a manual modulus
test; the implementation uses is_multiple_of without a lint exemption. Both
negative captures remain distinct from reviewed-source success.

Unix transport/diagnostic IPC regression passes 31 tests; collector regression
passes 20, and doctests pass 58. Root gate/evidence/performance contracts pass
143. Current/MSRV all-target/all-feature checks and strict lint pass on Linux and
Windows; format, documentation, registry and book checks pass. This is a focused
step gate, not a whole-workspace release milestone. Frozen qualification inputs,
product crates, dependencies and scripts remain unchanged. Ship admission stays
closed. No server inspection/mutation or numerical workload occurs.

Run `C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-kernel-admission-20261014/verify_packet.py`
for read-only source/hash verification. The manual exact cargo command is
`cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --lib
diagnostic_kernel_admission::tests::kernel_admission_inherited_socket_identifies_child_and_refuses_after_sigkill
-- --ignored --exact --nocapture --test-threads=1`. It needs nonroot Linux,
python3 and both kernel socket options; run single-flight under the registered
180-second gate, with a 30-second per-helper harness deadline.

The [pre-server checklist](../../pre-server-closure-checklist-20261014.md) records
the operator public-key/pin input and remaining P1/P2/P3 integration. Kernel facts
in a receiver namespace do not establish independently enrolled host identity,
controller provenance, original start or writer revocation.
