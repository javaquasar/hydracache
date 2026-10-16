# Original post-ack causal challenge: local proof and open parallel regression

Implementation: 2799318acf18aa5fc11f5895cbe3d0c558e4488c.
Baseline: 7dff8a6bb04f3786016e699020c87d5ee8889c4e.
[Preregistration](../../diagnostic-collector-causal-plan-20261016.md):
9452e416. [Tiering amendment](../../diagnostic-collector-causal-tiering-20261016.md):
064b05cb. See the [design](../../diagnostic-collector-causal-design.md) and
[contract](../../diagnostic-collector-causal-contract.toml).

## Implemented boundary

An acknowledged original reservation is consumed into a challenge generated with
32 system-random bytes after original revalidation. Its canonical bounded
document commits original request/Reserved/frozen-state bytes, identity and
original kernel issuance. One authenticated Heartbeat must echo the exact
domain-separated digest as its nonce. Issuance is immutable, its exclusive
ten-second freshness does not renew, and all original windows/holds still apply.
The same original authority/storage/fence and receiver provenance remain held.

The immutable byte getter is data-only and may be stale. A digest inspector
cannot establish entropy provenance or recover an owner. A valid response
establishes a post-ack commitment, not full-record review, continuous controller
presence, global replay protection or runtime execution authority. Failures keep
the same refused owners/fence and do not retry.

## Final focused checks

Rust 1.94.0 and MSRV 1.88.0 each pass:

- Four ordinary causal tests; the one new manual proof is ignored there.
- The exact registered manual proof, explicitly executed: one test with four
  fresh real-kernel fixtures. Case 0 accepts a real response, then refuses an
  original-file fault and keeps the fence. Case 1 actually enqueues a correctly
  signed old nonce before challenge issuance and dequeues afterward; the exact
  challenge binding refuses it. Case 2 explicitly fails entropy without fallback.
  Case 3 waits the real challenge interval and refuses a fresh correct response
  with the unchanged issuance deadline. Every case checks owned files/lock and
  refusal of retained context; no output/start is created.
- 29 kernel tests (one pre-existing manual ignore), 30 signed-projection tests and
  28 compile-fail examples. The negative examples fail on type/move/trait/method/
  argument constraints, not unresolved or private imports.

Eight Linux/Windows current/MSRV check and strict-Clippy lanes pass for supervisor
and xtask, with all targets/features and locked offline dependencies. All 175
static contracts pass. Format, doc-check, registry (176 discovered registered
gates) and the local non-promotable performance contract pass. Ship admission is
expected-red. Windows lanes are build/static checks, not Linux runtime evidence.

## Ordinary regression remains red

The untiered default-parallel attempt has 96 passes, four failures and two
existing manual ignores. Three old compositions and the new composition fail
through Authentication(Time), including one old child-thread failure surfaced by
its parent. No concurrent build or whole-workspace formatting was dispatched.
No generic kernel refusal site was attributed by this run.

Under R-5 the separate tiering amendment moves only the new costly four-fixture
composition to a registered external_tool/manual gate. Its only source change
is the ignore annotation. Removing that one line is byte-identical to the
retained ordinary-composition-before-tiering.rs, SHA256
d739cb212a90d6ac4febe7b4585b287abc2d0afb9e3d3d98b2f1c71fe3d7038c.
The 900-second harness guard changes no authentication or product threshold.
Both compiler lanes explicitly execute its unchanged body with --ignored --exact.

The subsequent final-source ordinary run still has 98 passes, one failure and
three manual ignores. The unchanged old heartbeat ownership composition refuses
with Authentication(Time). All old tests remain enabled, with default test
parallelism and no --skip. Again no concurrent build or whole-workspace
formatting was dispatched. This is not a green regression, and test tiering did
not fix the validation-cost/window interaction. Isolated focused success cannot
waive it. Its complete cause and arbitrary shared-load robustness remain open;
these time refusals do not explain historical uninstrumented Kernel(Invalid).
There is no full default-parallel MSRV original-input rerun in this slice.

## Development captures are not final source receipts

The six-test old heartbeat baseline passes. The API red is E0599 before
issue_progress_challenge exists; the shape probe then passes, with an interim
unused-test-hook warning. A missing test import fails E0433 and is fixed. The
first focused five-test success covers only three live cases, before adding the
real-expiry fourth case and before tiering; it is not the final four-case proof.

Unprefixed lane/fmt/static transcripts and pre-tier-* logs precede final tiering.
Three tier-hex-red captures retain the E0277 formatting error in the static hash
helper; final-* build/lint/static receipts are distinct clean runs. Both ordinary
negative transcripts are retained, not overwritten by focused successes.

## Consistency verification and remaining work

Run powershell -NoProfile -File ./verify.ps1 from any directory. It checks the
source tree/thirteen source blobs, twenty-one unchanged inputs, exact artifact
inventory and SHA256/byte lengths, command outcomes and explicit negative/positive
counts. There are 55 command captures and 58 hashed artifacts plus manifest.json.
Transcripts normalize line endings to LF; empty output is represented by a
newline. expected_exit_code describes each retained outcome: exit 1 for the
ordinary regression records a failure, not a waiver or a successful test gate.
A successful verifier asserts packet consistency, not that all tests passed.

This unsigned packet is not binary identity, a clean-build attestation or a
trusted execution receipt. cargo xtask verify and a new SIGKILL matrix were not
run. No new durable write boundary was introduced. Historical process-loss
packets are unchanged, not relabeled as current execution evidence.

Next attribute and address the unchanged original revalidation/time-window
failure without widening deadlines, replacing original owners or suppressing
old tests. Starting then requires an explicit consuming lifecycle authority
handoff, signed state, durable replay/uncertain-outcome handling and actual
SIGKILL/lost-ack checks. Root output creation, authenticated original executable/
streams/process/cgroup start, autonomous watchdog, all-writer cessation, real
host/trust enrollment and installation/rehearsal remain open. No server,
workflow, product candidate, qualification or performance claim is admitted.
No push was performed.
