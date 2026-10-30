# IMap 0.75 scoped coverage

This development gate measures added executable lines since
`98d7321b1cda568c386cbfdef1993c7e4dcebe38`. The policy is recorded in
[coverage-profile.json](coverage-profile.json). Release 0.75 remains planned,
W0 remains open and production IMap remains disabled.

Install `llvm-tools-preview` for the exact active Rust 1.88.0 or 1.94.0
toolchain; use JDK 17 or 21. Linux and Windows x86_64 results are separate.
Run from a clean committed worktree:

```text
cargo xtask imap-coverage-capture --release 0.75 --output target/imap-075/coverage-unique
cargo xtask imap-coverage-check --release 0.75 --receipt target/imap-075/coverage-unique/report.json
```

The controller uses the P1 bounded process executor, an isolated Cargo target,
explicit instrumentation flags, fresh PID-specific LLVM profiles and fresh
JaCoCo execution files with one session per Maven phase. It copies its live
instrumented executable before Cargo can rebuild that target. The instrumented
analysis process exits before the final report is reconstructed. A measurement
with uncovered lines is retained as `analysis.json` with `qualification=false`;
it cannot satisfy the final gate.
The measurement defers only its own still-live PID profile, checked against its
PID/birth marker. The final pass defers nothing and requires the completed analysis
profile. Git diff/list queries apply the declared source scopes before their byte
budget; unrelated predecessor source changes do not consume this scope's budget.
For a synthetic PR merge with one parent descending from the fixed 0.75 baseline,
the changed-file set comes from that parent before patches are requested. Every
final tracked source is still byte-bound. Changes in a shared selected file are
conservatively retained in full; predecessor-only files have no 0.75 changed-line
obligation. `unchanged` in the summary refers to this selected scope.

LLVM profiles are bounded at 128 MiB per file, 2,048 files and 6 GiB in total.
Control/capture v2 also binds the bootstrap Cargo stream. Each physically
completed Cargo phase immediately archives its executables, host build scripts
and proc-macro/dynamic libraries by content hash. Later relinking at the same
target path cannot replace an earlier measured object. Reconstruction requires
an exact stream-to-object correspondence and the complete archive, bounded at
512 unique objects, 512 MiB per object and 8 GiB in total. LLVM still checks every
binary ID; host profiles are never discarded. The export command uses a bounded
UTF-8 response file to avoid Windows command-line length limits.
Build-script filenames are matched against Cargo's actual target name, including
`aws-lc-sys`'s `build-script-main`. Capture rejects invalid or existing outputs
before loading tool distributions. Unit executors exercise publication and CLI
dispatch with unqualified measurements; publication retains the verdict verbatim,
and the real checker refuses those fixtures.
The v2 policy/profile records also bind two code-chosen Cargo overrides: only
`sha2@0.11.0` is optimized at level 3 in this isolated dev/test measurement build.
Scoped implementation crates retain their unoptimized coverage maps. This avoids
spending the analysis budget on debug-mode hashing of multi-gigabyte evidence;
artifact contents, source checks and the zero-uncovered-line gate remain required.
The first actual capture observed 33–50 MiB profiles for this large development
tooling binary. Coverage test cases run one at a time to keep profile write I/O
out of unrelated short deadline canaries; explicit concurrency inside each test
and real multi-client workload remains exercised. These bounds and scheduling
are measurement controls and do not change the ordinary test or production plan.
The later 95fed9bba diagnostic retained 231 closed profile files and
4,076,955,432 raw bytes before the final analysis. The aggregate allowance includes
the additional fault cases and closed analyzer; it never permits dropping a
profile, lowering the changed-line gate or admitting a partial report.
Healthy build-controller fixtures allow 10 seconds in the instrumented profile
(three seconds ordinarily) for the final LLVM dump. Their outer test budget is
15 seconds (five ordinarily). Explicit negative deadlines, byte/event limits,
cleanup requirements and the public 60-second proof ceiling are unchanged. The
two-event fixture schedules no periodic heartbeat within its phase; the combined
stream fixture flushes both streams before its potentially slow profile dump.

Intentionally killed proof/soak workers suppress the automatic exit dump. They
serialize bounded counter buffers and atomically replace their PID profile before
publishing observable frames. A kill during serialization/publication leaves the
previous complete checkpoint intact. `profile-staging/` holds only incomplete
temporary writes and is never a counter input. Every admitted `.profraw` is still
required to merge successfully; corrupt profiles are not discarded or skipped.
Only those two worker binaries use the documented
[LLVM manual-buffer runtime mode](https://github.com/llvm/llvm-project/blob/main/clang/docs/SourceBasedCodeCoverage.md#using-the-profiling-runtime-without-a-filesystem).
Ordinary tooling/test controllers retain LLVM's exit dump. Counters are never reset.

After the real preparation, the capture also invokes the public native probe and
soak commands. All-cohort smoke and reference-only checkpoint/report resumes use
separate segments and fresh output directories. The actual compiler-graph replay
and provenance mutation tests run against this preparation's inputs; mutations
are isolated test fixtures and cannot replace the producer's archive. A fixed
SBOM check command separately replays the original producer-bound provenance.
The measurement phase reconstructs its own unqualified result a second time;
only the closed-process final phase may enforce and issue qualification.
The all-cohort CLI smoke has a 180-second shared host budget; each active segment
still lasts 200 ms. Producer-bound originals are replayed by their own CLI;
mutation-test binaries bind separate unqualified copies to their own producer.

The external consumer uses a newly created, exclusively owned temporary workspace
at a stable path. Its generated manifest points directly to the tracked absolute
consumer source and the testkit dependency. This avoids copied relative source
paths and compiler-version-dependent LLVM remapping. Existing or stale directories
refuse, the manifest and lock input are freshly written, tests run each time,
and ordinary drop removes the newly created workspace. The common
historical gated-test registry remains byte-identical to its predecessor; the
closed 0.75 discovery extension lives in this directory and its tests execute
through the 0.75 capture rather than generic release-evidence receipts.

The final check reconstructs LLVM LCOV and JaCoCo source counts from the retained
execution profiles and exact binaries/JARs. Source SHA, all scoped source bytes,
tool distributions, compiler cfg/features, Cargo executable sets, Java test
completion and session identities are bound to that capture. Missing sources,
empty reports, substituted bytes, live controllers and foreign paths refuse.
Changed comments and other non-executable lines do not inflate the denominator.
Compiled-out platform files and cfg ranges are disclosed; another platform's
result cannot qualify them. No branch coverage or production correctness claim
follows from a passing line gate.

New Java classes also have mandatory Maven JaCoCo class checks (80% line floor)
in their test phase. The source-bound changed-line gate is stricter and requires
zero uncovered added executable lines. Existing global Rust and Java ratchets
remain unchanged. Test dependencies and instrumentation are development-only.

CI retains the existing three-job topology and exact Rust/JDK matrix. Coverage
runs before final artifact preparation because instrumented external consumer
builds must not invalidate an earlier exact-artifact receipt. CI v3 receipts
include the checked report digest. A local capture is not an S2 cluster result,
release admission or a long-duration soak.
