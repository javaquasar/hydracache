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
source/AST and exact retained measured-helper erasure. The installed helper must
separately equal its original uninstrumented baseline. No Git or workload is run. It retains
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

## Diagnostic isolation after package compatibility failure

All sixteen additive controls now live in `package_diagnostics_074`, leaving
the two original authority-context tests and complete artifact helper unchanged.
Observer construction/erasure and mutation controls remain pure source tests;
the artifact's original twenty-case body no longer imports the observer.
The old capture recipes/compiled-source receipts remain historical and are not
silently retargeted. `helper_capture.py` still refuses an uninstrumented helper.
The [focused repair and evidence](../../docs/testing/performance/0.74/package-compatibility-isolation-result-20261025.md)
close seven source-view failures without admitting a full package or repeating
any attribution cohort.

## Offline contract frontier audit

`contract_frontier_audit.py` reads the pinned v1/v2 full-package packets and AST
data only; it never invokes Git, Cargo or a profiler. Literal/default module
resolution reconstructs the same 321 qualified names in both versions, with
114/115 source files respectively. Duplicate/ignored cases, cycles, unsafe or
ambiguous edges and changed identities refuse. This is not a general cfg or
compiler-expansion engine.

Both old attempts have the same 171 successful names and 150 without verdicts.
The latter are not necessarily unstarted or failed. Warning lines are assigned
to surrounding target ranges; the old over-60-second message belongs to a
completed different target. Three fixed-source passing loops prove 1264 direct
show/read calls per attempt, excluding nested work and without timing ownership.
The [result and sole standalone target check](../../docs/testing/performance/0.74/repaired-scope-contract-frontier-result-20261025.md)
retain two reader-accounting refusals and a separate first 321/321 Windows pass.
It is not spliced into the still-red full package, and no cache or optimization
is admitted by the audit.

## Owned Windows process observer foundation

`owned_process_observer.py` is a separate retained-handle collector, not a Cargo
dispatcher. Its sole CLI is `synthetic-control NEW_OUTPUT_DIR`; it runs one short
owned no-descendant Python fixture with exclusive intent/stream/result files.
The library checks attachment identity, lifetime CPU/I/O monotonicity, terminal
signaling and a fixed sample budget without reopening a PID. Model controls
cover first-refusal retention, counter/identity drift, API errors and fixture
timeout cleanup. The fixture's direct-child kill is not process-tree retirement.

The [scope, two synthetic outcomes and remaining adapter boundary](../../docs/testing/performance/0.74/owned-process-observer-result-20261025.md)
keep parent-only CPU, I/O transfer and working-set/private-commit observations
distinct from descendant totals, physical disk I/O, retained heap and per-case
timing. Both native controls pass; no target cohort or overhead acceptance is
claimed. Prove original-wrapper/Cargo/test-image lineage before integrating it.

## Held diagnostic lineage

`process_lineage.py` joins one unique fixed-image shape between two bounded
process-only snapshots. It borrows the original owned root handle, holds every
descendant and checks live parent/child identity and creation order before a
final complete live frame. Counter sampling then uses only the held leaf;
ambiguity or identity drift refuses without reopening or selecting a replacement.

`lineage_control.py synthetic-controls NEW_OUTPUT_DIR` has a fixed three-case
synthetic schedule: two levels, three levels and a wrong-full-path expected
refusal. The [first native results and 28 new model controls](../../docs/testing/performance/0.74/held-lineage-result-20261025.md)
are retained, including the initial test-only short-path expectation failure.
This is not Cargo dispatch or authenticated production ownership: parent IDs
and full path strings do not attest caller credentials, argv or mapped bytes.

## Fixed launch preflight and caller-owned coordinator

`launch_preflight.py preflight NEW_OUTPUT_DIR` has no dispatch mode. It checks a
literal-pinned local receipt, retained no-run Cargo build, declared source inputs,
image/wrapper bytes, original command/CWD and the existing v3 environment/effect
scope. Receipt drift refuses instead of adopting current discovery responses.

`observation_coordinator.py` bounds discovery and makes exactly one held-chain
attachment attempt. First refusal disables further observations while the
caller drains its original owner. It never kills the owner, treats telemetry as
a test verdict or releases live-owner custody on outer expiry.

`coordinator_control.py synthetic-controls NEW_OUTPUT_DIR` runs three finite
synthetic cases through the unchanged 780-second wrapper: healthy, wrong full
image path and injected census failure. The [first results and limitations](../../docs/testing/performance/0.74/launch-coordinator-result-20261025.md)
retain one no-run build, successful preflight, all three wrapper completions and
136 passing Python cases. Actual Cargo topology/target dispatch, durable custody,
overhead acceptance, production ownership and release qualification remain open.

## Dispatch result and observation custody handoff

`dispatch_request.py prepare|dispatch NEW_LABEL` has an explicit zero-spawn
dispatch refusal under the current fixed receipt. `DispatchOnce` in
`dispatch_handoff.py` consumes an exclusive marker before the single caller-owned
spawn; lost ACK or attachment uncertainty cannot authorize replay of that slot.
`ObservationCustody` freezes telemetry and separately checks the original held
processes' terminal identities before releasing query handles. It does not prove
all-writer cessation, durable custody or production completion.

`target_result_join.py` keeps a single target's exact source/case verdict separate
from telemetry and package status. `historical_target_replay.py replay NEW_OUTPUT_DIR`
reads only the sealed old 321-case result; it cannot supply a new live verdict.
`handoff_control.py synthetic-controls NEW_OUTPUT_DIR` exercises healthy/lost-ACK
handoff through the unchanged wrapper. The [results and protocol boundary](../../docs/testing/performance/0.74/dispatch-result-custody-20261025.md)
retain the first session and explicitly leave real-target dispatch disabled.

## Original output binding

`attempt_output_binding.py` checks an absent original-wrapper slot and holds
its live directory/stream objects under the original owner. Terminal sealing
checks file IDs and locks all four inputs against ordinary writers before reads.
An open writer, identity drift or read/cleanup failure refuses without retry.
This is trusted local Windows output custody, not all-writer revocation or
power-loss-safe recovery.
The absence check is not a global reservation: `DispatchOnce` owns the exclusive
consumed marker and the unchanged wrapper exclusively creates its directory.

`bound_target_observation.py` composes the existing dispatch, frozen custody and
held files with the independent target reader; it has no process-launch API or
external log paths. `output_binding_control.py synthetic-controls NEW_OUTPUT_DIR`
has a fixed two-case Python-fixture schedule. Its [first results](../../docs/testing/performance/0.74/original-output-binding-result-20261025.md)
retain both expected refusals and 279 final Python checks. The real Cargo caller
still needs review/authorization; the fixed dispatch CLI remains disabled.

## Short Git lifetime descendant CPU

`git_job_cpu.py BUILD_LABEL NEW_LABEL` is a separate local Windows diagnostic,
not a package dispatcher or a source-reader replacement. The fixed Rust worker
uses ordinary short Git commands in an unlimited private accounting Job with no
kill-on-close policy. Only exact diagnostic argv can activate its two child
entries; ordinary tests are inert. The local owner checks the worker through
its original handle before the unchanged CI primitive's existing resume, without
suspending Git or keeping Git stdin open.

The [design and fixed protocol](git-job-cpu-design.md) require two stable terminal
Job snapshots, complete membership, original worker/launcher lifetimes and
nonnegative subtraction. The
[first results](../../docs/testing/performance/0.74/git-job-cpu-result-20261026.md)
retain a terminal image-query refusal and a single changed-source successful
control, all raw checks and both first old elapsed-output fixture failures.
Residual CPU includes native descendants, not attested native image CPU per call.
No overhead, speedup, package pass or production authority is accepted.

## Per call observer attribution inside the same Job

`git_cpu_overhead.py BUILD_LABEL NEW_LABEL` fixes four alternating ABBA/BAAB
blocks using the unchanged worker. Both reference/profile arms keep the same
unlimited Job and all original byte, identity, terminal and deadline checks.
The [design](git-cpu-overhead-design.md) separates whole Job, worker and external
coordinator CPU and retains every raw sample. It cannot measure Job-membership
overhead or turn a noisy negative delta into negative observer cost.

The [first result](../../docs/testing/performance/0.74/pipe-fixture-git-overhead-result-20261026.md)
retains the sole sixty-four-call attempt and the separate pipe-fixture footer
repair. Current/MSRV pipe cases now pass six each without normalizing output;
the previous first failures remain fixed evidence. Mixed CPU/wall differences
leave overhead acceptance false. V6 and the old 96-call cohort are not rerun.

## Offline CPU owner and phase variance

`git_cpu_variance.py analyze NEW_LABEL` reads only the fixed sealed 108-member
input, without importing a collector, launching processes or reading clocks.
The [design](git-cpu-variance-design.md) retains all sixteen workers, separates
worker/descendant/coordinator CPU and reports the thirty-two existing profile
phase wall observations. Missing reference counters/phases stay null; signed
deltas and zeros are never trimmed or converted into an acceptance claim.

The [result and raw output](../../docs/testing/performance/0.74/git-cpu-variance-result-20261026.md)
locate observed variance by CPU owner and recorded wall span, not a causal API
or native image. Seventeen tests cover immutable input, custody/counter faults,
closed scope and exclusive first results. No new workload or V6 occurs, and
Job overhead, causal leaf timing and release admission remain unresolved.

## Opt in coordinator phase clock brackets

The attribution CLI's exact `--coordinator-frames` flag selects
`coordinator_frames.py` and a separate output root; default mode still uses its
unchanged original frame. The [design](coordinator-frames-design.md) fixes setup,
spawn, wait, terminal reads, decode and close with at most twelve raw wall/CPU
probes. An ordinary clock fault freezes capture while original owner draining
continues; operational refusals and cleanup remain separate from diagnostic
failure. A failed diagnostic stops the cohort, never selects a successful subset.

The [model-only result](../../docs/testing/performance/0.74/coordinator-frames-result-20261026.md)
retains 56 passing model/regression cases, the operational AST differential and
four injected-clock examples. That model-only stage ran no enabled-mode native
control or workload. Wall bounds concern bracketed CPU-read instants, not
effect-only latency; CPU is process-wide and includes bookends. All original
deadlines remain unchanged.

The subsequent [first native control](../../docs/testing/performance/0.74/coordinator-native-result-20261026.md)
completes the unchanged sixteen-worker schedule on exact source/build with all
192 probes valid. Six independent retained-data controls pass. This establishes
enabled-mode compatibility for that diagnostic only: both arms have coordinator
tracing, not an on/off overhead comparison. Phase wall brackets and process CPU
observations remain distinct from pure API cost and production authority.

## Offline nested capture attribution

`capture_attribution.py analyze NEW_LABEL` reads only the fixed native
coordinator packet. The [design](capture-attribution-design.md) preserves both
arms' raw coordinator brackets and separate profile-only nested worker wall
durations. CPU-owner partitions, signed blocks and arithmetic complements
close without adding nested timing scopes or inventing reference phases.

The [result](../../docs/testing/performance/0.74/capture-attribution-result-20261026.md)
retains the first offline output, fourteen new models, seventeen unchanged
regressions and five integrity controls. No process, clock or retained collector
API is used by the analyzer. Zero CPU deltas are not zero work; positive wall
does not isolate IO, scheduler or creation cost. The full package gate and
recorder overhead remain unaccepted, with all live checks and deadlines intact.

## Opt in held owner and decode leaves

Only exact `--coordinator-frames --capture-leaves` on the fixed Git attribution
CLI selects `capture_leaves.py` and its separate output root. The
[design/protocol](capture-leaves-design.md) preserves the original worker, Job,
rows operations, preflights, deadline origins and old modes. Two owner leaves
start after child retention; six decode leaves add at most 48 clock reads,
84 recorder reads including the existing outer Tape, excluding unchanged
original metric/deadline reads. Ordinary leaf failure freezes both
groups while original ownership drains; cleanup and CreateProcessW are not
wrapped. Worker wait/collection remains unchanged and opaque.

The [model-only result](../../docs/testing/performance/0.74/capture-leaves-result-20261026.md)
retains eighteen new cases, all 56 old caller/accounting/outer cases and five
final integrity cases, including preserved first errors. No native leaf control
or measurement runs in this stage. Raw CPU/wall bracket semantics do not isolate
pure API cost or establish overhead, package or production admission.

The [first native leaf result](../../docs/testing/performance/0.74/capture-leaves-native-result-20261026.md)
now completes the exact fixed schedule with all 448 outer/leaf probes valid,
original semantics and zero cleanup errors. Six integrity cases check nesting,
raw arithmetic and first outcomes. Read/entry, not JSON parsing, contains most
recorded decode wall in that input. Both arms trace leaves; overhead remains
unmeasured on/off, and zero leaf CPU or a spawn complement is not causal cost.
