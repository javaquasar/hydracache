# First real Git process boundary control

The sole native control on `3807c4749873d9a3e9495c755bbf3ef1f73f9c7b`
passes with two distinct retained processes: the cmd Git launcher and its
mingw64 native child. EOF releases both original objects; root exit is zero,
streams are empty and cleanup has no error. No object request is sent.

Index SHA256: `c59c64b3f666377ad1098c1795ceead8583f0f1a0d96f03efda1d5f41ad741c5`.
The packet has 75 members/48 objects: exact source and independent image pins,
precommitted protocol, raw first control, before/after source maps, all model
stages, first late-close failure and actual pre-fix source copies. Early ten-
and twelve-case model stages retain their streams/source hashes; their transient
test sources are not represented as raw archived copies.

```text
python docs/testing/performance/0.74/local-runs/git-process-boundary-20261026/verify.py
python -m unittest discover -s docs/testing/performance/0.74/local-runs/git-process-boundary-20261026 -p test_verify.py
```

The verifier does not import or execute the retained collector, launch a child
or access Git. Four integrity controls refuse changed/missing source, result,
stream, independent pin and first-failure bytes.

The 13-case late-close test first fails against the preserved implementation;
the same tests pass after a fresh post-close deadline check. Two later discovery/
protocol controls and two failure-path controls bring new models to seventeen;
with 43 unchanged observer/lineage cases, all sixty pass. This is diagnostic
correctness, not overhead acceptance or performance qualification.

The control does not attest old cohorts' topology, mapped images or every
descendant. Before/after on-disk pins are not an ABA proof. Direct-root CPU must
not be reported as total Git CPU on this observed chain. No old result is
recomputed, no source-reader implementation is replaced and V6 stays red.
See the [analysis and next attribution boundary](../../git-process-boundary-result-20261026.md).

The separate [final checks index](final-checks/index.json) retains successful
post-documentation doc/structural-contract checks, integrity replay/controls,
indentation and exact verifier sources: 23 members/16 objects, SHA256
`256a36378782a1f67fac27ccc5c3c90887f822e252c0e47bdf6ef9ff81123e44`.
It does not change the immutable first-outcome index or admit a package.
