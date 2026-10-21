# Isolated pidfd info pathname allocation candidate

The [report](../../pidfd-info-path-20261021.md) distinguishes the measured pure
allocation result from unmeasured full validation latency. Baseline/test commit:
`1c7e00f327e45be8ec543fcd9b8da21faf481874`; implementation commit:
`0cb604afdc14adb544ba613fd6d46189401254b4`.

`baseline/` retains the old complete origin, reference allocator control and its
first GNU/MSRV results before the production operand changes. `candidate/` keeps
the new source, unit/control results and checks separately. `final/` retains
both formatting refusals, the passing final formatting/source checks and local
contracts. The baseline is not overwritten by candidate control output.

The allocation capture only constructs pathname bytes; ordinary own-process
pidfd units separately check fresh opens/reads, trace counts and refusals.
No ignored test, original-owner profile, workload, clock/service modification,
server, installation or supervisor action occurs. The preceding first refused
packet and its offline attribution remain unchanged and do not become passed.

Read-only packet review from the repository root:

```powershell
pwsh -NoProfile -File docs/testing/performance/0.74/local-runs/pidfd-info-path-20261021/review.ps1 -Mode Review
```

Review validates raw membership, exact source binding, baseline/candidate count
rows, test/check receipts and prior seals without executing a Rust unit or WSL.
The inventory is written once after collection and never replaced.
