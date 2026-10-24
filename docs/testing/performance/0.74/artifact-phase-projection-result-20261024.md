# Exact artifact inventory projection and held build preservation

All twenty original artifact-fixture cases now pass, including the 437, 438,
440, 441, 442, 443 and 446-file whole-body comparisons and the held-build
rejection controls. The new adapter verifies the complete 530-file current
snapshot before returning its known 446-file inventory. It does not change any
old assertion, expected count, runtime predicate or deadline.

## Current snapshot before a historical phase

The [preceding audit](role-census-and-inventory-result-20261024.md) records 84
additions and 31 changed old files relative to the 446-file phase. Its immutable
ledger is SHA256
`cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0`.
The adapter verifies that digest before parsing the ledger; only this immutable
description is cached. Current observations are repeated on each projection.

Before exposing the old list it requires the exact sorted current paths, Git
index mode/blob/stage entries, filesystem inventory and canonical content hashes
for all 530 files. Missing, duplicate, reordered or unknown names refuse. The
filesystem walk rejects symlinks/reparse points and nonregular files, including
untracked additions absent from the Git list. Working CRLF is normalized to LF
for the canonical source digest; other content differences refuse. The existing
snapshot runner also brackets each check with complete selected source hashes.

Projection applies only to the existing exact Git query
`ls-files -- tools/long-run-supervisor-074/src`. Other Git commands retain their
real arguments and results. All 446 preceding paths remain; the original
successive phase-specific list adapters still reconstruct earlier counts.

The test reader separately uses the existing verified source gateway before
its unchanged normalization chains. The original tests then compare every old
body, including the 31 changed old files, against their old Git versions. No
new old-body exclusion or normalization rule is added. The first saved held-build
failure is repaired by the verified reader input, not by changing its missing,
duplicate or unrelated-drift assertions.

Accepted code commit `45db045d97fbe6f18c0967862f20249e93e40c35` changes:

- `crates/xtask/tests/diagnostic_artifact_fixture_074.rs`: reader imports, verified
  input and the single exact inventory-query branch; original tests unchanged.
- `crates/xtask/tests/support/artifact_inventory_input.rs`: pinned census,
  current index/filesystem/content checks and test-only projection.
- `crates/xtask/tests/artifact_inventory_reader_074.rs`: complete reader-delta
  equality and exhaustive refusal controls.

## First refusals and focused acceptance

The first baseline and candidate invocations fail compilation because the locked
SHA256 output type does not implement `LowerHex`. No tests execute in those two
invocations. The source variants and compiler errors remain retained. A local
byte-to-hex formatter fixes the test helper; dependencies are unchanged.

The corrected baseline passes three projection controls and fails only at the
missing reader wiring. The unchanged canary then passes after implementation,
along with all twenty original cases. An additional index control requires exact
mode/blob/stage/path records rather than only checking source names and content.

The final source passes thirty-three cases across five targets: original artifact
fixture 20, new inventory reader 5, original census 3, census reader 2 and
historical input 3. None is ignored or filtered. All-targets check,
all-targets/all-features Clippy with `-D warnings` and three-file rustfmt pass.
Negative controls cover every one of the 530 paths for removal, duplication,
changed body, read failure, mode drift, blob drift and nonzero index stage; extra
and reordered paths also refuse. The reader canary reconstructs its entire
permitted delta from the pinned preceding file, retaining all old assertions.

The [retained packet](local-runs/artifact-phase-projection-20261024/index.json)
contains raw commands/results, selected source hashes, first compiler and
baseline refusals, first passing source, final source and runner identity. These
are correctness/source-contract results, not product performance measurements,
SIGKILL tests or proof of production provenance.

The sealed packet has fifty-nine members and forty-five content-addressed
objects; index SHA256 is
`6bd92ada951acb5f37c091bdde0c51fd02061097d14a0101763856ef20f0a04d`.
Doc-check, the local non-promotable performance-contract 0.74 and all 188
discovered gated-test registrations also pass.

## Remaining release gate

The original partial/red wider-package packet is not replaced by this targeted
success. Next close the independent current-tree history gate with exact
approved deltas and rejection controls, then reassess the wider xtask cohort.
The execution/watchdog/all-writer, durable completion/cells, crash/reconnect,
preparation fit and actual enrollment gates in the
[closure audit](release-closure-audit-20261024.md) remain separate.

Runtime code, dependency inputs and qualification manifest are unchanged.
The fifteen-advisory dependency refusal remains audit-only. No live observation
frequency/deadline changes, server action, clock/service change, original-owner
or measurement rerun, expensive qualification, extra cleanup or push occurs.
