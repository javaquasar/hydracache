# Upgrade Runbook

1. Confirm the target binary supports the durable formats listed in
   `docs/COMPAT.md`.
2. Roll one StatefulSet ordinal at a time and wait for `/ready`.
3. Watch `hydracache_admission_rejected_total` and
   `hydracache_replication_backpressure_total`; pause the rollout if either grows.
4. Keep the previous certificate/key material in the rotation window until every pod
   reports the new identity.
5. Finish by running a backup and PITR restore validation against the new version.
