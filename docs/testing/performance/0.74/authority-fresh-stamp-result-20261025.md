# Remaining preparation cost and fresh authority leaf stamps

The authority journal now uses fresh `fstat` projection for three existing
held/after Stamp comparisons on Linux x86_64. Eight independent processes
support about 8.5% lower CPU cost for the complete `Leaf::revalidate` method
against its frozen same-image reference. All eleven compared fields remain
present. No directory, ownership, budget, content, signature, live predicate,
conditional read, validation round or original time window is removed.
The first complete preparation of this candidate still expires during enrollment;
the enclosing admission gate remains red.

## Residual attribution before this candidate

One separately frozen attribution-only healthy prefix uses the formatted kernel
identity image, SHA-256
`62caf35baa1f3320c5184b6304fb2623d64338dfb7470dfbefcacd1613113e5a`,
runtime commit `cdf10d042`. The existing `HC074_CONSUMING_HELD_COST=1` observer
runs in the original four-CPU affinity and 300/60/10-second windows. This is not
a repetition of the previous failed functional gate into a pass.

| Phase | Wall seconds | Process CPU seconds | Returned |
| --- | ---: | ---: | --- |
| Prepare consent | 0.559495 | 0.559483 | yes |
| Accept consent | 0.291534 | 0.291509 | yes |
| Outputs | 6.803713 | 6.712456 | yes |
| Enrollment frame | 0.347098 | 0.346981 | yes |
| Fixed manager message | 0.330993 | 0.325261 | yes |
| Owned root preparation | 0.936870 | 0.936030 | no |

Outputs retains 46 calls/completions in every original round slot. It contains
76544 Origin, 153088 Boot and 306176 Namespace observations. Boot accounts for
1.415889 seconds outside its nested children; Namespace accounts for 1.125483
seconds. The separate live ActiveAuthority view records 17066 calls and
1.120879 seconds. These are overlapping wall views, not additive CPU owners.
Neither their sum nor wall minus process CPU is an I/O estimate.

The profile exits 101 in owned-root preparation. Its first causal expiry is
10033755319 ns, issued BOOTTIME 1646858399125130 ns and observed
1646868432880449 ns. Four later phases are unobserved. The previous baseline,
profile and kernel-identity first failures remain immutable.

## Isolated hypothesis and acceptance

Authority leaf validation still constructs standard-library metadata for its
full Stamp comparisons, although it consumes only eleven Unix fields. The
new hypothesis changes only how those fields are freshly projected, not which
fields are compared or when the observation happens. Directory metadata and the
metadata preceding regular-file, UID/GID, mode, link-count, device and budget
checks remain on their original path.

Commit `3e72d0185` freezes the [protocol](../../../../tools/authority-fresh-stamp-074/protocol.json)
and standalone prototype before integration. Each of eight new nonroot GNU
processes executes twelve samples in all six forward/reverse Latin permutations,
rotated by process. Each leaf sample has 8192 iterations over seven held objects;
the installed-method sample has 1024 complete validations of a 1024-byte leaf.
Only each measurement child is pinned to CPU 5; no build/test workload overlaps.

The estimator takes each process's median paired log CPU ratio, then the median
over eight processes. The 95% percentile bootstrap has 20000 draws and seed
`0x074fd105`. Identical-reference control must remain within [0.97,1.03]; the
candidate upper interval must be at most 0.95. No process/sample is discarded.

| Scope | Candidate CPU ratio | 95% interval | Identical control ratio |
| --- | ---: | --- | ---: |
| Eleven-field projection prototype | 0.627880 | [0.621457,0.632375] | 0.999897 |
| Actual complete authority leaf method | 0.915110 | [0.913530,0.915821] | 0.999282 |

The control intervals are [0.999650,1.000176] and [0.998992,0.999775]
respectively. Both preregistered CPU screens pass. The second result supports
the integrated leaf change; it is not a measurement of full authority inspection,
complete preparation or RESP/native cache requests.

## Implementation and native controls

Runtime commit `317f8a727f80b93663d705c661338d8761da7dba` adds
`diagnostic_authority_stamp.rs` and changes exactly three expressions in
`diagnostic_authority_fixture.rs`: the held Stamp comparison and the two
after-read comparisons, including initial reading. Every call observes the same
borrowed FD synchronously and returns a newly populated Stamp. Device, inode,
UID, GID, mode/type, link count, length, mtime/nanoseconds and ctime/nanoseconds
are retained. I/O failures capture errno immediately. Other Linux architectures
retain `File::metadata`; no cache, retained capability or durable/wire format is
introduced.

The original before-read metadata, `regular` check, fresh parent-device query,
buffer selection, bounded read, after-read comparison and content comparison
keep their order and short-circuit behavior. The fresh named FD stays alive
through both comparisons. Scratch eviction and journal refusal latching are
unchanged. Reversing only the module declaration and three expressions exactly
reconstructs the baseline fixture source after line-ending normalization.

A separate private source copy preserves the previous 2560 source inputs except
the one changed fixture file and adds two new Rust files. The resulting image
SHA-256 is
`70b666ce7e1afb5009b733cf88ba96f82f389edcb8aef3d11ea3f5764c33326c`.
Default-feature release build, all-target/all-feature check and strict Clippy
pass. The authority filter passes 57 tests, with the new manual screen ignored.
Two new unit tests cover real object fields, 32 seeded temporary-file timestamp changes (not clock changes),
permissions, length, hard links, unlink/replacement, bad FDs and nineteen
valid/fault leaf cases. The latter include each original stored field, scratch
eviction, ownership/budget refusal and held-metadata failure before a missing
named file. Existing authority reopen, refusal, publication-cut, locking and
history controls are included in the 57, not counted as additional new tests.

## First complete preparation remains refused

The new candidate's first functional attempt uses the full five-case holder
schedule, with no case/cost override, unchanged bundle and original windows.
Its first healthy child and parent exit 101 at `consuming_held_live.rs:267`,
`enroll_once`, with a nested `Causal(Time)` refusal. The causal record is
`expired` at 10116757205 ns, issued 1647495437583299 ns and observed
1647505554340504 ns. Zero cases complete; revalidate, role-before, role-after
and trust-after remain unexecuted. There is no retry-to-pass.

Reaching enrollment rather than the earlier owned-root-preparation frontier is
a first-attempt observation, not a repeated end-to-end speed estimate. The local
leaf gain is insufficient to admit this preparation. No original manager send,
product start, all-production-writer retirement or completion is established.

## Evidence and next boundary

The content-addressed [evidence index](local-runs/authority-fresh-stamp-20261025/index.json)
has 244 members/172 objects, SHA-256
`19bd1fa220d0b77cbcfbea33ccd1491f423acc1726d13714ffa802355b09d81c`.
It retains both screens, all first logs/results, source manifests, the residual
profile, host frames, compiler/check/lint commands and source/protocol bytes.
The saved-byte audit recomputes both estimators and validates the exact source
delta, process identities, original windows, unchanged endpoint frames and red
holder result. Six reader controls and four saved-byte audit tests pass;
staged Git blobs independently reproduce the audit. Local formatting, doc-check
and non-promotable performance-contract checks pass.

The [final checks packet](local-runs/authority-fresh-stamp-checks-20261025/index.json)
retains the staged-byte verifier and final doc/contract logs: 15 members/14
objects, index SHA-256
`4fcf1a58849402161f98e84969becbf7f7566cd36118cadb7cd198b2a0c00809`.
An initial local evidence-retainer parse error is retained there; only collection
is corrected, with no test, measurement or server action repeated.

Boot, all nine absent campaign/lifecycle markers, PID1 mount table, installed
binary/receipt hashes and supervisor PID 8839/NRestarts 0 remain unchanged.
No fixture cgroup remains at endpoint frames. AppArmor, clocks, services,
installed release, dependencies and product workloads are untouched.

Next isolate residual directory/before-read metadata and Boot/Namespace syscall
costs under separate leaf hypotheses; do not broaden this candidate or infer a
speedup from the failed preparation. Manager 255 compatibility, the retained
51 signer refusals with correlated AppArmor denials, original delivery,
every-writer retirement, later lifecycle cells and full-package/security gates
remain separate blockers. The qualification manifest remains
`prepared-not-authorized`, expensive runs disabled, SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
