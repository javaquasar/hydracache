# Static contract Git attribution

One isolated instrumented `performance_contract_074` execution passes all 321
original cases with four case threads. Source, HEAD, binary and the exact case
catalog remain unchanged. The observation confirms 12424 completed Git sessions,
mostly distinct historical blob reads. It does not establish Rust parser cost,
Trace2 overhead, a product throughput gain or a green full package.

## Instrumentation and preserved scope

Code commit `8e95c5c9283b2bf924f1e9688650f81f5b6cedb1` adds five files under
`tools/xtask-git-trace-074`: `capture.py`, `trace_review.py`, `test_capture.py`,
`test_trace_review.py` and `README.md`. No Rust test, assertion, source reader,
current-file observation, dependency or live validation frame is modified.

A Rust 1.94 `--no-run` build receipt reports `fresh=true`. The test executable
SHA256 is `ef89a0d8939121a7e8540f579396bc175c236b293b8293ba22282bc8362b3615`.
Every captured build input must still match, and the executable's `--list`
must equal the 321 names in the previously sealed package evidence. There is
no test filter, ignored-test mode, nocapture mode or executable substitution.

Git 2.51.0.windows.1 supplies its own Trace2 EVENT format 4. A separate read-only
version control first proves a complete trace session. Existing Trace2 config,
capture-affecting environment and replacement refs refuse. Only child environment
variables select the private trace directory; PATH and Git configuration are
unchanged. Installed Git documentation hashes and the actual Git executable
receipt are retained. No kernel tracing or privileged service action is used.

The target retains the 780-second outer bound and 4-GiB disk floor. Parent wall
time uses a monotonic interval; parent CPU uses `GetProcessTimes` on the held
Popen Windows handle. Initial/final creation identity matches and final exit
time is present. The parent time excludes child CPU. Exception cleanup controls
address only this owned test tree and exact process handle, not other images,
services or controllers. No cleanup is needed in this successful attempt.

## First instrumented observation

| Observation | Seconds | Interpretation |
| --- | ---: | --- |
| Parent monotonic wall | 523.944 | Brackets test process creation and completion |
| Libtest target span | 523.92 | Rounded harness result for all 321 cases |
| Parent user CPU | 107.859 | Excludes Git children |
| Parent kernel CPU | 132.797 | Excludes Git children |
| Parent total CPU | 240.656 | Not per-test or parser CPU |
| Sum of Git elapsed spans | 847.300 | Overlapping per-process spans, not CPU |

All 321 cases pass, with no failed, ignored, measured or filtered cases.
The target exits 0; capture refusal is null and the owned target is no longer
alive. Every one of 12424 target Trace2 sessions has one matching version,
start, command name, exit and final atexit lifecycle, consistent elapsed order
and zero exit. There are no nested Git sessions or discard sentinels. The
separate version control is not included in those target counts.

The per-process Git spans can overlap between four test threads and omit
Windows process creation before Trace2 initialization. They cannot be added
to, or subtracted from, target wall/CPU as disjoint phases. Likewise, parent
CPU does not isolate parsing, token comparison, byte/string processing or
parent-side launch work. Post-processing and source verification happen after
the target interval; their elapsed time is not included in the table.

The older 334.14/361.33-second uninstrumented observations have different
observer and execution conditions. This single instrumented result cannot
measure Trace2 overhead or a regression/speedup. No second target attempt,
full-package retry or larger bound follows.

## Git operations and the cache hypothesis

| Command | Completed sessions |
| --- | ---: |
| `show` | 12383 |
| `ls-tree` | 29 |
| `rev-parse` | 6 |
| `merge-base` | 3 |
| `status` | 3 |

An additional offline derivation verifies every archived trace member and groups
the exact captured `show` arguments without running Git or resolving revisions.
Five synthetic shape controls accompany that calculation.

| File argument shape | Requests | Unique raw arguments | Repeated raw arguments |
| --- | ---: | ---: | ---: |
| Full forty-hex revision | 1884 | 1858 | 26 |
| Short hex revision | 10499 | 10270 | 229 |

These are argument-shape counts, not admitted immutable identities. A short
revision must not be silently treated as a full object ID or resolved later
and represented as the original observation. Full/short aliases are not merged,
and no current-file read is counted as an immutable cache request.

The initial idea of a broad memoization cache is not supported as the main
cost reduction: most captured argument strings occur once. This is a negative
screen for that argument-repetition hypothesis, not a benchmark of an installed
cache. It says nothing about an unmeasured cache keyed by proven resolved OIDs
or about bytes shared between different historical revisions.

The next hypothesis is a separately extracted batch reader for distinct
immutable historical blobs, reducing process creation without dropping a
comparison or caching a fresh observation. It needs exact byte/error/ref and
concurrency differential controls before integration. Neither batching nor a
new cache is implemented or admitted by this diagnostic.

## Controls and evidence

Seven Trace2/parser controls and seven capture controls pass, including malformed
JSON/time/lifecycle, duplicates, truncation, case drift, actual current-process
handle observations and mocked exception cleanup. The earlier eleven-control
version and the final fourteen-control version are retained with exact sources;
the cleanup gap found in source review is closed before the sole target run.
Compilation and indentation checks pass. There is no new Python dependency,
external linter claim, Rust check/Clippy repetition or product benchmark.

The [sealed packet](local-runs/static-contract-trace-20261024/index.json) has
53 logical members and 41 objects. Index SHA256 is
`11e89b85754443b1c5ff818ebfa4e5fcd2e896d812a85065cac2ffe70389e085`.
All 12425 raw trace files, including the separate control, are retained in a
deterministic 1646102-byte gzip/tar member with per-file hashes. Archive members
are verified in memory without extracting paths into the filesystem. Source,
build/list receipts, process identity, raw stdout/stderr, result and coordinators
are also retained.

The separate [argv-shape derivation](local-runs/static-contract-trace-shapes-20261024.json)
binds that packet/archive, every raw member, the exact parser source and its own
coordinator. SHA256 is
`2fef3f7a2e2107297b81c3c8a5ae03a495f90e6a1c1ab083e558bcc4f8db1ac8`.
No new workload or revision-resolution command is run by that derivation.

The interrupted full package remains interrupted; this isolated target pass
does not fill its twelve-target suffix. ReservedLive frequency/order, original
300/60/10-second windows, 0.73 artifacts and qualification manifest digest
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc` remain
unchanged. There is no server, service, clock, original-owner, installation,
qualification, product candidate or push action. Production admission remains
closed at the [existing boundaries](release-closure-audit-20261024.md).
