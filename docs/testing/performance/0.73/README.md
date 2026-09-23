# HydraCache 0.73 local performance screening

`local-screening-073-v1` is the first W0 contract. It makes local development evidence reusable
without allowing a laptop, workstation, WSL, or shared runner to create a release performance
claim.

Validate the checked-in contract and example:

```text
cargo xtask performance-contract-check --release 0.73
```

Validate a generated receipt:

```text
cargo xtask performance-contract-check --release 0.73 --receipt target/performance-evidence/0.73/local/<attempt>/receipt.json
```

Every receipt binds the exact source, prebuilt binary, scenario, host fingerprint, raw series,
instrumentation mode, pair order, and complete operation outcomes. Failed and invalidated attempts
remain append-only. Local receipts always carry `promotable: false` and
`numerical_claim_eligible: false`; `--require-ship` therefore fails until later W0/W10 contracts
add dedicated-host and integrated-candidate evidence.
