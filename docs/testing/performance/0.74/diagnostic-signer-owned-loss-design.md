# Loss of the original private owner at two child stops

The test-only matrix kills the process running the unchanged private Owner,
rather than killing its child or dropping the guard. It exercises the existing
constructor and first-exec transition with a fixed owned /bin/true image. The
normal executable's full main-thread/empty-environment CLI is not this matrix's
entry: a reexecuted Rust test role calls the same private methods on its test
thread. No fault argument, report descriptor or alternate image enters the
normal CLI or library API.

A separate monitor process alone enables subreaping. It owns the temporary
image directory and retains its owner process pidfd. The owner reports a fixed
24-byte frame over a private socket after create or first, then waits for an
acknowledgement. The monitor corroborates the original creating task, sole child,
PPid, TracerPid and child pidfd while the child remains ptrace-stopped. It must
observe both originals live before injecting loss. Report bytes alone authorize
nothing: identity fields still require the live kernel relationship checks.

For each loss case, only the owner pidfd receives SIGKILL. The monitor requires
owner SIGKILL, original child pidfd readiness, adopted waitid(P_PIDFD) reporting
CLD_KILLED/SIGKILL, exact reap and subsequent ECHILD. It does not signal the child
on a passing path. The initial cut is before Owner::first installs EXITKILL;
the child already has verified PDEATHSIG. At first exec both protections are
present, so the terminal status cannot identify which protection caused death.

Two healthy controls grant the socket acknowledgement and let Owner::drop
terminate/reap its own child. The monitor then requires owner exit0 and ECHILD,
not a successful adoption. In all four cases an unrelated original /bin/cat
stays alive until separate explicit cleanup. PIDfd readiness plus wait status,
not elapsed time, establishes termination. Waits are bounded to five seconds;
status reads to 65536 bytes and monitor captures to 16384 bytes. Test role parent
loss is armed before exec, and failure cleanup can signal retained originals;
failure cleanup must never be confused with a passing no-child-signal result.

The first harness used self-SIGSTOP as its acknowledgement and saw a zombie
before fault injection. kill(SIGSTOP) returning does not establish a logical
barrier for the calling thread. The corrected bidirectional gate prevents the
owner from reaching cleanup until the healthy grant; failed captures remain
evidence of the rejected harness. No normal owner or child code was changed.

Full normal-entry loss, pre-arm and running-child boundaries, independently
selected root/runtime and mapped-image checks, protected custody and authenticated
handoff remain open. Worker original-start, all-writer revocation and cells 2–4
remain closed. See the [preregistration](diagnostic-signer-owned-loss-plan-20261015.md)
and [contract](diagnostic-signer-owned-loss-contract.toml). No cache-performance
measurement, server mutation, credential access or signing follows from this test.
