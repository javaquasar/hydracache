# Intermediate plan for the diagnostic server pilot

This is the W11 completion sequence approved on 2026-10-12, subordinate to the
[0.74 release plan](../../../plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md)
and [repository rules](../../../RULES.md). It is not a replacement release plan,
qualification amendment or product activation. Starting source is
`ceee4bc7b5b45a7dcb7cc0f018a8e74bb9fbbea8`. Initial Reserved publication and
cell-one consumption are locally implemented; later lifecycle and execution
remain closed. The [readiness audit](production-readiness-and-server-launch-20261011.md)
describes the existing primitives and host prerequisites.

## P0 Actual process interruption and lost acknowledgement

Run bounded local Linux nonroot subprocess fixtures. Parent owns every temporary
directory and exact child handle; kill only that child with SIGKILL at logical
write/sync checkpoints. Cover all eight Reserved-publication stages and all
14 consumption stages, plus death after each successful API return. Include
uninterrupted controls. Confirm actual signal death, original lock contention
before death and lock release after death, unchanged retained bytes/metadata,
and refusal of existing context in a fresh read-only process. No crash attempt
may start a replacement writer, repair data, append a refusal after the owner
died, or reconstruct an acknowledged runtime guard. A before-create crash has
no durable mutation; empty-fence eligibility there is not global nonce replay
protection. Track that earlier lifecycle boundary in P1.

Use seed `0x074d`, three repeats and separately registered ignored/manual proof
gates. Bounded readiness timeouts diagnose a stuck test harness; they are not
correctness or performance thresholds. Actual process loss does not prove
power-loss durability, host reboot recovery or installed-root enrollment.
Exit gate: complete named crash/control matrix, no surviving owned helper,
fresh-process observations and an independently verified exact-source packet.
P0 starts immediately; it opens no service route or server workload.

## P1 Fixed lifecycle and remaining cells

Preregister transitions for cells two through four using the original authority,
fence and retained history. Define legal Reserved/Starting/running/terminal
transitions and exact state revision, completed-cell and previous-record rules.
Durable intent precedes any future execution. Refuse duplicate cell/request/nonce,
partial pairs, uncertain acknowledgement, stale boot/epoch, unknown context and
changed original objects. Reopening history is inspection, never permission to
replay. Specify crash before the first durable intent and reboot handling without
claiming durable revocation from an in-memory latch.

Exit gate: transition/property/concurrency/regression tests, fault matrix at
each new write boundary, no overwrite/adoption/retry, and compatibility review
for any durable change. P0 success alone does not close P1.

## P2 Trusted peer time and external authority

Connect peer credentials from the original kernel IPC context rather than caller
assertions. Bind controller provenance, admitted host/boot/namespace/account
context, bounded freshness checks and monotonic lease/progress authority. Never
use wall-clock time as epoch/version authority. Load independently selected
production issuer keys, digest and epoch pins from reviewed trusted ownership;
never enroll fixture keys, synthetic identities or envelope-supplied trust.

Exit gate: valid original-context admission and wrong-peer/signature/epoch/boot,
clock-change, replay and substitution refusals, with no secrets in evidence.
Selecting real operator keys/pins is a required operator input before enrollment.
Whole-host snapshot rollback remains outside the selected local authority model;
adding an external monotonic authority would require a separate decision.

## P3 Production output start watchdog and cleanup

Prepare root-owned fenced production output for the exact reserved cell, with
exclusive creation and retained ancestry/stream identities. Bind the reviewed
loaded executable, actual environment, original streams, process generation,
unit and cgroup to the signed intent. Wire only the narrow typed diagnostic
backend; expose no arbitrary command/path/descriptor or fixture conversion.
Implement autonomous lease/progress watchdog, writer revocation and uncertain
terminal retention. Release requires original writer cessation and proof that
the exact owned cgroup is empty, not PID-only liveness or a replacement process.

Exit gate: original-start and bounded-write tests; cancellation, supervisor/
controller loss, hang, lease expiry, replacement and cleanup-failure canaries;
no automatic restart or incomplete attempt promoted to a successful receipt.

## P4 Reviewed installation and dedicated-host preflight

After P1 through P3 pass, freeze the exact tooling source/build/config identities
and signed bundle. Verify installed source receipt and signatures, artifact
hashes, trusted directory ownership, actual nonroot UID/GID/group set, systemd/
cgroup-v2 capabilities, filesystem/space limits and admitted host/boot context.
The deployed worker-owned campaign layout requires a reviewed quiescent ownership
and coordinated binary rollout; do not silently chmod/chown it into eligibility.

Host inspection starts read-only. An active campaign, diagnostic/pending marker,
lifecycle fixture or uncertainty blocks maintenance. Any separately approved
maintenance stop targets only `hydracache-performance-supervisor-074.service`,
with explicit restoration, receipt and health verification. Do not interrupt
0.73 or unrelated campaigns. Exit gate: exact-host positive/refusal preflight and
installation/restoration evidence; local modeled roots cannot satisfy it.

## P5 Nonproduct host lifecycle rehearsal

Run a bounded separately reviewed nonproduct positive lifecycle followed by fault
cases: signatures/pins, revoked epoch, task/namespace/credential churn, original
object substitution, incomplete/overflow output, manager/controller/supervisor
loss, stalled progress, lease expiry, uncertain journal and incomplete cleanup.
Host reboot testing needs its own quiescent maintenance approval. Retain every
attempt, prove zero duplicate executors, writer cessation and original empty
cgroup, then restore and verify the stable supervisor.

Exit gate: independently verified host packet and acceptable controller overhead.
Only then define and separately admit a finite numerical pilot with frozen
workload, payload/keys, seed, concurrency, duration, estimator and thresholds.
P5 is not a product candidate run, six-hour qualification or 24-hour confirmation.

## Product release remains a separate boundary

[Composition](composition-ledger.toml) currently has zero accepted product
candidates and C74 is unresolved. [Release admission](release-admission-contract.toml)
remains closed. A product release still requires an admitted measured candidate,
exact I74/C74/Redis/runner identities, native nonregression and compatibility,
focused pairs, independently approved six-hour/24-hour stages, exact packages,
supported targets, dependency/SBOM receipts and immutable nested-hash archive.
The inherited plan/qualification digest mismatch needs explicit reviewed
reconciliation before qualification. No frozen input or threshold is changed by
this intermediate plan. Passing P0 through P5 does not itself close release 0.74.

## Evidence and execution cadence

For every package retain baseline, red test, implementation, targeted repeats,
checks/lint/format and proportionate regression. Use small logical commits and
separate evidence; update this checklist's status with the exact packet, never
with a readiness score. Full verification is required at milestones and before
merge/tag. Keep historical negative results immutable and distinguish local
functional, real process-loss, exact-host and numerical qualification evidence.
