# WSL tracing configuration error localization

One separately authorized diagnostic session localizes EINVAL without repeating
the clock observation. The first capture archive remains pinned at SHA256
89912e476413847377bed1f22d1e52c799d2ded988fd3f861ecfedc56960da03.

The candidate repeats only the ordered configuration preceding buffer activation:
create a fresh private instance; set tracing_on=0, buffer=256 KiB per CPU,
mono_raw and stacktrace; register the same seven exact probes, the same modes!=0
filter and per-instance enables; verify registrations and read formats. It never
writes tracing_on=1, never reads a trace and never invokes the modes=0 clock
helper. Thus it is a configuration diagnostic, not a replacement clock series.

Before every operation, fsync an intent record containing sequence, operation,
exact path and value; after success record completion. On the first exception,
record errno, filename and full traceback, then stop configuration. Preserve the
partial registry and private/root error logs before cleanup. Do not skip a failed
stage, try alternative spellings or continue into a passing configuration.

Keep all existing preflight checks: exact WSL kernel and BTF digest/layout/ABI,
helper hash, target symbols and empty/idle tracer configuration. Arm the same
detached pidfd guardian before effects, with the original 120 RAW-second deadline;
keep the Windows 180-second outer timeout. Product 300/60/10 windows are unchanged
and unused here. Retain both cleanup receipts and compare root controls, boot,
PID1 start ticks and time namespace. Never overwrite global state to restore it.

The lane is exclusive even on failure. Retain exact source and result bytes in
a new archive, without changing the first archive or auto-dispatching again.
No clocks, services, original-owner, product workload, packages or server actions
are in scope. The diagnostic may identify a configuration failure; it cannot
identify an actual clock setter without a separately authorized observation.

Before the session, unit-test journal ordering, durable intent refusal, exact
failure location, fail-fast behavior and successful configuration without trace
activation using fake operations only. Run the existing ABI/cleanup/guardian
controls unchanged, review the diff and commit the candidate. After the one
session, preserve its first result and verify cleanup and the archive.
