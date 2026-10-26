# Model only capture leaves evidence

The [immutable index](index.json) contains 63 members in 44 addressed objects,
SHA256 `d539ff2fa93959cf1fbb176e4994ebb5d7f83c5e141a0e1bae81f672f0cb2000`.
Final implementation source is `8a0a10cab2a87da519e882838b11645d702fb5e9`.
This is a model/source packet: zero native controls and zero new workload calls.

Nine receipts retain the first twelve-case fixture error, subsequent 15/17/18
passing suites, 56 old regressions, syntax/protocol bounds, check, strict Clippy
and the final exact-source 74-case run. All wrappers retain source/HEAD and
original bounds. Exact historical/final code, original native packet index and
body bytes, wrapper/sealer sources and qualification manifest are included.
Retained collection code is evidence, not instructions to run it.

```text
python docs/testing/performance/0.74/local-runs/capture-leaves-20261026/verify.py
python docs/testing/performance/0.74/local-runs/capture-leaves-20261026/test_verify.py -v
```

The independent reader only hashes/parses retained bytes and compares restricted
AST transformations. It never imports a retained collector, samples clocks or
starts a process. Five final integrity controls cover first-result/source
mutation, missing objects and unsafe normalization. The
[report](../../capture-leaves-result-20261026.md) keeps native compatibility,
wait/collection separation, overhead and production admission explicitly open.

The [separate final-checks ledger](final-checks/README.md) preserves both first
verifier errors, their exact reader sources, the five-case corrected pass and
passing syntax/doc/local-contract checks. Its index SHA256 is
`64b1ada902211b722908a13f62a05cdaeb8e9d976aa0318670c05c705fddf8d1`.
