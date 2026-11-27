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

- Deterministic tests in the server's `imap_chrony` and replicated value tests cover exact-bound expiry, either
  direction of a clock step, aged references, sample timeout, backward/forward slew, overflow,
  source loss, recovery, and independent observations on two members.
- Run a three-member Linux qualification with the admitted host profile. Record the earliest
  allowed expiry across owner promotion, the fenced tombstone and applied prefixes, and recovery
  after a clock fault. Failure to qualify the bound leaves OD-2 open.

### Collecting evidence on a target host

The distributed HC/2 profile accepts a Linux `[chrony]` table with the absolute `chronyc` path,
the `references` IP allowlist and `sample_interval_ms`. Its `[chrony.policy]` table names
`maximum_uncertainty_ms`, `maximum_snapshot_age_ms`, `maximum_reference_age_ms`,
`maximum_acquisition_ms` and `maximum_drift_ppm`. Set these explicitly for the host profile.
The local observer runs `chronyc -n -c -h 127.0.0.1 tracking` on an interval. The operator must
enable chronyd's read-only monitoring on loopback and use the same disciplined clock for the
collector and the member. Readiness reports `clock_uncertainty_exceeded` when the clock
withholds its observation after the topology and replica checks pass.

To record evaluation data, place the contents of the `chrony` table in a standalone TOML
document, using `[policy]` for its nested table. On each independently clocked member host,
use the frozen candidate and the desired observation window.

```sh
HYDRACACHE_OD2_CHRONY_PROFILE=/path/to/clock.toml \
HYDRACACHE_OD2_CLOCK_REPORT=/path/to/fresh-clock-report.jsonl \
HYDRACACHE_OD2_SAMPLE_SECONDS=60 \
cargo test --locked -p hydracache-server --lib -- --exact \
  imap_chrony::linux::tests::records_od2_clock_evidence_on_the_current_host --nocapture
```

The JSONL records local elapsed time, clock generation, uncertainty, snapshot age and an upper
bound on reference age when the clock supplies an observation. A refused observation has no
uncertainty value. Record the host's chrony version and configuration, the exact candidate and
binary hashes, and the timestamps of source loss, holdover, clock step or suspend and recovery
alongside the log. Use the matching HC/2 and replica receipts to verify the expiry decision across
promotion before approving the profile.

## Revisit when

The target environments require another clock provider or measured expiry availability differs
from the frozen release contract.
