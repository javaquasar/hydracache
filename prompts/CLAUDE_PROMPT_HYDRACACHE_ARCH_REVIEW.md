You are reviewing an existing architecture proposal for the local `hydracache` project.

Repository path:
`C:\Workspace\prj\jq\cashe\hydracache`

Primary document to review:
- `C:\Workspace\prj\jq\cashe\hydracache\HYDRACACHE_UNIFIED_ARCHITECTURE.md`

Your task is not to rewrite the document from scratch.
Your task is to act as a strict architecture reviewer and attack this proposal for hidden risks, contradictions, weak assumptions, scope leaks, and implementation traps.

## Primary goal

Stress-test the architecture in `HYDRACACHE_UNIFIED_ARCHITECTURE.md` and produce a review document that answers:

1. What is strong and should stay.
2. What is underspecified.
3. What is risky or internally inconsistent.
4. What will likely hurt implementation speed.
5. What should be simplified before coding starts.

## Required output

Create or update:

- `C:\Workspace\prj\jq\cashe\hydracache\HYDRACACHE_ARCHITECTURE_REVIEW.md`

Optionally, if necessary, also update:

- `C:\Workspace\prj\jq\cashe\hydracache\HYDRACACHE_UNIFIED_ARCHITECTURE.md`

Only make targeted edits to the architecture doc if there is a concrete correction.
Prefer putting criticism and recommendations into the review file.

## Review stance

Be skeptical, specific, and implementation-oriented.
Do not be polite at the expense of accuracy.
Do not praise generic good intentions.
Do not restate the architecture unless needed for critique.

Assume a small team will try to build this and will get punished by any vague or over-ambitious part.

## What to review

You must review the architecture against these criteria:

### 1. Scope discipline
- Does the proposal really stay library-first?
- Is there hidden proxy/daemon complexity sneaking back in?
- Are there any places where distributed ambitions contaminate Phase 0 or Phase 1?

### 2. SQLx integration realism
- Is the proposed `sqlx` integration actually realistic?
- Are the macro/API assumptions compatible with SQLx’s real compile-time model?
- Is the architecture quietly assuming more control over SQLx internals than we actually have?

### 3. Cache-key design
- Is the cache key model stable, deterministic, and future-proof?
- Is there risk of schema drift, SQL normalization mismatch, or argument-serialization bugs?
- Is the design too optimistic about query-shape equivalence?

### 4. Invalidation model
- Is explicit/tag-based invalidation enough for the proposed product shape?
- Are the guarantees honest?
- Is the invalidation API likely to become unmanageable in real applications?
- Are there silent correctness traps around partial invalidation or over-broad tags?

### 5. Moka choice
- Is using `moka` the right tradeoff?
- What do we gain?
- What control do we give up?
- Are there features the architecture assumes that `moka` does not naturally provide?

### 6. Duplicate load suppression
- Is the local single-flight plan well placed?
- Is its boundary clear relative to caching and invalidation?
- Could it accidentally create contention, head-of-line blocking, or bad failure semantics?

### 7. Distributed roadmap realism
- Is the future `groupcache`-style path actually compatible with the earlier local architecture?
- Are we deferring enough?
- Is there any part of the current design that would need a rewrite to support distributed ownership later?

### 8. API ergonomics
- Is the public API too magical?
- Too many concepts too early?
- Are users forced to understand internals they should not need to know?
- Is there a mismatch between “simple library” and the number of knobs exposed?

### 9. Crate structure
- Is the proposed crate split too fine-grained too early?
- Which crates should maybe collapse together for Phase 0?
- Which ownership boundaries are good, and which are premature abstraction?

### 10. Testing strategy
- Are the proposed guarantees testable?
- What classes of tests are missing?
- Which risks require integration tests before feature expansion?

## Required output structure

Your review file must contain:

1. `Executive Verdict`
2. `What Is Strong`
3. `High-Risk Assumptions`
4. `Contradictions Or Tension Points`
5. `Likely Implementation Traps`
6. `What To Simplify Before Coding`
7. `Recommended Architecture Corrections`
8. `Phase-by-Phase Risk Notes`
9. `Open Questions That Must Be Resolved`

## Severity model

Use severity labels:
- `Critical`
- `High`
- `Medium`
- `Low`

For every serious issue, include:
- severity
- why it matters
- what failure mode it creates
- what change would reduce the risk

## Important constraints

- Do not turn this into a rewrite proposal unless necessary.
- Do not widen scope.
- Do not suggest “build ReadySet later” style scope inflation.
- Prefer simplifying the design over making it more powerful.
- If a section is good, say so briefly and move on.

## Final instruction

Review the current architecture like an engineer who will be blamed if the team builds the wrong abstraction too early.

Now read `HYDRACACHE_UNIFIED_ARCHITECTURE.md` and produce the review.
