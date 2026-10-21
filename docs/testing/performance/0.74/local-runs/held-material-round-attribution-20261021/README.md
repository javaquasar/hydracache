# Offline attribution of repeated material validation rounds

This packet derives costs from the sealed first original-owner refusals. It
starts no Rust fixture, WSL session, manager, clock/service operation, product
or server workload. See the [report](../../held-material-round-attribution-20261021.md).

The immutable input is `held-material-live-20261021`, inventory SHA256
`0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109`, measured at
`1d6d179bd7150ef564560821a897794e20b24b36`. The offline analyzer is frozen at
`5536087e7ad1acfa1af66f796631bdfb4338d2e0`. `facts.json` verifies all 65 input
members, reconstructs its original summary and retains both first expirations.

The facts conserve each phase as kernel/other wall inside the eight-step scopes
plus kernel/unassigned wall outside. Kernel, live and authentication inclusive
views overlap and cannot be added. Residuals do not identify effects or observer
overhead. The input gate and cleanup assertions remain false, not repaired by
this offline arithmetic. No source/observer, check frequency or deadline changes.

Read-only review from the repository root:

```powershell
pwsh -NoProfile -File docs/testing/performance/0.74/local-runs/held-material-round-attribution-20261021/review.ps1 -Mode Review
```

`method/` retains the new analyzer and tests. `checks/` retains the initial
synthetic test-oracle failure and its exact source, corrected/frozen controls
and local checks. The initial failure was an earlier invariant rejecting a
deliberately impossible synthetic trace, not a new real input refusal. Neither
the old packet nor its verifier is modified to accommodate this successor.
The new inventory is created once after collection and never replaced.
