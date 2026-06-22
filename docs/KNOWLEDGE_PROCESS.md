# HydraCache Knowledge Process

> Purpose: keep a parallel learning and book-writing track while HydraCache is being built.
> This document defines where knowledge goes, when to update it, and how raw notes become durable material.

---

## 1. Why This Exists

HydraCache is both an engineering project and a learning project.

The engineering track answers:

- What are we building?
- What code should change next?
- What architecture decisions are binding?

The knowledge track answers:

- What are we learning while building it?
- Which ideas are reusable beyond this project?
- Which explanations could later become a book chapter?

These tracks should stay connected but not mixed.

---

## 2. Document Families

### Architecture

Directory:

- [architecture](./architecture)

Use for stable project design:

- architecture proposals
- risk registers
- implementation plans
- phase roadmaps

Architecture docs are allowed to be opinionated and binding.

### Development Log

Directory:

- [development-log](./development-log)

Use for chronological notes:

- what changed today
- what was confusing
- what failed
- what got clarified
- what should be revisited

Development logs are intentionally rough. They capture motion.

### ADR

Directory:

- [adr](./adr)

Use for important decisions:

- problem
- options considered
- decision
- consequences
- when to revisit

ADR files should be short and specific. If a choice affects architecture or public API, write an ADR.

### Learning

Directory:

- [learning](./learning)

Use for concept-focused notes:

- cache internals
- invalidation
- query result caching
- single-flight
- distributed coordination
- performance hot paths

Learning notes are written for understanding, not for immediate implementation.

### Book

Directory:

- [book](./book)

Use for polished synthesis:

- outline
- chapter drafts
- diagrams
- examples
- teaching narratives

Do not write the book directly from memory. Convert learning notes, ADRs, and development logs into chapters after the concepts are stable.

The book is a Quarto project. Start here:

- [Book Authoring Guide](./book/BOOK_AUTHORING_GUIDE.md)
- [Book outline](./book/00_book_outline.md)
- [Quarto config](./book/_quarto.yml)

---

## 8. Reference Project Knowledge

Primary reference index:

- [HydraCache reference reread index](../HYDRACACHE_REFERENCE_REREAD_INDEX.md)

Actor/distributed runtime reference:

- [Coerce-rs knowledge base](../../coerce-rs/COERCE_RS_KNOWLEDGE_BASE.md)
- [Coerce-rs HydraCache reread](../../coerce-rs/COERCE_RS_HYDRACACHE_REREAD.md)

Use reference project notes as source material for learning notes, ADRs, architecture revisions, and book chapters.

---

## 3. Update Rhythm

Use this rhythm during development:

1. During work: update architecture docs only when decisions or contracts change.
2. After meaningful work: add a short development log entry.
3. After a decision: create or update an ADR.
4. After studying a concept: add or refine a learning note.
5. After a phase or milestone: update the book outline or draft a chapter section.

Default rule:

- If it helps build the project, put it in architecture or ADR.
- If it helps remember the journey, put it in development-log.
- If it helps explain the topic, put it in learning.
- If it is polished enough to teach, put it in book.

---

## 4. Naming Rules

Development logs:

```text
YYYY-MM-DD.md
```

ADR:

```text
ADR-0001-short-kebab-title.md
ADR-0002-short-kebab-title.md
```

Learning notes:

```text
local-cache-core.md
cache-invalidation.md
query-result-caching.md
distributed-cache-coordination.md
performance-hot-path.md
```

Book chapters:

```text
00_book_outline.md
01_why-caching-is-hard.md
02_local-cache-core.md
03_invalidation.md
```

---

## 5. Writing Rules

Prefer:

- concrete examples
- diagrams when they clarify flow
- references to source files or reference projects
- explicit tradeoffs
- short summaries at the top

Avoid:

- vague enthusiasm
- repeating architecture docs word-for-word
- hiding uncertainty
- mixing raw notes and polished book text in the same file

Mark uncertainty clearly:

```text
Confirmed:
Inference:
Open question:
```

---

## 6. Book Pipeline

The book track should follow this path:

```text
development work -> development log -> learning note -> synthesis -> chapter draft
```

This keeps the book grounded in real engineering work instead of abstract advice.

Every future chapter should answer:

- What problem appeared in the project?
- What naive solution was tempting?
- What did reference projects teach us?
- What decision did HydraCache make?
- What can another engineer reuse?

---

## 7. Initial Book Theme

Working title:

```text
Building a Local-First Distributed Cache in Rust
```

Working promise:

```text
Use the construction of HydraCache to explain local caching, invalidation,
query-result caching, duplicate-load suppression, and distributed cache coordination.
```

The book should not be a manual for HydraCache only. It should teach the ideas behind the project.
