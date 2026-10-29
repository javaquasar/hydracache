# Chitchat MTU repair: budget guards and in-place prediction

This continues the existing security/backport work on source HEAD
`09a5f4f458112b209940adbdd205c0384decb5af`. It does not adopt a product dependency,
close Root's Zstd findings, or admit a release/server run. See the
[predeclared sequence](chitchat-inplace-plan-20261029.md) and
[fixed method](../../../../tools/chitchat-inplace-074/README.md).

## Matched inputs and decision

All variants restore the sealed S source and use its identical dependency lock
`89e7e80fceb5562ae4ffaf78093073770e1864ff31d7b443afcb9b9a4926d674`,
including Zstd 0.14.3, zstd-safe 8.2.0 and LRU 0.18.2. B is an attribution-only
counterfactual using the known-wrong original estimator. It is never a product
candidate. The old M decline (nine lost valid admissions) and S decline remain.

The original 239-operation workload, seed `0x0741028`, payloads, ordering, MTUs,
admission decisions and exact serialized bytes are checked independently against
the original packet. There are 136 accepted and 103 rejected operations. Each
new allocation cohort uses four **release-profile** native processes in the
order B/candidate/candidate/B, two observations per case inside each process.
Both repetitions within an image and both same-image processes must match exactly.
The old debug allocation measurements are not mixed into these new cohorts.

| Variant | Production change | Cases adding Rust allocation cost vs B | Decision |
| --- | --- | ---: | --- |
| S | Clone writer, append and finish to predict exact size | 71: 23 accepted, 48 rejected | Declined; baseline preserved |
| D | Reject on old estimate; accept on safe all-block upper bound; otherwise unchanged S | 9, all accepted | Functional pass; allocation gate still refuses |
| P | Replace only D's ambiguous branch by counting through existing buffers | 0 | Selected Rust allocation subgate passes |

For P, **all 239 cases match B exactly** in Rust allocation count, reallocation
count, total requested bytes and peak live requested bytes for the whole
operation. Live requested bytes return to zero after each measured operation.
No averaging hides a regression. D and P retain all 136 admissions and all wire
bytes, including the nine cases lost by M.

This is a measured Rust allocator result for this corpus, not a CPU, throughput,
native C allocator, RSS or long-lived retained-memory claim. Those gates remain
open. Test instrumentation uses allocation-free thread-local counters and the
same observer in every variant.

## Attribution and implementation

D first checks the old estimate. S's estimate cannot be smaller, so an already
over-budget operation can be refused without serialization or capacity growth.
When a conservative bound counting every block header already fits, admission
needs no exact compression preview. Only the ambiguous interval invokes S.
This removes all allocation regressions on rejected operations and leaves nine
accepted cases. D's original semantics and strict lint gate passed before P was
implemented.

P serializes only that ambiguous item into the existing pending buffer, counts
compressed/raw chunks using the existing scratch, and preserves the old estimate
as a minimum. It uses the same chunk boundaries, compression level, destination
size, raw fallback, three-byte block header and end tag as the actual writer.
It neither clones existing output nor creates shared scratch, a lock or a pool.
Only this internal admission helper becomes mutable; public traits stay exact,
including `Send + Sync`.

Two RAII guards restore pending and scratch lengths on return or unwind. Pending
serializers are the existing internal append-only implementations. Scratch bytes
are disposable; its original length is restored even after a shorter final block.
Original output and pending prefix remain intact. The old immutable S predictor
stays explicitly **test-only**, as an independent reference. Wide tests calling
that reference are not presented as production P execution: added controls invoke
the actual helper, and the 239-case cohort measures actual full DeltaSerializer
operations.

Actual-path counters record 103 old-budget rejections and 366 conservative
acceptances across the two original calls per case (node plus key/value). D has
nine remaining S predictor invocations; P has nine in-place predictor invocations
and **zero** production S predictor invocations.

There is a concrete retained-capacity limitation. A deliberately ambiguous
rejected item in the extra stateful control grows pending capacity from **64 to
4,099 bytes**, while logical length returns to three and subsequent output stays
exact. This is retained until the writer is dropped. Old-budget rejections never
serialize or grow capacity. Zero whole-operation live bytes after drop must not
be described as zero retained capacity for a still-live writer.

## Functional gates and first refusals

Before P, D passes 70 focused cases, check and strict all-target/all-feature
Clippy on Windows Rust 1.94. P passes 70 cases and check on Windows Rust 1.94 and
MSRV 1.88, plus strict Clippy on 1.94. These cover original serialize/delta/digest/
message/state groups, the original wide/MTU/oversize matrices, six new production
helper controls and the unchanged allocator control.

First strict MSRV Clippy on P refuses seven inherited format-string findings in
library/performance tests. A separate test-only successor L fixes these and adds
three stateful controls: populated scratch/unwind, mixed compressed/raw/Unicode
streams and ambiguous rejection. L's next MSRV Clippy exposes eight additional
format findings in the integration test target. The separate L2 successor fixes
those too. All 15 fixes affect formatting of test messages/generated test names,
not predicates, values or thresholds. No lint exception is added.

L2 passes **73 focused cases**, check and strict all-target/all-feature Clippy on
both Windows compilers. The complete Windows catalog has 119 cases; 46 are
explicitly outside these focused executions. Async gossip/socket/performance
integration execution and full Chitchat/product gates are not claimed. P's
allocation results are bound to P's release image, not relabeled as an L2 image.
The P production predictor and admission bodies stay byte-exact in L/L2.

The first D formatter invocation returns sandbox `Access denied`; separate
access-authorized formatting uses the exact preserved unformatted input. The
first ten inert producer tests expose a shared-object error in the *synthetic*
cohort fixture (`deepcopy([value] * 4)` preserves aliases). This does not occur
in independently JSON-decoded native observations. The corrected fixture passes
all ten controls. The first inert failure is retained as a summary with its
producer version and tool chunk identity: raw stream files had not been opened
for that direct invocation. Native measurement streams are retained separately.

GNU first exposes a separate owned-observer portability defect: Instant is used
only by the Windows CPU test but imported unconditionally. The separate L3 overlay
adds only `#[cfg(windows)]` to that import; the old measured overlay is untouched.
Thirteen final inert/source-equivalence controls pass, including exact comparison
of P/L3 production predictor, admission, append/flush/finish, manifest and lock.

Final L3 passes **73 focused cases, check and strict all-target/all-feature
Clippy on all four peers: Windows/GNU × Rust 1.94/1.88**. The actual GNU catalog
has 118 cases (the Windows-only CPU test is absent), with 45 cases outside the
focused selection; Windows has 119 with 46 outside. These are 292 final selected
test executions, without ignored cases. No full library/integration or product
gate is inferred.

The first GNU profile through `/mnt/c` retains an outer 1,200-second timeout,
60 passing selected cases with owned receipts and no inner completion receipt. All six executed
test-command receipts report retirement without errors; the next partition has
empty stdout and no execution receipt, so no completion is credited to it. Later
groups/check/Clippy did not execute. The outer Windows Job reports timeout/retired, which is
not substituted for a missing inner complete result. The first source-copy
adapter refuses the absent result before running tests; a separate successor
recognizes the preserved outer-only refusal.

The successor observes no old diagnostic processes, verifies a source capsule
byte for byte on native storage and retains the same guard predicates, count,
order and original 1,100/1,200 bounds. A single declared private cache-root
constant adjustment is reversed and hash-checked against the capsule; build
targets are separate from the first image. Actual Cargo registry sources remain
checked separately. Root Windows source/HEAD are checked before/after the parent
phase; inner fresh source checks address the copied inputs actually consumed.
The first source-copy L2 run passes all 73 cases/check before refusing the owned
Instant import. The separate L3 copy passes both GNU compilers. These are distinct
source contexts, not a passing replay of the initial profile. Read-only process
availability observations are not production all-writer authority or proof.

Only completed Windows debug caches below this experiment's own `targets/`
were removed, freeing 3,935,706,532 bytes. Source, evidence and release B/S/D/P
images were preserved; final L3 images were built after that cleanup. No existing
service or unrelated process was stopped.

## Source/effect boundary and remaining work

These candidates alter private fixture serialization/admission only. They add no
authorization, tenant, consistency, durability, expiration or backpressure change,
filesystem/network effect in the production helper, global cache or public API.
The independent fresh source/config/registry/lock/image checks remain in the
runner. Original product checks/frequency/order and 300/60/10, tests 5/10/45,
package 780, workspace 4800, local owned 1200/session 1100, capacity four GiB,
five-second polling/retirement remain unchanged. No service, clock, supervisor,
server, installation or product workload is changed.

Root lock remains
`34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c`;
deny remains
`16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930`;
qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The three Zstd findings stay open until compatible maintained source distribution,
public consumers, real Root adoption and full policy/package/workspace gates pass.
Native C allocations, CPU and retained-memory behavior still need qualification.
Original delivery, all-writer retirement, lifecycle/reconnect, enrollment and the
compatible-host rehearsal remain independent blockers.

Sources, official archives, all frozen variants/producer versions, original
refusals, raw commands/catalogs/case accounting, native-copy provenance and
retirement receipts are retained in
[`local-runs/chitchat-inplace-20261029/index.json`](local-runs/chitchat-inplace-20261029/index.json):
3,476 logical members / 1,130 objects, index SHA-256
`9381608b814a7fcdc16835ece436534fc1d48f816039023f9c0adbc16a234fd1`.
Compiled binaries/build/vendor caches and redundant ZIP containers are excluded;
ZIP member bytes are retained separately. Final current-source check receipts
are retained in the adjacent verification directory.
