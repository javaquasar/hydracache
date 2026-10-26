# Native leaf control final checks

The [37-member index](index.json) has SHA256
`7ec1c4af34c9bfe99890d6febbe7d54ad9e2ec704bedd5c44f35cd40b19d8cab`.
Eight source/HEAD-preserving receipts pass: first/final six-case independent
integrity suites, all 74 model/caller/accounting regressions, syntax/indentation,
xtask all-target check, strict all-target/all-feature Clippy, doc-check and the
local non-promotable performance contract. The first integrity source is kept;
the final suite additionally exercises shifted-but-arithmetically-consistent
leaf clocks outside their parent and loss of the explicit leaf intent.

Final verifier sources, qualification manifest and sealing source are retained
without rewriting the first native packet. These checks launch no second
native attempt, Git workload or V6. Independent replay only reads retained bytes:

```text
python docs/testing/performance/0.74/local-runs/capture-leaves-native-20261026/final-checks/verify.py
```

Passing local gates do not certify tracing overhead, full-package coverage,
product speedup or production/server admission.
