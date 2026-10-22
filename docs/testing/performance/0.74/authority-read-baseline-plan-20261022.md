# Fresh authority bounded read reference baseline

Starting tree: `94114c62`. Baseline only, no runtime/storage candidate. Copy the
complete Leaf declaration/open/read/revalidate, Stamp declaration/of and regular
predicate into a dependency-free Windows test image. Substitute only synthetic
File/Metadata/open receivers and error conversion; keep generic std Read::take/
read_to_end and every copied expression unchanged. File tokens are not OS FDs.

Use the unchanged isolated allocation counter. Nine healthy synthetic byte cases
run 256 complete reads each, with payload seed 0x074a17b3: empty; one byte; marker
65 bytes; marker in one-byte fragments; record 256 bytes; record 2048 bytes;
2048-byte record in seven-byte fragments; 256 bytes in eleven-byte fragments
with one Interrupted; binary 37 bytes in three-byte fragments. Budgets remain
65/2048. Synthetic byte validity is not accepted JSON/authority validity.

Retain constructor, generic read, drop and whole-return/drop as separate one-call
stages for each case. Stage request conservation and main-per-call multiplication
must hold within each compiler; do not assume equal std allocation strategies
between Rust 1.94 and 1.88. Record all first counts, capacities and read traces;
do not set a candidate threshold after seeing them. No timing or CPU/RSS claim.

Cover all 32 open/before/parent/read/after fault masks and 64 revalidation masks
including the initial held-stamp read, preserving first errors and observation
prefixes. Observe late allocated failures separately from early allocation-free
failures; do not pool these or stage calls with the healthy main cohort.

Check all eleven held/after stamp fields, regular metadata predicates, parent
device mismatch, premature EOF, growth inside/beyond budget, metadata over-budget,
partial non-Interrupted errors, standard-library Interrupted handling, arbitrary
binary content, returned ownership, fake-File destruction and 256 seeded content
mutations. No application retry, live fault, real descriptor or sticky registry
proof is inferred. Preserve caller latching through the existing full source pins.

Source canaries must restore complete copied bodies and reject altered clocks,
host/network/process APIs, unsafe shortcuts or missing checks. Run both first
Windows compiler lanes, strict check/clippy/format, existing source canaries and
offline readers, docs/registry/local non-promotable contracts. Preserve seventeen
old raw seals and 955 members. Any first failure stays retained; a code repair is
a separate source/cohort, never overwriting a first observation.

No WSL, real original-owner profile, server/install/supervisor/product workload,
clock/service change or qualification. Original 300/60/10 windows, first Causal
refusals and rejected boot hypothesis remain unchanged. A later transient-buffer
candidate requires its own preregistration and full semantic/cost gate.
