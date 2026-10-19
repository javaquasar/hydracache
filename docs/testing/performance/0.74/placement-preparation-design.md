# Preparation time observations across the original fixture

The [approved attribution](placement-preparation-plan-20261020.md) extends the
existing test-only causal and raw-age observations to the entire unchanged
same-owner placement fixture. The previous join-only capture could not observe
an earlier Causal(Time) in prepare_owned_root_guardians.

The wrapper calls the original body exactly once. Catching unwind inside both
captures preserves their bounded first, last and first-refusal observations
after original cleanup, without changing the original panic payload. Facts
are emitted after the call and a panic resumes. The nested join-only capture
keeps its original scope and restores the preparation scope on return/unwind.
No new clock read, admission predicate, retry, callback, challenge refresh or
time origin is added. All runtime modules remain byte-for-byte unchanged.

Kernel-time observations retain their actual issued/now BOOTTIME and boot IDs.
The three bounded raw-age buckets retain original caller file/line and integer
arguments, but do not sample a boot ID. A raw-only refusal must not borrow boot
identity from another observation. First refusal cannot be replaced by a later
healthy cleanup observation. A successful fresh attempt does not reconstruct
the missing tuple in the sealed historical partial cohort.

Five ordinary tests cover unchanged return/error, exactly-once invocation,
first-refusal retention, original panic identity/resumption, nesting/concurrency
and exact already-received kernel time/boot copying. Three source guards remove
only the exact test wrapper and module hook, reconstructing all 430 old source
bodies, compatibility, registry and external creator harness. The new helper
is reachable only through the existing cfg(test) Linux fixture, not the normal
library. The old seven-case assertions and original 46 brackets and 300/60/10
origins/bounds remain intact.

The [separate evidence](local-runs/placement-preparation-20261020/README.md)
distinguishes ordinary compiler/source checks from actual privileged local
cohorts. It admits no original send, systemd held unit, production issuer,
completion, fence release, server operation or product workload.

The fresh GNU attempt reproduces expiry in the original causal window:
884474515163 → 894476360325 ns, age 10001845162 ns, identical actual boot IDs.
The error propagates from owned-root preparation before the healthy join.
Kernel-time and raw-age facts independently match at the original age caller.
This closes attribution of that new Time refusal's predicate, not the missing
historical tuple or attribution of preparation cost. A first custody observation
already at 8736206870 ns motivates fixed-phase cost observation next.

The independent MSRV cohort passes all seven cases once, including the four
previously unexecuted binding/trust negatives. GNU remains red; the full matrix
does not pass. Both clocks/roots and signed fixture inputs are restored/unchanged.
