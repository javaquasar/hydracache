# Data Residency Governance

HydraCache residency governance is a legal/safety boundary, not a placement
preference. Performance placement may choose efficient homes and backups, but a
`ResidencyPolicy` first defines where value bytes are allowed to exist. If those
rules conflict with availability, HydraCache refuses the operation or reports a
degraded state instead of silently moving bytes across the boundary.

## Policy Shape

Policies are committed through the authoritative control plane and carry a
format version plus a policy epoch:

```rust
use hydracache::{
    ClusterEpoch, RegionId, ResidencyPolicy, ResidencyPolicyEnforcer,
    ResidencyPolicySet,
};

let mut policies = ResidencyPolicySet::new();
let eu_only = ResidencyPolicy::new(
    vec![RegionId::from("eu")],
    2,
    ClusterEpoch::new(7),
)?;

policies.commit_namespace_policy("users", eu_only)?;
let enforcer = ResidencyPolicyEnforcer::new(policies);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Namespace policies can be overridden per key when a namespace contains mixed
residency classes. Unknown future policy formats are rejected before commit.

## Enforcement Points

Placement filters candidate nodes to allowed regions before asking the
zone-aware strategy to satisfy RF. If the required RF cannot fit inside the
allowed set, the put is rejected loud.

WAN links call `RegionLink::try_send_with_residency` before admitting a batch.
If any governed value would be sent to a forbidden destination, the batch is not
sent and `residency_refused_crossing_total` is incremented.

Reads call `guard_read` with the region and observed policy epoch. Reads from a
forbidden region, or reads using a stale policy epoch, fail closed even if a
stale local copy is present.

Include-value invalidations must check `include_value_allowed`. A forbidden
subscriber receives an invalidation without value bytes.

## Policy Narrowing

When a policy is narrowed, existing value locations are scanned through
`plan_policy_narrowing`. Every out-of-policy location gets an explicit
remediation action, currently `Evict` or `MarkDegraded`. The detail is retained
in audit events, while metrics remain bounded:

- `hydracache_residency_rejected_placement_total`
- `hydracache_residency_refused_crossing_total`

W6 promotes these residency audit events into the consumer-facing audit sink.
