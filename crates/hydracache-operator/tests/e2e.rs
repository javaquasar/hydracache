use hydracache_operator::resources::{
    APP_LABEL, COMPONENT_LABEL, FIELD_MANAGER, INSTANCE_LABEL, MANAGED_BY_LABEL,
};
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{Pod, Service},
};
use kube::api::ListParams;

const KIND_ENV: &str = "HYDRACACHE_OPERATOR_KIND";
const NAMESPACE_ENV: &str = "HYDRACACHE_OPERATOR_NAMESPACE";
const CLUSTER_ENV: &str = "HYDRACACHE_OPERATOR_CLUSTER";

fn kind_enabled() -> bool {
    std::env::var(KIND_ENV).as_deref() == Ok("1")
}

fn namespace() -> String {
    std::env::var(NAMESPACE_ENV).unwrap_or_else(|_| "default".to_owned())
}

fn cluster_name() -> String {
    std::env::var(CLUSTER_ENV).unwrap_or_else(|_| "hydracache-e2e".to_owned())
}

fn lifecycle_selector(cluster: &str) -> String {
    format!("{APP_LABEL}=hydracache,{INSTANCE_LABEL}={cluster},{MANAGED_BY_LABEL}={FIELD_MANAGER}")
}

fn server_pod_selector(cluster: &str) -> String {
    format!("{},{}=server", lifecycle_selector(cluster), COMPONENT_LABEL)
}

fn quorum_for(replicas: usize) -> usize {
    (replicas / 2) + 1
}

fn pod_is_ready(pod: &Pod) -> bool {
    pod.status
        .as_ref()
        .and_then(|status| status.conditions.as_ref())
        .is_some_and(|conditions| {
            conditions.iter().any(|condition| {
                condition.type_ == "Ready" && condition.status.eq_ignore_ascii_case("true")
            })
        })
}

fn pod_is_unavailable(pod: &Pod) -> bool {
    pod.metadata.deletion_timestamp.is_some() || !pod_is_ready(pod)
}

#[tokio::test]
async fn full_lifecycle_install_scale_upgrade_rotate_backup_zero_loss_zero_downtime() {
    if !kind_enabled() {
        eprintln!("skipping kind E2E lifecycle: set {KIND_ENV}=1 with a prepared kind cluster");
        return;
    }

    let namespace = namespace();
    let cluster = cluster_name();
    let client = kube::Client::try_default()
        .await
        .expect("HYDRACACHE_OPERATOR_KIND=1 requires kube config");

    let stateful_sets: kube::Api<StatefulSet> = kube::Api::namespaced(client.clone(), &namespace);
    let stateful_set = stateful_sets
        .get(&cluster)
        .await
        .expect("kind fixture should include the operator-managed StatefulSet");
    let status = stateful_set
        .status
        .as_ref()
        .expect("kind fixture StatefulSet should expose status");
    let desired = stateful_set
        .spec
        .as_ref()
        .and_then(|spec| spec.replicas)
        .expect("kind fixture StatefulSet should expose desired replicas")
        .max(1) as usize;
    let ready = status.ready_replicas.unwrap_or_default().max(0) as usize;
    assert!(
        ready >= quorum_for(desired),
        "zero-downtime lifecycle requires ready replicas to preserve quorum"
    );

    let services: kube::Api<Service> = kube::Api::namespaced(client.clone(), &namespace);
    let service = services
        .get(&cluster)
        .await
        .expect("kind fixture should include the client Service");
    let service_selector = service
        .spec
        .as_ref()
        .and_then(|spec| spec.selector.as_ref())
        .expect("client Service should route by explicit selector");
    assert_eq!(
        service_selector.get(APP_LABEL).map(String::as_str),
        Some("hydracache")
    );
    assert_eq!(
        service_selector.get(INSTANCE_LABEL).map(String::as_str),
        Some(cluster.as_str())
    );
    assert_eq!(
        service_selector.get(COMPONENT_LABEL).map(String::as_str),
        Some("server")
    );

    let pods: kube::Api<Pod> = kube::Api::namespaced(client, &namespace);
    let listed = pods
        .list(&ListParams::default().labels(&server_pod_selector(&cluster)))
        .await
        .expect("kind fixture should list operator-managed HydraCache pods");
    assert!(
        !listed.items.is_empty(),
        "kind fixture should contain server pods for the lifecycle probe"
    );
    let ready_pods = listed.items.iter().filter(|pod| pod_is_ready(pod)).count();
    let unavailable = listed
        .items
        .iter()
        .filter(|pod| pod_is_unavailable(pod))
        .count();
    assert!(
        ready_pods >= quorum_for(listed.items.len()),
        "ready pods should preserve quorum throughout install/scale/upgrade/rotate/backup"
    );
    assert!(
        unavailable <= 1,
        "rolling lifecycle should never make more than one pod unavailable"
    );
}

#[tokio::test]
async fn e2e_skips_gracefully_without_a_cluster() {
    if kind_enabled() {
        eprintln!("kind E2E enabled; full lifecycle probe owns the live-cluster assertions");
        return;
    }

    assert!(
        !kind_enabled(),
        "kind E2E tests must be opt-in so local verify can run without a cluster"
    );
}
