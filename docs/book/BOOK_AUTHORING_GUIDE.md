# Book Authoring Guide

> This guide explains how to grow the HydraCache book from the project knowledge base.

---

## 1. Location

The Quarto book project lives here:

- [book project](./)

The main configuration file is:

- [_quarto.yml](./_quarto.yml)

The rendered output is written to:

```text
docs/book/_book/
```

Do not commit `_book/` unless intentionally publishing generated output.

---

## 2. Build Commands

From the book directory:

```powershell
quarto render
```

Render only PDF:

```powershell
quarto render --to pdf
```

Render only HTML:

```powershell
quarto render --to html
```

Preview while writing:

```powershell
quarto preview
```

---

## 3. Source Material

Use the existing HydraCache knowledge track as raw material:

- [Knowledge process](../KNOWLEDGE_PROCESS.md)
- [Learning track](../learning/00_learning_track.md)
- [Development log](../development-log)
- [ADR directory](../adr)
- [Unified architecture](../../HYDRACACHE_UNIFIED_ARCHITECTURE.md)
- [Reference reread index](../../HYDRACACHE_REFERENCE_REREAD_INDEX.md)
- [Coerce-rs knowledge base](../../../coerce-rs/COERCE_RS_KNOWLEDGE_BASE.md)
- [Coerce-rs HydraCache reread](../../../coerce-rs/COERCE_RS_HYDRACACHE_REREAD.md)

Book chapters should not copy these files mechanically. They should synthesize them into teachable material.

---

## 4. Promotion Workflow

Use this path:

```text
development work -> development log -> learning note -> ADR/synthesis -> book chapter
```

Example:

1. During implementation, add a short note to `docs/development-log/YYYY-MM-DD.md`.
2. If a concept became clearer, update a file in `docs/learning/`.
3. If a decision became binding, create an ADR in `docs/adr/`.
4. When the concept is stable, update the matching `.qmd` chapter.

---

## 5. Chapter Rules

Each chapter should answer:

- What problem appeared while building HydraCache?
- What naive solution was tempting?
- What did reference projects teach us?
- What did HydraCache decide?
- What can another engineer reuse?

Prefer:

- concrete examples
- small diagrams
- source-backed claims
- explicit tradeoffs
- short sections

Avoid:

- dumping raw notes into chapters
- copying architecture docs verbatim
- publishing unresolved uncertainty as fact
- turning every chapter into a product manual

---

## 6. Mermaid Diagrams

Use Quarto Mermaid blocks:

````markdown
```{mermaid}
flowchart LR
  A[Request] --> B[HydraCache]
  B --> C{Hit?}
  C -->|yes| D[Return cached value]
  C -->|no| E[Run loader]
```
````

For PDF, Quarto renders Mermaid diagrams as PNG images through Chrome or Edge.

Use diagram captions when they are important to the chapter:

````markdown
```{mermaid}
%%| label: fig-cache-aside
%%| fig-cap: "Cache-aside lookup flow."
flowchart LR
  A[Request] --> B[Lookup]
  B --> C{Hit?}
```
````

---

## 7. Styling Direction

Target style:

- clean technical-book PDF
- readable code blocks
- automatic table of contents
- numbered chapters and sections
- figure captions
- enough polish for a Manning/O'Reilly-like draft, without heavy custom publishing machinery

Start with Quarto defaults. Add custom LaTeX only when a real layout problem appears.

---

## 8. File Naming

Use numbered chapter filenames:

```text
01_why-caching-is-hard.qmd
02_local-cache-core.qmd
03_invalidation.qmd
```

Appendices:

```text
appendix_reference-projects.qmd
```

Keep raw notes in `docs/learning/`, not in chapter files.
