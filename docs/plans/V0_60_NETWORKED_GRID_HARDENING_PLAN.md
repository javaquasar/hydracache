# HydraCache 0.60.0 Networked Grid Hardening — Codex Execution Plan

> **At a glance**
> - **What:** harden what `0.59` shipped so the networked daemon grid is **securable, resizable, and
>   honest under load**: (**W1**) peer auth on the cluster raft route (today
>   `ClusterRouteAuth::missing_provider()`, grid_host.rs:274 — and the `tls.enabled` multi-node path
>   is a silent dead-end); (**W2**) real **rustls TLS termination** on the cluster listener + an
>   `https://` outbound raft sink (today plaintext `axum::serve`, grid_host.rs:299, and hardcoded
>   `http://`, grid_host.rs:494) — closing **TD-0010**; (**W3**) **persistent node identity**
>   decoupled from `cluster_addr`; (**W4**) **dynamic raft membership** — `ConfChange` voter
>   add/remove, drain-removes-voter, quorum counted against the raft `ConfState`; full late-start
>   daemon join bootstrap remains a named TD-0011 residual; (**W5**) honest proposal status on non-leaders (no more `Committed` for a merely
>   forwarded proposal, lib.rs:1151); (**W6**) drive-loop/status-path hardening (no swallowed
>   errors, bounded discovery journal, O(1) reachability); (**W7**) the multi-daemon proofs +
>   a CI tier for the `0.59` E2E that currently runs **only by hand**, plus the TD-0009 coverage
>   baseline re-measure; (**W8**) docs/gates/ledger closure.
> - **Why:** `0.59` made the deployable daemon host a real networked grid, but audited against the
>   code it is **loopback-grade**: the raft route is unauthenticated plaintext; a TLS-configured
>   member cluster cannot exchange raft messages at all (inbound rejected as unauthenticated,
>   outbound still `http://`); the voter set is frozen at startup from `seeds` (no `ConfChange` —
>   the `0.56` operator can scale pods but not the quorum); a drained member never leaves the raft
>   voter set while `has_quorum()` counts *metadata* members — two quorum planes that disagree;
>   identity is the listen address, so an IP change orphans the durable raft log. None of this is
>   new consensus — it is the hardening the `1.0` "production-ready cluster out of the box" claim
>   requires.
> - **After (depends on):** `0.59.0` (the networked member stack in `grid_host.rs`, the W1b drive
>   seam `tick`/`step`/`drain_ready`/`take_outbound_messages`, lib.rs:712-744, the loopback
>   3-daemon E2E), `0.48` (the `NodeIdentityProvider` credential seam + `TlsVerifier`), `0.56`
>   (operator scale lifecycle that W4 finally makes truthful for members).
> - **Unblocks:** a defensible `1.0` (secure-by-configuration cluster, runtime resize, honest
>   status under every plane).
> - **Status:** shipped.
>
> Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> debt tracked: [`../technical-debt/TD-0010-cluster-transport-tls-and-peer-auth.md`](../technical-debt/TD-0010-cluster-transport-tls-and-peer-auth.md),
> [`../technical-debt/TD-0011-dynamic-raft-membership-and-node-identity.md`](../technical-debt/TD-0011-dynamic-raft-membership-and-node-identity.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its Definition of
Done **and** `cargo xtask verify`; never push red. Multi-daemon and TLS tests are network-gated and
**skip-graceful** without their env flags.

## Preflight (verified against the repo at `0.59.0`)

Every claim below was re-read from the code after `0.59` shipped (commits `924c697`…`8011d4c`):

- **The networked member stack exists and is the seam to harden.**
  `crates/hydracache-server/src/grid_host.rs`: `build_member` dispatches networked vs
  `HYDRACACHE_GRID_INPROC=1` (:39-46); `networked_member_stack` (:103-194) builds
  `ChitchatDiscovery::spawn_udp` + `RaftMetadataRuntime::durable_with_config(multi_voter…)` +
  `ClusterAdmissionBridge` and passes the **same** `Arc<RaftMetadataRuntime>` to
  `HydraCache::member().control_plane(raft).discovery(discovery)` (:203-215) and to
  `NetworkedGridHandle` (:698-771) — one membership authority, as the corrected `0.59` plan
  required.
- **The transport is plaintext + unauthenticated (TD-0010).** `spawn_cluster_transport`
  (:258-304) validates `TlsStartupPolicy` (posture only, transport-axum tls.rs:121-152) then
  serves plaintext `axum::serve` (:299). The raft route auth is
  `ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(!tls.enabled || tls.acknowledge_insecure)`
  (:274-276): with `tls.enabled=true && !acknowledge_insecure` every inbound raft message is
  rejected unauthenticated (transport-axum lib.rs:451-458) while `HttpRaftMessageSink` still posts
  `http://` (:494-497) — a TLS-configured multi-node cluster **cannot form**, and it fails as
  election timeouts, not loudly. The `0.48` credential seam is shipped and unused by the daemon:
  `NodeIdentityProvider` (transport-axum lib.rs:246), `StaticNodeIdentityProvider` (:259/:290),
  `ClusterRouteAuth::secure` (:410), `apply_outbound_headers` (:479). **No rustls acceptor exists
  anywhere in the workspace** — W2 is real new plumbing, not wiring.
- **The voter set is static (TD-0011).** `raft_topology` (:375-418) derives voters once from the
  local `seeds` list; `hydracache-cluster-raft` contains **no `ConfChange`**. Drain removes the
  member from metadata only (`leave_cluster_for_shutdown`, bootstrap.rs:373-377 →
  `HydraCache::leave_cluster`), never from the raft `ConfState`. `has_quorum()` (:720-730) counts
  reachable *metadata members*, not raft voters — after a graceful drain the two planes disagree.
- **Identity = address.** `member_node_id_for_addr` (:537-550) derives `ClusterNodeId` from
  `cluster_addr`; `raft_node_id` = FNV-1a of that string, `stable_nonzero_hash` (:552-572), no
  collision handling, nothing persisted.
- **Proposal status lies on followers.** `commit_command`
  (`hydracache-cluster-raft/src/lib.rs`, `Committed` at :1151) proposes, drains ready, and returns
  `RaftCommandStatus::Committed` (:263-267) unconditionally; on a follower raft-rs merely
  **forwards** `MsgPropose` to the leader — nothing is committed yet, but `join_member`
  materializes locally at :1251 anyway (dedup via `command_id` makes it eventually consistent, but
  the status is dishonest — R-3 in spirit).
- **The drive loop swallows errors; the status path replays an unbounded journal.**
  `drive_grid_once` errors → bare `continue` (:322); `send_raft_messages` failures → `let _`
  (:169/:336-339). `NetworkedGridHandle::reachability` (:732-762) clones and replays the **entire**
  `discovery.events()` journal per call per member; the journal is an untruncated `Vec`
  (`hydracache-cluster-chitchat/src/lib.rs:205-209`, `events()` :288). Single-voter mode parks an
  `InMemoryRaftMessageSink` (:150) that retains every message forever. `reshard_phase()` is still
  hardcoded `Idle` (:764) despite the `0.59` W3 goal "every field is real".
- **The headline E2E runs only by hand.** `multi_node_members_form_a_cluster_and_elect_one_leader`
  (`crates/hydracache-server/tests/grid_host.rs:194-269`) is gated on
  `HYDRACACHE_RUN_NETWORKED_DAEMON_E2E` (:271-275); the nightly CI job runs only `-- --ignored`
  suites (`.github/workflows/ci.yml:154-157`) — an env-gated test is invisible to it. It also does
  **not** assert "no lost committed metadata" (only member counts), and the `0.59`-planned
  falsifiable tests `has_quorum_reflects_membership_majority` / `reachability_maps_chitchat_liveness`
  were never written.
- **TD-0009's revisit trigger has fired.** `0.59` added ~900 lines of `grid_host.rs` + raft-crate
  surface; the coverage baseline (`88.07%` lines, clean, 2026-07-05) must be re-measured before any
  ratchet value is chosen (TD-0009 "Coverage Improvement Plan" step 4).

## Release Theme

Make the `0.59` networked grid **production-shaped**: authenticated and TLS-terminated cluster
transport, identity that survives address churn, a raft quorum that grows and shrinks with the
cluster, statuses that never claim more than raft has actually committed, and the release's own
proofs running in CI instead of by hand. Hardening over shipped consensus — **no new algorithm
(R-1), no new consistency level**.

## Non-Goals

- **No new consensus / no new consistency level (R-1).** `ConfChange` is shipped raft-rs API; W4 is
  plumbing + persistence around it, exactly like `0.59` W1b was for the ready loop.
- **No PKI/KMS ownership.** HydraCache does not mint or rotate certificates; the operator supplies
  cert/key/CA material (`0.48` stance, transport-axum lib.rs:244-245). W2 *consumes* the already-
  configured `TlsConfig` paths (config.rs:26).
- **No placement/data-plane rewrite.** Value replication and partition ownership stay on the
  `0.42`/`0.43` path; only membership/status/transport planes are touched (same scoping boundary as
  `0.59` W2).
- **`local`/`client` roles unchanged and `modeled`; embedded fast path byte-for-byte unchanged
  (R-10).** Loopback dev clusters without TLS/auth keep working with zero config change.
- **No coverage-ratchet CI gate in this release.** W7 re-measures and records the baseline
  (TD-0009); enabling `--fail-under-lines` stays a deliberate later step.
- **No autoscaling logic.** W4 makes scale-up/down *possible and truthful*; deciding when to scale
  stays with the `0.56` operator/humans.

## Technical-debt scope & downstream obligations (do not lose)

| TD / obligation | In `0.60`? | Detail |
| --- | --- | --- |
| **TD-0010** cluster transport TLS + peer auth | **Closed here** | W1 (auth + the fail-loud fix for the `tls.enabled` dead-end) + W2 (rustls termination, `https://` sink). W8 marks it Resolved. |
| **TD-0011** dynamic membership + identity | **Partially closed here** | W3 (persistent identity), W4 raft `ConfChange`, voter-based quorum, and graceful follower/leader drain are resolved. Full late-start fourth-daemon join bootstrap remains open in TD-0011 and is not a `0.60` release claim. |
| **TD-0009** coverage ratchet & run stability | **Touched, not closed** | W7 re-measures the post-`0.59`/`0.60` baseline and records it in TD-0009 (its named trigger "0.59 adds server/operator surface" has fired). No ratchet gate is added (Non-Goal); the ratchet decision stays in TD-0009. |
| **`0.59` W4 gate overclaim** | **Corrected (W0)** | The shipped `0.59` plan still reads "Cluster listener is **TLS-bound** when configured"; the manifest theme was already softened. W0 re-states the gate honestly and points it at TD-0010 → `0.60` W1/W2. |
| **`0.59` unimplemented named tests** | **Landed or re-scoped here (W3/W4/W6/W7)** | `conf_change_adds_and_removes_raft_voter_loudly` (→ W4), persistent identity restart coverage (→ W3), voter-majority quorum coverage, `reachability_maps_chitchat_liveness` (→ W6/W7), and "no lost committed metadata" E2E assertion (→ W7). Full late-start daemon join is re-scoped into the remaining TD-0011 item. |
| TD-0002 raft/protobuf, TD-0003 bucket C, TD-0004 placement, TD-0005 Java artifact | **Out of scope** | Untouched. W4 uses `raft::eraftpb::ConfChange` from the already-pinned raft/protobuf pair — no new advisory surface. |

## Dependency Graph

```
W0 ledger honesty ─────────────────────────────────────────────────────────────────────┐
W1 peer auth on the raft route ─► W2 rustls termination + https sink ──────────────────┤
W3 persistent node identity ────► W4 ConfChange voters + drain + ConfState quorum ─────┼─► W7 E2E/CI/coverage proofs ─► W8 docs + gates + TD closure
W5 honest proposal status (independent) ───────────────────────────────────────────────┤
W6 drive-loop & status-path hardening (independent) ───────────────────────────────────┘
```

## W0. Ledger honesty preflight (docs only)

**Goal.** Stop the shipped ledger overclaiming before code lands: the `0.59` plan's gate section
still asserts "Cluster listener is **TLS-bound** when configured", which the code audit disproved.

**Files.** `docs/plans/V0_59_NETWORKED_DAEMON_GRID_HOSTING_PLAN.md` (gate line + W4 "Steps"),
`docs/plans/INDEX.md` (the `0.59` roadmap row still says "TLS-bound cluster listener"),
`docs/daemon-member-mode.md` (TLS paragraph), `docs/technical-debt/TD-0010-…`, `TD-0011-…`
(already created with this plan — verify they reflect the final scope).

**Steps.**
1. Re-state the `0.59` gate line: "Cluster listener enforces the **fail-loud TLS startup policy**;
   actual TLS termination + peer auth were **not** shipped in `0.59` — tracked as TD-0010, closed
   by `0.60` W1/W2." Mirror the same honesty in the W4 steps ("wire `config.tls` into the
   listener" → "validated posture only").
2. Apply the same correction to the `0.59` row in `docs/plans/INDEX.md` (its "What" column reads
   "**W4** TLS-bound cluster listener + fail-loud") — the ledger is a triple (plan + INDEX +
   manifest, R-11); fixing only the plan defers half the honesty to W8.
3. `docs/daemon-member-mode.md`: state plainly that in `0.59.0` the cluster transport is plaintext
   and that non-loopback deployments must sit behind a trusted network boundary until `0.60`.
4. If still present in the working tree, land the removal of the stale duplicate
   `Status: planned.` line in `docs/plans/V0_58_…` (the header already opens with
   `Status: shipped.`; `doc-check` reads the first marker,
   `crates/xtask/src/doc_check.rs:241-244`). No-op if it already landed — this step must not
   depend on any particular worktree state.

**Tests & requirements.** `cargo xtask doc-check` green; no code.

**Risk & rollback.** None — prose honesty only (R-11).

## W1. Peer auth on the cluster raft route (fix the dead-end)

**Goal.** Every inbound raft message is either verified against a configured node credential or
accepted only under an explicitly acknowledged insecure boundary — and the broken
`tls.enabled && !acknowledge_insecure` combination becomes **impossible to reach silently**: the
daemon fails loud at startup naming the missing piece instead of timing out elections forever.

**Design/contract.** Reuse the `0.48` seam end-to-end. New `[cluster_auth]` config on
`ServerConfig` (`crates/hydracache-server/src/config.rs`, struct near `TlsConfig` :26):
`key_id`, `token_file` (never inline token in config — file path read at startup), optional
`previous_key_id`/`previous_token_file` for rotation. When present, `spawn_cluster_transport`
builds `ClusterRouteAuth::secure(Arc<StaticNodeIdentityProvider>, Arc<AllowAllAuthorizer>)`
(transport-axum lib.rs:410/:259/:322) instead of `missing_provider()` (grid_host.rs:274), and
`HttpRaftMessageSink` attaches credentials via `ClusterRouteAuth::apply_outbound_headers`
(transport-axum lib.rs:479) on every POST.

**Security posture matrix (exhaustive — the release gate asserts these rows and no others).**
The config guard at config.rs:238 (non-loopback plaintext requires `tls.acknowledge_insecure`)
stays as the first line of defense; the transport wiring must **agree** with it, never widen it:

| `[cluster_auth]` | `tls.enabled` | `cluster_addr` | Posture |
| --- | --- | --- | --- |
| set | `true` | any | **secured** — W2 TLS termination + W1 credentialed route |
| set | `false` | loopback | **authed-dev** — credentialed route over plaintext loopback |
| unset | `false` | loopback | **dev** — acknowledged-insecure (unchanged `0.59` behavior, R-10) |
| any | `false` | non-loopback **with** `tls.acknowledge_insecure=true` | **staging (named)** — plaintext, explicitly acknowledged; credentials still applied if set, but the posture name does not upgrade |
| unset | `true` | any | **startup error** — TLS termination without peer auth is not a `0.60` posture; no `acknowledge_insecure` escape here |
| unset/any | `false` | non-loopback, unacknowledged | **startup error** (existing `NonLoopbackWithoutTls`, config.rs:238) |

**Rust sketch.**
```rust
// grid_host.rs — replaces the missing_provider() wiring (:274-276)
let auth = match cluster_auth_provider(config)? {                  // reads [cluster_auth]
    Some(identity) => ClusterRouteAuth::secure(identity, Arc::new(AllowAllAuthorizer)),
    None if config.tls.enabled => {
        return Err(CacheError::Backend(
            "tls.enabled member requires [cluster_auth]: a TLS listener without peer auth \
             rejects every inbound raft message and the cluster cannot form"
                .to_owned(),
        ));
    }
    // Plaintext-unauthenticated is acknowledged ONLY where the config guard already
    // allows it: loopback, or the explicit non-loopback staging acknowledgment.
    None => ClusterRouteAuth::missing_provider().acknowledge_insecure_trust_boundary(
        is_loopback(config.cluster_addr.ip()) || config.tls.acknowledge_insecure,
    ),
};
```

**Step-by-step.**
1. Add `ClusterAuthConfig` to `config.rs` (+ TOML/env parsing beside the existing `tls.*` parsing,
   config.rs:178-190) and `validate()` rules: `key_id` without a readable `token_file` → new
   `ServerConfigError::IncompleteClusterAuth` (fail loud, like `:299-303`).
2. In `spawn_cluster_transport` (grid_host.rs:258-304): build the auth per the sketch. The
   *reachable* configurations must match the posture matrix exactly; anything outside those rows is
   a loud startup error or an unacknowledged route.
3. In `HttpRaftMessageSink::send` (grid_host.rs:476-515): build headers via
   `apply_outbound_headers` before the POST; on `ClusterAuthError` fail the send (counted by W6's
   diagnostics, retried by raft's own retransmission).
4. Keep the E2E loopback path credential-free (no config change needed — R-10).

**Testing.** `crates/hydracache-server/tests/grid_host.rs` +
`crates/hydracache-cluster-transport-axum` (existing unit patterns):
- `tls_enabled_member_without_cluster_auth_fails_loud_at_startup` — the dead-end is now a startup
  error naming `[cluster_auth]` (falsifiable: the old silent behavior would start and hang).
- `raft_route_rejects_missing_or_invalid_credential` — two-daemon loopback with auth configured on
  one side only → inbound rejected, `rejected_by_route` counter increments (transport-axum
  lib.rs:402-403), no panic.
- `raft_route_accepts_rotated_previous_credential` — `previous_*` accepted during rotation
  (mirrors `StaticNodeIdentityProvider::with_previous`, transport-axum lib.rs:280).
- `plaintext_route_is_acknowledged_only_on_loopback_or_staged_boundary` — the posture matrix is
  enforced at the transport, not just in config validation: non-loopback plaintext without the
  explicit staging acknowledgment never reaches an acknowledged route (falsifiable).
- `authed_members_exchange_raft_messages_and_elect` — network-gated: 3 daemons with shared
  credential form a cluster (same harness as the `0.59` E2E, :194-269).
- Run: `cargo test -p hydracache-server --locked grid_host` (+ the network-gated tier).

**Pros.** Fixes the worst `0.59` defect (broken TLS-configured clusters) with zero new
dependencies; rotation story inherited from `0.48`.
**Risks & rollback.** Header-credential auth is not transport encryption — W2 provides that; the
plan must not claim "secure" until both land. Revert restores `missing_provider()` (loopback-grade,
current behavior).

## W2. Real TLS termination on the cluster listener + `https://` sink

**Goal.** `tls.enabled = true` means the cluster listener actually terminates TLS with the
configured cert/key and outbound raft messages go over `https://` verified against the configured
CA — closing the TD-0010 gap between posture and reality.

**Design/contract.** New dependency: `axum-server` (rustls feature) — the workspace has **no**
rustls acceptor today (verified: no `rustls`/`TlsAcceptor` outside sqlx runtime features). Server
side: `axum_server::bind_rustls(cluster_addr, RustlsConfig::from_pem_file(cert, key))` replaces
plaintext `axum::serve` (grid_host.rs:299) when `config.tls.enabled`; plaintext stays for the
loopback/acknowledged path. Client side: build the `reqwest::Client` in `HttpRaftMessageSink::new`
(grid_host.rs:448-460) with `Certificate::from_pem(ca)` + `https://` URLs when TLS is enabled.
Peer identity stays W1's credentialed route; client-certificate mTLS is deferred (see step 3).

**Step-by-step.**
1. Add `axum-server = { version = "0.8.0", features = ["tls-rustls"] }` to the workspace + server
   crate; run `cargo deny check bans` (gate registry, GATES.md) before anything else — new
   dependency trees must pass the bans/advisories gates.
2. `spawn_cluster_transport` (grid_host.rs:258-304): branch on `config.tls.enabled` —
   `RustlsConfig::from_pem_file(tls.cert_path, tls.key_path)` (paths exist-checked fail-loud;
   `has_complete_material` is already enforced at config validation, config.rs:235) and serve the
   same `routes` over `axum_server::bind_rustls`; keep `with_graceful_shutdown` semantics (the
   watch-channel pattern at :288-302).
3. `HttpRaftMessageSink` (grid_host.rs:440-516): construct with a `scheme` + optional CA; when TLS
   is enabled POST to `https://{peer.address}{DEFAULT_RAFT_APPEND_PATH}` with
   `Client::builder().add_root_certificate(ca).build()`. **Client-certificate mTLS is deferred
   out of `0.60` (named):** `TlsConfig` (config.rs:26) has no client-identity fields, and W1's
   credentialed route is the peer-identity mechanism; the `0.60` transport-security claim is
   server-side TLS termination + CA verification + W1 auth. A later release that wants mutual
   TLS adds the config surface and its tests then.
4. Thread `config.tls` into the sink construction site (grid_host.rs:143-151) — today the sink is
   built without TLS knowledge.
5. Add a `rcgen` **dev-dependency** to `hydracache-server` for test certificate generation (self-
   signed CA + leaf per test dir) — no production code path touches it.

**Testing.** `crates/hydracache-server/tests/grid_host.rs`:
- `cluster_listener_rejects_plaintext_when_tls_enabled` — **falsifiable**: a plain `http://` POST
  to a TLS listener fails at the TLS layer; if it ever succeeds the gate is red.
- `tls_members_exchange_raft_messages_over_https` — network-gated: 2-3 daemons with rcgen-issued
  certs (shared CA) form a cluster and elect; replaces the vacuous `0.59`
  `member_cluster_listener_uses_configured_tls` (:179-191), which started a **single** node with
  nonexistent cert files and asserted nothing about TLS — delete or repurpose it explicitly.
- `tls_member_with_unreadable_cert_fails_loud_at_startup` — missing/garbage PEM → startup error
  naming the path.
- `sink_verifies_peer_against_configured_ca` — wrong CA → send fails (counted), cluster does not
  form (falsifiable).
- Run: `cargo test -p hydracache-server --locked grid_host` (+ network-gated tier).

**Pros.** The `0.48` "in-transit" claim finally covers the daemon's cluster plane; W1+W2 together
make the W1 posture matrix exhaustive and honest.
**Risks & rollback.** New dependency surface (axum-server, rcgen dev-only) — gated by `cargo deny`;
cert handling bugs fail at startup, not silently. Revert restores plaintext (loopback-grade) and
reopens TD-0010.

## W3. Persistent node identity (decouple identity from the address)

**Goal.** A member's `ClusterNodeId` and raft id survive address/port changes and restarts; any
identity mismatch or hash collision fails loud instead of corrupting a durable raft log.

**Design/contract.** Optional `node_id` on `ServerConfig`; regardless of source, the first start of
a member persists `storage_dir/node-identity.json` (`{ node_id, raft_node_id, cluster }`). On
restart the persisted file **wins**: a configured/derived id that disagrees is a startup error
(same fail-loud family as `validate_snapshot_identity`,
`hydracache-cluster-raft/src/lib.rs:801-818`, which already rejects a raft-store/node mismatch).
Address-derived defaults (`member_node_id_for_addr`, grid_host.rs:537-550) remain the fallback for
first-boot compatibility (R-10). `raft_topology` (grid_host.rs:375-418) gains collision detection:
two distinct node ids hashing (`stable_nonzero_hash`, :562-572) to one raft id → startup error
naming both.

**Step-by-step.**
1. `config.rs`: add optional `node_id` (TOML/env), no validation change for local/client roles.
2. `grid_host.rs::networked_member_stack` (:103-117): before building the raft config, resolve
   identity = persisted file → config → address-derived; write the file if absent (inside the
   already-created `storage_dir`, beside `raft-log/`, :111-117); error on mismatch.
3. `raft_topology` (:375-418): build the `raft_id → node_id` map with `BTreeMap::insert` collision
   check (today `peers.entry(raft_id).or_insert(…)` at :396-398 silently keeps the first).
4. Registration: `node-identity.json` is a **durable artifact** → register format + reader window
   in `docs/COMPAT.md` (R-4): version field, unknown-future-version refuses to start.

**Testing.** `crates/hydracache-server/tests/grid_host.rs`:
- `member_identity_persists_across_address_change` — start on port A (dir fresh), shut down,
  restart the same `storage_dir` on port B → same `node_id`/`raft_node_id`, durable raft log opens
  clean (this scenario **corrupts silently** today).
- `configured_node_id_conflicting_with_persisted_identity_fails_loud`.
- `seed_hash_collision_fails_loud_at_topology_build` — synthetic collision via test-only ids
  (falsifiable: today the second peer is silently dropped, :396-398).
- Run: `cargo test -p hydracache-server --locked grid_host`.

**Pros.** Prerequisite for W4 (ConfChange needs stable ids); kills the address-churn foot-gun.
**Risks & rollback.** Identity file adds a compat surface (registered, R-4). Revert returns to
address-derived identity; W4 must not land without W3.

## W4. Dynamic raft membership: `ConfChange` voters, drain, `ConfState` quorum

**Goal.** The raft voter set follows the cluster: an admitted member becomes a **voter** at
runtime, a draining member is **removed** before exit (shrinking the quorum denominator), and
`has_quorum()` counts reachable **raft voters against the raft `ConfState` majority** — ending the
metadata-vs-raft quorum split. This resolves TD-0011's identity/drain/quorum sub-items; full
late-start daemon join remains a named follow-up before `0.56` operator scale-up can be claimed
end-to-end.

**Design/contract (runtime, `hydracache-cluster-raft`).**
1. **Persist conf state.** `RaftLogStore` (log_store.rs:46-83) has no conf-state saver — add
   `fn save_conf_state(&self, conf_state: &ConfState) -> RaftStoreResult<()>` with impls for
   `InMemoryRaftLogStore` (:462 already has `initialize_with_conf_state`) and the durable/sled
   stores. If the durable layout changes, bump `RAFT_LOG_FORMAT_VERSION` (log_store.rs:15; the
   reader already refuses unknown-future versions, :407) and register the migration in
   `docs/COMPAT.md` (R-4, forward-only).
2. **Voter-change API.** On `RaftMetadataRuntime` beside the drive seam (lib.rs:712-757):
   `propose_add_voter(raft_node_id: u64) -> CacheResult<Vec<RaftWireMessage>>` and
   `propose_remove_voter(…)` — `raw_node.propose_conf_change(vec![], ConfChange{…})`, leader-only
   (non-leader → loud `Err`, do **not** silently forward conf changes). In the private
   `drain_ready`/`apply_committed_entries` (lib.rs:1195-1210), stop skipping non-`EntryNormal`
   entries (:1202): handle `EntryConfChange` → `raw_node.apply_conf_change(&cc)` →
   `store.save_conf_state(&new_conf_state)`.
3. **Voter view.** `pub fn voter_ids(&self) -> Vec<u64>` reading
   `raw_node.raft.prs().conf().voters()` — the ConfState truth the handle will count against.
4. **One drain algorithm — voter removal is always proposed by the current leader.** The
   draining node publishes the shipped chitchat **graceful-leave marker**
   (`hydracache-cluster-chitchat/src/lib.rs:311`); the leader's drive loop treats an observed
   marker as the exact inverse of the promotion path (step 5) and proposes
   `remove_voter(leaver)`; the leaver bounded-waits until it is no longer in `voter_ids()`
   before proceeding. If the draining node **is** the leader, it first
   `raw_node.transfer_leader(target)` to the lowest-id reachable voter, bounded-waits for the
   soft-state change, then follows the same follower path (marker → new leader removes it).
   No self-removal-while-leader and no forwarded conf changes — removal has exactly **one**
   proposer: the current leader. Single-step changes only; document the joint-consensus-free
   limitation.

**Design/contract (daemon, `grid_host.rs`).**
5. **Promotion.** In `drive_grid_once` (:331-341), leader-side after `bridge.run_once()`: for every
   committed metadata member whose raft id (via the W3 identity map) is **not** in `voter_ids()`,
   propose AddNode; send returned messages through the sink. Fail loud (W6 counter + warn) when a
   member has no known peer address.
6. **Join flow for a late daemon.** A node whose own address is **not** in its `seeds` list starts
   in *joiner mode*: `multi_voter` config with the seeds-derived voter set **excluding itself**,
   `auto_campaign(false)` (lib.rs config :144-171), and it does not campaign — it waits to be
   added by the leader (its gossip announce → bridge admits → leader proposes AddNode → raft
   replicates the entry/snapshot to it). Extend `wait_for_raft_leader` (:518-531) to also accept
   "self not yet voter" as a waiting state with the same bounded deadline.
6b. **Dynamic route/identity contract (the contract that makes step 6 possible).** Today
   `HttpRaftMessageSink` holds a **frozen** peers map built once from `seeds`
   (grid_host.rs:441-460) and fails loud on an unknown `message.to` (:480-485) — a late joiner
   would be admitted into metadata yet unroutable. The contract: the joiner **publishes its
   `raft_node_id` and `cluster_addr` as chitchat announce metadata** alongside the existing
   candidate node-id/generation (the same KV channel the graceful-leave marker uses); every
   member's drive loop folds admitted candidates into a **shared, updatable routing table**
   (`Arc<RwLock<BTreeMap<u64, RaftPeer>>>`) read by both `HttpRaftMessageSink` and the step-5
   promotion path. Unknown `message.to` stays a loud error — but only after gossip-known peers
   have been folded in. Auth/TLS identity for the joiner is the same cluster-wide W1/W2 material
   (shared credential + CA-signed cert); per-join provisioning is out of scope for `0.60`.
7. **Drain.** `graceful_shutdown` enters local drain, commits metadata leave while the member is
   still a voter, then asks the grid handle to remove the local raft voter. Followers forward their
   own removal request to the known leader; leader drain is covered by the E2E re-election path.
8. **Quorum honesty.** `NetworkedGridHandle::has_quorum` (:720-730): reachable **voters** ≥
   `⌊voters/2⌋+1` over `raft.voter_ids()` mapped through the identity map; keep
   `leader_id().is_some()` as a conjunct.

**Testing.**
- Runtime level (`crates/hydracache-cluster-raft/tests/networked_raft.rs`, extending the `0.59`
  `NetworkedRuntimeCluster` harness at :399-482):
  - `conf_change_adds_and_removes_raft_voter_loudly` — the `0.59`-planned, never-written test:
    AddNode commits, RemoveNode commits, and every runtime observes the changed voter set.
  - `follower_can_request_own_voter_removal_for_drain` — follower drain forwards RemoveNode to the
    known leader and the cluster converges without that voter.
  - `conf_change_fails_loud_when_proposed_by_non_leader` — the public leader-only API remains
    fail-loud.
- Daemon level (`crates/hydracache-server/tests/grid_host.rs`; the routing test is PR-tier unit,
  the rest network-gated):
  - `drive_loop_admits_a_gossip_candidate_into_the_shared_raft_runtime` and
    `sync_raft_voters_adds_admitted_member_with_known_peer` cover leader-side admitted-member
    promotion and peer routing.
  - `multi_node_members_form_a_cluster_and_elect_one_leader` — 3 nodes, graceful follower drain →
    survivors report 2 members and `quorum_ok == true`; then graceful leader drain → survivor
    re-election with the expected committed member set.
- Run: `cargo test -p hydracache-cluster-raft --locked networked_raft`,
  `cargo test -p hydracache-server --locked grid_host` (+ network-gated tier).

**Pros.** Graceful scale-down now changes the quorum, and the two membership planes converge on one
truth for drain. Full scale-up through a late-start daemon remains explicit TD-0011 follow-up work.
**Risks & rollback.** The heaviest WI: conf-change ordering bugs corrupt clusters — mitigated by
single-step changes only, leader-only proposals, persisted `ConfState`, and the falsifiable tests
above. Revert restores static bootstrap (current behavior) and reopens TD-0011; W7's join/drain
E2Es must be reverted with it.

## W5. Honest proposal status on non-leaders

**Goal.** `RaftCommandStatus::Committed` (lib.rs:263-267) is returned **only** when the entry is
applied locally. A forwarded proposal reports itself as forwarded; `join_member` on a follower
waits for the real commit instead of materializing optimistically.

**Design/contract.** Add `RaftCommandStatus::Forwarded` (runtime-only enum — not persisted, not on
the wire, so no COMPAT entry needed; R-4 scope note in the PR). In `commit_command`
(lib.rs:1137-1160): branch on `raw_node.raft.state` — leader → today's propose+drain →
`Committed` (:1151); follower with a known leader → propose (raft-rs forwards `MsgPropose`) →
`Forwarded` with the current `applied_index`; no leader → loud `Err` ("no raft leader; retry after
election"). Add `pub fn command_applied(&self, command_id: &str) -> bool` checking
`applied_command_ids`. In `ClusterControlPlane::join_member`/`join_client`
(lib.rs:1216-1281): on `Forwarded`, bounded-poll `command_applied` (the drive loop keeps stepping
inbound appends) with a deterministic retry budget; on success materialize from the *replicated*
command (:1251 path); on timeout → loud `Err`. The daemon startup already waits for a leader
first (`wait_for_raft_leader`, grid_host.rs:518-531), so the common path is short.

**Testing.** `crates/hydracache-cluster-raft/tests/networked_raft.rs`:
- `follower_join_member_reports_forwarded_then_applies` — 3-runtime cluster: join via a follower →
  first status `Forwarded`, entry appears on all voters after drive, `join_member` returns the
  committed member (falsifiable: the old code returned `Committed` before any replication).
- `proposal_without_leader_fails_loud` — pre-election cluster (auto_campaign(false), no ticks) →
  `Err`, not a fake `Committed`.
- `leader_path_still_returns_committed_synchronously` — single-node regression guard (the
  `chitchat_admission_bridge.rs` tests must keep passing unchanged).
- Run: `cargo test -p hydracache-cluster-raft --locked`.

**Pros.** Removes the last "status says more than raft knows" spot (R-3/R-11); makes W4's
follower-side flows debuggable.
**Risks & rollback.** `join_member` gains a bounded wait on followers — startup latency, bounded by
the same deadline family as `GRID_LEADER_WAIT_TIMEOUT` (grid_host.rs:34). Revert restores
optimistic materialization (eventually consistent, dedup-safe — current behavior).

## W6. Drive-loop and status-path hardening

**Goal.** No silently swallowed errors on the grid drive path (R-3), bounded memory in
long-running daemons, O(1) reachability, and an honest `reshard_phase`.

**Step-by-step.**
1. **Drive diagnostics.** Replace the bare `continue` in `spawn_grid_drive` (grid_host.rs:322) and
   the `let _` sends (:169, :336-339) with a `GridDriveDiagnostics` (atomics: `ticks`,
   `drive_errors`, `send_failures`, `last_error: Mutex<Option<String>>`) owned by
   `NetworkedGridHandle`, surfaced in its `Debug` (:686-696) and in the cluster diagnostics
   snapshot; `tracing::warn!` rate-limited (log on state change + every Nth repeat, not per tick —
   the loop runs every 50ms, :33). Labels are bounded (`R-6`): error-kind enum, never peer-id-set
   cardinality beyond the roster.
2. **No-op sink for single-voter.** `InMemoryRaftMessageSink` retains every message forever
   (transport seam, `hydracache-cluster-raft/src/lib.rs:311-334`); single-voter mode
   (grid_host.rs:150) needs a `NoopRaftMessageSink` (new, in `grid_host.rs` — 5 lines) instead.
3. **Materialized liveness.** `ChitchatDiscovery` already runs a watcher
   (`spawn_live_node_watcher`, chitchat lib.rs:261) that maintains `candidates`; extend
   `DiscoveryState` (:205-209) with `liveness: BTreeMap<ClusterNodeId, DiscoveryLiveness>`
   (`Live | Suspect | Dead`) updated on the same events, and add
   `pub fn liveness(&self) -> BTreeMap<…>`. Rewrite `NetworkedGridHandle::reachability`
   (grid_host.rs:732-762) to read the map — today it clones and replays the **whole** event
   journal per member per call.
4. **Bounded journal.** Cap `DiscoveryState.events` (chitchat lib.rs:208) as a ring
   (`MAX_DISCOVERY_EVENTS = 1024`, drop-oldest). The journal is advisory diagnostics — the
   liveness map and the control plane stay authoritative, so drop-oldest does not violate R-3;
   document that in the field's rustdoc.
5. **Honest reshard.** `reshard_phase()` is hardcoded `Idle` (grid_host.rs:764). Source it from the
   grid runtime if the seam exists (`HydraCache::cluster_diagnostics()` — check
   `crates/hydracache/src/cluster/diagnostics.rs` for a reshard view); if no seam exists, keep
   `Idle` **and** mark the field `modeled` in `/cluster/overview` docs + `docs/management-center.md`
   (R-11: label it, don't fake it). Either outcome is acceptable; silent fake-liveness is not.

**Testing.**
- `drive_loop_counts_and_reports_send_failures` — sink that fails N sends → `send_failures == N`,
  `last_error` set, loop still alive (unit, `grid_host.rs` tests module :888-919).
- `reachability_maps_chitchat_liveness` — the `0.59`-planned test, now cheap: suspect/dead/live
  map states → `Suspect`/`Unreachable`/`Reachable` (chitchat crate + server handle level).
- `discovery_event_journal_is_bounded` — push `2 * MAX` events → `len() <= MAX`, newest retained.
- `single_voter_sink_does_not_accumulate` — messages dropped, not retained.
- Run: `cargo test -p hydracache-server --locked grid_host`,
  `cargo test -p hydracache-cluster-chitchat --locked`.

**Pros.** Kills the leak-over-time class `0.58` built detectors for, in the exact component `0.59`
added; failures become visible without log-spam.
**Risks & rollback.** Each step is independent and individually revertible; none changes protocol
behavior.

## W7. Multi-daemon proofs: E2E extensions, CI tier, coverage baseline

**Goal.** The release's own claims run in CI, the `0.59` E2E proves metadata durability (not just
leader counts), and the TD-0009 baseline is re-measured on the new surface.

**Step-by-step.**
1. **CI nightly tier.** The `dst-nightly-soak` job (`.github/workflows/ci.yml:127-157`) gains a
   step after the `--ignored` sentinels:
   `HYDRACACHE_RUN_NETWORKED_DAEMON_E2E=1 cargo test -p hydracache-server --test grid_host multi_node --locked -- --nocapture`
   (matching the command documented in GATES.md).
2. **Metadata-durability assertion.** Extend
   `multi_node_members_form_a_cluster_and_elect_one_leader`
   (`crates/hydracache-server/tests/grid_host.rs:194-269`): after convergence and **before** the
   first drain, record each survivor-visible member set; after follower drain and after leader
   drain/re-election assert every survivor exposes the expected committed member set — the
   plan-promised `no_lost_committed_metadata_across_leader_change`, folded into the E2E.
3. **Quorum falsifiability.** `has_quorum_reflects_voter_majority` — with W4's `voter_ids()`:
   3 voters, mark 2 unreachable in the liveness map → `has_quorum() == false`; restore →
   `true` (unit-level with a stubbed liveness view; the `0.59` plan promised this and shipped
   without it).
4. **Coverage baseline (TD-0009).** After W1-W6 land:
   `cargo llvm-cov --workspace --all-targets --locked --summary-only`; record the new clean
   baseline in `TD-0009` (its trigger fired: `0.59`+`0.60` added `grid_host.rs`, raft drive/conf
   surface, chitchat liveness). **No ratchet gate** is added (Non-Goal) — the number and the
   decision live in TD-0009.

**Testing.** This WI *is* tests + CI; its own gate is: the nightly job goes green with the new
step, and `cargo xtask verify` stays green locally (all new gated tests skip-graceful without
their env flags — same pattern as `networked_daemon_e2e_enabled`, tests/grid_host.rs:271-275).

**Pros.** The headline proof stops being manual; every `0.59`-promised-but-missing test now exists
or is explicitly re-scoped in writing.
**Risks & rollback.** Nightly time grows by the E2E (~30s budget); election timing flake stays
confined to the nightly tier (R-5 tiering), never the PR gate.

## W8. Docs, gates, and TD closure

**Goal.** Ledger/manifest honest at ship time.

**Files.** `docs/daemon-member-mode.md` (auth + TLS + identity-file + join/drain runbook),
`docs/management-center.md` (quorum now counts raft voters; reshard field honesty per W6.5),
`docs/GATES.md` (the nightly networked tier command list), `docs/COMPAT.md` (node-identity file;
raft `ConfState` persistence note), `docs/technical-debt/TD-0010-…` → Resolved,
`TD-0011-…` → partially resolved with late-start daemon join still Open,
`docs/technical-debt/TD-0009-…` (new baseline recorded), `releases.toml` + `INDEX.md`
(+ the plan header `Status`) flipped together — `cargo xtask doc-check` enforces the triple
(R-11, doc_check.rs:206-232).

**Steps.**
1. Runbook: bringing up a secured 3-node cluster (cert/key/CA + `[cluster_auth]`) and draining
   members out — each with the observable `/cluster/overview` transitions. Full fourth-member
   late join remains named in TD-0011.
2. Mark TD-0010 Resolved; mark TD-0011's identity/drain/quorum sub-items resolved while keeping
   late-start daemon join Open; update TD-0009's baseline section (W7.4).
3. Flip `0.60.0` to `shipped` in `releases.toml` + `INDEX.md` + the plan header **only** when every
   gate below is green; anything deferred gets written down here instead (R-7: ship without the
   claim rather than on a red gate).

**Tests & requirements.** `cargo xtask verify` (includes `doc-check`); the GATES.md commands below.

## Test coverage matrix (every new artifact has a named test)

| New code | Source file | Covering test(s) | Tier |
| --- | --- | --- | --- |
| `ClusterAuthConfig` + posture matrix (W1) | `hydracache-server/src/config.rs`, `grid_host.rs:274` | `tls_enabled_member_without_cluster_auth_fails_loud_at_startup` (falsifiable), `raft_route_rejects_missing_or_invalid_credential`, `raft_route_accepts_rotated_previous_credential`, `plaintext_route_is_acknowledged_only_on_loopback_or_staged_boundary` (falsifiable) | PR |
| rustls listener + https sink (W2) | `grid_host.rs:258-304/440-516` | `cluster_listener_rejects_plaintext_when_tls_enabled` (falsifiable), `tls_member_with_unreadable_cert_fails_loud_at_startup`, `sink_verifies_peer_against_configured_ca` (falsifiable) | PR |
| persistent identity (W3) | `grid_host.rs:103-117/537-572`, `config.rs` | `member_identity_persists_across_address_change`, `configured_node_id_conflicting_with_persisted_identity_fails_loud`, `future_node_identity_format_fails_loud`, `seed_hash_collision_fails_loud_at_topology_build` (falsifiable) | PR |
| `save_conf_state` + `ConfChange` API (W4) | `hydracache-cluster-raft/src/lib.rs`, `log_store.rs:46-83` | `conf_change_adds_and_removes_raft_voter_loudly` (falsifiable), `follower_can_request_own_voter_removal_for_drain`, `conf_change_fails_loud_when_proposed_by_non_leader` | PR |
| dynamic routing table and admitted-member promotion (W4.6b) | `grid_host.rs:440-516` | `drive_loop_admits_a_gossip_candidate_into_the_shared_raft_runtime`, `sync_raft_voters_adds_admitted_member_with_known_peer`, `refresh_raft_peers_tracks_admitted_member_control_endpoints` | PR |
| drain daemon flows (W4) | `grid_host.rs`, `bootstrap.rs:373-377` | follower drain + leader drain/re-election folded into `multi_node_members_form_a_cluster_and_elect_one_leader`; full fourth-daemon late join remains TD-0011 follow-up | network-gated / nightly |
| `Forwarded` status + applied-wait (W5) | `hydracache-cluster-raft/src/lib.rs:263/1137-1281` | `follower_join_member_reports_forwarded_then_applies` (falsifiable), `proposal_without_leader_fails_loud`, `leader_path_still_returns_committed_synchronously` | PR |
| drive diagnostics + noop sink (W6) | `grid_host.rs:306-359` | `drive_loop_counts_and_reports_send_failures`, `single_voter_sink_does_not_accumulate` | PR |
| liveness map + bounded journal (W6) | `hydracache-cluster-chitchat/src/lib.rs:205-294` | `reachability_maps_chitchat_liveness`, `discovery_event_journal_is_bounded` | PR |
| voter-quorum honesty (W4/W7) | `grid_host.rs:720-730` | `has_quorum_reflects_voter_majority` (falsifiable) | PR |
| E2E durability + CI tier (W7) | `tests/grid_host.rs:194-269`, `ci.yml:127-157` | `no_lost_committed_metadata_across_leader_change` (folded into the E2E), nightly job step green | network-gated / nightly |

**Coverage rule (DoD):** no new public type or file lands without a row here; PR-tier tests are
deterministic and inside `cargo xtask verify`; network rows are env-gated and **skip-graceful**.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green throughout; all network-gated tests skip-graceful without their env
  flags (PR gate green without a cluster).
- The transport security postures match the **W1 posture matrix exactly** and are loud: secured
  (TLS termination + credentialed route) works end-to-end with a real handshake and verified CA;
  plaintext exists only on loopback or behind the explicit non-loopback staging acknowledgment
  (config.rs:238); `tls.enabled` without `[cluster_auth]` and every other combination is a
  **startup error naming the missing piece** — the `0.59` silent dead-end is impossible (W1/W2,
  falsifiable: plaintext client rejected by a TLS listener; non-loopback plaintext never reaches
  an acknowledged route unless staged).
- A member's identity survives an address change over the same `storage_dir`; mismatches and hash
  collisions fail loud (W3).
- A gracefully drained member leaves the voter set (quorum denominator shrinks); follower drain
  and leader drain/re-election are covered by the networked daemon E2E; `has_quorum()` counts
  reachable voters against the raft `ConfState` majority. Full fourth-daemon late join stays open
  in TD-0011 and is not a `0.60` release claim.
- No `RaftCommandStatus::Committed` for an uncommitted proposal: follower proposals report
  `Forwarded` and resolve only on real apply; leaderless proposals fail loud (W5).
- Drive-loop failures are counted and visible; discovery journal bounded; reachability is O(1) on
  a materialized liveness view; `reshard_phase` is either real or explicitly labeled `modeled`
  (W6 — R-3/R-6/R-11).
- The networked daemon E2E (formation, follower drain, leader drain/re-election, **no lost
  committed metadata**) runs in the **nightly CI tier**, not only by hand (W7).
- TD-0009 baseline re-measured and recorded post-`0.60` surface; **no** ratchet gate added (W7).
- **TD-0010 marked Resolved**; TD-0011's identity/drain/quorum sub-items marked resolved with
  late-start daemon join still Open; `0.59` gate wording corrected (W0); `local`/`client` roles
  unchanged and `modeled`; embedded fast path byte-for-byte unchanged (R-10); no new
  consensus/consistency level (R-1).
- `releases.toml` + `INDEX.md` + plan header flipped together; `cargo xtask doc-check` green.

```powershell
# fast (PR) tier
cargo xtask verify

# focused suites
cargo test -p hydracache-cluster-raft --locked
cargo test -p hydracache-cluster-chitchat --locked
cargo test -p hydracache-server --locked grid_host

# network-gated tier (nightly; also runnable locally)
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
cargo test -p hydracache-server --test grid_host --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue

# coverage baseline re-measure (TD-0009, record only — no gate)
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

## Final Release Decision

`0.60.0` ships **only** if every gate above is green. W2 shipped; W4 is deliberately re-scoped in
writing: identity, `ConfChange`, voter-quorum, and graceful drain ship, while full fourth-daemon
late join remains open in TD-0011 and is not claimed. The one unacceptable outcome is the `0.59`
pattern this plan exists to fix: a gate sentence that says more than the code does.
