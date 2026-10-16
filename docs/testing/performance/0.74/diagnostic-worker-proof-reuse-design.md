# Original worker verification proof reuse

This is the second candidate under the
[new preregistration](diagnostic-worker-proof-reuse-v2-plan-20261016.md).
The withdrawn first screen and its failure remain in the immutable first packet.
Setup now accounts for both independent constructor sites; no global proof cache
or second runtime optimization is introduced.

The candidate removes only repeated document parsing and signature verification
from CheckedWorkerPolicy::revalidate. The original constructor still performs the
complete bounded canonical/schema, external pin, signature, epoch and context
verification before creating the private checked owner. No unchecked document or
copied receipt can construct that owner. It gains no field, retained buffer,
public method, global cache or runtime switch.

## Integrity and live checks

Each revalidate keeps the original ordering: prior refusal, exact current trust
equality (key/body pin/epoch), exact asserted context equality, signed-envelope
size bound and SHA256 comparison against the original envelope commitment. It
then hashes the canonical LF retained projection and requires the original
trusted body pin. The final check preserves detection of deliberate corruption
in the stored projection; simply deleting the old verifier/equality would not.
Every error still latches refusal before any subsequent call can succeed.

Under the same SHA256 integrity assumptions as the existing envelope/body pins,
the successful constructor already proves the unchanged envelope's signature and
schema. The retained projection must still match that admitted body. A changed
envelope, signature encoding, field, trust, key, pin or epoch cannot acquire a
new proof by revalidation. This is local reuse of an original immutable proof,
not refreshed admission or global replay authority. The private original trust,
context and commitments are initialized only by the unchanged constructor.

The standalone host fields remain caller assertions. Real kernel/file/authority
enrollment stays in the enclosing owners. Their source reads, active epoch
views, original packet/receiver checks and before/after brackets remain exact.
No current authority, process/context observation, configuration or deadline is
cached or skipped. The selected source contract compares every live guard file
to baseline and permits only the exact inner substitution in worker policy.
The worker document verifier has no validity-clock predicate to reuse: controller,
OIDC, packet-hold and progress/challenge time checks live in the unchanged outer
chain and still execute. A future time-dependent worker policy would require a
new review rather than inheriting this proof unchanged.

## Differential and mechanism tests

The exact prior revalidate method is retained as a private cfg(test) reference
method. It is not a production switch or exported API. Tests compare typed
errors and sticky refusal with this oracle for 128 seeded input/trust/context/
priority cases (0x0740c0a1), all sixteen retained top-level projection fields and
independent concurrent owners. A fifth test constructs two equal-document owners
and requires two verifications across sixteen revalidations, preserving independent
admission rather than merging their constructor checks. Existing worker-policy,
file/context/credential, signature and projection tests remain unchanged.

The mechanism test starts red at ten full verifications rather than one, then
requires exactly one constructor verification across repeated revalidation and
an intentional projection fault. The stage probe's old positive-call assertion
was implementation-specific. Its preregistered replacement requires exactly two
full verifications in Setup and zero in Fence/Retain/Observe/Publish/Bind, without
changing actions, signatures, bytes, state, file/lock assertions or clocks.
The static gate compares the entire probe to its baseline after restoring only
that one assertion. Registry command/tier/900-second harness budget stay exact;
only its descriptive reason changes. Signed 60-second and progress/challenge
ten-second windows stay unchanged.

The legacy guard marker is updated from parsed-projection equality to the exact
retained-body-pin comparison. All other assertions remain; an independent gate
compares the old gate source after only that marker and module registration are
restored. Trust/context/envelope, body-check removal and unapproved-tail canaries
must fail exact source restoration. This does not normalize arbitrary branches.

## Local comparison boundary

The [preregistered screen](diagnostic-worker-proof-reuse-v2-plan-20261016.md) uses a
separately retained hash-pinned baseline libtest binary and the candidate binary,
five alternating pairs and a fixed paired-bootstrap estimator/floor. Both run
the same real-kernel single-reservation debug fixture. Instrumentation and every
semantic operation are unchanged; the stricter implementation counter is outside
the captured work. A practical diagnostic pass does not become production CPU/op,
native/RESP goodput, release qualification or attribution of earlier failures.
There is no new allocation/retained-memory numerical claim; the checked owner's
fields and lifetime are unchanged and the substitution adds no retained buffer.

Any failed pair or failed practical floor remains evidence and requires reverting
only this runtime substitution. No failed sample may be discarded or threshold
relaxed. A successful ordinary repeat does not establish shared-load robustness
or explain earlier Time/Kernel(Invalid)/IPC Host(Busy) captures. Starting, runtime
handoff, all-writer cessation, real enrollment/installation and server rehearsal
remain separate prerequisites. No product workload, remote infrastructure, ship
admission or push is opened by this candidate.
