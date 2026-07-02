# HydraCache Operator

`hydracache-operator` is the Kubernetes lifecycle controller for a
`HydraCacheCluster`. It is a deployable binary crate, not a public library API,
and stays `publish = false`; it is shipped through repository manifests and
container images instead of crates.io.

The operator orchestrates existing HydraCache primitives:

- W0 admin HTTP surface on `hydracache-server`: `/healthz`, `/readyz`,
  `/admin/status`, `/admin/drain`, `/admin/reshard`, `/admin/backup`.
- 0.43 online resharding for scale.
- 0.48 graceful drain, mTLS lifecycle, backup, and PITR surfaces.
- 0.51 persistence policy mapped to Kubernetes PVCs.

It does not add a new consistency model, storage engine, or application API.
The embedded `hydracache` library path remains Kubernetes-free.

## Helm And Operator Boundary

Use the Helm/static manifests for simple installs where Kubernetes owns only
basic placement and restart. Use the operator when HydraCache lifecycle actions
must be coordinated with cluster correctness:

- scale out and scale in with quorum guard, reshard, and drain-before-remove;
- rolling upgrade one pod at a time, with leader drain delayed until
  re-election;
- mTLS Secret rotation projected through the pod template;
- persistence preflight and PVC retention;
- scheduled backup/PITR orchestration;
- status conditions that fail loud instead of silently continuing.

The operator is not a general PaaS, not a cloud abstraction, and not a Helm
replacement for non-lifecycle-only installs.

## Install

Apply the CRD and least-privilege RBAC:

```powershell
kubectl apply -f deploy/operator/hydracacheclusters.hydracache.io.crd.yaml
kubectl apply -f deploy/operator/rbac.yaml
```

Run the operator deployment from the release image or a locally built image. The
controller uses namespace-scoped `Role`/`RoleBinding`, server-side apply, owner
references, and a Kubernetes `Lease` for leader election when multiple operator
replicas are running.

## CRD Reference

`HydraCacheCluster` is namespaced under `hydracache.io/v1alpha1`.

Required fields:

- `spec.image`: `hydracache-server` image.
- `spec.version`: server version used for rolling-upgrade compatibility checks.
- `spec.replicas`: desired member count, minimum 1.
- `spec.regions`: non-empty region/zone placement hints.

Optional fields:

- `spec.persistence`: `storageClassName`, `size`, and `reclaimPolicy`.
  `Retain` is the default for data safety; `Delete` must be explicit.
- `spec.tls.secretName`: Secret containing `tls.crt`, `tls.key`, and `ca.crt`.
- `spec.resources`: server container resource requirements.
- `spec.backupSchedule`: `schedule`, `location`, and `retention`.
  `location` is required when backups are enabled.

Observed status:

- `status.phase`: `Ready`, `Scaling`, `Rebalancing`, or `Upgrading`.
- `status.health`: `Healthy`, `Degraded`, or `Forming`.
- `status.observedReplicas`, `status.leader`, `status.lastBackup`.
- `status.conditions`: loud lifecycle, safety, backup, TLS, persistence, and
  restore conditions.

## Day-2 Operations

Scale:

```powershell
kubectl scale hydracachecluster demo --replicas=5
```

The operator creates new pods first, waits for readiness, triggers reshard, and
refuses scale-below-quorum. Scale-in reshard/drain runs before the StatefulSet
replica count is reduced.

Upgrade:

```powershell
kubectl patch hydracachecluster demo --type merge -p '{"spec":{"image":"ghcr.io/javaquasar/hydracache-server:0.56.0","version":"0.56.0"}}'
```

The StatefulSet uses `OnDelete`; the operator deletes one drained pod at a time.
Skipped version jumps outside the supported compatibility window are refused.

Rotate mTLS:

```powershell
kubectl create secret generic hydracache-mtls-next --from-file=tls.crt --from-file=tls.key --from-file=ca.crt
kubectl patch hydracachecluster demo --type merge -p '{"spec":{"tls":{"secretName":"hydracache-mtls-next"}}}'
```

The Secret fingerprint is stamped on the pod template, and pods rotate one at a
time. A leader pod is drained only after re-election.

Backup:

```powershell
kubectl patch hydracachecluster demo --type merge -p '{"spec":{"backupSchedule":{"schedule":"0 * * * *","location":"s3://bucket/hydracache/demo","retention":"168h"}}}'
```

Backups run only when the cluster is healthy and all replicas are ready. Missing
locations and failed admin backup calls set loud conditions and degrade health.

PITR restore is planned only into a fresh cluster. The restore plan carries the
target authority epoch so restored state cannot move epoch authority backwards.

Delete:

```powershell
kubectl delete hydracachecluster demo
```

PVCs are retained by default. Set `spec.persistence.reclaimPolicy: Delete` only
when the data should be deleted with the cluster.

## Health And Readiness

The client Service selects only server pods. Kubernetes readiness uses
`/readyz` on the admin port, and liveness uses `/healthz`. Draining pods stop
serving readiness before removal, so client traffic stays on ready members.

## E2E Gate

Local verify does not require a Kubernetes cluster. The kind E2E suite is
opt-in:

```powershell
cargo test -p hydracache-operator --locked --test e2e
$env:HYDRACACHE_OPERATOR_KIND='1'
$env:HYDRACACHE_OPERATOR_NAMESPACE='default'
$env:HYDRACACHE_OPERATOR_CLUSTER='hydracache-e2e'
cargo test -p hydracache-operator --locked --test e2e
Remove-Item Env:\HYDRACACHE_OPERATOR_KIND,Env:\HYDRACACHE_OPERATOR_NAMESPACE,Env:\HYDRACACHE_OPERATOR_CLUSTER -ErrorAction SilentlyContinue
```

Without `HYDRACACHE_OPERATOR_KIND=1`, the E2E tests skip cleanly. With a prepared
kind fixture, they assert that the operator-managed StatefulSet, client Service,
and pods preserve quorum and avoid more than one unavailable pod during the
install/scale/upgrade/rotate/backup lifecycle.
