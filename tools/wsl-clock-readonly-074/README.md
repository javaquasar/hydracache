# Read only WSL clock diagnosis

This observer is separate from original-owner admission. It never runs libtest,
the bootstrap, a worker, enrollment, a transient service or a server command.
The original clock-order refusal and frozen qualification manifest are inputs,
not replaceable outputs. Runtime and fixture sources stay at 8fb0fb5a.

The method is fixed before observation: one HydraCache074Native session holds
metadata, 601 sample rows with 100 ms sleeps, and postflight. Each row reads
BOOTTIME, REALTIME, MONOTONIC and MONOTONIC_RAW in that order. Each clock is read
through libc, a direct Linux x86_64 clock_gettime syscall, then libc again. CPU
IDs bracket the row. No affinity is changed. The observer sleeps between rows;
this is a low-rate diagnostic, not a runtime cost or workload measurement.
The helper has an 80-second timeout; the outer session has a 120-second timeout.
These diagnostic bounds do not replace any original admission deadline.

Metadata includes current/available clocksource, kernel/systemd versions, CPU
allowance, clock-related masks and loaded units, process names and bounded clock
logs. Only read APIs are used. The journal read can report no entries; a Windows
time status query can fail and is retained as metadata, not retried. No clock,
service, mask, mount or account configuration is changed. Helper artifacts and
logs are written only under this workspace's fixed target directory.

Pure tests reject changed methods, reordered/incomplete observations and invalid
data. Analysis preserves the first backward or libc/syscall bracket discrepancy.
Adjacent REALTIME-minus-BOOTTIME delta residuals use non-simultaneous reads: they
are not an exact offset, drift estimator or causal attribution to a daemon.
CPU endpoint equality cannot exclude a migration inside a row. libc behavior is
observed, not assumed to prove a particular vDSO implementation.

The read-only session compares boot ID, PID1 start ticks, user/mount/pid/time
namespaces and namespace time offsets before and after. Normal PID1 cpu counter
changes are not a new init. The shared VM boot ID alone is insufficient to join
observations across WSL distro inits.

Manual sequence from the repository root:

    python -B -m unittest discover -s tools/wsl-clock-readonly-074 -v
    python -B tools/wsl-clock-readonly-074/diagnose.py build
    python -B tools/wsl-clock-readonly-074/diagnose.py observe
    python -B tools/wsl-clock-readonly-074/diagnose.py retain
    python -B tools/wsl-clock-readonly-074/diagnose.py verify

Build and observe consume exclusive output/intent paths. Do not remove these to
retry, even when the first observation fails. analyze is offline-only and also
refuses to overwrite an existing analysis. verify does not enter WSL. retain
keeps exact sources/logs in a separate non-admitting content-addressed archive;
the old original-owner archive is never resealed. A complete ordered sample is
not permission to repeat original-owner execution or open a release gate.

The fixed lane is now consumed. Its first series reproduced two backward
REALTIME intervals through both read paths. Use only verify for the retained
[result](../../docs/testing/performance/0.74/wsl-clock-readonly-20261023.md);
do not repeat build or observe from the historical command sequence above.

## Separate discipline and NTP correlation

causality.py and discipline.c are a separate read-only lane, not an extension
or rerun of the old observer. The [plan](../../docs/testing/performance/0.74/wsl-clock-causality-plan-20261023.md)
and [result](../../docs/testing/performance/0.74/wsl-clock-causality-20261023.md)
retain the first PID1 namespace permission refusal and the separate corrected
root-context collection. Query helpers remain artur, modes=0; root only reads
metadata. No service, tracing or clock configuration is modified.

Both collection paths are consumed. Only offline verification is appropriate:

    python -B tools/wsl-clock-readonly-074/causality.py verify

The old 601-row archive stays unchanged. The newer rate model explains the
observed tick/frequency effect, not historical caller identity or admission.
