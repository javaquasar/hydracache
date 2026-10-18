# Watchdog arming before target enrollment

Baseline is `a44618c2`. Implement the next local mechanics step of the approved
[two-phase plan](two-phase-launch-plan-20261019.md): an actual guardian must be
armed before any target exists, rather than substituting a live sleep pidfd for
watchdog readiness. This package grants no original delivery or execution permit.

The guardian captures its creating process identity from the kernel, pins that
owner, retains the selected absolute BOOTTIME alarm and a real exclusive fence,
and isolates its inherited descriptors before publishing Pending. Atomic
CLONE_PIDFD must pin the guardian; raw child code uses stack and syscalls only.
The parent restores its exact signal mask. No async runtime crosses the fork.

Enrollment is a separate private versioned seqpacket message with one pidfd.
SO_PASSCRED credentials must name the same creating PID/UID/GID, not merely a
copy of the socket. The parent API also retains its creating thread identity.
Validate complete ancillary shape, atomic CLOEXEC, exact message/version and
one real live pidfd. Close rejected received descriptors without signaling an
untrusted target. Bind an accepted pidfd before any Armed ACK can be sent.
Publish sender uncertainty before the enrollment syscall; no missing ACK or
error may allow a second enrollment send.

Once bound, actual owner death, control-channel loss, expiry or an unexpected
second message must trigger SIGKILL through the retained target pidfd. A notice
that a signal was issued is not proof of cessation. Keep the fence regardless
of ACK, target exit or observed pipe EOF; this package has no completion or
unlock path. A dead guardian/host still needs independent custody and durable
history exclusion, not a claim that a final flock owner survives SIGKILL.

Use the existing held-barrier mechanics only in owned nonproduct fixtures.
Require actual Pending, then target enrollment/Armed, before the fixture sends
its release packet. Before release the target must produce no output; after
release it performs a constant pipe effect and parks without monitoring its
owner itself, so owner-loss termination actually depends on the guardian.
Exercise Pending owner death, Armed owner death, channel loss, original expiry,
lost ACK at a controlled stop, foreign sender, invalid rights and repeat refusal.
Retain two additional expected-negative controls after release: killing the sole
guardian must expose a still-live writer and released last-holder flock; stopping
the guardian must expose inability to revoke at the original deadline until it
resumes. Neither observation is an admission success or a reason to refresh time.
An external nonroot subreaper receives exact pidfds with actual credentials,
verifies direct parent/executable identity, and kills/reaps only owned children.

Run baseline and red checks before implementation. Add deterministic codec and
ancillary tests, current/MSRV kernel controls and a separate process-loss matrix.
Freeze all 366 prior supervisor Rust bodies and old compatibility/registry bytes
through exact additive adapters. Keep the old v0 barrier protocol and signed v2
requirements unchanged. Run formatting, strict lint, affected tests, source
contracts, registry, docs and the local non-promotable performance contract.
Retain raw negative results and committed-source/image receipts in a new packet.

This authenticates only a local creating process and private target enrollment.
It does not authenticate the historical admitted owner, complete original frame,
systemd manager, signed executable, original sinks or whole cgroup. It does not
revoke unknown forks/FD copies or enable completion, later cells or reconnect.
Actual original origins and 300/60/10-second windows must remain untouched.
No release installation, supervisor operation, system-bus send, product workload,
dedicated-server operation or expensive qualification is authorized here.

## Local result

Implementation `ddcd8aa89bc235603666e860a6b4f0f8a5996e0b` passes fourteen
ordinary two-phase controls and all twelve new process cases on GNU 1.94 and
MSRV 1.88. The actual guardian retains an accepted target before Armed, revokes
it after creator death and keeps the fence after missing ACK, expiry and EOF.
Existing twelve-case barrier and thirty-control exchange suites remain green.
The full source suite passes 265 controls and restores all 366 previous Rust
bodies exactly; affected lint, formatting, API, registry, docs and local contract
checks pass. The [packet](local-runs/prearmed-watchdog-20261019/README.md) retains
the source/image receipts, preliminary runs and initial strict-lint failure.

The post-release guardian-death control exposes a live target and free last
flock. The guardian-stop control exposes a live writer at the original fixture
deadline, followed by SIGKILL only after guardian resume. These expected
negatives keep independent revocation/custody and durable history mandatory.
The local direct-child admission does not admit systemd-managed targets. Join
retained original kernel/manager/cgroup and stream identities inside the full
frame, preserve the existing collector/custodian holders, and prove every writer
ceased before any completion or release. This package does not open that gate.
