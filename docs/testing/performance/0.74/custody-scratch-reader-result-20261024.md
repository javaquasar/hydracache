# Custody and document scratch preservation readers

The retained custody whole-body failure and both document-scratch failures now
pass targeted coverage. The repairs change only historical test input. They
retain every old assertion and the complete 469-file and 473-file comparisons;
runtime code, dependencies, live observation frequency and deadlines are fixed.

## Separate causes

The [original wider package result](package-frontier-result-20261024.md) first
refuses custody at `src/bin/diagnostic_held_launcher.rs`. Its activation entry is
newer than that preservation phase. The existing reviewed-input gateway checks
the entire current launcher before exposing its known predecessor. Commit
`edd2c8953f8dd5572f96d436c17259dfc49f3313` connects the custody reader to that
gateway, leaving its normalization chain and all original comparisons unchanged.
Unknown paths are not excluded from comparison.

Document scratch has two different causes. The adopted scratch block contains
the later bounded pidfd-path constructor, while the old frozen-helper assertion
expects its predecessor. Separately, five inspected test readers have acquired
verified IO adapters after the old exact-reader comparison was written.

Commit `6dde20eb5f76b4061b299d16964a77b0701a5132` applies the existing exact
pidfd-path reconstruction only to the origin input. It adds a test-only input
helper for exactly five reader paths. Their complete current bytes must match
the pinned reviewed commit before an immutable predecessor is exposed.
An independent canary reconstructs all five approved current readers from that
predecessor using only literal IO/import substitutions. Changed assertions,
unknown edits, empty inputs and unknown path requests refuse. All other inputs
use the existing strict runtime-source gateway; no supervisor source is edited.

## First outcomes and acceptance

Each independent reader canary first reports one pass and one expected failure
against its unchanged old reader. The passing case proves the existing input
gateway or the five exact approved reader deltas; the expected failure identifies
the missing new reader wiring. Both canary sources stay unchanged between their
baseline and candidate runs. No first result is overwritten or retried to pass.

The custody candidate passes sixteen cases across seven selected targets,
including its full 469-file comparison, canary and previous fresh/PID/input
controls. Its all-targets check, strict all-targets/all-features Clippy and direct
rustfmt checks pass before committing.

The final scratch candidate passes twenty-three cases across ten targets, with
no ignored or filtered cases. These include the complete 473-file comparison,
frozen-helper equivalence, complete sender/pidfd prototype checks, five-reader
assertion preservation, rejection canaries and custody/fresh/PID/status controls.
All-targets check and all-targets/all-features Clippy with `-D warnings` pass;
no lint waiver is introduced. These are source-contract checks, not SIGKILL,
delivery, writer-retirement or performance results.

The five changed Rust files are:

- `crates/xtask/tests/custody_preparation_cost_074.rs`
- `crates/xtask/tests/custody_reader_074.rs`
- `crates/xtask/tests/document_scratch_cost_074.rs`
- `crates/xtask/tests/document_scratch_reader_074.rs`
- `crates/xtask/tests/support/document_scratch_inputs.rs`

The [retained packet](local-runs/custody-scratch-readers-20261024/index.json)
contains commands, source hashes, stdout/stderr, first baseline refusals, runner
source and exact old/current test source. Every bounded check reports unchanged
source and HEAD during its own execution. The original wider-package timeout
and its first failed cases remain in their separate immutable packet.

The sealed index has sixty-three members and forty-eight content-addressed
objects, with SHA256
`9a68a5cd6cd1024ed37dfed6811c843bf1e009876e7b40877a6b846f4f9f50b0`.
Doc-check, local non-promotable performance-contract 0.74, all 188 discovered
gated-test registrations and the five-file rustfmt check pass. A read-only
inventory capture lists 530 unique current runtime source paths; that observation
does not waive the old inventory mismatches or change their expected counts.

## Remaining boundary

The earlier [fresh and PID reader result](fresh-status-pid-reader-result-20261024.md)
and these repairs do not establish a green full xtask package. Next audit the
old artifact source inventories, held-build compatibility and role-census body
comparisons. Require exact current additions and old-file bodies rather than
filtering arbitrary names or widening acceptance.

Cargo.toml, Cargo.lock and the qualification manifest retain their prior raw
SHA256 values. Dependencies remain audit-only with fifteen unresolved advisories.
No original-owner or performance rerun, server action, clock/service change,
qualification, extra cleanup or push occurs. The execution/watchdog/all-writer,
completion/cells, crash/reconnect and actual enrollment gates in the
[closure audit](release-closure-audit-20261024.md) still block server admission.
