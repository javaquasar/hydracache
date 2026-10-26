# Native control final checks

Seven source/HEAD-preserving checks pass: six packet-integrity controls, verifier
syntax/indentation, xtask all-target check, strict all-target/all-feature Clippy,
doc-check, local non-promotable performance contract and 56 original model/caller/
accounting regressions. These checks do not launch another native cohort or V6.

The 32-member [index](index.json) has SHA256
`d5bd23a54f15415f2cc1ea5f3ca39d71e24fe2e9691e0e6f4f7c81d5f71f9e8b`.
It includes raw receipts, exact parent verifier/test sources, the unchanged
qualification manifest and the sealing source. Independent replay only reads
these retained bytes:

```text
python docs/testing/performance/0.74/local-runs/coordinator-native-20261026/final-checks/verify.py
```

This ledger does not change the first native packet, certify collector overhead,
close full package coverage or authorize a production/server run.
