# Offline native leaf attribution evidence

Index SHA256:
`70ba30460b8e4c995c23d3f37090b0ef849bc2915e6507e62dfc5b23a7837417`.
The 73-member packet retains exact analyzer source
`10f160bef32c02171d42f0f8138acbbbc4d8695d`, first offline intent/result,
seven source-preserving check receipts, raw native inputs and their original
index. Six check receipts pass; the first formatting receipt preserves Windows
command-line refusal `os error 206`, not a formatting verdict. No new workload
or native measurement is executed.

Run `python verify.py` for independent hash/provenance and arithmetic replay.
Run `python -m unittest discover -s . -p test_verify.py -v` for six integrity
controls. The verifier reads data only and does not import retained analyzer or
collector code. It checks all worker projections, CPU owners, leaf totals,
distributions, signed blocks and parent CPU partitions, preserving the first
formatting failure. Neither command is V6 or a native benchmark.

See the [result and decision](../../leaf-attribution-result-20261026.md).
The original native packet and qualification manifest are unchanged. Recorder
overhead, causal API cost, full-package coverage and production admission remain
unaccepted.

The [final checks](final-checks/README.md) separately retain passing integrity,
documentation and local performance-contract receipts without rewriting this
packet or its first refusal.
