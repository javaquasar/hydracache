# Isolated historical blob batch prototype

The Python prototype tests process amortization for a finite, preplanned set
of immutable historical source reads. No existing runtime, source guard or
current-file reader imports it. The isolated native test target below executes
an oracle that uses the Python prototype as one control. No existing validation
graph or live observation changes; this is not release or performance admission.

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
