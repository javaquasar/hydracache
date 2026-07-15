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

Article drafts should include a short series/resources block near the top with the current part number, planned series entries, GitHub, and crates.io links. Bare `https://` links are converted to clickable links by the Medium draft script.

Refresh the generated series block after changing the series manifest at [hydracache-runtime-series.json](hydracache-runtime-series.json):

```powershell
node scripts/update-article-series.mjs --article docs/articles/001-why-rust-needs-cache-semantics.md
```

After publishing an article, save its public URL in the series manifest and refresh the block:

```powershell
node scripts/update-article-series.mjs --article docs/articles/001-why-rust-needs-cache-semantics.md --set-url https://medium.com/your-published-url
```

Future article drafts use that URL to link back to previous parts.

If Playwright is not installed locally yet:

```powershell
npm --prefix console install
npx --prefix console playwright install chromium
```

## Drafts

- [001 - Why Rust Needs Cache Semantics, Not Just Another Cache Map](001-why-rust-needs-cache-semantics.md)
  - Cover: [001-why-rust-needs-cache-semantics-cover.png](001-why-rust-needs-cache-semantics-cover.png)
  - Prompt: [001-why-rust-needs-cache-semantics-cover.prompt.md](001-why-rust-needs-cache-semantics-cover.prompt.md)
- [002 - Raft Snapshot Bugs, AI Agents, and the Cost of Ignoring Contradictions](002-raft-snapshot-agent-bug.md)
  - Cover: [002-raft-snapshot-agent-bug-cover.jpg](002-raft-snapshot-agent-bug-cover.jpg)

## Planned Articles

- 003 - Single-flight is not an optimization.
- 004 - TTL is not enough.
- 005 - Local-first distributed invalidation.
- 006 - Typed query caching in Rust.
