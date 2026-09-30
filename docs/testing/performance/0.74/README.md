# HydraCache 0.74 RESP/native performance evidence

This directory begins W0/W1 from the exact frozen 0.73 product candidate
`16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b`. The predecessor has not yet been
published as an annotated `v0.73.0` tag, so the checked-in identity is deliberately named
`B73-candidate`, D0 remains open, and no numerical release claim is permitted.

The scenario matrix keeps RESP, HC/1, HC/2, direct `ClientSurfaceState`, raw embedded
`HydraCache`, and typed embedded `HydraCache` results separate. The three execution tiers are
also separate: `local-quick` is a developer smoke, `local-attribution` is non-promotable D1
evidence, and `release-qualification` requires explicit authorization on an admitted host.

Run the structural gate locally with:

```text
cargo xtask performance-contract-check --release 0.74
```

The gate validates the frozen dimensions, native non-regression rules, workload-equivalence
fields, proposal isolation, required metrics, and the expected pending predecessor state. It must
fail if asked for ship admission while publication, Redis provenance, or release evidence is
incomplete.

The first non-promotable local attribution and the rejected W2/W3 results are documented in
[`w1-local-attribution.md`](w1-local-attribution.md). Raw identity-bound receipts are retained under
`local-runs/`.

Numerical receipts must never be hand-edited into claims. Every receipt binds the trace, payload
and key corpora, seed, warmup, duration, offered schedule, concurrency, pipeline depth, security,
persistence and final-state digest. Errors, timeouts, rejections, late operations and incomplete
operations stay in the goodput denominator.
