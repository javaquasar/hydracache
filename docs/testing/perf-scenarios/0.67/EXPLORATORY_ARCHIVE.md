# HydraCache 0.67 exploratory archive

This index separates the knowledge retained from the 2026-08-01 through
2026-08-04 server campaign from release qualification and bootstrap evidence.
The reports are useful for engineering decisions and for designing the next
rental, but they do not satisfy a release gate.

## Immutable identity

| Field | Value |
|---|---|
| Annotated archive tag | `explore-0.67-telemetry-20260803` |
| Exact archived commit | `dbc2f82f7f303528b3cca7842818730c82232b9c` |
| Split-PR base | `f2567ee` (`origin/main` when the archive was split) |
| Changed paths in archive versus that base | 5,535 |
| Paths under `results/` | 5,469 |
| Markdown reports/manifests | 34 |
| Full tree | [archive tag](https://github.com/javaquasar/hydracache/tree/explore-0.67-telemetry-20260803) |
| Raw result tree at exact commit | [raw archive](https://github.com/javaquasar/hydracache/tree/dbc2f82f7f303528b3cca7842818730c82232b9c/results) |

The annotated tag is a convenient name. The 40-character commit is the
authoritative identity and must be checked before using archived data.

## Evidence classification

- The last fully verified pre-fix qualification referenced exact main
  `1ce50cb455742395d303f46cb81866efa513c664`, run `30613577155`, job
  `91101596670`, artifact `8786642124`, runner fingerprint
  `55503d33d6592cb062ecfcd289fa67cad93d123ee9657312172615da67a26238`,
  and storage digest
  `6e9320c0e7c4968670961ee94972fb16bee054ec3bedba2f2ee19f75e9dbb35c`.
- Bootstrap run `30614325548`, job `91103917615`, failed the W4B/W5C manifest
  identity check. Artifact `8787545365` was retained unchanged for diagnosis,
  but the run did not count as a bootstrap sample.
- The producer-side canonical-manifest fix subsequently reached `main` as
  `f2567ee`. The rented server was stopped before that new exact main could be
  requalified and before five accepted, serialized, same-fingerprint bootstrap
  samples could be collected.
- Some archived directories are labelled `accepted` within an exploratory
  stage. That means their local exploratory noise checks passed; it does not
  promote them to qualification, bootstrap, SLO, release, or product-ranking
  evidence.
- Failed, cancelled, incomplete, and IRQ-contaminated attempts remain in the
  archive because they document harness defects and host-noise failure modes.
  They must not be averaged into accepted exploratory summaries.

## Curated material in the normal repository

The repository keeps human-sized methodology, analysis, and conclusion files:

- [host preparation and measurement report](exploratory-preparation-and-measurement-report.md);
- [relative eight-case methodology](relative-eight-cases-methodology.md) and
  [telemetry extension](relative-eight-cases-telemetry.md);
- [six development experiments](development-experiments.md);
- [memory investigations](memory-investigations.md) and
  [leak/retention stage](memory-leak-stage.md);
- [expanded metrics stage](metric-expansion-stage.md);
- [future cluster and resilience plan](cluster-resilience-testing-plan.md);
- [curated result index](results/README.md).

The raw CSV, JSONL, logs, container inspection data, receipts, and rejected-run
diagnostics stay in the archive rather than inflating ordinary clones and every
future pull request.

## Reproducing or auditing the archive

```bash
git fetch origin --tags
git worktree add --detach ../hydracache-exploratory-archive \
  explore-0.67-telemetry-20260803
cd ../hydracache-exploratory-archive
test "$(git rev-parse HEAD)" = \
  dbc2f82f7f303528b3cca7842818730c82232b9c
git status --short
```

Use the exact source commit, pinned image digests, host receipt, affinity, case
ordering, request count, keyspace, payload, pipeline, and client counts recorded
by each report. Do not compare samples across a changed host fingerprint or
changed freeze receipt. Run the exploratory harness separately from
qualification/bootstrap and never weaken the release gates to make an
exploratory result pass.

The reusable host automation is reviewed in
[#65](https://github.com/javaquasar/hydracache/pull/65), and the extracted
measurement harness is reviewed in
[#66](https://github.com/javaquasar/hydracache/pull/66). The archive remains
self-contained even if those draft pull requests change during review.
