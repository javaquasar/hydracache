# TD-0008: Networked daemon grid hosting is deferred after W6a

## Status

Open.

Owner: server / cluster runtime integration.

Candidate target: follow-up to `0.57` that completes W6b from
`docs/plans/V0_57_MANAGEMENT_CENTER_AND_OBSERVABILITY_PLAN.md`.

## Context

`0.57` W6a wires `role = "member"` in `hydracache-server` to build a grid-mode
`HydraCache::member()` with an in-process `RaftStyleMetadataControlPlane`.
Management Center and Prometheus now receive `source:"live"` from a real
member table/epoch/term instead of the old modeled `"local"` placeholder.

The networked half of W6 remains deferred: the standalone daemon does not yet
wire `hydracache-cluster-raft`, `hydracache-cluster-transport-axum`, and
`hydracache-cluster-chitchat` into `ServerRuntime` so multiple daemon processes
join over `cluster_addr`/`seeds` and report a true elected raft leader.

The explicit sentinel is:

- `crates/hydracache-server/tests/grid_host.rs::multi_node_members_form_a_cluster_and_elect_one_leader`
  is `#[ignore]` until W6b is implemented.

## Why It Is A Debt

The `source:"live"` tag is now true for a single member process, but it must not
be read as evidence that the deployable daemon has end-to-end networked cluster
formation. Without W6b, a regression in daemon-level seed discovery,
networked raft transport, TLS-bound cluster listeners, or leader handoff would
not be caught by the server package.

## Risk While Open

- Management Center can prove in-process member liveness but not daemon
  multi-node elections.
- `/cluster/overview` has `leader:null` for W6a because there is no networked
  raft soft-state leader wired into the server status handle.
- Existing networked raft adapter tests can pass while the standalone daemon
  integration remains unwired.

## Revisit Triggers

Address when one of:

- `0.57` is prepared for final release notes and the team wants the stronger
  "multi-node daemon live" claim;
- `0.58` soak/overload work needs a real multi-daemon cluster harness;
- operator lifecycle E2E starts asserting leader re-election through the
  deployed server.

## Future Definition Of Done

- `ServerRuntime` constructs the networked `RaftMetadataRuntime`,
  cluster transport listener, and chitchat discovery for `role = "member"`.
- The status handle reads member table, term, epoch, quorum, reachability,
  reshard phase, and elected leader from that networked runtime.
- `multi_node_members_form_a_cluster_and_elect_one_leader` is enabled or
  replaced with an equivalent skip-graceful network gate that drives three
  daemon runtimes over loopback.
- `local` and `client` roles remain `source:"modeled"`.

## How To Verify The Debt Can Be Removed Safely

- Run the server grid-host tests and confirm the multi-node election test is no
  longer ignored in the normal gated tier, or is covered by an explicit
  network-gated command documented in `docs/GATES.md`.
- Kill the current leader in the loopback daemon test and confirm
  `/cluster/overview` transitions from `leader:null` during election to the new
  elected id without showing a stale leader.
- Run `cargo xtask verify`.

## Related Plans

- `docs/plans/V0_57_MANAGEMENT_CENTER_AND_OBSERVABILITY_PLAN.md` (W6b)
- `docs/plans/V0_58_ENDURANCE_SOAK_AND_OVERLOAD_HARDENING_PLAN.md`
- `crates/hydracache-server/tests/grid_host.rs`
- `crates/hydracache-cluster-raft/tests/networked_raft.rs`
