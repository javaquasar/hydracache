# Read only diagnosis of WSL clock ordering

The first read-only clock observation reproduced backward REALTIME without
running original-owner admission. Both libc and direct clock_gettime syscalls
observed two backward intervals, while BOOTTIME and MONOTONIC continued forward.
The earlier original-owner refusal remains exit 101 and is not replaced by this
diagnostic result. No runtime predicate, validation round or original deadline
changed; original delivery and server admission remain closed.

## Method and execution boundary

The [standalone observer](../../../../tools/wsl-clock-readonly-074/README.md) was
committed before observation as 3d3b02c5a0b2398c09dc01ec0bd86627d9de7590. It uses
no runtime admission objects, enrollment, bootstrap, workers or service fixture.
The GNU observer binary SHA-256 is
e5c44ef4920caf685b0860d695ac25d78cf791e868621e5981dc4f30838ed71e.

One bounded HydraCache074Native session collected metadata, 601 rows separated
by 100 ms sleeps, and postflight. Each row read BOOTTIME, REALTIME, MONOTONIC and
MONOTONIC_RAW in that order. For each clock it read libc, direct Linux x86_64
syscall, then libc again. CPU identifiers bracketed each row; no affinity changed.
The helper bound was 80 seconds and the outer session bound 120 seconds. These
are diagnostic execution limits, not replacements for the original 300/60/10
admission windows. The first collection completed with exit zero and no retry.

Boot ID cf222c07-5476-4f70-9a32-455e99204d0a, PID1 start ticks 655191 and all
observed user/mount/pid/time namespace identities remained unchanged within the
session. Both recorded namespace time offsets were zero. No earlier WSL init is
joined to this one merely because it shares the VM boot ID.

## Observed clock behavior

All 601 rows were retained. Each of the three REALTIME read positions saw the
same two backward intervals. The direct syscall observations were:

| Row | REALTIME delta ns | BOOTTIME delta ns | CPU endpoints of both rows |
| --- | ---: | ---: | --- |
| 85 | -5933526268 | 100809907 | 4 and 4 |
| 380 | -5932402622 | 100567561 | 2 and 2 |

No BOOTTIME, MONOTONIC or MONOTONIC_RAW read position moved backwards. All direct
reads fell between the two surrounding libc reads of the same clock. Observed
CPU endpoints covered 0 through 6, with no within-row endpoint change. This does
not exclude an intermediate migration or prove how libc implemented its read.
Nevertheless, the backward REALTIME effect is visible through a direct kernel
syscall, not solely through the libc read path.

The direct first-to-last differences were 61,005,095,599 ns for BOOTTIME,
66,461,546,336 ns for REALTIME, 61,005,097,216 ns for MONOTONIC and
66,551,008,278 ns for MONOTONIC_RAW. Adjacent REALTIME-minus-BOOTTIME delta
residuals ranged from -6,034,336,175 to 8,762,749,113 ns. The reads are not
simultaneous, so these are diagnostic differences, not a calibrated offset or
clock-drift estimate. Their disagreement is another observation to explain, not
a new admission tolerance or a performance result.

## Synchronization context and limits of attribution

The recorded environment was WSL 2.5.10.0, kernel
6.6.87.2-microsoft-standard-WSL2, systemd 257.13 in HydraCache074Native, and eight
online CPUs. The current clocksource was tsc before and after; available sources
also included hyperv_clocksource_tsc_page, hyperv_clocksource_msr and acpi_pm.
The kernel command line contains hv_utils.timesync_implicit=1 and its boot log
records TimeSync IC version 4.0. Bounded kernel logs contain earlier journald
messages saying time jumped backwards. These identify available mechanisms and
earlier observations, not the caller responsible for the sampled intervals.

The diagnostic distro's systemd-timesyncd and systemd-timedated were already
masked; their selected current-boot journal query returned no entries. A separate
read-only query of the already running Ubuntu found systemd-timesyncd active,
with its earlier initial synchronization recorded in the journal. Ubuntu
reported the same VM boot ID and time namespace time:[4026531834], with zero
namespace offsets. This is context from a later query, not a second clock series
or a claim of simultaneous daemon tracing. Native distro masks alone do not
establish an independent clock environment.

The Windows w32tm /query /status query returned 0x80070426, service not started.
That first metadata error is retained and was not repaired or retried. It does
not prove that no other host or hypervisor mechanism can affect guest time.
No clock-setting caller was traced. The data do not distinguish corrections
requested by another distro, host/kernel synchronization, clocksource behavior
or a combination. In particular, Ubuntu's active service is a candidate source,
not an attributed cause of either exact interval or the earlier original-owner
failure. No particular process has been blamed or disabled.

## Preserved evidence and checks

The separate local-runs/wsl-clock-readonly-20261023 archive retains 56 logical
files in 36 exact SHA-256 objects: observer/analysis sources, GNU and MSRV build
and pure-test logs, raw sample series, pre/postflight identities and metadata,
Windows/WSL status receipts and the separate Ubuntu context query. Its index
SHA-256 is edec06e2ac4fa97126ce14ac5c0e4d1aff9cae7bd39f96bb91391102f3d606e3.
The nonzero Windows metadata exit is preserved separately from the successful
collector exit. Binary copies are excluded. verify checks archive objects offline.

Ten Python controls pass, covering the exact method, incomplete/reordered input,
first anomaly retention, syscall/libc disagreement, PID1 identity parsing,
read-only command boundaries, frozen inputs and exclusive publication. The
standalone observer builds with warnings denied and passes its pure timespec
test on GNU 1.94 and MSRV 1.88. Shell syntax and rustfmt checks pass. No original
clock guard was bypassed to obtain these results.

doc-check, gated-test-check (188 gates registered) and the local non-promotable
0.74 performance contract pass. The Windows workspace cargo fmt invocation
refused with OS error 206, command length; the same formatting check passed
through the existing Ubuntu toolchain. That tooling failure was not an admission
retry. Git index byte audit also matches all 36 staged raw objects and the
separate archive index.

The earlier [original-owner refusal](bootstrap-original-getunit-20261023.md)
and its index remain unchanged:
9f2fd0f457ec6d48f4e87e7bfc2bd1262bbf191cf51b760fe744eb369394b9f6.
The qualification manifest remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
Rust runtime/fixture sources, Cargo.lock and compatibility contract are unchanged
from 8fb0fb5a. There were no original-owner repetitions, clock/configuration
changes, service lifecycle operations, product workloads or server invocations.

## Remaining decision

Clock inconsistency is now demonstrated independently of admission preparation.
The next causal investigation must distinguish the writers of shared guest time
and the MONOTONIC versus RAW rate difference. Existing read-only status and
journal data do not identify the exact setter. Trace instrumentation, stopping a
time agent, changing clocksource or WSL configuration would be a separate
authorized experiment, not an automatic extension of this read-only task.
There is no basis to refresh original origins, enlarge a window, replace REALTIME
with BOOTTIME, weaken ordering or repeat the consumed original-owner lane.
