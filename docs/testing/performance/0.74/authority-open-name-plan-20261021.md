# Proposed short file name encoding candidate

This is a preregistration, not an implemented or accepted runtime change.
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
