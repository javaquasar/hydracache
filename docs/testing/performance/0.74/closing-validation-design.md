# Closing validation observations and unchanged time guards

Inherits `docs/RULES.md`. This attribution adds only test-only observations.
The production age predicate, clock reads, validation calls and deadlines remain
unchanged. The [plan](closing-validation-plan-20261019.md) separates cost
attribution from the real external writer-fault gate.

## The previously missing comparison

`OriginalCustodiedPreparation::validate` performs custody inspection, the full
original grouped validation, another custody inspection and `inspect_deadline`.
The last method inspects the existing alarm, then passes the original challenge
issuance and one freshly read BOOTTIME to `causal::age`. That last age call can
produce `OriginalCustodyError::Challenge(Time)` after the earlier window/final
clock observers have all reported success. The pending alarm is not a substitute
for that independent earlier-issued challenge guard.

The earlier MSRV packet records only enrollment observations ending at
9,724,276,622 ns. Its later failing tuple cannot be reconstructed from that
value. New attempts copy actual arguments at the original age predicate and use
test-only caller location to distinguish the custody call at line 121. Three
bounded buckets retain first, last and first refusal: custody, causal window and
other age sites (including validation-frame clocks). Raw-age samples have no
newly observed boot identity; existing kernel-time observers separately retain
the actual issued/now boot IDs they already sampled.

## Instrumentation does not decide admission

The age observer has no clock, file descriptor, guard factory, replay state or
execution method. Its counters are thread-local, bounded and restored after
nested capture or unwind. Units compare the original result at zero issuance,
backward time, just before expiry, exact expiry and overflow-sized age. A caller
location annotation is cfg(test) only. Removing the exact diagnostic hooks
recovers all 418 previous source bodies, including the original predicate.

One wrapper calls the existing `owned.revalidate()` exactly once and returns its
unchanged result. Facts mode copies existing causal observations and raw ages.
Cost mode also uses existing authentication, live-walk and kernel span observers.
Their wall times overlap and include instrumentation overhead; they are not
exclusive CPU, a syscall census or product latency. Printing follows the observed
call. No mode refreshes progress, re-arms an alarm or bypasses a closing check.

## New observations and the historical refusal

The [new packet](local-runs/closing-validation-20261019/README.md) preregisters
three independent cost attempts per compiler. Its first GNU attempt reproduces
the same custody error with actual age 10,003,292,291 ns; the last earlier
final-clock observation was 9,985,119,124 ns. Another GNU attempt refuses at the
causal-window comparison at 10,068,131,423 ns. These are new observed expirations,
not recovered historical MSRV tuples or evidence of a compiler-specific defect.

The three complete MSRV cost rounds take 218.287–303.411 ms, with 34
authentication/signature checks, 6,688 Boot and 13,376 Namespace observations
each. This identifies work performed after most of the original ten-second
window has already elapsed. No runtime optimization is accepted from these wall
measurements. Missing writer-fault cases require separate facts-only attempts,
actual SIGKILL before original expiry, drained EOF and a still-held fence.

Production root enrollment, original delivery, complete lifecycle/reconnect,
completion and unlock remain separate gates. No server or product workload is
admitted by a local successful fixture or by this attribution.

The four missing MSRV facts-only cases and owner regression now pass. A final
first-dead regression refuses earlier, during enrollment, at observed challenge
age 10,024,697,034 ns. Closing validation and fault injection are never reached.
The packet preserves five real SIGKILL proofs and this exact negative, not a
completed six-case gate. All original comparisons and deadlines remain intact.
The next cost hypothesis must cover preparation/enrollment as well as closing;
changing the clock origin or removing the last round would change the contract.

The subsequent [fresh boot comparison](boot-comparison-design.md) tests one
allocation reduction within those same live checks. It preserves the original
decoder, traversal and observation counts, and measures complete revalidation
and sampling with the existing opt-in Rust allocator counter. The repeated
allocation reduction is separate from elapsed preparation cost and the fresh
unprofiled MSRV fault gate; no historical negative is retroactively promoted.
