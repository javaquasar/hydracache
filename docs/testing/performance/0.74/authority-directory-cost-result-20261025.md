# Fresh authority directory validation cost

The authority directory and ancestry validators now obtain their existing
metadata predicates through fresh `fstat` projection on Linux x86_64. An
eight-process comparison supports approximately 18.6% lower CPU cost for the
complete fixture-directory method and 19.7% for modeled fixed ancestry, against
their frozen reference methods. Every original fresh observation, conditional
open, FD lifetime, predicate and time window remains in place. The new
candidate's first complete preparation still expires during enrollment, so
this is not full-preparation or production admission.

## Baseline and a separate directory hypothesis

The [previous attribution](authority-fresh-stamp-result-20261025.md) leaves
directory metadata unchanged. Its outputs phase includes 17066 ActiveAuthority
observations and 1.120879 seconds of overlapping wall time. Directory validation
is a concrete component of that view, but the directory result must not be
multiplied into an enclosing CPU speedup or added to nested profile spans.

The fixed path freshly inspects named/held root, three named/held ancestors,
held parent and named/held leaf: eleven metadata observations. The fixture path
inspects named/held parent and named/held leaf: four observations. The predicates
require directory type plus device, inode, UID, GID and complete mode. Mutable
directory times, size and link count are deliberately not equality predicates;
benign unrelated entries must retain their existing behavior.

Commit `96b6f5511ce6e0220c73b2582fb762b66c0f00c0` freezes the
[protocol](../../../../tools/authority-directory-cost-074/protocol.json),
test-only prototype/reference and manual gates before measurement. Production
directory logic is unchanged in this baseline. The corrected native probe image
SHA-256 is
`b31d18291c51d1c10800e63802acacc6c2b2c662f73f384019d767338a19809f`.

The first probe build passes compilation, check and strict Clippy, but its new
fault test fails: 58 passed, 1 failed, 3 ignored. One fixture scenario replaces
its parent FD with another open of `/tmp`, the original parent itself, so no
identity change is injected. That image and first failure remain retained;
no CPU measurements use it. A separate corrected source copy selects a distinct
owned temporary directory and adds failure-context text, changing only those
two test expressions. The expected refusal and all predicates remain unchanged.
That copy passes 59 tests, with three manual screens ignored.

## Frozen comparison and attribution

Each phase starts eight independent nonroot GNU processes, with only each
measurement child pinned to CPU 5. Each process uses fixture then modeled fixed
ancestry, with 32 warmups per arm and sixteen samples of 1024 complete directory
validations per arm. Eight forward/reverse cyclic four-arm permutations run
twice, rotated by process. No compilation or other test from this session
overlaps the measurement.

The four arms are frozen reference, identical-reference control, actual runtime
and the test-only `fstat` prototype. In the baseline phase, actual runtime must
agree with the reference within [0.97,1.03]. This calibrates the copied method
against real code rather than assuming identical code generation. In the
installed phase, actual runtime must agree with the prototype within the same
interval. Identical-reference control must pass in both phases. Candidate CPU
upper interval must be at most 0.95 for both layouts, without changing thresholds.

The estimator takes the median of sixteen paired log process-CPU ratios within
each process, then the median over eight process estimates. The 95% percentile
bootstrap uses 20000 process resamples and seed `0x074d1a05`. Every process,
sample and first outcome is retained; no overhead or outlier is subtracted.

| Complete method | Baseline reference CPU per call | Prototype CPU ratio and 95% interval | Installed runtime CPU ratio and 95% interval |
| --- | ---: | --- | --- |
| Fixture directory | 7.286 microseconds | 0.815736 [0.814505,0.819397] | 0.814115 [0.812528,0.818441] |
| Modeled fixed ancestry | 18.827 microseconds | 0.802427 [0.800404,0.803416] | 0.802622 [0.800780,0.803677] |

Baseline runtime/reference ratios are 0.997567 and 0.997768; both control
intervals fit [0.97,1.03]. Installed runtime/prototype ratios are 0.997874 and
0.997951, also inside that interval. Identical-reference controls remain close
to one in all four comparisons. Both preregistered phases pass. These are
complete directory-method comparisons against frozen same-image references,
not cross-artifact end-to-end, full registry or cache API measurements.

## Narrow runtime integration and fault coverage

Runtime commit `64f184d9ba642ff504aa1943b4724858944b9ade` changes eleven
existing directory-predicate expressions across `diagnostic_authority_fixed.rs`
and `diagnostic_authority_fixture.rs`. `Stamp::directory_matches_file` in
`diagnostic_authority_stamp.rs` calls the already tested fresh Stamp projection
and compares exactly the original directory predicates. Other Linux
architectures keep the original `File::metadata` path. The previous metadata
predicate remains available to frozen test references and that fallback.

There is still one fresh query per original predicate site. The named root is
opened before the first root comparison; each ancestor is reopened in original
order. The ancestry's named root closes before the outer named leaf opens.
Short-circuit failures still suppress later queries/opens. Directory capture,
leaf before-read regular/ownership/budget checks, content reads, signatures,
refusal latching and 300/60/10-second origins/windows are untouched. No durable
or wire artifact, authority capability or FD ownership is introduced.

Two new unit tests compare original, prototype and actual runtime outcomes.
They enumerate all eleven stored fields, distinguishing the five required
equality fields plus directory type from benign fields; inspect a nondirectory
and verify FD flags; and exercise twenty layout/fault cases. The latter include
wrong held objects, wrong stored identity/ownership, missing/replaced names,
mode drift and first metadata failure before a missing later name. Ordered
observation traces have four or eleven sites on the healthy paths. Existing
ancestor replacement, journal refusal, locking, reopen and publication-cut
tests are part of the 59 passing authority cases, not additional new cases.

The runtime image has SHA-256
`98b63f7f9409763aa777aeaac2008df3278c36ae5d31a7267306f2f90ba9b2dd`.
Native release build, all-target/all-feature check and strict Clippy pass.
The private source manifest has 2563 inputs; only the four explicit overlay
files differ from the frozen probe. Reversing the eleven predicate replacements,
fallback availability annotation, one formatting fold, new adapter and two
actual-runtime assertions exactly reconstructs the baseline source after
line-ending normalization. The benchmark/prototype body is unchanged.

## First complete preparation and remaining boundaries

One first full five-case holder attempt uses this runtime image, the unchanged
nine-file bundle, original four-CPU affinity and 1200-second outer bound. It has
no case/cost/material override. The healthy child and parent exit 101 at
`consuming_held_live.rs:267`, `enroll_once`, with nested `Causal(Time)`. The
first expiry is 10086211570 ns, issued BOOTTIME 1653107525320152 ns and observed
1653117611531722 ns. Zero cases complete. The four later cases remain
unexecuted, and the candidate is not retried into a pass.

The failed frontier's age must not be compared with the previous candidate's
single expiry as a performance estimate. A lower directory CPU cost does not
establish that complete preparation fits its ten-second causal window. Original
manager delivery, every-writer retirement, completion and later cells remain
unproved on this path.

The content-addressed [evidence index](local-runs/authority-directory-cost-20261025/index.json)
has 308 members/226 objects, SHA-256
`6df2aa8770bc5621c9339ae03f245721c6ff8164e253c4fa77ab7dbaa9fbf5a5`.
It retains the first faulty fixture, corrected and runtime builds, both raw
eight-process cohorts, exact commands/manifests, source delta, host frames and
first full refusal. Six offline reader controls and four saved-byte audit tests
pass. Independent staged-blob review reproduces both CPU estimates and source
proof while rejecting promotion of either first failure. Formatting, doc-check
and the local non-promotable performance contract pass.

The [final checks packet](local-runs/authority-directory-checks-20261025/index.json)
retains the staged-blob verifier and final doc/contract logs: fourteen members,
thirteen objects, index SHA-256
`11d33cf8b011d80228ab31aecc35044df1017de69273f0039e1b235d87927ba4`.

Endpoint frames preserve boot, PID1 mount table, nine absent campaign/lifecycle
markers, installed binary/receipt hashes and supervisor PID 8839/NRestarts 0.
No fixture cgroup remains. AppArmor, clocks, services, installed artifacts,
dependencies and product workloads are unchanged. Manager 255 compatibility and
the retained signer/AppArmor refusals remain separate environment boundaries.

Next isolate before-read/parent-device metadata and remaining Boot/Namespace
costs as separate hypotheses. Do not combine them with this candidate or reduce
observation frequency. The qualification manifest remains
`prepared-not-authorized`, expensive runs disabled, SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
