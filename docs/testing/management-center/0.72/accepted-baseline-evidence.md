# Accepted Management Center 0.72 baseline evidence

The external baseline capture completed successfully in GitHub Actions run
[`35723493297`](https://github.com/javaquasar/hydracache/actions/runs/35723493297). Its immutable
artifact is
`management-baselines-072-24927c28c279c6c34ad90111ee6470b4065e0815-35723493297-1`.

The artifact binds both baseline receipts and their raw records to qualified candidate
`24927c28c279c6c34ad90111ee6470b4065e0815`. The pre-feature source is
`8d205fa302d81a07c19147cb4431e16390d256c3`; the published previous source is annotated tag
`v0.71.0`, resolved to `da8d6de409a657e0260e7fbfb4ab31d8d6ad5ca8`.

Both source checkouts and the candidate were clean. The pre-feature and published 0.71 captures
reported zero retained origin owners after quiescence, a 20,915-byte legacy bundle, 93.1142%
executed JavaScript byte-range coverage and complete browser/resource measurements. The published
0.71 capture additionally passed all 12 Playwright project/test instances.

After materializing this artifact together with the exact-candidate artifacts from full campaign
`35537203094` and ship campaign `35556487047`, the unchanged candidate passed:

```text
cargo xtask management-center-check --release 0.72 --require-evidence
cargo xtask release-evidence --release 0.72 --receipts-dir target/release-evidence/receipts --require-ship
```

The final aggregate contained 15 `ship-ready` work items, zero lower-stage work items, no reasons,
`current_worktree_dirty = false`, and `receipts_supplied = true`. No product source, threshold,
duration, rate, TTL or fault schedule changed after the long-running qualification campaigns.

The retained archive is commit
[`351766af9241399306a385298f0cc252aca6036c`](https://github.com/javaquasar/hydracache/tree/evidence/0.72/management-center-ship/docs/testing/perf-artifacts/0.72/management-center-ship-20260922)
on branch `evidence/0.72/management-center-ship`. Its `SHA256SUMS` file covers the normalized
receipts and original baseline/candidate/ship/compatibility/fuzz/coverage artifacts.
