# HydraCache Server Deployment

HydraCache 0.48 adds a standalone `hydracache-server` image and Kubernetes
artifacts for a stateful production-style deployment.

## Required Runtime Inputs

- stable member identity from the StatefulSet ordinal;
- persistent storage mounted at `HYDRACACHE_STORAGE_DIR`;
- mTLS material mounted at `/etc/hydracache/tls`;
- headless-service seeds in `HYDRACACHE_SEEDS`;
- off-host backup location in `HYDRACACHE_BACKUP_LOCATION`.

Non-loopback listeners require TLS. Local staging can set
`HYDRACACHE_TLS_ACK_INSECURE=true`, but production manifests should provide
`HYDRACACHE_TLS_ENABLED=true` plus certificate, key, and CA paths.

## Validation Gates

- `cargo test -p hydracache-server --locked deploy_smoke`
- nightly image build for `Dockerfile`
- nightly `kind` StatefulSet rolling-update and backup/restore drill

The checked-in Kubernetes files are intentionally conservative: three replicas,
a headless service for stable DNS, PVC-backed member storage, HTTP probes for
`/health` and `/ready`, and a PodDisruptionBudget that keeps quorum during
voluntary maintenance.
