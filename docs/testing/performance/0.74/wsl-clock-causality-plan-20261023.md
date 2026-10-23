# Read only WSL discipline and NTP correlation plan

This separate diagnostic lane investigates time corrections without repeating
original-owner admission or changing clocks, services, configuration or server
state. The first original-owner and first clock-series archives stay immutable.
Every live predicate and original 300/60/10 deadline remains unchanged.

Exploratory read-only metadata found tick=9167 us at USER_HZ=100 and a much
smaller frequency adjustment. The next fixed observation collects 81 rows in
one existing Ubuntu session, with one-second sleeps, then stops regardless of
outcome. No pass-seeking repetition is allowed. The outer bound is 130 seconds;
the session's own timeout is 110 seconds. Neither is an admission budget.

Each row records CLOCK_MONOTONIC_RAW before and after BOOTTIME, REALTIME,
MONOTONIC and an adjtimex query with a zero-initialized modes field. The helper
runs as artur, EUID 1000, without CAP_SYS_TIME. It has no clock-setting inputs.
Read-only NTPMessage getters query the existing org.freedesktop.timesync1 owner
with bus auto-start disabled. They neither request new synchronization nor
alter log levels. Boot/PID1/namespaces and daemon status bracket the session.

Analysis records every adjacent REALTIME-minus-MONOTONIC residual, backward
interval and NTP packet change, not only events above a chosen cutoff. It
computes NTP's signed offset from the four retained timestamps. The reported
MONOTONIC/RAW rate uses RAW endpoint enclosures, not a host-accuracy claim.
The tick-plus-frequency prediction is kept separate from the observed range.
Unknown method, incomplete rows, mutation modes, context changes, reversed
brackets or packet-count reset refuse analysis rather than retrying.

Read-only audit/trace/journal inspection may establish whether historical caller
records already exist. No tracing events, probes, audit rules or logging are
enabled. Existing empty buffers cannot identify a past writer. Matching an NTP
update to a clock event supports correlation, not exact syscall attribution.
Source review must use primary kernel/systemd sources and preserve the distinction
between upstream behavior, installed package bytes and observed events.

The previous clock index is
edec06e2ac4fa97126ce14ac5c0e4d1aff9cae7bd39f96bb91391102f3d606e3;
the original-owner index is
9f2fd0f457ec6d48f4e87e7bfc2bd1262bbf191cf51b760fe744eb369394b9f6.
Only workspace helper binaries/logs/evidence may be created. A result that
cannot name an exact historical setter must remain explicitly unresolved.

## Metadata permission correction before any clock rows

The first lane exited 1 while artur was reading PID1 namespace symlinks, before
context completion or any kernel/NTP sample. Its exclusive path is consumed and
retained with zero rows, index
6a7d11e8e8dc830262b8191bacdce03b1a0b822261d246dbaae0991da0694298.
It is not a clock-order pass or a failed original-owner repetition.

The separate root-context lane keeps all the same rows, bounds, context checks
and source guards. Only the metadata collector runs as root; runuser executes
the unchanged modes=0 query helper as artur for every row. No namespace check
is removed to accommodate the permission refusal. The old lane is not reused;
both first outcomes must be retained. No service or account action is added.
