# Original collector absolute safety alarm

Baseline is `4c33c5ca97ef38e877e1a030fbb3fd06f676b8dc`. The original owner
can retain an unsent pipe message but has no kernel deadline alarm. Implement
the alarm prerequisite before choosing or exposing an execution producer.
This does not constitute an autonomous watchdog or authorize a systemd call.

## Fixed origins and ownership

Consume the original pipe-message owner with no arguments. Use its exact signed
Starting and reservation plus the original authenticated progress packet's
kernel receive point. Derive absolute BOOTTIME deadlines for total reservation
300 seconds, original first-cell Starting 60 seconds, and received progress
10 seconds. The later Starting controller timestamp cannot replace that receive
origin. Checked arithmetic, reversed observations, different boot, invalid state
and expired/equal deadline refuse. Never use now plus a duration to arm it.

Create one CLOEXEC/nonblocking timerfd on CLOCK_BOOTTIME and arm it once with
TFD_TIMER_ABSTIME, zero repeat interval, at the earliest original safety bound.
Inspect its kernel type/clock/flags, pending/disarmed state and remaining time
between kernel samples. Revalidation never consumes an expiry or rearms the FD.
The [Linux timerfd contract](https://man7.org/linux/man-pages/man2/timerfd_create.2.html)
distinguishes this suspend-aware clock from a wake-from-suspend alarm; no wake,
reboot, power-loss or autonomous response claim follows.

Retain alarm, unsent message, bundle, original outputs and fence in one opaque
owner. Drop alarm before message and original fence; errors permanently refuse
the whole chain and construction failure retains it. No descriptor, interval,
clock, unit, key, process, start/send or completion input/output is public.
No new durable record or cross-process protocol is introduced.

Each new alarm operation gets fresh opening/closing full original live rounds,
with the unchanged material/message checks at both sides. Do not recursively
call complete validators at every layer. No existing validator, observation
origin, predicate or public schedule changes. Preserve any observed budget
failure rather than extending admission.

## Tests and gates

First retain a missing-API red result. Add seeded deadline/reference boundaries,
including overflow and equality, and a canary demonstrating that Starting time
would extend the original progress deadline. Exercise real one-shot kernel
readability, no-read validation, disarm/rearm/interval/flags/wrong-object/clock
refusals and descriptor closure. Validate current/MSRV units, API/compile-fail,
all-target check and strict Clippy, formatting, static contracts, registry,
documentation and the local non-promotable performance contract.

Use the authorized nonroot isolated server lane for three complete healthy
original chains per compiler and independent original trust/output/bundle,
message/alarm/origin/pre-refused/real-expiry cases. Require exact errors, sticky
refusal, unchanged three intent documents and fence Busy until owner drop.
Record actual original deadline offsets and authentication counts, not product
performance. Preserve prior proofs at their exact source. No installation,
supervisor stop, clock change, product workload or expensive qualification.

The next execution gate must retain the armed alarm in an independent guardian
before sending, prove creator/thread/supervisor loss and handoff windows, bind
the exact manager invocation/process/cgroup and revoke every writer. A timerfd
alone cannot kill or drain anything and disappears when its final owner exits.
Those remaining obligations are not marked complete by this slice.

## Current disposition

The candidate fails its first healthy original-chain server screen at
Causal(Time), after constructing the timer. The production entry is withdrawn;
the [negative packet](local-runs/original-safety-alarm-20261017/README.md) retains
the complete candidate, failed screen and separate attribution. Kernel/reference
probes remain test-only. Descriptor ownership is not certified by querying a
reusable fd number after Drop. No original deadline or existing check changes.
The proposed grouped observation schedule needs separate agreement; the full
healthy/fault and subsequent guardian/SIGKILL gates remain open.
