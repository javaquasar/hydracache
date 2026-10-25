# Boot and Namespace operation attribution

A finite eight-process cohort now separates selected operations inside the
actual Boot and Namespace readers without removing an observation or changing
an original deadline. Open is the largest measured selected span. Decode and
namespace-path construction are close to the empty-observer cost and do not
justify a runtime optimization from these observations. No optimization is
implemented, no preparation is retried, and the prior enrollment refusal remains
the full-path result.

## Frozen method and source boundary

The [preceding residual profile](authority-parent-cost-result-20261025.md)
retains 153088 Boot and 306176 Namespace observations in outputs, but its nested
wall views cannot identify individual operation CPU costs. This experiment
addresses those operations directly rather than adding another whole-preparation
timing sample.

Commit `ed0707328a3e77f08ca2f63dfadaa0d1bde119c2` freezes the
[protocol](../../../../tools/kernel-operation-cost-074/protocol.json), test-only
observer/references, finite screen, offline reader and manual non-ship gate before
the first cohort. Runtime statements come from the parent-device candidate
`e5284749e84a1a025ea1ef47f2b20e744a72bed1`, documented at `b0a8030cb`.
The new observer is compiled only under `cfg(test)`; it is not an admission
input or a new production validation policy.

Twelve test-only hook statements surround existing operations in
`diagnostic_kernel_origin.rs`: opens, filesystem validation, namespace identity,
bounded boot-document read, decode, path construction and one empty span per
whole call. Every original syscall, flag, conditional refusal and FD lifetime
stays in its original order. Removing exactly those hooks and the test-module
declarations reconstructs the previous source after line-ending normalization.
Seven separately copied reference functions match the baseline after mechanical
name and whitespace normalization. This proves runtime statement preservation,
not binary identity or unchanged source-line diagnostic locations.

The observer selects one operation kind per sample. It preserves errno around
CPU-clock reads, records clock failure without replacing the original function
result, rejects a nested capture without replacing the outer one and prevents
stale spans from recording into a later scope. Capture/span types are thread-local
through non-Send/non-Sync markers. Unit controls cover errno, independent threads,
clock failure/underflow, stale scope and preservation of the original panic.

## Cohort and calibration

Eight independent Rust 1.94 GNU release processes run as UID/GID 1000, with
each measurement child pinned to CPU 5. No build or other test from this session
overlaps the cohort. Each process runs Boot, user, mount, PID and time Namespace
paths, in that order. For every path it selects open, filesystem, identity,
document, decode, path and empty, also in fixed order.

For each selected kind, sixteen samples of 256 whole calls execute four balanced
arms: frozen reference, identical-reference control, actual runtime with no active
operation capture, and actual runtime with the selected capture. Eight cyclic
forward/reverse orders run twice, rotated by process. Both reference and runtime
receive sixteen warmups per case. All 17920 rows remain retained, with no outlier
or overhead subtraction.

Each Boot call must have four opens/four filesystem validations, one document
read/one decode, no namespace identity/path operation and one empty control.
Each Namespace call must have one open/filesystem/identity/path/empty operation,
with no boot-document or decode operation. Inactive reference/runtime arms
record no selected spans. The reader refuses changed counts, ordering, types,
flags, clocks, incomplete work or a missing exact libtest verdict.

For whole CPU comparisons the estimator takes the median of sixteen paired
log ratios per process and then the median over eight process estimates. The
95% percentile bootstrap uses 20000 process resamples and seed `0x074d1a05`.
Both identical-reference and inactive-runtime/reference intervals must fit
[0.97,1.03] for all 35 path/kind combinations. All pass. Inactive-runtime point
ratios range from approximately 1.0073 to 1.0125: even disabled test hooks have
a measurable cost, which must not be described as zero overhead.

## Selected operation observations

Each value below is a median over eight process medians of the sixteen selected
CPU times per operation. Units are microseconds, and every value includes
observer/clock cost. A dash means that operation is absent from that call, not
that it executes for free. Boot open/filesystem values are per operation, with
four such operations per whole call.

| Path | Open | Filesystem | Identity | Document | Decode | Empty control |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Boot | 1.942 | 0.935 | — | 1.388 | 0.615 | 0.575 |
| User Namespace | 3.309 | 0.958 | 1.083 | — | — | 0.571 |
| Mount Namespace | 3.322 | 0.957 | 1.083 | — | — | 0.571 |
| PID Namespace | 3.329 | 0.959 | 1.087 | — | — | 0.571 |
| Time Namespace | 3.317 | 0.959 | 1.088 | — | — | 0.570 |

Namespace-path spans are 0.577–0.579 microseconds, near the 0.570–0.571 empty
controls. Boot decode is likewise close to its empty control. Subtracting those
medians would not establish an unbiased operation cost, so no such subtraction
is made and no decode/path optimization is accepted from this profile.

The unobserved reference whole calls are approximately 9.9 microseconds for
Boot and 4.2 for Namespace, including caller-side descriptor retirement. The
open-selected arm has whole-CPU/reference ratio 1.481546 for Boot and roughly
1.292–1.295 for the four Namespace paths. The observer's effect is therefore
material. Selected spans from different runs must not be summed into a production
budget or used as exact syscall-only timings. Namespace open includes the
OpenOptions/path work; filesystem and identity spans include their checks and
wrappers. Close and surrounding glue remain in the whole-call measurement rather
than separately attributed spans.

The useful conclusion is a direction for the next isolated hypothesis: test
the open path or filesystem/identity wrapper with a matched aggregate whole-call
comparison and differential error/flag/EINTR controls. The present data does not
separate kernel cost from wrapper cost within those spans. It does not justify
removing filesystem checks, retaining an earlier successful observation, changing
AppArmor or lengthening the ten-second window.

## Tests and retained evidence

Native release build, all-target/all-feature check and strict Clippy pass on the
first build. Forty-three origin cases pass; five manual screens remain ignored
in that ordinary run. The five added cases cover four observer contracts plus
actual/reference values, conditional refusals and operation counts. Missing/NUL
namespace names and a wrong boot root refuse at the same boundary with the same
error class/errno; later operation counters remain zero.

The private source manifest has 2568 inputs, verified before and after build,
check, lint, normal tests and the eight-process cohort. The test image SHA-256 is
`5123ef8a8bbb4f04840ab4259908ff20b6383af354c44a2e6893b819494d34b7`.
Four offline-reader controls and four saved-byte audit tests pass. Independent
staged-blob review reproduces every estimate, verifies source reversal and rejects
changed source, host frame, pins, protocol, lint flags, process reuse and altered
operation counts.

The content-addressed [evidence index](local-runs/kernel-operation-cost-20261025/index.json)
contains 126 members/96 objects, SHA-256
`45b9c9024a388801f2a2aaeafff592fdb00306151f845a1ad42979eef9544165`.
It retains raw cohorts, native gate logs, commands/manifests, source/reference
proof, frozen tools, local checks and host frames. Formatting and the local
non-promotable performance contract also pass.

The [final checks packet](local-runs/kernel-operation-checks-20261025/index.json)
retains staged verification, doc-check and the final local contract logs:
fourteen members/thirteen objects, index SHA-256
`8f462e1fed444cea87040fa008ecb821030cc4cfeb3876bf0172006ae5eac421`.

Endpoint frames preserve boot, PID1 mount table, nine absent campaign/lifecycle
markers, installed binary/receipt hashes and supervisor PID 8839/NRestarts 0.
AppArmor, clocks, services, installed artifacts and dependencies are unchanged.
No product workload, original delivery, full preparation or qualification runs.
The [prior first full refusal](authority-parent-cost-result-20261025.md) remains
`enroll_once`/`Causal(Time)` at age 10057459559 ns, with zero completed cases.
This attribution does not change the still-open original delivery, all-writer
retirement, completion, later cells or manager/enrollment boundaries.

Qualification remains `prepared-not-authorized`, expensive runs disabled,
manifest SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
