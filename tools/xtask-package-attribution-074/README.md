# Offline xtask package log review

Python 3.11 or newer is required. This reader verifies the two fixed retained
packet indexes and each referenced object before parsing their Cargo/libtest
logs. It does not invoke Cargo, Git, a process, network, clock or service.

```powershell
python tools/xtask-package-attribution-074/review.py
python -m unittest discover -s tools/xtask-package-attribution-074 -p test_review.py
python -m compileall -q tools/xtask-package-attribution-074
python -m tabnanny tools/xtask-package-attribution-074
```

The current `crates/xtask/Cargo.toml` must match the raw manifest hash in both
captured source maps. This refuses a changed manifest, including changed raw
line endings; it does not invoke current Cargo metadata. The captured source
maps supply the expected default-discovery target catalogs. Custom/disabled
target discovery refuses rather than guessing a schedule.

Cargo's sequential target order connects stderr schedule entries to stdout
blocks; there are no cross-stream timestamps. The reader requires the schedule
to be an exact prefix of the captured catalog, one block per scheduled target,
unique case names and exact agreement of declared counts, observed case verdicts,
summary totals and exits. Unknown, nested, missing or inconsistent data refuses.
An outer timeout can leave one incomplete block or an unstarted suffix. An
incomplete target never receives an estimated duration or a completion verdict.

The output ranks only libtest's rounded `finished in` spans. These are target
wall spans with concurrent cases, not per-test CPU or syscall attribution. Their
sum excludes unscheduled targets and has no duration for the incomplete target.
The nominal 780-second budget minus that sum is not an overhead measurement:
it also contains unreported intervals, the incomplete target, rounding and
coordinator bracketing. The build span is separate. Neither retained package is
green, and this review grants no runtime or server admission.

The AST self-check covers the reader's import/call surface, not arbitrary Python
as a sandbox. Its fixed two-argument backslash-to-slash string conversion is
distinguished from filesystem replacement. Fixtures exercise malformed hashes,
counts, schedules, exits and censored results; integration controls reproduce
both immutable first outcomes. No Rust source, dependency or gate is changed.

## Next attribution protocol not executed

Target times locate the next question but do not establish an optimization.
The largest completed target is `performance_contract_074`. Before proposing
a cache or parser change, a separately scoped diagnostic needs to distinguish
child Git process work, immutable blob decoding and Rust token/AST comparison.
It must retain the exact source, target and case catalog, four case threads,
all assertions and the original package bound. No current-file observation may
be cached or skipped in the name of immutable-source reuse.

Record first outcomes and instrumentation overhead separately; compare only
the same call graph. Count immutable lookup keys and actual child launches,
then isolate one confirmed leaf candidate. Preserve unknown-revision/path,
mutation, cache-key and whole-body refusal controls. A focused pass must remain
separate from the interrupted package, and the incomplete suffix must not be
declared green by combining older runs. No diagnostic rerun, profiler, cache
candidate, increased timeout or server action is dispatched by this utility.

## Fixed-source preservation sweeps

`sweep_review.py` joins the exact retained `3e9bcc06` source, syntax inventory
and successful target verdicts. Its manual seven-loop ledger proves a lower
bound of 3116 direct show/read calls in `diagnostic_artifact_fixture_074`; it is
not a general Rust analyzer, process trace or attribution of the target's
108.31-second duration. No current source or historical outcome is repaired.

`tree_locality.py <captured-tree-directory>` reads an explicitly collected,
bounded seven-tree snapshot without executing Git. The recorded trees contain
3116 distinct revision/path keys and 465 blob OIDs. The zero-hit field models an
initially empty cache fed only those loop requests, excluding other cases and
helper warming; it is not a measured package hit rate. Object overlap does not
admit a cache. New
no-replace tree data cannot attest the old helper's replacement/config state.

`helper-attribution-protocol.toml` prepares the next separate twenty-case,
four-thread diagnostic under the original bound. It remains unexecuted, requires
exact-source observer erasure and preserves every original read/assertion.
No cache, persistent child, package pass or product admission follows. See the
[scope, checks and first-refusal evidence](../../docs/testing/performance/0.74/preservation-sweep-attribution-20261025.md).

## Opt-in direct helper scopes

`helper_contract.rs` proves byte-exact recovery of the old complete artifact
test source after removing only the bounded observers. `helper_trace.rs` and
`helper_model.rs` retain the original helper graph; default-off scopes read no
clock and allocate no event. Active data is per-thread/case, with parent/order,
byte and terminal-buffer checks. `helper_review.py` rejects incomplete graphs
and whole missing cases against the retained original AST catalog.

`helper_capture.py` enforces the separate `helper-execution-protocol.toml`,
original twenty cases/four threads, finite leg order, first-output directories,
source/executable pins and unchanged baseline runtime inputs. It runs only this
local read-only source/Git target, not the full package, product or server.
Fixture filters are explicit and never accepted for the main target.

The first triplet completes, with 12611 profile records locating cumulative
helper elapsed cost in `Command::output()`. It does not separate startup from
child execution or establish overhead acceptance. See the
[measurements, limitations and preserved first refusals](../../docs/testing/performance/0.74/preservation-helper-spans-result-20261025.md).
No cache or persistent-child candidate is admitted.

## Separate fresh command pipe diagnostic

`pipe-execution-protocol.toml` fixes a single 96-call, four-worker ABBA/BAAB
screen before dispatch. It is not a replay or extrapolation of the original
twenty-case target. One 23589-byte historical test source comes from independent
archived bytes; a changed Git replacement/configuration response must refuse,
not become its own reference. Original Git environment and ordinary `git show`
remain inherited. No current-input/source guard is redirected through this probe.

`pipe_capture.rs` leaves `Command::output()` as its unobserved control and uses
explicit null stdin, piped stdout/stderr, spawn and `wait_with_output` only in
the isolated diagnostic. Empty/binary/nonzero/large-pipe/EOF and spawn-error
controls cover this fresh-command scope, not arbitrary preconfigured commands.
A duplicated original process handle permits terminal CPU observation after
`wait_with_output` consumes Child; no PID-based reopening is used. The pipe
path is not the earlier tempfile/polling transport mirror.

Command setup, spawn, handle observation, wait/collection and terminal observation
have separate wall brackets. Child CPU is whole-process lifetime, overlaps those
wall intervals and excludes descendants. Parent CPU is measured only over the
whole concurrent cohort, never attributed independently to simultaneous calls.
Wait/collection includes remaining child execution and pipe draining; this does
not isolate pure copying or waiting that could be removed. The new child has a
120-second outer diagnostic limit; all original 300/60/10 and package limits stay
untouched. One screen does not admit overhead, caching or a persistent process.

Build the new target with `--no-run --message-format=json` through the local
source-preserving wrapper, then invoke `pipe_capture.py <build-label> <new-out-label>`
through that wrapper. It creates an exclusive output directory before preflight,
retains the first outcome, refuses inherited diagnostic overrides and joins exact
source/build/executable receipts. `pipe_review.py` and `test_pipe_review.py` check
all 96 unique slots, original schedule, bytes/status, phase containment, terminal
process identities and non-admission. No server, service, clock or product is used.

The [first screen and exact limitations](../../docs/testing/performance/0.74/direct-pipe-attribution-result-20261025.md)
retain all 96 calls: median control/profile wall is about 71.190 ms, profile spawn
14.940 ms and wait/collection 53.774 ms. The latter still includes child execution;
the similar medians do not establish overhead acceptance or a product speedup.

## Offline reuse boundary audit

`preservation_reuse_audit.py` joins the saved complete helper trace to its original
source/AST and exact current-helper erasure. No Git or workload is run. It retains
literal short/full query spellings, counts repeats without claiming actual cache
hits, and separates current inventory/file/attribute observations. Unknown shapes
and missing coverage refuse rather than being silently classified as immutable.

The [audit result](../../docs/testing/performance/0.74/preservation-reuse-audit-result-20261025.md)
finds 3124 distinct literal queries among 3141 shows, only seventeen occurrences
beyond first. All show arguments omit a replacement-disabling flag; effective
inherited policy is not attested. Ten new controls
include explicit logical ABA/prefetch countermodels; these are not new real-Git
measurements. Neither small literal reuse nor equal blob addresses admit a cache,
eager batch or persistent process. Refresh the independent package-effect review
next; do not spend another cohort tuning this unadmitted exact-key hypothesis.
