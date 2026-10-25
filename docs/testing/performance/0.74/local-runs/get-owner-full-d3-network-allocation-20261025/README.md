# GET owner real network allocation rejection

Terminal negative decision for GET-owner in Release 0.74. All 240 processes
complete; all four required RESP mTLS cells miss the unchanged 20% gross-layout
reduction floor. The complete paired results are in the indexed
`analysis-first.json`; the [result and scope](../../get-owner-full-d3-terminal-result-20261025.md)
explain why this is sufficient for rejection, not positive full-D3 admission.

Measured source: `be0499a8cb0a2ad0b7328d6ac0cf6227114cebcc`.
Autonomous finite coordinator: `53b99dea6eb56f437b0ed765db293169f91858e4`.

| GNU release image | SHA256 |
| --- | --- |
| allocation off | `f28862c0f594a91e9cb38af9266ffe9689995009c77b56d05e7a63517ae76d12` |
| allocation on | `7a5b99472df28ef6d143533e0010633829a3f6c13bed912c559ecac9edfa4bb2` |

`index.json` maps logical raw members to exact SHA256-named binary objects.
Keep `objects/* -text -diff`: CRLF logs, original JSON and compressed transport
streams must not be normalized. The packet includes the original streams,
source/build receipts, configs, first refusals, fault controls, per-attempt writer
retirement and final supervisor snapshot. Private PKI and ELF images are excluded.
No reader contacts the server or executes a candidate.

```text
python docs/testing/performance/0.74/local-runs/get-owner-full-d3-network-allocation-20261025/verify.py
python docs/testing/performance/0.74/local-runs/get-owner-full-d3-network-allocation-20261025/test_verify.py -v
```

Eight mutation controls cover raw bytes, missing pairs, first-refusal rewriting,
source/wire drift, populated writers and rounding into acceptance. The reader
also validates all measured receipts against the original pinned coordinator
predicates and recomputes the unchanged five-pair log-scale t(4) estimator.

The approved short maintenance ends with supervisor PID 377415 active/running,
NRestarts=0, original boot/image/receipt and no campaign/lifecycle marker. CPU,
p99, goodput, full allocator retention and production original-start admission
are not established by this packet. The candidate is rejected, not qualified.
