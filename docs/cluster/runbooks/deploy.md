# Deploy Runbook

1. Use a published `ghcr.io/javaquasar/hydracache-server` image (see
   `docs/PUBLISHING.md`), or build one from the checked-in Dockerfile.
2. Create the `hydracache-mtls` secret with `tls.crt`, `tls.key`, and `ca.crt`.
3. Create the cluster peer-auth secret with the shared token in item `token`
   (a TLS member rejects unauthenticated raft peers and refuses to start
   without it):
   `kubectl create secret generic hydracache-cluster-auth --from-file=token`.
4. Apply `deploy/k8s/service.yaml`, `statefulset.yaml`, and `pdb.yaml`, or install
   the Helm chart from `deploy/helm/hydracache`:
   `helm install hydracache deploy/helm/hydracache --set tls.secretName=hydracache-mtls --set clusterAuth.keyId=cluster-v1 --set clusterAuth.secretName=hydracache-cluster-auth`.
   The chart refuses to render a configuration the server would refuse to
   start with (TLS without these secrets, plaintext without
   `tls.acknowledgeInsecure=true`, backup without a location).
5. Verify `/healthz`, `/readyz`, and the Prometheus exporter before admitting
   user traffic.
6. Backup is off by default. Enable it with `backup.enabled=true` and a real
   `backup.location` (or `HYDRACACHE_BACKUP_ENABLED`/`HYDRACACHE_BACKUP_LOCATION`
   in the raw manifest), confirm the destination credentials outside the chart,
   and run a restore drill.
