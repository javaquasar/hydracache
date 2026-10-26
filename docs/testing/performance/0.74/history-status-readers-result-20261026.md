# History/status compatibility repair; package remains red

The two additional reader failures from the first v5 package are repaired in
focused local checks. This is test-source compatibility, not a cache runtime
optimization, complete package pass or server admission. The original v5 command
still has its timeout, three failures, one missing verdict and 67 unstarted targets.

## Exact source change

The fixed predecessor is `52abf7ab0efd35557eb3117a45c253e0ebe76469`.
The repair is committed as `d257839d50ebd1509501312b0ff4fef75f90f9f3`.
Checks run before that commit with the predecessor HEAD and the retained final
tracked/untracked source snapshot; this is not a clean-source full package run.
`tools/history-status-readers-074/` declares three complete-body constructions;
descriptor SHA256 is
`188763c292af6f6f603414ace264926009c59d7795cea1047084867609c77023`.
The generator reconstructs them from that literal revision, not observed worktree
approval. Stale, partial, duplicate and unrelated inputs refuse.

1. `authority_rust_history_074.rs` changes only its input-module path. The new
   adapter reaches the already reviewed twelve-path frontier and fourteen exact
   additions while preserving the old 9/81 implementation and ledger. Its selected
   IO body is the old frontier reader's body, with only its fallback alias changed.
   All ten cases, safe-path/protected-path predicates, assertions and mutations stay.
2. `fresh_status_reader_074.rs` constructs the exact existing kernel successor and
   verifies its reverse projection before the unchanged complete-body comparison.
   Both old cases/assertions/mutation inputs stay. Reverse projection adds one fixed
   historical expected-input lookup through the existing cache, not another current
   filesystem observation. No reduction in cost is claimed.
3. `tools/authority-runtime-074/history_reader.rs` retains its original fixed Git
   read/check prefix and old extension checks. It constructs the new whole caller
   before comparison, still returning the original historical body. The constructor
   module is exposed within the crate and reused by the new test: no duplicate
   module load, additional Git read or current-file observation in this helper.

`history_status_readers_074.rs` adds four controls: exact constructions and all
twelve original cases/assertion-token sequences; producer drift/unknown refusal;
selected paths/additions and unchanged 9/81 controls; exact helper/IO preservation
and stale-caller refusal. No runtime source, dependency, deadline or live predicate
is changed.

## First results, not overwritten receipts

The first four-case regression command on the old consumers gives two passes and
two failures. One is the intended construction mismatch. The other is a new oracle
error: it requests the history file from a source revision predating that file.
The correction uses the existing `runtime::HISTORY` API contract; it does not alter
an old predicate. Its initial raw test and log remain in evidence.

The first candidate passes 26 cases but reports unused-module/API warnings. The
follow-up removes those warnings by narrowing the facade and using the exact reverse
projection, without lint suppression. The first format check refuses a chain layout;
that refusal and pre-format bodies remain. Two 82-case compiler cohorts then pass,
but strict Clippy rejects duplicate loading of the new contract in the new test.
Those green tests do not override that lint refusal.

The shared-module correction reuses the helper's module instead of loading it
again. On this final source, the bounded current/MSRV commands each check all
17 selected targets: 82 passed, zero failed/ignored/filtered (78 existing cases,
four new). All-target check, strict all-target/all-feature Clippy, producer
reproduction and scoped formatting pass separately. Each command retains identical
before/after source and HEAD; neither compiler's result is spliced from prior runs.
Documentation consistency and the local, non-promotable performance-contract
check also pass; they do not change the package or production admission verdict.

The [raw packet and offline replay](local-runs/history-status-cleanup-20261026/README.md)
retain exact source bindings and first outcomes. Seven offline integrity controls
check unchanged replay and refusal of altered index, source, counts, first outcome,
birth identity and diagnostic promotion. Full package coverage is not inferred.
Packet index SHA256:
`d0ce29b1044d27e41c7342a20865034bf107fbfc2e094fbeb9123eaa9e995331`.

## Windows cleanup observation has a narrower meaning

The separately scoped [single taskkill control](watchdog-held-taskkill-contract-20261026.md)
uses one synthetic root/leaf with finite self-exit and the existing read-only held
object observers. It does not rerun the old watchdog fixture or change its original
0.35-second timeout and marker-absence assertion.

Taskkill returns 0 and reports terminating both enrolled processes. The same held
root and leaf objects are terminal after the call and in cleanup, with unchanged
creation identities. No cleanup error is recorded. The marker nevertheless exists.
This diagnostic starts the delay after enrollment, not at the original launch cut;
it does not record taskkill begin/end or marker-write timestamps. Consequently,
marker presence is not evidence of writing *after* termination, and these results
do not establish the original v5 failure's cause. They also do not prove arbitrary
writer-alias cessation or qualify a different Windows/Linux execution path.

Next cleanup work must establish the write/termination chronology and review the
actual termination backend. Increasing the timeout, deleting the assertion or
treating PID absence as retirement would not repair that contract.

The retained original `scripts/ci/run-with-heartbeat.py` also exposes three
source-level gaps, independent of the marker's unresolved chronology. Its cleanup
returns immediately if the root has exited, without establishing descendant
termination. Its Windows taskkill invocation discards both streams and ignores
the return code. That invocation has no subprocess timeout and is not followed
by a held-object terminal check. These are observations about this CI helper,
not a demonstrated cause of the old case or a claim about the separate Linux
production watchdog. The next backend repair must cover those paths with owned
process identities, bounded cleanup and fail-loud evidence; PID polling alone is
not sufficient.

There is already a distinct owned-job implementation for registered evidence
commands in `crates/xtask/src/evidence_run/windows_job.rs`, with suspended launch,
job assignment, job termination and emptiness queries. The Python CI watchdog
does not call it. Review that existing boundary and its caller/receipt compatibility
before introducing another containment implementation. Its presence is not proof
that the old watchdog uses containment or that its failing test has passed.

That reuse is not a drop-in CLI substitution: `evidence-run` accepts registered
gates, uses whole-second `u64` timeouts clamped to at least one second and emits its
own evidence receipt. The old helper accepts argv and fractional deadlines and
keeps its CI classification/receipt contract. Sending its 0.35-second case through
the existing whole-second API would change the deadline, so it is not an allowed
repair. Any shared low-level containment must preserve the old caller's timing,
stream and error semantics and retain a termination witness through cleanup,
rather than assuming pipe EOF or root exit proves an empty writer set.

## Remaining gate

The [first v5 full result](package-closure-v5-result-20261026.md) is immutable. Its
frozen source/effect pins must continue rejecting this changed graph. Before another
full dispatch: finish owned cleanup, attribute the existing validation cost and
review the exact new complete source/effect inventory. No unchanged-source retry,
longer deadline, skipped suffix or joined focused cohorts closes the package gate.

Cargo.toml/Cargo.lock and qualification manifest stay unchanged. No new product
hypothesis, dependency update, clock/service/server action, installation or product
qualification occurs. GET-owner stays rejected; C74, production lifecycle,
enrollment and reconnect admission remain unresolved.
