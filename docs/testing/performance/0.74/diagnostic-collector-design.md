# Diagnostic collector and original writer cessation

The operator approved the root-owned collector architecture on 2026-10-14.
Baseline is `802d1a20ea66d94df219a8c787803e82fe58e950`. This amendment is
subordinate to P2/P3 in the [intermediate plan](diagnostic-server-readiness-intermediate-plan-20261012.md).
It does not admit a host operation or a product measurement.

## Output ownership change

The old diagnostic unit appends regular files in a worker-owned cell directory.
An empty original cgroup cannot exclude an outside holder of a writable file
descriptor. Changing file permissions does not revoke such an already-open
descriptor on the selected local filesystem. The new architecture gives the
worker only stdout/stderr pipe write ends. A root-owned collector is the sole
holder of evidence-file write descriptors in a root-private directory.

Old direct-output unit projections, signatures, readers and evidence remain
unchanged. A future enrolled collector start needs a distinct signed output
contract and unit projection: it must not reinterpret a v1 regular-file stream
binding as a pipe-to-collector binding. Local fixture outputs use new exclusive
names `collector-stdout.bin` and `collector-stderr.bin`; no production reader
accepts them as a completed cell. No migration or overwrite is allowed.

The separate [collector intent](diagnostic-collector-intent-design.md) now signs
those output requirements under distinct body/envelope/unit schemas and domain.
It is byte-only verification, not a start or a proof of actual pipe/sink ownership.
The root constructor and authenticated producer below remain necessary.

## Collector implementation sequence

The [original pipe preparation](local-runs/original-collector-pipes-20261016/README.md)
now consumes acknowledged Starting without caller descriptors and retains two
distinct actual kernel pipes, nonblocking readers, blocking writers and CLOEXEC.
Implementation `a533e6df` passes 19 joined cases/compiler and preserves the
original authority/time/storage checks. Private pipe fields drop before the
Starting/fence owner, with actual EOF checked after healthy drop. This is not the
root collector constructor or worker handoff. That constructor must consume these
same objects, not replace them with the fixture factory's newly created pipes;
it still needs sole-root sinks, production ancestry and enrolled original start.

The first [original sink composition](local-runs/original-collector-sinks-20261017/README.md)
is withdrawn. Its first server lane stops before RootCreated; a separate fresh
original-chain attribution obtains Causal(Time) with no root created. Kernel
creation success does not establish complete preparation inside the anchored
challenge window. The full candidate patch is retained and runtime restored.
A separately preregistered constant-depth join must prove unchanged live guards,
effect ordering and deadlines before repeating a complete new candidate screen.

The approved [J1 join plan](diagnostic-output-join-plan-20261017.md) now implements
one consuming framed-consent-to-sinks producer with no asserted inputs. Its
effect-local working/closing rounds reuse every original frame predicate, while
the old standalone publishers remain exact. Both compilers pass three healthy
25-stage construction controls with private EOF after drop. The corrected
joined cohort also passes 27 sink and 41 other fault/live/expiry cases on each
compiler; its first red oracle and independent attribution remain retained.
The separate full process-loss gate passes all 25 effects plus AfterReturn in
three repetitions/compiler: 156 SIGKILL, six controls and 324 fresh inspections.
This is process loss, not power loss or exported-writer cessation. The
[new packet](local-runs/original-collector-sinks-join-20261017/README.md) does not
admit original start, sole-writer drain or an installed production route.

The next [original material slice](diagnostic-original-start-material-plan-20261017.md)
consumes those same outputs and inspects the fixed installed bundle under the
retained builder policy. It compares the whole build statement/identity, full
Reserved state digest and collector unit digest, retaining bundle descriptors
with the original Starting/pipes/sinks/fence. Material preparation is not a
unit transport: it exports neither a legacy file-output spec nor descriptors,
and it cannot start or complete a process. Its separate
[framed material gate](local-runs/original-collector-start-material-20261017/README.md)
passes three healthy chains and thirteen exact refusals per compiler within
the original windows. The first ordinary-composition time refusal remains
retained. This closes temporary fixture material preparation, not installed
production ancestry, executable/process start or writer cessation.

First implement the actual bounded pipe-to-file writer and test its kernel EOF
boundary locally. Retain both read ends and exclusively created evidence files;
export only pipe write ends through a fixture-only Stdio transfer. Readers are
nonblocking, writers blocking, and all originals close-on-exec. One pump step
reads at most 8192 bytes from each stream, preventing a hot stream from starving
the other. Keep the existing 8 MiB per-stream and 16 MiB combined ceilings.
Overflow fails explicitly and retains only the bounded prefix; it never produces
a successful drain receipt. I/O, metadata substitution and unexpected poll
events latch refusal. No retry, truncation, unlink or automatic repair follows.

Both streams must return an actual zero-length read after all queued bytes drain.
A hangup event, dead leader, empty read queue or caller boolean is insufficient.
Only then may the original collector sync the files, make them read-only, verify
their retained bytes against the incremental stream hashes, sync
the directory and close its writable descriptors before returning byte counts
and hashes. That result proves local stream drain only, not authenticated start,
successful workload output, cgroup cessation or lifecycle completion.

The first local constructor accepts only an owned private temporary fixture root,
refuses privileged execution and production trees, and creates a new exclusive
attempt directory. It cannot enroll or convert into production authority. The
same writer algorithm will need a separate private production constructor fed
by the acknowledged attempt/fence and trusted root ancestry, not a path or FD
provided over IPC.

Next connect per-packet kernel sender credentials and boot/monotonic time to
independently enrolled issuer/digest/epoch pins. Implement the narrow original
start producer with exact executable, environment, pipe objects, collector,
process generation, manager invocation and retained cgroup. Its watchdog must
remain effective after controller/supervisor loss. Stop/revoke only the original
owned cgroup; drain EOF also detects pipe write ends leaked outside that tree.
Missing EOF or uncertain stop/sync/close leaves an incomplete retained attempt.
All creator/manager copies of write ends must close, including those retained
across systemd RemainAfterExit. Never infer cessation merely from unit-name lookup.

Authenticated completion then joins original start, successful output, original
exit outcome, exact revoked and empty cgroup, and the collector's consumed drain
result. Cells two through four and their new SIGKILL/lost-ack boundaries follow
that producer, not signatures over supplied completion assertions. Production
issuer enrollment still needs operator-selected public keys and digest/epoch
pins; fixture keys are never a fallback.

## Preregistered local tests

Capture the existing process-exit and output-preparation baseline, then a missing
collector red test. Cover binary stdout/stderr, fragmentation, empty streams,
two-stream fairness, exact and overflowing byte limits, a leaked duplicate writer
after original child exit, actual owned child SIGKILL, and late bytes before EOF.
Check exclusive concurrent creation, symlink/hard-link/path substitution,
owner/mode/size/content drift, invalid descriptor events, sticky refusal and
retained partial outputs. Inject failures at file creation, writes, sync,
read-only conversion, explicit close and acknowledgement; no fault yields a drain
receipt. Test compile-time prohibition of forging or promoting the receipt.

Use seed `0x0750` for deterministic fragmentation, with three exact-source
repeats. Process tests own only bounded nonproduct children, with a 30-second
harness deadline and RAII cleanup. These tests do not model power loss, actual
root isolation or production cgroup revocation. A separate ignored owned-child
proof interrupts directory creation/sync, both stream creation/sync, append,
read-only conversion/sync, final directory sync, close, acknowledgement and
after-return lost acknowledgement. Use 14 checkpoint deaths plus after-return,
one uninterrupted control and two fresh refusal inspections for every retained
attempt. At each barrier the parent snapshots retained bytes and metadata before
SIGKILL, then requires unchanged snapshots and original signal-nine termination.
Fresh inspectors cannot recreate the collector or repair its files. Run this
180-second manual gate three times, single-flight without relinking libtest.
Run package tests, current/MSRV
checks, warnings-denied Clippy/Rustdoc, scoped formatting, documentation and
non-promotable performance-contract checks. Retain negative results and raw logs.
Collector overhead must later pass the existing host budget before measurement
admission; no numerical benefit or negligible overhead is assumed.

## Original pipe message preparation

The [separate message preparation](diagnostic-original-pipe-transport-plan-20261017.md)
consumes original material and retains an unsent FD-bearing systemd message.
It checks every fixed typed worker property against the signed collector
projection and uses only the same original stdout/stderr write ends. The two
message-owned duplicate writers close before the original material and fence.
No evidence file descriptor enters that message, and no public extraction,
connection, send or execution method exists.

Fresh full original rounds bracket bundle and message inspection. The immutable
RuntimeMaxUSec requirement is not a refreshed execution lease: the future
authenticated start must derive remaining absolute deadlines and arm an
independent watchdog before sending. Effective executable/environment, manager
invocation, process/cgroup ownership and closure of every manager/creator writer
remain actual-start obligations. Marshalled FD identities and temporary EOF
tests alone cannot fulfill them or publish completion.
