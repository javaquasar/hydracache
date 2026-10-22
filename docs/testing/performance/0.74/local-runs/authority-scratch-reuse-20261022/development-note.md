# First observations and tooling repairs

Reference-only source: `187ac5c7`; first storage candidate source: `cefa6709`.
The first runner used a single-quoted output path containing literal `$Phase`.
Both candidate compiler invocations refused at the no-clobber guard before any
step because the reference receipts already existed. No reference log was
overwritten and no candidate observation was collected by those invocations.
The command refusal is described here, not presented as an invented raw log.

The host cohort did execute against `cefa6709` and refused in the new source
canary: removing the retention block left a doubled space in TokenStream's
display text. The exact first host log/receipt and source snapshot are retained.
Repair `c9fc9ec5` reparses the remaining token stream and interpolates a separate
phase path. No reference or candidate algorithm, test corpus or timing helper
changed. The two first actual candidate compiler lanes then passed; neither
measurement lane was rerun. The repaired host cohort passed separately.

Compiler lanes and host source checks were dispatched concurrently. Wall-clock
intervals are consequently preliminary host observations, not isolated benchmark
claims, product speedups, CPU/op, syscall, copy or release-headroom evidence.
All seven first intervals per arm/case remain; no outliers were discarded.

The image has synthetic File/Metadata/Read receivers, not Linux descriptors or
accepted authority records. Full trace equality is asserted in the paired Rust
controls and bound to their pinned source. The offline reader checks the exact
schedule membership, outcome/bounds/order/lifetime invariants and old controls;
it is not a second implementation of the standard-library read algorithm.

The first development invocation of the offline mutation suite had seven passing
tests and one assertion failure: its fixture searched for max_request=66, which
was not present (the bound is 66, not an asserted observed request). The mutation
was corrected to replace an actually observed request of 65 with out-of-bound 67.
This was a test-fixture repair, not a reader, evidence or measured candidate change;
the console failure is recorded here, not represented as a retained raw log.
