# Private GET owner phase B0: separate timing and native controls

This finite local screen follows the passed D3a allocation/requested-live screen.
It implements instrumentation and preregistration, not a new product change.
The immutable D3a policy, measurement tool/source and all archived bytes remain
untouched. See `get-response-owner-phase-b0-contract.toml` for the exact matrix,
noise/invalidation rules and unchanged regression floors. No numerical phase B0
attempt has run at preregistration.

## Why the existing harness cannot simply certify timing

Audit of `native-api-profile-074`, `resp-pipeline-profile-074` and
`performance_local_pairing_074.py` found useful semantics and placement helpers,
but both Rust profilers depend on loadgen's process-wide counting allocator.
Turning off product instrumentation does not compile that allocator out. The
RESP TCP profiler also wraps writes with atomic socket counters. Existing timing
is therefore attribution/diagnostics, not the required unprofiled confirmation.
The integrated 0.73 host harness is a separate Linux/daemon/security workload;
it is not modified or reused as a pretend four-surface Windows control.

## Isolated implementation

`tools/get-owner-controls-074` is a separate workspace and lock. Its default
timing builds have neither a counting allocator nor a loadgen dependency or
socket counter wrapper. The separate `allocation-profile` builds reuse only the
historical tool-only System allocator by source reference. Product instrumentation
is disabled in all four binaries. Allocation timing is diagnostic only; timing
gross bytes are null rather than an apparent zero-allocation result.

Only `get-owner` forwards the existing private feature. Feature-off/on binaries
use the same clean instrumentation SHA and dependency lock; this is an isolated
canonical-off comparison, not a new B73 identity or accepted integrated C74.
The new lock was resolved separately; numerical comparisons must never mix it
with old-tool binary measurements. Root/product/native/store/protocol and old
tool locks are not changed.

Ten cells cover embedded encoded GET, direct ClientSurfaceState GET/SET and real
unwrapped RESP2 plaintext loopback TCP GET. Logical concurrency is 1/8; fixed
Tokio workers are two. ClientSurfaceState uses the RESP binary-key shape so both
native and RESP preload/oracle semantics interoperate, not an authorization
bypass. The native control does not claim all possible native key distributions.
An explicit yield every 64 native operations bounds synchronous task monopolizing.
The same yields/work counts/key space/payload bytes occur in both variants.

Corpus and preload precede a READY/GO gate. The runner applies CPU affinity 0/1
and normal priority before opening GO, then warmup precedes the counted/timed
window. Every fresh process checks its compiled SHA against clean current Git.
Measured whole-process CPU/goodput includes task and socket setup, every exact
response validation and workload completion; it excludes preload, warmup, final
verification and receipt serialization. No server-service mutation or external
host is involved. No TTL, security, persistence or multikey behavior is changed.

## Estimators and limits

Two lanes run five A/A and five counterbalanced A/B pairs per cell: 400 fresh
processes total. Policy, source, compiler, tool lock, all four binaries and full
schedule are sealed before the first sample. Attempt directories are create-new;
commands, placement, output, failure and raw hash survive. Source/binary/contract/
lock identity is checked before and after each attempt. Any invalid window,
background CPU or A/A noise stops without retry/tuning. A valid red completes the
finite matrix. Offline replay requires every raw/attempt/placement marker file,
exact order, complete five-pair log t(4) intervals and the independently rebuilt
summary. No selection of fast pairs or successful-only retries is allowed.

The unchanged native floors are goodput >=0.98, CPU and p99 <=1.03, gross bytes
<=1.05; the same conservative local floors are applied to RESP. Minimum one
second measured CPU and wall time protects against accepting a tiny CPU window.
Microsecond histogram rounding and Windows CPU accounting still limit precision;
an insufficient or noisy sample is invalid, never permission to change thresholds.

Latency at p1 is a closed-loop operation exchange. Deep-pipeline latency is the
whole batch and is never divided by pipeline or called per-operation/scheduled
p99. The preliminary batch guard supplements, not replaces, exact scheduled
per-operation latency and coordinated-omission analysis. Counting requested
layouts cannot establish allocator active/resident/retained or OS RSS release.
The profiling lane's requested-live counts are diagnostic; full concurrency
peak/idle/refill admission is not inferred from them.

## Required next cohorts

HC1 and HC2 independent native controls, matched mTLS/RESP3, concurrency 32/128,
scheduled per-operation latency, misses/errors/slow readers/size transitions,
allocator active/resident/retained and timed idle/refill remain open. Ordinary
feature-on hosted CI still needs a current-source execution receipt. None is
waived by this contract. Full D3, default enablement, integrated C74, expensive
infrastructure and qualification are not authorized by a local B0 pass.

## Development diagnostics and focused verification

Before any numerical run, the initial compile caught an attempted import of the
private RESP key-mapping helper. The tool now mirrors the documented byte mapping
locally, with real TCP oracle interoperability tests; the product API was not
opened. Strict lint also caught a manual clamp, corrected in tooling only. These
are premeasurement diagnostics, not hidden performance attempts or policy tuning.

Run the four feature combinations' serialized Rust tests, all-target/all-feature
check and strict clippy. Rust fixtures cover bounds, exact native/TCP GET/SET,
pipeline count, disabled product metrics, trace binding and p1 small/large replies.
Python tests cover full schedule, sealed floors, partial/reordered/failed/unplaced
attempts, counting/timing separation, latency/CPU denominators, precision/noise,
native/batch regression, trace identity and all-file offline hash replay.
`get_owner_phase_b0_separates_unprofiled_timing_without_waiving_full_d3` enrolls
the finite contract and closed admission boundaries in xtask.
