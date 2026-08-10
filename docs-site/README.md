# HydraCache Documentation Site

This directory contains the public mdBook documentation site for HydraCache.

The site is intentionally separate from the long-form Quarto book under `docs/book/`:

- `docs-site/` is the practical public documentation surface.
- `docs/book/` remains the longer manuscript/book track.

## Local Build

Build the site:

```powershell
mdbook build docs-site
```

Open the generated site:

```powershell
Start-Process ".\docs-site\book\index.html"
```

Preview the site locally with live reload:

```powershell
mdbook serve docs-site --open
```

## Checked Examples

Runnable documentation examples live under:

```text
docs-site/examples/src/bin/
```

Documentation pages include snippets from those files. This keeps displayed code aligned with code that compiles against the current branch.

Check the examples:

```powershell
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets
```

## First Scope

The first version focuses on:

- getting started;
- installation;
- core cache semantics;
- single-flight;
- TTL and invalidation;
- local-first distributed invalidation;
- typed query caching;
- local cache and database query caching guides;
- links to the published article series.

GitHub Pages publishing and custom domain configuration are intentionally left for a later pass.
