//! Builders for Kubernetes objects owned by a `HydraCacheCluster`.

use std::collections::BTreeMap;

use k8s_openapi::api::apps::v1::{
    StatefulSet, StatefulSetPersistentVolumeClaimRetentionPolicy, StatefulSetSpec,
    StatefulSetUpdateStrategy,
};
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, EmptyDirVolumeSource, EnvVar, HTTPGetAction, Lifecycle,
    LifecycleHandler, PersistentVolumeClaim, PersistentVolumeClaimSpec, PodSpec, PodTemplateSpec,
    Probe, Secret, SecretVolumeSource, Service, ServicePort, ServiceSpec, Volume, VolumeMount,
    VolumeResourceRequirements,
};
use k8s_openapi::api::policy::v1::{PodDisruptionBudget, PodDisruptionBudgetSpec};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta, OwnerReference};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::ResourceExt;

use crate::crd::{HydraCacheCluster, PvcReclaimPolicy};
use crate::scale::quorum_for;
use crate::tls::{TLS_SECRET_FINGERPRINT_ANNOTATION, TLS_SECRET_NAME_ANNOTATION};
use crate::upgrade::VERSION_ANNOTATION;

pub const APP_LABEL: &str = "app.kubernetes.io/name";
pub const INSTANCE_LABEL: &str = "app.kubernetes.io/instance";
pub const MANAGED_BY_LABEL: &str = "app.kubernetes.io/managed-by";
pub const COMPONENT_LABEL: &str = "app.kubernetes.io/component";
pub const FIELD_MANAGER: &str = "hydracache-operator";
pub const SERVER_CONTAINER: &str = "hydracache";
pub const DATA_VOLUME: &str = "data";
pub const TLS_VOLUME: &str = "tls";
pub const CLIENT_PORT: i32 = 8080;
pub const CLUSTER_PORT: i32 = 7000;
pub const METRICS_PORT: i32 = 9090;
pub const ADMIN_PORT: i32 = 9091;

/// Complete desired shape for the W2 reconcile pass.
#[derive(Clone, Debug, PartialEq)]
pub struct OwnedResources {
    pub stateful_set: StatefulSet,
    pub headless_service: Service,
    pub client_service: Service,
    pub admin_secret: Secret,
    pub pod_disruption_budget: PodDisruptionBudget,
}

impl OwnedResources {
    pub fn build(cluster: &HydraCacheCluster) -> Self {
        Self::build_with_replicas(cluster, cluster.spec.replicas)
    }

    pub fn build_with_replicas(cluster: &HydraCacheCluster, replicas: u32) -> Self {
        Self::build_with_replicas_and_tls_fingerprint(cluster, replicas, None)
    }

    pub fn build_with_replicas_and_tls_fingerprint(
        cluster: &HydraCacheCluster,
        replicas: u32,
        tls_secret_fingerprint: Option<&str>,
    ) -> Self {
        let bootstrap_replicas = cluster
            .status
            .as_ref()
            .and_then(|status| status.bootstrap_replicas)
            .unwrap_or(replicas)
            .max(1);
        Self::build_with_replicas_bootstrap_and_tls_fingerprint(
            cluster,
            replicas,
            bootstrap_replicas,
            tls_secret_fingerprint,
        )
    }

    pub fn build_with_replicas_bootstrap_and_tls_fingerprint(
        cluster: &HydraCacheCluster,
        replicas: u32,
        bootstrap_replicas: u32,
        tls_secret_fingerprint: Option<&str>,
    ) -> Self {
        let name = cluster.name_any();
        let namespace = cluster.namespace();
        let labels = base_labels(&name);
        let owner = owner_reference(cluster);
        let bootstrap_replicas = bootstrap_replicas.max(1);

        Self {
            stateful_set: stateful_set(
                cluster,
                namespace.clone(),
                &labels,
                owner.clone(),
                replicas,
                bootstrap_replicas,
                tls_secret_fingerprint,
            ),
            headless_service: headless_service(&name, namespace.clone(), &labels, owner.clone()),
            client_service: client_service(&name, namespace.clone(), &labels, owner.clone()),
            admin_secret: admin_secret(&name, namespace.clone(), &labels, owner.clone()),
            pod_disruption_budget: pod_disruption_budget(
                &name, namespace, &labels, owner, replicas,
            ),
        }
    }
}

/// Deletion-time plan. PVCs are retained unless the CR explicitly opts into deletion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupPlan {
    pub delete_pvcs: bool,
    pub pvc_selector: BTreeMap<String, String>,
}

pub fn cleanup_plan(cluster: &HydraCacheCluster) -> CleanupPlan {
    CleanupPlan {
        delete_pvcs: cluster
            .spec
            .persistence
            .as_ref()
            .is_some_and(|persistence| persistence.reclaim_policy == PvcReclaimPolicy::Delete),
        pvc_selector: pvc_selector_labels(&cluster.name_any()),
    }
}

pub fn base_labels(name: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        (APP_LABEL.to_owned(), "hydracache".to_owned()),
        (INSTANCE_LABEL.to_owned(), name.to_owned()),
        (MANAGED_BY_LABEL.to_owned(), FIELD_MANAGER.to_owned()),
    ])
}

pub fn pod_selector_labels(name: &str) -> BTreeMap<String, String> {
    let mut labels = base_labels(name);
    labels.insert(COMPONENT_LABEL.to_owned(), "server".to_owned());
    labels
}

pub fn pvc_selector_labels(name: &str) -> BTreeMap<String, String> {
    let mut labels = base_labels(name);
    labels.insert(COMPONENT_LABEL.to_owned(), "data".to_owned());
    labels
}

pub fn headless_service_name(name: &str) -> String {
    format!("{name}-headless")
}

pub fn admin_secret_name(name: &str) -> String {
    format!("{name}-operator-admin")
}

pub fn owner_reference(cluster: &HydraCacheCluster) -> Option<OwnerReference> {
    let uid = cluster.metadata.uid.clone()?;
    Some(OwnerReference {
        api_version: "hydracache.io/v1alpha1".to_owned(),
        block_owner_deletion: Some(true),
        controller: Some(true),
        kind: "HydraCacheCluster".to_owned(),
        name: cluster.name_any(),
        uid,
    })
}

pub fn stateful_set(
    cluster: &HydraCacheCluster,
    namespace: Option<String>,
    base_labels: &BTreeMap<String, String>,
    owner: Option<OwnerReference>,
    replicas: u32,
    bootstrap_replicas: u32,
    tls_secret_fingerprint: Option<&str>,
) -> StatefulSet {
    let name = cluster.name_any();
    let pod_labels = pod_selector_labels(&name);
    StatefulSet {
        metadata: object_meta(name.clone(), namespace.clone(), base_labels, owner.clone()),
        spec: Some(StatefulSetSpec {
            persistent_volume_claim_retention_policy: Some(pvc_retention_policy(cluster)),
            pod_management_policy: Some("Parallel".to_owned()),
            replicas: Some(replicas as i32),
            selector: LabelSelector {
                match_labels: Some(pod_labels.clone()),
                ..Default::default()
            },
            service_name: Some(headless_service_name(&name)),
            template: pod_template(
                cluster,
                namespace,
                &pod_labels,
                bootstrap_replicas,
                tls_secret_fingerprint,
            ),
            update_strategy: Some(StatefulSetUpdateStrategy {
                type_: Some("OnDelete".to_owned()),
                ..Default::default()
            }),
            volume_claim_templates: cluster
                .spec
                .persistence
                .as_ref()
                .map(|_| vec![data_volume_claim(cluster, owner)]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn pod_disruption_budget(
    name: &str,
    namespace: Option<String>,
    labels: &BTreeMap<String, String>,
    owner: Option<OwnerReference>,
    replicas: u32,
) -> PodDisruptionBudget {
    PodDisruptionBudget {
        metadata: object_meta(name.to_owned(), namespace, labels, owner),
        spec: Some(PodDisruptionBudgetSpec {
            min_available: Some(IntOrString::Int(quorum_for(replicas) as i32)),
            selector: Some(LabelSelector {
                match_labels: Some(pod_selector_labels(name)),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn headless_service(
    name: &str,
    namespace: Option<String>,
    labels: &BTreeMap<String, String>,
    owner: Option<OwnerReference>,
) -> Service {
    Service {
        metadata: object_meta(headless_service_name(name), namespace, labels, owner),
        spec: Some(ServiceSpec {
            cluster_ip: Some("None".to_owned()),
            publish_not_ready_addresses: Some(true),
            selector: Some(pod_selector_labels(name)),
            ports: Some(vec![
                service_port("cluster", CLUSTER_PORT),
                service_port("admin", ADMIN_PORT),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn client_service(
    name: &str,
    namespace: Option<String>,
    labels: &BTreeMap<String, String>,
    owner: Option<OwnerReference>,
) -> Service {
    Service {
        metadata: object_meta(name.to_owned(), namespace, labels, owner),
        spec: Some(ServiceSpec {
            selector: Some(pod_selector_labels(name)),
            ports: Some(vec![
                service_port("http", CLIENT_PORT),
                service_port("metrics", METRICS_PORT),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub fn admin_secret(
    name: &str,
    namespace: Option<String>,
    labels: &BTreeMap<String, String>,
    owner: Option<OwnerReference>,
) -> Secret {
    Secret {
        metadata: object_meta(admin_secret_name(name), namespace, labels, owner),
        string_data: Some(BTreeMap::from([
            ("HYDRACACHE_CLIENT_ID".to_owned(), "operator".to_owned()),
            ("HYDRACACHE_TENANT".to_owned(), "system".to_owned()),
            ("HYDRACACHE_ADMIN".to_owned(), "true".to_owned()),
        ])),
        type_: Some("Opaque".to_owned()),
        ..Default::default()
    }
}

fn pod_template(
    cluster: &HydraCacheCluster,
    namespace: Option<String>,
    pod_labels: &BTreeMap<String, String>,
    bootstrap_replicas: u32,
    tls_secret_fingerprint: Option<&str>,
) -> PodTemplateSpec {
    let mut volumes = Vec::new();
    let mut mounts = vec![VolumeMount {
        mount_path: "/var/lib/hydracache".to_owned(),
        name: DATA_VOLUME.to_owned(),
        ..Default::default()
    }];

    if cluster.spec.persistence.is_none() {
        volumes.push(Volume {
            name: DATA_VOLUME.to_owned(),
            empty_dir: Some(EmptyDirVolumeSource::default()),
            ..Default::default()
        });
    }

    if let Some(tls) = &cluster.spec.tls {
        volumes.push(Volume {
            name: TLS_VOLUME.to_owned(),
            secret: Some(SecretVolumeSource {
                secret_name: Some(tls.secret_name.clone()),
                ..Default::default()
            }),
            ..Default::default()
        });
        mounts.push(VolumeMount {
            mount_path: "/etc/hydracache/tls".to_owned(),
            name: TLS_VOLUME.to_owned(),
            read_only: Some(true),
            ..Default::default()
        });
    }

    let mut annotations =
        BTreeMap::from([(VERSION_ANNOTATION.to_owned(), cluster.spec.version.clone())]);
    if let Some(tls) = &cluster.spec.tls {
        annotations.insert(
            TLS_SECRET_NAME_ANNOTATION.to_owned(),
            tls.secret_name.clone(),
        );
        if let Some(fingerprint) = tls_secret_fingerprint {
            annotations.insert(
                TLS_SECRET_FINGERPRINT_ANNOTATION.to_owned(),
                fingerprint.to_owned(),
            );
        }
    }

    PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(pod_labels.clone()),
            annotations: Some(annotations),
            namespace,
            ..Default::default()
        }),
        spec: Some(PodSpec {
            containers: vec![Container {
                env: Some(server_env(cluster, bootstrap_replicas)),
                image: Some(cluster.spec.image.clone()),
                image_pull_policy: Some("IfNotPresent".to_owned()),
                lifecycle: Some(Lifecycle {
                    pre_stop: Some(LifecycleHandler {
                        http_get: Some(HTTPGetAction {
                            path: Some("/admin/drain".to_owned()),
                            port: IntOrString::String("admin".to_owned()),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                liveness_probe: Some(http_probe("/healthz", 10, 3)),
                name: SERVER_CONTAINER.to_owned(),
                ports: Some(vec![
                    container_port("http", CLIENT_PORT),
                    container_port("cluster", CLUSTER_PORT),
                    container_port("metrics", METRICS_PORT),
                    container_port("admin", ADMIN_PORT),
                ]),
                readiness_probe: Some(http_probe("/readyz", 5, 2)),
                resources: cluster.spec.resources.clone(),
                volume_mounts: Some(mounts),
                ..Default::default()
            }],
            termination_grace_period_seconds: Some(45),
            volumes: (!volumes.is_empty()).then_some(volumes),
            ..Default::default()
        }),
    }
}

fn server_env(cluster: &HydraCacheCluster, bootstrap_replicas: u32) -> Vec<EnvVar> {
    let tls_enabled = cluster.spec.tls.is_some().to_string();
    let tls_ack_insecure = cluster.spec.tls.is_none().to_string();
    let backup_location = cluster
        .spec
        .backup_schedule
        .as_ref()
        .map(|backup| backup.location.as_str())
        .unwrap_or("");
    vec![
        env("HYDRACACHE_ROLE", "member"),
        env("HYDRACACHE_LISTEN_ADDR", "0.0.0.0:8080"),
        env("HYDRACACHE_CLUSTER_ADDR", "0.0.0.0:7000"),
        env(
            "HYDRACACHE_BOOTSTRAP_REPLICAS",
            &bootstrap_replicas.to_string(),
        ),
        env(
            "HYDRACACHE_CLUSTER_HEADLESS_SERVICE",
            &headless_service_name(&cluster.name_any()),
        ),
        env(
            "HYDRACACHE_SEEDS",
            &seed_list(&cluster.name_any(), bootstrap_replicas),
        ),
        env("HYDRACACHE_JOIN_TIMEOUT_MS", "30000"),
        env("HYDRACACHE_STORAGE_DIR", "/var/lib/hydracache"),
        env("HYDRACACHE_TLS_ENABLED", &tls_enabled),
        env("HYDRACACHE_TLS_ACK_INSECURE", &tls_ack_insecure),
        env("HYDRACACHE_TLS_CERT_PATH", "/etc/hydracache/tls/tls.crt"),
        env("HYDRACACHE_TLS_KEY_PATH", "/etc/hydracache/tls/tls.key"),
        env("HYDRACACHE_TLS_CA_PATH", "/etc/hydracache/tls/ca.crt"),
        env(
            "HYDRACACHE_BACKUP_ENABLED",
            &cluster.spec.backup_schedule.is_some().to_string(),
        ),
        env("HYDRACACHE_BACKUP_LOCATION", backup_location),
        env("HYDRACACHE_ADMIN_ADDR", "0.0.0.0:9091"),
    ]
}

pub fn seed_list(name: &str, replicas: u32) -> String {
    (0..replicas)
        .map(|ordinal| {
            format!(
                "{name}-{ordinal}.{}:{}",
                headless_service_name(name),
                CLUSTER_PORT
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn data_volume_claim(
    cluster: &HydraCacheCluster,
    owner: Option<OwnerReference>,
) -> PersistentVolumeClaim {
    let persistence = cluster
        .spec
        .persistence
        .as_ref()
        .expect("data_volume_claim is only built when persistence exists");
    let mut requests = BTreeMap::new();
    requests.insert("storage".to_owned(), Quantity(persistence.size.clone()));

    PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(DATA_VOLUME.to_owned()),
            labels: Some(pvc_selector_labels(&cluster.name_any())),
            owner_references: owner.map(|owner| vec![owner]),
            ..Default::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            access_modes: Some(vec!["ReadWriteOnce".to_owned()]),
            resources: Some(VolumeResourceRequirements {
                requests: Some(requests),
                ..Default::default()
            }),
            storage_class_name: Some(persistence.storage_class_name.clone()),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn pvc_retention_policy(
    cluster: &HydraCacheCluster,
) -> StatefulSetPersistentVolumeClaimRetentionPolicy {
    let policy = match cluster
        .spec
        .persistence
        .as_ref()
        .map(|persistence| persistence.reclaim_policy)
        .unwrap_or_default()
    {
        PvcReclaimPolicy::Retain => "Retain",
        PvcReclaimPolicy::Delete => "Delete",
    };
    StatefulSetPersistentVolumeClaimRetentionPolicy {
        when_deleted: Some(policy.to_owned()),
        when_scaled: Some("Retain".to_owned()),
    }
}

fn object_meta(
    name: String,
    namespace: Option<String>,
    labels: &BTreeMap<String, String>,
    owner: Option<OwnerReference>,
) -> ObjectMeta {
    ObjectMeta {
        name: Some(name),
        namespace,
        labels: Some(labels.clone()),
        owner_references: owner.map(|owner| vec![owner]),
        ..Default::default()
    }
}

fn container_port(name: &str, port: i32) -> ContainerPort {
    ContainerPort {
        container_port: port,
        name: Some(name.to_owned()),
        protocol: Some("TCP".to_owned()),
        ..Default::default()
    }
}

fn service_port(name: &str, port: i32) -> ServicePort {
    ServicePort {
        name: Some(name.to_owned()),
        port,
        protocol: Some("TCP".to_owned()),
        target_port: Some(IntOrString::String(name.to_owned())),
        ..Default::default()
    }
}

fn http_probe(path: &str, period_seconds: i32, failure_threshold: i32) -> Probe {
    Probe {
        failure_threshold: Some(failure_threshold),
        http_get: Some(HTTPGetAction {
            path: Some(path.to_owned()),
            port: IntOrString::String("admin".to_owned()),
            ..Default::default()
        }),
        period_seconds: Some(period_seconds),
        timeout_seconds: Some(2),
        ..Default::default()
    }
}

fn env(name: &str, value: &str) -> EnvVar {
    EnvVar {
        name: name.to_owned(),
        value: Some(value.to_owned()),
        ..Default::default()
    }
}
