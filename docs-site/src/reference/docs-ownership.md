# Docs Ownership

The public docs site is the primary reader-facing documentation surface.

## Rules

- `README.md` stays short and points readers into `docs-site`.
- Public docs should be self-contained; Medium articles are linked only from the home page as background.
- Runnable snippets belong in `docs-site/examples` and are included into Markdown.
- Branch docs compile against branch code.
- Playground run buttons stay disabled until snippets target published crates.io versions.
- Brand assets live under `docs-site/src/assets/brand`.
- Architecture diagrams live under `docs-site/src/assets/diagrams` as checked static assets, not CDN-rendered runtime dependencies.
- Canonical release notes live under `docs/releases`; `docs-site/src/releases` contains only mdBook
  include wrappers and the release index.
- Public long-form material that already has a canonical file under `docs/` is exposed through an
  mdBook include wrapper instead of being copied and edited independently.
- Files included from outside `docs-site/src` use absolute public links. Relative links would be
  interpreted from the generated page rather than from the canonical source file.

## Review

A documentation PR should be reviewed for:

- correctness of keys, tags, TTL, refresh, and invalidation examples;
- whether new examples compile or are intentionally marked as non-runnable;
- local link integrity;
- mobile readability and lack of horizontal overflow;
- whether content duplicates README instead of linking to the right docs page.
- whether every release from 0.71 onward has an exact include wrapper and navigation entry.
