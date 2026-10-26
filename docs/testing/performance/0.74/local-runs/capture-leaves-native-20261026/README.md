# Native capture leaves evidence

The sole leaf-enabled local control on source
`ef3d0cadd70e436b75be94e579a605447aaf7e2b` completes all sixteen workers and
sixty-four Git calls. All 448 outer/leaf probes are complete and valid, with
original operation verdicts and no cleanup errors. The exact no-run build and
native caller retain unchanged source/HEAD receipts. No attempt is retried.

The immutable [index](index.json) has 164 members in 161 addressed objects,
SHA256 `926f30f0eca6ae708b3706976f256a9ca52a3a5d7e9a9c447a8f45cec59a0855`.
It includes the complete raw worker logs/ledgers, intent/build/result, exact
collector/environment/worker sources, wrappers/sealer, model predecessor index,
unchanged qualification manifest and old independent outer verifier. Retained
collection code is evidence, not instructions to execute it.

```text
python docs/testing/performance/0.74/local-runs/capture-leaves-native-20261026/verify.py
python docs/testing/performance/0.74/local-runs/capture-leaves-native-20261026/test_verify.py -v
```

The independent verifier hashes bytes and checks original build/source, Git
semantics, worker lifetimes, terminal Job census, signed blocks, raw outer/leaf
clocks, cross-group order and parent containment. It computes leaf sums and
arithmetic complements without executing retained code or sampling clocks.
Six integrity cases cover corrupt/missing objects, semantics and nested raw
timing faults. The [report](../../capture-leaves-native-result-20261026.md)
distinguishes this native functional result from overhead and causal API cost.
No V6, server action, product candidate or production admission follows.

The separate [final-checks ledger](final-checks/README.md) retains eight passing
receipts and exact first/final verifier sources, including all 74 regressions,
check/Clippy and documentation/local-contract checks. Index SHA256:
`7ec1c4af34c9bfe99890d6febbe7d54ad9e2ec704bedd5c44f35cd40b19d8cab`.
