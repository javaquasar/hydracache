# TD-0004: Deferred home-region placement and autoscaling controllers

## Status

Open.

Owner: cluster/control-plane roadmap.

Candidate target: first release that explicitly owns adaptive placement and
provider integration after the 0.50 demo track. Until that release exists, the
tracked state is "intentionally deferred", not "silently forgotten".

## Context

The 0.45 active-active release introduced explicit home regions, capacity
signals, and guarded admission, but deliberately did not ship two higher-level
automation layers:

- automatic home-region placement / latency-based home assignment;
- cloud/provider-specific autoscaler controllers.

Those items were then repeated as deferred work through 0.45, 0.46, and 0.47.
Repeating them in every release plan makes the roadmap look open-ended and
harder to review. This debt record is the committed home for that deferral.

## Why It Is Deferred

Automatic home-region placement is not a cosmetic feature. It changes placement
authority: the system would move a key or namespace's preferred write/read home
based on observed latency, load, residency rules, and failure state. That needs
clear interaction with control-plane epochs, read-your-writes/session
watermarks, residency policy, and active-active bounded staleness.

Provider-specific autoscaler controllers are also outside the current library
boundary. HydraCache can emit capacity/admission signals, but turning those into
Kubernetes HPA, cloud autoscaling groups, or vendor-specific control loops adds
deployment-specific behavior and operator risk.

## Risk While Open

- Operators must assign home regions explicitly.
- Capacity signals are advisory; external automation must consume them.
- Latency-aware placement experiments live outside the supported release claim.
- Documentation must not imply automatic placement or provider autoscaling is
  already included.

## Revisit Triggers

Revisit this debt when at least one of these is true:

- control-plane APIs can safely commit a home-region change with epoch fencing;
- residency policy can prove the candidate home region is legal before the move;
- session/read-your-writes tests cover a home-region move during live traffic;
- an operator-facing deployment profile needs first-party Kubernetes/cloud
  controller artifacts.

## Future Definition Of Done

Closing this debt requires more than wiring a heuristic:

- deterministic planner tests for latency/load/residency inputs;
- multi-node simulation scenarios for home-region migration and rollback;
- control-plane compatibility registration for any new durable/wire artifact;
- operator runbook for enabling, pausing, and reverting placement automation;
- provider controller integration tests or explicit per-provider smoke gates;
- `cargo xtask verify` and the named integration/DevOps gates green.

## Related Plans

- `docs/plans/V0_45_ACTIVE_ACTIVE_MULTIREGION_PLAN.md`
- `docs/plans/V0_46_CLUSTER_RESILIENCE_AND_COORDINATION_PLAN.md`
- `docs/plans/V0_47_CROSS_REGION_SESSION_CONSISTENCY_PLAN.md`
