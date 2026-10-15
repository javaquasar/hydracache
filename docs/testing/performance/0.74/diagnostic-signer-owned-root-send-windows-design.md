# Creator loss before send and before receive resume

This local W11 diagnostic adds two fixed creator-side stops to the ordinary
selected-root dummy transfer: after the attempt latch, and after send16 while
the original worker remains at recv entry. Two creator-loss, two healthy and
two no-EXITKILL controls distinguish lifetime enforcement from an untraced
child's apparent success. See the
[preregistration](diagnostic-signer-owned-root-send-windows-preregistration.md)
and [contract](diagnostic-signer-owned-root-send-windows-contract.toml).

## Attempt and dispatch state

An attempt, a dispatch latch and successful send are separate facts. The attempt
latch consumes the only local opportunity. A second sticky latch is set before
the actual send syscall and stays set on failure; it is not a successful-send
receipt. Both methods reject replay. Successful send16 alone increments the
send counter. A failed send leaves attempted/dispatched true and sends zero,
with no continuation that can retry it.

| Cut | Worker stop | Attempt latch | Dispatch latch | Successful sends |
| --- | --- | --- | --- | --- |
| latched 11 | Original recv entry before any send dispatch | 1 | 0 | 0 |
| sent 12 | Same original recv entry, before resumption | 1 | 1 | 1 |

The private continuation retains the same original observed stack pointer.
Healthy H from latched dispatches once and observes the original recv return;
healthy H from sent only resumes recv, never sends again. Both then require the
existing ACK/close/zero-exit sequence, original reap/ECHILD and EOF. Startup256,
protocol16 events, full context boundaries and the raw child/policy remain
unchanged. This is per-lifetime diagnostic state, not a durable permission ledger
or permission recovery after a creator restart.

## Loss and negative controls

The existing monitor retains the original Root/socket objects before its fixture
creator fork. Kernel packet credentials, exact sole-child/tracer/pidfd binding,
two setup descriptors and G/H gates are unchanged. It holds the parent peer
through original creator SIGKILL/reap and never retains a child-end duplicate
across G. Positive loss sends SIGKILL only to that original creator, then requires
the adopted original worker's exact SIGKILL/WNOWAIT/reap/ECHILD before EOF and
root cleanup. Unknown quiescence preserves the root; the unrelated original
pause child must survive until explicit cleanup.

The new no-EXITKILL controls remove only EXITKILL at the same recv-entry stops.
They inject no group-stop and never signal the worker. With no input, the
untraced original child returns from recv without16 bytes and exits74 without
ACK. With the dummy already queued, it reads the dummy, emits exact ACK16 and
exits0. The latter close is not ptrace-observed; terminal socket EOF is checked
separately. Both are negative counterexamples, not healthy completion or an
authenticated transfer. In particular ACK and zero exit do not imply original
creator survival or independent writer revocation. Old group-stop survivor
controls retain their separate mode2 behavior.

## Reset followed by EOF

An unread peer input changes Linux seqpacket termination. The preliminary socket
check and real socket units observe one ECONNRESET before zero/HUP when the peer
closes without consuming the dummy. A reset alone never proves EOF or process
death. The existing five-read, 48-byte drain body is unchanged and still refuses
that reset.

Only sent-loss may consume exactly one ECONNRESET prefix, and only sent mode3
may consume exact ACK16. Other dispositions allow neither prefix. A bounded
peek must then return zero without an extra record, followed by the unchanged
drain's mandatory zero/HUP proof. Missing/wrong/duplicate ACK, unexpected reset,
other errors, live zero-length records or EAGAIN refuse. Prefix budget is one
read, peek budget one read, followed by the existing five reads. Nothing is
reopened, retried, read in an unbounded loop or silently discarded as success.

## Verification and scope

Four new units cover sticky dispatch/replay/EPIPE, exact reset versus EOF,
single ACK versus missing/wrong/duplicate/oversized records, and closed-mode/live
peer refusal. Existing cut-state and frame-bit mutation units cover new facts
and selected frame pairs. The six new process cases follow the old25 and old14
cases inside their existing test; no additional competing fixture is introduced.
MSRV's extra-libm image must explicitly refuse all six, without a graph exception.

The [local packet](local-runs/diagnostic-signer-owned-root-send-windows-20261015/README.md)
keeps baseline, red, transport feasibility and current checks with source/frozen
input pins. It is unsigned consistency, not execution attestation. Earlier
parallel-fixture timeouts and MSRV macro lint failures remain open and retained.

Two fixed boundaries do not prove arbitrary-instruction or in-syscall creator
loss, restart/recovery, pre-arm/running/thread-only loss, fresh creator CLI exec,
hostile same-UID custody, trusted credentials/runtime/issuer/job custody,
authenticated worker original-start or P2/P3 cessation of every writer. Later
cells, production output, server rehearsal, qualification and performance claims
remain closed.
