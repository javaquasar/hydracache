# Authenticated heartbeat under the original collector reservation

Initial Reserved retention and controller presence are different facts. The
consuming bind_authenticated_progress preflight retains the acknowledged original
reservation together with one new kernel packet. It authenticates Heartbeat
without changing the state, appending a journal entry or authorizing Starting.

The private kernel join requires identical receiver-origin objects, ordered boot,
BOOTTIME and realtime observations, and a receive point strictly later than the
reservation owner's last observation at transfer. Matching host strings or a
different socket on the same host cannot satisfy this identity check. The actual
new sender credentials and current kernel time enter the existing signature and
principal verifier through the original configured controller policy. The worker
join brackets that check with its unchanged policy, epoch authority and context.

The authenticated request must be Heartbeat at revision 1, with the original
identity, controller, source and preset. Its request id and nonce must differ from
the Reserve request. The original owner is consumed exactly once, including on
failure; neither the result nor the failure can extract it or accept another
packet. This bounds the current preflight to one follow-up without pretending to
implement the future durable heartbeat replay/commit protocol.

A valid follow-up establishes authentication and bounded freshness of that
message only. It does not prove uninterrupted controller presence since Reserve
or revive an expired execution lease. Starting must separately commit and enforce
its complete lifecycle/deadline history rather than infer it from this preflight.
The receive point is sampled around dequeue, not a kernel enqueue timestamp or
proof that the controller generated the signed message after observing Reserved.
A queued, previously signed heartbeat can satisfy receive order while its sender
remains live. A future execution/progress grant must define its challenge/causal
binding and replay history; this preflight cannot substitute for those predicates.

Freshness expires exclusively ten seconds after the new packet's original
receive BOOTTIME. Revalidation never replaces that point. Both the original and
new packet holds, both signed request windows, the collector signature window
and original OIDC/build/worker checks remain in force. In particular, receiving a
valid heartbeat cannot extend the original 60-second admission hold. The frozen
Reserved state, all three initial timestamps and its two retained files remain
unchanged. The 300-second total bound stays anchored to the initial reservation.

Revalidation checks reservation, progress, reservation and progress in order;
the final progress check follows the potentially slow original-chain checks.
Any error refuses the original chain, storage and new packet, retaining their
fence and descriptors until drop. Already-refused calls return before more I/O.
No new write boundary exists here, so this slice adds no new SIGKILL write matrix;
the prior Reserved crash evidence remains historical and immutable.

Tests separate real kernel-message/ownership observations from pure boundary
and binding cases. Local nonroot roots and synthetic signing keys do not enroll
production trust. A future consuming Starting publication still needs its own
signed state projection, durable uncertain-outcome/replay handling and SIGKILL
and lost-ack tests. Runtime authority beyond admission freshness, root output
creation, authenticated executable/streams/process/cgroup start, autonomous
watchdog and all-writer cessation remain prerequisites for server admission.

The next protocol cannot simply add Starting writes under this guard. Slow
publication and repeated original checks consume its fixed admission and
message-freshness budgets. A separately owned lifecycle authority handoff must
define which admission predicates are discharged, which policy/epoch/context
checks remain live, how newly authenticated progress is committed and how
uncertain publication refuses restart. It must not refresh old packet clocks or
silently stop checking expired admission inputs in the existing guard.

The first full default-parallel original-input run retained 93 passes and two
Authentication(Time) failures in the old positive Reserved test and the new
composition. Workspace formatting and read-only gates overlapped that attempt.
An unchanged default-parallel repeat without those tasks passed all 95 active
tests, with two existing manual cases ignored. This does not establish the
overlap as the cause or robustness under arbitrary shared load. Both results
remain evidence; no signing window, hold, freshness deadline or test parallelism
was changed. The MSRV focused composition passes six tests separately, not a
rerun of the entire MSRV original-input group.
