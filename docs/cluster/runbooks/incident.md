# Incident Runbook

1. Check `/health`, `/ready`, and the production dashboard first.
2. If admission rejections are rising, reduce incoming load and honor retry-after.
3. If replication backpressure is rising, pause topology changes and prioritize repair.
4. If corruption is detected, do not serve the suspect artifact; restore from backup or
   repair from a valid peer copy.
5. Capture diagnostics snapshots before restarting pods.
