# Full Windows milestone after the Markdown security update

Result: **refused by the remaining Zstd advisory findings**. Markdown's
RUSTSEC-2026-0414 is absent. No workspace/release/server admission is granted.

The one full `cargo xtask verify` ran on clean feature commit
`c62a2aa38810c96c9e480c6db39a8afbfa41b456`, after the
[focused Markdown qualification](markdown-security-result-20261029.md).
No prior suffix or successful portion of an earlier milestone replaces a stage.

## Actual result and unchanged contract

The original owned Windows Job used 4,800 seconds including startup, a 16-GiB
initial capacity condition, 4-GiB running floor, five-second sampling and
five-second cleanup. Cargo naturally exited 1 on dependency policy. Retirement
completed without errors. The minimum sampled free space was 19,404,816,384 bytes;
neither capacity nor the deadline caused this refusal.

Eleven stages completed: doc-check, management/performance contracts, release
feature leak, formatting, all preceding workspace/allocator Clippy variants and
dependency bans. Dependency policy then refused exactly:

- RUSTSEC-2026-0391, zstd 0.13.3;
- RUSTSEC-2026-0398, zstd-safe 7.2.4;
- RUSTSEC-2026-0403, zstd-safe 7.2.4.

Bans, licenses and sources passed. No deny exception or dependency exclusion was
added. Nine remaining stages, including both workspace test groups, were never
started. The separate 97-case HC/2 results are focused checks, not substitutes
for those unstarted workspace stages.

Advisory DB observations were clean at both ends and both reported
`40f5be6fb12ed93fc591bf4ab23b7d793b3f2239`. These observations do not seal each
internal lookup or turn this policy run into a complete transitive safety audit.
For example, the separately documented old LRU issue in Chitchat's closure is
not excused by its absence from this product policy diagnostic.

## Complete member custody and evidence

The 3,740-file source frame includes all 36 workspace members and 1,962 tracked
member files. The previously omitted fuzz/consumer inputs are now covered.
Source, refs, configuration, actual default toolchain and inherited-environment
frames were byte-identical before/after. Whole-input historical checks retain
their original conditional order, independent pins and all original runtime
checks and deadlines. No live result is cached or omitted.

The packet
[`local-runs/windows-markdown-workspace-20261029/index.json`](local-runs/windows-markdown-workspace-20261029/index.json)
retains exact framed source bytes, raw streams, request/controller/retirement and
ordered 21-stage accounting, advisory observations, fixed producers, and focused
control/source/documentation receipts. Native binaries/build caches are excluded.
It contains 3,821 logical members / 3,746 objects; index SHA-256:
`913735b0aad4b31945f03525ee584d2781e03ee66fa6aa238dcbca61df30a433`.
The original once-per-input execution claim is retained. deny.toml stays at
`16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930`;
the qualification manifest stays at
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
An offline inspection initially used Windows' cp1252 default on UTF-8 Cargo
diagnostics and failed to decode. That inspection refusal is retained; reading
the original bytes as UTF-8 completed the analysis without rerunning Cargo.

## Remaining boundary

Do not rerun this unchanged graph or enable later stages past the failed policy.
The next package is the approved Chitchat/Zstd security-backport boundary:
reconcile security, exact supported MTU admissions/wire behavior, original bounds
and memory cost. The
[preview candidate S](chitchat-preview-cost-result-20261028.md) remains declined
because of its measured allocation amplification. The weaker conservative bound
also lost valid admissions; neither is a release workaround. Preserve both
negative results and the complete public-consumer/distribution requirements.
No new performance hypothesis or package publication is opened by this milestone.

The separately open original delivery/all-writer, durable lifecycle/reconnect,
trusted enrollment, compatible host environment, server rehearsal and final
exact-candidate qualification gates remain required. No existing service,
supervisor, clock, server or product workload changed during this work.
