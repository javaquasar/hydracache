# Creator loss at ordinary dummy transfer stops

This local W11 diagnostic checks the ordinary selected-self root, policy and
public-dummy transfer together. It covers six stopped boundaries, with six
healthy continuations and two no-EXITKILL survivor controls. The
[preregistration](diagnostic-signer-owned-root-handoff-loss-preregistration.md)
and [contract](diagnostic-signer-owned-root-handoff-loss-contract.toml) fix this
scope. It is not original-start authentication or production writer revocation.

## Original objects and transport

The monitor retains the original selected Root and creates the unnamed socket
pair before its sole-main fixture creator fork. The creator supplies that fixed
transport to the same private Owner construction body; ordinary modes still
create their own transport at the same point. The raw worker child, namespace,
root/exec transitions, policy installer and child protocol are unchanged.

Setup transfers the original worker pidfd and parent endpoint through the
existing kernel-credential-checked channel. The monitor checks original creator,
sole child, tracer and pidfd links. It compares the received parent endpoint to
its retained original parent and the child's initial O_PATH FD3 metadata to its
retained original peer. Opposite socket endpoints must have different inodes;
equal pipe-style identities would be the wrong oracle. Socket type, AF_UNIX,
seqpacket, nonblocking read/write and CLOEXEC are checked on owned endpoints.
After this check the monitor closes both extra pair copies, including the child
endpoint, before G; the transferred original parent duplicate stays live until
creator terminal/reap. A retained child endpoint must never mask worker EOF.

## Cuts and controls

| Cut | Original observed stop | Successful sends at cut |
| --- | --- | --- |
| installed | Completed original policy installation | 0 |
| ready | Exact READY send exit and parent receive | 0 |
| receive | Exact recv entry, full context/maps and empty queue | 0 |
| delivered | Original recv exit16 and exact stack dummy | 1 |
| acked | Exact ACK send exit and parent receive | 1 |
| closed | Observed close3 exit0 and stdio-only full context | 1 |

Each loss kills only the original creator with SIGKILL. The monitor requires
exact creator terminal/reap and adopted original worker SIGKILL/WNOWAIT/reap/
ECHILD, then bounded socket EOF before original root cleanup. There is no worker
SIGKILL in a passing automatic-death path. An unrelated original pause child
must remain nonterminal through loss and EOF before explicit cleanup. The
temporary subreaper setting is restored; unknown quiescence preserves the root.

Installed and delivered loss/negative pairs use a proven group-stop. Delivered
queues SIGSTOP while at the original recv exit, resumes only to its signal
delivery stop, reinjects SIGSTOP and requires GETSIGINFO EINVAL. Removing only
EXITKILL at the matched negative cut must leave the worker adopted, untraced,
nonterminal and State T. The monitor explicitly kills it only after that proof;
this is negative cleanup, never automatic-death evidence. Other loss cuts remain
ptrace stops and establish nothing about a running worker.

Healthy H continues the original private protocol to ACK/close/zero exit and
ordinary reap/ECHILD. The receive-entry continuation holds the original observed
stack pointer locally, not an external address. No second read-entry resume,
send retry or fabricated payload API is added. The existing sticky attempt
latch, 256 startup pairs, 16 protocol events, 48-byte drain, four packets per
direction and five-second wait/outer guard remain fixed.

## Verification and remaining limits

New units reject foreign/swapped/equal socket endpoints, wrong endpoint flags
and cut step/attempt/send/close drift. Existing frame-bit mutation tests now
cover all 14 new modes. Process coverage runs all six losses, six healthy paths
and two survivor controls, plus environment and extra-argument refusal. MSRV's
extra-libm image must refuse all 14 modes under the unchanged four-role graph.

The two unit-only rights-cleanup EOF oracles now use an exec'd copy of the same
unit-test binary, selected by exact test name. The outer harness keeps default
parallelism and the existing five-second child guard. This prevents sibling
forks from extending a fixture writer's lifetime; no Channel code, rejection
condition or EOF assertion changes. A separate gated child proves the inheritance
mechanism with WouldBlock before release and EOF only after exact original exit/
reap. Retaining actual received rights still prevents EOF, and a successful child
with zero selected tests is rejected. See the
[fixture experiment](local-runs/scm-rights-eof-isolation-20261015/README.md).
The writer owner in the historical EAGAIN capture remains unknown.

The initial standalone matrix passed. Adding it as another parallel root fixture
coincided with an old handoff five-second timeout. The retained red is not fixed
by the final scheduling correction: the 14 cases now follow the 25 old cases in
their existing process test, keeping the baseline number of concurrent fixtures
and the default parallel harness. No scheduler trace attributes the timeout;
overlapping-load robustness remains open.

The [local packet](local-runs/diagnostic-signer-owned-root-handoff-loss-20261015/README.md)
records execution logs and historical source consistency, not signed execution
attestation. These six cuts bracket, but do not interrupt, the creator between
latching an attempt and calling send or after successful send before resuming
the original recv. The separate
[send-window composition](diagnostic-signer-owned-root-send-windows-design.md)
adds two fixed stops there; no permission recovery/restart is tested by either.

These cuts do not prove pre-arm/running/thread-only loss, a fresh
creator CLI exec or hostile same-UID custody. Trusted runtime/issuer/job custody,
real credentials, authenticated worker original-start, P2/P3 all-writer cessation,
completion/later cells and server admission remain separate. No product workload,
qualification, throughput or allocation claim follows.
