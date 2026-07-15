# Agentic Debugging For Raft Snapshot Failures

HydraCache treats flaky distributed failures as correctness evidence until the
contradiction is explained. A failure may be rare, seed-dependent, or only seen
in CI, but it must not be closed as environmental noise while any Raft apply,
snapshot restore, membership divergence, or invariant violation remains
unexplained.

## Contradiction Ledger

Every Raft snapshot or membership proof failure must preserve a contradiction
ledger with these fields:

- `current_hypothesis`: the working explanation being tested.
- `supporting_evidence`: facts that support the hypothesis.
- `contradicting_evidence`: facts that do not fit the hypothesis.
- `unexplained_state_machine_errors`: Raft apply, snapshot restore, membership,
  or invariant errors that still need a root cause.
- `replay_seed`: deterministic seed or scenario id.
- `schedule`: ordered replay steps, including snapshot boundary and tail replay.
- `trace_artifact`: path to the log, manifest, test, or uploaded artifact that
  replays the failure.
- `decision`: `fixed`, `explained`, or `blocked`.

If the ledger has unexplained state-machine errors or contradicting evidence,
the failure cannot be marked environmental. If the proposed fix only downgrades
or hides logs, the failure remains open; changing log level is not a correctness
fix.

## Required Artifact

At least one snapshot-membership proof must keep an executable replay manifest.
The current static fixture is:

```text
crates/hydracache-cluster-raft/tests/vectors/snapshot_replay_manifest.json
```

The fixture is validated by:

```powershell
cargo test -p hydracache-cluster-raft snapshot_replay_manifest --locked
```

When a new distributed flake appears, update or attach a manifest with the same
shape before changing code. Keep child logs, last `/admin/status` samples,
known leader/term/voter sets, and the exact env-gated command beside the
manifest whenever the failure came from a daemon or CI run.
