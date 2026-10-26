# Exact repaired package syntax inputs

This immutable source-only packet captures the first contextual scan of the
reader and Windows CI cleanup repairs. It contains 368 Rust sources and 25,868
syntax sites: three changed and three added rows relative to v5, no removals.
All 362 other rows are identical. The module closure has no unresolved edge.
Historical source, all 45 auxiliary inputs, original build/scan receipts and
collector sources are retained; no executable is distributed.

Index SHA256: `4788224085aadbfa874f16d13826fc34d2c32ac880116d7e959c8e27df53a062`.
There are 436 members and 431 objects. Canonical syntax SHA256:
`2552f3a6d96786433ea27b111a3f9f3b7274e82446634d68ca7d49090e24aa54`.

```text
python docs/testing/performance/0.74/local-runs/package-review-v6-20261026/verify.py
python -m unittest discover -s docs/testing/performance/0.74/local-runs/package-review-v6-20261026 -p test_verify.py
```

Syntax closure is not an effect review or package pass. The separate fixed
`package_effects_review_v6.py` reviews actual Windows entry-point effects and
requires these exact source/auxiliary/target identities. Old v5 pins remain
unchanged and refuse this new graph. No product or server admission follows.
