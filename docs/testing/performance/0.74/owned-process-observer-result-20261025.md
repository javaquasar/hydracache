# Owned Windows process observer controls

The external observer foundation now reads one owned synthetic child's lifetime
CPU and I/O through the same retained handle before and after exit. It does not
dispatch Cargo or the 321-case target. The earlier
[321-case semantic pass](repaired-scope-contract-frontier-result-20261025.md)
stands separately; full-package attribution and release admission remain open.

## Scope and fixed control

`owned_process_observer.py` opens only the still-live `Popen` child, checking it
before and after attachment. The queried PID and exact executable path must
match. One handle stays held until the terminal sample and is then closed;
neither sampling nor terminal observation reopens a PID. Requested rights are
limited query and synchronization, with inheritance disabled. The collector
has no termination, process-tree enumeration, Git or Cargo API.

The only CLI is `synthetic-control NEW_OUTPUT_DIR`. It creates an exclusive
directory and intent before platform/API preflight, captures both streams, and
runs a no-descendant Python fixture. The fixture writes and reads 262144 bytes,
performs bounded arithmetic, and exits. A ten-second observation deadline,
at most 400 samples and waits of at most 50 ms bound this diagnostic. These
limits do not replace or reset the original 780-second package limit or the
300/60/10 live windows. The fixture alone may kill/reap its own direct child
on failure; this is not general Cargo-tree or all-writer retirement.

Samples preserve creation identity, kernel/user CPU in 100 ns units, six
operation/transfer counters, working set and private commit. CPU and I/O must
not regress. Sampling after a terminal record refuses. A first read/budget
failure latches; later reads cannot silently recover it. Cleanup errors are
recorded separately from the first refusal. Existing output directories refuse
without overwriting their results.

The Win32 calls are sequential observations, not an atomic snapshot. CPU is
whole-process lifetime across threads, not the elapsed wait bracket; descendants
are excluded. I/O transfer bytes are not physical disk traffic or copied bytes.
Working set/private commit do not measure allocator retained heap. No
`wall - CPU` calculation is used to invent waiting time.

The API contract leaves live exit time undefined, so it is masked until the
handle is signaled. A terminal memory-query failure is an explicit unavailable
field, never a fabricated zero. Live memory-query failure refuses the sample.
These choices follow the official
[process times](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes),
[process handles](https://learn.microsoft.com/en-us/windows/win32/procthread/process-handles-and-identifiers),
[I/O counters](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getprocessiocounters)
and [memory counters](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex)
contracts.

## First results

The first twelve model controls pass, followed by the first native synthetic
control. Cleanup/error hardening adds nine controls. The final complete Python
attribution suite passes **83 tests**, including **21 new observer controls**;
the final-code native control also passes. Both native outcomes are retained,
not selected from failures. There is no target or performance cohort retry.

The final native control has nine records, one terminal. Creation identity stays
fixed, terminal CPU/I/O are readable after exit, all counters are monotonic,
the expected payload/streams match and exit status is zero. Both controls
exercise actual Win32 structure layouts and handle calls on this Windows host.
The terminal working set is much smaller than the earlier live samples; it
cannot be promoted into a low-retention claim. No observer-overhead acceptance,
product improvement or causal test attribution is established by these controls.

The complete raw wrapper receipts, fixture records and sources are retained in
[the evidence index](local-runs/owned-process-observer-20261025/index.json).
Each wrapper reports unchanged source and HEAD during its own command. The
first native code predates cleanup hardening; its exact source is retained
separately rather than silently assigned the final code's identity.

Code commit: `25a15422c607563b7f1b61b5cdbc6c67c9063d46`.
The primary index SHA256 is
`9812d360f72d9a1ddfd000dbed77353eeffa7520525c8e106f38af3c0f3a03ac`
(38 members, 25 distinct objects). The
[supplemental gate packet](local-runs/owned-process-observer-gates-20261025/index.json)
has SHA256
`d1e3d391c8a657c4dea0db8bc7a2c5cd42f968b0f56f423b960b09b2a67e3c24`
(31 members, 21 objects). Its staged data-only verifier independently checks
the object hashes, original/final source joins, terminal records and monotonic
counters without executing either control.

The initial verifier used `code/first-owned_process_observer.py` instead of the
actual `code/first-native-owned_process_observer.py`. Direct verification and
the first supplemental retention therefore refused on the same missing member.
Both pre-fix reader sources and a labeled observation of those errors are
retained; raw stderr bytes were not saved and are not reconstructed as a raw
receipt. Correcting the member name reuses the original staged packet, not a new
native result. The independent verifier then passes.

The 91 historical-tool Python cases also pass, for **174 Python cases** across
the two suites. Compileall, tabnanny, doc-check, the local non-promotable
performance-contract check and the unchanged v3 source/environment preflight
pass. No Rust source is changed, so previous Rust compilation/lint results are
not presented as fresh runs in this Python-only step.

## Remaining Cargo integration boundary

The collector is not yet a full target runner. Before any new attribution
dispatch, a separate adapter must prove the chain from an owned unchanged
780-second wrapper through the Cargo shim/toolchain to the exact built test
image, retaining original handles and rejecting ambiguous or reused identities.
It must preserve Cargo's original CWD, environment, streams and command rather
than directly launching the test executable as an assumed equivalent.

Observer failure must retain its first record without resetting the original
deadline, shortening the target or retrying it. Short-lived Git descendants
cannot be accounted by occasional PID snapshots. Parent-only counters will
therefore remain explicitly parent-only unless another complete method is
reviewed. Per-case time, all-descendant CPU, retained heap and all-writer
cessation remain outside this foundation.

No Rust source, dependency, qualification manifest, live predicate or schedule
changes. No server, service, clock, product load, installation or qualification
operation occurs.
