use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use http::{Method, Request, Response, StatusCode};
use hydracache_operator::controller::{
    apply_cluster, cleanup_cluster, holds_leader_lease, reconcile, Ctx, Error, FINALIZER,
};
use hydracache_operator::crd::{sample_spec, HydraCacheCluster, PvcReclaimPolicy};
use hydracache_operator::resources::OwnedResources;
use k8s_openapi::api::coordination::v1::Lease;
use k8s_openapi::api::core::v1::PersistentVolumeClaim;
use kube::client::Body;
use kube::runtime::controller::Action;
use kube::Client;
use serde::Serialize;
use serde_json::{json, Value};
use tower::service_fn;

#[derive(Debug)]
struct ExpectedRequest {
    method: Method,
    path: String,
    status: StatusCode,
    body: Value,
    request_body_contains: Vec<String>,
}

impl ExpectedRequest {
    fn json<T: Serialize>(method: Method, path: impl Into<String>, body: &T) -> Self {
        Self {
            method,
            path: path.into(),
            status: StatusCode::OK,
            body: serde_json::to_value(body).unwrap(),
            request_body_contains: Vec::new(),
        }
    }

    fn status(method: Method, path: impl Into<String>, status: StatusCode) -> Self {
        let reason = status
            .canonical_reason()
            .unwrap_or("scripted Kubernetes API error");
        Self {
            method,
            path: path.into(),
            status,
            body: json!({
                "apiVersion": "v1",
                "kind": "Status",
                "status": "Failure",
                "message": reason,
                "reason": reason.replace(' ', ""),
                "code": status.as_u16(),
            }),
            request_body_contains: Vec::new(),
        }
    }

    fn requiring_body(mut self, expected: impl Into<String>) -> Self {
        self.request_body_contains.push(expected.into());
        self
    }
}

#[derive(Clone)]
struct ScriptedApi {
    pending: Arc<Mutex<VecDeque<ExpectedRequest>>>,
}

impl ScriptedApi {
    fn client(steps: Vec<ExpectedRequest>) -> (Client, Self) {
        let script = Self {
            pending: Arc::new(Mutex::new(steps.into())),
        };
        let service_script = script.clone();
        let service = service_fn(move |request: Request<Body>| {
            let script = service_script.clone();
            async move {
                let expected = script
                    .pending
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("unexpected Kubernetes API request");
                assert_eq!(request.method(), expected.method);
                assert_eq!(request.uri().path(), expected.path);
                if !expected.request_body_contains.is_empty() {
                    let body = request.into_body().collect_bytes().await.unwrap();
                    let body = String::from_utf8(body.to_vec()).unwrap();
                    for fragment in expected.request_body_contains {
                        assert!(
                            body.contains(&fragment),
                            "request body did not contain {fragment:?}: {body}"
                        );
                    }
                }
                let response = Response::builder()
                    .status(expected.status)
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&expected.body).unwrap()))
                    .unwrap();
                Ok::<_, Infallible>(response)
            }
        });
        (Client::new(service, "default"), script)
    }

    fn assert_exhausted(&self) {
        let pending = self.pending.lock().unwrap();
        assert!(pending.is_empty(), "unconsumed API steps: {pending:#?}");
    }
}

fn cluster(name: &str) -> HydraCacheCluster {
    let mut spec = sample_spec();
    spec.replicas = 1;
    spec.persistence = None;
    spec.tls = None;
    spec.backup_schedule = None;
    let mut cluster = HydraCacheCluster::new(name, spec);
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(4);
    cluster
}

fn not_found(method: Method, path: impl Into<String>) -> ExpectedRequest {
    ExpectedRequest::status(method, path, StatusCode::NOT_FOUND)
}

#[tokio::test]
async fn reconcile_apply_uses_real_kube_api_paths_and_patches_status() {
    let mut cluster = cluster("api-apply");
    cluster.metadata.finalizers = Some(vec![FINALIZER.to_owned()]);
    let desired = OwnedResources::build(&cluster);
    let resource_root = "/apis/apps/v1/namespaces/default/statefulsets/api-apply";
    let lease = hydracache_operator::controller::operator_lease_for_cluster(&cluster, "operator-a");
    let (client, script) = ScriptedApi::client(vec![
        ExpectedRequest::json(
            Method::GET,
            "/apis/coordination.k8s.io/v1/namespaces/default/leases/api-apply-operator",
            &lease,
        ),
        not_found(Method::GET, resource_root),
        ExpectedRequest::json(Method::PATCH, resource_root, &desired.stateful_set),
        ExpectedRequest::json(
            Method::PATCH,
            "/api/v1/namespaces/default/services/api-apply-headless",
            &desired.headless_service,
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/api/v1/namespaces/default/services/api-apply",
            &desired.client_service,
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/api/v1/namespaces/default/secrets/api-apply-operator-admin",
            &desired.admin_secret,
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/apis/policy/v1/namespaces/default/poddisruptionbudgets/api-apply",
            &desired.pod_disruption_budget,
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/apis/hydracache.io/v1alpha1/namespaces/default/hydracacheclusters/api-apply/status",
            &cluster,
        ),
    ]);

    let action = reconcile(
        Arc::new(cluster),
        Arc::new(Ctx::new(client, "operator-a", Some("default".to_owned()))),
    )
    .await
    .unwrap();

    assert_eq!(action, Action::requeue(Duration::from_secs(30)));
    script.assert_exhausted();
}

#[tokio::test]
async fn apply_cluster_api_failure_stops_before_any_mutation() {
    let cluster = cluster("apply-error");
    let (client, script) = ScriptedApi::client(vec![ExpectedRequest::status(
        Method::GET,
        "/apis/apps/v1/namespaces/default/statefulsets/apply-error",
        StatusCode::INTERNAL_SERVER_ERROR,
    )]);

    let error = apply_cluster(
        Arc::new(cluster),
        Arc::new(Ctx::new(client, "operator-a", Some("default".to_owned()))),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, Error::Kube(_)));
    script.assert_exhausted();
}

#[tokio::test]
async fn unavailable_admin_endpoint_is_recorded_in_status_without_undoing_workload_apply() {
    let cluster = cluster("admin-unavailable");
    let desired = OwnedResources::build(&cluster);
    let mut existing = desired.stateful_set.clone();
    existing.status = Some(k8s_openapi::api::apps::v1::StatefulSetStatus {
        replicas: 1,
        ready_replicas: Some(1),
        ..Default::default()
    });
    let resource_root = "/apis/apps/v1/namespaces/default/statefulsets/admin-unavailable";
    let (client, script) = ScriptedApi::client(vec![
        ExpectedRequest::json(Method::GET, resource_root, &existing),
        ExpectedRequest::json(Method::PATCH, resource_root, &desired.stateful_set),
        ExpectedRequest::json(
            Method::PATCH,
            "/api/v1/namespaces/default/services/admin-unavailable-headless",
            &desired.headless_service,
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/api/v1/namespaces/default/services/admin-unavailable",
            &desired.client_service,
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/api/v1/namespaces/default/secrets/admin-unavailable-operator-admin",
            &desired.admin_secret,
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/apis/policy/v1/namespaces/default/poddisruptionbudgets/admin-unavailable",
            &desired.pod_disruption_budget,
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/apis/hydracache.io/v1alpha1/namespaces/default/hydracacheclusters/admin-unavailable/status",
            &cluster,
        )
        .requiring_body("ScaleActionFailed")
        .requiring_body("AdminActionFailed"),
    ]);

    let action = apply_cluster(
        Arc::new(cluster),
        Arc::new(Ctx::new(client, "operator-a", Some("default".to_owned()))),
    )
    .await
    .unwrap();

    assert_eq!(action, Action::requeue(Duration::from_secs(30)));
    script.assert_exhausted();
}

#[tokio::test]
async fn blocked_persistence_patches_degraded_status_without_applying_resources() {
    let mut cluster = cluster("persistence-blocked");
    cluster.spec.persistence = sample_spec().persistence;
    cluster
        .spec
        .persistence
        .as_mut()
        .unwrap()
        .storage_class_name = " ".to_owned();
    let (client, script) = ScriptedApi::client(vec![
        not_found(
            Method::GET,
            "/apis/apps/v1/namespaces/default/statefulsets/persistence-blocked",
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/apis/hydracache.io/v1alpha1/namespaces/default/hydracacheclusters/persistence-blocked/status",
            &cluster,
        ),
    ]);

    let action = apply_cluster(
        Arc::new(cluster),
        Arc::new(Ctx::new(client, "operator-a", Some("default".to_owned()))),
    )
    .await
    .unwrap();
    assert_eq!(action, Action::requeue(Duration::from_secs(30)));
    script.assert_exhausted();
}

#[tokio::test]
async fn missing_tls_secret_patches_degraded_status_without_workload_apply() {
    let mut cluster = cluster("tls-blocked");
    cluster.spec.tls = sample_spec().tls;
    let (client, script) = ScriptedApi::client(vec![
        not_found(
            Method::GET,
            "/apis/apps/v1/namespaces/default/statefulsets/tls-blocked",
        ),
        not_found(
            Method::GET,
            "/api/v1/namespaces/default/secrets/hydracache-mtls",
        ),
        ExpectedRequest::json(
            Method::PATCH,
            "/apis/hydracache.io/v1alpha1/namespaces/default/hydracacheclusters/tls-blocked/status",
            &cluster,
        ),
    ]);

    let action = apply_cluster(
        Arc::new(cluster),
        Arc::new(Ctx::new(client, "operator-a", Some("default".to_owned()))),
    )
    .await
    .unwrap();
    assert_eq!(action, Action::requeue(Duration::from_secs(30)));
    script.assert_exhausted();
}

#[tokio::test]
async fn leader_lease_404_creates_once_and_conflict_loses_election() {
    let cluster = cluster("lease-create");
    let lease_path = "/apis/coordination.k8s.io/v1/namespaces/default/leases/lease-create-operator";
    let lease_collection = "/apis/coordination.k8s.io/v1/namespaces/default/leases";
    let mut created =
        hydracache_operator::controller::operator_lease_for_cluster(&cluster, "operator-a");
    created.metadata.resource_version = Some("1".to_owned());
    let (client, script) = ScriptedApi::client(vec![
        not_found(Method::GET, lease_path),
        ExpectedRequest::json(Method::POST, lease_collection, &created),
    ]);
    assert!(holds_leader_lease(client, &cluster, "operator-a")
        .await
        .unwrap());
    script.assert_exhausted();

    let (client, script) = ScriptedApi::client(vec![
        not_found(Method::GET, lease_path),
        ExpectedRequest::status(Method::POST, lease_collection, StatusCode::CONFLICT),
    ]);
    assert!(!holds_leader_lease(client, &cluster, "operator-a")
        .await
        .unwrap());
    script.assert_exhausted();
}

#[tokio::test]
async fn leader_lease_api_failure_is_not_misreported_as_not_leader() {
    let cluster = cluster("lease-error");
    let (client, script) = ScriptedApi::client(vec![ExpectedRequest::status(
        Method::GET,
        "/apis/coordination.k8s.io/v1/namespaces/default/leases/lease-error-operator",
        StatusCode::INTERNAL_SERVER_ERROR,
    )]);

    let error = holds_leader_lease(client, &cluster, "operator-a")
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Kube(_)));
    script.assert_exhausted();
}

#[tokio::test]
async fn reconcile_with_foreign_lease_requeues_without_mutating_resources() {
    let cluster = cluster("lease-foreign");
    let mut lease: Lease =
        hydracache_operator::controller::operator_lease_for_cluster(&cluster, "operator-b");
    lease.metadata.resource_version = Some("2".to_owned());
    let (client, script) = ScriptedApi::client(vec![ExpectedRequest::json(
        Method::GET,
        "/apis/coordination.k8s.io/v1/namespaces/default/leases/lease-foreign-operator",
        &lease,
    )]);

    let action = reconcile(
        Arc::new(cluster),
        Arc::new(Ctx::new(client, "operator-a", Some("default".to_owned()))),
    )
    .await
    .unwrap();
    assert_eq!(action, Action::requeue(Duration::from_secs(15)));
    script.assert_exhausted();
}

#[tokio::test]
async fn cleanup_delete_policy_lists_and_deletes_every_owned_pvc() {
    let mut cluster = cluster("cleanup-pvcs");
    cluster.spec.persistence = sample_spec().persistence;
    cluster.spec.persistence.as_mut().unwrap().reclaim_policy = PvcReclaimPolicy::Delete;
    let pvc = |name: &str| PersistentVolumeClaim {
        metadata: kube::core::ObjectMeta {
            name: Some(name.to_owned()),
            namespace: Some("default".to_owned()),
            ..Default::default()
        },
        ..Default::default()
    };
    let list = json!({
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaimList",
        "metadata": {"resourceVersion": "1"},
        "items": [pvc("data-cleanup-pvcs-0"), pvc("data-cleanup-pvcs-1")],
    });
    let deleted = json!({
        "apiVersion": "v1",
        "kind": "Status",
        "status": "Success",
        "code": 200,
    });
    let collection = "/api/v1/namespaces/default/persistentvolumeclaims";
    let (client, script) = ScriptedApi::client(vec![
        ExpectedRequest::json(Method::GET, collection, &list),
        ExpectedRequest::json(
            Method::DELETE,
            "/api/v1/namespaces/default/persistentvolumeclaims/data-cleanup-pvcs-0",
            &deleted,
        ),
        ExpectedRequest::json(
            Method::DELETE,
            "/api/v1/namespaces/default/persistentvolumeclaims/data-cleanup-pvcs-1",
            &deleted,
        ),
    ]);

    let action = cleanup_cluster(
        Arc::new(cluster),
        Arc::new(Ctx::new(client, "operator-a", Some("default".to_owned()))),
    )
    .await
    .unwrap();
    assert_eq!(action, Action::await_change());
    script.assert_exhausted();
}
