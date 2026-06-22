# HydraCache — Agent Entry Point

This is the **single context door** for any agent (or human) working in this repo.
Read this file first, then follow the bootstrap recipe below. Keep this file short;
it links out instead of inlining.

HydraCache is a Rust workspace: a local-first cache + DB query-result caching
adapters (sqlx / diesel / seaorm) + cluster coordination (Local / Client / Member
roles). Crates live under `crates/`; the workspace version is in the root
`Cargo.toml`.

## Bootstrap recipe (do this in order)

1. **Read the rules.** `docs/RULES.md` holds the non-negotiable, cross-cutting
   invariants every change must respect (authority model, hard non-goals,
   fail-loud, fault-model determinism, metric cardinality, compatibility,
   boolean gates, test coverage). They are the single source of truth — plans do
   not redefine them.
2. **Find the active release.** `docs/plans/INDEX.md` (human) and
   `docs/plans/releases.toml` (machine-readable) list every release plan, its
   file, status, theme, and dependencies. Pick the one marked `in-progress` (or
   the lowest-numbered `planned`).
3. **Read that release plan.** Each plan under `docs/plans/V0_*` is a step-by-step
   spec: work items with problem/design/Rust sketch/implementation steps/tests/
   gates. Implement against it.
4. **Verify before finishing.** Run the gates — see `docs/GATES.md`. One command:
   `cargo xtask verify`. CI runs the same gates.
5. **Update the registries.** If you changed a durable/wire artifact, update
   `docs/COMPAT.md`. If a release changed status or you added/renamed a plan,
   update `docs/plans/releases.toml` (and INDEX.md). The `doc-check` gate enforces
   that the manifest stays consistent.

## Where things live

| What | Where |
| --- | --- |
| Cross-cutting invariants (rules) | `docs/RULES.md` |
| Release plans + status | `docs/plans/INDEX.md`, `docs/plans/releases.toml` |
| Enforcement gates + how to run them | `docs/GATES.md` |
| Compatibility register (durable/wire artifacts) | `docs/COMPAT.md` |
| Architecture decision records | `docs/adr/` |
| Architecture overview | `ARCHITECTURE.md`, `docs/architecture/` |
| Market positioning ("why not Redis?") | `docs/POSITIONING.md` |
| Cross-project review (Hazelcast/ScyllaDB/etc.) | `docs/plans/V0_37_41_REVIEW_AND_IMPROVEMENTS.md` |
| Competitive analysis + evolution (cluster) | `docs/COMPETITIVE_ANALYSIS_AND_EVOLUTION.md` |
| Storage engine + data-platform evolution | `docs/STORAGE_AND_DATA_PLATFORM_EVOLUTION.md` |
| Testing conventions | `docs/TESTING.md` |
| Observability contract | `docs/OBSERVABILITY_CONTRACT.md` |
| Feature matrix | `docs/FEATURE_MATRIX.md` |
| Repo automation (xtask) | `crates/xtask/` |

## House rules in one line

Boolean release gates, no numeric self-scores; fail loud, never silently degrade;
every new code path is covered by tests; distributed transactions are a permanent
non-goal. Details and the full list: `docs/RULES.md`.
