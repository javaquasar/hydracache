# Quarantine of an owned incomplete Starting attempt

Baseline is `7f18ae34e2de2c4163bba305f443ced367c25cb6`. This P1 slice
consumes the original acknowledged Starting guard into a permanently unusable
quarantine while retaining its original borrow, objects and fence. It does not
publish Terminal, complete a cell, prove writer cessation or authorize cleanup.
The approved intermediate server-readiness plan remains in force.

## Ownership and refusal protocol

Expose a consuming `quarantine` method only on `FixedStartingIntentRead`.
Latch the original intent, task and publication readers before attempting any
durable operation. Use the existing original authority refusal bridge with its
captured trust and host pins, and the existing Policy refusal reason. Do not
take a new envelope, peer, time assertion, path, descriptor or caller outcome.
Revocation does not renew the expired launch consent. This operation can decline
the original authority even if the retained Starting document has drifted; it
must never inspect, adopt or repair that changed document as permission.

Return an opaque non-cloneable quarantine owning the consumed chain. Expose
only a borrowed disposition: Acknowledged for a successfully validated original
authority refusal, Uncertain with the exact persistence error, or PreviouslyRefused
without claiming durable acknowledgement. Already refused readers do no further
I/O. There is no retry, conversion to a healthy reader, start, release, state
export or descriptor access. Dropping the guard releases its runtime borrow and
lock, but retained context still prevents fresh admission.

No new durable schema or namespace entry is introduced. The existing bounded
authority record and commit pair are unchanged. A successful authority refusal
is not an attempt completion receipt. A partial pair or lost acknowledgement is
not repaired or replayed. Missing process identity and cgroup inode are never
evidence of an absent writer. Cleanup remains pending P3's authentic evidence.

## Preregistered local proof

Capture baseline Starting tests, then a missing-method red test. Cover successful
refusal and exact original object conservation, all four existing logical
authority write faults, unsafe or epoch-advanced authority, original Starting
object replacement, expired launch consent, already refused no-I/O behavior,
independent concurrent chains and retained-context fresh admission refusal.
Add compile-fail checks for cloning, launch, release and consumed-guard reuse.
Use three same-source functional repeats with the existing local fixtures.

Run affected Linux and Windows regression, current and MSRV all-target checks,
warnings-denied lint, formatting, Rustdoc and documentation checks. Retain every
negative and positive capture with exact-source hashes and a read-only verifier.
This slice adds no storage write boundary; logical faults do not establish real
SIGKILL during authority append, machine reboot or physical power-loss durability.
Those proofs remain explicit follow-up work, not inferred from the prior Starting
record SIGKILL matrix. No host operation, workload, qualification or ship is opened.
