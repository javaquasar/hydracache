# Original tracer thread exit after options

## Preregistered local hypothesis

Baseline source is `cd0e66ba0bd881740808ce8434a9fc5ca95dc5c7`.
The original creating and tracing thread exits through raw `SYS_exit` while a
separate keeper keeps the same original TGID alive. The exact original child
must reach `(CLD_KILLED, SIGKILL)` before that TGID exits at each of six existing
cuts: initial options installed, rooted, exec, installed, ready, delivered.
Initial still overlaps the parent death guard; only the later cuts isolate
EXITKILL after the existing child clears PDEATHSIG on its first resume.

The independent isolated subreaper validates the original child pidfd, retained
original creator task directory, tracing TID, original parent and four root
directories before releasing the cut. It requires the original task to be gone,
the original TGID pidfd still nonterminal, and the original child pidfd terminal.
The keeper reports WNOWAIT only; after normal TGID exit the independent observer
must see, consume and reconcile the same terminal record to ECHILD. Positive
paths never signal the child. Cleanup or a timeout cannot establish success.

## Falsifying and healthy controls

One fixed negative control at the rooted cut removes only EXITKILL, retaining
TRACEEXEC and TRACESYSGOOD. After original thread exit it must observe the
original child nonterminal and detached (`TracerPid: 0`) while its TGID remains
nonterminal. This is a bounded point observation, not a continuous survival
claim. Only then may the observer SIGKILL the already validated original child
pidfd for control cleanup. That terminal record is never positive evidence.
Unexpected natural termination fails this control; predictions are not widened
after observing results. Healthy controls at all six cuts must complete the
same original dummy ACK, stream close and exit zero, with the keeper joined.

Closed private roles and a distinct exact 32 byte frame bind stage, mode and
original TID. Unknown fields, versions, lengths, modes and terminal tuples
refuse. Seeded fresh and independent concurrent observers supplement the six
cuts; existing bounds remain 5 seconds per protocol gap, 30 seconds outer wait
and 8192 captured bytes. These are stall limits, not performance claims.

## Scope and next boundary

This changes only Linux x86_64 libtest fixtures, without keys, tokens, remote
hosts, workloads or public APIs. Real runtime custody, arbitrary threads,
production issuer custody, authenticated original start, P2/P3 all writers,
completion cells 2 through 4 and server admission remain unproven and closed.
The qualification manifest and all prior packets remain unchanged.

Linux documents [per thread tracing and EXITKILL](https://man7.org/linux/man-pages/man2/ptrace.2.html)
and the distinction between [raw thread exit and libc group exit](https://man7.org/linux/man-pages/man2/_exit.2.html).
The fixture tests that distinction on the local kernel rather than inferring it
from whole process loss.
