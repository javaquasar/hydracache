# Deploy Runbook

1. Build and publish the `hydracache-server` image from the checked-in Dockerfile.
2. Create the `hydracache-mtls` secret with `tls.crt`, `tls.key`, and `ca.crt`.
3. Apply `deploy/k8s/service.yaml`, `statefulset.yaml`, and `pdb.yaml`, or install
   the Helm chart from `deploy/helm/hydracache`.
4. Verify `/health`, `/ready`, and the Prometheus exporter before admitting user
   traffic.
5. Confirm backup location credentials outside the chart and run a restore drill.
