# HydraCache Articles

This directory contains external-facing article drafts for HydraCache.

## Project Links

- [GitHub](https://github.com/javaquasar/hydracache)
- [crates.io](https://crates.io/crates/hydracache)

## Medium Drafting

Create a Medium draft from the first article:

```powershell
node scripts/medium-draft.mjs --article docs/articles/001-why-rust-needs-cache-semantics.md
```

The script opens Medium in a persistent local browser profile, waits while you log in if needed, fills the draft, and stops before publishing.

If Playwright is not installed locally yet:

```powershell
npm --prefix console install
npx --prefix console playwright install chromium
```

## Drafts

- [001 - Why Rust Needs Cache Semantics, Not Just Another Cache Map](001-why-rust-needs-cache-semantics.md)
- [002 - Raft Snapshot Bugs, AI Agents, and the Cost of Ignoring Contradictions](002-raft-snapshot-agent-bug.md)

## Planned Articles

- 002 - Single-flight is not an optimization.
- 003 - TTL is not enough.
- 004 - Local-first distributed invalidation.
- 005 - Typed query caching in Rust.
