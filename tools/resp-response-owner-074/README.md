# RESP response ownership attribution, 0.74

Standalone **D1 instrumentation**, not a product candidate. Preregistration is
`docs/testing/performance/0.74/response-reduction-attribution-contract.toml`.
It uses public translation-plan and verified-dispatch seams; no product source,
experimental feature, response scratch, output batching or store shortcut.

Each fresh process uses a fixed binary key and seeded payload, a deterministic
cache clock, independently initialized direct/combined client surfaces and 100
warmup operations. The measured plan is preconstructed: dispatch windows include
request-envelope clone, response-vector collection and followup validation just
as canonical `execute_plan` does. They exclude command decode/translation,
server request-id generation, encoding, transport and connection scheduling.
The isolated borrowed reducer keeps its prebuilt original response alive outside
the allocation window. Nonempty GET values must have equal bytes and distinct
backing pointers while both owners exist. Empty hits, misses and SET are controls.
Dispatch, mutation, final cardinality and final value must reconcile exactly.

The audited System requested-layout counter is reused from the independent
scratch-screen tool without importing that workload or its retired feature.
Gross bytes, successful allocation/reallocation calls, active requested-live peak
above each window's starting owners, response-owner checkpoints and post-window
release are recorded. Realloc counts the entire successful new requested layout;
hidden backing-allocator old/new overlap and metadata are excluded. Stage peaks
are not additive. This tool does **not** measure RSS, allocator retention,
CPU/op, latency, goodput, write syscalls or native numerical nonregression.

Run tests serialized because the allocator is process-wide:

```powershell
cargo test --manifest-path tools/resp-response-owner-074/Cargo.toml --locked -- --test-threads=1
cargo check --manifest-path tools/resp-response-owner-074/Cargo.toml --all-targets --locked
cargo clippy --manifest-path tools/resp-response-owner-074/Cargo.toml --all-targets --all-features --locked -- -D warnings
python -m unittest scripts/perf/test_performance_response_owner_074.py
```

Commit instrumentation first. Build a release binary from that clean source,
then supply its **full actual SHA** to the runner (replace the placeholders):

```powershell
cargo build --release --manifest-path tools/resp-response-owner-074/Cargo.toml --locked
python scripts/perf/performance_response_owner_074.py --root . --binary tools/resp-response-owner-074/target/release/resp-response-owner-074.exe --source <full-sha> --output target/performance-evidence/0.74/response-owner-<sha-prefix>
python scripts/perf/performance_response_owner_074.py --replay <retained-directory>
```

The source, compile-time Git identity, binary, tool lock, contract and corpus
receipts must match. Output is create-new only. Three rotating repeats cover six
cells (18 attempts); every raw receipt and attempt outcome remains retained.
A failed attempt stops the campaign with no automatic retry. Replaying an
incomplete, reordered, duplicated, modified, unreconciled or mixed packet fails.
Attribution may motivate a **new** D2 contract; it cannot reopen earlier terminal
rejections or certify a future consuming reducer's gain or semantics.
