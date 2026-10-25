# Fresh kernel identity optimization and signer fixture refusals

The supervisor now uses fresh `fstat` device/inode projection for Linux x86_64
kernel-origin checks. A separately frozen eight-process comparison supports
about 7.2% lower CPU cost for the complete `Origin::revalidate` method. Every
original filesystem, namespace-type, named/held identity, Boot and time check
remains in place, with the same conditional order and observation frequency.
This is a monitoring/control-path improvement, not a cache API throughput claim.
The first complete preparation of this candidate still fails its ten-second
custody/challenge window. Server or release admission remains closed.

## Signer compatibility with the repaired environment

The [previous first library result](server-capacity-attribution-result-20261025.md)
remains 1348 passed, 82 failed, 122 ignored. A new signer-only cohort uses the
same default-feature GNU image, SHA-256
`6640e1b9b31ae7c87137b335be4f2665d729ea7b4324ee0e874bdbedc346c520`,
and the previously verified private HOME. No real account HOME is modified.
The exact `diagnostic_signer_` catalog is frozen before execution: 151 names,
four test threads, UID/GID 1000, CPU affinity 0–7, unchanged 780-second outer
bound, no ignored-test flags and no retry.

The first result is 100 passed, 51 failed, zero ignored in 4.809 seconds. The
missing rustup prerequisite is no longer the only barrier. Read-only kernel
inspection finds AppArmor enabled and
`apparmor_restrict_unprivileged_userns=1`. The journal records diagnostic signer
threads transitioning into `unprivileged_userns` and being denied capability
18, `sys_chroot`; the fixed child setup requires `chroot` after creating its
user namespace. This establishes a host-policy incompatibility for this fixture
class, not a per-case causal proof for every generic refusal. No AppArmor,
capability, namespace, credential or test predicate is weakened to obtain a pass.

The remaining cases require a separately reviewed compatible fixture environment
or narrowly scoped policy decision. General server CPU permission does not
authorize disabling AppArmor or weakening the host globally. Neither the old
ordinary cohort nor the new signer cohort is retrospectively passed.

## Baseline and isolated metadata hypothesis

The preceding profile identifies repeated fresh kernel observation as a cost
owner: outputs contains 153088 Boot and 306176 Namespace observations. The
isolated hypothesis is narrower than eliminating any of those observations:
device/inode-only consumers need not construct the full standard-library
metadata projection on each call.

Commit `11445ff7304fa49beaf3be16ed6393155f5a183b` freezes the standalone source
and [leaf protocol](../../../../tools/kernel-fresh-identity-074/protocol.json)
before runtime integration. Eight independently started nonroot GNU processes
each execute twelve samples, with all six forward/reverse Latin permutations
of original, identical-original control and `fstat` prototype. Each sample
executes 8192 iterations over seven retained objects: procfs, four thread
namespaces, a character device and an owned regular file. Values are checked
before and after measurement. There is no cached successful observation.

The preregistered estimator takes the median of twelve log paired CPU ratios
within each process, then the median across eight process estimates. The 95%
percentile bootstrap uses 20000 resamples and seed `0x074fd105`. Every first
process and every sample is retained. Same-function control must remain wholly
inside [0.97,1.03]; prototype CPU upper interval must be at most 0.95.

The leaf ratio is 0.626101, interval [0.625447,0.628068]. The identical-function
control is 1.000147, interval [0.999212,1.001004]. This supports the isolated
metadata hypothesis, not a 37% improvement to full validation or the product.

## Runtime implementation and differential controls

Commit `8862986662063b329f17547f61458f08832767e2` adds
`diagnostic_kernel_fd_identity.rs` and routes only the existing device/inode
projection sites in `diagnostic_kernel_origin.rs` through it. Each x86_64 call
performs a fresh synchronous `libc::fstat` on the same borrowed FD, captures
failure immediately and returns only device/inode. Other Linux architectures
retain `File::metadata`; no cache, retained owner, public API, wire/durable
artifact, FD ownership or authority capability is added.

The replacement preserves filesystem validation before identity observation,
namespace type before namespace-object comparison, short-circuit refusal before
Boot/named-namespace reads, and all sample/clock brackets. Original files are
reopened and checked as before. No directory prefix, Boot document read,
signature, policy, freshness predicate or validation round is removed.

The first new test compilation fails because the new test module omits the
`std::fs` import. That compiler result and its source manifest remain retained;
no test or measurement runs from it. A separate corrected source copy adds only
the missing import. Its source manifest has 2560 entries and preserves all
unchanged baseline bytes; the original 2558-input source copy remains intact.

The corrected default-feature GNU image has SHA-256
`7ac9b2462222ce61c0486ab79cdebf47d6f94a62540abd6eaf561dbb6ccff9ec`.
Release no-run build, all-target/all-feature check, strict Clippy with
`-D warnings`, and origin regressions pass. The last group passes 38 cases,
including three new differential controls; four manual measurements stay
ignored. New controls compare real filesystem objects and flags, retained
unlinked/replaced files, bad-FD errors and original trace counts/refusal sites
for valid and drifted contexts. These 38 include the earlier 35, not 38 extra
unique cases. No full package or milestone pass follows from this filter.

## Complete origin CPU comparison

The separate [installed-origin protocol](../../../../tools/kernel-fresh-identity-074/installed-origin-protocol.json)
is frozen before its observations. It compares an original reference and its
identical control with the actual installed `Origin::revalidate` in the same
image. Eight fresh nonroot processes run twelve balanced samples of 1024 full
validations per arm, pinned only within each child to CPU 5. Compilation and
signer workloads have already finished. The estimator/control are unchanged;
the preregistered full-origin CPU upper bound is 0.98.

Installed/reference CPU is 0.928180, interval [0.927090,0.928674]. The identical
reference control is 0.999809, interval [0.999717,0.999894]. All eight exact tests
pass. This supports lower CPU cost of complete fresh origin validation on this
host and compiler. It does not establish the whole preparation's speedup,
RESP/native API improvement, MSRV/other-target qualification or original start.

## First full preparation of the changed candidate

One full five-case original holder schedule then uses the corrected candidate
image without cost/case overrides, in its original four-CPU affinity, original
300/60/10-second windows and 1200-second outer bound. The same nine read-only
bundle inputs are checked before/after; there is no original manager send or
product execution. This is the changed candidate's first functional attempt,
not another execution of the failed baseline image.

Its first healthy subprocess and parent exit 101 at
`consuming_held_live.rs:245`, `prepare_owned_root_guardians`, with
`Context(Frame(Original(Original(Challenge(Time)))))`. The custody age record
states `expired` at 10005245622 ns, from issued BOOTTIME 1642281237309790 ns.
The separate causal trace has no recorded refusal and its last observation is
9989735741 ns; a null causal refusal is not a pass of the custody check. Zero
cases complete and the remaining four are unexecuted. The old baseline and
profile first failures remain unchanged. Single-attempt frontier ages are not
used as comparative performance estimates, and this candidate is not retried.

## Evidence and remaining gates

The content-addressed [evidence packet](local-runs/fresh-kernel-identity-20261025/index.json)
retains the frozen sources/protocols, exact first stdout/stderr/results, compiler
refusal, old and corrected source manifests, component hashes, host frames,
kernel policy/audit observations and independent offline estimates. A first
read-only export also fails on a missing helper argument; its refusal is kept,
and only artifact collection is corrected, without repeating a workload.

The primary packet has 352 members/204 objects, index SHA-256
`ae5029f4ce80d03e1aa4983278daf44320415eefb0b17b7bd61dc43129a888d1`.
Seven offline estimator controls and four saved-byte falsification controls pass;
the latter reject host drift, promoted failures, weakened lint, changed windows
and reuse of a measurement process. They do not replace native fault gates.

A final formatter check requires only moving one conditional brace to the next
line. Commit `cdf10d042` applies that exact whitespace-only difference. A fresh
private source copy again passes native release build, all-target/all-feature
check, strict Clippy and 38 origin cases. Its image SHA-256 is
`62caf35baa1f3320c5184b6304fb2623d64338dfb7470dfbefcacd1613113e5a`.
The exact whitespace proof and final source hashes are retained in the
[supplemental checks](local-runs/fresh-kernel-identity-checks-20261025/index.json).
The first offline proof expression retained an extra space and refused; only
that expression is corrected against the already downloaded bytes. Neither a
CPU measurement nor full preparation is repeated. The numerical results above
belong to measured commit `886298666`, not a newly qualified formatted artifact.

The supplemental packet has 66 members/51 objects, index SHA-256
`656f55ee849d3c7ea75248acd67852c1891f7134f06d397bdd024411bf2ced12`.
It retains the final native build/check/lint/regression records, formatter check,
staged-byte primary review, doc-check and local non-promotable performance
contract, all passing. The full package, signer cohort and complete preparation
remain distinct red gates.

Boot, nine absent campaign/lifecycle markers, PID1 mount-table hash, installed
binary/receipt hashes and supervisor PID 8839/NRestarts 0 match endpoint frames.
No owned fixture cgroup remains. Only private temporary source, tools and
fixtures are used; no release installation, supervisor stop, service/clock/host
policy change, dependency update or product workload occurs. Endpoint cleanup
does not prove cessation of every production writer.

Next attribute the remaining preparation cost under a new finite protocol and
decide the compatible signer/manager environment separately. Preserve the
existing frequency, origins and first refusals. Production enrollment,
authenticated original delivery, every-writer retirement, later lifecycle cells,
the full package and qualification remain open. The qualification manifest
retains SHA-256 `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
