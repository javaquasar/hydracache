# Native coordinator brackets evidence

This packet retains the sole enabled-mode local native attempt on source
`1030acb263e96d1e023f852dceb021de458ce6f4`, following an exact no-run build.
All sixteen workers and sixty-four ordinary Git calls pass. All twelve probes
per worker are complete and valid; no first result is replaced or retried.

The immutable [index](index.json) contains 159 members in 157 addressed objects;
its SHA256 is
`55b802621e147681f9478a164053a04c055ec28d8d35227d68d49ad0b4de380f`.
It preserves the old model index and exact sources/protocols, two bounded
source/HEAD receipts, raw intent/build/result and all worker logs/result ledgers.
Collector and wrapper sources are evidence, not instructions to run them.

```text
python docs/testing/performance/0.74/local-runs/coordinator-native-20261026/verify.py
python docs/testing/performance/0.74/local-runs/coordinator-native-20261026/test_verify.py -v
```

These commands only read retained bytes. The independent verifier hashes every
member and checks build/source/worker joins, original semantics, terminal Job
census, all raw bracket arithmetic and signed block sums. Six integrity tests
exercise both byte corruption and semantic mutations without invoking retained
code, clocks or a process. The [result](../../coordinator-native-result-20261026.md)
distinguishes functional native success from unresolved overhead and package
admission. Both worker arms have coordinator tracing enabled, so these are not
an enabled/disabled recorder-overhead comparison. No V6 or server run occurs.

The separate [final-checks ledger](final-checks/README.md) retains seven passing
checks, including all 56 model/regression cases, check, Clippy, documentation and
the local contract. Its independent index SHA256 is
`d5bd23a54f15415f2cc1ea5f3ca39d71e24fe2e9691e0e6f4f7c81d5f71f9e8b`.
