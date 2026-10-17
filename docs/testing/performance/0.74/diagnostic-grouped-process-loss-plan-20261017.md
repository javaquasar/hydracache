# Grouped preparation process loss and return acknowledgement

Pin baseline 674ea58677527933b3ffd33d8f3cdda46f6bd037. The grouped candidate
and every production predicate, observation point and deadline stay exact.
Add only a private process harness beneath the original output-join crash
tests. Reuse their owned scratch roots, snapshots, lock/retained-history checks,
pidfd owner cleanup and complete output-inventory validator. Strip only the
exact new child-module registration when checking the old frozen harness;
do not relax its body, earlier crash matrix or existing reference tests.

## Fixed interruption points

Use a fresh child and fresh context for each point: opened, loaded, bound,
message, armed, returned-unacknowledged. The five internal cuts use the existing
candidate callback; the last occurs after its successful return and before the
test acknowledgement can be emitted. The parent waits for a bounded private
readiness record containing the exact cut and its owned Child PID, pins that
unreaped child with pidfd, then sends SIGKILL only through that descriptor.
Require kernel termination readiness, wait status SIGKILL and no acknowledgement.
No PID search, arbitrary signal target, worker, systemd call or outgoing message.

One complete matrix uses deterministic seed 0x07406c17 to order the six cuts.
Repeat it three times on normal GNU release Rust 1.94 and MSRV 1.88. Each repeat
also has one ordinary zero-exit control with exactly one test ACK and one
zero-exit lost-return-ACK control with no ACK. Both must finish the unchanged
reference validation before normal owner drop; ACK suppression is explicit
test-harness behavior, not a network/systemd delivery claim.

Before terminating or releasing the child, prove the original host flock is busy
and independently inspect complete sealed intent/Reserved/Starting records and
empty sinks. Snapshot original ancestry, context and parent-owned authority
metadata/bytes. After terminal wait, require the flock can be acquired, original
and authority snapshots are unchanged and interrupted context is preserved.
Normal exits must clean their temporary context. After every outcome use two
fresh selected helper processes to prove retained history refuses a new empty
context, with no changes to the retained evidence. A completed preparation or
missing ACK never licenses adoption, retry, repair or publication of completion.

## Screening and scope

First run one acknowledged control and one returned-unacknowledged SIGKILL on
Rust 1.94; stop the cohort on failure. Only then run both three-repeat matrices.
Per compiler the full matrix must have 18 SIGKILLs, three acknowledged controls,
three lost-ACK controls and 48 independent fresh inspections. The screen is a
separate two-case cohort, not part of those repeated counts. Logical checkpoint
and exact wait/ACK results are the oracle; existing 60-second harness waits and
an outer runner timeout are fail-safes, never refreshed admission deadlines.

Run only nonroot isolated fixtures on the authorized server. The harness uses
owned 0700 direct /tmp leaves required by the existing fixed-ancestry fixture;
they are not installed roots or a global cleanup target. The driver root and
signed nonexecuted bundle data are separately isolated. Guard exact boot,
supervisor PID/start/restarts, installed binary/receipt hashes, absent active
contexts and absent diagnostic install before/after runs. Final executable-object
postflight must find no remaining helper binary. No install, supervisor stop,
root/clock mutation, product workload, observer or expensive Actions.

Retain source capsule/blobs, binary and script hashes, raw case/inspection rows,
fixed seed/order, an independent local verifier and the exact prior packet proof.
Run current/MSRV protocol units, original alarm/trace units, targeted integration
and compile-fail checks, all-target check, strict Clippy, static contracts,
registry, workspace format, docs and local non-promotable performance contract.
Preserve any negative result without retrying a failed protocol attempt into a
pass or widening an original deadline.

This proves only grouped nonexecuting preparation process loss and synthetic
return-ACK loss. It is not a full production SIGKILL/power-loss matrix,
authenticated original start, autonomous guardian, P2/P3 all-writer cessation,
completion/later-cell admission, installation or qualification.
