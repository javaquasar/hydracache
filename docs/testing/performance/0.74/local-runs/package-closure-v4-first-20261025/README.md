# First full Windows package closure packet

Exact source `b0f627fec0d592485233b527c61014128b4d689a`; unchanged command,
780-second origin/bound and four test threads. This result remains red:
114/138 targets finish, with 777 passed and 46 failed cases. The interrupted
321-case target has 104 passing/30 failing observations, 187 without verdicts.
Twenty-three targets never start. No old result fills those gaps and no unchanged
package is retried.

The 30 indexed members retain raw streams, intents, source maps, cleanup,
preflight and focused controls/check/lint logs. Index SHA256:
`0b0b568cc495e0a24f21f6684dbc6d471e3e3d1c1240fce002958975ebd31a9a`.
Keep `objects/* -text -diff`: panic bodies and CRLF streams must not be normalized.

```text
python docs/testing/performance/0.74/local-runs/package-closure-v4-first-20261025/verify.py
python docs/testing/performance/0.74/local-runs/package-closure-v4-first-20261025/test_verify.py -v
```

The reader checks every raw member, exact source/command/wrapper and complete
target catalog, then recomputes accounting with the byte-pinned current offline
parser. It executes neither archived implementation nor Cargo, subprocesses or
network calls. Seven mutation controls check first-result preservation. See the
[result and next repairs](../../package-closure-v4-result-20261025.md).
