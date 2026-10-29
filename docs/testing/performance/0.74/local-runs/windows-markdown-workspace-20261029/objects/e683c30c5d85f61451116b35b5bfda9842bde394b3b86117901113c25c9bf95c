# Authority before read CPU attribution

This finite nonproduct comparison isolates the before-read file projection in
the full leaf validator. It keeps parent-device metadata unchanged and preserves
all four live metadata observations, conditional reads, FD lifetimes and original
windows. See the frozen [protocol](protocol.json) and
[result with evidence](../../docs/testing/performance/0.74/authority-before-cost-result-20261025.md).

Eight independent processes per phase compare marker and record payloads with
balanced reference, identical-reference control, actual runtime and prototype
arms. Baseline runtime must calibrate against reference before integration;
integrated runtime must calibrate against prototype afterward. Both payload CPU
upper intervals must pass the frozen 0.98 floor. A libtest success alone is not
that performance gate and never authorizes original start.

```text
python tools/authority-before-cost-074/test_review.py
```

`review.py::review(cohorts, phase)` takes eight untouched stdout byte streams and
`prototype` or `installed`. Missing/reordered work, changed types/flags/clocks,
budgets, payloads and observation counts refuse. It reuses only estimator/parser
helpers from the separately frozen directory reader. Synthetic reader tests are
not measurements. The two ignored Rust selectors and `HC074_BEFORE_COHORT` are
registered as manual, non-ship gates.

Use only an independently approved isolated environment with no overlapping
builds/tests. Keep the exact baseline image for the prototype phase and preserve
first failures. The first integrated full preparation still expires; do not
automatically retry it or turn a local CPU result into a product claim.
