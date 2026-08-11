# Examples Contract

Documentation examples should stay tied to real code.

The public docs use checked Rust files under:

```text
docs-site/examples/src/bin/
```

Markdown pages include snippets from those files with mdBook include directives. This keeps the displayed code and the compiled code in one place.

## Local Checks

Build the documentation site:

```powershell
mdbook build docs-site
```

Compile the documentation examples:

```powershell
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets
```

For a complete docs publishing workflow, see [Publishing Docs](publishing-docs.md).

## Rules

- Runnable examples belong in `docs-site/examples`.
- Markdown should include snippets instead of copying them by hand.
- Use `rust,ignore` only for intentionally incomplete sketches.
- When an API changes, update the example and the page in the same branch.

This means a documentation branch is checked against the code in that branch, and the published site from `main` reflects code that has already passed review.
