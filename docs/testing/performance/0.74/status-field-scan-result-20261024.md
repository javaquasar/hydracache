# Fresh status indexing passes the isolated CPU gate

Runtime candidate `81f4add2` indexes Pid, Tgid, Uid, Gid and Groups once within
each freshly read status document. It passes the preregistered pure-parser CPU
gate and both GNU 1.94/MSRV 1.88 correctness lanes. It does not establish a full
preparation margin, original delivery, all-writer retirement or release readiness.
The complete source-contract cohort still has 57 failing historical checks;
their separate repair remains an integration prerequisite.

## Mechanism and retained semantics

The old parser scanned the same fresh text separately for five fields. The
candidate creates an invocation-local index of borrowed field bodies and sticky
duplicate flags. UTF-8 validation still precedes parsing. Actual parsing and
refusals remain ordered Pid, Tgid, Uid, Gid, Groups, including duplicate-before-
token errors within each field, the 33-token cap, widths and credential checks.
Unknown lines, CRLF, invalid numbers and sorted group requirements are unchanged.
No status read, sender observation, authority check or original clock is removed
or cached. `Sender::revalidate`, BOOTTIME/REALTIME order and original 300/60/10
windows are otherwise unchanged. This gate measures CPU, not RSS or retention.

The new differential tests compare with the exact retained baseline function,
including its first refusal site, over 4,096 seeded reordered cases, missing and
duplicate fields, simultaneous errors, invalid UTF-8, token caps and credentials.
The raw-current source canary accepts only the exact isolated edit and pinned
index helper; removed reads, deadlines, checks, field ordering and helper drift
refuse before any historical view is returned.

## Matched CPU control

The dependency-free GNU 1.94 control compiles the exact extracted parser and
group collector, with a three-integer ucred data shim and diagnostic error/trace
plumbing. It reads no proc file and creates no live owner. The five alternating
pairs use the same eight shuffled synthetic documents, seed, 20,000 iterations
per target shape and byte-identical semantic transcripts. Targets can be exceeded
by a final noise line. Workload witnesses, build source and image hashes are
retained; these synthetic documents are not a representative kernel workload.

| Target bytes | Geometric mean CPU C/B | Lower 95 percent | Upper 95 percent | Fixed upper bound |
| ---: | ---: | ---: | ---: | ---: |
| 512 | 0.2524 | 0.2272 | 0.2805 | 0.85 |
| 2,048 | 0.2425 | 0.1849 | 0.3180 | 0.85 |
| 8,192 | 0.2015 | 0.1722 | 0.2358 | 0.85 |

The estimator uses paired log ratios and the preregistered five-pair t interval.
All three bounds pass without changing the method or repeating the cohort.
CPU uses CLOCK_THREAD_CPUTIME_ID; independent wall observations are retained,
including the known WSL clock-rate mismatch. No absolute calibrated time, native
host improvement or product goodput is inferred. The older native profile's
counts/spans must not be multiplied by these leaf ratios to invent total savings.

## Local correctness and first failures

Both compiler lanes pass all-target check, all-target/all-feature strict clippy,
and the same selected release libtests:

- `status_field_scan_`: 2 pass;
- `diagnostic_kernel_admission::`: 124 pass, 23 manual tests ignored;
- `projection_`: 41 pass, 1 manual test ignored;
- `fresh_status_`: 3 pass, 1 manual test ignored;
- `fixed_leaf_path_`: 54 pass, 21 manual tests ignored;
- `authority_`: 222 pass, 21 manual tests ignored.

Groups overlap and are not a unique summed test count. No ignored measurement,
original-owner or manual server test is executed by these lanes. Formatting,
xtask all-target check and strict clippy pass. Two raw-current parser source
controls, five authority source controls and 22 bootstrap/original compatibility
controls pass. The earlier host compile error, Windows colon-in-log-path error,
and stale older parser binding failures are retained. Repairs change the
validation wrapper, never the CPU sample selection or runtime predicates.

The independent baseline source cohort selects 321 tests and fails 68; the first
candidate cohort fails 59. Exact current-file checks plus two missing versioned
readers remove the two new failures, leaving 57 failures shared with the baseline.
The older parser binding also predated the accepted inline group collector; its
assertion is preserved behind exact current-file and predecessor hash checks.
No failure is filtered or reclassified as a successful integrated gate.

Pinned runtime test images are `ae77d1bb99f339fc259e4cc6c9a664edc3e713c7da19c5475f582a16c7d26365`
(GNU 1.94) and `ae33035a42392fe3f7fe7776c399e40a33198587786337643e7c6031a220d225`
(MSRV 1.88). These are libtest images, not signed production bundles. The old
native-attempt image `ca5484aa951414b80aefd0148c7d8f5185a8fdf73bb197cf98a790de59b8cff1`
is still intact. Product code, Cargo.lock, COMPAT and qualification inputs remain.

## Disk space and evidence

Recursive cache removal was refused by the local tool policy. Reversible NTFS
compression of only the checked incremental cache preserved its files and reduced
26,140,371,231 logical bytes to 14,687,591,072 stored bytes: 11,452,780,159 bytes
of storage savings. Concurrent builds affect the volume free-space delta. The
temporary clean baseline worktree was removed after use and is reconstructible
from its retained commit; no preexisting worktree was changed or removed.

The [207-file packet](local-runs/status-field-scan-20261024/index.json) contains
130 objects, exact baseline/candidate inputs and first outputs, all five pairs,
compiler checks and source snapshots, pinned image receipts, source failures,
cleanup receipts and derived facts. Index SHA-256:
`fa6cf67055f7e857ecee141e8ac32972684ca3d8d46558e5352904bac465e0fc`.
The earliest host E0277 output exists in chat only and is labelled as such; no
raw log is invented. The [source guard repair plan](source-guard-repair-plan-20261024.md)
is separate from this accepted leaf and the still-unpassed live holder path.
