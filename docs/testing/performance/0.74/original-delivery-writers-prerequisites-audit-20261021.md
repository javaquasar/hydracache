# Original delivery and writer cessation prerequisites

## Decision and scope

The full original-path SIGKILL/lost-acknowledgement matrix is **not ready** at
source `3107fe33360f97a17ae336f2da6714832924dd68`. Successful preparation and
holder refusal/cleanup cover the existing nonexecuting boundary. They do not
connect the retained original owner to held-unit delivery, barrier release,
active evidence collection or a terminal proof that every writer has ceased.
These are intentionally closed capabilities, not evidence that the current
preparation API starts an unsafe workload.

This audit changes no runtime, Rust tests, protocol, policy, admission predicate
or deadline. The original 300/60/10-second windows and their absolute origins
remain. New checks use ordinary-user, isolated existing unit/property tests;
there is no new privileged/original fixture, manager send, worker execution,
clock/service/idle-policy change, installation, supervisor or server operation.
The [audit packet](local-runs/original-delivery-writers-audit-20261021/README.md)
binds source files, exact existing default-feature images, checks and the
preceding sealed evidence. There is no new performance measurement or claim.

## What is already implemented

| Boundary | Existing mechanism and evidence | What it does not establish |
| --- | --- | --- |
| Original preparation | `JoinedHeldPlacement` consumes fixed placement, retained role files, trust and documents; two full frame/trust/roles/frame rounds; sticky refusal and retirement | No send, release, active collector or completion method |
| Manager identity | `FixedManagerChannel` checks fixed bus path/peer/kernel context, manager/client unique names and PID 1/UID 0 owner; original join pins the receiver and full frame | Read-only channel validation is not delivery |
| Message and reply correlation | Original uniquely addressed Message retains body/serial/stream alias checks; `OriginalReplyState` checks prepared correlation without owning writer FDs | Its public-to-parent surface has only prepare/check; no original arm/transport/observe path |
| Exchange semantics | Private `DispatchExchange` models uncertainty before a possible send, one-shot replies and lost-ACK refusal; MethodReturn accepts only zero/NoReplyExpected flags | Claimed names/private test messages do not authenticate the fixed original sender |
| Root guardians | Original-context-derived Enroll bytes, both guards consumed before first SCM_RIGHTS send, retained root before ACK, two authenticated ACKs, original absolute timer | Root Armed does not prove launcher placement, active collection or closure of manager/external FD copies |
| Revocation mechanics | Pinned cgroup2 kill/event FDs; owner/timer/peer loss invokes permanent revocation, including later population; uncertainty parks while retaining custody | No original terminal/disarm capability or all-writer cessation certificate |
| Launcher mechanics | Standalone inherited-FD helper validates bootstrap/peer/streams/content/guards and performs one fixed execveat after release | No consuming original owner constructs/delivers/releases this helper through the manager |
| Starting durability | Original storage latches the attempt, creates exclusively, syncs creation directory, writes/seals bytes, fsyncs file/directory, then acknowledges | Starting persistence is not a durable send latch, completion, or an admitted later cell |
| Lifecycle model | `DiagnosticCoordinator` already models four cells, uncertain Starting recovery, cleanup and terminal publication over `DiagnosticBackend` | Caller clock/backend observations in this local model are not the kernel-authenticated original path |

Implementation anchors are under `tools/long-run-supervisor-074/src/`:
`diagnostic_consuming_held_join.rs`, `diagnostic_manager_channel.rs`,
`diagnostic_original_manager_join.rs`, `diagnostic_original_manager_message.rs`,
`diagnostic_dispatch_exchange.rs`, `diagnostic_original_root_guardians.rs`,
`diagnostic_root_guardian.rs`, `diagnostic_root_guardian_raw.rs`,
`diagnostic_held_launcher_kernel.rs`, `diagnostic_collector_starting_storage.rs`,
`diagnostic_lease.rs` and `diagnostic_ipc.rs`.

## Blocking integration gaps

### Held unit projection must precede transport

The current original message deliberately reconstructs the unchanged collector
worker projection in `diagnostic_original_pipe_transport::worker`. Addressing
that message to a pinned unique manager does not turn it into a held-launcher
unit. Signed v2/v3/v4 expected launch/placement/role documents require distinct
held semantics and do not upgrade legacy signatures or create a runtime unit.
`JoinedHeldPlacement` exposes only revalidation and retirement, not delivery.

The next bounded implementation should first produce and inspect the exact
held-unit projection from this consumed owner: retained launcher/worker/config
roles, fixed slice and unit names, bootstrap/release endpoints, original stream
identities, guardian identities and original minimum absolute deadline. It must
reject legacy-direct-worker, alternate path/argv, replacement FD/root/channel,
foreign role and renewed-origin substitutions before adding any send method.
No original FD/message extractor or caller-selected unit/callback is needed.

### Original delivery needs its own durable uncertainty boundary

The preparation-only `OriginalReplyState` remains beside the original message.
Private exchange tests already establish useful correlation and sticky refusal,
including compatibility with the allowed Linux MethodReturn flag. That flag
compatibility is implemented; it is not an outstanding blocker.

What remains is the consuming bridge: all original/live/role/root checks,
guardians Armed before first possible delivery, durable at-most-once attempt
record before a transport side effect, exactly one send on the same pinned
connection and bounded reply handling. A job path, correlated manager error,
timeout, disconnect, dropped/delayed ACK or partial send cannot authorize resend,
completion or release. Actual held executable/streams/process start ticks/boot/
cgroup membership must bind to the original admitted context before the one-shot
release. A new owner or reconnect must not clear the old uncertainty.

### Cgroup emptiness is only part of writer cessation

The original descriptor inventory has ten roles: fence and alarm, two pipe
readers, two writable sinks, two original pipe writers and two legacy message
writer aliases. Addressed message preparation adds another pair of writer
aliases. Further copies can exist in transport buffers, queued SCM_RIGHTS,
the bus/manager, held launcher, worker descendants and collector/evidence
processes. These owners are not all inside the attempt cgroup.

| Writer or capability | Existing containment | Missing original terminal proof |
| --- | --- | --- |
| Creator's original, legacy and addressed stream aliases | Opaque owners, checked identity/flags, deliberate drop order | Close every owned alias after the single send/abort boundary without exposing a retry path |
| Queued transport/manager copies | Private message tests show a hidden clone keeps EOF open even after a job reply and owner drop | Account for queues/manager lifecycle and require actual EOF, not assume cgroup.kill closed external copies |
| Held launcher, worker and descendants | Separate pinned root and permanent root revocation mechanics; held helper has a release barrier | Original held placement/process binding, release and recursive revocation on owner loss/hang/deadline |
| Barred collector and custody processes | Original readers/sinks, pidfds, alarm and fail-closed quarantine retaining fence | Active drain/sole evidence writer, bounded bytes/hash reconciliation, file/directory sync and terminal process proof |
| Guardians holding the fence | Independent original timer/owner/peer observation; retained kill obligation | A safe terminal ownership protocol that cannot release custody merely on ACK, first empty observation or caller success |

The current barred collector explicitly performs no stream IO. Its loss/expiry
path closes its four IO descriptors and parks while retaining fence custody.
It is not the active collector required after execution. Guardian `Revoking`
notice is also not a stopped-writers or safe-unlock acknowledgement.

The required release proof is conjunctive: original recursive cgroup empty
under continuing revocation, creator/manager/external copies closed, both
kernel read EOFs after bounded drain, collector byte/hash reconciliation,
all sink/journal/evidence writers terminal, files and directories synced, and
durable completion under retained fence. Timeout or missing proof must keep
the attempt uncertain/quarantined, not fabricate completion or renew a lease.
The kernel hidden-alias tests make this distinction concrete.

### Lifecycle and trust cannot be supplied by the local model

The existing coordinator's four-cell transitions and replay ledger must not
be described as absent. Their integration is absent: no production
`DiagnosticBackend` on this original chain supplies authenticated start,
tree/EOF/receipt cessation and completion facts. Advancing cells 2–4 requires
the same original consumed owner and durable, one-shot transitions; incomplete
Starting/delivery/release attempts cannot be executed again after restart.

Production enrollment remains a separate gate: signed schema inspection,
temporary synthetic role bundles and root inode claims do not establish the
installed policy/epoch, protected issuer/receiver/workflow pins, account or
installed-source receipt. Older campaign/ordinary supervisor paths are not
substitutes for this diagnostic original-owner path. The audit does not change
release status, production policy, the qualification manifest or admission.

## Fault matrix prerequisites and cuts

| Cut family | Current scope | Required before a full original-path pass |
| --- | --- | --- |
| Starting creation/prefix/seal/file fsync/directory fsync/lost acknowledgement | Existing ignored original Starting crash fixture and retained earlier work | Keep its original cuts and refusal semantics; replay separately if composition changes |
| Guardian creation, partial Enroll, first/second/queued Armed ACK, creator death | Existing dedicated root creator/guardian mechanics and consuming holder evidence | Join to exact held delivery, not treat fixture Armed as a start permit |
| Before durable send latch / after latch before syscall / possibly delivered before reply | Not connected to consuming original delivery | Durable attempt receipt, external no-descriptor-owning kill harness, exact no-resend oracle |
| Manager receives rights / ACK lost, duplicated, reordered or late | Private exchange model and socket tests only | Same authenticated channel and original FD owners; retain uncertainty and fence |
| Held ready / before release / release sent but ACK lost / exec boundary | Standalone held launcher/release mechanics, not original owner integration | Actual original process/executable/stream/root binding plus no worker output before release |
| Creator/controller/collector/guardian loss or hang with active writers | Separate custody/root mechanics, no joined active run | Autonomous revocation, recursive descendants, duplicate/outside/queued writer tests, no early unlock |
| First empty cgroup / late population / one EOF / both EOFs before sink closure | Root revocation and hidden-alias local controls | Continuous revocation plus complete IO/evidence writer proof |
| Completion prefix/fsync/directory sync/ACK / recovery / cells 2–4 | Coordinator model, not joined original completion | Durable terminal record only after writer proof; restart preserves identity and refuses re-execution |

Every future cut needs an external exact-pidfd harness, a predeclared mutation,
fixed seed, positive original-path control and retained first failure. It must
check original boot/PID/start ticks/cgroup/root/FD identities, unchanged origins,
all writers and retained records. A failed positive control stops that lane;
there is no retry-to-pass, extended deadline, reset attempt or new admission
from a missing ACK. Existing preparation/fault packets remain immutable.

This audit runs none of these new cuts. Implementation order is held-unit
projection, consuming one-shot delivery and original process/barrier binding,
active collector/all-writer terminal protocol, then joined completion/recovery
and the full preregistered fault matrix. Production enrollment and a separately
authorized server rehearsal remain independent of all local fixture results.

## Verification

Both pinned default-feature GNU/MSRV images pass 32 selected existing local
unit/property controls each: reply flags/state, one-shot uncertainty, delayed
ACK, hidden aliases, owner drop, manager error, root packet claims, placement/
launch signature domains and owner-first retirement. They establish the named
local contracts, not live delivery or production writer cessation. No ignored
test, root fixture, held-launcher executable, service or server is invoked.
The preceding holder packet and its 27 predecessors are independently reviewed.

The source/runtime tree remains exactly at the accepted candidate for these
checks. Source and fixture hashes agree before/after. Formatting, doc/registry,
local non-promotable performance-contract and diff checks accompany the report;
fresh Rust compilation/lint and full release qualification are not claimed for
this documentation-only audit. Evidence stores exact commands, source hashes,
image bindings, unit logs and boolean scope, with no performance estimators.
