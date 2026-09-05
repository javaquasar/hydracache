# W14 coverage, canary and admission evidence

W14 is implemented as release machinery, not as a claim that 0.72 is already shippable. The
candidate command remains intentionally red until all external exact-candidate inputs exist.

## Implemented gates

- `management-center-check` parses the claim, source-map, coverage, canary and failure-taxonomy
  registries. It rejects duplicate or missing IDs, unsafe/missing symbols and tests, unowned routes,
  unknown canaries, incomplete status, missing evidenced receipts, partial taxonomy and coverage
  floor/module drift.
- `canary-registry-0.72.json` registers W0-W14. `canary-sweep` runs the unchanged selector first and
  then one reversible defect, requiring the exact `HC-CANARY-RED` marker and writing a clean-SHA,
  command-digest and registry-digest receipt.
- `release-evidence/0.72.toml` maps every work item to sources, executable Rust tests, artifacts,
  `fast.workspace-nextest`, and the applicable daemon/resource/coverage gates.
- Four dedicated cargo-fuzz entry points cover the management envelope, durable recovery,
  placement trace and opaque cursor decoders behind a 16 KiB input ceiling. Each is registered as
  its own ship-mandatory `tool.cargo-fuzz.management-*-072` gate and runs through `evidence-run` in
  scheduled/tag CI. The fast corpus gate replays committed valid and hostile seeds and asserts the
  oversize short-circuit; scheduled candidate runs retain four distinct time-bounded libFuzzer
  receipts, so one green decoder cannot stand in for another.
- `release-evidence --release 0.72 --require-ship` invokes strict management admission before it can
  aggregate ordinary gate receipts. The `MC72-W14-PAPER-GREEN` test proves this path cannot bypass
  missing semantic evidence.
- `cargo xtask verify` and the Linux CI Rust job run the structural management check; CI also runs
  all 15 release-scoped expected-red work-item proofs and emits the non-promoting evidence report.
- The complete dependency policy (`advisories`, `bans`, `licenses`, `sources`) is enforced rather
  than the former bans-only subset.

## Executed development proof

The following checks are green on the implementation branch:

```text
cargo test -p xtask --test management_center_072 --locked       11 passed
cargo test -p hydracache-fuzz --test fuzz_corpus_regression --locked 4 passed
cargo xtask management-center-check --release 0.72              OK
cargo xtask canary-check --release 0.72                          OK
cargo xtask canary-sweep --release 0.72 --tier fast              15 ExpectedRed
cargo test -p xtask --test release_evidence --locked             10 passed
cargo test -p xtask --test doc_check --locked                    16 passed
cargo deny check                                                 all four checks OK
```

The canary receipts are generated under `target/release-evidence/canaries/` and are intentionally
not committed: any later source commit makes them stale. The final candidate must regenerate them
from a clean checkout after the tag candidate is frozen.

## Deliberately open promotion evidence

`--require-ship` currently rejects promotion for real reasons:

1. origin contains no shipped `v0.71.x` tag/artifact, so the mandatory real 0.71/0.72 mixed-binary
   upgrade, leadership-change, peer-restart and rollback scenarios cannot be executed honestly;
2. the six-hour candidate and 24-hour ship-confirmation runs require a frozen SHA and admitted host;
3. Linux FD/RSS and full workspace LLVM coverage receipts must be generated for that same SHA;
4. the covered `bounded-resource-pressure` row still requires the dedicated Linux resource gate
   receipt; its source-tree status cannot substitute for execution;
5. implemented claims retain `status = "implemented"` until their exact-candidate evidence files are
   produced. Static source-tree success is not allowed to relabel them `evidenced`.

These are release-admission inputs, not missing feature implementations. Quiet skip, a development
branch standing in for v0.71, a dirty receipt, a stale commit, or a retry that overwrites a failed
attempt remains non-evidence.
