# Explicit start consent byte contract

This packet covers a portable signed-data prerequisite for authenticated original
start, not process creation or durable Starting. A Heartbeat signature is not
reinterpreted as a start request. The new domain binds explicit start-cell-one
claims; successful signature inspection returns ordinary untrusted data.

Baseline is fdf0474f. Source/tree and selected files are pinned in manifest.json.
The [preregistration](../../diagnostic-start-consent-plan-20261016.md) is 9c223c73;
31400506 corrects test construction/inventory before a successful cohort.
Existing original-owner, kernel, authority/fence, worker, lifecycle, host-lock,
request/challenge/collector handlers and qualification inputs remain unchanged.
Only lib.rs exports the new codec. COMPAT records matched opt-in readers.

The [field design](../../diagnostic-start-consent-design.md) fixes exact digest
conventions and the comparisons still required of the consuming original owner.

## Signed data and the remaining runtime boundary

Body/envelope schemas and the Ed25519 domain are distinct from request,
collector-intent and progress-challenge contracts. Canonical sorted-key LF JSON
binds eighteen top-level fields: explicit first-cell action, consent/reservation
ids, original attempt/Reserved/state/challenge/response hashes, policy digest/
epoch, collector-unit digest, controller/source/preset, revision-two Starting
projection and a positive signing window of at most sixty seconds. Body/envelope
ceilings are 4096/8192 bytes. Unknown, duplicate, future, partial, noncanonical,
oversized or incorrectly signed documents refuse. No migration or downgrade.

Starting-state checks are byte grammar: first cell, revision two, no cgroup,
outcome, cleanup success or admission flags, positive ordered bounded timestamps.
They do not establish those observations or validate an adjacent original
transition from just a previous-state hash. Hash grammar does not prove
challenge entropy or any original document's existence.

The signature inspector accepts a public key as a primitive parameter. It never
selects/enrolls that key, reads trust or private keys, samples a clock, accepts a
peer assertion, opens files or creates runtime authority. Tests deliberately
allow repeated inspection and a syntactically valid historical signing window.
They prove neither replay permission nor freshness. Exported body/envelope data
may be cloned/serialized, but cannot start, publish or release resources.

Next consume the original causal owner, derive/freeze Starting using its retained
kernel context, compare every consent field against live trust/epoch/fence and
original records, then retain the adjacent durable replay/uncertain intent before
root output or process creation. Bootstrap windows are not discharged here.
Autonomous watchdog, all-writer cessation, completion/later cells and host
enrollment/rehearsal remain separate gates.

## Tests and development negatives

Four platform/compiler lanes (Linux/Windows, Rust 1.94/1.88) each pass:

- seven portable consent tests, including 36 signed-leaf mutations under seed
  0x0740c0a2, 30 invalid grammar cases, four foreign signature domains, canonical/
  duplicate/unknown/budget refusals and four independent concurrent checkers;
- four compile-fail API examples, failing specifically because start/publish/
  release methods do not exist;
- affected supervisor/xtask all-target/all-feature check and warnings-denied Clippy.

Both Linux compilers pass unchanged IPC/lease/original-fence/host integrations
(24/20/5/6). No invocation is retried after an integration failure. All 180 static
contracts pass. Five historical static read closures normalize only the exact
new COMPAT paragraph, main registration and reader adapters before their prior
comparisons; a new exact-source test restores all five files and old lib.rs,
preserving every prior assertion and runtime guard. Caught canary panics and the
old IPC injected-loss panic are expected under nocapture, not failed tests.

Formatting, docs, all 176 gated-test registrations and local non-promotable
performance contract pass; ship admission remains expected-red. Final docs/
registry/local/ship receipts follow the evidence documentation edits.

Development transcripts remain separate from the source-pinned final matrix:

- missing-module.log: E0432 from the initial import smoke test, not all later cases;
- private-helper-development.log: E0603, fixed by reproducing the private canonical
  Value-then-Vec algorithm in the integration test, not exposing a new library API;
- inventory-development.log: six pass/one failure because a guessed greater-than-
  forty leaf count was wrong. The schema has eighteen top-level fields and 36
  leaves, now both fixed as exact inventories before mutating every leaf;
- development-green.log: first seven-test green before final formatting/matrix.

These are test development corrections, not changes to a performance/admission
threshold, old test selection, workload, seed or signing window. Logs are complete
captures, not independent binary/clean-build attestations.

The ordinary 99-test original-input group, separate kernel/manual causal/crash
suites and full release verify are not rerun in this byte-only slice. No original
chain is entered or changed. No server, Actions, product workload, timing screen,
qualification, key enrollment, new SIGKILL matrix or push occurs.

## Offline verification

Run verify.ps1 from this packet, repository or its parent. It checks source/tree,
preregistration/amendment ancestry, exact selected paths and frozen inputs, command
results/coverage markers and artifact hash/length inventory. It builds/launches
nothing and writes no files. Unsigned local evidence detects drift relative to
its manifest, not independent execution or release attestation.
