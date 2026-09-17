# AX42 D4 evidence and provisional 0.71 release decision

Status: **D4 campaign accepted; 0.71 release admission still in progress.** This is not a ship
authorization and does not authorize deleting the rented AX42 host.

The measured source and workflow SHA is `da8d6de409a657e0260e7fbfb4ab31d8d6ad5ca8`.
The [D4 evidence index](../../perf-artifacts/0.71/ax42/d4/README.md) binds the immutable M3,
M8, M9 and M10 campaigns to their GitHub runs, artifact IDs, sanitized archive hashes and the
separate evidence-branch commit. Every campaign receipt reports success and ship-evidence
eligibility. The exact-candidate `memory-campaign-check --require-ship` accepted the complete
10/8/2/2-job chain. M10 B1 ran for 86,400.002 seconds and M10 C for 86,400.008 seconds, each with
1,440 scenario iterations, 289 heartbeats and 360 HC/2 churn records. The 24-hour jobs ran
serially. The compatibility receipt is part of M10's artifact.

The D3 review admitted W2a retained-byte accounting as foundation-only and W4 active-expiry
reclamation as a safety fix, **not** as numerical RSS wins. The D4 M3 TTL candidate samples
reconciled to zero logical entries at post-idle; the B1 samples retained 5,000 at that phase.
Those different cardinalities must not be presented as a like-for-like RSS improvement. The M10
steady checkpoints were approximately 32.7 MiB RSS for B1 and 32.9 MiB for C at 10,000 entries;
that observation is not an improvement claim. No Redis, Hazelcast, allocator, universal sizing,
or cross-host advantage follows from this campaign.

The [release policy](release-policy.toml) records W2b, W5–W7, W8 and W9–W11 as deferred only
because the currently shipped owners are bounded and correct. A future efficiency proposal needs
new preregistered evidence. The 0.71 release narrative may state improved accounting, explicit
bounds, active-expiry cleanup and diagnostic confidence, but no optional numerical memory win.

The historical 0.67.1 prerequisite is an evidence-only milestone, not a package release: it
intentionally has no `v0.67.1` tag. The 0.71 release gate instead verifies the pinned SHA-256 of
its committed W0–W7 ship-ready closure receipt and confirms that its measured source is an
ancestor of this candidate. The 0.70.0 dependency remains a normal tagged release check.

Before ship, complete and review: generated `target/memory-evidence/0.71/release-claims.json`,
exact-commit fast/gated receipts and `release-evidence --require-ship`, public API compatibility,
workspace/target checks, final documentation and claim wording, and the exact measured SHA tag.
The final release tag must point to the measured commit above. The post-campaign
`release/0.71-finalization` branch adds only documentation and test-governance wiring; it is not a
new measured runtime candidate. Any runtime-code or scenario change requires a new immutable D4
chain rather than inheriting this result.
