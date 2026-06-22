# HydraCache Grid Repair Runbook

HydraCache grid hardening in 0.42/0.43 is still not distributed transactions. A
repair runbook helps operators restore cache-grid health; it does not make
cross-key or cross-node application writes atomic.

## Repair Debt

Signal: `hydracache_tombstone_repair_debt` or
`hydracache_repair_debt_degraded_mode`.

Action:

1. Check `GET /cluster/status` and confirm `repair_debt`, quorum posture, and the
   latest split-brain report.
2. Keep writes on the healthy quorum path. Do not force-enable minority nodes.
3. Let anti-entropy converge. If debt keeps growing, reduce replication admission
   or remove the permanently offline backup from the committed topology.

## Replication Lag

Signal: `hydracache_replication_lag`.

Action:

1. Identify slow backups from diagnostics snapshots, not metric labels.
2. Check disk/network health for the slow member.
3. Confirm AIMD windows recover after the slow member becomes healthy.

## Split Brain

Signal: `hydracache_split_brain_detected_total`.

Action:

1. Inspect the `SplitBrainReport` in `GET /cluster/status`.
2. Confirm the higher-epoch side won.
3. Review discarded loser-side entries before re-enabling traffic that depended
   on the losing side.

## Credential Rotation

Signal: `hydracache_cluster_auth_rejected_total`.

Action:

1. Confirm each node has the current credential.
2. During rolling rotation, keep the previous credential in the accepted window.
3. Remove the previous credential only after every member has restarted or
   reloaded credentials.

## Zone Loss

Signal: `hydracache_placement_zone_underspread` or a zone-loss fault in staging.

Action:

1. Confirm the lost zone from diagnostics snapshots; do not infer it from a
   high-cardinality metric label.
2. Verify `ZoneAwareReplicationStrategy` still reports write quorum across the
   surviving zones.
3. Keep `AutoRepairPolicy` in `Advisory` mode until the hot path is stable.
4. Move to `Active` only when repair debt and replication lag are within the
   documented cap, then let bounded re-replication restore target RF.

## Online Resharding

Signal: `hydracache_reshard_moves_inflight` or
`hydracache_reshard_backfill_lag`.

Action:

1. Inspect the committed `ReshardPlan`; each `PartitionMove` should progress
   through `Prepare -> Backfill -> Commit -> Cleanup`.
2. If lag rises, lower `max_concurrent` before starting new moves.
3. Never commit a move that fails `validate_move_preserves_zone_quorum`.
4. On coordinator restart, resume from the persisted plan progress instead of
   creating a fresh plan for the same partitions.

## Control-Plane Restore

Signal: lost control-plane state or failed restore rehearsal.

Action:

1. Load the latest `ControlPlaneSnapshot` from the operator-supplied
   `SnapshotSink`.
2. Refuse restore if `format_version` is newer than the current binary.
3. Rebuild `TopologyAuthority` with `restore_topology_from_snapshot`.
4. Re-run placement readiness and quorum checks before admitting writes.

## Upgrade Guard

Signal: `UpgradeGuard` rejects a rolling step.

Action:

1. Check `docs/COMPAT.md` for raft-log, value-record, snapshot, and wire-frame
   versions.
2. Upgrade only within the registered 0.42 -> 0.43 window.
3. Do not bypass a format mismatch; stop the rollout and upgrade readers first.
