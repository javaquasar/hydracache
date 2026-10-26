# Capture leaves verifier history and final local checks

The [31-member index](index.json) has SHA256
`64b1ada902211b722908a13f62a05cdaeb8e9d976aa0318670c05c705fddf8d1`.
It retains six source-preserving receipts and all three independent reader
revisions. The first two five-case integrity runs each have one verifier error:
an overly broad whole-caller comparison, then getter selection instead of the
setter. Both remain unchanged. The corrected reader passes all five cases;
original measure AST and untouched dependency bytes remain mandatory.

Syntax/indentation, doc-check and the local non-promotable performance contract
pass. Exact final verifier/test sources, first reader sources, qualification
manifest and sealer are retained separately from the immutable first model
packet. No native control, workload, V6 or server action occurs.

```text
python docs/testing/performance/0.74/local-runs/capture-leaves-20261026/final-checks/verify.py
```

This replay only reads retained bytes. It asserts both original failed outcomes
and the final successful integrity/gate outcomes, not a rewritten all-pass
history or full-package/production admission.
