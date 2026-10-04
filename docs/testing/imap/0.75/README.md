# HydraCache 0.75 Extended IMap foundation evidence

This directory is provisional, test-only foundation work. It does not enable a production
distributed IMap capability, allocate wire or durable identities, or close W0 before 0.74 is
published.

Run the bounded authority model and validate its exact-source receipt:

```powershell
cargo xtask imap-value-plane-model --release 0.75 --seed 117 --output target/imap-075/model-receipt.json
cargo xtask imap-foundation-evidence-check --release 0.75 --receipt target/imap-075/model-receipt.json
```

Execute all locally available foundation probes, write the four canonical receipts into a new
directory, and validate the set as one exact-source unit:

```powershell
cargo xtask imap-foundation-evidence-generate --release 0.75 --seed 117 --output target/imap-075/foundation
cargo xtask imap-foundation-evidence-check --release 0.75 --receipts target/imap-075/foundation
```

The generator runs the bounded authority explorer, deterministic response-loss/replay probe,
128-step seeded stateful chaos campaign, linearizability oracle, and acknowledged-owner-loss/RPO
probe. The testkit also sweeps deterministic three-node schedules for proxy routing, replication
proof, promotion, partition-scoped repair/rebalance, epoch catch-up, partial bulk and listener
overflow, plus executable guards for every threat in `security-contract.json`.

Run all locally available Rust distributed-foundation proofs and optionally retain their evidence
with one command:

```powershell
cargo xtask imap-distributed-correctness --release 0.75 --evidence target/imap-075/local-proof
```

Validate the operation/security contracts and the pinned Hazelcast source provenance:

```powershell
cargo xtask imap-contract-check --release 0.75
cargo xtask imap-hazelcast-source-check --release 0.75
```

Passing these gates means only that the foundation is internally consistent. Production routing,
backend activation, final protocol identities, performance claims, and release admission remain
blocked by the dependencies recorded in `status.json`.
