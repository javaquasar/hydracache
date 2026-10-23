# WSL probe registry append error localization

## Result

The separately authorized configuration-only session localized EINVAL to
operation 6: Python opening `/sys/kernel/tracing/kprobe_events` in append mode.
The exception occurred inside `Path.open`, before `stream.write` could submit
the first probe definition. This is a collector I/O failure, not an observed
rejection of the probe's field offsets, function address or argument syntax.

The first five operations completed: private-instance creation, tracing_on=0,
256 KiB per CPU, mono_raw and stacktrace=1. The durable journal retains both
the before/after records for these operations and the before/failed record for
operation 6, with exact path, planned command, errno and full traceback. The
configuration stopped there. No probe was registered, no buffer was activated
and no clock query/helper or original-owner run was executed.

## Source level explanation

The [CPython 3.12.3 FileIO implementation](https://github.com/python/cpython/blob/v3.12.3/Modules/_io/fileio.c#L452-L460)
explicitly seeks to the end while initializing append mode. Its error path
suppresses ESPIPE, not EINVAL. The pinned
[WSL kprobe registry](https://github.com/microsoft/WSL2-Linux-Kernel/blob/linux-msft-wsl-6.6.87.2/kernel/trace/trace_kprobe.c#L1185-L1192)
uses seq_lseek. The pinned
[seq_lseek implementation](https://github.com/microsoft/WSL2-Linux-Kernel/blob/linux-msft-wsl-6.6.87.2/fs/seq_file.c#L292-L324)
accepts SEEK_SET/SEEK_CUR and otherwise returns EINVAL, including SEEK_END.

This source-level call chain explains the open-time failure and the empty
probe/error registries. The session did not trace the actual lseek syscall;
the measured fact is the failure during append-stream construction. It does
not establish whether registration would succeed after changing the writer.

The proposed isolated correction is a low-level non-truncating command writer:
open the existing registry with O_WRONLY|O_APPEND|O_CLOEXEC, perform no seek,
submit one complete command, check its byte count, and close. Reject short
writes without blindly resubmitting a command. Do not switch to Python `w`:
the kernel's O_TRUNC path releases probe definitions, which could affect other
owners. No such writer correction or follow-up registration was executed in
this diagnostic step.

## Cleanup and unchanged boundaries

The pre-cleanup registry and both error logs are empty. The owner and detached
guardian both confirmed that the owned instance is absent and no owned probes
remain. Global root controls compare equal, including tracing_on=1, nop tracer,
local trace clock, original options and empty event/probe/instance configuration.
Boot, PID1 start ticks and time namespace compare unchanged. The guardian
reported normal cleanup completion; the configuration session itself exited 1.

All original preflight checks and the 120 RAW-second guardian/180-second outer
timeout remain intact. Product 300/60/10 windows, runtime predicates and the
qualification manifest are unchanged. There were no clock/service changes,
product workloads, package installations or server actions. The first trace
archive remains pinned and unchanged; this configuration result is separate.

## Evidence and step gate

Candidate source: `07da6024`. [Exact archive](local-runs/wsl-clock-einval-20261023/index.json):
27 logical files and 26 content-addressed objects, index SHA256
`99c84217cbe5d99e3aef1899e94178bc20bf03c5728dabf954bf135ad5c7f758`.
It retains the frozen method/source, 21 successful controls, operation journal,
first exception, partial state, command exit and both cleanup receipts.

```powershell
python -B tools/wsl-clock-trace-074/einval.py verify
python -B tools/wsl-clock-trace-074/run.py verify
wsl.exe -d Ubuntu -u artur --exec python3 -B -m unittest discover -s tools/wsl-clock-trace-074 -v
.\target\debug\xtask.exe doc-check
.\target\debug\xtask.exe performance-contract-check --release 0.74
```

The mock tests cover durable intent before effects, refusal before effects when
intent fsync fails, exact failure location, no operation after first failure,
read refusal and complete configuration with no trace activation. Existing
ABI, partial/foreign cleanup and owned-process SIGKILL guardian controls pass
unchanged. No runtime crate was changed or rebuilt by this step.

The next work item is the isolated non-seeking writer correction and its
negative tests. A subsequent live registration/capture needs its own explicit
scope and fresh lane; neither consumed lane may be replayed into a pass. Actual
clock-setter attribution, healthy original delivery and server admission remain
open.
