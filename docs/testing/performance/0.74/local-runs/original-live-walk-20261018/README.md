# Original live worker walk attribution

The original retained worker validator spends about 69 percent of its observed
wall time in two kernel-context checks. Authority inspection contributes about
12 percent. These before-reservation controls narrow the next profiling target
without removing any validation boundary or accepting an optimization.

The [protocol](../../original-live-walk-attribution-plan-20261018.md) was
preregistered in `d5cb0b0e`, against baseline
`b2adb5b73a2f66fc690637260c100b5c8489aa35`. Selected test-only source is
`bd65514b144c6b3575f0255a2dc3f10fb23cdbc3`. The approved runtime preparation
remains unchanged from `2ed04dc8`; this packet adds neither manager delivery nor
execution authority.

## Actual nested measurements

Six entry spans wrap the existing functions. They do not copy a validator,
replace live observations, cache mutable context or change call order. The
source guard restores all 349 pre-existing Rust files exactly after removing
the six cfg(test) spans and two test registrations. All 46 effect/checkpoint
brackets in joined output preparation and the original 300/60/10-second origins
and deadlines remain unchanged. The instrumentation is absent from normal
builds.

Each compiler runs one exact ignored test invocation, with three fresh original
fixtures and 32 consecutive retained worker validations per fixture. The host
cohort passes on its first execution: 192 validations in six fixtures, producing
36 numeric span rows. The 32 iterations in a fixture are repeated observations,
not 32 independent fixtures. Original full OIDC validation succeeds before and
after each batch; the request and worker/OIDC bytes remain unchanged, refusal
stays false, and IPC authentication/signature counters stay zero. No reservation,
Starting, output, manager send or worker execution is created.

The table reports milliseconds contributed **per whole worker validation**,
dividing each 32-call batch by 32. A row with four calls already includes all
four calls. Rows are nested: do not add worker-policy to its worker-source and
active-authority children.

| Instrumented function | Calls per worker | Rust 1.94 | Rust 1.88 |
| --- | --- | --- | --- |
| kernel-worker | 1 | 1.495–1.504 | 1.490–1.502 |
| worker-policy | 2 | 0.255–0.256 | 0.255–0.256 |
| worker-source | 4 | 0.076–0.076 | 0.076–0.076 |
| active-authority | 4 | 0.174–0.174 | 0.174–0.175 |
| kernel-context | 2 | 1.027–1.033 | 1.023–1.030 |
| local-context | 1 | 0.212–0.215 | 0.211–0.215 |

[observations.json](observations.json) retains every exact numeric batch, with
compiler, sample, kind, count, inclusive nanoseconds and time outside instrumented
direct children. The verifier checks the parent/child arithmetic exactly:
kernel-worker minus its outside-children span equals worker-policy plus
kernel-context plus local-context; worker-policy minus its outside-children span
equals worker-source plus active-authority. The four leaf kinds have no
instrumented children.

The spans measure instrumented wall time, not exclusive CPU/IO time, syscall
ownership, allocations or lock contention. There is no uninstrumented A/B
estimate or confidence interval. These controls precede reservation and the
progress challenge; they do not decompose the earlier 7.2-second joined-output
bucket or establish a critical-path speedup. Full IPC authentication timers
also exclude other cryptographic work. Keep the earlier approximately 24 ms
observed final challenge margin as an unresolved risk.

The next attribution should split kernel context into sender liveness/status,
origin/namespace/boot observations and clock sampling on the same retained
objects, before the challenge. Preserve every live predicate and original
observation count. Select a concrete optimization only after that attribution;
do not infer permission to reuse a successful observation across an effect.

## Exact inputs and host scope

The two GNU release images are bound to Cargo artifact JSON and host hashes:

- Rust 1.94: `1eb38b1ce6960d15d7ce4502d59f9177dc033fe4b4f748d40a3a070527c0c386`.
- Rust 1.88: `ed7947af0e56a26178e5eebb3c71500492333af31e187ae15006b8c2a6720bca`.

Fourteen changed source/registry/guard files are retained as raw capsules and
bound to the selected commit. The input inventory contains 351 Rust files,
three manifests/lockfiles and eight additional literal embedded inputs: 362
distinct repository paths in total. A census of all 27 literal
include_str/include_bytes calls identifies twelve unique files, four overlapping
the Rust/manifest inventory. Their actual raw bytes are retained in
embedded-capsule as well as LF-normalized Git bindings. This is a repository
source/literal-input census, not an attestation of compiler or registry-crate
contents. The conservative Rust inventory includes five binary sources not
linked into the selected library test image.

The runner directory is `/tmp/hc074-original-live-walk-20261018.E2UxSa`,
owned by UID/GID 1000 with mode 0700 and umask 077. Existing helpers create
separate owned private TempDir roots, including the strict direct-/tmp authority
leaf. The working directory is not a sandbox; no single-parent containment or
per-path fixture-write census is claimed. TMPDIR and authority-path policy were
not changed.

Preflight, between-compiler guards and independent postflight preserve boot
`e81671d6-bf23-4ebf-b8fe-5dad1f45866f`, active/running supervisor PID 8839,
NRestarts=0 and the exact installed binary/receipt hashes. No campaign,
lifecycle or authority marker is present, and the executable-object scan finds
no remaining fixture from the registered runner families. Read-only sudo
tests/hashes do not install or modify anything. No service, clock, trust
configuration, product workload or qualification run is changed.

## Checks and retained failures

Both GNU compilers pass four tracer units, six existing trace units, fifteen
manager-channel tests (three manual tests remain ignored), nineteen exchange
tests, five original pipe tests and ten worker/kernel-context tests. They also
pass affected all-target check and strict all-feature Clippy. Windows passes
44 portable units and supervisor/xtask all-target check and strict Clippy.
All 245 source contracts pass, all 184 discovered gates are registered,
formatting and doc-check pass, and the local performance contract remains
non-promotable. Full milestone cargo xtask verify was not run.

The initial exact-source guard caught an extra terminal blank line in the new
registry block. The first full static suite also caught three guard-integration
problems: an old context reader split at the newly inserted cfg(test), and two
direct reference readers had not removed the exact test-only spans. These were
fixed without changing a runtime statement or weakening a baseline comparison.
The reproduced first failure and first full-suite failure remain in distinct
logs. PowerShell parsing caught a foreach/pipeline construction before those
evidence scripts executed; it was corrected before capture. None of these was
a failed/retried host cohort.

Run `verify.ps1 -SelfTest` for a read-only check of the closed file inventory,
committed source diff, all 362 inputs, twelve raw embedded capsules, source and
image identities, all rows/nesting, host completion and six negative parser
controls. It neither executes tests nor grants admission. Keep verification
output outside this closed packet, then copy only a completed receipt before
recapturing the manifest.

The qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
Authenticated production original-start, active cessation of every writer,
completion/later cells with full SIGKILL/lost-ack coverage, same-attempt reconnect
and real trust/enrollment remain separate open gates.
