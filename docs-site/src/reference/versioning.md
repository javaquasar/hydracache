# Versioning

The published docs should describe the released crate line that readers can install from crates.io.

## Branches

`main` is the source of truth for the next published docs site. Documentation branches should compile examples against the code in the same branch.

## Releases

When a crate release is published:

- update release notes under `docs/releases`;
- make sure docs.rs links point at `latest` unless documenting a version-specific behavior;
- build `docs-site` from the same commit that passed release checks;
- publish the GitHub Pages site from that reviewed state.

## URLs

The repository GitHub Pages path is expected to be:

```text
https://javaquasar.github.io/hydracache/
```

If a custom domain is added later, keep `/hydracache/` compatibility or add redirects so existing article and README links do not break.

## Examples

Public examples in `docs-site/examples` are branch-coupled. A documentation PR should fail if examples no longer compile against that branch.

Rust Playground examples are different: they run against published crates, not the current branch. Only enable runnable playground blocks for APIs that exist in the latest published crate version, or pin the docs to a version-specific target.
