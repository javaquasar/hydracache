# Boot and Namespace operation attribution

This finite nonproduct screen measures selected operations within the actual
Boot and four Namespace readers. It adds only test-compiled hooks, retains all
original observations/order/FD lifetimes and never authorizes a preparation
retry or start. See the frozen [protocol](protocol.json) and
[result with evidence](../../docs/testing/performance/0.74/kernel-operation-cost-result-20261025.md).

Eight independent processes compare frozen reference, identical-reference
control, inactive runtime and runtime with one selected observer. Both calibration
intervals must fit [0.97,1.03] for every path/stage before interpreting attribution.
The empty-span control remains in the result; no overhead is subtracted, and
selected spans are not additive or exact production-operation costs.

```text
python tools/kernel-operation-cost-074/test_review.py
```

`review.py::review(cohorts)` consumes eight untouched stdout byte streams. It
rejects changed work, selected-operation counts, order, types, flags, clocks or
verdicts. Synthetic tests are not measurements. The single ignored Rust selector
and `HC074_KERNEL_OPERATION_COHORT` are a manual non-ship gate, requiring its
exact pinned image and an independently approved isolated environment without
overlapping builds/tests.

The first cohort passes calibration, not an optimization gate. Open is the
largest measured span, while decode/path are near observer cost. A future
replacement needs a separate matched whole-call protocol and fault checks;
do not infer a cache API gain or retry the retained failed full preparation.
