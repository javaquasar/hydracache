# Technical Debt

This directory tracks intentional tradeoffs that should be revisited later.

Each item should explain:

- why the debt exists
- what risk it carries
- what condition should trigger a revisit
- how to verify the debt can be removed safely

## Open Items

- [TD-0002: Raft protobuf advisory](TD-0002-raft-protobuf-advisory.md)
- [TD-0003: Dependency upgrade policy and backlog](TD-0003-dependency-upgrades.md)
- [TD-0004: Deferred home-region placement and autoscaling controllers](TD-0004-deferred-placement-and-autoscaling.md)
- [TD-0005: Release-claim evidence gap (Hibernate L2 / JVM artifact)](TD-0005-release-claim-evidence-gap.md)
- [TD-0008: Networked daemon grid hosting is deferred after W6a](TD-0008-networked-daemon-grid-hosting.md)
- [TD-0012: crates.io publication automation](TD-0012-crates-publication-automation.md)

## Resolved Items

- [TD-0001: Historical MSRV-pinned SQLx/testcontainers transitive dependencies](TD-0001-msrv-pinned-sqlx-transitive-dependencies.md)
- [TD-0006: Release-plan header status is not validated against the manifest](TD-0006-release-plan-status-validation.md)
- [TD-0007: Operator lifecycle E2E coverage is a prepared-state snapshot, not a driven chain](TD-0007-operator-lifecycle-e2e-coverage.md)
- [TD-0009: Coverage ratchet and coverage-run stability](TD-0009-coverage-ratchet-and-coverage-run-stability.md)
- [TD-0010: Cluster transport has no TLS termination and no peer auth](TD-0010-cluster-transport-tls-and-peer-auth.md)
- [TD-0011: Static raft voter set and address-derived node identity](TD-0011-dynamic-raft-membership-and-node-identity.md)
