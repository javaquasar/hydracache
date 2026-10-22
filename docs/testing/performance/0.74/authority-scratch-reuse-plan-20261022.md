# Owner-bound authority scratch experiment

Starting tree: `84b5e62c`. One storage hypothesis, in a separate synthetic Windows
test image only: reuse a temporary Vec for fresh reads of the same borrowed Leaf.
Never cache an observation or replace the independently owned expected bytes.
Runtime, registry latching, live observation count/order, original 300/60/10
windows, first Causal(Time) refusals and qualification remain unchanged. No WSL,
server, service/clock changes, original-owner/live/fault/product runs or push.

First commit and measure a reference-only full-revalidate preflight, before the
candidate exists. Copy the frozen read baseline receiver and complete original
model bodies; reuse its nine controls unchanged. Use the same isolated allocator.
Two fixed cost cases are marker65/budget65/chunk65 and record2048/budget2048/
chunk2048, seed 0x074a17b3. Each interval creates an owner outside measurement,
warms 64 complete revalidations and times 512, including rewind and black_box.
Seven intervals per case. Separately capture 512 reference revalidations after
the same warmup. Allocation capture is inactive during timing. Keep these first
logs and immutable source commit; no reference preflight retry-to-pass.

Then implement only an owner-bound scratch borrowing that exact Leaf. Keep the
complete original open/metadata/regular/parent/budget/read/after/stamp/bytes
checks and generic Read::take/read_to_end expressions. Reuse only when capacity
equals freshly observed metadata length; otherwise discard and construct the
original exact-capacity Vec. Clear length before every fresh read. Hold the fresh
File through comparisons. Retain successful capacity only when still equal to
the stamp length and within budget; release all cached storage on any error.
No rebinding, mutable buffer exposure, authority latch, retry or cross-owner pool.

Compare the frozen nine cases over 256 full revalidations per arm, with cold
scratch and no hidden warmup. Capture scratch drop separately, not pooled with
main calls or failures. Compare exact outcomes and complete traces for every
call. Additional paired controls: all 64 warm-cache revalidation fault masks;
28 metadata predicate/stamp/device cases; 256 seeded content changes; and 1512
IO schedules (declared lengths 0/1/9/10/11/65/66, bodies 0/9/10/11/65/67,
chunks 1/7/128, error calls 0/1/2/3/4/12, Interrupted calls 0/1), original held
length 10, budget 65. Preserve first-error reachability and read request geometry.
Check cache eviction on refusal, owner pointer identity, wrapper size/alignment,
independent original bytes and separate cached-buffer drop. These controls are
not OS descriptor, admission or all-writer termination proofs.

Use the frozen timing helper for seven paired intervals per cost case, alternating
arm order. Each arm gets a new owner and the same 64 warm/512 measured calls.
Keep every first duration/checksum; no outlier removal or rerun. Reject if any
semantic/trace/bound/lifetime guard differs or warm exact-capacity reads still
allocate. Reject adoption if candidate median exceeds reference on both compilers
in either cost case. Otherwise the result remains a non-promotable prototype,
pending GNU/full-path and retained-memory/copy evidence. Allocation reduction
alone is insufficient: the extra retained capacity is a cost, not free memory.
Capacity/layout and logical delivered bytes are not RSS or physical copy metrics.

Run first Windows MSVC 1.94/1.88 lanes, format/check/strict clippy, source canaries,
offline mutation controls, documentation/registry/local performance contracts.
Verify all eighteen previous packet seals and 1000 members. Seal raw first logs,
reference and candidate source snapshots, independently reconstructed facts and
local checks. Any failure and any source repair remain separately attributable.
