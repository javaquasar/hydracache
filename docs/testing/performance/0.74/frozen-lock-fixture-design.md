# Immutable worker lock in synthetic build and receipt tests

The [local repair plan](frozen-lock-fixture-plan-20261020.md) separates a moving
workspace dependency graph from the exact bytes required by the frozen worker
build. Production verification was correctly refusing the synthetic test input;
the production pin was not the defect.

## Why the old fixture drifted

The artifact test reconstructed the root lock by deleting two dependency edges
from the current workspace package block. That no longer reproduced the complete
old worker lock. The receipt test directly hashed the current root lock and
failed the same fixed digest check. Both files and the runtime were unchanged
in the preceding baseline; fixing the first revealed the second in the broader
ordinary test invocation.

The replacement fixture is the exact Cargo.lock Git blob from the already-pinned
worker source 62114be0f5da3218706e30d7424acfb5d0579d07, not another generated lock.
Its 175300 bytes retain the existing production SHA256, and -text preserves its
LF encoding on every checkout. The test helper includes those bytes, still
asserts the original hash, and has no Git/network/fallback path. The receipt
fixture changes only its include path. Cargo.lock and dependency versions stay
unchanged.

## Assertions and provenance

The two new controls reject 32 deterministic byte corruptions through the
unchanged content verifier, reject workspace-lock substitution and reject its
re-signed digest through the unchanged receipt verifier. Static controls restore
every old assertion in both integration files, freeze all 437 runtime files and
production inputs, bind the new fixture to its exact source blob and test
missing/duplicate registration or unrelated assertion drift.

The [evidence](local-runs/frozen-lock-fixture-20261020/README.md) retains original
and newly uncovered failures. It also preserves a new guard formatting compile
error, a seed-spelling lint refusal and the Linux runner's wrong eight-case
receipt oracle. Windows has eight receipt cases; Linux has nine because of its
existing spool/offline control. The corrected runner enumerates the images and
requires all nine, preserving both original successful test logs and the refused
runner outcome separately.

## Confirmed local scope

Selected source 84fee2cc passes the complete ordinary Windows monitor package:
296 ordinary tests and twenty docs, zero failures/ignored, with 79 test-image
hashes. GNU 1.94 and MSRV 1.88 each pass the complete selected artifact target
with 31 tests and receipt target with nine, plus strict lint. Four source-scope
controls, portable lint, formatting, registry/docs and the local contract pass.
The previous 321-check suite is not rerun; this package changes no runtime source.

These are synthetic build statements, reports and test issuers. Correct fixture
bytes do not independently prove a compiled or installed executable, production
issuer selection, real systemd slice ownership, held-unit placement or original
delivery. Existing expiry, all-writer cessation, durable lifecycle, reconnect
and server admission gates remain unchanged. No runtime optimization or product
performance result follows from this test-input repair.
