# Original child synthetic handoff

The Linux x86_64 libtest fixture transfers one public 16-byte dummy only after
the original selected-root child has executed the separately compiled fixed GNU
helper, installed the exact namespace policy and become non-dumpable. It is not
a signer credential API or worker start authorization. The
[preregistered protocol](diagnostic-signer-handoff-plan-20261015.md) fixes the
messages, file set, fault cases and refusal bounds; the
[contract](diagnostic-signer-handoff-contract.toml) keeps production admission closed.

## Ownership and ordering

The original thread-bound owner creates an unnamed nonblocking CLOEXEC
SOCK_SEQPACKET pair before its own fork. Only the child endpoint survives at FD3;
the parent immediately drops its duplicate. The selected root and executable are
privately duplicated above FD3 for this profile only. Old default FD closure and
old helper images remain unchanged. Root/runtime validation, original exec stop,
retained environment/live FD inventory, mappings and exact installation reuse
the existing owner rather than accepting a PID, path, descriptor or old guard.

Before ND0, proc FD3 must match the original socket inode. After ND0, retained
live inventory must include exactly stdio and FD3 until the reviewed close.
Every subsequent syscall is traced in a fixed sequence: READY send, dummy read,
ACK send, channel close, zero exit. A descriptor-replacement or ancillary-capable
syscall is refused at entry, before dispatch. Stack bounds and exact bytes are
checked around each send/read. A READY alone cannot release the dummy: the next
original syscall must be the exact bounded read. Only then does the parent latch
its single send attempt, send the dummy, and observe its original read result.
ACK loss or any uncertain transport result is terminal, never retryable.

The private original trace, not SO_PEERCRED or a PID field, establishes who made
the protocol call. A pre-fork socketpair's creation credentials would not prove
the child's PID. Linux documents [socketpair creation](https://man7.org/linux/man-pages/man2/socketpair.2.html)
and [UNIX packet and credential semantics](https://man7.org/linux/man-pages/man7/unix.7.html).
Nonblocking 16-byte reads use MSG_TRUNC to detect an oversized record without
growing storage; [recv semantics](https://man7.org/linux/man-pages/man2/recvmsg.2.html)
distinguish record length, EOF and unavailable data. Only traced sendto is
dispatched; no sendmsg/SCM_RIGHTS operation is admitted or received.

## Refusal and limits

Thirteen cases cover withholding, exact success, malformed/short/oversized or
duplicate readiness, wrong FD, early channel use, inherited FD leak, unexpected
sendmsg/dup2/nonstack bytes, send failure, ACK faults, phase replay, substituted
endpoint/flags, missing context sources, actual child SIGKILL/Drop at installed,
ready and delivered stops, seeded cases and independent owners. Send failure
latches attempted even when no packet was sent. Both post-send ACK failure and
pre-send refusal kill/reap only the original child. Successful ACK is not final
until exact close, stdio-only inventory and original zero terminal status.

The first implementation incorrectly blocked the loader's reads on unrelated
FDs. The corrected startup guard rejects channel operations on FD3 and preserves
the original selected-library validation. That failure and the initial missing
method/moved-enum compile failures remain in evidence; no workload, bound, policy
byte or compiler waiver changes to pass them.

This step proves local synthetic transfer, not actual tracer-loss EXITKILL,
independent root/job/issuer custody, continuous memory/runtime attestation or a
complete sandbox. Actual tracer SIGKILL requires a separate supervised owner-loss
fixture. Real key/OIDC integration, authenticated receiver/replay/fence, worker
original-start, P2/P3 cessation of all writers, completion/cells2-4 and server
rehearsal remain separate. Normal libraries contain no new symbols or consumer;
no dependency, signed/durable schema, reviewed pin or qualification input changes.

The later [ordinary selected-self composition](diagnostic-signer-owned-root-handoff-design.md)
adds a separate binary-private socket profile using the normal root/policy path.
Its 25-mode public-dummy proof does not substitute for this fixture's loss cases
or establish trusted credentials and authenticated worker start.
