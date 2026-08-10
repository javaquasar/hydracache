# Publishing Docs

The public documentation site lives under `docs-site`.

## Local Preview

From the repository root:

```powershell
mdbook serve docs-site --hostname 127.0.0.1 --port 3000
```

Open:

```text
http://127.0.0.1:3000
```

## Build

```powershell
mdbook build docs-site
```

The generated site is written to:

```text
docs-site/book
```

## Example Checks

Documentation snippets are included from checked Rust files:

```powershell
cargo fmt --manifest-path docs-site/examples/Cargo.toml --check
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets --locked
node scripts/docs-link-check.mjs
```

When an API changes, update the Rust example and the Markdown page in the same branch.

## Rust Playground

The Rust Playground run button is disabled in `book.toml`:

```toml
[output.html.playground]
runnable = false
copyable = true
```

This is deliberate. The public docs include snippets from checked examples that use local path dependencies, while Rust Playground can only run against published crates. Keep the run button disabled until the runnable examples target a published crate version and each runnable block is a complete standalone program.

## Visual Smoke

With `mdbook serve` running:

```powershell
node scripts/docs-visual-smoke.mjs
```

The smoke test opens the home page, architecture page, production checklist, database guide, and API links page on desktop and mobile viewports. It checks that content is present, the home logo is visible, and pages do not introduce horizontal overflow.

## Assets

Brand assets live under:

```text
docs-site/src/assets/brand
```

Use the `*-256.png` asset in pages and README content. Keep the original PNG as the source asset.

## Publishing Target

The intended production target is GitHub Pages, optionally behind a custom domain later. The `book.toml` `site-url` is set to `/hydracache/`, which matches the repository GitHub Pages path.
