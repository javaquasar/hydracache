# Deploy Runbook

1. Use a published `ghcr.io/javaquasar/hydracache-server` image (see
   `docs/PUBLISHING.md`), or build one from the checked-in Dockerfile.
2. Create the `hydracache-mtls` secret with `tls.crt`, `tls.key`, and `ca.crt`.
   Members dial each other by the seed names the chart renders,
   `<release>-<ordinal>.<release>-headless:7000`, and verify the peer
   certificate against that name. The certificate must therefore list every
   member's name as a DNS SAN, for example `hydracache-0.hydracache-headless`,
   `hydracache-1.hydracache-headless`, ... (add spare ordinals if you plan to
   scale out). A wildcard such as `*.hydracache-headless` does **not** work:
   rustls rejects a wildcard with a single label after it, so every peer
   connection fails certificate name verification and no leader is elected. The certificate needs both `serverAuth` and `clientAuth` usage.
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
   user traffic. Confirm the members formed one cluster: `/cluster/overview`
   on the admin port (9091) of every pod must list all members as `reachable`
   and name the same `leader`.
   If pods crash-loop with `timed out ... waiting for networked raft leader`,
   the message ends with the last raft peer error (a certificate name
   mismatch, an unresolvable seed, a rejected cluster-auth token). Fix the
   cause, then delete all member pods together
   (`kubectl delete pod -l app.kubernetes.io/name=hydracache`): pods that
   crash-looped on different back-off timers may otherwise never be up at the
   same time to elect the first leader.
6. Backup is off by default. Enable it with `backup.enabled=true` and a real
   `backup.location` (or `HYDRACACHE_BACKUP_ENABLED`/`HYDRACACHE_BACKUP_LOCATION`
   in the raw manifest), confirm the destination credentials outside the chart,
   and run a restore drill.
