# Exact security input integration for historical preservation guards

The accepted rustls, WebPKI and Tokio updates now enter the historical source
preservation chain through a whole-body-checked adapter. The previously failing
OIDC lock assertion passes unchanged after its one fresh-reader edit; all 14
manager namespace guards pass. This closes their specific compatibility failures,
not complete package coverage or release admission.

## Current input validation before historical projection

The [active adapter](../../../../tools/security-input-view-074/README.md) checks
four exact paths and complete LF-canonical hashes, performs only pinned inverse
edits and checks each complete output hash. Its immutable rule descriptor is
itself pinned. Other paths are untouched, including unknown-path CRLF bytes;
known inputs allow only CRLF-to-LF conversion. Successful current observations
are not cached or reused. There is no filesystem, Git, clock, environment or
process API in the adapter.

Cargo.toml, Cargo.lock and the manager channel match the fixed accepted producer
`2a4a771d` and project to the pre-security `fa9e7088` bodies. The independent
22-case graph audit still allows exactly three locked-node version/checksum
changes and two declared floors, with unchanged other packages/edges/features,
MSRV 1.88, roots and deny policy. This projection is a test input, not a dependency
downgrade: the installed lock, ownership helper and kernel module are unchanged.

The sole executing root-test edit replaces `std::fs::read_to_string` for its
marked lock read with the existing `runtime_io::read_to_string`. That facade
still reads current bytes freshly and retains its original canonical path and
containment checks. All expected crypto edges, assertions and historical Git
references in that case remain unchanged. No new Git read is inserted into the
fresh guard. The source-successor entry adds only calls to pure exact adapters;
the old gate, IO facade and historical-input helper are unchanged.

## Two additional predecessor mismatches

The first broader manager cohort records nine passes and five failures. Every
failure refuses the current COMPAT body before historical assertions because
the old descriptor predates five already committed diagnostic schema entries.
A separate diagnostic-document adapter verifies the entire current document
against `d3256fc08`, removes exactly the 2,395-byte insertion and verifies the
entire `a67a5299` predecessor. It grants no new schema or security policy. The
actual COMPAT document and original five-path descriptor remain unchanged.
Mutation tests use the exact insertion offset when exercising those old edits.

After this document integration, a second cohort records thirteen passes and
one failure. The remaining root-source gate expects its old IO-prefixed `33c54b0c`
body, which predates the terminal GET-owner rejection. The first new integration
test independently identifies the same mismatch. Its corrected root projection
reconstructs exactly that existing expected text, not a broader predecessor.
An independent whole-body producer test proves that the executing root still
differs from `fa9e7088` only in the one fresh lock read. Current GET-owner rejection
assertions remain executing and the product feature is not reactivated; historical
source strings are never compiled or evaluated.

## Focused controls and retained first outcomes

Windows Rust 1.94 passes seven new adapter/integration tests, all five prior
source-successor tests and all three historical-input tests. They cover complete
independent producers and predecessors, CRLF, every partial/duplicate inverse
edit, stale/empty/mixed/wrong-path bodies, unknown paths, successful-observation
reuse, unchanged IO and old rules, real fresh-read entry joins and the separate
diagnostic document. The exact old OIDC case passes with its original assertion;
the corrected manager cohort passes all fourteen cases. Twenty-two independent
dependency-graph mutation controls pass. These overlapping selected cohorts are
not a complete workspace milestone or a filtered package acceptance.

The same fifteen adapter/history cases pass on Windows MSRV 1.88 and GNU/Linux
Rust 1.94. Windows 1.94 all-targets check, strict all-targets/all-features Clippy,
formatting, doc-check and local non-promotable performance-contract pass. Both
the terminal GET-owner target and its executing root feature-disable assertion
pass separately, with no numerical D3 rerun or candidate reactivation.

The first GNU test execution compiles successfully but all three historical-input
cases refuse before the other targets run: Linux Git interprets the worktree's
Windows `.git` pointer as a relative path. The independent read-only Git diagnosis
retains its exit 128 and exact error. The corrected test environment explicitly
sets GIT_DIR and GIT_WORK_TREE to Linux paths for the same worktree/object store;
its preflight verifies both paths. No `.git`, Git config, clock or service changes.
The changed-context fifteen-case result does not replace that first refusal.

The first new test compilation fails because the installed digest array does
not implement LowerHex; explicit byte formatting corrects only that test oracle.
The first five COMPAT failures, later one root mismatch and the first new
integration failure remain retained. No old lock failure or unchanged V6 attempt
is rerun to obtain a passing result. The corrected executions use distinct source
and rule inputs and append-only result directories.

## Evidence and remaining gate

The [evidence packet](local-runs/security-input-view-20261026/index.json) retains
the command/argv, complete tracked-source snapshots, raw first outputs and changed
candidate inputs. The earlier [read-only audit](security-successor-audit-result-20261026.md)
and its first old lock failure remain immutable.
Implementation commit: `49cbb7bef1255c63491fd5e8abe081bf4168d1df`.
The 284-member evidence index is SHA256
`db231b09668cc784fca1b8464cbcf37119dd45f41b870b8ac14c1ddab2f21431`.
Its offline `verify.py` and six `test_verify.py` controls pass without Git,
processes or execution of stored source. The separate
[28-member verification receipt](local-runs/security-input-view-20261026/verification/index.json)
is SHA256 `f2ed8702502da343749de059e8d2373263669ebe6d3b0a165e244dcb9bc160e3`.
It also retains the final tool README's WSL reproduction note, added after the
primary packet. Test-only re-pinning exercises semantic rejection; it grants no
approval to a modified archive. Explicit -text attributes preserve raw evidence
and the intentional diagnostic fragment's final blank line.
The first staged Git audit compares 157 packet blobs with their physical bytes
without filters and finds zero mismatches; its receipt precedes the final
doc/contract postflight records. Those final checks also pass on the committed
implementation with unchanged source and HEAD.

The qualification manifest remains SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

Next close the fresh current source and transitive effect review before any
changed-source full package dispatch, under the original 780-second/four-thread
contract. The unchanged V6 is not repeated. Nine pinned-database security findings,
the separate root-catalog trust decision, authenticated worker release/all-writer
cessation, durable completion/cells 2–4, crash/lost-ACK/reconnect and compatible
host admission remain open. No product/runtime change, performance measurement,
server/service/clock operation, installation or qualification is performed here;
all live predicates/frequency and original 300/60/10 origins/windows remain.
