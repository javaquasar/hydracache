# Private GET ownership D3a early screen

One isolated, default-off hypothesis. This standalone workspace links the product
with feature off/on; it does not alter the product allocator or old profilers.
The tool-only System requested-layout allocator and exact-source build receipt
are reused by source reference from the retired scratch screen, which remains
unmodified. Its rejected flag is not used or reactivated.

Policy: `docs/testing/performance/0.74/get-response-owner-d3-contract.toml`.
Nine cells, five fresh-process AA pairs then five counterbalanced AB pairs each,
180 attempts total. AA executes the same off binary for both named roles. Corpus
and byte oracle are prepared outside counting; the complete `serve_connection`
decode/dispatch/reduce/encode/write/flush/close path runs inside. Preload, warmup
and final values/cardinality validation are outside. Empty hits and misses are
exact controls. Input/output bytes, seed, counts and compiled variant must match.

Tests must run serialized because the allocator is process-global:

```text
cargo test --manifest-path tools/resp-get-owner-screen-074/Cargo.toml --locked -- --test-threads=1
cargo test --manifest-path tools/resp-get-owner-screen-074/Cargo.toml --features get-owner --locked -- --test-threads=1
python -m unittest scripts/perf/test_performance_get_owner_screen_074.py
```

Commit contract/instrumentation after tests/check/strict lint, then build release
binaries into separate target directories from that same clean SHA. Runner
seals source, contract, lock, compiler and both binary hashes before any sample.
Never overwrite a packet, retry an invalid matrix or select pairs. Valid red
completes the sealed matrix, retains negative results and rejects this candidate.

This is scripted plaintext RESP2 IO, not socket or syscall qualification. Gross
counts and outstanding requested-layout peak omit allocator metadata and hidden
realloc overlap. Next-read owner checkpoints and close are not timed idle/RSS
retention proof; RSS endpoints/lifetime peaks are supplemental. Tool allocator
overhead cannot certify CPU/op, p99 or goodput. Passing A cannot accept/default-
activate a proposal or resolve C74; native/unprofiled timing/real secure transport,
concurrency/retention/refill/hosted CI still require independent phase B evidence.
Still not distributed transactions. No rented host or expensive campaign.
