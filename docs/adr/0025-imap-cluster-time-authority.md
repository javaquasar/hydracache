# ADR-0025: Bounded time observations for IMap expiry

## Status

Proposed for 0.75. OD-2 remains open pending evidence from the target hosts and approval of the
provider profile and its uncertainty limit.

## Context

An expiry deadline must survive owner promotion and replication. A fast clock can expire a live
value early. A stale synchronization report can make that clock appear healthy. The owner must
prove a conservative time bound before it proposes expiry, then apply a versioned tombstone on the
ordered partition lane. Raft epoch and applied prefixes establish ownership and ordering under R-1
and [ADR-0024](0024-imap-quorum-backed-read-barrier.md).

## Options considered

- Host `CLOCK_REALTIME` alone provides a shared time domain and lacks an uncertainty bound.
- A local monotonic clock is useful for measuring age and cannot express a deadline across hosts.
- Managed chrony on Linux can report a bounded estimate of time error under an explicit trusted
  reference and host clock profile. It needs freshness and clock-step handling at the application
  boundary.

## Proposed decision

Use the Linux host clock disciplined by an admitted chrony profile. Sample chrony's `tracking`
data off the request path. Use a local collector with access to read monitoring data, and give the
server an immutable bounded snapshot through a local trusted interface. The server needs neither
chrony administration privileges nor shell execution on the request path. Qualification records
the chrony version and configuration, trusted references, leap-handling policy and drift bound for
each host. A local-reference-only or unsynchronized source cannot qualify. The documented chrony
error estimate assumes a correct upstream reference. [chronyc tracking](https://chrony-project.org/doc/4.6/chronyc.html) ·
[Chrony configuration](https://chrony-project.org/doc/4.6/chrony.conf.html)

For each accepted sample use `|system time offset| + root dispersion + root delay / 2`. Include the
measurement window, rounding and conservative drift since acquisition. Round uncertainty up. Age
the snapshot with `CLOCK_BOOTTIME`, and independently check the age of the underlying reference
update. Re-reading the same reference cannot refresh its age. Reject malformed or non-finite
fields, invalid source state, stale data and arithmetic overflow. Record the Linux clock generation
with the sample. Invalidate it on a forward or backward wall-clock step, restart, suspend/resume or
loss of the clock watcher. A Linux `timerfd` armed with `TFD_TIMER_CANCEL_ON_SET` can detect
discontinuous realtime steps. Sample and validate the generation around the realtime read so the
application cannot consume an old error bound with a stepped clock. [Linux timerfd](https://man7.org/linux/man-pages/man2/timerfd_create.2.html)

For evaluation, use an initial **10 ms** maximum uncertainty and a **3 s** maximum local snapshot
age with a **1 s** collector cadence. These are trial values. Freeze the reference-age and drift
bounds from the managed synchronization profile. Freeze the production uncertainty limit only
after measuring the target hosts against the agreed 0.75 TTL and availability budgets. Retain
uncertainty and reference-age distributions during steady state, reference loss, holdover,
clock steps and rejoin.

The owner computes `expires_not_before_ms` from the upper time bound plus the requested finite
TTL, with checked arithmetic and rounding that cannot shorten the TTL. Replicas apply the resulting
deadline unchanged. To propose a tombstone, take a current observation and require its lower bound
to reach the deadline. Recheck the current epoch, owner and entry version on the partition lane.
An invalid observation marks the member unready for owner work. A previously armed expiry timer must
pass the checks again after recovery. Publish the time-dependent readiness reason and preserve the
existing [quorum read barrier](0024-imap-quorum-backed-read-barrier.md) checks independently.

## Verification before acceptance

- Deterministic tests in the planned `time_authority` target cover exact-bound expiry, either
  direction of a clock step, aged references, sample timeout, backward/forward slew, overflow,
  source loss, recovery, and independent observations on two members.
- Run a three-member Linux qualification with the admitted host profile. Record the earliest
  allowed expiry across owner promotion, the fenced tombstone and applied prefixes, and recovery
  after a clock fault. Failure to qualify the bound leaves OD-2 open.

## Revisit when

The target environments require another clock provider or measured expiry availability differs
from the frozen release contract.
