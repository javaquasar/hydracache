# Original watchdog arming boundary

Audit baseline is `1fa0a920fe09919f235cdc0f684fd0e281d2c3f9`. Original Pending
custody is implemented, but the transition to Armed needs a production revocation
topology decision. This audit does not select one or authorize a host operation.
It inherits R-3, R-4, R-7, R-8 and R-10 from `docs/RULES.md`.

The operator subsequently approved a separate attempt cgroup contour and new
enrollment protocol on 2026-10-19. The [local implementation plan](cgroup-root-enrollment-plan-20261019.md)
supersedes the pending decision, not the baseline findings below. Approval does
not authorize release installation, server operations or a production workload.

## Existing authority and the missing transition

| Retained mechanism | What it establishes | What it does not establish |
| --- | --- | --- |
| Original Pending owner | Same original borrow, fence/timer OFDs, two authenticated Pending children and unchanged full frames | A target, Armed or a revocation root |
| Private prearmed guardian | One live direct-child pidfd retained before its Armed acknowledgment | Admission of a systemd-created process or complete cgroup revocation |
| Signed v2 launch requirements | Separately signed expected artifacts and an explicit ordering contract | Enrolled issuer selection, an opened launcher or a live execution permit |
| Process and tree readers | Retained kernel generation and bounded tree consistency against expected input | Authenticated original-start provenance, permission to kill or proof of all-writer cessation |
| Original manager message | Original fixed channel, receiver, addressed aliases and full validation | v2 held-launcher delivery; it still binds the v1 worker projection |

The code boundary is concrete:

- [The unchanged guardian](../../../../tools/long-run-supervisor-074/src/diagnostic_prearmed_watchdog.rs)
  requires the target's actual `PPid` to equal the creating owner and signals only
  that retained target pidfd. Removing that predicate would turn arbitrary
  received pidfds into cleanup authority. A process created by systemd is not a
  direct child of this original owner.
- [The Pending adapter](../../../../tools/long-run-supervisor-074/src/diagnostic_inherited_pending_watchdog.rs)
  has no enrollment method and reuses that unchanged guardian loop. Borrowing the
  original fence/timer does not change the target admission rule.
- [The v2 requirements](../../../../tools/long-run-supervisor-074/src/diagnostic_two_phase_launch.rs)
  require `independent-watchdog-armed-before-first-delivery`, followed by held
  launcher delivery, complete process/stream/cgroup binding, full validation and
  one-shot release. The old signature cannot authorize a different order.
- [The fixed tree scope](../../../../tools/long-run-supervisor-074/src/diagnostic_tree.rs)
  currently names a per-cell service under `system.slice`. Its read-only tree
  constructor requires an independently authenticated expected root identity;
  `empty_at_read` explicitly grants no release authority.
- [The original material](../../../../tools/long-run-supervisor-074/src/diagnostic_original_start_material.rs)
  retains the v1 collector projection and installed worker build. It must not be
  repurposed as a held-launcher projection under the old consent.

There is no implementation of a pinned complete-cgroup kill operation in the
supervisor source at this baseline. Existing unit `KillMode=control-group`
properties and read-only manager/tree observations do not connect an autonomous
guardian to such authority. There is also no production constructor that joins
the separately signed v2 launch artifacts to the original admission owner.

Two existing helpers are not substitutes. The private
[autonomous watchdog mechanics](../../../../tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog.rs)
can signal an explicit bounded list of supplied pidfds, but do not authenticate
the original target set or pin a complete cgroup. The generic
[StopUnit helper](../../../../tools/long-run-supervisor-074/src/systemd_unit.rs)
opens its own system connection and waits against a new relative timeout. It is
not the original fixed-manager/full-frame/absolute-bound revocation path and
must not replace it.

Consequently, forwarding a future systemd worker pidfd to the current guardian
cannot complete this transition: the worker does not exist before delivery and
would fail the unchanged direct-child check afterwards. Calling Pending Armed,
moving arming after delivery, accepting an asserted PID/cgroup path, or changing
v1 message semantics are not admissible implementation shortcuts.

## Recommended decision for review

Use a separately enrolled, attempt-specific empty cgroup revocation root before
held-launcher delivery. An independent guardian would be armed against that
retained kernel root and the original absolute timer, not against an invented
future worker pidfd. A dedicated systemd slice is a candidate for that root, not
an approved topology or an already verified kernel guarantee.

This requires a separate versioned guardian enrollment protocol and held-unit
projection. Keep the old direct-child protocol, v1 projection and their tests
unchanged; register each new wire/durable artifact in `docs/COMPAT.md`. Bind the
new root, manager, attempt and signed helper artifacts through the original
trust/epoch/full-frame chain. Reusing the existing attempt must not create a
second replay slot. Signed v2 bytes may be retained only where their exact
semantics remain true; any changed signed requirements need a new schema/domain.

Before any host implementation, resolve who creates and owns that empty root,
how systemd places only the admitted held unit underneath it, which retained
descriptor permits bounded recursive revocation, and how root replacement,
foreign members, late population and uncertain setup are refused. The dedicated
root must exclude the guardian and sole collector themselves. Its creation and
cleanup are host mutations requiring their own preflight and authorization;
this recommendation does not permit them now.

Armed would prove readiness to revoke that admitted root, not worker identity,
responsiveness, EOF or completion. After original held delivery, bind actual
executable, streams, PID/start ticks, boot and complete descendant cgroup before
release under the unchanged full frame. No timeout or ACK may refresh any of the
original 300/60/10-second origins.

The root cannot revoke external copies held by the creator, manager or other
processes. Enumerating and closing those aliases, observing both actual kernel
EOFs after bounded drain and exclusively finalizing the collector remain separate
requirements before durable completion or fence release. Empty cgroup and
successful signal submission are insufficient.

The alternative is Pending, then held delivery, then target enrollment/Armed,
then release. That changes the approved pre-delivery arming contract and leaves
a window with delivered writer FDs but no armed target revoker. It is not the
recommended option and cannot be adopted by changing a test expectation.

## Work after approval

1. Specify and test the new revocation-root enrollment protocol and held-unit
   projection without any manager send, cgroup write or production constructor.
2. Implement owned nonproduct root/guardian mechanics, then the original
   trust/artifact/channel/frame join. Preserve every old live predicate, all 46
   effect brackets and the original deadlines; test partial enrollment and
   lost-ack uncertainty before any release API exists.
3. Extend real SIGKILL/lost-ack coverage across root setup, Armed, delivery,
   release and writer shutdown. Retain incomplete attempts with no resend.
4. Prove durable completion, later cells and same-attempt reconnect before the
   separately authorized installation and nonproduct server rehearsal.

Current evidence covers Pending and private direct-child mechanics, not these
production transitions. Stop at this architecture decision; do not manufacture
another wrapper that labels the unresolved transition as complete.

## Local audit result

The [regression packet](local-runs/original-arming-boundary-audit-20261019/README.md)
records 22 ordinary two-phase/kernel controls on each retained GNU 1.94 and
MSRV 1.88 image, 49 Windows unit and 17 compile-fail controls, and the three
focused Pending source-contract controls. Documentation, registry, scoped
formatting and the local non-promotable performance contract pass. The 94-file
Pending inventory and independent 182-file historical audit pass again.

All 381 supervisor Rust files, xtask, wire compatibility, registry and frozen
qualification inputs remain unchanged against the baseline. These are existing
regressions and an architecture audit, not a new Armed implementation or new
SIGKILL/lost-ack matrix. The three ignored two-phase process harnesses and the
full original-frame gates were not rerun. Existing source/image receipts remain
the provenance for the unchanged GNU binaries. WSL time synchronization stays
active; no server, supervisor, product or system-bus operation occurred.
