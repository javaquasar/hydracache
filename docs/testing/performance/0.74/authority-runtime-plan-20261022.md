# Exact owned storage runtime integration plan

Integrate the user-approved storage candidate and mutable observer from
`f193fabae86c9bd7a2e778a9463a43984834deb5`, then compile and exercise actual
filesystem/refusal/reopen behavior locally. Starting tree is `6bfcfa73`.
Inherits RULES R-3, R-5, R-7, R-8 and R-10. No server admission follows.

## Exact implementation and separate gate

Install candidate.rs as the authority fixture source without changing its tokens,
and observer.rs as the mutable owner controls. Add only necessary scratch None
initialization and mutable receiver/caller adjustments to legacy test helpers.
Preserve every assertion and all original live checks, call order and deadlines.
Legacy measurement probes may compile but must not execute or generate claims.

A new current-runtime gate compares the complete installed fixture and observer
with independently derived frozen expectations, not with a current-source
fallback. Count-check every compatibility edit and reject unrelated changes.
Append filesystem tests to the existing fixture test module with a separately
guarded complete append. Existing live modules and qualification stay untouched.

Earlier baseline-only source tests remain historical phase checks. Their only
adapter is a versioned view that first verifies an exact permitted full file,
then returns its pinned predecessor for the historical assertions. Independently
check complete adapted test files against their original committed methods.
Reject malformed/partial candidates before any restoration. The new runtime gate
uses raw current files and must not infer acceptance from historical success.

## Baseline and local Linux verification

Record a pre-integration Linux all-target/all-feature compile in nonroot WSL
Ubuntu with Rust 1.94 in a new isolated target directory. Do not reuse or
overwrite old pinned qualification/profile images. Linux work here is limited
to compilation and selected filesystem correctness tests; no live preparation,
original-owner admission, transport/start or ignored measurement tests.

Add bounded tests for cold constructors, real successful fresh reads and scratch
storage reuse, record/marker ordering and cache bounds, all five journal refusal
cleanup boundaries, partial publication/tail refusal versus durable commit,
unchanged original descriptors/stamps/bytes, sticky refusal with no publication,
reopen after success/refusal and absence of cache transfer between owners.
The mandatory closing reopen validation remains unchanged and may warm slots.
Cover wrong ownership/mode, replaced/missing files, symlinks, hardlinks, FIFO,
oversized records and changed bytes with existing and targeted tests as applicable.

List the selected Linux tests before execution, require exact membership and no
ignored entries, run once per frozen final source/compiler lane with first logs
retained, and preserve failures separately. Use logical assertions, not elapsed
time success thresholds. Check MSRV 1.88 as well when locally available. Run
format/check/strict clippy and current/historical source controls. Do not claim
allocation/latency/RSS savings from these correctness tests.

## Scope and evidence

Keep qualification raw SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc,
300/60/10 windows and first Causal refusals 10042390247/10010835739 ns unchanged.
No clock/service changes, root enrollment, server actions, supervisor changes,
product workload or full SIGKILL/lost-ACK campaign. Freeze final method source
before the final captured cohort; retain source snapshots, commands, raw logs,
compiler/kernel metadata and a hashed packet. Document remaining actual process
loss/all-writer/reconnect and server admission gates separately.
