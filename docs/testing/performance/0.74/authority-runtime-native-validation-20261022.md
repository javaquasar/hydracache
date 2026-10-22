# Native WSL validation of builds dependency seeds and timeout cleanup

The limited native gate passes: four distinct fresh Linux release images build,
lint and typecheck; dependency seeds stay unchanged; the real subprocess adapter
times out and terminates its owned synthetic leader and descendant. No compiled
runtime image, clock/semantic control or measurement cohort is executed. This
does not establish a CPU/elapsed benefit or open production/server execution.

## Source and scope

The [separate validator](../../../../tools/authority-runtime-native-074/README.md)
cannot call the measurement execute/cohort/freeze path. It reuses exact source
overlays and the unchanged CPU probe, but selects cargo test --no-run, strict
lib/tests clippy and check only. Clock/semantic controls remain mandatory before
any separately authorized future cohort; they are deferred, not waived here.

Initial validator source is 85e5719f. The successful native run uses
fdd8793bdb044c8ce7bc0b463e30a7ea96c06e14 after the portability correction below.
Baseline remains 33c54b0cda8c330b115009cdf092ad002077cedf; installed runtime remains
66d5f371680d085a24de8f2558e1f049c71b9e7b. The runtime package, historical probe,
old readers and all sealed packets remain unchanged.

The local WSL host is nonroot UID/GID 1000, kernel
6.6.87.2-microsoft-standard-WSL2, boot d242b9cf-3f52-4116-ac59-871af8fb82e8.
Boot, namespaces, CPU identity/affinity, configuration hashes, clean source,
input hashes and qualification match before and after. This is a new native
validation context, not a continuation of the old timing cohort.

## First native refusal exposed inventory ordering

The first invocation refused before creating its output directory or starting
cleanup/builds. The frozen CPU inventory had been sealed using Windows Path
ordering, but Linux sorted uppercase README.md before collect.ps1. Independent
comparison found exactly 183 members with identical names, lengths and SHA-256;
only the list order differed. No packet byte was repaired or resealed.

The isolated correction in balanced.portable_history_guard replays the same
safe-member, source/import, complete inventory, facts, all 46 CPU receipts,
allocation predecessor and qualification checks. It compares actual members in
the sealed Windows order explicitly, also refusing case-colliding members.
No workload observation is reordered, removed or reweighted. A regression test
checks identical frozen facts and rejection of inventory/digest tampering.
The original refusal and read-only diagnosis remain in the packet. The second
invocation uses the new committed source, not a retry of a measurement sample.

## Build and seed results

| Compiler | Role | Fresh release libtest | Strict clippy | Cargo check | Image execution |
| --- | --- | --- | --- | --- | --- |
| Rust 1.94.0 | Baseline | Pass | Pass | Pass | None |
| Rust 1.94.0 | Installed | Pass | Pass | Pass | None |
| MSRV 1.88.0 | Baseline | Pass | Pass | Pass | None |
| MSRV 1.88.0 | Installed | Pass | Pass | Pass | None |

All 14 compiler/build/lint/check command receipts pass. Four independent targets
and fresh:false compiler-artifact guards prevent image reuse. Independent audit
checks each final image SHA-256 and ELF64 x86-64 header without executing it.
The root manifests/lockfile, eight allowed role differences, full package and
static includes retain their original source guards. Builds are offline/locked,
with two Cargo jobs and -Dwarnings; both exact toolchains were already installed.

Each compiler's immutable seed has 1,869 permitted files. Membership and bytes
match the original sealed CPU seed receipt before and after all builds. Four
copy manifests match those inputs. Original no-link, mode-preservation, byte
checks and supervisor-artifact exclusion remain active. Neither seed is rebuilt
or substituted. No old cached supervisor image is accepted as a fresh build.

## Real owned group cleanup

The target is a private synthetic Python fixture, not HydraCache. Through the
actual LinuxBackend adapter, taskset starts it in a new session/process group.
The fixture forks one descendant, which inherits the captured streams and waits
without product work. An observer verifies original PID/start ticks, PPID,
group/session and owner, then opens both pidfds while the processes are alive.

At the fixed five-second synthetic timeout, the adapter sends SIGKILL only to
that owned group. The leader receipt has exit=-9, timeout=true and no adapter
exception; the partial output token survives. Both original pidfds become
readable within the separate exit-observation bound, proving both processes
exited even if a zombie awaits reaping. No unrelated group, service or supervisor
is signalled. The adapter's generic error says First measured child refused;
here its executable is the synthetic script, not a compiled measurement image.

This proves the ordinary inherited-group timeout case, not escaped sessions,
cgroup-wide all-writer revocation, uninterruptible kernel I/O or complete
controller-loss recovery. Those production gates remain independent.

The synthetic timeout is 5 seconds, exit observation 10 seconds, and the native
validation has its own 1,200-second budget. None changes measurement 300/3600,
method freeze 600 or production 300/60/10 windows. The run passes its original
validation budget without extending it or replacing an unsuccessful cell.

## Tests and evidence

Twenty-one balanced-runner controls and four validator controls pass on Windows;
the four validator controls also pass under Linux. The real native result is
separate from those mocks. Registration of 186 gates, the local non-promotable
performance contract and documentation checks pass. No full workspace/release
qualification is implied.

Evidence commit is 0e1fffd3. The [native packet](local-runs/authority-runtime-native-validation-20261022/)
contains 91 sealed members plus inventory.json, SHA-256
cc7e20a236abfefa560109098cc5393d9c24ffe35b9de6cae422d5f5efa22f55.
All 92 files match staged raw Git objects. It retains the first refusal,
ordering diagnosis, source snapshots, seed/copy manifests, raw command outputs,
cleanup identities/receipt and independent audit. New inventory ordering uses
case-sensitive POSIX member strings explicitly on either host; old seals stay
byte-for-byte unchanged.

Next, separately authorize the unchanged clock/semantic controls and the complete
64-process cohort if timing attribution is still required. This validation alone
provides no new CPU/end-to-end numbers, native non-regression or server admission.
All live checks, original deadlines, first Causal refusals 10042390247/10010835739 ns
and qualification SHA-256 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc
remain. No clocks/services are changed; no server, installation, supervisor or
product-workload action occurs.
