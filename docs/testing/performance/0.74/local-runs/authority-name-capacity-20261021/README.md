# Authority name capacity local control

This packet accepts only reserving initial String capacity in private journal
name formatting. It is local fixture evidence, not cache API throughput, full
ActiveAuthority cost or server admission.

Before source is `5364b8215c3892f40987d3284dff6be853208e4d`; After and runtime
candidate are `433a7cb1806d7296c8dff72bf83620eb48a91d42`. The TSV lists commits
from baseline `36f9f61f` through the exact After source. Each compiler has one
Before and one After capture; no failed measured invocation is retried.

## Attribution and conservation

The same standard formatter writes 16 padded lowercase hexadecimal digits,
a dot and the unchanged suffix. Reserving 17 plus suffix byte length removes
capacity growth; it does not remove the resulting String. Setup stays outside
capture. Frozen reference, proposed reservation and actual implementation use
32 warmups and three rotated 1024-operation samples on GNU 1.94 and MSRV 1.88.
Every sample and compiler agrees. Actual matches reference Before and proposal
After; frozen reference/proposal request counts are identical across stages.

| Operation | Allocations | Reallocations | Requested bytes |
| --- | ---: | ---: | ---: |
| json name at revisions 1, 7, 128 | 1 to 1 | 2 to 0 | 56 to 21 |
| commit name at revisions 1, 7, 128 | 1 to 1 | 2 to 0 | 56 to 23 |
| 256-byte suffix at revision 128 | 1 to 1 | 3 to 0 | 329 to 273 |
| Complete fixture inspect, 1 generation | 29 to 29 | 8 to 0 | 1942 to 1806 |
| Complete fixture inspect, 7 generations | 129 to 129 | 56 to 0 | 10068 to 9116 |

Rows are per operation, divided from raw 1024-operation totals. Complete
inspect runs inventory twice, generating one json and commit name per revision
in each scan's comparison. It saves exactly twice the pair's reduction times
depth: 8 reallocations/136 requested bytes at one generation, 56/952 at seven.
Read requests remain 4/28 per operation; six other leaf cases and all tracked
kernel counts remain equal. Allocation, proc I/O and kernel captures are
separate. Full inspection is not measured at depth 128 or with long suffixes.

Requested bytes include reallocation request sizes, not actual copied bytes,
retained memory, CPU, latency or all libc/kernel allocations/syscalls. Different
Before/After boot IDs do not establish continuity or repair old clock refusals.

## Tests and immutable inputs

The seeded formatter differential covers all bounded revisions, hexadecimal
boundaries, usize::MAX and seven suffix shapes including empty, long, NUL and
Unicode bytes. It checks the reference, proposal and actual output. Existing
authority tests retain canonical records, journal read-back and fault refusals.
An exact source-input adapter restores the old formatter only for historical
assertions; its own test restores all baseline bytes, including the preceding
inventory test file. No old assertion is edited. Unknown helper bodies/hooks
refuse; outside-helper drift remains visible.

The first new negative canary accidentally targets Entry::open's revision
argument instead of name(). Its documented refusal and exact test-only fix
precede every measurement. Runtime, reference and workload do not change.

Before/After compiler lanes pass 194 authority tests each, with 14 tests ignored
in that suite; the two manual probes run explicitly. Focused kernel/trust/counter
checks, affected all-target checks and strict all-feature clippy pass. All 373
historical/current assertions and repository format/docs/registry/local
non-promotable performance checks pass.

Use `check.ps1`, `profile.ps1`, `final-check.ps1` and `review.ps1`. Review binds
committed input blobs and normalized hashes, image/recipe/check-log hashes,
actual-stage agreement, three repeated samples, revision-invariant costs,
depth conservation, ten scope mutations and eight structural JSON negative
controls. It rehashes twelve prior sealed packets. Seal once, then review only;
never rewrite observations or summary to obtain a passing result.

Changed runtime is `tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs`.
Test-only hooks are in that file and `src/diagnostic_fixed_leaf_alloc_tests.rs`;
controls are `tests/support/authority_name_capacity_{controls,alloc}.rs`.
Exact old/new source templates, guards and two assertions are under
`crates/xtask/tests/authority_name_capacity_074.rs` and support files. Only the
preceding inventory test's read helper is adapted; its assertions stay exact.
The gate registry, attribution contract, plan, article and readiness notes are
updated. No public API or durable/wire byte format changes.

Workload/reference/qualification blobs remain equal. Qualification manifest raw
SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
Every live check and original 300/60/10 window remains. Native clock-order,
original delivery, all-writer cessation, later lifecycle cells, SIGKILL/lost-ACK,
reconnect and trust/enrollment stay independent closed gates. No new live matrix,
server run, installation, supervisor, clock/service or product operation and no
push occurs. A rebuilt tool cannot inherit old campaign admission.
