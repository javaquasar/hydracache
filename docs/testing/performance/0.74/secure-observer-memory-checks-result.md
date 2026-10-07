# Secure observer and local memory diagnostics

Preregistration: `f22c3efff939711d168b2717f4fc667853a15f16`. Instrumentation:
`263d50758e82e1f7f317450cbbdfb7ab742f980b`. Exact measured and rechecked source:
`d22520123880a8a67c816c83a7f76448cbf50af9`. This is a feature-off diagnostic
packet, not a product A/B candidate, native nonregression result or B0 retry.

## Verification

The exact source passed 68 serialized observer checks in each default/get-owner
variant (31 unit, four allocation/argument/RSS unit, eight native integration,
nine plaintext RESP integration, 12 scheduler and four secure integration tests).
Check and strict all-feature clippy passed. The secure grid contains 112 cells,
plus separate large payload/batch boundaries, shared-PKI/dataset guards and
public native/RESP delete/refill checks. AUTH failure is redacted with no
HELLO/preload fallback, and the five-second deadline is tested with virtual time.
These are semantic counts, not performance estimates.

The first dirty-source secure attempt passed three tests but failed the large
payload preload. An explicit buffered-transport regression isolated the missing
final request flush in the tool actor. After adding owned flush state, the same
case passed with unchanged deadlines/limits. The retained failure is not a
recovered passing numerical attempt. Initial source was uncommitted; no exact
dirty-source blob identity is claimed for that attempt. Earlier disk-exhaustion
link failures executed no secure runtime tests and are not throughput evidence.

Hosted semantic CI for `d2252012`, run `37697414775` attempt 1, completed
successfully: format/check/strict clippy and 68 serialized tests per feature
variant on Ubuntu 24.04/Rust 1.94.0. Its identity, raw logs, toolchain and hashes
are retained in [the hosted receipt](local-runs/secure-memory-d2252012/hosted-semantics-receipt.json).
It is not full-workspace release CI, native numerical proof or qualification.

## Finite memory packet

The [seal](local-runs/secure-memory-d2252012/seal.json),
[summary](local-runs/secure-memory-d2252012/summary.json) and
[analysis](local-runs/secure-memory-d2252012/analysis.json) retain 30 fresh
processes: five surfaces, 256/65,536-byte payloads, three repeats. Every process
completed all seven phases with exit zero. All 30 stdout hashes were rechecked.
Raw stdout/stderr and pinned semantic-check logs are preserved byte-for-byte.
Source was clean before sealing; tracked source remained unchanged throughout.
Raw files produced by the runner were expected untracked output, not source edits.

All delete phases report zero logical entries/value bytes. All refill phases
report 16 entries and exactly 16 times the payload bytes. Explicit shutdown
completed everywhere. This proves the checked logical/transport ownership
oracles, not return of allocator or OS memory to zero.

As one independent cell (not a cross-surface ranking), RESP3/mTLS with 64 KiB
payload records 50,199,184 gross requested bytes and 7,573 successful allocation
calls over 128 GET workload calls in each of three repeats. Its idle working set
ranges 21,544,960–21,962,752 bytes. Post-shutdown outstanding requested layouts
are 69,665 bytes in all repeats: the still-live tool/runtime/report/global owners
are inside the process scope; this is not a zero-retention proof. The packet's
min/max values are diagnostic statistics, not paired A/B confidence bounds.

One Windows RSS observation varies substantially: direct/256 idle working set
ranges 8,884,224–24,371,200 bytes despite identical GET requested-layout totals.
No causal explanation or regression is inferred from that OS-level variation.
RSS is not allocator resident memory, and requested layouts exclude hidden
allocator metadata/native allocator allocations. Allocation epochs cannot
certify CPU/op, goodput or latency. GET/SET contain 128 workload calls;
preload/delete/refill use 16 keys plus transport-specific verification reads.

Allocator active/resident/retained remains unavailable and the retention
admission flag is false. No counting allocator is linked into the timing
library. B73, old D3a/B0 packets, root lock, qualification manifest and default-off
get-owner policy are unchanged. Full D3, separate native numerical floors and
W10 remain open; see [readiness audit](full-d3-readiness-20261008.md).
