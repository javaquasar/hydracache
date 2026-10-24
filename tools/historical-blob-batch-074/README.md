# Isolated historical blob batch prototype

This Python prototype tests process amortization for a finite, preplanned set
of immutable historical source reads. No runtime, source guard, Cargo target or
current-file reader imports it. It does not change the validation graph or any
live observation, and it is not a performance result or release admission.

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

`decoder.rs` is a separate protocol component, imported only by the new ordinary
`crates/xtask/tests/historical_blob_batch_074.rs` target. It consumes a bounded
stream against ordered, independently supplied SHA256 byte pins. It checks exact
object ID/type/length headers, complete bodies, delimiters, content, missing
identity and complete stream consumption; errors publish no prefix. This is not
a Git transport or a port of Python's SHA1 recomputation. Its synthetic OIDs and
caller-created pins carry no original-source/trust authority. There is no default
pin producer, cache, source reader or integration into an old guard. Existing
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
equate a caller assertion with verified historical identity. A bounded native
transport, independent real-Git/Python differential corpus, reference provenance,
and default-replacement/short-ID semantics remain separate gates. No integrated
speedup is claimed by either the decoder or the offline eligibility audit.
