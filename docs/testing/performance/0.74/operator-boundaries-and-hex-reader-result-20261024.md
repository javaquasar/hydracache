# Restored dependency boundary and exact authority hex reader

The explicit operator replies restrict dependency work to audit/planning and
require the previous ReservedLive observation frequency. They supersede the
earlier general continuation permission. The dependency experiment is reverted;
the next implemented step repairs only historical source-test input, without a
runtime, dependency, observation-count/order or deadline change.

## Exact dependency rollback

Commit `b30c099996a4b4a6f98f48c491fb1e7b0bdac5f0` reverses the three-file code
delta of `e0244861`. Rustls's declared minimum is again 0.23.41 and its locked
version 0.23.45. Both Git blobs equal the pre-experiment baseline `cd6c6931`;
the working bytes are restored from the verified pre-experiment copies as well:

| Input | Restored working SHA256 |
| --- | --- |
| Cargo.lock | `d2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c` |
| Cargo.toml | `18a8a27cf140a1d6eb31c8316ee39bb7d5832866a990346eee16cd0c30de344b` |

The rollback removes the newly added TLS regression target from the ordinary
old-version suite. Its exact source remains in `e0244861` and the immutable
[experiment packet](rustls-security-result-20261024.md), together with successful
candidate outcomes and all first refusals. No history or raw evidence is erased.
This is an authorization rollback, not a failed technical result or a claim that
the old version has become secure.

The reverted working source passes all 178 ordinary RESP cases, with 23 unchanged
ignored cases, no filtered cases and fourteen successful target/doc verdicts.
Its all-targets check, strict all-targets/all-features Clippy and package format
check pass. Those source snapshots were tested before committing the exact
revert as `b30c0999`. The read-only full dependency-policy check again reports
fifteen advisories, with bans/licenses/sources OK. There are no new exceptions.

## Narrow authority hex source-test repair

The first wider xtask packet retained an authority-hex preservation failure at
its registry comparison. Its reader uses raw current registry bytes where the
old assertion expects a known preceding view. The reader's Git blob
`b1780bed472bc12516044e2445cf8272b1f661f8` is equal at `cd6c6931` and `1dc9f08b`.
The saved [wider-package first result](package-frontier-result-20261024.md)
remains unchanged.

Candidate `5161b50bd01f846c0553096e50210f34bbe5058c` routes only the exact registry
path through the existing reviewed-input gateway. That gateway checks the full
current bytes before returning the immutable preceding version. Every other read
and every old assertion remains unchanged. This is test input adaptation, not
runtime trust, policy normalization or acceptance of arbitrary source drift.

The new independent target `crates/xtask/tests/authority_hex_reader_074.rs`
constructs the complete permitted reader delta from a pinned predecessor. Its
first baseline has one pass and one expected failure at the missing exact reader
delta. The unchanged canary source passes after implementation. Negative controls
reject a broad `if true`, another pathname, an unrelated function, a changed old
assertion, an empty registry, an extra registry line and a changed schema version.

Twenty cases pass across six explicitly selected targets: authority hex 2, new
reader canary 2, authority context 2, historical input 3, immutable lookup cache 7
and owned case 4. No selected case is ignored or filtered. Xtask all-targets
check and all-targets/all-features strict Clippy pass; direct rustfmt of both
changed Rust files passes. One previously retained failure is therefore repaired
in targeted coverage, not a complete rerun or green result of the wider package.

## Space and unchanged launch authority

The operator reports removing the old incremental cache. A read-only check finds
the directory absent and about 27 GiB free. Subsequent bounded checks use
`CARGO_INCREMENTAL=0`; Cargo recreates an empty incremental directory. No further
deletion is performed. Exact later free-space readings belong to their capture
time, not an asserted independent cleanup saving.

All ReservedLive observations and their ordering stay intact. No constant-depth
coalescing, clock/service/supervisor change, original-owner rerun, server action,
product performance workload or qualification is performed. The frozen manifest
keeps SHA256 `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The [closure audit](release-closure-audit-20261024.md) still requires preparation
fit, compatible original delivery, execution/watchdog/all-writer retirement,
completion/cells, crash/reconnect and actual enrollment before server admission.

## Structural checks and retained packet

Subsequent doc-check, local non-promotable performance-contract 0.74 and the
registration check for all 188 discovered gated tests pass. These are structural
checks, not a complete verify or package/runtime qualification. The active
security-policy refusal and previous wider-package failures remain red.

The [fifty-member packet](local-runs/source-continuation-20261024/index.json)
retains forty content-addressed objects and index SHA256
`4346cf62829c6565865861df8857c9b4bddbb649cbaa5c0b5a09fe42e4ce2dc6`.
It includes exact commands, raw first outcomes, complete selected source hash
maps, old/current reader source, the unchanged canary and cleanup observations.
At capture, 27,938,852,864 bytes are free and the recreated incremental directory
has zero entries. Since `cd6c6931`, the final code delta contains only the two
xtask reader/canary files; dependency blobs and runtime code are unchanged.

Next audit the retained fresh-status/pid-path body mismatches and historical
source-inventory comparisons individually. A known registry-reader cause cannot
explain every failure. Preserve the old 446/528/530 inventory evidence and first
refusals; do not filter arbitrary filenames, accept unknown current source,
weaken live checks or change observation frequency to obtain a pass.
