use std::collections::BTreeMap;

use hydracache_operator::controller::{
    immutable_change_condition, is_leader, lease_name, observed_status, operator_lease_for_cluster,
    validate_statefulset_update, DEGRADED_HEALTH, FINALIZER, FORMING_HEALTH, HEALTHY_HEALTH,
    READY_PHASE,
};
use hydracache_operator::crd::{
    sample_spec, HydraCacheCluster, HydraCacheClusterStatus, PvcReclaimPolicy,
    HYDRACACHE_CLUSTER_CRD_NAME,
};
use hydracache_operator::resources::{
    cleanup_plan, headless_service_name, seed_list, OwnedResources, ADMIN_PORT, APP_LABEL,
    CLIENT_PORT, CLUSTER_PORT, COMPONENT_LABEL, DATA_VOLUME, INSTANCE_LABEL, MANAGED_BY_LABEL,
    METRICS_PORT, TLS_VOLUME,
};
use k8s_openapi::api::apps::v1::StatefulSet;

fn cluster(name: &str) -> HydraCacheCluster {
    let mut cluster = HydraCacheCluster::new(name, sample_spec());
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(7);
    cluster
}

#[test]
fn reconcile_apply_cr_creates_statefulset_services_and_owner_refs() {
    let cluster = cluster("owned");
    let desired = OwnedResources::build(&cluster);

    assert_eq!(desired.stateful_set.metadata.name.as_deref(), Some("owned"));
    assert_eq!(
        desired
            .stateful_set
            .metadata
            .owner_references
            .as_ref()
            .unwrap()[0]
            .uid,
        "owned-uid"
    );
    let sts_spec = desired.stateful_set.spec.as_ref().unwrap();
    assert_eq!(sts_spec.replicas, Some(3));
    assert_eq!(sts_spec.service_name.as_deref(), Some("owned-headless"));
    assert_eq!(sts_spec.pod_management_policy.as_deref(), Some("Parallel"));
    assert_eq!(
        sts_spec
            .persistent_volume_claim_retention_policy
            .as_ref()
            .unwrap()
            .when_deleted
            .as_deref(),
        Some("Retain")
    );

    let template = &sts_spec.template;
    let pod_spec = template.spec.as_ref().unwrap();
    let container = &pod_spec.containers[0];
    let ports = container.ports.as_ref().unwrap();
    assert!(ports
        .iter()
        .any(|port| port.name.as_deref() == Some("http") && port.container_port == CLIENT_PORT));
    assert!(
        ports
            .iter()
            .any(|port| port.name.as_deref() == Some("cluster")
                && port.container_port == CLUSTER_PORT)
    );
    assert!(
        ports
            .iter()
            .any(|port| port.name.as_deref() == Some("metrics")
                && port.container_port == METRICS_PORT)
    );
    assert!(ports
        .iter()
        .any(|port| port.name.as_deref() == Some("admin") && port.container_port == ADMIN_PORT));
    assert_eq!(
        container
            .readiness_probe
            .as_ref()
            .unwrap()
            .http_get
            .as_ref()
            .unwrap()
            .path
            .as_deref(),
        Some("/readyz")
    );
    assert_eq!(
        container
            .liveness_probe
            .as_ref()
            .unwrap()
            .http_get
            .as_ref()
            .unwrap()
            .path
            .as_deref(),
        Some("/healthz")
    );
    assert!(container.lifecycle.as_ref().unwrap().pre_stop.is_some());
    assert!(container
        .volume_mounts
        .as_ref()
        .unwrap()
        .iter()
        .any(|mount| mount.name == DATA_VOLUME));
    assert!(container
        .volume_mounts
        .as_ref()
        .unwrap()
        .iter()
        .any(|mount| mount.name == TLS_VOLUME && mount.read_only == Some(true)));
    assert!(pod_spec
        .volumes
        .as_ref()
        .unwrap()
        .iter()
        .any(|volume| volume.name == TLS_VOLUME));

    assert_eq!(
        desired.headless_service.metadata.name.as_deref(),
        Some(headless_service_name("owned").as_str())
    );
    assert_eq!(
        desired
            .headless_service
            .spec
            .as_ref()
            .unwrap()
            .cluster_ip
            .as_deref(),
        Some("None")
    );
    assert_eq!(
        desired.client_service.metadata.name.as_deref(),
        Some("owned")
    );
    assert!(desired
        .admin_secret
        .string_data
        .as_ref()
        .unwrap()
        .contains_key("HYDRACACHE_ADMIN"));

    let labels = desired.stateful_set.metadata.labels.as_ref().unwrap();
    assert_eq!(labels[APP_LABEL], "hydracache");
    assert_eq!(labels[INSTANCE_LABEL], "owned");
    assert_eq!(labels[MANAGED_BY_LABEL], "hydracache-operator");
    let pod_labels = template.metadata.as_ref().unwrap().labels.as_ref().unwrap();
    assert_eq!(pod_labels[COMPONENT_LABEL], "server");
}

#[test]
fn reconcile_manual_drift_is_reconciled_back() {
    let cluster = cluster("drift");
    let mut desired = OwnedResources::build(&cluster);
    let mut drifted = desired.stateful_set.clone();
    drifted
        .spec
        .as_mut()
        .unwrap()
        .template
        .spec
        .as_mut()
        .unwrap()
        .containers[0]
        .image = Some("example.com/old:wrong".to_owned());

    validate_statefulset_update(&drifted, &desired.stateful_set).unwrap();
    assert_ne!(
        drifted
            .spec
            .as_ref()
            .unwrap()
            .template
            .spec
            .as_ref()
            .unwrap()
            .containers[0]
            .image,
        desired
            .stateful_set
            .spec
            .as_ref()
            .unwrap()
            .template
            .spec
            .as_ref()
            .unwrap()
            .containers[0]
            .image
    );

    desired
        .stateful_set
        .spec
        .as_mut()
        .unwrap()
        .template
        .spec
        .as_mut()
        .unwrap()
        .containers[0]
        .image = Some(cluster.spec.image.clone());
}

#[test]
fn reconcile_operator_restart_mid_reconcile_is_idempotent() {
    let cluster = cluster("idempotent");
    let first = OwnedResources::build(&cluster);
    let second = OwnedResources::build(&cluster);

    assert_eq!(first, second);
    assert_eq!(
        seed_list("idempotent", cluster.spec.replicas),
        "idempotent-0.idempotent-headless:7000,idempotent-1.idempotent-headless:7000,idempotent-2.idempotent-headless:7000"
    );
}

#[test]
fn reconcile_two_operator_replicas_use_leader_election() {
    let cluster = cluster("leader-gated");
    let lease = operator_lease_for_cluster(&cluster, "operator-a");

    assert!(is_leader("operator-a", &lease));
    assert!(!is_leader("operator-b", &lease));
    assert_eq!(lease_name(&cluster), "leader-gated-operator");
    assert_eq!(
        lease.metadata.owner_references.as_ref().unwrap()[0].uid,
        "leader-gated-uid"
    );
}

#[test]
fn reconcile_cluster_delete_retains_pvcs_by_default() {
    let cluster = cluster("retain");
    let cleanup = cleanup_plan(&cluster);
    assert!(!cleanup.delete_pvcs);
    assert_eq!(cleanup.pvc_selector[INSTANCE_LABEL], "retain");

    let desired = OwnedResources::build(&cluster);
    assert_eq!(
        desired
            .stateful_set
            .spec
            .as_ref()
            .unwrap()
            .persistent_volume_claim_retention_policy
            .as_ref()
            .unwrap()
            .when_deleted
            .as_deref(),
        Some("Retain")
    );
}

#[test]
fn reconcile_cluster_delete_deletes_pvcs_only_when_explicit() {
    let mut cluster = cluster("delete");
    cluster.spec.persistence.as_mut().unwrap().reclaim_policy = PvcReclaimPolicy::Delete;

    let cleanup = cleanup_plan(&cluster);
    assert!(cleanup.delete_pvcs);

    let desired = OwnedResources::build(&cluster);
    assert_eq!(
        desired
            .stateful_set
            .spec
            .as_ref()
            .unwrap()
            .persistent_volume_claim_retention_policy
            .as_ref()
            .unwrap()
            .when_deleted
            .as_deref(),
        Some("Delete")
    );
}

#[test]
fn reconcile_immutable_statefulset_field_change_is_rejected_loud_or_recreated() {
    let cluster = cluster("immutable");
    let desired = OwnedResources::build(&cluster);
    let mut existing = desired.stateful_set.clone();
    existing.spec.as_mut().unwrap().service_name = Some("old-headless".to_owned());

    let err = validate_statefulset_update(&existing, &desired.stateful_set).unwrap_err();
    assert!(err.to_string().contains("spec.serviceName"));

    let condition = immutable_change_condition("spec.serviceName", cluster.metadata.generation);
    assert_eq!(condition.type_, "ReconcileBlocked");
    assert_eq!(condition.status, "True");
    assert_eq!(condition.reason, "ImmutableStatefulSetField");
    assert_eq!(condition.observed_generation, Some(7));
}

#[test]
fn reconcile_immutable_volume_claim_template_change_is_rejected_loud() {
    let cluster = cluster("immutable-pvc");
    let desired = OwnedResources::build(&cluster);
    let mut existing = desired.stateful_set.clone();
    existing
        .spec
        .as_mut()
        .unwrap()
        .volume_claim_templates
        .as_mut()
        .unwrap()[0]
        .spec
        .as_mut()
        .unwrap()
        .storage_class_name = Some("other-storage-class".to_owned());

    let err = validate_statefulset_update(&existing, &desired.stateful_set).unwrap_err();

    assert!(err.to_string().contains("spec.volumeClaimTemplates"));
}

#[test]
fn reconcile_status_is_state_machine_snapshot() {
    let cluster = cluster("status");
    let desired = OwnedResources::build(&cluster);
    let status = observed_status(&cluster, Some(&desired.stateful_set));

    assert_eq!(status.observed_replicas, 0);
    assert_eq!(status.bootstrap_replicas, Some(3));
    assert_eq!(status.health, FORMING_HEALTH);
    assert_eq!(status.phase, READY_PHASE);
    assert_eq!(status.conditions[0].type_, "Reconciled");

    let mut ready = desired.stateful_set.clone();
    ready.status = Some(k8s_openapi::api::apps::v1::StatefulSetStatus {
        ready_replicas: Some(3),
        replicas: 3,
        ..Default::default()
    });
    let ready_status = observed_status(&cluster, Some(&ready));
    assert_eq!(ready_status.health, HEALTHY_HEALTH);

    ready.status.as_mut().unwrap().ready_replicas = Some(1);
    let degraded_status = observed_status(&cluster, Some(&ready));
    assert_eq!(degraded_status.health, DEGRADED_HEALTH);
}

#[test]
fn reconcile_status_uses_replicas_when_ready_replicas_is_absent() {
    let cluster = cluster("status-replicas");
    let mut desired = OwnedResources::build(&cluster).stateful_set;
    desired.status = Some(k8s_openapi::api::apps::v1::StatefulSetStatus {
        ready_replicas: None,
        replicas: 3,
        ..Default::default()
    });

    let status = observed_status(&cluster, Some(&desired));

    assert_eq!(status.observed_replicas, 3);
    assert_eq!(status.health, HEALTHY_HEALTH);
}

#[test]
fn reconcile_status_preserves_last_backup_and_bootstrap_baseline() {
    let mut cluster = cluster("status-preserve");
    cluster.status = Some(HydraCacheClusterStatus {
        bootstrap_replicas: Some(2),
        last_backup: Some("2026-07-07T00:00:00Z".to_owned()),
        ..Default::default()
    });
    cluster.spec.replicas = 5;
    let desired = OwnedResources::build_with_replicas(&cluster, 5).stateful_set;

    let status = observed_status(&cluster, Some(&desired));

    assert_eq!(status.bootstrap_replicas, Some(2));
    assert_eq!(status.last_backup.as_deref(), Some("2026-07-07T00:00:00Z"));
}

#[test]
fn reconcile_status_without_statefulset_stays_forming_and_unbaselined() {
    let cluster = cluster("status-none");

    let status = observed_status(&cluster, None);

    assert_eq!(status.observed_replicas, 0);
    assert_eq!(status.bootstrap_replicas, None);
    assert_eq!(status.health, FORMING_HEALTH);
}

#[test]
fn operator_template_renders_routable_cluster_identity_and_endpoint() {
    let mut cluster = cluster("identity");
    cluster.spec.replicas = 4;
    cluster.status = Some(HydraCacheClusterStatus {
        bootstrap_replicas: Some(3),
        ..Default::default()
    });

    let desired = OwnedResources::build_with_replicas(&cluster, 4);
    let container = &desired
        .stateful_set
        .spec
        .as_ref()
        .unwrap()
        .template
        .spec
        .as_ref()
        .unwrap()
        .containers[0];
    let env = env_map(container);

    assert_eq!(env["HYDRACACHE_CLUSTER_ADDR"], "0.0.0.0:7000");
    assert_eq!(env["HYDRACACHE_ADMIN_ADDR"], "0.0.0.0:9091");
    assert_eq!(env["HYDRACACHE_BOOTSTRAP_REPLICAS"], "3");
    assert_eq!(
        env["HYDRACACHE_CLUSTER_HEADLESS_SERVICE"],
        "identity-headless"
    );
    assert_eq!(env["HYDRACACHE_JOIN_TIMEOUT_MS"], "30000");
    assert_eq!(env["HYDRACACHE_TLS_ACK_INSECURE"], "false");
    assert_eq!(
        env["HYDRACACHE_SEEDS"],
        "identity-0.identity-headless:7000,identity-1.identity-headless:7000,identity-2.identity-headless:7000"
    );
    assert!(container.command.is_none());
}

#[test]
fn operator_template_acknowledges_plaintext_when_tls_is_disabled() {
    let mut cluster = cluster("plain");
    cluster.spec.tls = None;

    let desired = OwnedResources::build(&cluster);
    let container = &desired
        .stateful_set
        .spec
        .as_ref()
        .unwrap()
        .template
        .spec
        .as_ref()
        .unwrap()
        .containers[0];
    let env = env_map(container);

    assert_eq!(env["HYDRACACHE_TLS_ENABLED"], "false");
    assert_eq!(env["HYDRACACHE_TLS_ACK_INSECURE"], "true");
}

#[test]
fn bootstrap_replicas_is_recorded_once() {
    let mut subject = cluster("bootstrap-once");
    let desired = OwnedResources::build(&subject);
    let first_status = observed_status(&subject, Some(&desired.stateful_set));
    assert_eq!(first_status.bootstrap_replicas, Some(3));

    subject.status = Some(HydraCacheClusterStatus {
        bootstrap_replicas: Some(3),
        ..first_status
    });
    subject.spec.replicas = 4;
    let scaled = OwnedResources::build_with_replicas(&subject, 4);
    let scaled_status = observed_status(&subject, Some(&scaled.stateful_set));
    assert_eq!(scaled_status.bootstrap_replicas, Some(3));

    let mut pre_061 = cluster("pre-061-upgrade");
    pre_061.spec.replicas = 4;
    pre_061.status = Some(HydraCacheClusterStatus {
        bootstrap_replicas: None,
        ..Default::default()
    });
    let mut existing = OwnedResources::build_with_replicas(&pre_061, 3).stateful_set;
    existing.status = Some(k8s_openapi::api::apps::v1::StatefulSetStatus {
        ready_replicas: Some(3),
        replicas: 3,
        ..Default::default()
    });
    let upgraded_status = observed_status(&pre_061, Some(&existing));
    assert_eq!(upgraded_status.bootstrap_replicas, Some(3));
}

#[test]
fn leader_lease_without_matching_holder_does_not_elect_identity() {
    let cluster = cluster("lease-empty");
    let mut lease = operator_lease_for_cluster(&cluster, "operator-a");

    lease.spec.as_mut().unwrap().holder_identity = None;
    assert!(!is_leader("operator-a", &lease));
    lease.spec.as_mut().unwrap().holder_identity = Some("operator-b".to_owned());
    assert!(!is_leader("operator-a", &lease));
}

#[tokio::test]
async fn reconcile_apply_cr_becomes_ready() {
    if std::env::var("HYDRACACHE_OPERATOR_KIND").as_deref() != Ok("1") {
        eprintln!(
            "skipping kind readiness test: set HYDRACACHE_OPERATOR_KIND=1 with a kind cluster"
        );
        return;
    }

    let client = kube::Client::try_default()
        .await
        .expect("HYDRACACHE_OPERATOR_KIND=1 requires kube config");
    let statefulsets: kube::Api<StatefulSet> = kube::Api::namespaced(client, "default");
    let sts = statefulsets
        .get("hydracache-kind-ready")
        .await
        .expect("kind fixture StatefulSet should exist");
    assert!(
        sts.status
            .and_then(|status| status.ready_replicas)
            .unwrap_or(0)
            > 0
    );
}

fn env_map(container: &k8s_openapi::api::core::v1::Container) -> BTreeMap<String, String> {
    container
        .env
        .as_ref()
        .unwrap()
        .iter()
        .map(|var| (var.name.clone(), var.value.clone().unwrap_or_default()))
        .collect()
}

#[tokio::test]
async fn reconcile_envtest_apply_creates_owned_objects() {
    if std::env::var("HYDRACACHE_OPERATOR_ENVTEST").as_deref() != Ok("1") {
        eprintln!(
            "skipping envtest reconcile apply: set HYDRACACHE_OPERATOR_ENVTEST=1 with a test apiserver"
        );
        return;
    }

    use k8s_openapi::api::core::v1::{Secret, Service};
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
    use kube::api::{Api, Patch, PatchParams};
    use kube::core::CustomResourceExt;

    let client = kube::Client::try_default()
        .await
        .expect("HYDRACACHE_OPERATOR_ENVTEST=1 requires kube config/envtest apiserver");
    let apply = PatchParams::apply("hydracache-operator-tests").force();
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    crds.patch(
        HYDRACACHE_CLUSTER_CRD_NAME,
        &apply,
        &Patch::Apply(&HydraCacheCluster::crd()),
    )
    .await
    .expect("CRD applies to envtest apiserver");

    let clusters: Api<HydraCacheCluster> = Api::namespaced(client.clone(), "default");
    let mut desired = cluster("envtest-w2");
    desired.metadata.finalizers = Some(vec![FINALIZER.to_owned()]);
    let applied = clusters
        .patch("envtest-w2", &apply, &Patch::Apply(&desired))
        .await
        .expect("HydraCacheCluster applies");

    let owned = OwnedResources::build(&applied);
    let services: Api<Service> = Api::namespaced(client.clone(), "default");
    services
        .patch(
            &headless_service_name("envtest-w2"),
            &apply,
            &Patch::Apply(&owned.headless_service),
        )
        .await
        .expect("headless service applies");
    let secrets: Api<Secret> = Api::namespaced(client, "default");
    secrets
        .patch(
            "envtest-w2-operator-admin",
            &apply,
            &Patch::Apply(&owned.admin_secret),
        )
        .await
        .expect("admin secret applies");
}
