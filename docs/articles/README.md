# HydraCache Articles

This directory contains external-facing article drafts for HydraCache.

## Project Links

- [GitHub](https://github.com/javaquasar/hydracache)
- [crates.io](https://crates.io/crates/hydracache)

## Published Articles

- 2026-07-31 - [001 - Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191) - Medium
- 2026-08-05 - [002 - Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d) - Medium

## Medium Drafting

Create a Medium draft from the first article:

```powershell
node scripts/medium-draft.mjs --article docs/articles/001-why-rust-needs-cache-semantics.md
```

The script opens Medium in a persistent local browser profile, waits while you log in if needed, fills the draft, and stops before publishing.

If Medium login does not respond in the default Playwright Chromium window, retry with the installed Chrome channel:

```powershell
node scripts/medium-draft.mjs --channel chrome --profile .playwright/medium-chrome-profile --article docs/articles/001-why-rust-needs-cache-semantics.md
```

If Medium blocks automated browsers with a security verification, copy the article as rich HTML and paste it into a signed-in Firefox or Chrome editor:

```powershell
node scripts/medium-draft.mjs --clipboard --article docs/articles/001-why-rust-needs-cache-semantics.md
```

For a more reliable Medium paste, copy the title and body separately. Upload the cover image manually:

```powershell
node scripts/medium-draft.mjs --clipboard-title --article docs/articles/001-why-rust-needs-cache-semantics.md
node scripts/medium-draft.mjs --clipboard-body --article docs/articles/001-why-rust-needs-cache-semantics.md
```

Article drafts should include a short series/resources block near the top with the current publication state, planned series entries, GitHub, and crates.io links. Bare `https://` links are converted to clickable links by the Medium draft script.

Draft and planned entries stay unnumbered in generated series blocks until they are published. Running `--set-url` records the public URL and promotes that draft to the next numbered series part.

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

## Article Sources

- [001 - Why Rust Needs Cache Semantics, Not Just Another Cache Map](001-why-rust-needs-cache-semantics.md)
  - Cover: [001-why-rust-needs-cache-semantics-cover.png](001-why-rust-needs-cache-semantics-cover.png)
  - Prompt: [001-why-rust-needs-cache-semantics-cover.prompt.md](001-why-rust-needs-cache-semantics-cover.prompt.md)
- [002 - Single-flight Is Not an Optimization](002-single-flight-is-not-an-optimization.md)
  - Cover: [002-single-flight-is-not-an-optimization-cover.png](002-single-flight-is-not-an-optimization-cover.png)
  - Prompt: [002-single-flight-is-not-an-optimization-cover.prompt.md](002-single-flight-is-not-an-optimization-cover.prompt.md)
- [Draft - Raft Snapshot Bugs, AI Agents, and the Cost of Ignoring Contradictions](002-raft-snapshot-agent-bug.md)
  - Cover: [002-raft-snapshot-agent-bug-cover.jpg](002-raft-snapshot-agent-bug-cover.jpg)

## Planned Articles

- Planned - TTL is not enough.
- Planned - Local-first distributed invalidation.
- Planned - Typed query caching in Rust.
