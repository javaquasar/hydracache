# Synthetic observer topology control

This Linux-only local tool isolates kernel observer work and its round snapshots.
It imports the unmodified supervisor observer modules by path. It neither calls
nor replaces any live guard, manager, product process or admission decision.
The original runtime tree and 300/60/10 windows must remain byte-identical.

The complete outputs subphase in both sealed refused prefixes has the same
per-step kernel nodes, roots and edges: 894,608 spans across 46 rounds. Every
count divides exactly by 46, and edge totals admit a uniform-arity canonical
reconstruction. This does not prove original per-occurrence/per-round arity.
`topology.py` verifies
the input inventory and exports those integer counts. Rust validates flow,
arity, an acyclic graph and bounded depth, then constructs a canonical event
tape. Original temporal/sibling order, per-occurrence nesting distribution and
cross-observer relationships were not recorded. Canonical order is synthetic;
matching the aggregate graph does not recover the original execution.

Three modes execute the same tape/body/checksum. `inactive` calls the actual
hooks without an active capture. `kernel` activates only kernel spans. `rounds`
also activates the actual round snapshot/delta observer. Live/auth observers
remain inactive, so this is not full observer-cost measurement. Allocation,
crypto, filesystem, IPC, syscall and live guard body costs are not replayed.

For each mode record walk wall/parent CPU, then separately time report
aggregation and JSON serialization outside that walk. Serialization is checked
by a round-trip and byte count; decoding is not counted as observer cost. The
report is the kernel/round subset, not the original complete phase report.
The standalone crate also has different optimization/callsite context. Never
subtract these times from previous live traces or claim product acceleration.

Fixed controls: seed `0x074c051e`, 46 rounds, three repetitions per mode and six
independently started cases per GNU 1.94/MSRV 1.88 compiler. All six permutations
balance mode positions. No warmup, adaptive duration, sample replacement,
outlier exclusion or thresholds are added. Compare per-case median walk deltas
and retain all signs/values; aggregation and serialization remain separate.

Build/check/test/lint both compilers before capture, using ordinary WSL user,
offline locked dependencies and separate bounded tool caches. The measured
session is serial and has no concurrent build. Every case is independently
bounded by 60-second timeout/five-second grace, parent capture by 65 seconds,
with a 300-second session budget and 330-second outer containment. Existing
capture logs and intent refuse overwrite; failure stops the lane, metadata
drift stops the session. External kill can leave incomplete evidence, never
permission to resume. No clock, service or WSL configuration changes occur.

Tests include imported observer unwind/thread-local/conservation controls,
graph/counter/checksum rejection and independent raw-JSON topology checks.
Evidence is retained in `local-runs/observer-topology-cost-20261021`; old
twenty-three packets and all failed admission evidence remain.
