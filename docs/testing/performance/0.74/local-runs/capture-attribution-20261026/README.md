# Offline capture attribution evidence

The [immutable index](index.json) has 73 members in 62 addressed objects and
SHA256 `055c7f306ad9dafb55b85a578ec788231a11f504e2117830991cdc0c5436a5b8`.
Analyzer source is `fa9b22330289ad0dacd05faf97310fa0b3b96336`; the fixed
input is the sole native coordinator packet with index
`55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f`.

Six raw source/HEAD-preserving receipts cover fourteen new models, seventeen
unchanged CPU-owner regressions, syntax, check, Clippy and the first offline
analysis. Exact analyzer/dependency/test/design sources, wrappers, exclusive
intent/result and selected native input sources/logs/ledgers are retained. All
selected input bytes are bound to the old immutable index. Retained collector
code is evidence, not an instruction to execute it.

```text
python docs/testing/performance/0.74/local-runs/capture-attribution-20261026/verify.py
python docs/testing/performance/0.74/local-runs/capture-attribution-20261026/test_verify.py -v
```

Independent replay hashes every member and recomputes CPU owners, raw phase
brackets, arithmetic complements, distributions and signed blocks. Five
integrity cases exercise bytes and derived-output mutation without importing a
retained collector or analyzer, reading clocks or invoking processes. No new
workload occurs. The [report](../../capture-attribution-result-20261026.md)
distinguishes observed spans from unmeasured leaf cost, overhead acceptance and
production authority; none of those is promoted by successful offline replay.

The separate [final-checks ledger](final-checks/README.md) retains five passing
receipts, both verifier revisions and unchanged qualification manifest. Index
SHA256: `ae6f7b3dc03c8df53f4f60a669829173a5d0bc4c1806eda4c331c02acfb3cbbb`.
