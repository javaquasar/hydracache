# Fresh filesystem output projection comparison

Removing the zeroing before a fresh `fstatfs` query does not pass the frozen CPU
gate. All seven baseline calibrations pass, but no candidate interval reaches
the required upper bound of 0.98. Proc/nsfs leaf checks are slightly slower in
this test image; complete Boot/Namespace calls remain near baseline. The
candidate stays test-only without runtime integration or a preparation retry.

## Isolated hypothesis and safety contract

The [preceding operation attribution](kernel-operation-cost-result-20261025.md)
identifies filesystem validation as a repeated cost but includes observer work.
The [rejected open-wrapper experiment](namespace-open-cost-result-20261025.md)
does not justify changing another part of that candidate. This is a separate
output-buffer hypothesis against baseline `c54c284bd`.

Original `filesystem()` initializes a full `libc::statfs` to zero before a fresh
libc query and then compares only `f_type`. The candidate instead supplies a full
aligned writable `MaybeUninit<statfs>`, captures errno immediately on any nonzero
return and reads only the initialized `f_type` after success. It never constructs
a reference to an uninitialized whole struct or calls whole-object `assume_init`.
EINTR remains an immediate error: there is no new retry, cached success or FD.

The [Linux interface](https://man7.org/linux/man-pages/man2/statfs.2.html)
defines filesystem type in this output; the
[Linux 6.8 native conversion](https://github.com/torvalds/linux/blob/v6.8/fs/statfs.c)
copies it into the userspace buffer before returning success. The experiment is
limited to GNU x86_64 and does not replace the libc call or introduce a new ABI.
Its injected query contract requires only `f_type` to be initialized on zero;
nonzero permits no output read. These are memory-safety prerequisites, not
evidence of a performance benefit.

Commit `958b3135a15c00b9f6aad41522cef9d1f6608d3a` freezes the
[protocol](../../../../tools/filesystem-projection-cost-074/protocol.json),
test-only candidate, four copied reference functions and pure reader before
measurement. Removing the two new test-module declarations reconstructs the
baseline production source after line-ending normalization. Reference filesystem,
fixed-child, Boot and Namespace functions match the baseline after mechanical
name/whitespace/comment normalization. The prototype changes only the filesystem
projection call. All fresh opens, syscall/error order, validation predicates,
document handling and FD retirement remain intact. It does not use the previously
rejected Namespace-open helper.

## Fault controls and native gates

The first GNU release build, all-target/all-feature check and strict Clippy pass.
Fifty origin tests pass, with nine manual screens ignored. Three new tests cover
real proc/nsfs, regular file, device, socket and invalid-FD behavior; flags and
identity remain unchanged by validation. Wrong magic, missing/NUL Namespace
names and a wrong Boot root preserve error class/errno and operation counts.

Seed `0x074f5a17` supplies 2048 partial-success/mismatch cases, initializing only
the output field that is actually read. Ten additional cases combine five errno
values, including EINTR, with two nonzero statuses and no output initialization.
Each query executes exactly once. Complete Boot retains four opens/filesystem
checks, one read/decode and one empty observer control; complete Namespace retains
one open/filesystem/identity/path/empty observation. The active operation observer
is used only for controls, not CPU samples.

The first local formatting command exceeded Windows command-line length before
the native build. Its refusal is retained; direct rustfmt on the three affected
files passes without touching unrelated sources. Native gates pass on their
first attempt. No failed product or preparation outcome is replaced.

## Cohort and decision

Eight fresh UID/GID 1000 processes run sequentially on CPU 5, without overlapping
builds/tests from this session. Seven cases cover proc/nsfs leaf checks, full
Boot and user/mount/PID/time Namespace paths. Four arms compare frozen reference,
identical-reference control, unchanged actual runtime and prototype. Each case
has 32 warmups per arm and sixteen samples of 1024 complete calls. Eight cyclic
forward/reverse orders run twice, rotated by process. All 3584 rows are retained.

Every call uses the original live work. Expected Namespace identities are
setup inputs, not additional timed identity queries. No overhead or outlier is
subtracted. The estimator takes a median of sixteen paired log CPU ratios per
process and a median across eight process estimates. A 95% percentile bootstrap
uses 20000 process resamples and seed `0x074d1a05`. Both baseline-control intervals
must fit [0.97,1.03], and every candidate/reference upper bound must be at most
0.98. All controls pass; all candidate gates fail.

| Case | Candidate/reference CPU ratio | 95% interval |
| --- | ---: | --- |
| Proc filesystem leaf | 1.003559 | [1.002867,1.004216] |
| Namespace filesystem leaf | 1.004120 | [1.003484,1.004842] |
| Full Boot | 0.999606 | [0.999023,1.000431] |
| User Namespace | 1.000638 | [1.000424,1.001607] |
| Mount Namespace | 1.000818 | [0.999614,1.002102] |
| PID Namespace | 1.001479 | [0.999215,1.002103] |
| Time Namespace | 1.000519 | [0.998725,1.001871] |

The reference CPU medians are about 375 ns for the two leaves, 9.988 microseconds
for Boot and 4.220–4.231 microseconds for Namespace. These timings include the
compiled wrappers and caller, not isolated hardware-instruction costs. The leaf
increases of about 0.36% and 0.41% do not identify their microarchitectural cause.
They and the near-baseline whole calls reject this exact candidate under the
original rule; no threshold adjustment or new cohort follows.

## Generated code and retained evidence

A separate read-only disassembly inspects the same protected image after the
cohort. Before `fstatfs`, both original and reference functions contain seven
16-byte zero stores plus one 8-byte store; the candidate has none. Thus the
compiler did remove the targeted 120 bytes of initialization. Fewer emitted
stores do not establish lower whole-method CPU cost. This inspection does not
execute a new timing workload or turn the result into a kernel/wrapper cost split.

The measured image SHA-256 is
`fcd26cedc7c37237ec659a0002668dfbdd46d8be2919370b1645e5eed3625b7b`.
The source manifest verifies 2572 inputs. The content-addressed
[evidence index](local-runs/filesystem-projection-cost-20261025/index.json)
retains 142 members/102 objects, SHA-256
`649ddcec8ae7ccf8c0469126131a31c52bb0c75a96740573a2f71af7962fde9b`.
It includes raw cohort, native commands/logs, source/manifests, read-only codegen,
the formatting refusal, host frames and local controls. Five pure reader tests
and four saved-byte audit cases pass; the latter also cover changed codegen pins.
Independent staged-blob review recomputes every estimate and verifies the source
boundary, observation counts, field controls, compiler flags and distinct process
identities. No production candidate is integrated.

The [final checks packet](local-runs/filesystem-projection-checks-20261025/index.json)
retains staged verification, successful doc-check and local non-promotable
performance-contract logs: fourteen members/twelve objects, index SHA-256
`43cee7d5c5d49688ceb1984400d8dae1c8da112ef80a65a161df361cf8ff7c0a`.
Formatting also passes. These focused gates do not complete the still-red full
package/security checks or authorize a push or release.

Boot, PID1 mount table, absent campaign markers and installed artifact identities
remain unchanged. Supervisor stays active with PID 8839/NRestarts 0. Clocks,
services, AppArmor and dependencies are untouched. No release installation,
original delivery, product workload or qualification occurs. The earlier first
full preparation still refuses `enroll_once`/`Causal(Time)` at age 10057459559 ns,
with zero completed cases and all 300/60/10 windows unchanged.

The next attribution should return to remaining complete preparation owners,
with a cost-ranked hypothesis before another runtime change, rather than assume
that a nearby zero-initialization or fewer instructions must save useful CPU.
Original delivery, all-writer retirement, durable completion, later cells and
real enrollment remain open. Qualification stays `prepared-not-authorized`, C74
unresolved and expensive runs disabled; manifest SHA-256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
