# Fixed Namespace open comparison

The fixed-literal GNU open candidate does not pass its preregistered CPU gate.
All four Namespace cases pass baseline calibration, but their candidate/reference
upper bounds exceed 0.98. The candidate remains test-only: no runtime integration,
installed-image cohort or full preparation retry follows this result.

## Hypothesis and preserved behavior

The [operation attribution](kernel-operation-cost-result-20261025.md) identified
open as the largest selected span, but did not separate kernel time from wrapper
work. This experiment changes only the wrapper for user, mount, PID and time
Namespace opens. It calls GNU `open64` on fixed terminated literals, still
executing the original `namespace_path(name)` exactly once. Other names keep
the original `OpenOptions` fallback, including invalid-input errors.

The [pinned Rust 1.94 filesystem implementation](https://github.com/rust-lang/rust/blob/1.94.0/library/std/src/sys/fs/unix.rs)
uses `open64` for GNU. Its [Unix conversion helper](https://github.com/rust-lang/rust/blob/1.94.0/library/std/src/sys/pal/unix/mod.rs)
repeats interrupted operations. The candidate retains `O_RDONLY | O_CLOEXEC`,
the unused mode 0666, immediate errno capture and EINTR retry. Success creates
one owned File; no FD, identity or earlier validation result is reused.

Commit `284808a69f4fbce76a68e73b60eaac42198dc8ce` freezes the
[protocol](../../../../tools/namespace-open-cost-074/protocol.json), test-only
candidate, copied reference and reader before measurement. The production
source is baseline `bb15ffb99` plus two `cfg(test)` module declarations.
Removing exactly those declarations reconstructs the baseline after line-ending
normalization. The whole-method reference matches its original function, and
the prototype differs only at the open expression. Filesystem validation, fresh
identity, error precedence and FD retirement stay in their original order.

## First build refusal and fault controls

The initial local draft had a stray separator token; formatting refused it before
native execution. After that syntax correction, the first native image built and
passed check but failed strict Clippy on a permissions literal and a complex test
function-array type. Its source, image receipt and logs remain retained, with no
fault tests or CPU samples from that image. The correction changes only `0` to
`0o0` and introduces a type alias; source-manifest comparison verifies that scope.

The corrected native GNU release build, all-target/all-feature check and strict
Clippy pass. Forty-seven origin tests pass; seven manual screens remain ignored.
Four added tests cover identity/flags/errors/fallback, scripted EINTR and terminal
errors, full-method observation counts, and real interruption of a blocking open.

The FIFO test creates its own child for each arm, installs a child-only SIGUSR1
handler without SA_RESTART, waits until that exact test thread is blocked in open,
then uses targeted `tgkill`. After the handler acknowledgement it observes the
blocked open again and releases the FIFO. Both arms report syscall 257/openat,
open flags 524288, FD flags 1 and file flags 32768. Only those fixture children
receive signals; no existing process or service is modified. This is an EINTR
control, not the original-delivery/all-writer SIGKILL matrix.

## Frozen cohort and negative decision

Eight fresh GNU release processes run sequentially as UID/GID 1000 on CPU 5,
with no overlapping build/test from this session. Each process measures four
Namespace cases, sixteen samples per case and 1024 whole calls per sample.
The four arms are frozen reference, identical-reference control, actual unchanged
runtime and prototype. Eight forward/reverse cyclic orders run twice, rotated
by process. Each arm receives 32 warmups per case. All 2048 rows are retained.

Every invocation freshly opens, checks nsfs, checks identity and retires its FD.
The operation observer is inactive, and no overhead or outlier is subtracted.
The estimator uses the median of sixteen paired log CPU ratios per process,
then the median across eight process estimates. A 95% percentile bootstrap uses
20000 process resamples with seed `0x074d1a05`. Both reference-control and
unchanged-runtime/reference intervals must fit [0.97,1.03]. They do for all four
cases. Acceptance additionally requires the prototype/reference upper bound to
be at most 0.98 for every case.

| Namespace | Prototype/reference CPU ratio | 95% interval | Passes 2% floor |
| --- | ---: | --- | --- |
| User | 0.995562 | [0.993711,0.996142] | No |
| Mount | 0.995114 | [0.994261,0.995694] | No |
| PID | 0.995336 | [0.989504,0.998122] | No |
| Time | 0.994172 | [0.978201,0.996726] | No |

The point reductions are approximately 0.44–0.58% of these whole methods, not
cache throughput. Reference CPU is approximately 5.55–5.78 microseconds per call
in this cohort. Those absolute times must not be compared with the prior
instrumentation cohort as a runtime regression or gain: they are not a matched
cross-image experiment. The calibrated result answers the narrow question here:
removing this conversion does not produce the required whole-method benefit.

The next experiment should target a separately attributed filesystem/identity
wrapper or another measured preparation cost. An expensive open span alone is
not evidence that its Rust wrapper is worth replacing. Do not lower the floor,
remove a fresh check, or bundle another hypothesis into this failed candidate.

## Evidence and release boundary

The content-addressed [evidence index](local-runs/namespace-open-cost-20261025/index.json)
has 176 members and 129 objects, SHA-256
`3412aef1833da4103485c072f67f3f2b6781428577d93a0e491edf081fef9e27`.
It retains both builds, the first lint-image receipt, raw cohort, source manifests,
exact commands, host frames, local controls and the offline audit drafting refusal.
The latter was an incorrect assumption about where the worker records a failed
command; fixing the reader does not alter raw results or execute another cohort.

The measured image SHA-256 is
`82217f857b4a43c55e8060f7df7f4dd3157587af51a4b66b3038c81cf75feb3f`;
the first lint-failed image is
`d8e277a56b20076d9611c019f22c4c819743cd8e8c91113d8e518a13008d6f88`.
The corrected source manifest has 2570 verified inputs. Five pure reader tests
and four saved-byte audit tests pass. Independent staged-blob review reproduces
every estimate and verifies source preservation, real FIFO results, host frames,
pins, protocol, strict lint, distinct process identities and unchanged work.

The [final checks packet](local-runs/namespace-open-checks-20261025/index.json)
retains staged-blob verification, successful doc-check and the local
non-promotable performance-contract check: fourteen members/twelve objects,
index SHA-256
`2cede81cd62bc0d43d6b0265b6e8fc5c4c8bfe5e53f5c0dab334f08ea512416c`.
Formatting also passes. These focused checks do not replace the still-red full
package/security gates or authorize a push or release.

Boot, PID1 mount table, absent campaign markers and installed artifact hashes
remain unchanged. Supervisor stays active with PID 8839 and NRestarts 0.
No AppArmor, clocks, services, release installation or dependencies change.
No original delivery, product workload or qualification runs. The previous first
full preparation refusal remains `enroll_once`/`Causal(Time)` at age 10057459559 ns
with zero completed cases; this cohort supplies no later-cell or retirement proof.

Qualification stays `prepared-not-authorized`, expensive runs disabled, candidate
unresolved, manifest SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
