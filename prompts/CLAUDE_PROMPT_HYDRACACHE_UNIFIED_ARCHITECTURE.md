You are working in the local workspace for the `hydracache` effort.

Your job is not to analyze a single repository.
Your job is to synthesize a **unified architecture proposal** for `hydracache` using the existing local knowledge bases and reference-project analysis.

## Primary goal

Produce a serious design document for `hydracache` as a Rust-native query cache system that combines:

- compile-time SQL correctness
- typed query execution ergonomics
- local cache performance
- duplicate load suppression
- future distributed cache evolution
- practical invalidation/freshness strategy

This is a synthesis and architecture task, not a generic summary task.

## Reference projects to use

You must use the existing local knowledge-base files as your primary source material.

### Highest priority references
- `C:\Workspace\prj\jq\cashe\sqlx\SQLX_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\readyset\READYSET_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\moka\MOKA_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\caffeine\CAFFEINE_KNOWLEDGE_BASE.md`

### Important secondary references
- `C:\Workspace\prj\jq\cashe\noria\NORIA_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\groupcache\GROUPCACHE_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\hazelcast\HAZELCAST_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\olric\OLRIC_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\pgcat\PGCAT_KNOWLEDGE_BASE.md`

### Optional supporting references
- `C:\Workspace\prj\jq\cashe\datafusion\DATAFUSION_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\scylladb\SCYLLADB_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\arroyo\ARROYO_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\sail\SAIL_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\curvine\CURVINE_KNOWLEDGE_BASE.md`
- `C:\Workspace\prj\jq\cashe\hikaricp\HIKARICP_KNOWLEDGE_BASE.md`

## What you must produce

Create or update this file:

- `C:\Workspace\prj\jq\cashe\hydracache\HYDRACACHE_UNIFIED_ARCHITECTURE.md`

Optionally create one extra companion file only if it clearly helps:
- `HYDRACACHE_DECISION_MATRIX.md`
- or `HYDRACACHE_PHASED_ROADMAP.md`

Prefer one strong main document over multiple weaker docs.

## What the architecture proposal must answer

### 1. Product shape
Define what `hydracache` should actually be:

- embedded library only?
- library first, daemon later?
- transparent DB-front proxy later?
- query-result cache only, or broader data-access layer?

Be explicit.

### 2. Core design decisions
You must make explicit decisions, not just discuss options, on:

- whether `sqlx` remains the compile-time source of truth
- whether cache keys are built from SQL + typed args + schema/query shape
- whether local cache should be built on `moka`, custom internals, or hybrid
- whether duplicate load suppression should be local only first, then distributed later
- whether invalidation should be explicit/tag-based first or replication-driven eventually
- whether distributed mode should look more like:
  - `groupcache`
  - `hazelcast`
  - `readyset`
  - `olric`
  - or a hybrid
- whether `noria`-style maintained results are in scope or deliberately deferred

Do not stay neutral. Choose a direction and justify it.

### 3. Crate architecture
Propose a concrete Rust crate/module structure for `hydracache`.

For example, define likely crates such as:
- API
- macros
- keying
- runtime
- local cache
- invalidation
- distributed coordination
- adapters
- telemetry
- testing

Name them and describe ownership boundaries.

### 4. Public API shape
Propose the public user-facing API shape:

- macro API
- trait API
- builder/config API
- query execution API
- invalidation API
- tagging API
- loader API

Show concrete Rust-like examples.

### 5. Runtime model
Explain how the system should work at runtime:

- lookup flow
- miss path
- local dedup path
- store path
- invalidation path
- expiration path
- eventual distributed path

### 6. Freshness and invalidation strategy
This is one of the most important sections.

You must define:
- what freshness guarantees `v0` has
- what invalidation mechanisms `v0` has
- what is intentionally not guaranteed
- what changes in later phases
- when replication-driven freshness becomes worth it
- when Noria/ReadySet-style incremental maintenance is overkill

### 7. Phased roadmap
Propose a phased implementation plan:

- Phase 0
- Phase 1
- Phase 2
- Phase 3

For each phase include:
- objective
- included capabilities
- explicitly deferred capabilities
- major risks
- reference projects most relevant to that phase

### 8. Decision matrix
For major architecture choices, provide:
- chosen option
- alternatives considered
- why rejected
- which reference project influenced the decision

### 9. Risk register
You must include a serious section on architectural risks, such as:
- invalidation correctness risk
- API over-complexity risk
- compile-time/runtime impedance mismatch
- distributed consistency risk
- operational scope creep
- trying to imitate ReadySet/Noria too early
- performance cliff from poor key design or serialization

### 10. “Steal / avoid / defer” table
Provide a table with three columns:

- steal now
- avoid for now
- defer for later

Map concrete ideas from specific projects into this table.

## Quality bar

This document should be opinionated and implementation-oriented.

Bad output:
- generic repo summaries
- neutral brainstorming
- repeating the reference docs
- “it depends” without decisions

Good output:
- explicit decisions
- concrete crate layout
- clear API direction
- phased plan
- realistic risks
- direct references to project ideas that should or should not be adopted

## Important constraints

- Do not assume that every good idea from the references belongs in `hydracache`.
- Keep the first version smaller than ReadySet, smaller than Noria, and narrower than Hazelcast.
- Favor a design that can actually be built incrementally by a small team.
- Treat `sqlx` compatibility and compile-time trust as a core strength, not an optional extra.
- Be very careful not to accidentally design a full database or full distributed stream processor.

## Output style

Write for senior engineers who will actually implement this.
Use strong structure, concrete terminology, and explicit tradeoffs.

Now read the local knowledge-base files and produce the architecture proposal.
