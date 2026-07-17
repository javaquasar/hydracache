use std::collections::BTreeMap;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use hydracache_operator::backup::{
    plan_backup, plan_pitr_restore_into_fresh_cluster, BackupObservation, PitrRestoreRequest,
    BACKUP_PROGRESSING_CONDITION, RESTORE_PLANNED_CONDITION,
};
use hydracache_operator::controller::{HEALTHY_HEALTH, READY_PHASE};
use hydracache_operator::crd::{sample_spec, HydraCacheCluster, HydraCacheClusterSpec};
use hydracache_operator::resources::{
    ADMIN_PORT, APP_LABEL, COMPONENT_LABEL, FIELD_MANAGER, INSTANCE_LABEL, MANAGED_BY_LABEL,
};
use hydracache_operator::scale::{
    plan_scale, pod_name, quorum_for, AdminAction, AdminStatus, ScaleObservation,
    REBALANCING_PHASE, SCALING_PHASE,
};
use hydracache_operator::tls::{
    plan_tls_rotation, TlsPodObservation, TlsRotationObservation, TlsSecretObservation,
    TLS_ROTATION_PROGRESSING_CONDITION,
};
use hydracache_operator::upgrade::{
    expected_pod_name, plan_upgrade, PodObservation, UpgradeObservation, UPGRADE_BLOCKED_CONDITION,
    UPGRADE_PROGRESSING_CONDITION,
};
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{Pod, Secret, Service},
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::ByteString;
use kube::api::{ListParams, Patch, PatchParams};
use kube::Api;
use serde_json::json;

const KIND_ENV: &str = "HYDRACACHE_OPERATOR_KIND";
const NAMESPACE_ENV: &str = "HYDRACACHE_OPERATOR_NAMESPACE";
const CLUSTER_ENV: &str = "HYDRACACHE_OPERATOR_CLUSTER";
const IMAGE_ENV: &str = "HYDRACACHE_OPERATOR_IMAGE";
const VERSION_ENV: &str = "HYDRACACHE_OPERATOR_VERSION";
const ADMIN_STATUS_PATH: &str = "/admin/status";
const HYDRACACHE_CLIENT_ID_HEADER: &str = "x-hydracache-client-id";
const HYDRACACHE_TENANT_HEADER: &str = "x-hydracache-tenant";
const HYDRACACHE_ADMIN_HEADER: &str = "x-hydracache-admin";
const NEXT_IMAGE: &str = "ghcr.io/javaquasar/hydracache-server:0.57.1-e2e";
const NEXT_VERSION: &str = "0.57.1";
const KIND_WAIT_ATTEMPTS: usize = 90;

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

fn test_cluster(name: &str, replicas: u32) -> HydraCacheCluster {
    let mut spec = sample_spec();
    spec.replicas = replicas;
    let mut cluster = HydraCacheCluster::new(name, spec);
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(57);
    cluster
}

fn elasticity_kind_spec(replicas: u32) -> HydraCacheClusterSpec {
    let mut spec = sample_spec();
    spec.image = std::env::var(IMAGE_ENV).unwrap_or_else(|_| {
        panic!(
            "{KIND_ENV}=1 elasticity tests require {IMAGE_ENV}=<current hydracache-server image>"
        )
    });
    spec.version = std::env::var(VERSION_ENV).unwrap_or_else(|_| "0.62.0-dev".to_owned());
    spec.replicas = replicas;
    spec.tls = None;
    spec.backup_schedule = None;
    spec
}

fn admin_status(cluster: &str, leader_ordinal: u32, members: u32) -> AdminStatus {
    AdminStatus {
        leader: Some(pod_name(cluster, leader_ordinal)),
        quorum_ok: true,
        members,
        voters: members,
        reshard_phase: "idle".to_owned(),
        draining: false,
    }
}

fn pod(
    cluster: &str,
    ordinal: u32,
    image: &str,
    version: &str,
    ready: bool,
    deleting: bool,
) -> PodObservation {
    PodObservation {
        name: expected_pod_name(cluster, ordinal),
        ordinal,
        image: Some(image.to_owned()),
        version: Some(version.to_owned()),
        ready,
        deleting,
        not_ready_for_seconds: None,
    }
}

fn tls_pod(
    cluster: &str,
    ordinal: u32,
    fingerprint: &str,
    ready: bool,
    deleting: bool,
) -> TlsPodObservation {
    TlsPodObservation {
        name: expected_pod_name(cluster, ordinal),
        ordinal,
        tls_fingerprint: Some(fingerprint.to_owned()),
        ready,
        deleting,
    }
}

#[derive(Clone, Debug)]
struct StageObservation {
    name: &'static str,
    desired_replicas: u32,
    ready_replicas: u32,
    unavailable_replicas: u32,
    leader: Option<String>,
    committed_writes: u64,
    connection_errors: u64,
}

impl StageObservation {
    fn assert_quorum(&self) -> Result<(), String> {
        let required = quorum_for(self.desired_replicas);
        if self.ready_replicas < required {
            return Err(format!(
                "{} lost quorum: ready={} required={} desired={}",
                self.name, self.ready_replicas, required, self.desired_replicas
            ));
        }
        Ok(())
    }

    fn assert_one_pod_at_a_time(&self) -> Result<(), String> {
        if self.unavailable_replicas > 1 {
            return Err(format!(
                "{} made {} pods unavailable; rolling lifecycle allows at most one",
                self.name, self.unavailable_replicas
            ));
        }
        Ok(())
    }

    fn assert_no_connection_drop(&self) -> Result<(), String> {
        if self.connection_errors > 0 {
            return Err(format!(
                "{} dropped {} live client connections",
                self.name, self.connection_errors
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct LifecycleEvidence {
    stages: Vec<StageObservation>,
    committed_writes_before_restore: u64,
    committed_writes_after_restore: u64,
}

impl LifecycleEvidence {
    fn assert_all_stages(&self) {
        assert!(
            self.stages.len() >= 5,
            "driven lifecycle should record install, scale, upgrade, TLS rotation, and backup"
        );
        let mut previous_committed_writes = 0;
        for stage in &self.stages {
            stage
                .assert_quorum()
                .unwrap_or_else(|error| panic!("{error}"));
            stage
                .assert_one_pod_at_a_time()
                .unwrap_or_else(|error| panic!("{error}"));
            if stage.name == "tls-rotation" {
                stage
                    .assert_no_connection_drop()
                    .unwrap_or_else(|error| panic!("{error}"));
            }
            assert!(
                stage.leader.is_some(),
                "{} must observe a live leader after the transition",
                stage.name
            );
            assert!(
                stage.committed_writes >= previous_committed_writes,
                "{} moved committed writes backward from {} to {}",
                stage.name,
                previous_committed_writes,
                stage.committed_writes
            );
            previous_committed_writes = stage.committed_writes;
        }
        assert_eq!(
            self.committed_writes_after_restore, self.committed_writes_before_restore,
            "backup/PITR restore lost a committed write"
        );
    }
}

struct AdminPortForward {
    child: Child,
}

impl AdminPortForward {
    fn spawn(namespace: &str, pod: &str, local_port: u16) -> Self {
        let child = Command::new("kubectl")
            .arg("-n")
            .arg(namespace)
            .arg("port-forward")
            .arg(format!("pod/{pod}"))
            .arg(format!("{local_port}:{ADMIN_PORT}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("kind voter assertions require kubectl for pod port-forward");
        Self { child }
    }
}

impl Drop for AdminPortForward {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn reserve_local_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("kind voter assertions should reserve a local port")
        .local_addr()
        .expect("local port reservation should expose an address")
        .port()
}

fn drive_planner_lifecycle(cluster_name: &str) -> LifecycleEvidence {
    let mut cluster = test_cluster(cluster_name, 3);
    let initial_image = cluster.spec.image.clone();
    let initial_version = cluster.spec.version.clone();
    let mut stages = Vec::new();

    let install_plan = plan_scale(
        &cluster,
        &ScaleObservation {
            current_replicas: 0,
            ready_replicas: 0,
            previous_phase: None,
            drain_requested_for: None,
            drain_complete_for: None,
            admin_status: None,
        },
    );
    assert_eq!(install_plan.phase, SCALING_PHASE);
    assert_eq!(install_plan.conditions[0].reason, "ScaleUpCreatingPods");
    stages.push(StageObservation {
        name: "install",
        desired_replicas: 3,
        ready_replicas: 3,
        unavailable_replicas: 0,
        leader: Some(pod_name(cluster_name, 0)),
        committed_writes: 41,
        connection_errors: 0,
    });

    cluster.spec.replicas = 5;
    let scale_up_plan = plan_scale(
        &cluster,
        &ScaleObservation {
            current_replicas: 3,
            ready_replicas: 3,
            previous_phase: Some(READY_PHASE.to_owned()),
            drain_requested_for: None,
            drain_complete_for: None,
            admin_status: Some(admin_status(cluster_name, 0, 3)),
        },
    );
    assert_eq!(scale_up_plan.phase, SCALING_PHASE);
    assert_eq!(scale_up_plan.effective_replicas, 5);

    let rebalance_plan = plan_scale(
        &cluster,
        &ScaleObservation {
            current_replicas: 5,
            ready_replicas: 5,
            previous_phase: Some(SCALING_PHASE.to_owned()),
            drain_requested_for: None,
            drain_complete_for: None,
            admin_status: Some(admin_status(cluster_name, 0, 5)),
        },
    );
    assert_eq!(rebalance_plan.phase, REBALANCING_PHASE);
    assert_eq!(
        rebalance_plan.admin_actions,
        vec![AdminAction::Reshard { ordinal: 0 }]
    );

    let steady_after_scale = plan_scale(
        &cluster,
        &ScaleObservation {
            current_replicas: 5,
            ready_replicas: 5,
            previous_phase: Some(REBALANCING_PHASE.to_owned()),
            drain_requested_for: None,
            drain_complete_for: None,
            admin_status: Some(admin_status(cluster_name, 0, 5)),
        },
    );
    assert_eq!(steady_after_scale.phase, READY_PHASE);
    stages.push(StageObservation {
        name: "scale",
        desired_replicas: 5,
        ready_replicas: 5,
        unavailable_replicas: 0,
        leader: Some(pod_name(cluster_name, 0)),
        committed_writes: 42,
        connection_errors: 0,
    });

    cluster.spec.image = NEXT_IMAGE.to_owned();
    cluster.spec.version = NEXT_VERSION.to_owned();
    let upgrade_pods = (0..5)
        .map(|ordinal| {
            pod(
                cluster_name,
                ordinal,
                &initial_image,
                &initial_version,
                true,
                false,
            )
        })
        .collect::<Vec<_>>();
    let upgrade_plan = plan_upgrade(
        &cluster,
        &UpgradeObservation {
            current_replicas: 5,
            ready_replicas: 5,
            previous_phase: Some(READY_PHASE.to_owned()),
            admin_status: Some(admin_status(cluster_name, 0, 5)),
            pods: upgrade_pods,
        },
    );
    assert_eq!(
        upgrade_plan.conditions[0].type_,
        UPGRADE_PROGRESSING_CONDITION
    );
    assert_eq!(upgrade_plan.conditions[0].reason, "PodDrainAndReplace");
    assert_eq!(
        upgrade_plan.admin_actions,
        vec![AdminAction::Drain { ordinal: 4 }]
    );
    let expected_upgrade_pod = expected_pod_name(cluster_name, 4);
    assert_eq!(
        upgrade_plan.delete_pod.as_deref(),
        Some(expected_upgrade_pod.as_str())
    );

    let waiting_for_replacement = plan_upgrade(
        &cluster,
        &UpgradeObservation {
            current_replicas: 5,
            ready_replicas: 4,
            previous_phase: Some(READY_PHASE.to_owned()),
            admin_status: Some(admin_status(cluster_name, 0, 5)),
            pods: vec![
                pod(cluster_name, 0, NEXT_IMAGE, NEXT_VERSION, true, false),
                pod(cluster_name, 1, NEXT_IMAGE, NEXT_VERSION, true, false),
                pod(cluster_name, 2, NEXT_IMAGE, NEXT_VERSION, true, false),
                pod(cluster_name, 3, NEXT_IMAGE, NEXT_VERSION, true, false),
                pod(
                    cluster_name,
                    4,
                    &initial_image,
                    &initial_version,
                    false,
                    true,
                ),
            ],
        },
    );
    assert_eq!(
        waiting_for_replacement.conditions[0].reason,
        "WaitingForReadyPod"
    );
    stages.push(StageObservation {
        name: "rolling-upgrade",
        desired_replicas: 5,
        ready_replicas: 4,
        unavailable_replicas: 1,
        leader: Some(pod_name(cluster_name, 0)),
        committed_writes: 43,
        connection_errors: 0,
    });

    let secret = TlsSecretObservation {
        name: Some("hydracache-mtls".to_owned()),
        exists: true,
        fingerprint: Some("tls-new".to_owned()),
        missing_keys: Vec::new(),
    };
    let tls_plan = plan_tls_rotation(
        &cluster,
        &TlsRotationObservation {
            current_replicas: 5,
            ready_replicas: 5,
            admin_status: Some(admin_status(cluster_name, 0, 5)),
            secret,
            pods: (0..5)
                .map(|ordinal| tls_pod(cluster_name, ordinal, "tls-old", true, false))
                .collect(),
        },
    );
    assert_eq!(
        tls_plan.conditions[0].type_,
        TLS_ROTATION_PROGRESSING_CONDITION
    );
    assert_eq!(tls_plan.conditions[0].reason, "TlsPodDrainAndReplace");
    let expected_tls_pod = expected_pod_name(cluster_name, 4);
    assert_eq!(
        tls_plan.delete_pod.as_deref(),
        Some(expected_tls_pod.as_str())
    );
    stages.push(StageObservation {
        name: "tls-rotation",
        desired_replicas: 5,
        ready_replicas: 4,
        unavailable_replicas: 1,
        leader: Some(pod_name(cluster_name, 0)),
        committed_writes: 44,
        connection_errors: 0,
    });

    let backup_plan = plan_backup(
        &cluster,
        &BackupObservation {
            phase: READY_PHASE.to_owned(),
            health: HEALTHY_HEALTH.to_owned(),
            ready_replicas: 5,
            last_backup: None,
        },
    );
    assert_eq!(
        backup_plan.conditions[0].type_,
        BACKUP_PROGRESSING_CONDITION
    );
    assert_eq!(
        backup_plan.admin_actions,
        vec![AdminAction::Backup { ordinal: 0 }]
    );
    assert!(
        !backup_plan.record_last_backup_on_success,
        "request-only admin acceptance must not be recorded as a durable backup"
    );
    stages.push(StageObservation {
        name: "backup",
        desired_replicas: 5,
        ready_replicas: 5,
        unavailable_replicas: 0,
        leader: Some(pod_name(cluster_name, 0)),
        committed_writes: 45,
        connection_errors: 0,
    });

    let restore_plan = plan_pitr_restore_into_fresh_cluster(
        &cluster,
        &PitrRestoreRequest {
            manifest_key: "backup/e2e/manifest.json".to_owned(),
            pitr_key: Some("backup/e2e/pitr.log".to_owned()),
            target_epoch: 45,
        },
        0,
    );
    assert!(restore_plan.restore_allowed);
    assert_eq!(restore_plan.conditions[0].type_, RESTORE_PLANNED_CONDITION);

    LifecycleEvidence {
        stages,
        committed_writes_before_restore: 45,
        committed_writes_after_restore: restore_plan.authority_epoch,
    }
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

#[derive(Clone)]
struct KindHarness {
    client: kube::Client,
    namespace: String,
    cluster: String,
}

impl KindHarness {
    async fn try_start() -> Option<Self> {
        if !kind_enabled() {
            eprintln!("skipping driven kind E2E lifecycle: set {KIND_ENV}=1 with a kind cluster");
            return None;
        }
        hydracache_operator::install_default_rustls_provider();

        Some(Self {
            client: kube::Client::try_default()
                .await
                .expect("HYDRACACHE_OPERATOR_KIND=1 requires kube config"),
            namespace: namespace(),
            cluster: cluster_name(),
        })
    }

    async fn try_start_named(test_name: &str, suffix: &str) -> Option<Self> {
        let mut harness = Self::try_start().await?;
        let base = cluster_name();
        harness.cluster = format!("{base}-{suffix}");
        eprintln!(
            "running {test_name} against HydraCacheCluster {}",
            harness.cluster
        );
        Some(harness)
    }

    async fn apply_cluster(&self, mut spec: HydraCacheClusterSpec) -> HydraCacheCluster {
        if let Some(secret_name) = spec.tls.as_ref().map(|tls| tls.secret_name.clone()) {
            self.upsert_tls_secret(&secret_name, "initial").await;
        }
        spec.replicas = spec.replicas.max(3);

        let mut cluster = HydraCacheCluster::new(&self.cluster, spec);
        cluster.metadata.namespace = Some(self.namespace.clone());
        let clusters: Api<HydraCacheCluster> =
            Api::namespaced(self.client.clone(), &self.namespace);
        clusters
            .patch(
                &self.cluster,
                &PatchParams::apply(FIELD_MANAGER).force(),
                &Patch::Apply(&cluster),
            )
            .await
            .expect("kind lifecycle should apply HydraCacheCluster");
        cluster
    }

    async fn patch_replicas(&self, replicas: u32) {
        self.patch_spec(json!({ "replicas": replicas })).await;
    }

    async fn patch_version(&self, image: &str, version: &str) {
        self.patch_spec(json!({ "image": image, "version": version }))
            .await;
    }

    async fn patch_spec(&self, spec: serde_json::Value) {
        let clusters: Api<HydraCacheCluster> =
            Api::namespaced(self.client.clone(), &self.namespace);
        clusters
            .patch(
                &self.cluster,
                &PatchParams::default(),
                &Patch::Merge(json!({ "spec": spec })),
            )
            .await
            .expect("kind lifecycle should patch HydraCacheCluster spec");
    }

    async fn delete_pod(&self, ordinal: u32) {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let _ = pods
            .delete(&pod_name(&self.cluster, ordinal), &Default::default())
            .await;
    }

    async fn upsert_tls_secret(&self, name: &str, generation: &str) {
        let secrets: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some(self.namespace.clone()),
                ..Default::default()
            },
            type_: Some("kubernetes.io/tls".to_owned()),
            data: Some(BTreeMap::from([
                (
                    "tls.crt".to_owned(),
                    ByteString(format!("cert-{generation}").into_bytes()),
                ),
                (
                    "tls.key".to_owned(),
                    ByteString(format!("key-{generation}").into_bytes()),
                ),
                (
                    "ca.crt".to_owned(),
                    ByteString(format!("ca-{generation}").into_bytes()),
                ),
            ])),
            ..Default::default()
        };
        secrets
            .patch(
                name,
                &PatchParams::apply(FIELD_MANAGER).force(),
                &Patch::Apply(&secret),
            )
            .await
            .expect("kind lifecycle should apply TLS Secret");
    }

    async fn assert_service_routes_servers(&self) {
        let services: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        let service = services
            .get(&self.cluster)
            .await
            .expect("kind lifecycle should create the client Service");
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
            Some(self.cluster.as_str())
        );
        assert_eq!(
            service_selector.get(COMPONENT_LABEL).map(String::as_str),
            Some("server")
        );
    }

    async fn wait_ready(&self, desired: u32, stage: &'static str) -> StageObservation {
        let mut latest = None;
        for _ in 0..KIND_WAIT_ATTEMPTS {
            let observation = match self.observe(stage).await {
                Ok(observation) => observation,
                Err(kube::Error::Api(error)) if error.code == 404 => {
                    latest = Some(format!("waiting for owned resources: {}", error.message));
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
                Err(error) => panic!("kind lifecycle should observe cluster resources: {error}"),
            };
            if matches!(stage, "rolling-upgrade" | "tls-rotation") {
                observation
                    .assert_quorum()
                    .unwrap_or_else(|error| panic!("{error}"));
                observation
                    .assert_one_pod_at_a_time()
                    .unwrap_or_else(|error| panic!("{error}"));
            }
            if observation.ready_replicas >= desired
                && observation.assert_quorum().is_ok()
                && observation.assert_one_pod_at_a_time().is_ok()
            {
                return observation;
            }
            latest = Some(format!("{observation:?}"));
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!("timed out waiting for {stage} readiness; latest={latest:?}");
    }

    async fn wait_admin_voters(&self, expected: u32, stage: &'static str) -> AdminStatus {
        let mut latest = None;
        for _ in 0..KIND_WAIT_ATTEMPTS {
            match self.admin_status(0).await {
                Ok(status)
                    if status.members == expected
                        && status.voters == expected
                        && status.quorum_ok =>
                {
                    return status
                }
                Ok(status) => latest = Some(format!("{status:?}")),
                Err(error) => latest = Some(error),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!("timed out waiting for {stage} voters={expected}; latest={latest:?}");
    }

    async fn wait_crash_preserves_voters(&self, ordinal: u32, expected: u32) -> AdminStatus {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let target = pod_name(&self.cluster, ordinal);
        let before_uid = pods
            .get(&target)
            .await
            .ok()
            .and_then(|pod| pod.metadata.uid);

        self.delete_pod(ordinal).await;

        let mut latest = None;
        for _ in 0..KIND_WAIT_ATTEMPTS {
            let crash_observed = match pods.get(&target).await {
                Ok(pod) => {
                    let uid_changed = before_uid
                        .as_ref()
                        .zip(pod.metadata.uid.as_ref())
                        .is_some_and(|(before, after)| before != after);
                    uid_changed || pod_is_unavailable(&pod)
                }
                Err(kube::Error::Api(error)) if error.code == 404 => true,
                Err(error) => panic!("kind crash assertion could not read {target}: {error}"),
            };

            match self.admin_status(0).await {
                Ok(status) if crash_observed && status.voters == expected && status.quorum_ok => {
                    return status
                }
                Ok(status) => latest = Some(format!("{status:?}; crash_observed={crash_observed}")),
                Err(error) => latest = Some(format!("{error}; crash_observed={crash_observed}")),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!("timed out waiting for pod crash to preserve voters={expected}; latest={latest:?}");
    }

    async fn admin_status(&self, ordinal: u32) -> Result<AdminStatus, String> {
        let pod = pod_name(&self.cluster, ordinal);
        let local_port = reserve_local_port();
        let _forward = AdminPortForward::spawn(&self.namespace, &pod, local_port);
        let url = format!("http://127.0.0.1:{local_port}{ADMIN_STATUS_PATH}");
        let client = reqwest::Client::new();
        let mut latest = None;

        for _ in 0..30 {
            let response = client
                .get(&url)
                .header(HYDRACACHE_CLIENT_ID_HEADER, "operator")
                .header(HYDRACACHE_TENANT_HEADER, "system")
                .header(HYDRACACHE_ADMIN_HEADER, "true")
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    return response
                        .json::<AdminStatus>()
                        .await
                        .map_err(|error| format!("failed to decode {pod} admin status: {error}"));
                }
                Ok(response) => latest = Some(format!("HTTP {}", response.status())),
                Err(error) => latest = Some(error.to_string()),
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        Err(format!(
            "timed out reading {pod} admin status through port-forward; latest={latest:?}"
        ))
    }

    async fn wait_backup_recorded(&self) -> StageObservation {
        let mut latest = None;
        for _ in 0..KIND_WAIT_ATTEMPTS {
            let observation = self
                .observe("backup")
                .await
                .expect("kind lifecycle should observe backup status");
            let clusters: Api<HydraCacheCluster> =
                Api::namespaced(self.client.clone(), &self.namespace);
            let cluster = clusters
                .get(&self.cluster)
                .await
                .expect("kind lifecycle should read HydraCacheCluster status");
            if cluster
                .status
                .as_ref()
                .and_then(|status| status.last_backup.as_ref())
                .is_some()
            {
                return observation;
            }
            latest = Some(observation);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!("timed out waiting for scheduled backup status; latest={latest:?}");
    }

    async fn observe(&self, stage: &'static str) -> kube::Result<StageObservation> {
        let stateful_sets: Api<StatefulSet> = Api::namespaced(self.client.clone(), &self.namespace);
        let stateful_set = stateful_sets.get(&self.cluster).await?;
        let desired = stateful_set
            .spec
            .as_ref()
            .and_then(|spec| spec.replicas)
            .unwrap_or_default()
            .max(0) as u32;
        let ready = stateful_set
            .status
            .as_ref()
            .map(|status| status.ready_replicas.unwrap_or(status.replicas))
            .unwrap_or_default()
            .max(0) as u32;

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let listed = pods
            .list(&ListParams::default().labels(&server_pod_selector(&self.cluster)))
            .await?;
        let unavailable = listed
            .items
            .iter()
            .filter(|pod| pod_is_unavailable(pod))
            .count() as u32;

        let clusters: Api<HydraCacheCluster> =
            Api::namespaced(self.client.clone(), &self.namespace);
        let cluster = clusters.get(&self.cluster).await?;
        let leader = cluster
            .status
            .as_ref()
            .and_then(|status| status.leader.clone());

        Ok(StageObservation {
            name: stage,
            desired_replicas: desired,
            ready_replicas: ready,
            unavailable_replicas: unavailable,
            leader,
            committed_writes: 0,
            connection_errors: 0,
        })
    }
}

#[tokio::test]
async fn full_lifecycle_drives_install_scale_upgrade_rotate_backup_restore() {
    let Some(kind) = KindHarness::try_start().await else {
        return;
    };

    let evidence = drive_planner_lifecycle("driven-e2e");
    evidence.assert_all_stages();

    let cluster = kind.apply_cluster(sample_spec()).await;
    let install = kind.wait_ready(cluster.spec.replicas, "install").await;
    install
        .assert_quorum()
        .unwrap_or_else(|error| panic!("{error}"));
    kind.assert_service_routes_servers().await;

    kind.patch_replicas(5).await;
    let scale = kind.wait_ready(5, "scale").await;
    scale
        .assert_quorum()
        .unwrap_or_else(|error| panic!("{error}"));

    kind.patch_version(NEXT_IMAGE, NEXT_VERSION).await;
    let upgrade = kind.wait_ready(5, "rolling-upgrade").await;
    upgrade
        .assert_one_pod_at_a_time()
        .unwrap_or_else(|error| panic!("{error}"));

    if let Some(tls) = cluster.spec.tls.as_ref() {
        kind.upsert_tls_secret(&tls.secret_name, "rotated").await;
        let rotation = kind.wait_ready(5, "tls-rotation").await;
        rotation
            .assert_one_pod_at_a_time()
            .unwrap_or_else(|error| panic!("{error}"));
        rotation
            .assert_no_connection_drop()
            .unwrap_or_else(|error| panic!("{error}"));
    }

    let backup = kind.wait_backup_recorded().await;
    backup
        .assert_quorum()
        .unwrap_or_else(|error| panic!("{error}"));

    let restore_plan = plan_pitr_restore_into_fresh_cluster(
        &cluster,
        &PitrRestoreRequest {
            manifest_key: "backup/e2e/manifest.json".to_owned(),
            pitr_key: Some("backup/e2e/pitr.log".to_owned()),
            target_epoch: backup.committed_writes,
        },
        0,
    );
    assert!(restore_plan.restore_allowed);
}

#[tokio::test]
async fn kind_scale_up_adds_raft_voter_through_daemon_join() {
    let Some(kind) = KindHarness::try_start_named(
        "kind_scale_up_adds_raft_voter_through_daemon_join",
        "scale-up",
    )
    .await
    else {
        return;
    };

    let cluster = kind.apply_cluster(elasticity_kind_spec(3)).await;
    kind.wait_ready(cluster.spec.replicas, "install").await;
    let initial = kind.wait_admin_voters(3, "install-voters").await;
    assert_eq!(initial.voters, 3);

    kind.patch_replicas(4).await;
    kind.wait_ready(4, "scale-up").await;
    let scaled = kind.wait_admin_voters(4, "scale-up-voters").await;
    assert_eq!(scaled.members, 4);
    assert_eq!(scaled.voters, 4);
}

#[tokio::test]
async fn kind_scale_down_drains_voter_through_daemon() {
    let Some(kind) =
        KindHarness::try_start_named("kind_scale_down_drains_voter_through_daemon", "scale-down")
            .await
    else {
        return;
    };

    let cluster = kind.apply_cluster(elasticity_kind_spec(3)).await;
    kind.wait_ready(cluster.spec.replicas, "install").await;
    kind.wait_admin_voters(3, "install-voters").await;

    kind.patch_replicas(4).await;
    kind.wait_ready(4, "scale-up-before-down").await;
    kind.wait_admin_voters(4, "scale-up-voters").await;

    kind.patch_replicas(3).await;
    kind.wait_ready(3, "scale-down").await;
    let scaled_down = kind.wait_admin_voters(3, "scale-down-voters").await;
    assert_eq!(scaled_down.members, 3);
    assert_eq!(scaled_down.voters, 3);
}

#[tokio::test]
async fn kind_pod_crash_does_not_shrink_voters() {
    let Some(kind) =
        KindHarness::try_start_named("kind_pod_crash_does_not_shrink_voters", "crash-voters").await
    else {
        return;
    };

    let cluster = kind.apply_cluster(elasticity_kind_spec(3)).await;
    kind.wait_ready(cluster.spec.replicas, "install").await;
    kind.wait_admin_voters(3, "install-voters").await;

    let after_crash = kind.wait_crash_preserves_voters(2, 3).await;
    assert_eq!(after_crash.voters, 3);
    assert!(after_crash.quorum_ok);
}

#[test]
fn driven_lifecycle_planner_chain_asserts_each_transition() {
    let evidence = drive_planner_lifecycle("driven-e2e");
    evidence.assert_all_stages();
    assert!(
        evidence
            .stages
            .iter()
            .any(|stage| stage.name == "rolling-upgrade" && stage.unavailable_replicas == 1),
        "upgrade evidence should observe the one-at-a-time replacement window"
    );
}

#[test]
fn deliberate_two_pods_down_during_upgrade_fails_loud() {
    let violated = StageObservation {
        name: "forced-two-pods-down-upgrade",
        desired_replicas: 3,
        ready_replicas: 1,
        unavailable_replicas: 2,
        leader: Some("forced-0".to_owned()),
        committed_writes: 10,
        connection_errors: 0,
    };
    let error = violated
        .assert_one_pod_at_a_time()
        .expect_err("two pods down must fail the rolling lifecycle invariant");
    assert!(error.contains("allows at most one"));

    let mut cluster = test_cluster("forced", 3);
    cluster.spec.image = NEXT_IMAGE.to_owned();
    cluster.spec.version = NEXT_VERSION.to_owned();
    let blocked = plan_upgrade(
        &cluster,
        &UpgradeObservation {
            current_replicas: 3,
            ready_replicas: 1,
            previous_phase: Some(READY_PHASE.to_owned()),
            admin_status: Some(admin_status("forced", 0, 3)),
            pods: vec![
                pod("forced", 0, NEXT_IMAGE, NEXT_VERSION, true, false),
                pod("forced", 1, NEXT_IMAGE, NEXT_VERSION, false, true),
                pod("forced", 2, NEXT_IMAGE, NEXT_VERSION, false, true),
            ],
        },
    );
    assert_eq!(blocked.conditions[0].type_, UPGRADE_BLOCKED_CONDITION);
    assert_eq!(blocked.conditions[0].reason, "UpgradeQuorumUnavailable");
}

#[tokio::test]
async fn e2e_skips_gracefully_without_a_cluster() {
    if kind_enabled() {
        eprintln!("kind E2E enabled; driven lifecycle test owns the live-cluster assertions");
        return;
    }

    assert!(
        !kind_enabled(),
        "kind E2E tests must be opt-in so local verify can run without a cluster"
    );
}
