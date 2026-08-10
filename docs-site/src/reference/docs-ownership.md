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

## Review

A documentation PR should be reviewed for:

- correctness of keys, tags, TTL, refresh, and invalidation examples;
- whether new examples compile or are intentionally marked as non-runnable;
- local link integrity;
- mobile readability and lack of horizontal overflow;
- whether content duplicates README instead of linking to the right docs page.
