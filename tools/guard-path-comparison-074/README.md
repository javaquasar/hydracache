# Fixed slice guard path comparison

One allocation-only leaf experiment compares the exact existing four formatted
path comparisons with borrowed prefix/suffix equality. It preserves comparison
positions, hook order, assertions and every live validation/window. It does not
reduce Git reads, cache current files, batch checks or change the cache runtime.

`proposal.toml` freezes the scope and baseline source. The raw baseline fixture
is binary-preserved; `contract.rs` permits precisely one predicate substitution.
`probe.rs` is a standalone Rust test image with its own thread-local allocation
counter, never linked into xtask, the supervisor or the cache. All 24 ABBA/BAAB
first samples are retained, with no warmup or retry. It reports gross requested
allocation/reallocation and scope peak/live deltas, not allocator RSS, retained
memory, CPU, full normalizer cost or a package speedup. Counter TLS disables on
unwind and remains separate across threads; System operations forward unchanged.

Build with `rustc +1.94.0 --edition 2021 --test probe.rs -o <owned-executable>`
from this directory, and run ordinary controls without fixture variables.
`capture.py` consumes a successful source-bound build receipt and a new output
directory, verifies the frozen current baseline before any measured child, and
enables only `allocation_child`. A changed installed source refuses a new capture;
the first raw output can still be reviewed offline after integration.

The one-line source change and exact expected-source adapter were initially
integrated and passed focused checks, but complete-source package contracts
subsequently failed. The [compatibility repair](../../docs/testing/performance/0.74/package-compatibility-isolation-result-20261025.md)
withdraws that installation, restores the original guard/gate and retains the
standalone candidate and first allocation evidence. New isolated controls require
the installed runtime gate to refuse this candidate. The component result cannot
close the 780-second package timeout or establish full-path benefit. See the
[result and limitations](../../docs/testing/performance/0.74/guard-path-comparison-result-20261025.md).
