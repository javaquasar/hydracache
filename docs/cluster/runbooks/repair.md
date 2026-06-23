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

## Active-Active Geo Staleness

Signal: `hydracache_region_staleness_window_ms` or
`HydraCacheGeoStalenessSloBreach`.

Action:

1. Check the read-only geo status surface and confirm
   `worst_staleness_window_ms`, `staleness_slo_target_ms`, and
   `active_active_acked`.
2. Inspect `hydracache_region_link_lag` for the affected bounded `link` label.
3. Reduce write fan-out or temporarily route writes to the home region if the
   WAN link remains behind the SLO.
4. Let cross-region anti-entropy converge; do not claim cross-region
   linearizability. Active-active is bounded staleness, not distributed
   transactions.

## Region Failover

Signal: `hydracache_region_state{state="down"}` or a region-down operator
declaration.

Action:

1. Confirm the old home region is explicitly `Down`, not only `Suspect`.
2. Run the promotion sequence as one control-plane operation:
   `freeze -> commit higher epoch -> anti-entropy converge -> unfreeze`.
3. Confirm `hydracache_region_promotion_total` increments and the promoted
   partitions have a surviving home region.
4. If the old region rejoins with a lower epoch, keep it fenced and backfill it
   from the current authority.

## Active-Active Disable

Signal: planned rollback or sustained SLO breach.

Action:

1. Stop admitting new remote active-active writes.
2. Wait for `hydracache_region_link_lag` to drain to zero on every bounded link.
3. Confirm anti-entropy has converged and CRDT metadata GC gates are satisfied.
4. Switch affected caches back to home-region-only write authority.
5. Keep geo alerts active until `hydracache_region_staleness_window_ms` remains
   inside the target window for the full observation period.
