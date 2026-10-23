# Read only attribution of WSL discipline and time corrections

The MONOTONIC/RAW rate difference is quantitatively explained by the observed
kernel tick and frequency settings. Positive REALTIME corrections correlate
with the existing Ubuntu NTP daemon's replies. An independent backward
correction is also visible, but the exact caller of that correction and the
historical writer of tick remain unresolved: available audit and trace records
do not identify them. No original-owner execution or server admission is opened.

## Fixed method and retained first refusal

The [plan](wsl-clock-causality-plan-20261023.md) freezes 81 low-rate rows with
one-second sleeps, RAW-bracketed kernel clock reads and adjtimex modes=0 queries,
followed by read-only NTPMessage getters on the already running daemon. Bus
auto-start is disabled. modes=0 reads parameters without setting them, and
frequency has 16 fractional bits in ppm, as documented by the
[Linux API](https://www.man7.org/linux/man-pages/man2/adjtimex.2.html).

The first lane exited 1 during PID1 namespace metadata, with zero clock/NTP
rows: artur could not dereference the root PID1 namespace symlinks. Its consumed
path and first negative result remain in a separate 28-file/22-object archive,
index 6a7d11e8e8dc830262b8191bacdce03b1a0b822261d246dbaae0991da0694298.
No clocks were sampled in that refused lane, and no predicate was removed.

The separate root-context candidate changes only metadata-reading credentials:
root reads the complete context and runuser executes every unchanged query
helper as artur, EUID 1000. Its source commit was
1658ec9caa3ac41cb222b4ab0164bb5802a3bab7; the helper SHA-256 was
f3e81c0e8080969e00ead542269566b6e079e3eacc586e771236a65f413180dc.
The first collection completed once, exit zero, with all 81 rows. The 110-second
session and 130-second outer execution bounds are diagnostic bounds, not changes
to any original 300/60/10 admission deadline.

Boot cf222c07-5476-4f70-9a32-455e99204d0a, PID1 start ticks 10939 and all observed
namespace identities matched before and after. The time namespace was
time:[4026531834], with zero namespace offsets. The daemon remained observed as
PID 176, D-Bus owner :1.1, CAP_SYS_TIME and the same executable hash before/after.
Its installed package is 255.4-1ubuntu8.18; its Ubuntu WSL drop-in explicitly
permits timesyncd in WSL. This is diagnostic provenance, not an admission frame.

## MONOTONIC versus RAW

All 81 timex reads reported tick=9167 us, USER_HZ=100 and frequency=-2279760
scaled ppm, or -34.786376953125 ppm. The nominal tick at this USER_HZ is 10000 us.
The version-matched [kernel discipline source](https://raw.githubusercontent.com/microsoft/WSL2-Linux-Kernel/linux-msft-wsl-6.6.87.2/kernel/time/ntp.c)
combines the tick term and frequency adjustment when computing the disciplined
rate. The [timekeeper](https://raw.githubusercontent.com/microsoft/WSL2-Linux-Kernel/linux-msft-wsl-6.6.87.2/kernel/time/timekeeping.c)
keeps separate adjusted and raw conversion multipliers.

The tick-plus-frequency model gives:

    9167 * 100 / 1000000 + (-2279760 / 65536) / 1000000
    = 0.9166652136230469

The observed MONOTONIC-per-RAW endpoint enclosure is
0.9166651831421702 through 0.9166652385255675, which contains that prediction.
The discrepancy is about 8.33348 percent relative to RAW, not an assertion of
absolute error relative to Windows or UTC. It is much larger than the frequency
field alone and is not explained by merely calling RAW a noisier clock.
The read-only state and observed rate identify kernel discipline as the local
mechanism for this series; they do not identify who originally set tick=9167.
No earlier series is retroactively assigned a timex state it did not record.

## NTP corrections and a separate backward event

The daemon's reported poll interval was 32 seconds. Its packet count advanced
three times, with spike=false and positive offsets computed from the four
timestamps exposed by the [systemd D-Bus implementation](https://raw.githubusercontent.com/systemd/systemd/v255/src/timesync/timesyncd-bus.c).

| Row | NTP packet | NTP offset us | Adjacent REALTIME minus MONOTONIC delta ns |
| --- | ---: | ---: | ---: |
| 11 | 308 | 8785012.5 | 2730957125 |
| 39 | unchanged 308 | no new response | -6057411005 |
| 40 | 309 | 8786289.5 | 8786289447 |
| 68 | 310 | 8786406.0 | 2727654979 |

At row 40, the positive clock residual agrees with the NTP offset to the
timestamp representation's sub-microsecond rounding. This is strong correlation
with timesyncd's correction path, not a sub-microsecond accuracy claim. The
[upstream 255.4 implementation](https://raw.githubusercontent.com/systemd/systemd-stable/v255.4/src/timesync/timesyncd-manager.c)
steps large accepted offsets via clock_adjtime/ADJ_SETOFFSET and publishes the
saved response afterward. Local package hashes and a bounded disassembly are
retained separately; upstream source review is not a byte-for-byte package build
attestation or a syscall trace.

At row 39, REALTIME itself fell 5,008,782,244 ns while MONOTONIC advanced
1,048,628,761 ns, with no packet-count change. Kernel status changed from 8192
to 8256, adding STA_UNSYNC; row 40 removed that bit. The other two NTP intervals
have much smaller net residuals than their positive offsets, compatible with an
additional opposing correction inside those intervals. One-second sampling
does not resolve its exact timing or all intermediate events.

The unchanged Hyper-V driver parameter timesync_implicit is Y. The
[version-matched driver](https://raw.githubusercontent.com/microsoft/WSL2-Linux-Kernel/linux-msft-wsl-6.6.87.2/drivers/hv/hv_util.c)
has a host-setting path through do_settimeofday64. Its implicit SAMPLE condition
only requests synchronization when the guest is at least five seconds behind;
that condition alone is not evidence for a backward correction. An explicit
SYNC request or another setter remains possible. STA_UNSYNC is likewise not a
unique caller signature: several kernel paths can set it. Attribution of this
exact backward event to Hyper-V is therefore still an inference, not a finding.

## Read only attribution boundary

The existing trace configuration was inspected without writes: tracer nop,
no configured events or dynamic probes, and buffer entries/writes 0/0. The audit
journal query returned no entries; /var/log/audit/audit.log was absent. No event,
probe, audit rule or log level was enabled to manufacture missing history.
The selected Windows clock-change log's latest returned event was October 14,
not a contemporaneous caller record for this October 23 series. That bounded
query does not exclude all host time mechanisms.

The most supported model is two effects: an abnormal coarse kernel tick explains
the MONOTONIC/RAW rate, while an active NTP correction path and an opposing
clock-setting path produce REALTIME steps. The read-only evidence cannot name
the historical tick setter or uniquely identify the opposing caller. Changing
time services or restoring tick would be a different, separately authorized
experiment. Exact attribution would require pre-existing caller records or a
separately authorized bounded trace; none were silently enabled here.

## Checks and evidence

Fifteen offline Python controls passed, including the signed NTP decoder,
tick model/RAW enclosure, first event preservation, incomplete data, changed
modes, packet reset, backwards brackets and read-only credential boundaries.
The C query compiled with -Wall -Wextra -Werror; shell syntax passed. Local
doc-check, the 188-gate registry and non-promotable 0.74 performance contract
passed. No Rust runtime or fixture source, Cargo.lock or compatibility artifact
changed from 8fb0fb5a.

The root-context archive retains 46 logical files in 36 exact objects, index
798b204c4b14865f482d64cdc4176aec7c1c3c4126b87bf635e8d9c2ba140082.
It includes sources, intent/build/test receipts, both raw data streams, context,
audit/trace and Windows queries, installed package observations and offline
status-transition analysis. Binaries are excluded. The metadata-only refusal
is a separate archive, not overwritten by this collection.
Git index byte audit matches all 22 objects and the index of that first archive,
and all 36 objects and the index of the root-context archive.

Code/evidence commits are 9ea5be0b (fixed method), edd0f2a1 (metadata refusal),
1658ec9c (credential separation) and 05fc4ab5 (complete read-only series).

The earlier clock-series index remains
edec06e2ac4fa97126ce14ac5c0e4d1aff9cae7bd39f96bb91391102f3d606e3;
the original-owner first-refusal index remains
9f2fd0f457ec6d48f4e87e7bfc2bd1262bbf191cf51b760fe744eb369394b9f6;
the qualification manifest remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
Original-owner was not repeated. No clocks, existing services, admission checks,
deadlines or server state were changed. Delivery, retirement and server admission
remain unproven rather than being inferred from this diagnostic progress.
