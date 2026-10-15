# Ordinary root creator process loss at owned stops

Base `72b4ba1e` retains ordinary root/policy composition and its earlier negative
results. Add fixed local normal-build loss diagnostics, not a public launcher,
credential handoff, server operation or product workload.

## Experiment and ownership

Use a fresh sole-main nonroot monitor with empty environment, default reapable
SIGCHLD, no inherited extra descriptors and no existing children. It creates the
selected root first and retains its original objects through a fixture-only fork
of the creator. The creator uses the existing ordinary Owner/build/transition/raw
child implementation and real selected self/runtime. This outer creator fork is
single-threaded fixture orchestration; it is not a fresh exec of the creator CLI.
The actual worker's raw post-fork code remains allocation-free and unchanged.

A private kernel-credential-checked seqpacket channel reports the creator's sole
original child, transfers that child's original pidfd and pipe read descriptor,
and gates progress. No supplied PID/path/FD is adopted by an existing production
API. The monitor validates actual parent/tracer/sole-child and pidfd identity,
and holds its endpoint until exact creator termination/reap, never using early
EOF as a creator-death signal. Root cleanup is delegated to the monitor, which
owns the original tree from creation, not reconstructed from reported paths.

Kill only the original creator via its owned pidfd at five stopped boundaries:
initial after options, rooted, first exec, loaded and installed. Require creator
SIGKILL/reap and adopted original-child SIGKILL via pidfd waitid/WNOWAIT, exact
reap/ECHILD and private pipe EOF before rechecking/removing the root. Passing loss
proof never signals the child. Two acknowledgement-gated healthy controls at
initial/installed exercise ordinary original-child termination instead. Retain
an unrelated original live child and prove it unaffected before explicit cleanup.

At installed loss, use GETSIGINFO/SIGSTOP delivery, reinjection and exact EINVAL
to establish a kernel group-stop. Apply the same stop to a negative control with
EXITKILL removed. After creator loss that control must remain nonterminal with
TracerPid0 and State T; its later explicit original-child kill/reap is cleanup,
never positive loss proof. Do not accept running, tracing-stop or natural EOF as
the positive terminal oracle.

## Fixed bounds and checks

Keep the four-image graph, 8 MiB image limit, maps/status 65536 bytes, maps128,
five-second waits and existing five-second process outer guard. Private setup and
cut frames are 16 bytes, setup carries exactly two descriptors, cut carries none;
unknown/truncated/duplicate/foreign-credential input refuses. Fixed one-byte gates
carry no authority. At most four packets per direction, 128 ancillary bytes and
48 failure-drain bytes; no bound increases to gain acceptance. Only this fresh
monitor enables/restores subreaper state. Unknown child/stream quiescence preserves
the original root rather than deleting it. Failure cleanup may signal only owned
originals and is never counted as passing automatic-death evidence.

Record the current ordinary 43-test baseline and new fixed-mode red first. Add
normal-process positive/healthy/negative cases, parser/credential/FD/terminal model
mutations, targeted default-parallel repeats, current/MSRV check/lint, old loss and
ordinary regressions, static contracts and unsigned evidence verification. MSRV's
extra-libm normal image continues to refuse the fixed graph. Preserve failures.

## Not established

No pre-arm, running-child, creator-thread-only loss, fresh creator-CLI exec,
all-boundary crash recovery or hostile same-UID custody claim. Constructor early
failure before original-handle transfer is conservative refusal, not a completed
monitor proof. No protected/dummy/real credential handoff, authenticated worker
start, P2/P3 all-writer cessation, completion/later cells, server/GitHub operation,
qualification, performance or ship admission. Isolated successes do not resolve
the retained overlapping-broad-load timeouts.
