# Original kernel context call tree attribution

Repeated original origin checks account for 82.2–82.5 percent of the measured
kernel-context span. Named namespace observations and boot-path work are
substantial; the inclusive clock-sampling timer mostly contains its surrounding
origin validations. This packet selects the next hypothesis without changing
any runtime guard or accepting a speedup.

Baseline is `be06c237c3c8c3b6cf4e5eb608c90d0f9feaa47c`.
The [protocol](../../original-kernel-walk-attribution-plan-20261018.md) was
preregistered in `ca7e51b8b52947441ca02fdeae436fc76b0fa080`.
Selected test-only source is `da6637907b377597ae37675f7f6e5e125b553ac0`.
The approved runtime preparation remains unchanged from `2ed04dc8`.
No release installation, bus delivery or execution authority is added.

## Measurements and direct edges

Three fresh fixtures per compiler each perform 32 consecutive retained worker
validations before reservation and challenge issuance. Both exact host test
invocations pass on first execution, with no retry: six fixtures and 192 worker
validations. These are six independent fixtures, not 192 independent samples.
Full original OIDC validation succeeds before and after, original request and
worker/OIDC bytes are unchanged, refusal stays false and IPC authentication
and signature counters remain zero.

All preregistered counters pass: per worker, two packet contexts contain four
sender validations, eight liveness checks, eight projection parses, sixteen
origin validations, thirty-two boot reads, sixty-four named namespace
observations, two time samples, two worker-origin joins and forty-eight bounded
proc reads. The previous six worker counters are unchanged. All 46 output
effect/checkpoint brackets and the original 300/60/10-second admission origins
and deadlines remain. No Reserved, Starting, output, manager send or worker
execution is created.

Ten test-only entry spans record nodes and thirteen direct parent/child edges.
The source guard restores all 351 old Rust files after removing exactly ten
cfg(test) spans and two test registrations. Normal builds contain none of the
instrumentation. [observations.json](observations.json) retains 60 node,
78 edge and 36 enclosing-worker rows with exact integer nanoseconds.

The table pools only the range of the six batch means, expressed in
milliseconds **per whole worker validation**. A row with 32 calls already
includes all 32 calls. Inclusive rows are nested and must not be added together.
Outside-children time is a partition of instrumented wall scopes, not exclusive
CPU, filesystem or allocator time.

| Node index and function | Calls per worker | Inclusive ms | Outside direct children ms |
| --- | --- | --- | --- |
| 0 packet context | 2 | 1.010–1.017 | 0.000420–0.000455 |
| 1 sender validation | 4 | 0.587–0.591 | 0.0357–0.0366 |
| 2 sender liveness | 8 | 0.0511–0.0524 | 0.0378–0.0388 |
| 3 projection parsing | 8 | 0.0310–0.0325 | 0.0310–0.0325 |
| 4 origin validation | 16 | 0.833–0.836 | 0.1945–0.1973 |
| 5 boot document | 32 | 0.3825–0.3840 | 0.2922–0.2936 |
| 6 named namespace | 64 | 0.2543–0.2558 | 0.2543–0.2558 |
| 7 time sampling | 2 | 0.2079–0.2094 | 0.000351–0.000420 |
| 8 worker-origin join | 2 | 0.2144–0.2155 | 0.00647–0.00663 |
| 9 bounded proc document | 48 | 0.1550–0.1568 | 0.1550–0.1568 |

The offline verifier checks exact edge membership/counts, incoming node counts
and times, roots and each parent's inclusive-minus-outside equality to its
direct child times. The sum of all ten outside-children spans equals packet
context's inclusive span exactly. The latter fits inside the original enclosing
worker kernel-context timer. This is a checked partition, not subtraction of
independent controls.

Origin's inclusive share is 82.2–82.5 percent of packet context. In the disjoint
partition, boot outside its bounded-reader child contributes 28.9–29.0 percent,
named namespace 25.1–25.2 percent, origin outside its children 19.2–19.4 percent
and all bounded proc reads 15.3–15.5 percent. These shares do not identify
individual syscalls or exclusive CPU residency.

Time sampling includes two full origin traversals per sample. Its
0.208–0.209 ms inclusive contribution cannot be called clock syscall cost;
only 0.000351–0.000420 ms remains outside those children, including clock calls,
string creation and tracer overhead. Retaining original clocks is not the
measured dominant cost.

The bounded-reader span includes buffer creation, one positional read and
truncation. It excludes destruction of its returned Vec in the caller.
Likewise, returned namespace files and boot strings can be destroyed in their
parent scope. Parent outside-children time must not be renamed pure metadata
or syscall time. The unchanged helper requests a 65537-byte zeroed Vec for each
read; the counts and source do not measure allocator calls, physical zeroing,
copied bytes, RSS or retained memory.

These timings have no uninstrumented A/B comparison, confidence interval,
product-performance claim or decomposition of the full earlier 7.2-second
preparation bucket. The earlier approximately 24 ms observed challenge margin
remains unresolved. No timer renews that challenge.

## Next isolated hypothesis

The fixed 37-byte canonical boot document currently uses the same large
bounded reader as status/fdinfo. A small, dedicated boot-read buffer is a
candidate for an isolated allocation experiment, not an accepted improvement.
First prove identical accepted documents and fail-closed short/EINTR/oversized
behavior with differential/property tests; retain single-read semantics,
all canonical boot predicates and the same live observation count. Measure
allocation and wall effects separately, including returned-buffer destruction.
Use matched plain current/MSRV controls and then the unchanged full preparation
gate before accepting anything. Do not cache mutable boot, namespace, sender,
time or authority observations to remove the sixteen origin walks.

## Inputs and server scope

Release library test images are bound to Cargo artifact JSON and host hashes:

- Rust 1.94: `6e9ed13742a64cb49d64d837b6e855bd05fa9ad129bc65c89444407629707017`.
- Rust 1.88: `2cf843a989fe5d73716411308a8ac6547f101242d58bba288ec5751b50324298`.

Nine changed source/registry/guard files are retained as raw capsules and bound
to the selected commit. The conservative repository input census contains 353
Rust files, three manifests/lockfiles and eight additional literal inputs,
364 distinct paths. Its 27 include_str/include_bytes calls reference twelve
unique files, whose raw bytes are also retained. Five binary sources are
included in the census but not linked into the selected library test image.
This binds repository source/literal inputs, not compiler or registry-crate
contents.

The runner is `/tmp/hc074-original-kernel-walk-20261018.ORIvsN`,
UID/GID 1000, mode 0700, umask 077. Existing helpers also allocate separate owned
private TempDir roots, including the strict direct-/tmp authority leaf. The
working directory is not a sandbox; no single-parent containment or per-path
fixture-write census is claimed. TMPDIR and authority-path policy are unchanged.

Preflight, between-compiler guards and independent postflight preserve boot
`e81671d6-bf23-4ebf-b8fe-5dad1f45866f`, active/running supervisor PID 8839,
NRestarts=0 and exact installed binary/receipt hashes. No registered campaign,
lifecycle or authority marker exists. The executable-object postflight finds
no remaining fixture from the registered runner families. Read-only sudo
tests/hashes do not install or modify anything. No service, clock, real trust
configuration, product workload or qualification is changed.

## Verification

Both GNU compilers pass six new tracer units, four previous worker-trace units,
six IPC trace units, fifteen manager-channel tests, nineteen exchange tests,
five pipe tests, ten worker-context tests and fourteen ordinary kernel-admission
tests. The context filter overlaps one kernel-admission test; do not sum these
groups as distinct cases. Three manual manager tests and one inherited-sender
manual test remain ignored. Current/MSRV affected all-target check and strict
all-feature Clippy pass. Windows passes 44 portable units and supervisor/xtask
all-target check/strict Clippy. All 247 source contracts, registration of 184
discovered gates, formatting, doc-check and local non-promotable performance
contracts pass. Full milestone cargo xtask verify was not run. No focused gate,
compiler build or host cohort failed in this step.

Run `verify.ps1 -SelfTest` read-only to verify the closed file inventory,
all source/input bindings, image and executed-script hashes, complete raw rows,
call-tree arithmetic, original qualification digest and nine negative parser
controls. Keep verification logs outside the closed packet; copy a finished
receipt only before recapturing the manifest.

Qualification remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
Positive authenticated original-start, active all-writer cessation, durable
completion/later cells and their SIGKILL/lost-ack matrix, same-attempt reconnect,
real trust/enrollment and approved installation remain separate open gates.
