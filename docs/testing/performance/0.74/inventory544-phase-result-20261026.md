# Verified 544 to 530 to 446 source inventory projection

The current-source inventory boundary is repaired without changing the immutable
530/446 ledgers or old assertions. The successor validates every one of 544
current source entries before publishing either predecessor list. Nine new and
five original inventory tests pass on Rust 1.94 and MSRV 1.88. The wider artifact
integration remains **7 passed, 13 failed** at its historical body/COMPAT views;
this is not a complete package or release pass.

Implementation commit: `77ccbe7d57203d1c5f263a4046aee0b81e06357e`.

## Independent provenance before worktree observations

The new description comes from exact historical producer tree
`958b3135a15c00b9f6aad41522cef9d1f6608d3a`, compared with the existing ledger's
`c91068bfa8b297bb075b63457da235aaa9c6232e`. Git replacement objects are disabled.
The current worktree/index is checked against that description, not used as its
approval source. There are fourteen additions, three changed old bodies and 527
unchanged old files, with no removals. Per-path producer history, exact trees and
all changed/added source bodies remain in the
[provenance packet](local-runs/inventory544-provenance-20261026/README.md).

The three changed files are `diagnostic_authority_fixed.rs`,
`diagnostic_authority_fixture.rs` and `diagnostic_kernel_origin.rs`. The fourteen
additions belong to the previously retained authority/kernel projections,
controls and test-only rejected namespace/filesystem experiments. Their presence
in an inventory is not acceptance of a rejected optimization. No supervisor
runtime, live observation, timer, process, service or product code changes here.

The immutable original ledger remains SHA256
`cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0`.
The separate successor description is SHA256
`fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f`.
It records old/new modes, Git blobs and canonical SHA256 values for the exact
three changes, plus the fourteen independent additions. Extending the map checks
the old three entries before replacement and forbids collisions/removals.

## Every fresh check precedes historical projection

`artifact_inventory_input.rs` retains its original physical-check code in exact
LF-canonical bytes: query scope, index mode/blob/stage/path comparison, filesystem walk,
symlink/reparse/nonregular-file refusal and fresh canonical-content reads. Its
expected current map now has 544 rows. All 530 old paths remain covered; none is
excluded. Only immutable expected descriptions use OnceLock, not observations.

The first projection requires the full ordered current name set and observes
every current body in that order before returning the pinned 530-file list.
Only then may the existing reader expose the original 446-file phase. An old
530 list/index cannot masquerade as current input. Body drift or read failure at
any position blocks both projections. Two successes do not suppress the next
fresh read or turn its failure into a cached success.

The original five inventory-test bodies and entire twenty-case artifact target
are LF-canonical byte-identical to `2b83dff9d7445aa8a14890b51ae5d4b276b048f7`. Their expected
437/438/440/441/442/443/446 comparison counts remain intact. The original mutation
loops now cover all 544 paths, including each mode/blob/stage, missing/duplicate,
body and read-error case. Additional controls independently match the current
index description to the fixed producer tree and check observation order,
early refusal, complete coverage and old-ledger preservation.

This reader returns test-source names, not execution/enrollment capabilities.
It does not prove arbitrary macro reachability, atomic filesystem snapshots or
production provenance. Old full-body comparisons remain separate obligations.

## First results and focused checks

The new one-case canary first fails as expected: 530 observed versus 544 required,
with unchanged source/HEAD. It is not a repeat of the previous full package.
After implementation, the first candidate passes eight new plus five original
cases. A further fresh-observation regression brings the final cohort to nine
plus five on each compiler. No case is ignored or filtered.

One initial provenance-retainer assertion wrongly compares its four-field row
to the old ledger's extra byte/working-digest metadata. Its original source and
explicit transcript-derived refusal note remain; stderr was not separately
captured. The corrected producer verifies the same mode/blob/canonical fields
against the independent old tree. No hash, source identity or test predicate is
relaxed by that correction.

All-target xtask check, all-target/all-feature strict Clippy and scoped formatting
pass. Six offline mutation controls verify raw object/index/ledger identity,
historical Git blob/content correspondence, independent source pins and stale
index refusal. Documentation and local performance-contract checks accompany
the final evidence. No performance measurement is performed.

## Remaining integration and next work

The first changed-source artifact integration still has seven passing and
thirteen failing cases. Its index refusal is no longer the stop point; retained
details expose downstream exact body and COMPAT/gated-registration views, including
the refusal of unreviewed current registration bytes. Do not infer that every
failure has one cause or suppress a negative control to get a pass.

The old v4 package-review explicitly refuses this changed source before dispatch.
Its pins are untouched. No new full package is run or spliced into the first
[v4 timeout](package-closure-v4-result-20261025.md). Next independently account
for the current body/COMPAT/registration deltas, preserve reconstruction and
mutation assertions, then freeze a new complete effect review before a full
unchanged-bound replay. The audit-only security boundary and all production
execution/retirement/lifecycle/enrollment gates remain open.

Changed code is limited to the inventory helper, its new phase module and a new
ordinary regression target. The [step evidence](local-runs/inventory544-step-20261026/README.md)
retains the first RED, source variants, changed-source integration failure,
current/MSRV controls, strict checks and the stale-v4 refusal. Dependencies,
0.73 artifacts, qualification manifest, native/RESP runtime and original windows
remain unchanged.

The step packet index SHA256 is
`b9aa94f2438307b77e672f966790da3704e4e5e6b341ce73b11ebc0d9dd4f013`.
