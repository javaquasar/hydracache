# Fresh boot equality allocation evidence

Inherits `docs/RULES.md`. Local WSL Ubuntu only, with synthetic issuers, copied
signed bundle data and harmless dummy writers. No server installation,
supervisor operation, original send, product workload or release qualification.
The [plan](../../boot-comparison-plan-20261019.md) fixes the hypothesis and
cohorts; the [design](../../boot-comparison-design.md) describes the ownership
boundary. The earlier closing attribution and partial MSRV packet stay sealed.

## Sources and unchanged observations

Baseline is `aa84aeb8`; `d54c25261c9bcc9c4035d7ea951ba984e4224ebe` adds only the
frozen allocation reference and counter test. The initial implementation
`35c11fce` moved the strict decoder into a private helper. An existing mutation
guard correctly requires its actual predicates in the original module. The
correction `0da439d2` retains the decoder there without changing that guard or
adding duplicate validation. The selected source and recipes are
`22b7a297a49665d5b7f85b0253dd982c2ab9f5c7`.

The preliminary full source-test executable was cancelled after the guard
failure; its raw log and cancellation note are retained. While that source was
being corrected, preliminary compiler cohorts completed units but their
source-stability checks refused to issue receipts. They are not selected builds.
Only `gnu194-final/` and `gnu188-final/` receipts identify ordinary final images.
Development corrections are not repeated functional attempts until green.

An exact reversible adapter reconstructs all 420 previous source bodies and
the unchanged owning external harness. The strict decoder, new helper and
frozen reference have separate mutation/differential guards. All fresh opens,
flags, filesystem/object checks, one positional read, close-before-decode,
UUID/UTF-8 refusal sites and original Boot/Document/Namespace counts remain.
The retained boot ID and returned time-point API still own their values. All
46 effect brackets and original 300/60/10 origins and bounds remain unchanged.

## Repeated allocation requests

The existing opt-in allocator counts only Rust requests on the calling thread.
It does not measure libc malloc, RSS, CPU or product throughput. Each measured
cohort contains three paired samples of 1,024 complete calls per operation.
The same frozen live reference is used in baseline and candidate images.
Only formatting of the out-of-capture report differs from the baseline counter
test; both frozen reference function bodies are pinned byte-for-byte.

| Operation | Baseline allocations per call | Candidate allocations per call | Baseline requested bytes | Candidate requested bytes |
| --- | ---: | ---: | ---: | ---: |
| Origin revalidation | 2 | 0 | 72 | 0 |
| Origin sampling | 5 | 1 | 180 | 36 |

All three baseline GNU pairs agree before implementation. All three candidate
pairs on GNU 1.94 and all three on MSRV 1.88 reproduce these counts, with zero
reallocations. Per 1,024 calls the revalidation pair changes 2,048/73,728 requests
and bytes to zero; sampling changes 5,120/184,320 to 1,024/36,864. The remaining
sampling allocation owns its returned boot ID. This confirms a narrower
allocation reduction across fresh validation used in preparation/enrollment,
not an elapsed preparation improvement or recovery caused by this change.

`allocations.ps1` pins source, compiler/image and recipe digests. The ordinary
functional images do not enable this feature. GNU ordinary image SHA256 is
`0a70454a1bbf3b94257b5c9ba881794b29f8c6347622bfecffbd55863089698c`;
MSRV ordinary image SHA256 is
`155de08349a7e7d8b28a95cc71ba893544e73707d1b7d13d4c978214b67c6ecb`.

## Separate MSRV fault cohort

After all compilation and source checks, `run.ps1 -Stage remaining` uses the
ordinary MSRV image in facts-only mode, never the allocation-counter image.
The fixed order is second-dead, first-stopped, second-stopped, outside-alias,
owner, first-dead. Each is a fresh original context. A pre-fault time refusal
stops the cohort, records `outcome.json` with unexecuted cases, and cannot produce
a positive receipt. No uncompleted attempt is reused or deadline refreshed.

The unchanged external harness owns exact pidfds and holds dummy stdin ends
outside the creator. Only actual creator SIGKILL, three writer SIGKILL waits
before original expiry, drained stdout/stderr and a still-busy fence count as
success. The outside alias must remain able to write and block EOF until closed;
it is not arbitrary external-FD revocation. The local clock-service stop/restore
and settle/probe are unchanged; clocks are never set manually.

All six cases pass once on the selected MSRV image; no case is retried. Each
captures fifteen original custody age calls, no refusal, and the same source
issuance bound to its ready capsule. Last observed custody ages are:

| Case | Last original custody age ns | Actual creator and three writer SIGKILL |
| --- | ---: | --- |
| second-dead | 8,368,297,173 | passed |
| first-stopped | 8,272,658,234 | passed |
| second-stopped | 8,496,463,675 | passed |
| outside-alias | 8,471,486,709 | passed |
| owner | 8,730,555,996 | passed |
| first-dead | 8,764,277,764 | passed |

The local clock probe records zero backward samples; clock restoration is
verified active, all fresh fixture cgroups are gone, and signed fixture bytes
are unchanged. `msrv-gate/receipt.json` is a positive receipt for this one complete
fresh cohort. It does not repair the older partial receipt, establish a new GNU
fault cohort or attribute this success to the allocation change. Functional
success and allocation reduction are independently supported, not a paired
elapsed-cost experiment. Server/production admission remains closed.

Both ordinary compiler builds pass strict lint, 22 origin, two ownership, four
closing and nine guardian regressions. All 306 source contracts, formatting,
Windows 54 unit/18 doc checks, registry and local non-promotable contracts pass.
The initial red test compile is retained separately from these passing checks.

## Reproduction and integrity

Use fresh labels with `build.ps1` and `allocations.ps1`; never overwrite a cohort.
Then run the separate facts gate and inspect it using `analyze.ps1`. The packet
verifier checks raw hashes, source/image/recipe identity, exact allocator values,
original bounds, fault order/results, local clock restoration, unchanged signed
fixture data and frozen qualification manifest. Sixteen mutation controls reject
promotion of partial MSRV results, deadline or scope drift and fabricated speedup.
Production trust/enrollment, original delivery, lifecycle completion, unlock,
reconnect and server rehearsal remain distinct gates.

The initial sealing attempt refused a missing cleanup log: PowerShell Tee does
not create its file for an empty `find` result. A new read-only cleanup check
records the exact command, exit code and empty roots array in
`fixture-cleanup.json`; the verifier requires that structured receipt. No fault
case is repeated and no empty artifact is fabricated to stand in for the check.
