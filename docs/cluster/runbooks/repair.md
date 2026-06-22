# HydraCache Grid Repair Runbook

HydraCache grid hardening in 0.42 is still not distributed transactions. A repair
runbook helps operators restore cache-grid health; it does not make cross-key or
cross-node application writes atomic.

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
