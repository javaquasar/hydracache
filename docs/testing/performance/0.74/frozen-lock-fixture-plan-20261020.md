# Immutable lock bytes for synthetic artifact tests

Inherits docs/RULES.md. Baseline 23c57b77523ad1cc16b1a79fc90b02a125f8d9b8
contains 437 supervisor sources and the complete 321-check source suite. Its
broader Windows integration invocation refuses the synthetic lock reconstructed
from moving Cargo.lock. The retained fixed-slice packet diagnoses the unchanged
baseline hash 629b483a…7b99 versus the required be46eaf8…2b19.

Repair only the test input. Copy the exact 175300-byte Cargo.lock Git blob from
the already-pinned worker source 62114be0f5da3218706e30d7424acfb5d0579d07 into
a test-only fixture. Preserve its LF bytes on every checkout with a -text
attribute. Include those immutable bytes without runtime Git, network, workspace
reconstruction or fallback. Retain the existing production ROOT_LOCK_SHA256
assertion and every old test body; do not alter production pins or Cargo.lock.

First retain a fresh targeted baseline failure. Add regression controls before
the repair: exact length/hash, seeded byte corruption rejected by the existing
VerifiedBuild verifier, moving workspace-lock substitution and even re-signed
metadata rejected by the unchanged production pin. Keep old ELF/signature/log,
builder policy, path and ownership controls intact. An exact test-only adapter
must reconstruct the old integration file after replacing only its input helper
and removing the precise new test-module hook.

New static guards check the fixture against its fixed source bytes/hash and
all 437 runtime sources, root lock, compatibility, registry and qualification
identity against baseline. No normalization may hide a runtime change. Run the
complete ordinary Windows monitor package, targeted GNU 1.94/MSRV 1.88 artifact
integration tests and strict all-target/all-feature lint. Run formatting,
registry/docs and local non-promotable contract checks. Preserve every further
failure independently; a passing artifact target alone is not a full-suite pass.

Record source/image/recipe identity, the baseline red, resulting target counts
and previous sealed-packet verification in a separate byte-exact evidence packet.
Keep the fixture and test change, evidence, and measured conclusions in separate
logical commits. This test asset is neither a new production artifact schema nor
a dependency update, execution capability or signature upgrade.

There is no runtime optimization, live/fault matrix change, new clock, origin or
deadline, systemd/cgroup creation, server access, installation, supervisor
operation, original send, product workload or qualification in this package.
Actual fixed-root admission, held executable/artifacts, original delivery,
all-writer cessation, lifecycle and reconnect remain subsequent gates.

## Second synthetic consumer found by the broader invocation

At 9f787ff8 the broader Windows invocation passes the artifact target and then
refuses all eight diagnostic_receipts tests at their shared fixture's
verify_receipt. That unchanged file still hashes the moving workspace lock.
It is the same immutable-input hypothesis, not permission to change runtime
validation. Record its targeted red and baseline byte equality before replacing
only that include path with the same frozen fixture. Extend the exact adapter
to reconstruct this second file and retain every old assertion. Then repeat
both artifact and receipt targets and the complete ordinary Windows package.
Any further failure remains independent and must not be counted as success.
