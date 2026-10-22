# Offline authority CPU variability analysis

Read the [analysis plan](../../docs/testing/performance/0.74/authority-runtime-variance-analysis-plan-20261022.md).
This tool only reads the sealed CPU cohort. It has no native image, WSL, server,
clock or service execution route. Imported historical modules define the frozen
reader; no runner function is invoked. Historical reader content and the input
inventory are pinned. The source guard accepts only exact archived bytes or
their complete LF-to-CRLF Git checkout representation, never general whitespace
normalization. Qualification and every original primary fact stay exact.

From the repository root:

```text
python -B tools/authority-runtime-variance-074/test_analysis.py
python -B tools/authority-runtime-variance-074/analyze.py --publish <new-packet> --checks <offline-check-logs>
python -B tools/authority-runtime-variance-074/analyze.py --verify <existing-packet>
```

Publication refuses an existing directory and seals derived facts, source, plan,
future methodology and optional check logs. Verification recomputes all derived
facts from the immutable input without executing a measurement. Old packets are
never rewritten. Keep the pinned analyzer version for historical verification.

MAD is median(abs(x - median(x))) without scaling. Variance/covariance use the
sample denominator n-1. Spearman is Pearson correlation of average ranks for
ties; constant vectors return null. All costs are original batch counters in
nanoseconds, except explicitly dimensionless shares/ratios/correlations. Signed
elapsed-minus-CPU values are not clamped or called scheduler waiting. A covariance
share can be negative and is not a percentage of causally explained cost.

All seven pairs remain in inference. Order/ordinal strata are post hoc diagnostic
associations, not new acceptance rules. The interrupted phased sum is not the
separately measured uninterrupted cycle. The [future method](../../docs/testing/performance/0.74/authority-runtime-variance-method-20261022.md)
is preparation only and has no runner here.
