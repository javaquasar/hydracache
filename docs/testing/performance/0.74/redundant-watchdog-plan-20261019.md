# Redundant local watchdog custody and revocation

Baseline is `6f1dbfab`. This continues step two of the approved
[two-phase plan](two-phase-launch-plan-20261019.md), not original execution
admission. The previous prearmed experiment exposed a live target and free
last-holder flock after guardian death, and a live writer at the original
deadline while the guardian was SIGSTOPed.

Compose exactly two existing private Watchdog owners without changing their
protocol or implementation bodies. Both must publish actual Pending before
target creation, retain aliases of the same exclusive fence, and use exactly
the same selected absolute BOOTTIME deadline. Each independently authenticates
the same creating owner and enrolls the same direct-child pidfd. Consume the
pair's uncertainty latch before the first enrollment send. Require both Armed
acknowledgements and an additional closing live check before a test fixture may
release its unchanged barrier. Any partial enrollment, dead peer, missing ACK
or retry poisons the pair. No new send attempt, replacement guardian or deadline
renewal is permitted.

Each guardian independently revokes the retained target on owner loss or that
same original deadline, even when the other guardian is dead or stopped. The
remaining holder must keep the original fence. This does not detect a stopped
sibling immediately, and a single failure while the creator remains live need
not revoke before the original bound. No new detection-time promise is made.
No completion, unlock, executable, cgroup or production constructor is added.
Keep existing original collector/custodian owners unchanged.

Before implementation, retain the old ordinary controls and twelve-case fault
matrix, including the negative controls, then an expected missing-module red
check. Add ordinary pair-latch/thread/closing-check controls. Use actual creator
SIGKILL, pidfds, credentials, terminal status and pipe EOF in a new owned nonroot
external process driver. Reuse old test helpers only through an exact additive
test-module registration; do not edit previous fixtures or receipts.

The process matrix must cover healthy expiry and owner loss; death or stop of
either guardian followed by owner loss; death or stop of either guardian while
the creator stays live through the unchanged deadline; lost second ACK with
the first guardian dead/stopped; partial enrollment failure and retry refusal;
global duplicate refusal; and a hidden external write alias. Preserve expected
negative controls for both guardians dead (live writer/free final flock) and
both guardians stopped (live writer at expiry until one resumes). These are
fault-model limits, not permission to open production execution or weaken time.

Freeze all 369 previous supervisor Rust bodies and existing protocol/registry
and imported driver bytes with exact additive adapters and mutation controls.
Run current GNU 1.94/MSRV 1.88 source-bound cohorts, old barrier/exchange tests,
strict affected lint, formatting, Windows/API controls, full source contracts,
registry, documentation and local non-promotable checks. Keep raw failures and
source/image receipts in a separate closed evidence packet. Update the article
with measured behavior and remaining boundaries, not performance claims.

Ambient procfs/direct-child admission still cannot admit systemd-managed targets.
The pair is a local single-guardian-failure experiment, not a production topology
decision or proof against common-mode failure, host death, unknown writers or
all-cgroup escape. Original kernel/manager/frame/streams binding, independent
all-writer revocation, durable exclusion/completion and reconnect remain later
gates. All original live checks, 46 effect brackets and 300/60/10-second origins
remain unchanged. No server, system-bus/original send, release installation,
supervisor operation, product workload or expensive qualification is permitted.
