# Two phase launch and writer revocation

Baseline is `db35fdc3`. The existing collector projection executes the worker
directly through `StartTransientUnit`. It cannot be reused to promise a held
launcher: changing its meaning would invalidate existing signatures. The
approved replacement therefore needs a separate signed schema and domain.

The first package defines signed expected requirements, not execution authority.
It binds the original start-consent claims, launcher, watchdog and collector
artifact hashes, and a deterministic two-phase requirements digest. The old
consent remains a nested historical claim, not permission to use a different
unit. Old readers must reject the new envelope. Supplied-key inspection must
not manufacture enrolled trust, actual peer credentials, a live clock, a cgroup
or an original-owner capability.

The required order is independent watchdog armed, held-launcher delivery,
original executable/process/streams/cgroup binding, complete live validation,
then one-shot barrier release. Neither a job reply nor a release acknowledgment
proves execution or cessation. A lost acknowledgment is an indeterminate
attempt, not permission to resend. The watchdog must revoke the pinned complete
cgroup and every known external writer; both actual pipe EOFs after drain and
exclusive collector shutdown precede fence release. Hidden manager or creator
writer copies must block release even after the cgroup is empty.

No new relative lease is introduced. Integration must use the same authenticated
receive origin, reservation origin and cell origin, retain all live predicates,
and select the minimum of their original absolute 10, 300 and 60 second bounds.
Signed timestamps are claims only. They cannot replace actual observations or
refresh any bound. The original receiver remains on its pinned thread; a raw
post-fork child must not enter its inherited async runtime.

## Implementation and checks

1. Add the versioned data-only requirements builder and strict canonical
   signature inspector. Cover domain separation, all nested/artifact binding,
   malformed and over-budget input, schema rejection and old-reader rejection.
   Freeze every existing supervisor Rust source and old schemas through an exact
   additive registration adapter. Retain baseline and red receipts first.
2. Develop the held launcher and pre-armed guardian as owned nonproduct kernel
   mechanics. Cover owner/guardian death, malformed release, unchanged absolute
   expiry, lost ACK and no output before release. This is not yet production
   executable or all-cgroup admission.
3. Join the new artifacts to original live admission, channel, negotiated FD
   delivery, collector custody and pinned complete cgroup revocation. Preserve
   every complete validation frame and original bound. No byte verifier or
   fixture may substitute for this join.
   Consume the same original durable attempt rather than granting a second v2
   replay slot; old and new protocols must never execute that attempt in parallel.
4. Exercise real SIGKILL and lost ACK at every new durable/send/release/close
   boundary, then completion and later cells. Retain incomplete attempts and
   fence ownership after uncertain delivery. Verify monitoring reconnection
   against the same boot, PID/start ticks and cgroup before server admission.

Use Windows and GNU current/MSRV checks, strict lint, source contracts,
formatting, documentation and registry checks in proportion to each package.
Record exact commands and raw receipts in a new local evidence directory.
Implementation, tests and evidence may share a commit where the tests define
the new schema; keep the plan and measured result separate.

This work authorizes no release installation, stable supervisor operation,
system-bus send, product execution or expensive qualification. Server rehearsal
and any production enrollment remain separate gates.

## First local package result

Source `fab1798c74f8a579184ec19893c2c76472421cbe` implements step one and owned
barrier mechanics from step two, not a production held executable or an armed
guardian. Both GNU 1.94 and MSRV 1.88 pass nine ordinary controls and the same
twelve owned kernel cases, including real SIGKILL before release and after
the constant pipe effect but before ACK. Failed send and lost ACK keep the
same sender consumed. Existing exchange and hidden-writer controls remain green.

The first healthy barrier's incorrect zero-flags predicate and two source-test
adapter failures remain in the [local packet](local-runs/two-phase-launch-20261019/README.md).
Only the new barrier's echoed CLOEXEC bit handling and exact test registrations
were corrected; no original predicate, bound or assertion was relaxed.
The final full source suite passes 262 controls, preserving all 362 old Rust
sources and old schema/registry bytes. Current/MSRV checks, strict lint,
Windows controls, documentation, registry and the local contract pass.

Next is actual pre-armed guardian enrollment, complete cgroup/external-writer
revocation and the original artifact/channel/frame join. The private v0 packet
is an ephemeral mechanics prototype, not the future production release protocol.
Neither these data types nor kernel fixture results open original start,
completion, later cells, reconnect, installation or server admission.

## Prearmed guardian mechanics result

The [next local packet](local-runs/prearmed-watchdog-20261019/README.md), source
`ddcd8aa89bc235603666e860a6b4f0f8a5996e0b`, adds actual Pending before target
creation and creator-authenticated one-shot enrollment with target retention
before Armed. Both GNU compilers pass the same twelve owned process cases,
including lost ACK and actual creator SIGKILL. This advances step two only;
its ambient-procfs/direct-child admission is not the original-frame/cgroup join.

Post-release death and SIGSTOP of the sole guardian expose live writers without
timely revocation; death also releases its final flock. Preserve those negative
results and existing original collector/custodian owners while implementing
independent all-writer revocation. Original delivery, durable completion, later
cells, reconnect and production/server admission remain subsequent gates.
