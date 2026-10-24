# Role census reader repair and artifact inventory audit

The retained role-census provider comparison now passes through one exact
synthetic lock-fixture adapter. Sixteen selected cases and strict checks pass.
The artifact inventory audit separately accounts for 530 current source files;
it does not repair or admit the old inventory comparisons.

## Rejected general gateway and accepted provider scope

The first candidate incorrectly routed all role-census reads through the general
historical source gateway. That gateway's predecessor is earlier than the census
phase: it removes material fields and hooks still required by these assertions.
The baseline also exposed the separate material-lock fixture delta in the Linux
provider. The generic candidate fails all three old census cases and is rejected,
not committed. Its exact source and first logs remain in the retained packet.

The replacement changes only `diagnostic_artifacts_linux.rs` as an input to this
test binary. It proves the already-reviewed material-lock fixture substitution
against the existing literal contract and pinned fixture-control helper, while
retaining the role-census hook and all production provider code. Other names,
including material and bundle projection, retain their original reads.

The first narrow candidate passes all three original census cases, but its new
canary exposes an additional boundary: the reused contract compares Rust tokens,
so comments do not change its result. The new reader adds a separate exact
current-byte pin, with CRLF-to-LF checkout normalization only. It still proves
the literal fixture delta; an unknown comment, changed census binding, removed
`O_NOFOLLOW`, unknown body or changed helper now refuses before the predecessor.
This strengthens the test input check rather than relaxing the rejection oracle.
Both narrow first refusals and their source variants are preserved.

Accepted code commit `c178ec522da8e8aeab5a14dc48f41e2b478727cf` changes:

- `crates/xtask/tests/held_complete_role_census_074.rs`: original assertions and
  observation-count checks remain; only the input reader calls the scoped view.
- `crates/xtask/tests/support/census_provider_input.rs`: exact current provider,
  known fixture delta and pinned helper, without a general phase rollback.
- `crates/xtask/tests/role_census_reader_074.rs`: independent full-reader equality,
  scope-preservation and rejection controls.

The final source passes sixteen cases across seven targets: original census 3,
new reader 2, historical inputs 3 and the fresh/PID/scratch/custody readers 2
each. None is ignored or filtered. All-targets check, all-targets/all-features
Clippy with `-D warnings` and the three-file rustfmt check pass. These are
source-contract checks, not delivery, SIGKILL or all-writer cessation evidence.

## Exact inventory observations

The read-only audit compares the current index, filesystem and canonical bytes
against `c91068bfa8b297bb075b63457da235aaa9c6232e`. All 530 paths match; there
are no extra or missing filesystem files, symlinks or reparse points. For each
path it retains mode, Git blob, canonical and working SHA256, and length. Git
blobs are read with replacement objects disabled. No old source file is removed
in any of the seven phase comparisons.

| Historical count | Added paths | Changed old files | Unchanged old files |
| --- | --- | --- | --- |
| 437 | 93 | 29 | 408 |
| 438 | 92 | 29 | 409 |
| 440 | 90 | 30 | 410 |
| 441 | 89 | 31 | 410 |
| 442 | 88 | 30 | 412 |
| 443 | 87 | 30 | 413 |
| 446 | 84 | 31 | 415 |

The audit report SHA256 is
`cfd5e7f1c153f5214dd71da196dfdbac2d1c05806fa14865605e9ce0776bc9c0`.
The [packet](local-runs/role-census-inventory-20261024/index.json) retains its
complete path lists and old/current blob identities, not only these totals.
Its disposition is **observed only**, not an approved inventory bridge. Simply
changing 446 to 530 or filtering all later filenames would conceal 31 modified
old bodies. The old inventory and held-build assertions remain unchanged and
are not rerun with a weaker oracle.

Next implement a separately checked phase adapter, starting from this ledger:
require the exact additions, reject missing/duplicate/unknown paths, and prove
each changed old body through its explicit known delta before exposing a
historical view. Keep inventory projection distinct from runtime provenance and
production execution authorization. The old held-build body mismatch also needs
its own attribution rather than assuming a source-count repair resolves it.

## Retained checks

The packet contains sixty-three members and fifty-one content-addressed objects;
its index SHA256 is
`86946490836001a8f887220767b3d3677a050bbf94664370b09fd548ff8a1251`.
Doc-check, the local non-promotable performance-contract 0.74 and registration of
all 188 discovered gated tests pass. Those structural results do not make the
full xtask package, dependency policy or server-admission gate green.

## Unchanged release boundaries

No supervisor/product runtime source, dependency, original live predicate,
observation frequency or deadline changes. All first runtime failures, old
package timeout and rejected candidate logs remain intact. No server action,
clock/service change, workload, original-owner rerun, qualification or additional
cleanup occurs. The fifteen-advisory dependency refusal remains audit-only.
The [closure audit](release-closure-audit-20261024.md) still blocks server
admission on preparation/delivery, all-writer cessation, lifecycle/crash/reconnect
and actual production enrollment.
