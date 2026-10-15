# Ordinary selected root one shot dummy transfer

The ordinary selected-self launcher now has a closed local socket profile for
the existing public 16-byte dummy protocol. It composes real root/exec/mappings,
unchanged TSYNC policy, ND0 and parent-only proc readers with an observed
one-shot transfer. It is not a credential producer or authenticated worker start.
See the [preregistration and feasibility correction](diagnostic-signer-owned-root-handoff-preregistration.md)
and [contract](diagnostic-signer-owned-root-handoff-contract.toml).

## Ownership and protocol

The original owner creates an unnamed AF_UNIX nonblocking CLOEXEC seqpacket pair
before its own fork. The ordinary raw child is unchanged: its prepared endpoint
becomes FD3, root/exec descriptors stay above it and all other descriptors close.
The parent immediately drops its child-end duplicate. Only this profile uses a
socket; existing pipe modes keep their narrower behavior. The child checks the
old fixed environment/identity/capability/context prerequisites before any new
fault action. No caller chooses a command, PID, path, FD, payload or filter.

The parent checks original FD3 inode before ND0. Retained live inventory plus
the exact later syscall sequence then excludes descriptor replacement. Neither
a pre-fork SO_PEERCRED value nor a READY record authenticates the child. This
diagnostic relies on the owner's original fork, pidfd and native ptrace events;
temporary same-UID root ownership remains cooperative, not production custody.

Only the new socket profile traces startup, with a finite loader/context-query
allowlist and at most 256 syscall pairs. Channel I/O, replacement, another exec,
process/thread creation and root/namespace/restriction changes refuse before
dispatch. The one exact loaded-marker write is observed from validated stack.
Existing eight-pair installation observation still requires the exact 64 BPF
instructions, successful TSYNC/ND0 and kernel filter increment. No policy change,
new root file, child procfs or inherited parent proc reader is introduced.

After installed stop the accepted sequence is fixed:

1. Native sendto of exact stack READY16 on FD3, MSG_NOSIGNAL, then return16 and
   matching parent receive.
2. Native recvfrom entry with FD3, stack16, MSG_TRUNC and zero address arguments.
   Full original context/maps and empty queue are checked here. Only then may the
   parent latch its sole attempt and send the public dummy.
3. Exact original read return16 and received stack bytes, followed by native
   stack ACK16 and matching receive.
4. Observed close3/return0, full original context/maps with stdio-only inventory,
   exit_group0, original zero terminal status, reap/ECHILD and socket EOF before
   final original root validation and fixed-name cleanup.

There are at most 16 post-install syscall events. Between full-context boundaries,
phase, original endpoint flags/inode, original nonterminal pidfd and exact
entry/exit/stack bytes remain checked. This is one private continuous protocol,
not a reusable cached-context guard. No other operation may dispatch, including
sendmsg, dup2, another READY/ACK or a nonstack pointer.

## Controls and failure behavior

Sixteen child cases include success, malformed/duplicate READY, short/oversized
record, wrong FD, early I/O, leaked FD, missing/wrong/duplicate ACK, close/exit,
ancillary attempt, replacement and rodata. Nine parent controls withhold the
dummy, force send failure, replay readiness and SIGKILL/Drop the original child
at installed/ready/delivered stops. Failed send remains attempted with zero
successful sends; lost ACK may have one send, never a resend. Refusal/control
success is not a successful handoff and is labelled separately.

Failure cleanup signals/reaps only the original child and uses actual datagram
lengths for the unchanged 48-byte drain limit. Zero receive on a live endpoint
is not EOF: HUP is mandatory. Five bounded receives cross EOF under this fixed
writer's maximum four-record trace; zero followed by further data refuses.
This is not an unbounded or caller-provided datagram-stream reader. Unknown
terminal/EOF proof preserves the root through the existing Owner Drop guard.
Drop controls retain original pidfd/parent/name objects for terminal/ECHILD and
root-absence checks, without adopting replacements.

## Feasibility and scope

The first implementation tried to inspect the old static loaded marker with a
stack-only reader. Only the new socket marker moved to stack. Repeated complete
context/root scans then exhausted the existing five-second process guard.
The preregistered correction places full checks at installed/recv/close and
uses the exact intervening trace as the no-mutation proof. All old Owner checks
remain; no timeout, compiler/profile, image bound or policy waiver was added.
Development failures and the temporary diagnostic stderr rejection remain in the
[local packet](local-runs/diagnostic-signer-owned-root-handoff-20261015/README.md).

Eight new units cover closed selectors, budgets and no-retry state, original
endpoint flags, packet lengths/queues, actual failure-drain lengths, finite
startup calls and full-width argument mutations. The ordinary process target
covers all 25 modes; MSRV's extra-libm graph refuses all of them explicitly.

Creator loss at these transfer boundaries is the next separate experiment, not
proved by child SIGKILL/Drop. Pre-arm/running/thread-only loss, trusted credentials,
issuer/job/runtime custody, continuous memory attestation, authenticated worker
original-start, P2/P3 all-writer cessation, completion/later cells, server rehearsal,
qualification and performance claims remain closed. Earlier overlapping-load
timeouts are retained; successful targeted tests do not close load robustness.
