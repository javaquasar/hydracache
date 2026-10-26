# Late cleanup witness refusal

This follows the [initial owned-Job packet](../ci-watchdog-owned-cleanup-20261026/README.md),
whose index and first results remain unchanged. Baseline commit:
`d9941796dc8766bcc67b2472b58abe94b6120855`. Index SHA256:
`cf57df5264e4a33ce629111d07e578373beec1995ba2cb3f01af5001d53822aa`.

Two new deterministic controls on that runtime give seventeen passes and two
failures: emptiness and close could finish outside the five-second cleanup bound
without refusing success. The correction adds only two deadline checks. The same
nineteen-test file then passes, with six native fault/stream controls in one cohort
and all twelve old cases per compiler. Execution stays at its original 0.35-second
marker deadline and cleanup stays at five seconds. No original failure is erased.

```text
python docs/testing/performance/0.74/local-runs/ci-watchdog-cleanup-budget-20261026/verify.py
python docs/testing/performance/0.74/local-runs/ci-watchdog-cleanup-budget-20261026/test_verify.py -v
```

The offline verifier reads only raw objects and data. It checks exact before/after
source construction, equal red/green test bytes, complete verdicts, source/HEAD
stability and held original process lifetimes. It runs no captured Python, Rust,
Git, process fixture or server action. Seven integrity controls test drift/refusal.
This remains CI correctness evidence, not a full package or production admission.
