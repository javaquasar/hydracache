# Exact current tree frontier accounting

This test-only successor closes the audit's fixed 26-input frontier: twelve
existing source/document bodies and fourteen independently inventoried supervisor
additions. It does not change protected-path selection, discover approvals,
compile old runtime text or admit a server/campaign.

`generate.py` describes the twelve inverse source deltas using literal independent
producer commits, never observed working-tree hashes. All but the main contract
reconstruct `33c54b0cda8c330b115009cdf092ad002077cedf`. The main contract first
reconstructs its exact pre-terminal body at
`37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a`; the unchanged runtime gate checks
its complete expected body before releasing the original source. Rejected
GET-owner remains disabled in the actual current product.

The descriptor SHA256 is
`4ef20240b953381c2e4dbfe7892b0a9205dbdefba3012e89a113030372d0d389`.
Current and predecessor hashes plus ordered exact inverse edits are checked.
Only selected fixed expected data is cached. Unknown paths delegate to the
unchanged reader; stale, partial, duplicated or unrelated source edits refuse.
Selected reads preserve one physical body observation, canonical root/path
checks and the old downstream gate. Narrow routing also verifies disjointness
from the existing owned-case/historical-input phases.

`additions.rs` reuses the immutable inventory544 provenance descriptor, SHA256
`fec2195d38bd7784e9c584704b90d5851f5a5c228a900513aa11cf91b42fad8f`.
All fourteen paths require regular nonsymlink/nonreparse entries, exact stage-0
mode/blob/path index rows, canonical bytes and independently fixed producer
bodies. Every invocation observes current metadata/index/body afresh. Old
9-patch/81-addition rules and checks remain byte-identical and are delegated to,
not rewritten. These sequential checks are not an atomic filesystem attestation.

The audit changes only three input joins plus one module declaration; its entire
body reconstruction and original assertion tokens are tested. Run:

```text
python tools/current-frontier-074/generate.py
cargo test --offline --locked -p xtask --test current_frontier_074 --test current_tree_input_audit_074 --test current_tree_reader_074 --test artifact_inventory_phase_544_074 -- --test-threads=4
```

`--write` creates the descriptor exclusively from the fixed producers. It cannot
refresh an existing descriptor from local discovery. This audit retains its
existing protected scope; complete package source/effect review is a separate
requirement and includes new sources outside this audit's historical scope.
No live predicate/frequency/order, time origin/deadline, dependency, threshold,
product runtime, clock, service or server action changes.
