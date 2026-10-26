# Offline Git CPU variance evidence

This packet retains 68 members in 60 SHA256-addressed objects. The
[index](index.json) has SHA256
`586480e0b6800d416ea2a83504b1beb3386fc0e33bb67fb78bcad125cf8a95d6`.
Analyzer source is `385feda3938aabf467857271047e60a5c961619f`.
The original 64-call input index remains
`1ba276dcf62f97801f69806ea5efba138c7d47425f30d2563874aa393681fcff`.
There are zero new workload calls and no V6 or measurement replay.

The packet includes the fixed old index, every original native worker log and
ledger, selected original collector/worker source, current analyzer/design/tests,
the first thirteen-model source, four source-preserving check receipts and the
first committed-source analysis intent/result. All seventeen final tests pass.
The [result](../../git-cpu-variance-result-20261026.md) distinguishes CPU-owner
arithmetic from unavailable causal API/phase attribution.

```text
python docs/testing/performance/0.74/local-runs/git-cpu-variance-20261026/verify.py
python docs/testing/performance/0.74/local-runs/git-cpu-variance-20261026/test_verify.py -v
```

The independent verifier recomputes CPU partitions, signed block deltas and all
32 profile phase statistics without importing retained analysis/collector code,
launching a process or sampling a clock. It retains reference counters/phases
as unavailable rather than inventing zeros. Overhead acceptance, causal leaf
cost, native-image attestation and the full package gate remain unresolved.

The [final-checks index](final-checks/index.json) retains passing integrity,
syntax/indentation, doc-check and local non-promotable performance-contract
receipts, exact verifier sources and unchanged qualification manifest. SHA256:
`5418f40dc1e2c469b61cbc1a4e8b67b83d559ef6bb6cb67123c81d5ab7276c8a`.
`python docs/testing/performance/0.74/local-runs/git-cpu-variance-20261026/final-checks/verify.py`
checks those bytes without replaying a workload or changing the analysis index.
