# Ordinary selected-root creator loss at owned stops

This binary-private diagnostic composes the existing ordinary root/policy launcher
with a monitor that survives creator process loss. It closes only five stopped
boundaries, not full original-start authentication or production writer revocation.
See the [preregistration](diagnostic-signer-owned-root-loss-preregistration.md)
and [contract](diagnostic-signer-owned-root-loss-contract.toml).

## Original ownership and ordering

The fresh, empty-environment, nonroot monitor must have no children, default
reapable SIGCHLD and no extra inherited descriptors. It creates the selected root
and retains the original tree objects **before** its fixture-only single-threaded
creator fork. This outer fork is not a fresh exec of the creator CLI. The creator
uses the unchanged ordinary Owner/build, raw worker child and root/policy methods.
There is no public API to signal or adopt caller-supplied processes or paths.

The private nonblocking CLOEXEC seqpacket pair has SO_PASSCRED enabled before fork.
Setup carries the original worker pidfd and original pipe reader. The monitor
checks per-packet kernel PID/UID/GID, exact original creator/sole-child/tracer links,
pidfd fdinfo and pipe identity against an O_PATH metadata handle of the initial
child's FD3. It never opens that FIFO for I/O. The worker is stopped before the
namespace transition, allowing this identity check before no-dumpability. Only
then may the G gate advance the original owner. A fixed cut frame carries no FDs.

The monitor kills only the original creator using its retained pidfd. Its endpoint
stays open through exact creator SIGKILL/reap, so channel EOF cannot accidentally
release an earlier boundary. As a temporary subreaper, it observes the adopted
original worker via the transferred original pidfd: exact SIGKILL/WNOWAIT,
reap/ECHILD, then private pipe EOF. Passing automatic-death paths never signal the
worker. Only after these proofs, plus explicit unrelated-child cleanup, may the
original root be checked and removed. The monitor restores its own subreaper
setting; it refuses a pre-existing enabled setting rather than assuming custody.

If the proof fails, cleanup may signal only owned originals. Creator cleanup
precedes worker cleanup to allow adoption. Such cleanup never turns refusal into
positive evidence. Unknown quiescence suppresses automatic root deletion. Failures
before setup transfer are conservative refusal, not a completed monitor proof.
The unchanged Owner retains its existing 48-byte failure-drain limit; the monitor
does not claim a successful failure-path EOF drain.

## Matched controls and bounds

Five positive cuts are initial after ptrace options, rooted, first exec, loaded
and installed. Healthy controls at initial and installed acknowledge H and let
the original creator terminate/reap its own worker. At installed loss, delivery
SIGSTOP is reinjected and GETSIGINFO must return EINVAL to prove a group-stop.
The negative control uses that same group-stop but removes EXITKILL. After creator
death it must remain nonterminal, adopted, untraced and State T. Its subsequent
explicit worker kill is negative cleanup, never automatic-death evidence.

A separate original pause child must remain live across loss and EOF. Four fresh
sequential installed-loss monitors also check unchanged caller root, cwd, user
namespace and UID/GID. These are not simultaneous creator-loss experiments.

Frames are 16 bytes/version 1; setup has exactly 2 FDs, cut 0, gates 1 byte/no FDs.
Four packets per direction, 128 ancillary bytes, five-second waits and the existing
five-second per-process outer guard remain fixed. Unsupported runtime shape,
credentials, flags, payload length, descriptor count or mode refuses. Received
rights enter RAII before rejection, including MSG_CTRUNC; EOF-based tests detect
leaked duplicate pipe writers. No root graph, image/map/status bound or policy
exception changes to pass these tests.

## Scope of evidence

The [local packet](local-runs/diagnostic-signer-owned-root-loss-20261015/README.md)
retains baseline, red, development and final checks. It is unsigned consistency
evidence, not execution attestation. MSRV's extra-libm image must refuse all eight
modes under the unchanged four-role graph; passing that test is not runtime support.

Not proved: pre-arm loss, running worker, creator-thread-only death, fresh creator
CLI exec, hostile same-UID custody or the retained overlapping-broad-load timeouts.
The subsequent [ordinary dummy transfer](diagnostic-signer-owned-root-handoff-design.md)
composes the fixed public protocol, not trusted credential handoff or creator loss
at transfer cuts. Independently authenticated runtime custody, real credentials,
authenticated worker original-start, all-writer cessation, completion/later cells,
server rehearsal, qualification and performance claims stay closed.
