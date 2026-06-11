# HydraCache 0.21.0 Cluster Operability Plan

Status: implementation plan.
Date: 2026-06-11.

HydraCache 0.20.0 introduced the first real cluster-adjacent shape: local,
client, and member roles; chitchat-backed discovery; raft-rs-backed metadata;
an admission bridge; generation-safe invalidation; and sandbox demos.

The 0.21.0 goal is to make that shape easier to verify, explain, and extend
without jumping straight into production multi-node Raft transport or remote
value execution.

## Release Theme

Make cluster behavior observable, testable, and ready for the next ownership
phase.

The release should improve confidence in the current cluster layers while
adding the smallest useful foundation for Groupcache-style ownership.

## Scope

1. Post-publish verification hardening.
   - Verify every published user-facing crate, not only the core runtime.
   - Add smoke coverage for the composition crate, actuator crate, and cluster
     adapter crates as external dependencies.

2. Cluster diagnostics v2.
   - Expose richer runtime health: role classification, bootstrap count,
     membership totals, bus subscriber count, and convenience health flags.
   - Keep diagnostics cheap and synchronous.

3. Ownership resolver.
   - Add a deterministic ownership API for mapping cache keys to admitted
     members.
   - Start with an in-memory rendezvous-style resolver built from cluster
     diagnostics.
   - Do not add remote value loading in this release.

4. Peer fetch design/API spike.
   - Introduce a small trait and in-memory implementation that show how a
     non-owner could request a value from an owner later.
   - Keep it value-agnostic and transport-neutral.

5. Sandbox cluster lab.
   - Add a runnable sandbox scenario that explains discovery, admission,
     ownership resolution, and invalidation in one report.
   - Include pass/fail assertions and OpenAPI schema coverage.

6. Release hardening.
   - Update docs, release notes, and publishing/testing instructions.
   - Run the normal release gate and MSRV checks before publishing.

## Non-Goals

- Production multi-node Raft transport.
- Durable Raft metadata storage.
- Remote owner-side database query execution.
- Security/authentication for cluster members or clients.
- External invalidation transports such as Redis, NATS, or Postgres
  LISTEN/NOTIFY.
- Replacing Moka or changing the local hot path.

## Success Criteria

0.21.0 is ready when:

- post-publish verification checks all publishable workspace crates;
- cluster diagnostics v2 is available through public APIs and tested;
- ownership resolution returns stable owners, handles no-member clusters, and
  changes predictably when membership changes;
- peer fetch has a documented trait/API seam and in-memory tests;
- sandbox exposes a cluster lab report that covers ownership and invalidation;
- README, release notes, and testing/publishing docs describe the new behavior;
- all workspace tests, docs, clippy checks, and MSRV checks pass.

## Deferred Beyond 0.21.0

- Network peer fetch transport.
- Owner-side cache fill execution.
- Ownership maps committed through Raft metadata.
- Member failover and ownership transfer.
- Persistent metadata snapshots.
