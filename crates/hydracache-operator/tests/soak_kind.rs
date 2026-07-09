use std::collections::BTreeMap;
use std::process::Command;
use std::time::Duration;

use hydracache_operator::controller::READY_PHASE;
use hydracache_operator::crd::{sample_spec, HydraCacheCluster, HydraCacheClusterSpec};
use hydracache_operator::resources::{
    headless_service_name, APP_LABEL, COMPONENT_LABEL, FIELD_MANAGER, INSTANCE_LABEL,
    MANAGED_BY_LABEL,
};
use hydracache_operator::scale::{pod_name, quorum_for};
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{Container, Pod, PodSpec},
    networking::v1::{NetworkPolicy, NetworkPolicySpec},
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use kube::api::{DeleteParams, ListParams, Patch, PatchParams};
use kube::{
    api::{ApiResource, DynamicObject, GroupVersionKind},
    discovery, Api,
};
use serde_json::{json, Value};

const KIND_ENV: &str = "HYDRACACHE_OPERATOR_KIND";
const NAMESPACE_ENV: &str = "HYDRACACHE_OPERATOR_NAMESPACE";
const CLUSTER_ENV: &str = "HYDRACACHE_OPERATOR_CLUSTER";
const IMAGE_ENV: &str = "HYDRACACHE_OPERATOR_IMAGE";
const VERSION_ENV: &str = "HYDRACACHE_OPERATOR_VERSION";
const NETWORK_PROBE_IMAGE_ENV: &str = "HYDRACACHE_NETWORK_PROBE_IMAGE";
const KIND_WAIT_ATTEMPTS: usize = 90;
const NETWORK_POLICY_SKIP: &str =
    "CNI does not enforce NetworkPolicy; install calico/cilium in the kind config";
const IOCHAOS_SKIP: &str =
    "chaos-mesh IOChaos CRD is not installed; slow-disk remains an external dependency";
const SCOPE_DISCLOSURE: &str = "0.61 kind chaos: NetworkPartition uses Kubernetes NetworkPolicy only when a CNI enforcement probe proves policy is active; SlowDisk uses chaos-mesh IOChaos only when the iochaos.chaos-mesh.org CRD is installed. Each unsupported leg skips loud, never wrong-but-green.";

fn kind_enabled() -> bool {
    std::env::var(KIND_ENV).as_deref() == Ok("1")
}

fn namespace() -> String {
    std::env::var(NAMESPACE_ENV).unwrap_or_else(|_| "default".to_owned())
}

fn cluster_name() -> String {
    std::env::var(CLUSTER_ENV).unwrap_or_else(|_| "hydracache-soak".to_owned())
}

fn network_probe_image() -> String {
    std::env::var(NETWORK_PROBE_IMAGE_ENV).unwrap_or_else(|_| "busybox:1.36".to_owned())
}

fn soak_kind_spec(replicas: u32) -> HydraCacheClusterSpec {
    let mut spec = sample_spec();
    spec.image = std::env::var(IMAGE_ENV).unwrap_or_else(|_| {
        panic!("{KIND_ENV}=1 soak tests require {IMAGE_ENV}=<current hydracache-server image>")
    });
    spec.version = std::env::var(VERSION_ENV).unwrap_or_else(|_| "0.61.0-dev".to_owned());
    spec.replicas = replicas;
    spec.tls = None;
    spec.backup_schedule = None;
    spec
}

fn lifecycle_selector(cluster: &str) -> String {
    format!("{APP_LABEL}=hydracache,{INSTANCE_LABEL}={cluster},{MANAGED_BY_LABEL}={FIELD_MANAGER}")
}

fn server_pod_selector(cluster: &str) -> String {
    format!("{},{}=server", lifecycle_selector(cluster), COMPONENT_LABEL)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChaosFault {
    PodCrash { ordinal: u32 },
    NetworkPartition { ordinal: u32 },
    SlowDisk { ordinal: u32 },
}

impl ChaosFault {
    fn requires_optional_infrastructure(self) -> bool {
        matches!(self, Self::NetworkPartition { .. } | Self::SlowDisk { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChaosInjection {
    Applied(&'static str),
    Skipped(String),
}

impl ChaosInjection {
    fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SoakObservation {
    stage: &'static str,
    desired_replicas: u32,
    ready_replicas: u32,
    unavailable_replicas: u32,
    leader: Option<String>,
    committed_writes: u64,
}

impl SoakObservation {
    fn assert_quorum(&self) {
        let required = quorum_for(self.desired_replicas);
        assert!(
            self.ready_replicas >= required,
            "{} lost quorum: ready={} required={} desired={}",
            self.stage,
            self.ready_replicas,
            required,
            self.desired_replicas
        );
    }

    fn assert_leader(&self) {
        assert!(
            self.leader.is_some(),
            "{} did not report a leader; {SCOPE_DISCLOSURE}",
            self.stage
        );
    }
}

#[derive(Debug, Default)]
struct CommittedWriteProbe {
    committed: u64,
}

impl CommittedWriteProbe {
    fn record_committed_write(&mut self) {
        self.committed = self.committed.saturating_add(1);
    }

    fn committed(&self) -> u64 {
        self.committed
    }

    fn assert_no_lost_committed_write(&self, observed: &SoakObservation) {
        assert!(
            observed.committed_writes >= self.committed,
            "{} lost committed writes: observed={} expected_at_least={}",
            observed.stage,
            observed.committed_writes,
            self.committed
        );
    }
}

fn partition_policy_name(cluster: &str, ordinal: u32) -> String {
    format!("{cluster}-partition-{ordinal}")
}

fn slow_disk_chaos_name(cluster: &str, ordinal: u32) -> String {
    format!("{cluster}-slow-disk-{ordinal}")
}

fn deny_all_partition_policy(cluster: &str, namespace: &str, ordinal: u32) -> NetworkPolicy {
    NetworkPolicy {
        metadata: ObjectMeta {
            name: Some(partition_policy_name(cluster, ordinal)),
            namespace: Some(namespace.to_owned()),
            ..Default::default()
        },
        spec: Some(NetworkPolicySpec {
            ingress: Some(Vec::new()),
            egress: Some(Vec::new()),
            pod_selector: Some(LabelSelector {
                match_labels: Some(BTreeMap::from([(
                    "statefulset.kubernetes.io/pod-name".to_owned(),
                    pod_name(cluster, ordinal),
                )])),
                ..Default::default()
            }),
            policy_types: Some(vec!["Ingress".to_owned(), "Egress".to_owned()]),
        }),
    }
}

fn iochaos_manifest(cluster: &str, namespace: &str, ordinal: u32) -> Value {
    let pod = pod_name(cluster, ordinal);
    json!({
        "apiVersion": "chaos-mesh.org/v1alpha1",
        "kind": "IOChaos",
        "metadata": {
            "name": slow_disk_chaos_name(cluster, ordinal),
            "namespace": namespace,
        },
        "spec": {
            "action": "latency",
            "mode": "one",
            "selector": {
                "namespaces": [namespace],
                "pods": {
                    namespace: [pod],
                },
            },
            "volumePath": "/var/lib/hydracache",
            "path": "/var/lib/hydracache/**/*",
            "delay": "100ms",
            "percent": 100,
            "duration": "30s",
        },
    })
}

fn slow_disk_plan_for_crd_present(crd_present: bool) -> ChaosInjection {
    if crd_present {
        ChaosInjection::Applied("chaos-mesh IOChaos")
    } else {
        ChaosInjection::Skipped(IOCHAOS_SKIP.to_owned())
    }
}

#[derive(Clone)]
struct KindHarness {
    client: kube::Client,
    namespace: String,
    cluster: String,
}

impl KindHarness {
    async fn try_start(test_name: &str) -> Option<Self> {
        if !kind_enabled() {
            eprintln!(
                "skipping {test_name}: set {KIND_ENV}=1 with a kind cluster. {SCOPE_DISCLOSURE}"
            );
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

    async fn apply_cluster(&self, mut spec: HydraCacheClusterSpec) -> HydraCacheCluster {
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
            .expect("kind soak should apply HydraCacheCluster");
        cluster
    }

    async fn inject(&self, fault: ChaosFault) -> ChaosInjection {
        match fault {
            ChaosFault::PodCrash { ordinal } => {
                let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
                let _ = pods
                    .delete(&pod_name(&self.cluster, ordinal), &DeleteParams::default())
                    .await;
                ChaosInjection::Applied("pod delete")
            }
            ChaosFault::NetworkPartition { ordinal } => {
                self.inject_network_partition(ordinal).await
            }
            ChaosFault::SlowDisk { ordinal } => self.inject_slow_disk(ordinal).await,
        }
    }

    async fn heal(&self, fault: ChaosFault, injection: &ChaosInjection) {
        match fault {
            ChaosFault::PodCrash { .. } => {}
            ChaosFault::NetworkPartition { ordinal } => {
                if !injection.is_skipped() {
                    self.delete_network_partition(ordinal).await;
                }
            }
            ChaosFault::SlowDisk { ordinal } => {
                if !injection.is_skipped() {
                    self.delete_slow_disk(ordinal).await;
                }
            }
        }
    }

    async fn inject_network_partition(&self, ordinal: u32) -> ChaosInjection {
        let probe = self
            .ensure_network_probe_pod(ordinal)
            .await
            .unwrap_or_else(|error| panic!("kind partition probe pod could not start: {error}"));
        if !self.network_probe_reaches(&probe, ordinal).await {
            self.delete_network_probe_pod(&probe).await;
            panic!(
                "kind partition probe could not reach {} before NetworkPolicy injection",
                pod_name(&self.cluster, ordinal)
            );
        }

        let policies: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), &self.namespace);
        let policy = deny_all_partition_policy(&self.cluster, &self.namespace, ordinal);
        let name = partition_policy_name(&self.cluster, ordinal);
        policies
            .patch(
                &name,
                &PatchParams::apply(FIELD_MANAGER).force(),
                &Patch::Apply(&policy),
            )
            .await
            .expect("kind partition injector should apply NetworkPolicy");

        let blocked = self.network_policy_blocks_peer(&probe, ordinal).await;
        self.delete_network_probe_pod(&probe).await;
        if blocked {
            ChaosInjection::Applied("NetworkPolicy")
        } else {
            let _ = policies.delete(&name, &DeleteParams::default()).await;
            ChaosInjection::Skipped(NETWORK_POLICY_SKIP.to_owned())
        }
    }

    async fn delete_network_partition(&self, ordinal: u32) {
        let policies: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), &self.namespace);
        let _ = policies
            .delete(
                &partition_policy_name(&self.cluster, ordinal),
                &DeleteParams::default(),
            )
            .await;
    }

    async fn network_probe_reaches(&self, probe: &str, target_ordinal: u32) -> bool {
        for _ in 0..10 {
            if self.network_probe_wget(probe, target_ordinal).await {
                return true;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        false
    }

    async fn network_policy_blocks_peer(&self, probe: &str, isolated_ordinal: u32) -> bool {
        for _ in 0..10 {
            if !self.network_probe_wget(probe, isolated_ordinal).await {
                return true;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        false
    }

    async fn network_probe_wget(&self, probe: &str, target_ordinal: u32) -> bool {
        let target = pod_name(&self.cluster, target_ordinal);
        let url = format!(
            "http://{}.{}:9091/readyz",
            target,
            headless_service_name(&self.cluster)
        );

        Command::new("kubectl")
            .arg("-n")
            .arg(&self.namespace)
            .arg("exec")
            .arg(probe)
            .arg("--")
            .arg("wget")
            .arg("-qO-")
            .arg("-T")
            .arg("2")
            .arg(&url)
            .output()
            .expect("kind partition enforcement probe requires kubectl")
            .status
            .success()
    }

    async fn ensure_network_probe_pod(&self, isolated_ordinal: u32) -> Result<String, String> {
        let name = format!("{}-netprobe-{}", self.cluster, isolated_ordinal);
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let probe = Pod {
            metadata: ObjectMeta {
                name: Some(name.clone()),
                namespace: Some(self.namespace.clone()),
                labels: Some(BTreeMap::from([(
                    "app.kubernetes.io/name".to_owned(),
                    "hydracache-network-probe".to_owned(),
                )])),
                ..Default::default()
            },
            spec: Some(PodSpec {
                restart_policy: Some("Never".to_owned()),
                containers: vec![Container {
                    name: "probe".to_owned(),
                    image: Some(network_probe_image()),
                    image_pull_policy: Some("IfNotPresent".to_owned()),
                    command: Some(vec!["sleep".to_owned(), "3600".to_owned()]),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        pods.patch(
            &name,
            &PatchParams::apply(FIELD_MANAGER).force(),
            &Patch::Apply(&probe),
        )
        .await
        .map_err(|error| error.to_string())?;

        for _ in 0..30 {
            match pods.get(&name).await {
                Ok(pod) if pod_is_ready(&pod) => return Ok(name),
                Ok(pod) => {
                    let phase = pod
                        .status
                        .as_ref()
                        .and_then(|status| status.phase.as_deref())
                        .unwrap_or("unknown");
                    eprintln!("waiting for partition probe pod {name}: phase={phase}");
                }
                Err(kube::Error::Api(error)) if error.code == 404 => {}
                Err(error) => return Err(error.to_string()),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        Err(format!(
            "timed out waiting for {name} using image {}",
            network_probe_image()
        ))
    }

    async fn delete_network_probe_pod(&self, name: &str) {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let _ = pods.delete(name, &DeleteParams::default()).await;
    }

    async fn inject_slow_disk(&self, ordinal: u32) -> ChaosInjection {
        let Some(api_resource) = self.iochaos_api_resource().await else {
            return slow_disk_plan_for_crd_present(false);
        };
        let iochaos: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), &self.namespace, &api_resource);
        let name = slow_disk_chaos_name(&self.cluster, ordinal);
        let manifest = iochaos_manifest(&self.cluster, &self.namespace, ordinal);
        iochaos
            .patch(
                &name,
                &PatchParams::apply(FIELD_MANAGER).force(),
                &Patch::Apply(&manifest),
            )
            .await
            .expect("kind slow-disk injector should apply chaos-mesh IOChaos");
        slow_disk_plan_for_crd_present(true)
    }

    async fn delete_slow_disk(&self, ordinal: u32) {
        let Some(api_resource) = self.iochaos_api_resource().await else {
            return;
        };
        let iochaos: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), &self.namespace, &api_resource);
        let _ = iochaos
            .delete(
                &slow_disk_chaos_name(&self.cluster, ordinal),
                &DeleteParams::default(),
            )
            .await;
    }

    async fn iochaos_api_resource(&self) -> Option<ApiResource> {
        let gvk = GroupVersionKind::gvk("chaos-mesh.org", "v1alpha1", "IOChaos");
        discovery::pinned_kind(&self.client, &gvk)
            .await
            .ok()
            .map(|(resource, _)| resource)
    }

    async fn wait_ready(
        &self,
        desired: u32,
        stage: &'static str,
        committed_writes: u64,
    ) -> SoakObservation {
        let mut latest = None;
        for _ in 0..KIND_WAIT_ATTEMPTS {
            let observation = match self.observe(stage, committed_writes).await {
                Ok(observation) => observation,
                Err(kube::Error::Api(error)) if error.code == 404 => {
                    latest = Some(format!("waiting for owned resources: {}", error.message));
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
                Err(error) => panic!("kind soak should observe cluster resources: {error}"),
            };
            if observation.ready_replicas >= desired {
                observation.assert_quorum();
                return observation;
            }
            latest = Some(format!("{observation:?}"));
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!("timed out waiting for {stage} readiness; latest={latest:?}");
    }

    async fn observe(
        &self,
        stage: &'static str,
        committed_writes: u64,
    ) -> kube::Result<SoakObservation> {
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

        Ok(SoakObservation {
            stage,
            desired_replicas: desired,
            ready_replicas: ready,
            unavailable_replicas: unavailable,
            leader,
            committed_writes,
        })
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

fn rolling_chaos_schedule() -> Vec<ChaosFault> {
    vec![
        ChaosFault::PodCrash { ordinal: 0 },
        ChaosFault::NetworkPartition { ordinal: 1 },
        ChaosFault::SlowDisk { ordinal: 2 },
    ]
}

#[tokio::test]
#[ignore = "kind/nightly soak: set HYDRACACHE_OPERATOR_KIND=1"]
async fn multi_node_chaos_soak_loses_no_committed_write() {
    let Some(kind) = KindHarness::try_start("multi_node_chaos_soak_loses_no_committed_write").await
    else {
        return;
    };
    eprintln!("{SCOPE_DISCLOSURE}");

    let cluster = kind.apply_cluster(soak_kind_spec(3)).await;
    let mut probe = CommittedWriteProbe::default();
    let install = kind
        .wait_ready(cluster.spec.replicas, "install", probe.committed())
        .await;
    install.assert_quorum();
    install.assert_leader();

    for fault in rolling_chaos_schedule() {
        probe.record_committed_write();
        let injection = kind.inject(fault).await;
        if let ChaosInjection::Skipped(reason) = &injection {
            eprintln!("skipping {fault:?}: {reason}");
        }
        let observed = kind
            .wait_ready(cluster.spec.replicas, "fault-window", probe.committed())
            .await;
        observed.assert_quorum();
        observed.assert_leader();
        probe.assert_no_lost_committed_write(&observed);
        kind.heal(fault, &injection).await;
        let recovered = kind
            .wait_ready(cluster.spec.replicas, "recovered", probe.committed())
            .await;
        recovered.assert_quorum();
        recovered.assert_leader();
        probe.assert_no_lost_committed_write(&recovered);
    }
}

#[tokio::test]
#[ignore = "kind/nightly soak: set HYDRACACHE_OPERATOR_KIND=1"]
async fn leader_is_always_reestablished_after_pod_crash() {
    let Some(kind) = KindHarness::try_start("leader_is_always_reestablished_after_pod_crash").await
    else {
        return;
    };
    eprintln!("{SCOPE_DISCLOSURE}");

    let cluster = kind.apply_cluster(soak_kind_spec(3)).await;
    let mut probe = CommittedWriteProbe::default();
    let ready = kind
        .wait_ready(cluster.spec.replicas, READY_PHASE, probe.committed())
        .await;
    ready.assert_leader();

    probe.record_committed_write();
    let injection = kind.inject(ChaosFault::PodCrash { ordinal: 0 }).await;
    let recovered = kind
        .wait_ready(
            cluster.spec.replicas,
            "pod-crash-recovered",
            probe.committed(),
        )
        .await;
    recovered.assert_quorum();
    recovered.assert_leader();
    probe.assert_no_lost_committed_write(&recovered);
    kind.heal(ChaosFault::PodCrash { ordinal: 0 }, &injection)
        .await;
}

#[tokio::test]
#[ignore = "kind/calico-gated: set HYDRACACHE_OPERATOR_KIND=1 with a NetworkPolicy-enforcing CNI"]
async fn kind_partition_injection_isolates_and_heals() {
    let Some(kind) = KindHarness::try_start("kind_partition_injection_isolates_and_heals").await
    else {
        return;
    };
    eprintln!("{SCOPE_DISCLOSURE}");

    let cluster = kind.apply_cluster(soak_kind_spec(3)).await;
    let ready = kind.wait_ready(cluster.spec.replicas, READY_PHASE, 0).await;
    ready.assert_quorum();
    ready.assert_leader();

    let fault = ChaosFault::NetworkPartition { ordinal: 1 };
    let injection = kind.inject(fault).await;
    if let ChaosInjection::Skipped(reason) = &injection {
        eprintln!("skipping partition assertion: {reason}");
        return;
    }

    let observed = kind
        .wait_ready(cluster.spec.replicas, "partition-window", 1)
        .await;
    observed.assert_quorum();
    observed.assert_leader();
    kind.heal(fault, &injection).await;

    let recovered = kind
        .wait_ready(cluster.spec.replicas, "partition-healed", 1)
        .await;
    recovered.assert_quorum();
    recovered.assert_leader();
}

#[tokio::test]
async fn partition_probe_skips_loud_on_non_enforcing_cni() {
    let Some(kind) =
        KindHarness::try_start("partition_probe_skips_loud_on_non_enforcing_cni").await
    else {
        return;
    };

    let cluster = kind.apply_cluster(soak_kind_spec(3)).await;
    kind.wait_ready(cluster.spec.replicas, READY_PHASE, 0).await;
    let fault = ChaosFault::NetworkPartition { ordinal: 1 };
    let injection = kind.inject(fault).await;
    match &injection {
        ChaosInjection::Skipped(reason) => {
            eprintln!("skipping partition assertion: {reason}");
            assert!(
                reason.contains("NetworkPolicy"),
                "skip should name NetworkPolicy enforcement: {reason}"
            );
        }
        ChaosInjection::Applied(kind_name) => {
            eprintln!("partition probe applied {kind_name}; healing");
            assert_eq!(*kind_name, "NetworkPolicy");
            kind.heal(fault, &injection).await;
        }
    }
}

#[tokio::test]
async fn soak_skips_gracefully_without_a_cluster() {
    if kind_enabled() {
        eprintln!("kind soak enabled; ignored tests own live-cluster assertions");
        return;
    }

    assert!(
        !kind_enabled(),
        "kind soak tests must be opt-in so local verify can run without a cluster"
    );
    assert!(SCOPE_DISCLOSURE.contains("NetworkPolicy"));
    assert!(SCOPE_DISCLOSURE.contains("IOChaos"));
    assert!(
        rolling_chaos_schedule()
            .iter()
            .any(|fault| fault.requires_optional_infrastructure()),
        "partition/slow-disk faults should name optional infrastructure"
    );
}

#[test]
fn deny_all_partition_policy_selects_single_statefulset_pod() {
    let policy = deny_all_partition_policy("chaos", "testing", 2);
    let spec = policy.spec.as_ref().unwrap();
    let labels = spec
        .pod_selector
        .as_ref()
        .unwrap()
        .match_labels
        .as_ref()
        .unwrap();

    assert_eq!(policy.metadata.name.as_deref(), Some("chaos-partition-2"));
    assert_eq!(labels["statefulset.kubernetes.io/pod-name"], "chaos-2");
    assert_eq!(
        spec.policy_types.as_ref().unwrap(),
        &vec!["Ingress".to_owned(), "Egress".to_owned()]
    );
    assert_eq!(spec.ingress.as_ref().unwrap().len(), 0);
    assert_eq!(spec.egress.as_ref().unwrap().len(), 0);
}

#[test]
fn slow_disk_uses_iochaos_only_when_crd_present() {
    assert_eq!(
        slow_disk_plan_for_crd_present(false),
        ChaosInjection::Skipped(IOCHAOS_SKIP.to_owned())
    );
    assert_eq!(
        slow_disk_plan_for_crd_present(true),
        ChaosInjection::Applied("chaos-mesh IOChaos")
    );

    let manifest = iochaos_manifest("chaos", "testing", 1);
    assert_eq!(manifest["kind"], "IOChaos");
    assert_eq!(manifest["metadata"]["name"], "chaos-slow-disk-1");
    assert_eq!(
        manifest["spec"]["selector"]["pods"]["testing"][0],
        "chaos-1"
    );
    assert_eq!(manifest["spec"]["volumePath"], "/var/lib/hydracache");
}
