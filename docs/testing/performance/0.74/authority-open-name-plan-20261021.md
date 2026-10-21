# Proposed short file name encoding candidate

This section records the original preregistration; the local outcome follows below.
Baseline code is `b99c1c759630cf632c0a13a905c3ddf9d822def5`, with the
conserved owners and selected leaf CPU packets. The aggregate open window
includes CString allocation/destruction and the syscall; it does not establish
CString CPU cost. Test one allocation hypothesis, not a CPU gain inferred
from those windows.

For `open_at` only when `directory == false` and the UTF-8 byte length is at
most 23, encode the complete name and NUL terminator in a bounded 24-byte
stack buffer. Validate interior NUL on every call before the syscall, with
the same Invalid refusal. Do not cache name validation or any kernel result.
Keep directory=true and longer names on the original CString path, including
their error classes. Empty names still reach the kernel and produce its
original error. Do not truncate, reject new lengths, change pathname semantics,
or optimize directory handling in this candidate.

Preserve the exact parent FD, flags, single fresh openat, error capture and
owned File transfer. All held/before/parent/after metadata calls, regular/dev/
length checks, bounded read_to_end, fresh bytes, stamp comparisons, File
lifetime, validation rounds and 300/60/10 windows remain. Do not change buffer
ownership, inventory, fsync, lifecycle, trust or observation frequency.

First capture the unchanged baseline under frozen new recipes. Differentially
compare the original CString helper and candidate using real private files:
empty names; byte lengths around 23/24; valid multibyte UTF-8; interior NUL
at every position; missing/long names; file/directory mismatches; symlinks;
nonblocking FIFO handling and all original flags/errno. Retain descriptors
through identity/flag comparisons. Extend existing mutation/refusal controls,
check actual/traced request/read equality and preserve sticky refusal.

Primary local effect is fewer Rust requests, not latency. The hypothesis is
full fixture inspection allocations 23/99 to 21/85 and requested bytes
1662/8444 to 1616/8122 at depths one/seven, with zero reallocations and the
same 4/28 reads. Directory/inventory and the other six control cases must
stay equal. These are expected arithmetic targets, not measured results.
Measure cold initialization/open separately from warm revalidation; do not
move work into retained heap state or another API. Record bounded stack use
and logical encoding writes without claiming physical copied bytes or RSS
from source inspection alone.

Use matching baseline/candidate instrumentation, interleaved paired CPU batches,
identical payload/name hashes, epochs, parameters and unmodified estimators.
Keep all raw ranges and signed pair differences; no post-hoc threshold changes,
overhead subtraction or retry into a healthy cohort. Request reduction alone
cannot establish CPU gain or nonregression. Any CPU ambiguity remains explicit;
the native/release qualification gate cannot open from this diagnostic.

Run GNU 1.94/MSRV 1.88 focused/differential tests, affected all-target checks,
strict all-feature clippy, exact source restoration canaries, historical source
contracts and repository gates. Keep instrumentation, candidate and evidence
separate. On failed semantics or unconfirmed primary request effect, retain
the result and revert only this candidate. No install, server/live matrix,
supervisor, clock/service, product workload, qualification or push is authorized
by this plan. Old live/time failures and independent admission gates remain.

## Frozen local comparison method

The new packet `local-runs/authority-open-name-20261021/README.md` specifies
the source-bound alternating helper pairs, separate cold leaf open and warm
revalidation, and unchanged owners/full controls. Baseline runtime remains
the original CString helper. Authoring preflight failures are retained before
the repaired instrumentation gate; they are not measured runtime cohorts.
Helper CPU ranges and signed pairs will not be promoted into an allocation
CPU estimate or a product nonregression claim.

## Local outcome

Instrumentation baseline `275cd253875dc32eda5c6d2ba3c3b1c2290e9da2` precedes
candidate `b074f8f9`. GNU 1.94/MSRV 1.88 reproduce the exact primary request
targets in all three batches, with the same reads, fresh expression counts,
actual/traced equality, payloads and epochs. Directory/inventory and six other
full controls remain equal. Cold leaf open includes the owned name; cold and
warm each remove one short-name request, with no new retained heap field.
The allocation hypothesis is confirmed locally, without a CPU gain claim.

Twenty alternating paired CPU batches per helper case retain both difference
signs and overlapping ranges, including the unchanged baseline. No CPU
nonregression or product/API improvement follows. Logical zeroing/copying of
the 24-byte buffer is not physical-copy, compiled-stack-size or RSS evidence.
The method and all raw observations are retained in the comparison packet.

Both stages pass 213 authority tests per compiler, ancillary controls, affected
checks and strict clippy. Differential file/FD controls and exact source
canaries preserve fallback, NUL, flags, errno, ownership and fresh checks.
The original failed authoring preflights and test-only repair remain, before
any measured baseline; no measured cohort fails or is retried. Existing live
refusals, original windows and independent admission gates remain unchanged.

All 388 historical/source assertions across 23 targets pass, and raw-log
review confirms exact targets while preserving the nineteen earlier packets.
Four reviewer mutations are refused. Next, measure residual complete-round
cost before selecting another runtime hypothesis; do not transfer helper CPU
observations into API or admission claims.

Evidence `de0ed3fee20b110e0c48dae6876e64eca2411527` seals 109 members plus
inventory SHA256
`7d5a88964d1df7498aced1d5ce81fbd8b4ad16fe693645bb5044503717a67ce8`.
Effective review is `final-review.ps1 -Mode Review`; final repository and
read-only environment receipts pass. All 110 Git files preserve raw bytes.
Qualification and independent server admission requirements remain unchanged.
