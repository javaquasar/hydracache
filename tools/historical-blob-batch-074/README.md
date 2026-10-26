# Isolated historical blob batch prototype

The Python prototype tests process amortization for a finite, preplanned set
of immutable historical source reads. No existing runtime, source guard or
current-file reader imports it. The isolated native test target below executes
an oracle that uses the Python prototype as one control. No existing validation
graph or live observation changes; this is not release or performance admission.

Current status: only the narrow Rust census test-source adapter is now connected,
with static independent references and unchanged conditional Git reads. The
Python prototype and native batch entry points are not substituted into its guard.
See the [integration report](../../docs/testing/performance/0.74/historical-blob-census-integration-result-20261024.md).

`read_batch(root, requests)` accepts at most 256 explicit requests, each with a
full lowercase SHA1 commit and a UTF8 repository-relative path. Short revisions,
refs, generators, traversal and line-protocol injection refuse before execution.
One `git --no-replace-objects cat-file --batch` process first returns exact commit
objects and then every requested object, including duplicate requests in order.
The parser checks types, object IDs, Git object-content hashes, lengths, delimiters,
complete consumption and aggregate budgets. It does not decode or normalize bytes.

Missing paths and non-blob objects delegate to the original
`git --no-replace-objects show <commit>:<path>` invocation, preserving its stdout,
stderr and exit status. A failed/diagnostic batch process, corrupt record or missing
commit refuses the entire call, without retry or partial results. SHA1 checks
establish the local Git object representation, not a new trust/enrollment scheme.

The Git and parsing phase shares a 30-second diagnostic bound, including fallbacks; it
never extends an original 300/60/10-second admission window. Input is limited to
256 KiB; captured batch/result stdout to 16 MiB and stderr to 64 KiB. Temporary
files avoid pipe-drain deadlock and unbounded in-memory subprocess capture. File
sizes are sampled for refusal, not enforced as a kernel disk quota. Cleanup kills
and waits only the owned Git child, with a separate five-second cleanup wait.
This is not an all-writer or production process-tree retirement implementation.

Repository-overriding environment refuses. Child-only `GIT_NO_LAZY_FETCH=1`
prevents partial-clone network fetch. That local-only boundary and explicit
replacement disabling are prerequisites of this prototype, not replacements for
unflagged existing loaders. Other Git configuration is not rewritten. The public
API does not promise equivalence for arbitrary `git show` flags/configuration,
SHA256 repositories, arbitrary filesystem path encodings or short/dynamic refs.

Run the deterministic-content semantic controls locally:

```powershell
python -m unittest discover -s tools/historical-blob-batch-074 -p test_*.py -v
python -m compileall -q tools/historical-blob-batch-074
python -m tabnanny tools/historical-blob-batch-074
```

Disposable synthetic repositories cover exact empty/binary/CRLF/Unicode/large and
symlink object bytes, multiple commits, order/duplicates, missing/tree fallbacks,
moving HEAD, replacement refs, current-file independence, root separation and
concurrent batches. Protocol/mocked-process controls cover corruption, budgets,
diagnostics, shared deadline and exact-child cleanup. No system service, clock,
server, product process or existing original-owner attempt is involved.

The retained v2 control failure is a fixture-placement error: the moving-HEAD
binary assertion was accidentally placed after a symlink assertion. Corrected
v3 restores that assertion to its own test; it does not change `reader.py`.

Before integration, extract a Rust reader, audit each eligible call site's flags,
path/root/environment and error semantics, preserve all fresh observations and
exact source comparisons, and pass focused/package gates in the original bound.
Most captured historical calls use short revisions, which this prototype rejects;
their coverage cannot be inferred from the full-commit controls. Throughput, CPU,
retained memory, parser cost and observer overhead are not measured here.

## Rust decoder and eligibility audit

`decoder.rs` is a separate protocol component, imported only by the isolated
ordinary `historical_blob_batch_074` and `historical_blob_transport_074` xtask
targets. It consumes a bounded
stream against ordered, independently supplied SHA256 byte pins. It checks exact
object ID/type/length headers, complete bodies, delimiters, content, missing
identity and complete stream consumption; errors publish no prefix. This is not
a Git transport or a port of Python's SHA1 recomputation. Its synthetic OIDs and
caller-created pins carry no original-source/trust authority. There is no default
production pin producer, cache, source reader or integration into an old guard. Existing
`sha2` is reused; Cargo manifests/lockfile do not change.

```powershell
cargo +1.94.0 test --offline --locked -p xtask --test historical_blob_batch_074
cargo +1.88.0 test --offline --locked -p xtask --test historical_blob_batch_074
python -m unittest discover -s tools/historical-blob-batch-074 -p test_eligibility.py
python tools/historical-blob-batch-074/eligibility.py
```

`eligibility.py` verifies the previously sealed Trace2 archive and reports command
shapes without running Git or resolving a revision. Of 12383 `show` calls, 45
have full IDs plus explicit replacement disabling, 1839 have full IDs with default
replacements, and 10499 use short IDs with default replacements. These are counts,
not timing attribution or admission. Six manually reviewed source sites retain
exact file hashes/anchors/excerpts and individual barriers; that site ledger is
not exhaustive and none of its entries is admitted. A matching command shape
does not establish a preplanned batch, unchanged observation order, trusted byte
pins or full error/environment compatibility.

Do not connect the decoder by deriving its pin from the same observed body, or
equate a caller assertion with verified historical identity. The native fixture
gate below covers a bounded transport and a real-Git/Python corpus, not production
reference provenance or arbitrary default-replacement/short-ID semantics.

## Native fixture transport

`transport.rs` uses owned child handles and file-backed stdin/stdout/stderr, with
sampled output budgets, a shared caller deadline and closing checks. Cleanup
attempts only the owned child and separately bounds its reap wait at five seconds.
It does not terminate descendants or revoke descriptor aliases and is not an
all-writer fence. A timeout/output-budget control leaves an unrelated same-image
child alive. Input is at most 256 requests/256 KiB; stdout/results are limited to
16 MiB and stderr to 64 KiB. Sampling is not a kernel disk quota. Bodies copied
into owned outputs are not claimed zero-copy or allocation improvements.

Replacement policy is explicit: default sends no replacement-disabling flag;
disabled sends the exact flag. Short queries remain short. `ShortMissing` is a
separate decoder variant; the old full-only `Missing` still refuses short input.
Preflight validates syntax/pins before Git. Missing/tree fallbacks use the same
policy and deadline and preserve original `show` stdout/stderr/status. Batch
failure or pin drift refuses without retry or partial return.

The experiment is local-only (`GIT_NO_LAZY_FETCH=1`), rejects repository/config
overrides, and explicitly isolates system/global Git configuration. Local fixture
configuration remains in use. This is not arbitrary existing-loader configuration
equivalence. There is no old source-guard call site or production pin enrollment.

`oracle.py` accepts only an existing empty test-owned root, creates two synthetic
repositories, and obtains independent per-object pins through separate Git
commands before the native batch. Six cases cover full/short queries and explicit
default/disabled policy with/without active blob replacement: 54 exact results,
including missing/tree, duplicates, binary/empty/CRLF/Unicode/large/symlink data.
The unchanged Python SHA1 prototype additionally agrees in the two full-ID,
replacement-disabled cases. Fixed commit timestamps affect only fixture metadata,
never host clocks. Pins and this temporary fixture confer no source authority.

```powershell
cargo +1.94.0 test --offline --locked -p xtask --test historical_blob_batch_074 --test historical_blob_transport_074
cargo +1.88.0 test --offline --locked -p xtask --test historical_blob_batch_074 --test historical_blob_transport_074
```

The targeted run has nine decoder and six transport tests (one is the child
fixture entry). It needs Git and Python (`python` on Windows, `python3` elsewhere),
no service, root account or product. The optional test-only
`HC074_BLOB_TRANSPORT_EVIDENCE` path retains first oracle/native outputs; directories
are exclusive, and the cleanup control uses a separate sibling directory to avoid
cross-test ordering dependencies. Source snapshots precede every captured phase.
Before integration, test mutation/ambiguity/config/provenance refusals, supported
platforms and the cost of reference preparation and spooling. No integrated
speedup or complete-package pass is claimed.

## First mutation controls before caller integration

The first `historical_blob_fault_074` version characterized stale pins and
deterministic mutation cuts on disposable repositories. Its original source and
all first outcomes remain in the linked fault report and content-addressed packet.

The first admission gate was **false**. Under default replacement policy, a missing
pin could validate, a replacement commit could introduce the path immediately
before fallback, and the unchecked `show` result returned the new bytes successfully.
Removing the replacement immediately afterward restores the initial ref state
but does not undo the returned bytes. Explicit replacement disabling withstands
this particular control; it does not establish general config/ref authority.
The old passing characterization deliberately reproduced the counterexample,
not a safe refusal. The following candidate closes this output gap; it does not
admit an old source loader.

Pre-batch stale blob content, changed replacement configuration, stale absence,
ambiguous short commits and missing objects produce exact refusal classes.
An unrelated branch/HEAD change leaves full-ID bytes unchanged. The installed
Git's Unicode tree `show` output did not change with `core.quotePath`; the first
opposite expectation is retained as a negative fixture hypothesis. Likewise,
missing loose objects yield a batch `missing` record and `Protocol(Header)`, not
the initially expected `Process`. Neither correction changes the reader.

```powershell
cargo +1.94.0 test --offline --locked -p xtask --test historical_blob_fault_074
cargo +1.88.0 test --offline --locked -p xtask --test historical_blob_fault_074
```

The fault oracle's explicit `--fault-controls` mode prepares only its two needed
reference profiles (18 results); the default six-profile/54-result oracle is
unchanged. Both fault targets pass on current/MSRV. The first MSRV attempt using
the full preparation hit its unchanged 30-second bound before fault controls.
Later concurrent current/MSRV combined runs pass the new fault target but hit
that bound in the old full-oracle target (five other transport cases pass).
Those combined runs are red and are not retried or relabeled as complete passes.
The earlier current-Rust complete 9/1/6 pass remains a separate first outcome.
See the retained [fault report](../../docs/testing/performance/0.74/historical-blob-fault-result-20261024.md).

## Pinned fallback candidate

`read_pinned` and `read_pinned_observed` require a positional optional
`FallbackPin` beside every original decoder pin. Blob entries use `None`.
Missing/non-blob entries require a binding prepared from an independent `show`
reference **before** reading: exact raw query, original object/missing pin,
replacement policy, stdout/stderr lengths and SHA256 digests, and terminal exit
code. Private fields keep the binding immutable during the read. Constructing
this type asserts a reference; it is not source enrollment or authority.

The batch still checks the original object pins. Fallback still executes the
original `show` spelling/policy, with every existing check and the same shared
deadline. After the effect, output verification must match the pre-supplied pin
before any result escapes. A byte, length, diagnostic or status change returns
`FallbackOutput`, even if the ref/config is restored in the after-fallback cut.
An error on a later item returns no earlier blob/fallback prefix. No pin is
refreshed from observed output, no revision is expanded and no replacement policy
is silently changed. This proves pinned output identity, not continuously stable
repository configuration.

The old raw-pin-only `read`/`read_observed` paths remain usable for blob-only
requests. Missing/non-blob requests without output bindings now fail before Git
or observer effects with `FallbackRequired`. Mismatched binding count, query,
original pin or policy returns `FallbackIdentity`. Reference and aggregate
output budgets remain bounded. Unknown/signal/abnormal reference statuses outside
0..255 refuse; absence cannot be enrolled with a successful status.

The updated fault target preserves all original stale-pin controls and the old
ABA cuts, now asserting safe refusal. Added controls cover tree-ref ABA, local
replacement-config ABA, binary stream/status drift, identity swaps, duplicate
fallback order, no prefix publication, budgets and the original deadline.
Current Rust and MSRV each pass 9 decoder, 5 fault and 6 transport cases, including
the unchanged six-profile/54-result reference corpus. The current run uses
separate sequential commands; MSRV runs all three targets in one command. Earlier
deadline/fixture refusals remain historical first outcomes, not overwritten by
these passes. Thirty-one Python controls and current check/strict lint pass.

The [pinned fallback report](../../docs/testing/performance/0.74/historical-blob-pinned-fallback-result-20261024.md)
records the candidate and exact evidence. The output-binding component gate is
green; real caller admission, independent production pin provenance, Linux
coverage, preparation cost and the old incomplete package/release gates remain
open. A final ref/config comparison alone still cannot exclude ABA, and fresh
live observations must not be removed during any later integration.

## Census caller provenance and local Linux checks

`caller_pins.py` prepares only the two blob pins used by the unchanged
`census_provider_input::frozen`. BASE uses the existing normalized SHA256 contract
only after proving raw bytes are canonical LF; the complete contract identity is
checked against the earlier sealed census intent. REVIEWED uses the earlier
sealed inventory's exact blob ID, length and canonical SHA256. The packet/index,
inventory and caller are verified before Git. A raw blob ID is derived only after
the independent byte hash matches. These are existing test-source trust anchors,
not production issuer keys or server enrollment.

`historical_blob_caller_074` invokes the actual old caller in disposable child
processes, without editing or redirecting it. Clean-profile native reads match
both pinned bodies, and original/acceptance/view behavior remains exact. But
child-local `GIT_TRACE=1` is a counterexample: the old caller accepts success and
ignores captured stderr, while native batch admission returns `Process`.
Characterization passes on current/MSRV; **caller admission remains false**.
Conditional read order, ambient environment/config and error compatibility must
be addressed explicitly before any integration or preplanned batching.

`linux_session.py` checks exact copied component sources in a private temporary
fixture using already installed GNU 1.94/1.88 toolchains and offline packages.
Its minimal `linux-fixture.toml` is not the workspace manifest or package gate.
All 33 resolved package identities/checksums must belong to the existing root
lock. The first `generate-lockfile` attempt correctly refused a foreign bitflags
resolution before building; the corrected seed uses ordinary metadata resolution
from the old lock with the same strict subset gate. Both first outcomes remain
retained. No dependency is fetched or installed and the root lock is untouched.

The local WSL session passes all 20 component cases per toolchain, including both
six-profile/54-result oracles. It records clocks/boot, exact inputs, phase outputs
and cleanup; both own temporary roots are removed. No stopped distro, service,
clock or server is changed. WSL's MONOTONIC/RAW disagreement remains recorded:
these are Linux byte/process semantics, not fresh-clock or production admission.
The concrete caller audit runs separately in the real Windows checkout; a Linux
component pass does not prove Linux caller/environment equivalence.

See the [caller and Linux report](../../docs/testing/performance/0.74/historical-blob-caller-linux-result-20261024.md)
for exact sources and first refusals. The new Python suite has 39 controls;
current check/strict lint and the unchanged caller's regression controls pass.
No runtime loader is integrated or numerical performance claim made.

## Census inherited environment and error adapter

`census_caller.rs` is a separate, test-only compatibility candidate. Each reached
read issues the original literal `show` command with inherited environment/config
and disabled replacements. It keeps successful stderr, status-before-UTF8 error
ordering, exact raw source pins and CRLF-only normalization. Typed failures retain
completed command output; there is no native-to-old retry or eager second read.
The lazy helper/BASE/REVIEWED/body schedule and all its early stops are tested.

Five Windows child profiles compare 25 old/candidate operation pairs per
toolchain; eight caller cases and two unchanged regression cases pass on current
Rust/MSRV. The first fixture's invalid-original panic remains retained. The
generic batch reader still refuses successful tracing stderr as before.
The [contract report](../../docs/testing/performance/0.74/historical-blob-census-contract-result-20261024.md)
distinguishes these finite-profile results from integration: the bounded,
file-backed, typed-error adapter is not arbitrary unbounded/panic-API equivalence
or production environment admission. The old caller is not redirected. Linux
caller proof and the error facade remain before a switch; no speedup is claimed.

## Local Linux caller and old panic boundary

The subsequent [Linux and panic report](../../docs/testing/performance/0.74/historical-blob-census-linux-panic-result-20261024.md)
closes those two isolated gates. `panic_api` preserves old process/helper/provider
messages and std's actual UTF8/IO unwrap payloads. Transport retains an original
IO sidecar while its old public refusal behavior, checks, budgets and cleanup
remain. New pin/deadline/budget refusals stay explicit, without fallback or retry.

`linux-census-fixture.toml` and the explicit census runner profile exercise the
unchanged caller in a new private Git fixture built from sixteen existing objects
and exact source/evidence bytes. Offline package identities must remain a subset
of the old lock. The first missing-serde/std and ambient-config refusals remain
retained; neither is fixed by relaxing a guard. Windows and GNU/WSL pass 32 cases
per toolchain on 1.94/1.88, including 25 old/typed/facade operation triples and the
old strict batch counterexample. Python has 40 controls; strict local gates pass.
The real caller is still unmodified. Integration must preserve the revision
allowlist and source provenance without importing diagnostic producer reads into
the conditional guard schedule. WSL clock authority and performance admission
remain closed.

## Connected census source caller

`census_reference.rs` supplies two literal metadata rows from earlier sealed
evidence without runtime preparation reads. `caller_integration.py` audits the
whole descriptor template and exact caller/import delta against the old packet
and original caller. Allowlist and panic signatures remain; selected views share
one deadline across both historical reads. Helper/body checks are not cached or
removed. The legacy contract import is one audited re-export, not a duplicate.

Windows passes 37 cases per current/MSRV toolchain, including the real consumer;
GNU/WSL passes the five-target 33-case fixture on both. Five profiles compare
old/typed/facade/connected argv, read order and panic payloads. Python has 45
controls; strict local gates pass. First source-expectation/lint refusals remain
retained. No extra Git command enters the guard; no speedup or production admission
is claimed. Full package replay and complete-path attribution remain separate.

## Connected caller cost and package inventory

`crates/xtask/tests/census_cost_074.rs` is an opt-in Windows/debug source-reader
diagnostic, not a product benchmark. Without `HC074_CENSUS_COST_INPUT` its child
case performs no measurements. Its ordinary protocol case checks the fixed
six-block ABBA/BAAB ordering and compiles both old public surfaces. Five complete
operations retain the actual legacy/current implementations and all predicates;
separate components measure reference construction, SHA256, interpretation,
helper read/hash, body predicate and pipe/owned Git capture. Component intervals
are not disjoint phases of a complete call. `observer` is an empty-loop control,
not a measurement or subtraction of every clock/CPU-observer instruction.

`census_cost.py BUILD_RECEIPT NEW_DIRECTORY` requires an append-only destination,
a successful source-bound no-run build, earlier independently sealed body pins,
the unchanged qualification digest and a 4-GiB free-space floor. It inherits the
same environment for both paths. Only the app's literal `GIT_PAGER=cat` is
permitted among Git overrides; no variable is removed or rewritten. Each original
caller deadline remains 30 seconds; the separate diagnostic has a 180-second
outer bound. A refusal is retained, not retried automatically. CPU is current
process `GetProcessTimes`, excluding Git children, and may be coarsely quantized.

`--review-saved CAPTURE OUTER_RECEIPT` only reads the saved first child outcome.
It permits the exact original libtest status prefix on the first sample, checks
the complete 120/48 schedule and preserves the initial parser refusal. This does
not dispatch or reclassify the original wrapper as successful. The first cohort
is slower on the connected path in this local debug observation; it is not a
release throughput claim or an admitted transport optimization.

`package_replay_audit.py` reads the default target catalog, literal path
candidates, source hashes and earlier sealed package outcomes. Its sightings are
lexical, including strings/comments, not expanded Rust AST or effect reachability.
The result explicitly leaves execution-effect clearance false. The package
command still has its original 780-second bound and four case threads; this
inventory does not run it, filter targets or fill an earlier incomplete suffix.
See the [cost and audit report](../../docs/testing/performance/0.74/census-cost-package-audit-result-20261024.md).

## Transport capture and default Windows effect review

`transport_profile.rs` is a diagnostic mirror, not an installed transport change.
`transport_attribution.mirror_check` erases observer lines to the exact original
function and checks private helpers/constants separately. Actual `transport.rs`,
census pins/read order and the 30-second deadline/5-second cleanup stay unchanged.
One six-block ABBA/BAAB cohort compares twelve unobserved installed captures with
twelve mirror profiles. Flat stage clocks and the original held child handle
separate temporary-file setup, spawn, metadata/wait, sleep, read and terminal CPU.
Sleep overlaps useful child work; its duration is not an avoidable-cost estimate.
There is no candidate acceptance or observer-overhead subtraction.

`effect_syntax.rs` parses actual Rust items and standard assert/format operands;
quoted source is data. `syntax_audit.py` follows both literal and default module
edges to a bounded closure. The first lexical catalog omitted two default census
modules; the 334-file closed inventory retains those inputs too. This is syntax
and source review, not arbitrary compiler/procedural-macro expansion.

`package_effects_review.py` records a manual entry-point review scoped only to
the default Windows package/four case threads/no ignored cases. It requires a
literal reviewed AST digest, exact source hashes and 21 independently pinned
script/API inputs. It cannot automatically approve a changed inventory. Its
environment preflight rejects inherited fixture controls, executable/lazy-fetch
Git settings and reparse scratch roots. Registry executors, self/Python fixtures,
protoc scratch generation, current Git reads and dependency API slices have
explicit classifications; service names in source assertions are not dispatches.
The receipt is not Linux/all-feature clearance, security-policy green or product,
server, installation or qualification admission. Full replay still needs its own
complete verdict under the original 780-second bound.

## Separate current Windows scope v2

`package_effects_review_v2.py` records a distinct manual review of 342 sources,
four changed/eight added inputs and 28 auxiliary pins. It does not edit or refresh
the v1 pin. New build/config and missing-executable refusals supplement the old
environment checks. The pinned source-only fixture is retained in
`package-effect-syntax-v2-20261025`; it is not an execution result.

`package_replay_v2.py NEW_LABEL` is preflight-only. The explicit `--execute` option
uses only the original command/four threads/780-second wrapper, whose two source
hashes are required. No target/filter/timeout override or automatic retry exists.
Source must be clean including untracked inputs. Ignored wrappers must already
exist with their retained exact bytes; the tool does not fetch or install them.
Destinations are exclusive and intent/failure files are written before further
validation; this is diagnostic retention, not an fsync/crash-durability protocol.

The [first actual v2 replay](../../docs/testing/performance/0.74/package-effect-review-v2-result-20261025.md)
completes 112/136 targets with 815 passed and seven failed cases, then reaches the
original timeout with 171/321 cases observed in the incomplete target. There is
no package pass, suffix retry, new product result or server admission. The seven
test-source compatibility failures and incomplete suffix require separate repair;
effect clearance does not waive them.

The [subsequent focused repair](../../docs/testing/performance/0.74/package-compatibility-isolation-result-20261025.md)
restores four frozen sources and isolates sixteen diagnostic cases. All seven
compatibility failures pass on both compilers, without a new package replay.
V2's production source pins remain unchanged and reject this repaired graph.
Its positive unit fixture materializes the exact retained v2 data; a substituted
catalog avoids executing archived Python, and current-tree refusal stays a
separate negative control. This fixture is not current-source effect clearance.

## Current security successor scope v7

`package_effects_review_v7.py` checks a separately reviewed 372-source AST,
364 unchanged complete rows, four changed/four added source pins, 51 auxiliary
inputs and 147 default targets. Inherited V6 review is read from its sealed
receipt, never refreshed against current sources. Project Python imports follow
auxiliary checks; exact child argv keeps Git CPU diagnostic entries dormant.
The source fixture and parser receipts are retained as data in the
[V7 review packet](../../docs/testing/performance/0.74/local-runs/package-review-v7-20261026/index.json).

`package_replay_v7.py NEW_LABEL` is preflight-only. Only `--execute` dispatches
the unchanged 780-second/four-thread/default entire package command. It checks
review bytes before import, original ignored wrappers, all inherited environment
guards and a clean expanded source scope. It accepts no timeout, filter or retry
option. Its coordinator deliberately never infers a package pass from process
exit; complete target/case output must be assessed separately. V6 is immutable
and is not retried. See the [review report](../../docs/testing/performance/0.74/package-review-v7-result-20261026.md).

The first actual V7 command stops at the unchanged disk floor during compilation;
no target starts. The outer capture terminates the coordinator before the inner
result file is written. Both cleanup logs and that absence remain evidence;
the old transcript parser is not relaxed. The data-only execution-packet
verifier distinguishes this resource refusal from assessed test failures or a
package pass. Capacity is repaired afterward by scoped obsolete-symbol cleanup,
without an automatic replay or changed source/limit.

## Exact approved roots scope v8

`package_effects_review_v8.py` preserves V7 and requires a separately reviewed
374-source/26,044-site AST. Exactly two existing rows change and two are added;
370 complete rows remain exact. All 54 auxiliary inputs are pinned before project
imports. The one ordinary target adds only read-only fixed Git/source and bounded
memory-mutation controls. The approved public-root graph is not projected into
the product runtime; only preservation-test text sees an exact predecessor.

`package_replay_v8.py NEW_LABEL` remains preflight-only unless `--execute` is
explicit. Review source is checked before import, source must be clean and the
unchanged original wrappers/command enforce 780 seconds including compilation,
four case threads and the original disk floor. No retry/filter/limit option is
available. Complete output needs independent assessment; the dispatcher never
infers a package pass. The [report](../../docs/testing/performance/0.74/roots-lock-view-package-v8-result-20261027.md)
separates the immutable first defects and earlier package refusals from current
source/effect clearance, focused passes and the eventual entire-package result.

## Repaired scope v3 without a dispatcher

`package_effects_review_v3.py SYNTAX_JSON` requires the separately reviewed
343-source AST, all 334 original rows unchanged, nine added-source pins, 29
auxiliary pins and 137 targets. It reuses the byte-pinned v2 environment guards
but never dispatches a package. V1/v2 pins remain immutable. The positive unit
fixture contains sealed source data, with an explicitly substituted catalog;
the actual committed-tree preflight is retained separately.

The [review and full target result](../../docs/testing/performance/0.74/repaired-scope-contract-frontier-result-20261025.md)
includes one unfiltered four-thread 321-case contract target inside the unchanged
780-second limit. Its complete names match the independent offline catalog.
No full package is repeated or declared green, no measured candidate is accepted,
and no Linux/live/qualification admission follows from v3.
