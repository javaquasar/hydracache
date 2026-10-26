# Short Git descendant CPU evidence

The sealed packet retains 97 raw members in 72 SHA256-addressed objects. The
original source `e3f560e69a8d1deda20999a0b1c9b7e31a75f11d` completes four
reference Git calls, then refuses a terminal image-path query. Changed source
`bd1b3cf2e6fc4a252037caf69603b0df6571aab4` checks the original worker before its
existing resume and completes one eight-call reference/accounted control.
Neither attempt runs V6 or a product workload.

The [index](index.json) maps logical names to exact raw bytes under `objects/`;
SHA256 is `c12ee4c7515ce6cd94744b7ce1754e172bbf7b2c7eba8f82893e4a3ab5defa41`.
It contains both native intents, build receipts, worker logs and first outcomes;
17 source-preserving check receipts; the original ten-model and pre-fix sources;
the unchanged CI creation primitive, pipe helper and process observers; and
both first old pipe-fixture refusals. Each old refusal differs only at the
libtest-printed `0.00s`/`0.01s` byte, verified offline from its full raw stream.

The [result](../../git-job-cpu-result-20261026.md) explains accounting scope,
numbers and remaining gates. Replay only the evidence, without Git, Cargo,
processes or importing the retained collector:

```text
python docs/testing/performance/0.74/local-runs/git-job-cpu-20261026/verify.py
python docs/testing/performance/0.74/local-runs/git-job-cpu-20261026/test_verify.py -v
```

Terminal Job CPU minus original worker and direct-launcher lifetime CPU is
218,750,000 ns for four accounted calls. It includes native descendants but is
not per-image/per-call CPU attestation, overhead acceptance or a speedup claim.
The immutable V6 verdict stays red. No windows, checks, source-reader semantics,
dependencies, qualification manifest, services, clocks or server state change.

The separate [final-checks index](final-checks/index.json) retains four passing
receipts: offline integrity, verifier syntax/indentation, doc-check and the local
non-promotable performance contract. It also pins the exact verifier sources and
unchanged qualification manifest. Its SHA256 is
`1c33f4088f5d4b2d81327bb4c91a9048eff0aa8ac58ccbbda54e6423f9fdd95a`;
`python docs/testing/performance/0.74/local-runs/git-job-cpu-20261026/final-checks/verify.py`
checks those bytes without rerunning any workload.
