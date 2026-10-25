# Authority parent device validation cost

Fresh parent-device projection now replaces two `File::metadata().dev()` sites
in the authority leaf reader on Linux x86_64. Eight independent processes support
approximately 7.6% lower CPU cost for the complete marker leaf validator and
7.4% for the record leaf validator, against their frozen same-image reference.
Before-read ownership/type/link checks, the parent observation, size-budget
precedence, all four observations in full revalidation and original windows
remain intact. First full preparation still expires during enrollment; the
local CPU gain does not authorize original start or product execution.

## Frozen baseline and narrow hypothesis

The [before-read candidate](authority-before-cost-result-20261025.md) retains
the parent-device query. Its runtime is the starting point here. Baseline
commit `68d4b89e516c3350e70d164b18dc9813f91f9980` freezes the
[protocol](../../../../tools/authority-parent-cost-074/protocol.json), test-only
reference/prototype, offline reader and two manual non-ship gates before CPU
measurement. No production expression changes in that baseline.

Both readers reopen the named file and freshly check its regular type, owner,
group, permissions and link count. Only then do they observe parent device,
before the size-budget check, read bytes and compare the fresh ending Stamp.
The prototype substitutes `Stamp::fresh(dir)?.dev` at that one query site. It
does not cache the parent device, change the directory validation policy or
make mutable parent fields into additional equality predicates.

The native probe passes release compilation, all-target/all-feature check,
strict Clippy and 63 authority tests on its first attempt; seven manual CPU
screens are ignored in that ordinary test run. Its SHA-256 is
`925dd4945a3d0c828a459198ac89fa959e91f6f22cf00e6994451987acf89ad5`.

## Matched CPU comparison

Each phase starts eight independent nonroot GNU Rust 1.94 release processes,
with each measurement child pinned to CPU 5. No build or other test from this
session overlaps either cohort. Each process runs marker then record payloads:
65 bytes/budget 65 and 1024 bytes/budget 2048. Four arms are frozen reference,
identical-reference control, actual runtime and prototype. Each has 32 warmups
and sixteen samples of 1024 complete leaf revalidations. Eight forward/reverse
cyclic orders repeat twice, rotated by process; no sample is discarded.

The estimator takes the median of sixteen paired log process-CPU ratios per
process, then the median over eight process estimates. The 95% percentile
bootstrap uses 20000 process resamples and seed `0x074d1a05`. Identical-reference
control must fit [0.97,1.03]. Actual baseline runtime must agree with the copied
reference inside that interval; after integration actual runtime must agree
with the prototype instead. Both payload candidate upper intervals must be at
most 0.98. These thresholds are frozen before observation.

| Full leaf payload | Baseline reference CPU per call | Prototype ratio and 95% interval | Integrated runtime ratio and 95% interval |
| --- | ---: | --- | --- |
| Marker | 5.637 microseconds | 0.925107 [0.922994,0.926438] | 0.924130 [0.922977,0.924902] |
| Record | 5.658 microseconds | 0.928110 [0.926827,0.928808] | 0.926178 [0.924135,0.927086] |

All controls and both phases pass. Actual baseline runtime/reference ratios
are 1.000581 and 1.000792. Integrated runtime/prototype ratios are 0.998279 and
1.000367. The result covers the complete leaf validator, not only one metadata
projection. It does not quantify a registry, full-preparation, RESP or native
API speedup. Earlier experiments have different denominators and their gains
must not be added to this one.

## Runtime delta and fault coverage

Runtime commit `e5284749e84a1a025ea1ef47f2b20e744a72bed1` adds
`Stamp::fresh_device` in `diagnostic_authority_stamp.rs`. Linux x86_64 consumes
device from the existing freshly queried Stamp; other Linux architectures
keep the original `File::metadata().dev()` projection. Exactly two parent-device
expressions change in `Leaf::read` and `Leaf::read_reusing` in
`diagnostic_authority_fixture.rs`. The regular-file guard remains before them;
device mismatch still short-circuits size and returns Budget. Bounded reads,
ending stamp/content comparison, scratch handling and descriptor lifetimes are
unchanged. No new cache, authority capability or durable/wire artifact appears.

The new device test compares original/prototype/runtime for regular, directory
and special descriptors, a retained duplicate, benign parent-entry/mode changes
and a distinct device. Fourteen full leaf scenarios compare all three paths:
healthy/empty payload, wrong UID/GID, budget, permissions/setuid, extra link,
oversize, missing/replaced/symlink name, NUL and held identity mismatch. Refusal
classes, errno and scratch state agree; before-read Invalid still precedes
Budget, and held mismatch still suppresses a later missing-name open. Existing
fresh-Stamp invalid-FD and filesystem/refusal/reopen tests remain in the same
63-case authority gate.

Integrated native release build, all-target/all-feature check, strict Clippy
and 63 authority cases pass. Its image SHA-256 is
`1d43e73b00403b0ee834d11b392f754b124b40e5e6151d1a28cc22a020f5afc1`.
Both private build manifests contain 2565 inputs. Reversing the two expressions,
new helper and two added runtime assertions exactly reconstructs the frozen
baseline after line-ending normalization. Reference, prototype, timing loop,
thresholds, before-read implementation and surrounding validations are unchanged.

## First preparation and residual profile

The first five-case holder attempt uses that exact image, the unchanged
nine-file bundle, original affinity and 1200-second outer bound, with no case,
cost or material override. Its healthy child and parent exit 101 at
`consuming_held_live.rs:267`, `enroll_once`, with nested `Causal(Time)`.
The first causal expiry age is 10057459559 ns: issued BOOTTIME
1656339693883905 ns, observed 1656349751343464 ns, same boot. Zero cases
complete; the other four remain unexecuted. This attempt is not retried into
a pass, and failed frontier ages are not an end-to-end speed comparison.

One separate attribution-only prefix uses the same protected image and existing
`HC074_CONSUMING_HELD_COST=1` observer, selecting only healthy. It is not a
functional gate or a retry of the matrix. Its original guard clocks/windows
remain unchanged, and it also exits 101, now in the profiled enrollment phase,
at causal age 10005376028 ns. Placement, fixed-slice and consuming phases are
not observed. Observer overhead is retained, not subtracted.

| Profile phase | Wall seconds | Process CPU seconds |
| --- | ---: | ---: |
| Prepare consent | 0.544193 | 0.544168 |
| Accept consent | 0.284035 | 0.284018 |
| Outputs | 6.680365 | 6.596969 |
| Enrollment frame | 0.342298 | 0.342224 |
| Fixed manager message | 0.322759 | 0.317186 |
| Owned root preparation | 0.958626 | 0.958186 |
| Enrollment before failure | 0.129972 | 0.129901 |

Outputs retains 46 calls/completions in each of the eight round slots and the
prior prefix counts: Origin 76544, Boot 153088, Namespace 306176 and
ActiveAuthority 17066. The Boot view records 1.420837 seconds outside its
instrumented children; Namespace records 1.137560 seconds. ActiveAuthority
records 0.959482 seconds in its separate overlapping view. These wall views
are not disjoint CPU costs and must not be summed across views or converted
into a repeated preparation speedup. Authentication is 782 calls/28.265145 ms,
including 782 signatures/24.051980 ms; it is not the dominant measured span.

The next bounded attribution should distinguish operation costs inside fresh
Boot/Namespace observation: fixed opens, filesystem checks, identity projection,
document read/validation and user-space wrappers. A proposed replacement needs
its own frozen comparison and fault gate. Reusing an earlier success, removing
an observation, extending a deadline or changing AppArmor is not permitted.

## Evidence and remaining admission boundaries

The [evidence index](local-runs/authority-parent-cost-20261025/index.json) has
291 members/209 objects, SHA-256
`afb69bd536c0399dc378095dd6d3420a4bbb451cf0ce79675370a00c08c7551b`.
It retains both builds, both raw eight-process cohorts, full first refusal,
residual profile, commands/manifests, source proof, checks and host frames.
Six reader controls and five saved-byte audit tests pass. Independent staged
Git blob review reproduces the estimates and rejects changed source, host,
windows, lint flags, process reuse and promotion of either the failed matrix
or attribution-only profile. Formatting and the local non-promotable
performance contract pass.

The [final checks packet](local-runs/authority-parent-checks-20261025/index.json)
retains staged-blob verification, doc-check and the final local non-promotable
performance-contract logs: fourteen members/thirteen objects, index SHA-256
`fa9debeda161d8361d74e92bf794998c12e786edeb64b7da2577d86c73bab9a8`.

Endpoint frames preserve boot, PID1 mount table, nine absent campaign/lifecycle
markers, installed binary/receipt hashes and supervisor PID 8839/NRestarts 0;
no fixture cgroup remains. No AppArmor, clock, service, installed release,
dependency or product workload changes occur. Manager 255 compatibility,
signer/AppArmor refusals, full preparation, original delivery and all-writer
retirement remain open. Qualification stays `prepared-not-authorized`, expensive
runs disabled, manifest SHA-256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
