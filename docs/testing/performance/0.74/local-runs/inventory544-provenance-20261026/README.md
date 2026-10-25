# Fixed historical provenance for the 544 source phase

The 47 indexed members preserve the complete old/current Git trees, per-path
producer history, old ledger, seventeen current and three old changed bodies,
the separate phase ledger and the observed index. The worktree is checked against
fixed historical tree `958b3135a15c00b9f6aad41522cef9d1f6608d3a`, not used to
derive an approval pin. The source data contains 14 additions and 3 changes,
with 527 old files unchanged and no removals.

Index SHA256:
`cf14d72fa0637ef5aeab50ca96ff2cfa69a9e331da3e358c779ff0f830d807d7`.
Separate phase ledger SHA256:
`fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f`.
Keep `objects/* -text -diff`; raw source and logs are not normalized on checkout.

```text
python docs/testing/performance/0.74/local-runs/inventory544-provenance-20261026/verify.py
python docs/testing/performance/0.74/local-runs/inventory544-provenance-20261026/test_verify.py -v
```

The reader executes no archived code, Git, subprocess or network call. It checks
all raw hashes, source Git blob IDs, canonical hashes, both trees, unchanged old
ledger and exact delta/index. Six mutation controls are offline only. The first
retainer's assertion failure has its original source and a transcript-derived
note, not a falsely claimed raw stderr capture. See the
[repair result and remaining integration](../../inventory544-phase-result-20261026.md).
