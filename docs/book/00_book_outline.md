# Building a Local-First Distributed Cache in Rust

> Working book outline. This is a living synthesis target, not a commitment to chapter titles.
> The active Quarto book project starts at [index.qmd](./index.qmd).

Authoring guide:

- [Book Authoring Guide](./BOOK_AUTHORING_GUIDE.md)

## Promise

Use the construction of HydraCache to explain local caching, invalidation, query result caching, duplicate-load suppression, and distributed cache coordination.

The book should teach transferable systems ideas through one concrete Rust project.

---

## Draft Outline

1. Why Caching Is Harder Than It Looks
2. Local Cache Core
3. Keys, Values, TTL, and Eviction
4. Invalidation Is The Real Problem
5. Query Result Caching
6. Compile-Time SQL and Typed Adapters
7. Duplicate Load Suppression
8. From Local Cache To Cluster Synchronization
9. Client And Member Modes
10. Freshness Guarantees And Failure Modes
11. Performance Discipline
12. Building HydraCache In Phases

---

## Source Material

Use these inputs before drafting chapters:

- [Knowledge process](../KNOWLEDGE_PROCESS.md)
- [Learning track](../learning/00_learning_track.md)
- [Development log](../development-log)
- [ADR directory](../adr)
- [Unified architecture](../../HYDRACACHE_UNIFIED_ARCHITECTURE.md)
- [Reference reread index](../../HYDRACACHE_REFERENCE_REREAD_INDEX.md)

---

## Chapter Template

Each chapter should answer:

- What problem did we hit?
- What naive solution looked tempting?
- What did reference projects teach us?
- What did HydraCache decide?
- What can another engineer reuse?
