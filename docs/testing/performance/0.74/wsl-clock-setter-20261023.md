# WSL clock setters in the bounded capture

The first newly authorized capture completed and decoded successfully. It
observed tick/frequency requests from kernel task name `chronyd`, absolute
settimeofday calls from `init`, and positive offset injections from
`systemd-timesyn`. This identifies captured kernel call paths, not authenticated
executables, services or distributions, and not authors of historical failures.

## Capture integrity and scope

The unchanged collector ran once from source `20e406f4` in a fresh exclusive
lane. It retained 161 read-only helper samples and 87,691 bytes of trace. The
buffer reports 502/502 entries; all eight per-CPU overrun, commit-overrun and
dropped-event counters are zero, and all seven kprobe miss counters are zero.
The 502 records comprise 251 decoded events and their stacks. There are no
unmatched non-query entries. The 161 entry-filtered query returns are excluded
from successful writer counts.

The original target was 90 RAW seconds, with a 120 RAW-second guardian and
180-second outer timeout. The capture receipt spans 90.416524376 RAW seconds
from start through loop completion and trace retention, not an exact active
90-second interval. The first/last query RAW-before span is 89.743627230 seconds.
No samples were trimmed and no deadline was changed. The original 200-query,
16 MiB and 256 KiB-per-CPU caps were retained.

Owner and guardian both confirmed removal of all owned probes and the instance;
the guardian reported normal completion. Root tracing controls, boot, PID1 start
ticks and time namespace compare unchanged. No clocks, services or packages were
changed by the collector. No original-owner, product workload or server ran.
Product 300/60/10 windows, runtime checks and qualification remain unchanged.

## Observed writers

All calls below have matched successful returns and kernel stacks. Counts refer
to instrumented calls, not independent physical corrections: offset injection
is nested inside its outer adjustment call.

| Kernel task name and PID | Captured operation | Successful calls |
| --- | --- | ---: |
| chronyd, 210 | adjustment modes 0x4002, tick 9163 us and frequency 5897350 scaled ppm | 18 |
| chronyd, 210 | adjustment modes 0x001c, status/error fields | 18 |
| init, 1126 / 1112 / 920 | do_settimeofday64 through __x64_sys_settimeofday | 3 |
| systemd-timesyn, 1157 | adjustment modes 0x211c | 3 |
| systemd-timesyn, 1157 | nested timekeeping_inject_offset | 3 |

The pinned [kernel mode definitions](https://github.com/microsoft/WSL2-Linux-Kernel/blob/linux-msft-wsl-6.6.87.2/include/uapi/linux/timex.h#L129-L139)
identify 0x4002 as ADJ_TICK|ADJ_FREQUENCY, and 0x211c includes ADJ_SETOFFSET and
ADJ_NANO. Fields not selected by the mode are not setter evidence. In particular,
the unrelated freq/time values in chronyd's 0x001c requests must not be interpreted
as frequency or time-setting commands.

All helper queries returned tick=9163 and frequency=5897350, with USER_HZ=100.
Their predicted MONOTONIC/RAW rate is 0.9163899864, inside the observed
RAW-bracketed enclosure [0.9163897226, 0.9164046606]. This attributes current
tick-setting requests to the observed chronyd task label, not the historical
writer of the earlier tick=9167. RAW is not a certified UTC reference.

## Opposing steps can be hidden between samples

Each successful init settime target was 5.745–6.026 seconds earlier than the
preceding helper's REALTIME read. Shortly afterward, a successful timesyn offset
injection added about 8.82 seconds in the same enclosing query interval. All 161
helper samples remained increasing in REALTIME; that did not exclude intervening
opposing corrections.

| init kernel PID | Successful offset request, s | Enclosing REALTIME minus MONOTONIC delta, s | Delta minus offset, s |
| --- | ---: | ---: | ---: |
| 1126 | +8.817187 | +2.731818 | -6.085369 |
| 1112 | +8.825669 | +2.745037 | -6.080633 |
| 920 | +8.824162 | +2.736646 | -6.087516 |

These are reproducible arithmetic windows around observed successful calls,
not direct measurements of each isolated step or nanosecond clock accuracy.
The absolute targets, syscall stacks and offset arguments are independently
retained. The hv_set_host_time probe had zero hits in this window; this does not
exclude host integration acting through the observed userspace init calls or
retroactively settle the older backward event.

## Namespace and identity limit

The post-capture visible Ubuntu procfs lookup reports PID 210 as agetty, while
PID 1157 is absent. It is not a valid identity join to the trace's kernel PIDs.
That metadata and its missing-file results are retained. Do not assign these
writers to Ubuntu services or a specific distribution from task names alone.
A namespace-aware, read-only identity investigation is the next diagnostic step;
changing clock policy or stopping a writer requires separate agreement.

## Evidence and checks

[Capture archive](local-runs/wsl-clock-setter-20261023/index.json): 54 logical
files / 46 objects, SHA256
`6b54d38f0ce7b4ceacdeb0ce879a7aee6570e53501cfc12206b39c78eaef051d`.
It includes the first raw trace, formats, loss/profile counters, first decoded
analysis, helper series, context/cleanup, source and 35 passing pre-dispatch
controls. All earlier archives remain pinned and unchanged.

[Offline attribution archive](local-runs/wsl-clock-setter-offline-20261023/index.json):
17 files / 15 objects, SHA256
`8fd6a9410913cd24f1134bd7af5f49d16a36c0114761b191fae3c1bd5c635443`.
Source `cf8f73b4` reproduces the counts, three windows and rate enclosure from the
sealed first capture without new live observations. Three new controls cover
exact arithmetic, loss/context/window refusal and excluding failed tick requests.
The integrated local suite passes 38/38; doc-check, local non-promotable
performance-contract and previous evidence verifiers pass. No Rust crate changed.

```powershell
python -B tools/wsl-clock-trace-074/observe.py verify
python -B -m unittest discover -s tools/wsl-clock-trace-074 -p test_attribution.py -v
```

The consumed capture is not repeated. Successful decoding is not healthy clock
policy, original delivery, all-writer retirement or server admission. No
performance improvement is claimed from this diagnostic trace.
