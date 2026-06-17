# HydraCache 0.41.0 Distributed Cache Grid Roadmap Plan

`0.41.0` should define the roadmap and first implementation slice toward a full
production distributed cache grid.

Current readiness:

```text
Full production distributed cache grid: about 3-4/10
```

Target after this release:

```text
Full production distributed cache grid: roadmap + first safe slice, about 4.5-5/10
```

This is intentionally framed as a roadmap release. A true production data grid
requires multiple releases and changes the product class: HydraCache would move
from embedded local-first caching with cluster coordination into distributed
storage, replication, failover repair, durable consensus, and operational
security.

## Release Theme

Define the path to a production data grid without accidentally claiming it too
early.

The release should:

- document architecture decisions for distributed values;
- define ownership, replication, backup, and failover semantics;
- choose the first supported topology;
- define durable control-plane requirements;
- define transport security requirements;
- add the first narrow replication/failover prototype if feasible;
- create the test matrix required before any production-grid claim.

## Non-Goals

- Do not claim production data-grid readiness in `0.41.0`.
- Do not add arbitrary remote code execution.
- Do not make remote value replication mandatory for local-cache users.
- Do not support every topology at once.
- Do not hide consistency limitations.
- Do not implement distributed transactions in the first slice.

## 1. Architecture Decision Records

### Required ADRs

Add ADRs covering:

- owner and backup ownership model;
- value replication model;
- consistency model;
- durable control-plane model;
- transport security model;
- failure detection and repair model;
- rolling upgrade/wire compatibility model.

### Pluses

- Prevents accidental architecture drift.
- Makes production claims reviewable.
- Separates embedded cache behavior from distributed storage behavior.

### Required Tests

Docs-only ADRs do not need runtime tests, but release gates should assert that
the ADR files exist and are linked from cluster readiness docs.

## 2. Ownership And Backup Replication Model

### Problem

Current ownership is deterministic rendezvous over admitted members, but there
is no value replication or backup owner repair.

### Planned Model

For each key:

```text
primary owner = rendezvous(key, members)[0]
backup owners = rendezvous(key, members)[1..replication_factor]
```

Candidate API:

```rust
let ownership = cluster.placement_for_key("user:42");

assert_eq!(ownership.primary.node_id(), "member-a");
assert_eq!(ownership.backups.len(), 1);
```

### What Changes

Before:

- owner selection is useful for peer-fetch/read-through;
- no backup value placement;
- owner failure means cache miss/reload path, not distributed repair.

After first slice:

- placement model can identify primary and backups;
- diagnostics expose placement decisions;
- replication protocol can target backup owners;
- failover design has a concrete contract.

### Risks

- Replication factor increases memory and network usage.
- Ownership churn during membership changes can cause extra replication traffic.
- Without durable value storage, replication still only protects in-memory
  cached values.

### Required Tests

- placement deterministic for same member set;
- placement changes predictably when member joins/leaves;
- no duplicate backup owners;
- replication factor larger than member count degrades clearly;
- diagnostics expose primary and backups;
- property test for stable placement distribution across many keys.

## 3. Value Replication Protocol Prototype

### Planned First Slice

Implement a narrow opt-in prototype:

- replicate encoded value bytes from primary to backup after local write/load;
- replicate invalidation to primary and backups;
- expose replication counters;
- keep local-only behavior unchanged when replication is disabled.

Candidate API:

```rust
let cache = HydraCache::member()
    .replication_factor(2)
    .replicate_values(true)
    .start()
    .await?;
```

### Pluses

- First real step toward data-grid behavior.
- Backup owners can serve a value if primary disappears in controlled tests.
- Replication counters make cost visible.

### Risks

- Replication correctness is harder than invalidation propagation.
- Encoded values may be large.
- Backpressure and memory limits become production concerns.
- Serialization compatibility matters across versions.

### Required Tests

- value loaded on primary replicates to backup;
- backup has encoded bytes after replication;
- invalidation removes primary and backup copies;
- replication disabled keeps current behavior;
- replication respects generation/epoch;
- stale-generation replication rejected;
- large value obeys configured max payload;
- replication failure increments counter and reports degraded state;
- rolling wire-version mismatch rejects replication safely.

## 4. Failover And Repair Design

### Problem

Production data grids need clear behavior when a primary owner leaves or fails.

### Planned Work

First release should design and optionally prototype:

- backup promotion after primary leaves;
- repair task after membership change;
- re-replication to restore replication factor;
- tombstone/invalidation handling during repair;
- diagnostics for under-replicated keys.

### Required Tests

- primary leaves, backup can serve value in controlled in-memory test;
- membership change triggers under-replication report;
- repair restores replication factor;
- invalidation during repair wins over stale value replication;
- failover does not resurrect invalidated value;
- timeout/degraded report when no backup exists.

## 5. Durable Multi-Node Control Plane

### Problem

The current raft metadata runtime drives real raft-rs lifecycle, but it is
single-node/in-memory for production purposes. Full grid readiness needs a
durable multi-node control-plane story.

### Planned Work

- design persistent raft log store trait;
- design snapshot/compaction behavior;
- define multi-node raft transport boundary;
- add deterministic in-memory multi-node raft tests if feasible;
- document whether a production deployment should use embedded raft, external
  consensus, or custom control-plane implementation.

### Risks

- Durable Raft is a large product area.
- Incorrect consensus integration is worse than no consensus.
- Storage engine choices affect portability.

### Required Tests

- persistent log append/replay fake store;
- snapshot recovery after restart;
- duplicate command id idempotency after replay;
- multi-node raft simulation if implemented;
- network partition simulation if implemented;
- control-plane degraded diagnostics when quorum unavailable.

## 6. Transport Security Model

### Problem

Token/header auth is enough for staging and internal pilot behind a trusted
boundary, but full production grid needs a stronger security model.

### Planned Work

- document that HydraCache still does not terminate TLS directly unless a
  dedicated transport layer is added;
- define mTLS/service-mesh recommended deployment;
- add node identity abstraction;
- require authenticated node identity for replication endpoints;
- support key rotation or token provider trait if keeping header auth;
- add authorization checks for peer-fetch, owner-load, replication, and admin
  routes.

### Required Tests

- identity required for replication route;
- peer-fetch and replication use same or explicitly separate auth policies;
- wrong identity rejected;
- token provider rotates token in client requests;
- actuator shows security posture without exposing secrets.

## 7. Operations And SLOs

### Required Observability

- replication success/failure counters;
- bytes replicated;
- replication lag;
- under-replicated key count;
- failover count;
- repair task count;
- repair failures;
- placement churn;
- memory overhead from backups;
- transport auth failures;
- wire compatibility failures;
- control-plane quorum health.

### Required Docs

- deployment topology;
- memory sizing;
- replication factor selection;
- failure scenarios;
- rolling upgrade;
- backup owner behavior;
- repair runbook;
- security checklist;
- "still not distributed transactions" warning.

## 8. Test Matrix For Future Production Grid Claim

Before any future release claims production data-grid readiness, the project
needs:

- deterministic unit tests for placement;
- in-memory multi-node replication tests;
- HTTP replication route tests;
- auth/wire compatibility tests;
- failover/repair tests;
- chaos/partition simulation;
- long-running soak with membership churn;
- large value and memory pressure tests;
- rolling upgrade compatibility tests;
- persistence/restart tests for metadata;
- optional persistence/restart tests for values if durable values are added;
- external consumer tests against published crates.

## Release Gates For 0.41

Focused:

```powershell
cargo test -p hydracache --locked placement
cargo test -p hydracache --locked replication
cargo test -p hydracache-cluster-transport-axum --locked replication
cargo test -p hydracache-cluster-raft --locked persistent
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.41.0` should only claim "distributed cache grid roadmap and first slice" if:

- ADRs define the data-grid direction and limits;
- placement model supports primary and backup owners;
- replication prototype is either implemented and tested or explicitly deferred;
- failover/repair semantics are documented;
- durable control-plane requirements are documented;
- security model is documented beyond staging token auth;
- docs still state that production data-grid readiness is not complete.
