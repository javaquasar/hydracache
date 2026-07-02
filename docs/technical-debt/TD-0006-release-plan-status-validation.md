# TD-0006: Release-plan header status is not validated against the manifest

## Status

**Resolved** (2026-07).

Owner: docs / release tooling (`cargo xtask`).

Resolution: `cargo xtask doc-check` now validates each manifest plan's strict
`**Status:**` header against `releases.toml` (`check_plan_header_status` /
`parse_status_header` in `crates/xtask/src/doc_check.rs`, covered by unit tests);
the four stale headers (`V0_50_DEMO_ENHANCEMENTS`, `V0_52`, `V0_53`, `V0_53_1`)
were backfilled; `cargo xtask verify`/`doc-check` are green. Plans that use a
prose status (idea-capture drafts) are intentionally skipped. Kept for history.

## Context

`cargo xtask doc-check` validates that `docs/plans/releases.toml` and
`docs/plans/INDEX.md` agree (versions, files, statuses). It does **not** parse the
per-plan `> - **Status:** …` header inside each `V0_*_PLAN.md`. As a result the
in-plan header can silently drift from the manifest.

Observed drift at the time of writing (manifest says shipped/superseded, header
still says `**Status:** planned`):

- `docs/plans/V0_50_DEMO_ENHANCEMENTS_PLAN.md` (manifest: superseded)
- `docs/plans/V0_52_IMAP_AND_FENCED_LOCK_JAVA_SURFACE_PLAN.md` (manifest: shipped)
- `docs/plans/V0_53_INTERACTIVE_CLUSTER_LAB_PLAN.md` (manifest: shipped)
- `docs/plans/V0_53_1_REAL_RAFT_ELECTION_IN_THE_LAB_PLAN.md` (manifest: shipped)

`doc-check` reports "OK" despite these, because header status is outside its
checks. This is the same class of drift that has repeatedly required manual
header edits when flipping a release to `shipped`.

## Why It Is A Debt

A reader who opens a shipped plan and sees `**Status:** planned` is misinformed;
it undermines the manifest as the single source of truth and erodes trust in the
roadmap. The fix is cheap and mechanical, but until the gate exists the drift
will keep recurring on every release flip.

## Risk While Open

- Stale `planned` headers on shipped work mislead readers.
- Each release flip must remember to hand-edit the header (easy to miss).
- No automated signal that a header and the manifest disagree.

## Revisit Triggers

Address when either:

- a new release is flipped to `shipped`/`in-progress` (fold the header check in as
  part of the flip), or
- `doc-check` is next touched for any reason.

## Future Definition Of Done

- Extend `crates/xtask/src/doc_check.rs` so `doc-check` parses each plan file's
  `> - **Status:** <value>` header and asserts it equals the `status` recorded for
  that `file` in `releases.toml` (map the header vocabulary — `planned`,
  `in-progress`, `shipped`, `superseded`, `draft` — one-to-one).
- Add an `xtask` test analogous to the existing verify-gate tests so the check is
  itself covered.
- Backfill the four stale headers above so `doc-check` is green.
- `cargo xtask verify` green (doc-check now includes header validation).

## How To Verify The Debt Can Be Removed Safely

- Introduce a deliberate mismatch (e.g. set one shipped plan's header to
  `planned`) and confirm `doc-check` fails loudly; revert and confirm it passes.
- Grep all `docs/plans/V0_*_PLAN.md` headers and confirm each matches its
  manifest status.

## Related Plans

- `docs/plans/releases.toml`, `docs/plans/INDEX.md`
- `crates/xtask/src/doc_check.rs`, `crates/xtask/src/verify.rs`
