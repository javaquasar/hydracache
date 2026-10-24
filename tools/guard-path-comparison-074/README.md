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

The one-line source change is now integrated with an exact expected-source delta
under the existing whole-Rust-token gate. Comments/formatting remain permitted;
hook/assertion or other executable drift refuses. Mutation/property, existing
432-source preservation and current/MSRV gates pass. The component result cannot
close the 780-second full-package timeout, and the preceding default-Windows
effect-review digest correctly refuses changed source. See the
[result and limitations](../../docs/testing/performance/0.74/guard-path-comparison-result-20261025.md).
