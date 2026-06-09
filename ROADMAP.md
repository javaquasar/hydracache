# Roadmap

## Phase 1

- Initialize modular workspace
- Define cache API skeleton
- Add local in-memory backend
- Add TTL support
- Add tag-based invalidation
- Add single-flight loader coordination

## Phase 2

- Clarify runtime guarantees
- Improve metrics and tracing hooks
- Design distributed invalidation event model

## Phase 3

- Add database-neutral adapter contract
- Implement SQLx as the first adapter
- Explore query-specific macros

## Phase 4

- Add attribute-style ergonomics
- Design cache events and listeners (`docs/plans/V0_18_CACHE_EVENTS_LISTENERS_PLAN.md`)
- Prepare for future distributed storage and stateful runtime evolution
