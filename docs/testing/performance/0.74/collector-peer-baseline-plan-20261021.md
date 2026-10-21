# Pure collector peer vector baseline

Freeze `peer` in `diagnostic_kernel_collector_inputs.rs` at `1cde821f`, before
any growth candidate. It clones supplementary groups into a Vec, pushes the
primary group, sorts and deduplicates, preserving uid and owned output. Keep
that entire production file and all live bodies unchanged; add only a private
test hook to the existing opt-in allocator binary. Historical guards reverse
exactly that hook and still compare complete old files.

Use 256 constructions for each of 31 deterministic cases: one empty list and
absent-primary, present-primary and duplicate-primary shapes at lengths
1/2/3/4/8/16/31/32/33/64. Measure Rust requests for the complete frozen body;
record final lengths/capacities and primary membership. Separately capture
one to_vec/push/sort-dedup construction per case and compare its summed requests
with one complete construction. Do not pool the stage control with the main
corpus or assume an allocator's growth policy before measurement.

Check sorted unique union against an independent BTreeSet oracle, uid/owned
values, input nonmutation, duplicates and boundary integer values. Include
1024 seeded cases and explicit 32-to-33 union/no-truncation cases. Inputs above
the upstream group limit are generic leaf controls, not admitted peers.

Freeze the complete original_collector_inputs body in a synthetic packet
model, substituting only the peer-call observation and fake field receivers.
Retain refused/sender-before/sample/hold/peer/sender-after/unix-seconds order,
first errors and refusal latching, including multi-fault combinations and
reentry. No actual KernelPacket, origin, clock, file or sender is created.
Token checks must restore the complete wrapper and peer bodies exactly.

Run only pure controls/source canaries, affected all-target check and strict
all-feature lint on host/GNU/MSRV, formatting and local structural gates.
Preserve first logs/refusals and nine earlier raw seals with 401 members.
No ignored/manual/live profile, server/SSH/Actions, clock/service change,
installation, supervisor or product workload. Keep 300/60/10-second windows
and both first Causal(Time) failures. This is a baseline, not an implementation,
timing improvement or material/fault/reconnect/server readiness promotion.
