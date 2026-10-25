# Direct pipe process attribution for Release 0.74

One local Windows debug cohort separates spawn from the remaining pipe-based
process/output interval without changing the installed preservation helper.
Spawn is measurable, but the larger wait/collection interval still includes
child execution. The screen admits no transport optimization or product claim.

## Fixed comparison and controls

Code commit `d8d0a0b3991223839bd7f8a0e8f4d59221dd49e5` adds only an isolated
test target and diagnostic tooling. The
[protocol](../../../../tools/xtask-package-attribution-074/pipe-execution-protocol.toml)
precedes the sole cohort: four workers, six alternating ABBA/BAAB blocks, four
slots per block, ninety-six calls and no retry. A barrier precedes each slot.
Every call uses ordinary `git show` with the same full revision/path, inherited
environment and working directory. All raw outputs must equal an independently
archived 23589-byte body, SHA256
`5819f89ec58a899e03d7c0f14c6af91f4be6248a26f8578186cfc47b5900a805`.
No response supplies or repairs its own expected pin.
Exact output equality does not attest every Git configuration or replacement-ref
observation during the completed cohort; it is insufficient authority for a cache.

This is a selected historical test-source payload, not the original twenty-case
target workload. Four workers match its concurrency count, not its complete call
graph, conditional reads, payload distribution or assertion work. The earlier
3157 helper calls are not multiplied by these timings to predict a package win.

The control calls `Command::output()` unchanged. The diagnostic uses fresh
commands with explicit null stdin, piped stdout/stderr, spawn and
`wait_with_output`. It is not the earlier tempfile/polling transport. Controls
compare empty output, EOF stdin, binary stdout/stderr, nonzero exit, one MiB per
pipe and spawn failure. Their scope excludes preconfigured stdio and arbitrary
process trees; no general replacement API is installed.

A duplicate of the original child process handle survives consumption of Child
by `wait_with_output`. Start/terminal snapshots join PID and creation time and
require a terminal exit time and nondecreasing CPU counters. No process is
reopened by PID. These are times for the directly created process, excluding
descendants; they are not an attribution of all Git implementation work.

## First finite cohort

All ninety-six calls pass exact bytes, empty stderr and exit zero. Forty-eight
profile children have complete retained-handle observations. Cohort wall time is
2.714 s. The parent accumulates 0.906 s CPU across the whole concurrent cohort,
including worker/control/diagnostic setup. It is not per-call parent CPU or an
overhead delta between variants.

| Observation | Median per call |
| --- | ---: |
| original output control wall | 71.189900 ms |
| diagnostic profile wall | 71.190250 ms |
| profile spawn wall | 14.940000 ms |
| profile wait and collection wall | 53.774150 ms |
| profile retained-handle observation wall | 0.008400 ms |
| profile terminal observation wall | 0.001200 ms |
| directly created child whole-lifetime CPU | 15.625000 ms |

Each phase median comes from its own distribution; adding these medians does
not reconstruct the median full call. Child CPU overlaps wall brackets rather
than adding another phase. Observed child CPU values include zero and are
quantized in 15.625 ms steps in this cohort; zero does not establish zero work.
The protocol does not isolate pure pipe-copy cost, scheduler delay or child
descendant CPU. In particular, the 53.774 ms wait/collection median is not an
amount of removable waiting.

Control and profile cumulative wall are 5.663 and 4.160 s, respectively, but
the first four control calls take about 280 ms each. All original slots and
tails remain in the result. Similar medians or different cumulative sums in one
fixed finite cohort do not establish an acceleration or overhead acceptance.
Concurrent sums are not partitions of the 2.714-second cohort. Output lengths
are logical bytes, not copied bytes, allocations or retained memory.

The next useful candidate must address a proven process-path cost while retaining
fresh refs/configuration, independent pins, ABA/fallback refusals and current
observations. This screen does not authorize a cache or persistent Git child.
No wait/polling change follows from a wall interval containing child execution.

## Checks and retained failures

The first compile fails before any diagnostic execution because this dependency's
SHA256 result has no LowerHex implementation. Its source and compiler log remain;
the fix formats each byte. The first strict Clippy run then rejects manual
divisibility syntax. Its source and log also remain; the fix tests parity without
changing the ABBA/BAAB schedule. Neither is an original cohort failure retried to
pass. The first actual measurement cohort is successful and is not repeated.

An initial offline sealing request also refuses because the existing final
doc-check wrapper is still running and has not written its result. After that
same check completes, sealing uses the original artifacts without rerunning any
child. The packet labels its coordination note as a transcription of the tool
result, not a captured process log.

The final five new target cases and eighteen existing authority/helper cases pass
on both Rust 1.94 and MSRV 1.88. Forty-three Python controls pass, including missing
or duplicate slots, changed order/schema/pins/status, impossible phase containment
and broken terminal process identity. All-target check, strict all-target/all-feature
Clippy, focused rustfmt, Python syntax/indentation, doc-check and the local
non-promotable performance contract pass.

The [evidence packet](local-runs/direct-pipe-attribution-20261025/index.json)
retains first logs, raw source/build/executable receipts, preregistered protocol,
first compile/lint failures, derived arithmetic and independent staged verification.
Its index SHA256 is
`2ac207a78f7b6b64d876a4136f9a9197ef35e2cb9c9289573eea30ebdf41e43a`.
The locally built executable is digest-addressed in the receipt, not distributed
as a signed production artifact. The installed artifact helper, census caller,
source gate, dependencies, qualification manifest and 300/60/10 live schedule
remain unchanged. The original incomplete package and fifteen audit-only
dependency advisories remain open. No server, service, clock or product is used.

The [subsequent offline reuse audit](preservation-reuse-audit-result-20261025.md)
finds only seventeen repeated literal-query occurrences in the complete saved
helper profile and preserves all freshness/admission boundaries. No further
measurement is used to select an exact-key cache for this caller.
