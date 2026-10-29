# W9b D3a allocation and requested-live-memory screen

**Retired candidate:** the sealed `3171d02a` screen rejected serial scratch on
window live peak. Its product feature/helper/writer are removed. The tool's
`serial-scratch` diagnostic flag now fails before workload, rather than silently
measuring a feature-on no-op. Current default tool builds remain canonical. Use
exact historical source `3171d02a` to inspect/reproduce the original experiment;
the sealed archive already contains all 140 attempts and must not be overwritten.

This standalone tool does not change the product allocator or the historical
pipeline/stage profilers. It is **not** a product CPU, throughput, latency, socket,
RSS-retention or release qualification lane. Still not distributed transactions.

The complete canonical `serve_connection` decoder/dispatch/reducer/encoder/writer
executes against a scripted, exact streaming response oracle. Corpus and oracle
are prepared before counting; neither expected payload allocation nor an output
collection occurs inside the window. One current-thread runtime has no other
tasks. IO is immediately ready and read boundaries are explicit. Warm-up and
post-window final-value/cardinality validation are separate. It counts successful
allocation requests (full successful realloc size), allocation calls, outstanding
requested layouts, window peak above the starting live layouts, and live layouts
at the next-read boundary. This is not allocator metadata, active/resident bytes,
hidden System realloc overlap or physical memory. Close/Rust drop does not prove
return of allocator pages to the OS. RSS endpoint/lifetime-peak snapshots are
supplemental only. No instrumentation timing is reported as product evidence.

Both original binaries were built from the **same clean source**, with feature off/on. The
off default product path corresponds to the `4d733e31` legacy baseline; it is not
an old baseline binary relabeled with the new harness. Product implementation is
`b8cc7c6c`; the source-bound instrumentation commit is recorded by the build script
and checked against the runtime Git SHA and clean status. The wrapper feature is
the only candidate switch. The runner seals both hashes, toolchain and tool lock
before any sample. Outputs are create-new and failures are retained without retry.

Policy: `docs/testing/performance/0.74/w9b-serial-scratch-d3-contract.toml`.
First run tests for both variants, check and strict lint; commit the instrumentation
and preregistration before building numerical binaries. Tests must be serialized
because outstanding-layout assertions use the process-wide allocator:

```powershell
cargo test --manifest-path tools/resp-scratch-screen-074/Cargo.toml --locked -- --test-threads=1
cargo test --manifest-path tools/resp-scratch-screen-074/Cargo.toml --features serial-scratch --locked -- --test-threads=1
```

The runner refuses dirty source, pre-existing outputs, different source/lock/binary
identity, workload drift, nonfinite/missing data, incomplete response accounting,
partial pair sets and changed attempt order. Phase A has five independent AA and
five counterbalanced AB pairs for every registered cell. It can **reject** a
candidate; it cannot accept one. If any valid memory/allocation guard is red,
retain the whole sealed A set and remove only this candidate. Real transport,
unprofiled timing, native controls, security/concurrency and idle/allocator memory
remain explicitly pending if A passes. No rented host or qualification is used.
