# Disaster Recovery Runbook

1. Freeze writers or route traffic to the surviving region before restoring.
2. Select the latest valid full backup manifest and PITR target sequence.
3. Restore only after manifest version, object lengths, and checksums validate.
4. Rebuild the control plane from the restored snapshot and durable values.
5. Run anti-entropy repair and keep the cluster in degraded mode until RF is restored.
