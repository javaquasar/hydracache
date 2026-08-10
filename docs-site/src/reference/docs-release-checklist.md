# Docs Release Checklist

Use this checklist before publishing a crate release or updating the public docs site.

## Content

- README still reads as a short project entry point.
- Crate Map reflects all public crates and adapter names.
- API Links point to the correct docs.rs crates.
- Versioning page matches the release branch and GitHub Pages target.
- Adapter pages match current SQLx, Diesel, and SeaORM helper names.
- Production Checklist and Anti-patterns still describe current behavior.

## Checks

```powershell
cargo fmt --manifest-path docs-site/examples/Cargo.toml --check
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets --locked
mdbook build docs-site
node scripts/docs-link-check.mjs
node scripts/docs-visual-smoke.mjs
```

## Publishing

- Build docs from the reviewed commit.
- Publish `docs-site/book` through the GitHub Pages workflow.
- Confirm no other workflow replaces the docs Pages artifact for the same release.
- Confirm favicon, logo, static diagrams, search, and mobile navigation after deploy.
- Keep article links on the home page only.
