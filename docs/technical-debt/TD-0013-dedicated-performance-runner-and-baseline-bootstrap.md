# TD-0013: Dedicated performance runner and baseline bootstrap

## Status

Open — blocked on dedicated infrastructure.

Owner: performance/release infrastructure.

Candidate target: the first evidence-only follow-up after a protected `hydracache-perf-v1` runner is available. This debt does not block shipping the 0.67 measurement framework without numerical performance claims.

## Context

Release 0.67 implements the open-loop load generator, scenario catalog, report schemas, runner attestation, immutable-anchor and rolling-baseline validation, exact-candidate receipts, and the protected self-hosted workflow. The remaining work is not product code: no non-shared bare-metal runner is currently available to generate stable official reference evidence.

The committed `reference-v1` profile, anchor, budgets, and baseline therefore remain `unbootstrapped`. The five reference execution gates remain registered, manual, serialized, and fail-closed, but are deferred evidence gates rather than 0.67 ship-mandatory gates.

## Why It Is Deferred

GitHub-hosted runners are shared and variable. Their results are useful as broad regression tripwires, but they cannot honestly establish an immutable capacity anchor, portable sizing guidance, or a same-box comparative release claim. Weakening repeat counts, zero-error rules, SLOs, the 15% spread limit, fingerprint checks, or baseline eligibility would manufacture confidence rather than evidence.

## Risk While Open

- HydraCache has no official capacity floors or sizing guidance.
- No Redis comparative result may be quoted as a 0.67 release claim.
- No numerical `reference-v1` baseline or budget is active.
- A regression that remains inside the broad hosted-runner tripwire tolerance may not be detected until the dedicated lane is bootstrapped.

## Constraints While Open

- GitHub-hosted `ci-shared` measurements are tripwire-only and never capacity evidence.
- `reference-v1` remains `unbootstrapped`; numerical release claims are forbidden.
- The self-hosted workflow stays protected, manual, serialized, and fail-closed.
- Missing tools, unstable spread, mismatched fingerprints, insufficient history, or failed measurements remain red; this deferral must not relax their checks.

## Definition Of Done

Close this debt only when all of the following are independently verified:

1. A protected non-shared bare-metal runner with the exact `hydracache-perf-v1` label is connected and satisfies the committed host contract.
2. At least five eligible, stable, successful `main` runs from one runner fingerprint and contract family are retained.
3. The immutable anchor, rolling baseline selection, and numerical budgets are independently reviewed.
4. The committed `reference-v1` profile, anchor, budgets, and baseline are changed from `unbootstrapped` to `bootstrapped` without candidate self-baselining.
5. The full reference pipeline is green for one frozen clean candidate, including core, RESP/Redis, control-plane, budget, canary, receipt, and artifact-integrity stages.

## Related

- `docs/plans/V0_67_PERFORMANCE_CHARACTERIZATION_PLAN.md`
- `docs/PERFORMANCE.md`
- `docs/testing/PERF_RUNNER_0_67.md`
- `docs/testing/perf-profiles/reference-v1.toml`
- `docs/testing/perf-budgets/0.67/reference-v1.toml`
- `docs/testing/perf-baselines/0.67/reference-v1.toml`
