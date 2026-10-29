# Exact Q clean CPU/memory gate: first control refuses clock admission

The clean diagnostic path is implemented and compiled, but **CPU/end-to-end,
RSS and allocator-retention qualification remain blocked**. The first actual B
control refuses the predeclared cross-clock predicate. No CPU or memory cohort
position is dispatched, no P/Q control follows that refusal, and no numerical
Q/B performance or retention claim is made. The previous [functional compatibility
pass](chitchat-compat-result-20261029.md), [fixed-caller allocation pass](chitchat-caller-result-20261029.md)
and [general stateful refusal](chitchat-stateful-result-20261029.md) stay unchanged.

## Clean instrumentation and frozen method

Allocation diagnostics cannot supply clean CPU measurements: the previous C
wrapper maintains pointer tables/tags/counters. This step restores the same
B/P/Q runtime fixtures and adds a normal, non-`cfg(test)` executable. Its actual
link symbol inventory contains neither allocation wrappers nor cost_review,
q_review or stateful_review hooks. Runtime method bodies remain exact; only
diagnostic module/entry exports and a binary target are appended. The already
locked serde_json dependency moves from dev-only to diagnostic normal dependency
with the same version/lock/features. Root Cargo.toml/lock, deny policy and
qualification manifest remain unchanged.

The normal driver preserves constructor, node/KV/max-version, stop-on-first-false,
finish, final Delta serialization and drop. It avoids the old allocation driver's
large stack trace/inventory/counter work. Full semantic/roundtrip exports outside
timing join the same 30 fixed-MTU requests/admissions/wires. No replacement
payload or optimization hypothesis is introduced. Normal images are:

| Variant | Actual executable SHA-256 |
| --- | --- |
| B | `0bcca84ff005884d1b80c8b90e640f3a380091820070c0d4b061e464282fc986` |
| P | `8dd1cf8a396b4964623447cba0e22d48fe6a36b0d30cc2738acea7609f9bd217` |
| Q | `fcd461da19210a63406049a53a4938d58f792df674ae943617e3e2bb06a3d777` |

The [protocol](../../../../tools/chitchat-cost-074/protocol.json) freezes five
independent counterbalanced B/P/Q/Q/P/B blocks per mode, separate CPU/memory
processes, four warm sweeps, 32 whole operations per CPU cell, and six consecutive
memory epochs of eight full-table sweeps. CPU uses current-thread CPU and
MONOTONIC_RAW elapsed time; MONOTONIC/BOOTTIME detect inconsistent admission.
Five block log ratios and a Student-t 95% interval (df4) retain every sample;
the original +3% non-regression ceiling remains. P is explanatory, B remains an
attribution control with a known-inexact estimator, never an acceptable product
repair. These are caller operations, not whole-process startup or network timing.

The memory method retains same-process/boot/start identity, all RSS/PSS/private
bytes and all mallinfo2 fields before and after epochs. Fixed-stack proc reads
precede JSON output. It compares post-warm footprint/allocated-byte growth without
trim, purge, allocator replacement or configuration changes; the strict local
screen allows no extra growth bytes. Free-space fields remain raw explanatory
telemetry. This short screen cannot replace continuous release retention or
complete allocator telemetry. See the [mallinfo2 limitations](https://man7.org/linux/man-pages/man3/mallinfo.3.html),
[clock API](https://man7.org/linux/man-pages/man3/clock_gettime.3.html) and
[Linux proc memory accounting](https://docs.kernel.org/filesystems/proc.html).

## First compile refusal and corrected source context

The original clean B build refuses E0599 because the diagnostic driver calls
`VersionedValue::for_test`, which is available only under `cfg(test)`. No control
or resource cohort ran in that context. Preserve its complete source, compiler
stream and negative outer result.

A separate r1 context constructs the exact same public fields directly:
value.to_string(), original version, DeletionStatus::Set. It does not expose the
test helper or modify any runtime method. Source controls compare that constructor
to the original helper and every original fixture file to its predecessor.
Compiler/dependency caches may be reused; source roots/receipts/outputs are
exclusive. All three GNU Rust1.94 normal builds, all-target/all-feature checks
and strict Clippy pass. The source/method controls pass before execution.

## First actual control and honest attribution boundary

Native metadata records WSL Linux 6.6.87.2, x86_64, glibc2.39, 4096-byte pages,
TSC clocksource and boot `4d382e4f-3fac-472a-be93-8112c5e7bd13`. The control's
original PID is 34794, start ticks 1584005. It uses one thread and returns exit0;
the normal image's full 30-row semantic/roundtrip body equals the prior caller.

The first complete control window records:

| Clock | Elapsed ns |
| --- | ---: |
| MONOTONIC_RAW | 2776883 |
| MONOTONIC | 2634582 |
| BOOTTIME | 2634583 |
| Current-thread CPU | 1286932 |

Reported resolution is 1ns for each clock. RAW minus MONOTONIC is 142301ns,
**5.1244867%** of RAW, above the frozen **1%** admission limit. The native session
and outer result therefore remain negative even though the child exits0.
Original Windows Job/LinuxGroup receipts prove retirement with no cleanup errors.
No old inner pass is promoted. All P/Q controls and all 60 CPU/memory cohort
positions remain undispatched.

This is one short window with differently ordered clock reads. Saved deltas
cannot distinguish clock-rate drift from scheduling/read skew; they do not prove
an NTP, hypervisor or specific WSL root cause. The 5.12% is a control discrepancy,
not a Q/B regression or latency claim. The two control memory snapshots happen
to match RSS/PSS/arena/in-use bytes; one extra control sweep is not a retention
observation or leak/no-leak verdict. No alternative clock, widened limit,
subtraction, retry or changed workload rescues this first refusal.

## Checks and retained evidence

Eight final offline source/fault controls pass. They cover exact fixture/runtime
derivation, direct constructor equivalence, expected clock refusal, no scope/work
promotion, missing cells/processes, clock/ownership ambiguity, memory telemetry,
pointwise growth refusal and the fixed five-block estimator without deletion.
Rustfmt, byte-only packet verification, doc-check and the unchanged local
non-promotable performance contract pass. Post-capture analysis/receipt tools
remain distinct from the frozen historical execution producers.

The [primary packet](local-runs/chitchat-cost-20261029/index.json) retains both
contexts, exact frozen protocol/table/runtime/probe sources, original ZIP inputs,
compiler/normal-image/symbol/command/ownership receipts, native ABI metadata and
the first clock body: 967 logical members / 373 objects, index SHA-256
`9589ebd9014f7b2be15aa8cfedec635e06d042a4b81fe92da3894fdea1008462`.
Compiled images/build caches are excluded. Source frames are
`436ad680283cd70e4472a0bb776822c4f74a7ccbe5d2813009b0d9b1aa0ad737`
and `818dcf2cca31ee308e41e962de491bc24666b0fb5582bd918cebf65ccc30d1b4`.
Final doc/byte receipts live in the separate
[verification packet](local-runs/chitchat-cost-verification-20261029/index.json).

All product live checks/count/order/frequency and original 300/60/10, tests5/10/45,
package780/workspace4800 remain. Local owned outer1200/native-session1100,
physical/native4-GiB floors and five-second poll/retirement remain. No clock,
service, power setting, foreign process, server, installation, supervisor or
product workload is changed.

Next establish an eligible Linux host with the same fixed premeasurement
control; do not replay the unchanged refused WSL attempt. Review host/source/
ownership adaptation and reservation before any new-host numerical cohort. A
healthy control admits only the unchanged preregistered cohort; it cannot turn
this first result green. CPU/RSS/retention, maintained backport/public consumers,
Root security/full gates and original execution/lifecycle/reconnect/trust/host
admission remain open. No replacement hypothesis or Root adoption follows.
