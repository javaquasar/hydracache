# Bounded local WSL clock setter trace

This is a separate diagnostic lane, explicitly approved by the user. It is not
an original-owner retry, benchmark, qualification or server admission. It keeps
the earlier first refusals and original 300/60/10 windows unchanged.

## Frozen method

Use Ubuntu, root only for tracefs, on the pinned x86_64 WSL kernel
6.6.87.2-microsoft-standard-WSL2. Check BTF digest, field offsets/widths and first
argument types before any instrumentation. Reuse the exact read-only modes=0
helper under artur; no packages are installed. All existing root tracer controls,
probe definitions and instance names must pass preflight; never overwrite global
tracing state to restore it.

Register only seven probes in a random owned group: entry/return for
do_adjtimex, do_settimeofday64 and timekeeping_inject_offset, plus entry for
hv_set_host_time. Filter adjustment entries to modes!=0. Return-only adjustment
events can be read-only queries and must not be counted as writes. Record kernel
stacks in only the private instance, using mono_raw and 256 KiB per CPU. No
function-graph, general syscall or product tracing is enabled. The global probe
registry is append-only for exact registrations and exact owned deletions.

One capture lasts 90 CLOCK_MONOTONIC_RAW seconds, with at most 200 read-only
queries and 16 MiB of retained trace text. A detached pidfd/control-pipe guardian
is armed before tracefs changes. It cleans only the owned nonce on owner death,
normal completion or its original 120 RAW-second hard deadline. The Windows
controller has an outer 180-second timeout and never redispatches. The exclusive
intent/output directory is consumed even by a failed capture. Any registration,
loss or cleanup failure is retained, not retried into success.

Compare root controls, boot, PID1 start ticks and time namespace after cleanup.
Retain both owner and guardian cleanup receipts, event formats, per-CPU loss
counters, kprobe misses, raw trace, query series, source digests and command exits.
Offline analysis must refuse incomplete/lossy evidence rather than infer that
an absent event proves no writer. No clock or service changes, original-owner,
product workload or server actions are allowed.

## Attribution boundary

An entry establishes a requested operation, not successful clock modification.
Pair returns on the same kernel PID/function and inspect caller stacks. The
kernel PID is not automatically a distro PID. A successful nested settime with
hv_set_host_time on its stack can attribute that captured operation to the
Hyper-V driver path. It cannot identify who wrote tick before this session or
retroactively prove the setter of an older refusal. Observe ADJ_TICK only when
its mode bit is present; timex.tick alone does not imply a tick-setting request.
Trace overhead precludes a performance claim.

The [kernel kprobe documentation](https://docs.kernel.org/trace/kprobetrace.html)
describes typed argument fetches and probe deletion; the
[ftrace documentation](https://docs.kernel.org/trace/ftrace.html) describes private
instances. The pinned [WSL kernel registry implementation](https://github.com/microsoft/WSL2-Linux-Kernel/blob/linux-msft-wsl-6.6.87.2/kernel/trace/trace_kprobe.c)
preserves explicit return-probe maxactive and argument text when showing entries,
allowing exact planned-definition checks without broad deletion or truncation.

## Step gate

Before dispatch, run Linux Python unit controls covering malformed BTF,
idempotent/partial cleanup, foreign/changed definitions and an external harness
that SIGKILLs its own test owner while the detached guardian uses mocked cleanup.
Those tests do not touch tracefs. Then commit this method and collector. Run
only one live trace. Preserve and report its first result, including failure.

No runtime source, runtime tests, qualification digest or compatibility artifact
changes are part of this diagnostic step. Healthy original delivery, all-writer
retirement and the server rehearsal remain separate gates.
