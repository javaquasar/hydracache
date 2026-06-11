# HydraCache 0.21.0 Release Strengthening Plan

Status: active strengthening pass.
Date: 2026-06-11.

0.21.0 is already release-ready, but the cluster-facing surface can be made
safer and easier to trust before tagging. The goal of this pass is not to turn
HydraCache into a production remote-value distributed cache yet. The goal is to
make the current embedded cluster primitives more observable, better tested, and
harder to publish incorrectly.

## Scope

1. Package verification for every publishable crate.
   - Add a repeatable release command/script for `cargo package` across all
     publishable workspace crates.
   - Document the expected publish/package order.

2. Ownership membership-change scenarios.
   - Cover client join/leave not changing ownership.
   - Cover member leave and newer-generation rejoin changing ownership
     predictably.
   - Cover stale candidates staying out of ownership decisions.

3. Peer-fetch negative scenarios.
   - Cover miss and removed-value behavior explicitly.
   - Cover no-owner request construction.
   - Cover generation metadata propagation and mismatch reporting semantics.

4. Ownership and peer-fetch diagnostics counters.
   - Track ownership resolutions and no-owner outcomes.
   - Track peer-fetch hits and misses for the in-memory peer-fetch seam.
   - Expose counters through cheap diagnostics structs.

5. Sandbox ownership-transfer scenario.
   - Add an OpenAPI/dashboard scenario that demonstrates member leave,
     ownership transfer, peer-fetch behavior, and client near-cache
     invalidation.
   - Include pass/fail assertions and generated-client coverage.

6. README cluster-support boundaries.
   - Explain what 0.21 cluster support includes.
   - Explain what remains intentionally out of scope.

7. Post-publish package/order smoke.
   - Extend post-publish verification so published crates are added and checked
     in dependency order.
   - Ensure the external consumer exercises the new ownership/peer-fetch APIs.

## Success Criteria

- Each scope item is committed separately.
- New code has focused tests.
- Sandbox scenarios are represented in OpenAPI, the dashboard, generated-client
  smoke checks, and route tests.
- Documentation describes how to run the new release/package checks.
- The full release gate remains green.

