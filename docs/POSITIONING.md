# HydraCache — Positioning

Why HydraCache is interesting on the market, what it has that others don't, who it is
for, and where it is honestly weak. Use this to answer "why not just use Redis?" in the
README, pitches, and design reviews.

Companion analysis: [`COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](COMPETITIVE_ANALYSIS_AND_EVOLUTION.md)
(cluster layer) and [`STORAGE_AND_DATA_PLATFORM_EVOLUTION.md`](STORAGE_AND_DATA_PLATFORM_EVOLUTION.md)
(storage/data-platform). Invariants: [`docs/RULES.md`](RULES.md).

## The wedge (one sentence)

> **An embeddable, DB-query-result-aware, correctness-first Rust cache that grows from
> an in-process library into a distributed cache grid — on one codebase.**

HydraCache does not try to beat Redis/Dragonfly at raw throughput or match
Hazelcast/Ignite on maturity. Its interest is the **intersection** of properties that
no single competitor offers together.

## What's different (and who else does / doesn't)

### 1. DB-query-result-aware invalidation, embedded in the Rust process

Most caches are opaque key→blob stores that know nothing about the database behind
them; staleness is handled by TTLs or manual deletes. HydraCache ties a cache entry to
its **SQL dependencies** and invalidates through a transactional outbox + CDC +
generated DB hooks, with a **SQL-dependency lint** that tells you at build time which
cache keys a query touches (`hydracache-sql-lint`, releases `0.37`/`0.38`).

- **ReadySet** does incremental view maintenance, but as a heavyweight network **proxy**
  in front of the DB — a different operational model.
- **Hazelcast / Infinispan** offer a second-level cache, but **JVM-only**.
- **Redis / moka / foyer** are DB-agnostic; correctness is the caller's problem.
- **HydraCache** is the only **Rust-native, embeddable, DB-aware** option with
  "assisted correctness, not magic, not a proxy" (RULES R-9).

### 2. Correctness as a product feature (explicit, tunable, checkable)

The consistency contract is a first-class, auditable artifact, not marketing:
boolean release gates with no numeric self-scores, fail-loud / never-silently-degrade
(R-3), versioned tombstones with repair-gated GC, per-operation consistency levels,
a deterministic seeded fault model (R-5), and honest hard non-goals (no distributed
transactions, R-2). The governance itself is executable in CI (`RULES.md`, `GATES.md`,
`cargo xtask doc-check`). The browser-facing
[`cluster simulator demo`](../demo/README.md) and
[`GitHub Pages build`](https://javaquasar.github.io/hydracache/) show that same
seeded engine and invariant checker to humans without replacing the release
gates. A cache you can *reason about and audit* is rare.

### 3. One codepath: embedded → cluster → geo

`moka`/`foyer` are embedded-only; Redis/Hazelcast are servers you run separately.
HydraCache spans **in-process cache → distributed grid → geo/active-active** on the
same library, opt-in, preserving embedded behavior byte-for-byte (R-10). Start as a
Rust in-process cache, scale to a grid without switching products.

### 4. A Rust-native data-platform trajectory

On top of a pluggable storage trait, optional feature crates can add SQL (DataFusion),
vectors/ANN (HNSW + quantization), and change-streams — the Hazelcast multi-modal idea,
but **embeddable and in Rust**, which nothing on the market offers. Aspirational, but
the direction is unique (see the storage companion doc).

## Target audience (where it wins first)

**Rust services backed by a SQL database** that need a correct cache with minimal
surprises — not another Redis. The DB-integrated invalidation + embeddability + the
"grows into a grid" path is most valuable to teams who today hand-roll TTL guesses or
fragile manual cache busting around `sqlx`/`diesel`/`seaorm`.

## What HydraCache is NOT (honest weaknesses)

- **Not a Redis throughput replacement.** Different goal; do not pitch on raw ops/sec.
- **Not yet production-deployable as a server.** No standalone daemon, in-transit
  encryption, or external client protocol yet (see prod-readiness gaps).
- **Distributed layer is young.** The 0.43 debt-closure gates now validate
  multi-node/zone behavior over real networked transport seams, but the project still
  needs production wrapping: server packaging, security, external protocols,
  operations, and longer-running soak history.
- **Pre-1.0.** API is still moving; no stability/semver commitment yet.
- **Not a database.** SQL/vector are read-only / opt-in modules; transactions are a
  permanent non-goal (R-2).

So "interesting on the market" is today a **design and niche bet** with a now-validated
distributed core, but it becomes a real deployment advantage only after the production
wrapping (server, security, external protocol, and operating model) lands.

## "Why not …" quick answers

| Alternative | Why HydraCache instead |
| --- | --- |
| Redis / Valkey / Dragonfly | They're DB-agnostic servers tuned for throughput; HydraCache is an embeddable, DB-query-aware cache with built-in invalidation correctness. Use Redis when you want a fast remote blob store; HydraCache when correctness vs your SQL DB matters in-process. |
| moka / foyer (Rust local caches) | Excellent but embedded-only and DB-agnostic. HydraCache adds DB-aware invalidation and a path to a distributed grid. |
| Hazelcast / Infinispan / Ignite | Mature JVM data grids. HydraCache is the Rust-native, embeddable answer with explicit, auditable consistency — no JVM, no separate cluster product to operate for the embedded case. |
| ReadySet | A DB proxy doing incremental view maintenance. HydraCache is a library, not a proxy: no extra network hop, assisted (not transparent) correctness, opt-in. |
| TiKV / ScyllaDB / qdrant | Full databases (KV / wide-column / vector). HydraCache is a *cache* with DB integration, not a system of record; it borrows their distributed/storage patterns (see analysis docs) without becoming a DB. |

## Maintaining the position

Every release must protect the wedge: keep the core a lean cache, keep correctness
checkable (gates), keep richer capabilities opt-in (R-10), and never quietly become a
database (R-2/R-9). The differentiation is the *combination*; diluting any one pillar
(DB-awareness, correctness discipline, embeddability, the embedded→grid continuum)
weakens the whole story.
