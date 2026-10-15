# SCM_RIGHTS EOF fixture isolation

The unchanged-source baseline passes 68 tests at
`61d664e97337f535b51f191561f6914a56f5b082`. Preregistration:
`01b78e98`, [plan](../../scm-rights-eof-isolation-plan-20261015.md).
The initial test-only implementation is `b1ee4fd1a9c707e956783284daac3b424b2fdcca`;
the final helper-placement correction is `3cfb6c37c88ac36d0054055e25805462e36e949e`.
Commands, capture status, digests and source identities are in
[manifest.json](manifest.json). This packet is unsigned consistency evidence,
not signed execution attestation or server admission.

## Controlled negatives and correction

An isolated test queues writer rights before fork. Its original child closes the
unrelated endpoints, checks CLOEXEC and acknowledges that it holds the inherited
writer. After local writer close and normal credential rejection, the old immediate
EOF assertion fails with EAGAIN. The original child is still nonterminal; the
existing owned Process guard handles panic cleanup. The passing characterization
requires WouldBlock at that cut, releases the child through a fixed gate, requires
original pidfd exit0/WNOWAIT/reap/ECHILD, then requires immediate EOF. Correctness
does not depend on scheduling delays or an EOF retry.

A second deliberately red control retains the actual two received SCM_RIGHTS
writers. Its EOF assertion also fails and the wrapper propagates the nonzero
inner result. The final control requires WouldBlock with two received descriptors
and then one, and EOF only after both close. A third new test rejects zero selected tests even
when the exec exits successfully; the exact completion marker is mandatory.

The two existing EOF fixture bodies are unchanged modulo indentation. They now
exec exactly their own registered unit tests, in an empty child environment with
a private marker. Outer default concurrency remains unchanged; the local mutex
passed to the existing guard is per invocation, not a global serialization lock.
Only cfg(test) code and its test-support import change. Channel, the raw normal
worker, policy/transport semantics, guard/deadlines, original seven process
fixtures, dependencies, product crates and qualification inputs are unchanged.

## Results and source phases

The focused loss units pass 13 tests. Two development root groups pass 71, but
these are intermediate observations: one ran before explicit child close-return
checks were added; the second overlapped that edit and is not assigned the final
source identity. Neither is used as a final-source canonical receipt.

The initial implementation passes 71 on Linux Rust1.94 and Rust1.88, including
one post-commit current-toolchain group and five independently started 51-unit
repetitions. The next static contract run fails 164/1 because the whole loss
source file forbids `Command::`, including its inline cfg(test) module. The
correction moves the exec adapter into the existing separate root-test file,
keeping only thin test-side delegation in the loss module. No contract assertion,
forbidden token, threshold or normal-build code changes. The focused static
check and then all 165 performance-contract tests pass. All six registry tests
pass and all 176 gates remain registered without a registry edit.

The corrected final-source root group passes 71 on Linux Rust1.94 and Rust1.88.
The latter explicitly refuses the extra-libm normal runtime image under the
unchanged four-role graph; it is not a positive runtime admission. Nine unchanged
process-guard tests pass on both toolchains, including their caught real SIGKILL
timeout. The zero-selection test similarly catches its intended missing-marker
panic; these caught negatives are distinct from the two exit1 red captures.

All-target/all-feature package checks and strict Clippy pass on Windows/Linux
Rust1.88/1.94 before and after the placement correction. Windows checks do not
execute the Linux-only root fixtures.
Additional post-commit repetitions and static gates are recorded individually
in the manifest and raw logs. No timing ratio or product performance claim is
derived from these correctness runs.

The previous [unexpected 46/1 capture](../root-guard-attribution-20261015/hash-attribution-2.log)
is unchanged. The acknowledged-child experiment proves the possible inheritance
mechanism, not that historical writer's identity. It closes the demonstrated
fixture ownership ambiguity; it does not prove overlapping-load robustness,
production writer revocation, authenticated original-start or completion/later
cells. No product workload, server action, qualification, push or release gate
was opened.
