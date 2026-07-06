# TD-0010: Cluster transport has no TLS termination and no peer auth

## Status

Open.

Owner: server / cluster transport security.

Candidate target: `0.60.0` Networked Grid Hardening (W1/W2).

## Context

`0.59` wired the networked member grid, but the cluster transport listener that
carries raft messages between daemons is plaintext and unauthenticated:

- `spawn_cluster_transport` (`crates/hydracache-server/src/grid_host.rs:258-304`)
  validates only the `TlsStartupPolicy` *posture* (fail-loud non-loopback
  without TLS, `hydracache-cluster-transport-axum/src/tls.rs:121-152`) and then
  serves **plaintext** `axum::serve` on `cluster_addr` (grid_host.rs:299). No
  rustls acceptor exists anywhere in the workspace.
- The outbound `HttpRaftMessageSink` always posts `http://`
  (grid_host.rs:494-497), regardless of `config.tls`.
- The raft route is wired with `ClusterRouteAuth::missing_provider()`
  (grid_host.rs:274-276) — no node identity provider. The `0.48` credential
  seam (`NodeIdentityProvider` / `StaticNodeIdentityProvider`,
  `hydracache-cluster-transport-axum/src/lib.rs:246/259`) is shipped but not
  wired into the daemon.

## The Broken Configuration (worse than "missing")

The auth boundary is built as
`missing_provider().acknowledge_insecure_trust_boundary(!tls.enabled || tls.acknowledge_insecure)`
(grid_host.rs:274-276). With `tls.enabled = true` and
`acknowledge_insecure = false`, `ClusterRouteAuth::verify` rejects **every**
inbound raft message as unauthenticated
(`hydracache-cluster-transport-axum/src/lib.rs:451-458`), while outbound is
still plaintext `http://`. A TLS-configured multi-node member cluster therefore
**cannot exchange raft messages at all** — it fails silently as election
timeouts, not loudly at startup (violates R-3 in spirit).

The `0.59` test `member_cluster_listener_uses_configured_tls`
(`crates/hydracache-server/tests/grid_host.rs:179-191`) does not catch this: it
starts a single-node member (no peers, no transport traffic) with cert paths
that do not exist on disk, and asserts only that the daemon starts.

## Why It Is A Debt

The `0.59` plan's gate wording "Cluster listener is **TLS-bound** when
configured" overstates what shipped (the release manifest theme was already
softened to "TLS policy remains fail-loud"). Until termination + peer auth
exist, any non-loopback deployment relies entirely on the network boundary, and
a TLS-configured deployment is broken outright.

## Risk While Open

- Any process that can reach `cluster_addr` can inject raft messages
  (elections, appends) into a member's runtime.
- Operators who set `tls.enabled = true` on members get a cluster that forms
  only as isolated single nodes, with no loud startup error naming the cause.
- The shipped `0.48` "in-transit mTLS" claim does not extend to the daemon's
  cluster listener; the honesty note lives only in the release theme.

## Revisit Triggers

- `0.60.0` starts implementation;
- any deployment guide recommends non-loopback member clusters;
- the operator (`0.56`) starts provisioning cluster-transport certificates.

## Future Definition Of Done

- Inbound raft route requires a verified peer identity
  (`ClusterRouteAuth::secure`) when auth material is configured; the
  `tls.enabled && !acknowledge_insecure` dead-end is impossible (either the
  transport is actually secured, or startup fails loud naming the gap).
- The cluster listener terminates TLS with the configured cert/key when
  `tls.enabled = true`; `HttpRaftMessageSink` speaks `https://` with the
  configured CA, and peer identity is enforced by the W1 credentialed route.
  Client-certificate mTLS is a named future extension that needs its own config
  surface and tests.
- A falsifiable test proves a plaintext client is **rejected** by a TLS-enabled
  listener, and a TLS-enabled pair exchanges raft messages end-to-end.
- Loopback development without TLS keeps working unchanged (R-10).

## Related

- `docs/plans/V0_60_NETWORKED_GRID_HARDENING_PLAN.md` (W1/W2)
- `docs/plans/V0_59_NETWORKED_DAEMON_GRID_HOSTING_PLAN.md` (W4 wording)
- `crates/hydracache-server/src/grid_host.rs`
- `crates/hydracache-cluster-transport-axum/src/lib.rs`
- `crates/hydracache-cluster-transport-axum/src/tls.rs`
