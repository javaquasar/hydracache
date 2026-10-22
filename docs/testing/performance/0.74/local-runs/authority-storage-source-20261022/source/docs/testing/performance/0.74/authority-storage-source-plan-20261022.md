# Owned authority storage source candidate plan

Prepare an unapplied source candidate for the fixture journal and a separately
versioned mutable observer. This step closes source review prerequisites only;
it does not enable the candidate in the supervisor or admit a server attempt.
Inherits RULES R-3, R-5, R-7, R-8 and R-10.

## Fixed inputs and allowed changes

Baseline is the exact fixture and observer at
`33c54b0cda8c330b115009cdf092ad002077cedf`. The current runtime must still match
that fixture. Historical readers and their sealed packets remain unchanged.

The candidate adds an owned optional scratch Vec to each Leaf. Constructors keep
their original allocating read and initialize the slot to None. Fresh validation
may take a matching-capacity slot, clear its length, and reuse its storage. All
fresh open, regular/ownership/device/budget, bounded EOF read, metadata, stamp and
byte comparisons remain in order. Retention follows both original comparisons,
requires exact observed length capacity within budget, and keeps the fresh File
alive. No cache is authority and no borrowed/refcell/global pool is introduced.

Leaf refusal clears its slot. Registry refusal clears every record and marker
slot, including previously validated entries, through all five existing latch
paths. The journal's record-before-marker order and all directory/inventory
brackets, publication effects and checkpoints stay unchanged. New owners do not
inherit old scratch. The existing final reopen validation remains mandatory and
can warm the newly created slots before reopen returns.

The versioned observer preserves its allocating constructor reference and all
observation sites. Only the fresh validation helper, mutable receivers and owned
storage cleanup change. Its fixture loops and synthetic test bindings must use
mutable references. The old observer stays intact.

## Source acceptance

An exact, count-checked patch allowlist derives expected complete candidate and
observer files from independently pinned baseline files. Compare full token
streams and parse both as Rust; do not accept extra methods, attributes, effects
or unreviewed edits. Independently unwrap the explicit observation hooks and
compare complete fresh read, validation and refusal wrapper bodies to candidate
methods. Negative controls cover removed checks, order changes, altered storage
bounds, missed eviction, changed effects, duplicated helpers and observer drift.

This is a fixed patch allowlist, not a general semantic equivalence checker.
Syntax and token checks do not establish Linux typechecking, real filesystem
behavior, allocation savings, latency improvement or full SIGKILL/lost-ACK safety.

## Verification and evidence

Run focused source/mutation controls, existing unchanged source canaries, package
format/check/strict clippy, documentation and local non-promotable performance
contract checks. Freeze the method commit before the first captured final cohort;
retain raw logs and a hashed source/evidence inventory. Record any first failure
separately without retry-to-pass relabelling. No old measurement images execute.

Do not modify runtime, qualification bytes, validation schedule, 300/60/10 windows
or first Causal refusals 10042390247/10010835739 ns. No WSL, live profiles, real
fault attempts, server, installation, supervisor, service or clock actions.
After source acceptance, historical Rust guard separation and Linux integration
remain explicit next steps, not consequences of a green historical replay.
