# Model only coordinator phase telemetry evidence

The sealed [index](index.json) retains 40 members in 31 SHA256-addressed objects.
Index SHA256:
`7f768549711cb4cdd5a0e2de8bcac1502951499c5a36e28f05c678dd483d2eb1`.
Implementation source is `e6ef8937297042a2cb4f45386af360db6c9c8732`.
No enabled-mode native control, Git workload or V6 is executed.

The packet contains six source-preserving receipts, twenty new models, sixteen
old caller and twenty accounting regressions, exact source/protocol/design and
the first seventeen-model source. Four Tape-only deterministic examples retain
healthy, clock-refused, dual-refused and interrupted capture output. They use
injected integer clocks with no Job, process owner or native clock control;
their modeled operation verdicts are not real process verdicts.

The original collector and its immutable index are also retained. Independent
AST replay strips only literal telemetry begin/end calls and final attachment,
then compares the traced operational body to the current and original body.
The [result](../../coordinator-frames-result-20261026.md) distinguishes model
compatibility from the still-required first native/overhead checks.

```text
python docs/testing/performance/0.74/local-runs/coordinator-frames-20261026/verify.py
python docs/testing/performance/0.74/local-runs/coordinator-frames-20261026/test_verify.py -v
```

These commands verify bytes, AST and synthetic bracket arithmetic only. They
do not execute retained collectors or examples, launch a process or qualify
production ownership. All previous packets and first results stay unchanged.

The [final-checks index](final-checks/index.json) retains six passing receipts:
integrity, verifier syntax/indentation, all-target xtask check, strict
all-target/all-feature Clippy, doc-check and local non-promotable performance
contract. Exact verifier sources and unchanged qualification manifest are also
pinned. Index SHA256:
`8a9ae6d38650c810241e5a816515de1c78f033456c5889161b1a9e4531aa9d2f`.
`python docs/testing/performance/0.74/local-runs/coordinator-frames-20261026/final-checks/verify.py`
checks those bytes without any command replay or native control.
