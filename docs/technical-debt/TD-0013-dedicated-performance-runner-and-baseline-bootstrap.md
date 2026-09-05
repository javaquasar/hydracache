# TD-0013: Dedicated performance runner and baseline bootstrap

## Status

Resolved — 2026-09-05. One protected bare-metal runner was qualified, two full-dress runs were
admitted, exactly five chained bootstrap samples were retained, and the resulting reference
contracts were independently reviewed and activated.

Owner: performance/release infrastructure.

Resolution target: the evidence-only `0.67.1` reference activation. The separate frozen-candidate
run remains release work item W7 and must pass before `0.67.1` may ship or publish a final
performance verdict.

## Context

Release 0.67 implemented the open-loop load generator, scenario catalog, report schemas, runner
attestation, immutable-anchor and rolling-baseline validation, exact-candidate receipts, and the
protected self-hosted workflow. Release 0.67.1 supplied the missing dedicated-host evidence: one
privacy-safe fingerprint, five exact pre-candidate samples, and independently reviewed anchor,
rolling baseline, and numerical budgets. The committed `reference-v1` contract is now
`bootstrapped`.

## Why It Was Deferred

GitHub-hosted runners are shared and variable. Their results are useful as broad regression tripwires, but they cannot honestly establish an immutable capacity anchor, portable sizing guidance, or a same-box comparative release claim. Weakening repeat counts, zero-error rules, SLOs, the 15% spread limit, fingerprint checks, or baseline eligibility would manufacture confidence rather than evidence.

## Resolution Evidence

- one non-virtualized bare-metal host passed the v5 topology, isolation, NVMe, kernel, toolchain,
  calibration, and IRQ admission contract;
- two distinct full-dress runs established immutable admission for the bootstrap chain;
- five successful `main` runs from commit
  `b3304e3a560fdddaf820d813a76773ca33565c50` used one runner fingerprint and one contract family;
- every sample was artifact-bound, zero-error, stable under its committed acquisition limits, and
  chained to the preceding accepted receipt;
- the proposal retained all five eligible samples, selected medians rather than a fastest run,
  preserved the fixed 10% anchor/rolling tolerance and 5% frozen-candidate per-report spread
  ceiling, and received an independent digest-bound approval;
- reviewed bytes are committed under `docs/testing/perf-{anchors,baselines,budgets,reviews}/0.67.1/`.

## Remaining Release Boundary

- GitHub-hosted `ci-shared` measurements remain tripwire-only and never capacity evidence.
- The reviewed numbers describe only the admitted physical host, exact scenarios, exact toolchain,
  and documented method. They are not portable sizing guidance or universal Redis comparisons.
- Hardware, kernel, topology, governor, turbo, storage identity, toolchain, or contract drift requires
  requalification; it must not migrate this baseline automatically.
- W7 must still run the complete frozen candidate from the exact activation merge SHA. Until it is
  green, `0.67.1` is not shipped and the reviewed bootstrap values are not a final release verdict.

## Definition Of Done

This debt was closed after all bootstrap-specific conditions were independently verified:

1. A protected non-shared bare-metal runner with the exact `hydracache-perf-v1` label is connected and satisfies the committed host contract.
2. At least five eligible, stable, successful `main` runs from one runner fingerprint and contract family are retained.
3. The immutable anchor, rolling baseline selection, and numerical budgets are independently reviewed.
4. The committed `reference-v1` profile, anchor, budgets, and baseline are changed from `unbootstrapped` to `bootstrapped` without candidate self-baselining.
5. The activation commit preserves candidate self-baseline prevention. The subsequent full frozen
   candidate, including core, RESP/Redis, control-plane, budget, canary, receipt, and artifact
   integrity stages, remains the distinct W7 ship gate.

## Related

- `docs/plans/V0_67_PERFORMANCE_CHARACTERIZATION_PLAN.md`
- `docs/PERFORMANCE.md`
- `docs/testing/PERF_RUNNER_0_67_1.md`
- `docs/testing/PERF_REFERENCE_0_67_1_REVIEW_AND_ACTIVATION.md`
- `docs/testing/perf-profiles/reference-v1.toml`
- `docs/testing/perf-budgets/0.67/reference-v1.toml`
- `docs/testing/perf-baselines/0.67/reference-v1.toml`
- future reviewed activation paths: `docs/testing/perf-{anchors,baselines,budgets,reviews}/0.67.1/`
