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
mdbook serve docs-site --hostname 127.0.0.1 --port 3000
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

Check local links:

```powershell
node scripts/docs-link-check.mjs
```

Run a browser smoke test while `mdbook serve` is running:

```powershell
node scripts/docs-visual-smoke.mjs
```

## Rust Playground

The mdBook Rust Playground run button is intentionally disabled for now:

```toml
[output.html.playground]
runnable = false
copyable = true
```

Most public snippets are included from `docs-site/examples`, which compile against the current branch through local path dependencies. Rust Playground can only run examples against published crates, so enabling the run button should wait until the documented APIs are published and the runnable snippets are written as complete standalone programs.

To enable it later:

- publish the required `hydracache*` crate versions to crates.io;
- keep runnable snippets aligned with the latest published version;
- add hidden `main`/setup code to every runnable block;
- keep branch-only snippets checked through `docs-site/examples`;
- run `cargo check`, `mdbook build`, link check, and browser smoke before publishing.

## Current Scope

The public docs now include:

- getting started;
- installation;
- architecture;
- use cases;
- decision guide;
- core cache semantics;
- single-flight;
- TTL and invalidation;
- local-first distributed invalidation;
- typed query caching;
- local cache, typed cache, refresh/stale reads, database query caching, diagnostics, and cluster guides;
- crate map, workspace layout, API links, quality gate, publishing workflow, and checked examples contract;
- links to the published article series on the main page only.

GitHub Pages publishing and custom domain configuration are tracked in `src/reference/publishing-docs.md`.
