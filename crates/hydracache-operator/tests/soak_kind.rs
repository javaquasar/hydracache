use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::panic::{resume_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::FutureExt;
use hydracache_operator::controller::READY_PHASE;
use hydracache_operator::crd::{sample_spec, HydraCacheCluster, HydraCacheClusterSpec};
use hydracache_operator::resources::{
    headless_service_name, APP_LABEL, COMPONENT_LABEL, FIELD_MANAGER, INSTANCE_LABEL,
    MANAGED_BY_LABEL, SERVER_CONTAINER,
};
use hydracache_operator::scale::{
    plan_scale, pod_name, quorum_for, AdminAction, AdminStatus, ScaleObservation,
};
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{Container, Pod, PodCondition, PodSpec, PodStatus},
    networking::v1::{NetworkPolicy, NetworkPolicySpec},
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use kube::api::{DeleteParams, ListParams, Patch, PatchParams, Preconditions};
use kube::{
    api::{ApiResource, DynamicObject, GroupVersionKind},
    discovery, Api,
};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{json, Value};

const KIND_ENV: &str = "HYDRACACHE_OPERATOR_KIND";
const NAMESPACE_ENV: &str = "HYDRACACHE_OPERATOR_NAMESPACE";
const CLUSTER_ENV: &str = "HYDRACACHE_OPERATOR_CLUSTER";
const IMAGE_ENV: &str = "HYDRACACHE_OPERATOR_IMAGE";
const VERSION_ENV: &str = "HYDRACACHE_OPERATOR_VERSION";
const REQUIRE_IOCHAOS_ENV: &str = "HYDRACACHE_OPERATOR_REQUIRE_IOCHAOS";
const NETWORK_PROBE_IMAGE_ENV: &str = "HYDRACACHE_NETWORK_PROBE_IMAGE";
const OPERATOR_EVIDENCE_DIRECTORY: &str = "target/test-evidence/0.66";
const OPERATOR_EVIDENCE_NONCE_ENV: &str = "HYDRACACHE_OPERATOR_EVIDENCE_NONCE";
const OPERATOR_CONTROLLER_LIVE_LOG: &str = "operator-controller-live.log";
const OPERATOR_CONTROLLER_RECEIPT_LOG: &str = "operator-controller.log";
// All live Kind tests operate on the same release cluster and Chaos Mesh
// objects. The Rust test harness may otherwise run them concurrently and let
// one proof replace the pods or IOChaos object observed by another proof.
static LIVE_KIND_PROOF_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[cfg(target_os = "linux")]
const OPERATOR_CONTROLLER_PID: &str = "operator-controller.pid";
#[cfg(target_os = "linux")]
const OPERATOR_CONTROLLER_BINARY_ENV: &str = "HYDRACACHE_OPERATOR_BINARY";
#[cfg(target_os = "linux")]
const OPERATOR_CONTROLLER_RUNTIME_DIRECTORY: &str = ".ci-runtime/0.66";
const OPERATOR_W5_CAPABILITY_ARTIFACT: &str = "operator-kind-w5-iochaos-capability.txt";
const OPERATOR_W11_CAPABILITY_ARTIFACT: &str = "operator-kind-w11-network-policy-capability.txt";
const OPERATOR_POD_LOG_ARTIFACT: &str = "operator-kind-pod-logs.txt";
const OPERATOR_RESOURCES_ARTIFACT: &str = "operator-kind-resources.txt";
const OPERATOR_EVENTS_ARTIFACT: &str = "operator-kind-events.txt";
const STATEFULSET_REVISION_LABEL: &str = "controller-revision-hash";
// Kind nodes can spend several reconciliation periods electing a leader after
// Chaos Mesh starts/stops an injection. Keep the wait bounded, but allow three
// minutes so the assertion observes a settled quorum rather than a transient
// `leader=None` status.
const KIND_WAIT_ATTEMPTS: usize = 150;
static SCALE_ADMIN_PROBE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const NETWORK_POLICY_SKIP: &str =
    "CNI does not enforce NetworkPolicy; install calico/cilium in the kind config";
const IOCHAOS_SKIP: &str =
    "chaos-mesh IOChaos CRD is not installed; slow-disk remains an external dependency";
const SCOPE_DISCLOSURE: &str = "0.66 kind chaos: NetworkPartition uses Kubernetes NetworkPolicy only when a CNI enforcement probe proves policy is active; SlowDisk targets the exact Raft-log path with chaos-mesh IOChaos and is accepted only after Selected=True/AllInjected=True for the current pod UID. Each unsupported leg skips loud, never wrong-but-green.";

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

fn iochaos_required() -> bool {
    std::env::var(REQUIRE_IOCHAOS_ENV).as_deref() == Ok("1")
}

fn operator_repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn operator_evidence_path(file_name: &str) -> PathBuf {
    operator_repository_root()
        .join(OPERATOR_EVIDENCE_DIRECTORY)
        .join(file_name)
}

#[derive(Clone, Copy)]
enum KubectlStdoutRequirement<'a> {
    AllowEmpty,
    NonEmpty,
    Contains(&'a str),
}

fn validate_kubectl_stdout(
    file_name: &str,
    stdout: &str,
    requirement: KubectlStdoutRequirement<'_>,
) -> Result<(), String> {
    let stdout = stdout.trim();
    match requirement {
        KubectlStdoutRequirement::AllowEmpty => Ok(()),
        KubectlStdoutRequirement::NonEmpty if stdout.is_empty() => Err(format!(
            "kubectl evidence capture for {file_name} returned empty stdout"
        )),
        KubectlStdoutRequirement::NonEmpty if stdout.contains("No resources found") => {
            Err(format!(
                "kubectl evidence capture for {file_name} selected no resources"
            ))
        }
        KubectlStdoutRequirement::NonEmpty => Ok(()),
        KubectlStdoutRequirement::Contains(expected) if stdout.contains(expected) => Ok(()),
        KubectlStdoutRequirement::Contains(expected) => Err(format!(
            "kubectl evidence capture for {file_name} did not contain expected identity {expected:?}"
        )),
    }
}

fn capture_kubectl_artifact(
    file_name: &str,
    args: &[String],
    stdout_requirement: KubectlStdoutRequirement<'_>,
) -> Result<(), String> {
    let output = Command::new("kubectl")
        .args(args)
        .output()
        .map_err(|error| format!("could not run kubectl for {file_name}: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut artifact = format!("$ kubectl {}\nstatus={}\n", args.join(" "), output.status);
    artifact.push_str("--- stdout ---\n");
    artifact.push_str(&stdout);
    artifact.push_str("\n--- stderr ---\n");
    artifact.push_str(&stderr);
    fs::write(operator_evidence_path(file_name), artifact)
        .map_err(|error| format!("could not write {file_name}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "kubectl evidence capture for {file_name} failed with {}",
            output.status
        ));
    }
    validate_kubectl_stdout(file_name, &stdout, stdout_requirement)
}

fn operator_evidence_nonce() -> Result<String, String> {
    let nonce = std::env::var(OPERATOR_EVIDENCE_NONCE_ENV)
        .map_err(|_| format!("{OPERATOR_EVIDENCE_NONCE_ENV} is required"))?;
    if nonce.is_empty()
        || nonce.len() > 200
        || !nonce
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!(
            "{OPERATOR_EVIDENCE_NONCE_ENV} must be 1..=200 ASCII letters, digits, '.', '_' or '-'"
        ));
    }
    Ok(nonce)
}

fn validate_controller_runtime_log(controller_log: &str, nonce: &str) -> Result<(), String> {
    let start_marker = format!("HC-OPERATOR-CONTROLLER-START nonce={nonce} ");
    let runtime_marker = format!("HC-OPERATOR-CONTROLLER-RUNTIME nonce={nonce} ");
    if !controller_log
        .lines()
        .any(|line| line.starts_with(&start_marker))
    {
        return Err(format!(
            "live operator controller log is missing current-run start marker for nonce {nonce}"
        ));
    }
    if !controller_log
        .lines()
        .any(|line| line.starts_with(&runtime_marker))
    {
        return Err(format!(
            "live operator controller log has no controller runtime output for nonce {nonce}"
        ));
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct OperatorControllerAttestation {
    pid: u32,
    binary: PathBuf,
}

#[cfg(target_os = "linux")]
fn attest_live_operator_controller() -> Result<OperatorControllerAttestation, String> {
    use std::os::unix::fs::MetadataExt;

    let pid_path = operator_evidence_path(OPERATOR_CONTROLLER_PID);
    let pid_text = fs::read_to_string(&pid_path)
        .map_err(|error| format!("could not read {}: {error}", pid_path.display()))?;
    let pid = pid_text
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|pid| *pid > 0)
        .ok_or_else(|| format!("{} does not contain a live PID", pid_path.display()))?;
    let proc_directory = PathBuf::from(format!("/proc/{pid}"));
    if !proc_directory.is_dir() {
        return Err(format!("operator controller PID {pid} is not live"));
    }

    let configured_binary = std::env::var(OPERATOR_CONTROLLER_BINARY_ENV)
        .map_err(|_| format!("{OPERATOR_CONTROLLER_BINARY_ENV} is required"))?;
    if configured_binary.trim().is_empty() {
        return Err(format!(
            "{OPERATOR_CONTROLLER_BINARY_ENV} must not be empty"
        ));
    }
    let configured_binary = PathBuf::from(configured_binary);
    if !configured_binary.is_absolute() {
        return Err(format!(
            "{OPERATOR_CONTROLLER_BINARY_ENV} must name an absolute candidate path, got {}",
            configured_binary.display()
        ));
    }
    let runtime_directory = operator_repository_root().join(OPERATOR_CONTROLLER_RUNTIME_DIRECTORY);
    let runtime_directory = fs::canonicalize(&runtime_directory).map_err(|error| {
        format!(
            "could not resolve operator candidate directory {}: {error}",
            runtime_directory.display()
        )
    })?;
    let expected_binary = configured_binary;
    let expected_binary = fs::canonicalize(&expected_binary).map_err(|error| {
        format!(
            "could not resolve expected candidate operator binary {}: {error}",
            expected_binary.display()
        )
    })?;
    if !expected_binary.starts_with(&runtime_directory) {
        return Err(format!(
            "{OPERATOR_CONTROLLER_BINARY_ENV} resolved outside the dedicated candidate directory {}: {}",
            runtime_directory.display(),
            expected_binary.display()
        ));
    }
    let proc_exe = proc_directory.join("exe");
    let proc_exe_target = fs::read_link(&proc_exe).map_err(|error| {
        format!("could not read live operator PID {pid} executable link: {error}")
    })?;
    let running_binary = fs::canonicalize(&proc_exe).map_err(|error| {
        format!(
            "could not resolve live operator PID {pid} executable target {}: {error}",
            proc_exe_target.display()
        )
    })?;
    let expected_metadata = fs::metadata(&expected_binary).map_err(|error| {
        format!(
            "could not inspect candidate operator binary {}: {error}",
            expected_binary.display()
        )
    })?;
    let running_metadata = fs::metadata(&proc_exe)
        .map_err(|error| format!("could not inspect live operator PID {pid}: {error}"))?;
    if running_binary != expected_binary
        || running_metadata.dev() != expected_metadata.dev()
        || running_metadata.ino() != expected_metadata.ino()
    {
        return Err(format!(
            "operator PID {pid} runs {}, expected exact candidate binary {}",
            running_binary.display(),
            expected_binary.display()
        ));
    }
    Ok(OperatorControllerAttestation {
        pid,
        binary: expected_binary,
    })
}

#[cfg(not(target_os = "linux"))]
fn attest_live_operator_controller() -> Result<OperatorControllerAttestation, String> {
    Err("operator release evidence requires Linux /proc process attestation".to_owned())
}

fn capture_operator_kind_release_evidence() -> Result<(), String> {
    if !iochaos_required() {
        return Err(format!(
            "{REQUIRE_IOCHAOS_ENV}=1 is required before operator release evidence can be captured"
        ));
    }

    let evidence_directory = operator_evidence_path("");
    fs::create_dir_all(&evidence_directory).map_err(|error| {
        format!(
            "could not create operator evidence directory {}: {error}",
            evidence_directory.display()
        )
    })?;
    let namespace = namespace();
    let cluster = cluster_name();
    let selector = server_pod_selector(&cluster);
    let nonce = operator_evidence_nonce()?;

    capture_kubectl_artifact(
        OPERATOR_POD_LOG_ARTIFACT,
        &[
            "logs".to_owned(),
            "-n".to_owned(),
            namespace.clone(),
            "--selector".to_owned(),
            selector,
            "--all-containers=true".to_owned(),
            "--prefix=true".to_owned(),
            "--tail=-1".to_owned(),
        ],
        KubectlStdoutRequirement::NonEmpty,
    )?;
    capture_kubectl_artifact(
        OPERATOR_RESOURCES_ARTIFACT,
        &[
            "get".to_owned(),
            "pods,statefulsets,services,hydracacheclusters".to_owned(),
            "-A".to_owned(),
            "-o".to_owned(),
            "wide".to_owned(),
        ],
        KubectlStdoutRequirement::Contains(&cluster),
    )?;
    capture_kubectl_artifact(
        OPERATOR_EVENTS_ARTIFACT,
        &[
            "get".to_owned(),
            "events".to_owned(),
            "-A".to_owned(),
            "--sort-by=.lastTimestamp".to_owned(),
        ],
        KubectlStdoutRequirement::AllowEmpty,
    )?;

    let attestation_before = attest_live_operator_controller()?;
    let live_log = operator_evidence_path(OPERATOR_CONTROLLER_LIVE_LOG);
    let controller_log = fs::read_to_string(&live_log).map_err(|error| {
        format!(
            "could not read live operator controller log {}: {error}",
            live_log.display()
        )
    })?;
    validate_controller_runtime_log(&controller_log, &nonce)?;
    let attestation_after = attest_live_operator_controller()?;
    if attestation_after != attestation_before {
        return Err("operator controller identity changed while evidence was captured".to_owned());
    }
    let receipt_log = format!(
        "release=0.66.0\nnonce={nonce}\npid={}\nbinary={}\n--- controller runtime ---\n{controller_log}",
        attestation_after.pid,
        attestation_after.binary.display()
    );
    fs::write(
        operator_evidence_path(OPERATOR_CONTROLLER_RECEIPT_LOG),
        receipt_log,
    )
    .map_err(|error| format!("could not snapshot operator controller log: {error}"))?;
    Ok(())
}

fn write_operator_capability_artifact(file_name: &str, evidence: &str) -> Result<(), String> {
    let evidence_directory = operator_evidence_path("");
    fs::create_dir_all(&evidence_directory).map_err(|error| {
        format!(
            "could not create operator evidence directory {}: {error}",
            evidence_directory.display()
        )
    })?;
    fs::write(operator_evidence_path(file_name), evidence)
        .map_err(|error| format!("could not write operator capability artifact: {error}"))
}

fn soak_kind_spec(replicas: u32) -> HydraCacheClusterSpec {
    let mut spec = sample_spec();
    spec.image = std::env::var(IMAGE_ENV).unwrap_or_else(|_| {
        panic!("{KIND_ENV}=1 soak tests require {IMAGE_ENV}=<current hydracache-server image>")
    });
    spec.version = std::env::var(VERSION_ENV).unwrap_or_else(|_| "0.62.0-dev".to_owned());
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

fn require_scale_partition_capability(injection: &ChaosInjection) -> Result<&'static str, String> {
    match injection {
        ChaosInjection::Applied(injector) => Ok(*injector),
        ChaosInjection::Skipped(reason) => Err(format!(
            "{KIND_ENV}=1 W11 scale-chaos lane requires an enforcing CNI: {reason}"
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SoakObservation {
    stage: &'static str,
    desired_replicas: u32,
    ready_replicas: u32,
    unavailable_replicas: u32,
    leader: Option<String>,
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

/// Deterministic authority model paired with the production scale planner.
///
/// Kubernetes readiness and the committed Raft voter set deliberately remain
/// separate: a crashed process stops running but remains a voter, while an
/// explicit drain removes the voter only after the modeled membership commit.
#[derive(Debug, Clone)]
struct ScaleChaosModel {
    running: BTreeSet<u32>,
    voters: BTreeSet<u32>,
    partitioned: BTreeSet<u32>,
    committed_metadata: BTreeSet<String>,
    applied_metadata: BTreeMap<u32, BTreeSet<String>>,
}

impl ScaleChaosModel {
    fn with_replicas(replicas: u32) -> Self {
        let voters = (0..replicas).collect::<BTreeSet<_>>();
        Self {
            running: voters.clone(),
            voters,
            partitioned: BTreeSet::new(),
            committed_metadata: BTreeSet::new(),
            applied_metadata: (0..replicas)
                .map(|ordinal| (ordinal, BTreeSet::new()))
                .collect(),
        }
    }

    fn partition(&mut self, ordinal: u32) {
        assert!(self.voters.contains(&ordinal));
        self.partitioned.insert(ordinal);
    }

    fn heal(&mut self, ordinal: u32) {
        assert!(self.partitioned.remove(&ordinal));
        if self.running.contains(&ordinal) && self.voters.contains(&ordinal) {
            self.applied_metadata
                .insert(ordinal, self.committed_metadata.clone());
        }
    }

    fn crash(&mut self, ordinal: u32) {
        assert!(self.running.remove(&ordinal));
    }

    fn restart(&mut self, ordinal: u32) {
        assert!(self.voters.contains(&ordinal));
        self.running.insert(ordinal);
        if !self.partitioned.contains(&ordinal) {
            self.applied_metadata
                .insert(ordinal, self.committed_metadata.clone());
        }
    }

    fn add_voter(&mut self, ordinal: u32) {
        assert!(self.voters.insert(ordinal));
        self.running.insert(ordinal);
        self.applied_metadata
            .insert(ordinal, self.committed_metadata.clone());
    }

    fn drain(&mut self, ordinal: u32) {
        assert!(self.running.remove(&ordinal));
        assert!(self.voters.remove(&ordinal));
        self.partitioned.remove(&ordinal);
        self.applied_metadata.remove(&ordinal);
    }

    fn commit_metadata(&mut self, command_id: &str) {
        let reachable = self
            .voters
            .iter()
            .copied()
            .filter(|ordinal| self.running.contains(ordinal) && !self.partitioned.contains(ordinal))
            .collect::<Vec<_>>();
        let required = quorum_for(self.voters.len() as u32) as usize;
        assert!(
            reachable.len() >= required,
            "metadata command {command_id} had no quorum: reachable={} required={required}",
            reachable.len()
        );
        assert!(
            self.committed_metadata.insert(command_id.to_owned()),
            "test schedule reused command id {command_id}"
        );
        for ordinal in reachable {
            self.applied_metadata
                .entry(ordinal)
                .or_default()
                .insert(command_id.to_owned());
        }
    }

    fn assert_all_authoritative_voters_caught_up(&self) {
        for ordinal in &self.voters {
            if self.running.contains(ordinal) && !self.partitioned.contains(ordinal) {
                assert_eq!(
                    self.applied_metadata.get(ordinal),
                    Some(&self.committed_metadata),
                    "authoritative voter {ordinal} lost committed metadata"
                );
            }
        }
    }
}

fn scale_target(name: &str, replicas: u32) -> HydraCacheCluster {
    let mut spec = sample_spec();
    spec.replicas = replicas;
    let mut cluster = HydraCacheCluster::new(name, spec);
    cluster.metadata.namespace = Some("default".to_owned());
    cluster.metadata.uid = Some(format!("{name}-uid"));
    cluster.metadata.generation = Some(66);
    cluster
}

fn scale_observation(
    current_replicas: u32,
    ready_replicas: u32,
    members: u32,
    voters: u32,
) -> ScaleObservation {
    ScaleObservation {
        current_replicas,
        ready_replicas,
        previous_phase: None,
        drain_requested_for: None,
        drain_complete_for: None,
        admin_status: Some(AdminStatus {
            leader: Some("scale-chaos-0".to_owned()),
            quorum_ok: true,
            members,
            voters,
            reshard_phase: "idle".to_owned(),
            draining: false,
        }),
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ScaleChaosAdminStatus {
    source: String,
    leader: Option<String>,
    epoch: u64,
    quorum_ok: bool,
    members: u32,
    member_ids: Vec<String>,
    voters: u32,
    voter_ids: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct RaftCompactionObservation {
    available: bool,
    applied_index: Option<u64>,
}

#[derive(Debug, Clone)]
struct RaftNodeObservation {
    ordinal: u32,
    status: ScaleChaosAdminStatus,
    compaction: RaftCompactionObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentPodIdentity {
    ordinal: u32,
    name: String,
    uid: String,
    revision: String,
}

impl RaftNodeObservation {
    fn applied_index(&self) -> u64 {
        self.compaction
            .applied_index
            .expect("live Sled-backed kind daemon must expose applied_index")
    }
}

fn exact_current_pod_identities(
    cluster: &str,
    replicas: u32,
    revision: &str,
    excluded_ordinal: Option<u32>,
    pods: &[Pod],
) -> Result<Vec<CurrentPodIdentity>, String> {
    let expected = (0..replicas)
        .filter(|ordinal| excluded_ordinal != Some(*ordinal))
        .map(|ordinal| (pod_name(cluster, ordinal), ordinal))
        .collect::<BTreeMap<_, _>>();
    let mut identities = Vec::new();
    for pod in pods {
        if pod.metadata.deletion_timestamp.is_some() || !pod_is_ready(pod) {
            continue;
        }
        let Some(name) = pod.metadata.name.as_deref() else {
            continue;
        };
        if excluded_ordinal.is_some_and(|ordinal| name == pod_name(cluster, ordinal)) {
            continue;
        }
        let ordinal = expected
            .get(name)
            .copied()
            .ok_or_else(|| format!("unexpected current Ready pod {name} at revision {revision}"))?;
        let pod_revision = pod
            .metadata
            .labels
            .as_ref()
            .and_then(|labels| labels.get(STATEFULSET_REVISION_LABEL));
        if pod_revision.map(String::as_str) != Some(revision) {
            return Err(format!(
                "current Ready pod {name} has revision {pod_revision:?}, expected {revision}"
            ));
        }
        let uid = pod
            .metadata
            .uid
            .clone()
            .filter(|uid| !uid.is_empty())
            .ok_or_else(|| format!("current Ready pod {name} has no Kubernetes UID"))?;
        identities.push(CurrentPodIdentity {
            ordinal,
            name: name.to_owned(),
            uid,
            revision: revision.to_owned(),
        });
    }
    identities.sort_by_key(|identity| identity.ordinal);
    let observed = identities
        .iter()
        .map(|identity| (identity.name.clone(), identity.ordinal))
        .collect::<BTreeMap<_, _>>();
    if observed != expected || identities.len() != expected.len() {
        return Err(format!(
            "current Ready pod set mismatch at revision {revision}: observed={:?} expected={:?}",
            observed.keys().collect::<Vec<_>>(),
            expected.keys().collect::<Vec<_>>()
        ));
    }
    Ok(identities)
}

fn raft_observations_converged(
    observations: &[RaftNodeObservation],
    expected_ordinals: &BTreeSet<u32>,
    expected_member_ids: &BTreeSet<String>,
    expected_voter_ids: &BTreeSet<u64>,
    expected_voters: u32,
    minimum_epoch: u64,
    minimum_applied: u64,
) -> bool {
    let observed_ordinals = observations
        .iter()
        .map(|observation| observation.ordinal)
        .collect::<BTreeSet<_>>();
    let authority = observations
        .first()
        .map(|observation| (observation.status.epoch, observation.status.leader.clone()));
    observations.len() == expected_ordinals.len()
        && observed_ordinals == *expected_ordinals
        && observations.iter().all(|observation| {
            let member_ids = observation
                .status
                .member_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            let voter_ids = observation
                .status
                .voter_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            observation.status.source == "live"
                && observation.status.voters == expected_voters
                && observation.status.members == expected_voters
                && observation.status.member_ids.len() == expected_member_ids.len()
                && &member_ids == expected_member_ids
                && observation.status.voter_ids.len() == expected_voter_ids.len()
                && &voter_ids == expected_voter_ids
                && observation.status.quorum_ok
                && observation.status.leader.is_some()
                && observation.status.epoch >= minimum_epoch
                && observation.compaction.available
                && observation
                    .compaction
                    .applied_index
                    .is_some_and(|index| index >= minimum_applied)
        })
        && authority.is_some_and(|authority| {
            observations.iter().all(|observation| {
                (observation.status.epoch, observation.status.leader.clone()) == authority
            })
        })
}

fn stable_raft_node_id(node_id: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    let mut hash = FNV_OFFSET;
    for byte in node_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash.max(1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IoChaosTarget {
    namespace: String,
    pod: String,
    pod_uid: String,
    ordinal: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IoChaosReceipt {
    chaos_uid: String,
    target: IoChaosTarget,
}

fn partition_policy_name(cluster: &str, ordinal: u32) -> String {
    format!("{cluster}-partition-{ordinal}")
}

fn slow_disk_chaos_name(cluster: &str, ordinal: u32) -> String {
    format!("{cluster}-slow-disk-{ordinal}")
}

fn pod_crash_delete_params(uid: &str) -> DeleteParams {
    DeleteParams {
        preconditions: Some(Preconditions {
            uid: Some(uid.to_owned()),
            ..Default::default()
        }),
        ..Default::default()
    }
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
            "action": "fault",
            "mode": "one",
            "selector": {
                "namespaces": [namespace],
                "pods": {
                    namespace: [pod],
                },
            },
            "containerNames": [SERVER_CONTAINER],
            "volumePath": "/var/lib/hydracache",
            "path": "/var/lib/hydracache/raft-log/**/*",
            "methods": ["WRITE", "FLUSH", "FSYNC"],
            "errno": 5,
            "percent": 100,
            "duration": "10m",
        },
    })
}

fn iochaos_condition_is_true(object: &Value, condition_type: &str) -> bool {
    object
        .pointer("/status/conditions")
        .and_then(Value::as_array)
        .is_some_and(|conditions| {
            conditions.iter().any(|condition| {
                condition.get("type").and_then(Value::as_str) == Some(condition_type)
                    && condition.get("status").and_then(Value::as_str) == Some("True")
            })
        })
}

fn iochaos_injection_receipt(
    object: &Value,
    target: &IoChaosTarget,
    current_pod_uid: &str,
) -> Result<IoChaosReceipt, String> {
    if current_pod_uid != target.pod_uid {
        return Err(format!(
            "target pod {} was replaced during injection: expected uid={} current uid={current_pod_uid}",
            target.pod, target.pod_uid
        ));
    }

    let chaos_uid = object
        .pointer("/metadata/uid")
        .and_then(Value::as_str)
        .filter(|uid| !uid.is_empty())
        .ok_or("IOChaos has no Kubernetes UID")?;
    if object
        .pointer("/metadata/namespace")
        .and_then(Value::as_str)
        != Some(target.namespace.as_str())
    {
        return Err("IOChaos namespace does not match the target namespace".to_owned());
    }

    let expected_pods = json!({ target.namespace.clone(): [target.pod.clone()] });
    let exact_selector = object.pointer("/spec/selector/namespaces")
        == Some(&json!([target.namespace.clone()]))
        && object.pointer("/spec/selector/pods") == Some(&expected_pods);
    if !exact_selector {
        return Err(format!(
            "IOChaos selector is not the exact target {}/{}",
            target.namespace, target.pod
        ));
    }

    let exact_fault = object.pointer("/spec/containerNames") == Some(&json!([SERVER_CONTAINER]))
        && object.pointer("/spec/action") == Some(&json!("fault"))
        && object.pointer("/spec/volumePath") == Some(&json!("/var/lib/hydracache"))
        && object.pointer("/spec/path") == Some(&json!("/var/lib/hydracache/raft-log/**/*"))
        && object.pointer("/spec/methods") == Some(&json!(["WRITE", "FLUSH", "FSYNC"]))
        && object.pointer("/spec/errno") == Some(&json!(5))
        && object.pointer("/spec/percent") == Some(&json!(100));
    if !exact_fault {
        return Err("IOChaos did not preserve the exact Raft-log fault boundary".to_owned());
    }

    if !iochaos_condition_is_true(object, "Selected")
        || !iochaos_condition_is_true(object, "AllInjected")
    {
        return Err(
            "IOChaos controller has not reported Selected=True and AllInjected=True".to_owned(),
        );
    }

    // Chaos Mesh identifies an IOChaos instance at container granularity even
    // though the selector is intentionally exact at pod granularity.
    let instance_id = format!("{}/{}/{}", target.namespace, target.pod, SERVER_CONTAINER);
    let instances = object
        .pointer("/status/instances")
        .and_then(Value::as_object)
        .ok_or("IOChaos status has no selected instances")?;
    if instances.len() != 1 || !instances.contains_key(&instance_id) {
        return Err(format!(
            "IOChaos selected instances are not exactly {instance_id}: {:?}",
            instances.keys().collect::<Vec<_>>()
        ));
    }

    let records = object
        .pointer("/status/experiment/containerRecords")
        .and_then(Value::as_array)
        .ok_or("IOChaos status has no container records")?;
    if records.len() != 1
        || records[0].get("id").and_then(Value::as_str) != Some(instance_id.as_str())
        || records[0].get("phase").and_then(Value::as_str) != Some("Injected")
    {
        return Err(format!(
            "IOChaos container record is not one injected {instance_id}: {records:?}"
        ));
    }

    Ok(IoChaosReceipt {
        chaos_uid: chaos_uid.to_owned(),
        target: target.clone(),
    })
}

fn slow_disk_plan_for_capability(
    crd_present: bool,
    required: bool,
) -> Result<ChaosInjection, String> {
    if crd_present {
        Ok(ChaosInjection::Applied("chaos-mesh IOChaos"))
    } else if required {
        Err(format!(
            "{REQUIRE_IOCHAOS_ENV}=1 requires the iochaos.chaos-mesh.org CRD; {IOCHAOS_SKIP}"
        ))
    } else {
        Ok(ChaosInjection::Skipped(IOCHAOS_SKIP.to_owned()))
    }
}

fn slow_disk_plan_for_crd_present(crd_present: bool) -> ChaosInjection {
    slow_disk_plan_for_capability(crd_present, iochaos_required())
        .unwrap_or_else(|error| panic!("{error}"))
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
                let uid = self
                    .pod_uid(ordinal)
                    .await
                    .unwrap_or_else(|error| panic!("pod crash target is invalid: {error}"));
                self.delete_pod_with_uid(ordinal, &uid).await;
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

    async fn scale_admin_status_from_probe(
        &self,
        probe: &str,
        target_ordinal: u32,
    ) -> Result<ScaleChaosAdminStatus, String> {
        self.admin_json_from_probe(probe, target_ordinal, "/admin/status")
            .await
    }

    async fn raft_compaction_from_probe(
        &self,
        probe: &str,
        target_ordinal: u32,
    ) -> Result<RaftCompactionObservation, String> {
        self.admin_json_from_probe(probe, target_ordinal, "/admin/raft/compaction")
            .await
    }

    async fn admin_json_from_probe<T: DeserializeOwned>(
        &self,
        probe: &str,
        target_ordinal: u32,
        path: &str,
    ) -> Result<T, String> {
        let target = pod_name(&self.cluster, target_ordinal);
        let url = format!(
            "http://{}.{}:9091{path}",
            target,
            headless_service_name(&self.cluster)
        );
        let output = Command::new("kubectl")
            .arg("-n")
            .arg(&self.namespace)
            .arg("exec")
            .arg(probe)
            .arg("--")
            .arg("wget")
            .arg("-qO-")
            .arg("-T")
            .arg("2")
            .arg("--header")
            .arg("x-hydracache-client-id: operator")
            .arg("--header")
            .arg("x-hydracache-tenant: system")
            .arg("--header")
            .arg("x-hydracache-admin: true")
            .arg(&url)
            .output()
            .map_err(|error| {
                format!("{KIND_ENV}=1 W11 scale-chaos lane requires kubectl: {error}")
            })?;
        if !output.status.success() {
            return Err(format!(
                "admin probe {path} for {target} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        serde_json::from_slice(&output.stdout).map_err(|error| {
            format!("admin probe {path} for {target} returned invalid JSON: {error}")
        })
    }

    async fn raft_node_observation_from_probe(
        &self,
        probe: &str,
        ordinal: u32,
    ) -> Result<RaftNodeObservation, String> {
        let status = self.scale_admin_status_from_probe(probe, ordinal).await?;
        let compaction = self.raft_compaction_from_probe(probe, ordinal).await?;
        Ok(RaftNodeObservation {
            ordinal,
            status,
            compaction,
        })
    }

    async fn wait_raft_nodes(
        &self,
        replicas: u32,
        expected_voters: u32,
        minimum_epoch: u64,
        minimum_applied: u64,
        excluded_ordinal: Option<u32>,
        stage: &'static str,
    ) -> Vec<RaftNodeObservation> {
        let sequence = SCALE_ADMIN_PROBE_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let probe = self
            .ensure_network_probe_pod(2_000_u32.saturating_add(sequence as u32))
            .await
            .unwrap_or_else(|error| panic!("{stage} admin probe could not start: {error}"));
        let expected_ordinals = (0..replicas)
            .filter(|ordinal| excluded_ordinal != Some(*ordinal))
            .collect::<BTreeSet<_>>();
        let expected_member_ids = (0..expected_voters)
            .map(|ordinal| pod_name(&self.cluster, ordinal))
            .collect::<BTreeSet<_>>();
        let expected_voter_ids = expected_member_ids
            .iter()
            .map(|node_id| stable_raft_node_id(node_id))
            .collect::<BTreeSet<_>>();
        let mut latest = Vec::new();

        for _ in 0..30 {
            let mut observations = Vec::new();
            let mut errors = Vec::new();
            let pod_identities = match self
                .current_ready_pod_identities(replicas, excluded_ordinal)
                .await
            {
                Ok(identities) => Some(identities),
                Err(error) => {
                    errors.push(error);
                    None
                }
            };
            for ordinal in 0..replicas {
                if excluded_ordinal == Some(ordinal) {
                    continue;
                }
                match self.raft_node_observation_from_probe(&probe, ordinal).await {
                    Ok(observation) => observations.push(observation),
                    Err(error) => errors.push(error),
                }
            }

            let all_converged = pod_identities.is_some()
                && raft_observations_converged(
                    &observations,
                    &expected_ordinals,
                    &expected_member_ids,
                    &expected_voter_ids,
                    expected_voters,
                    minimum_epoch,
                    minimum_applied,
                );
            if all_converged {
                match self
                    .current_ready_pod_identities(replicas, excluded_ordinal)
                    .await
                {
                    Ok(after) if Some(&after) == pod_identities.as_ref() => {
                        self.delete_network_probe_pod(&probe).await;
                        return observations;
                    }
                    Ok(after) => errors.push(format!(
                        "current pod identities changed during {stage}: before={pod_identities:?} after={after:?}"
                    )),
                    Err(error) => errors.push(error),
                }
            }

            latest = observations
                .iter()
                .map(|observation| format!("{observation:?}"))
                .chain(errors)
                .collect();
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        self.delete_network_probe_pod(&probe).await;
        panic!(
            "timed out waiting for {stage}: expected {} current live nodes with members/voters={expected_voters}, epoch>={minimum_epoch}, applied>={minimum_applied}; latest={latest:?}",
            expected_ordinals.len()
        );
    }

    async fn current_ready_pod_identities(
        &self,
        replicas: u32,
        excluded_ordinal: Option<u32>,
    ) -> Result<Vec<CurrentPodIdentity>, String> {
        let stateful_sets: Api<StatefulSet> = Api::namespaced(self.client.clone(), &self.namespace);
        let stateful_set = stateful_sets
            .get(&self.cluster)
            .await
            .map_err(|error| format!("could not read StatefulSet {}: {error}", self.cluster))?;
        let desired = stateful_set
            .spec
            .as_ref()
            .and_then(|spec| spec.replicas)
            .unwrap_or_default()
            .max(0) as u32;
        if desired != replicas {
            return Err(format!(
                "StatefulSet {} still desires {desired} replicas, expected {replicas}",
                self.cluster
            ));
        }
        let revision = stateful_set
            .status
            .as_ref()
            .and_then(|status| {
                status
                    .update_revision
                    .as_deref()
                    .or(status.current_revision.as_deref())
            })
            .filter(|revision| !revision.is_empty())
            .ok_or_else(|| format!("StatefulSet {} has no current revision", self.cluster))?;
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let listed = pods
            .list(&ListParams::default().labels(&server_pod_selector(&self.cluster)))
            .await
            .map_err(|error| format!("could not list current server pods: {error}"))?;
        exact_current_pod_identities(
            &self.cluster,
            replicas,
            revision,
            excluded_ordinal,
            &listed.items,
        )
    }

    async fn pod_uid(&self, ordinal: u32) -> Result<String, String> {
        let pod = pod_name(&self.cluster, ordinal);
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        pods.get(&pod)
            .await
            .map_err(|error| format!("could not read target pod {pod}: {error}"))?
            .metadata
            .uid
            .filter(|uid| !uid.is_empty())
            .ok_or_else(|| format!("target pod {pod} has no Kubernetes UID"))
    }

    /// Return a pod identity that remained unchanged across two API reads.
    ///
    /// StatefulSet replacement is asynchronous: a name can briefly resolve to
    /// the terminating pod while the controller is already creating its
    /// replacement. Chaos Mesh selectors are name-based, so accepting that
    /// transient identity can inject the old UID and make a valid receipt
    /// impossible. The double-read is a small, deterministic stability gate.
    async fn stable_pod_uid(&self, ordinal: u32) -> Result<String, String> {
        let first = self.pod_uid(ordinal).await?;
        tokio::time::sleep(Duration::from_millis(250)).await;
        let second = self.pod_uid(ordinal).await?;
        if first == second {
            Ok(second)
        } else {
            Err(format!(
                "target pod {} changed UID while preparing IOChaos: {first} -> {second}",
                pod_name(&self.cluster, ordinal)
            ))
        }
    }

    async fn delete_pod_with_uid(&self, ordinal: u32, expected_uid: &str) {
        let name = pod_name(&self.cluster, ordinal);
        let current_uid = self
            .pod_uid(ordinal)
            .await
            .unwrap_or_else(|error| panic!("pod crash target is invalid: {error}"));
        assert_eq!(
            current_uid, expected_uid,
            "pod {name} changed UID before the crash request"
        );
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        pods.delete(&name, &pod_crash_delete_params(expected_uid))
            .await
            .unwrap_or_else(|error| {
                panic!("Kubernetes rejected UID-preconditioned pod crash for {name}: {error}")
            });
    }

    async fn wait_for_replacement_pod_uid(&self, ordinal: u32, previous_uid: &str) -> String {
        let name = pod_name(&self.cluster, ordinal);
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let mut latest = None;
        for _ in 0..KIND_WAIT_ATTEMPTS {
            match pods.get(&name).await {
                Ok(pod)
                    if pod.metadata.deletion_timestamp.is_none()
                        && pod_is_ready(&pod)
                        && pod
                            .metadata
                            .uid
                            .as_deref()
                            .is_some_and(|uid| uid != previous_uid) =>
                {
                    return pod.metadata.uid.expect("guarded as a replacement UID");
                }
                Ok(pod) => {
                    latest = Some(format!(
                        "uid={:?} deleting={} ready={}",
                        pod.metadata.uid,
                        pod.metadata.deletion_timestamp.is_some(),
                        pod_is_ready(&pod)
                    ));
                }
                Err(kube::Error::Api(error)) if error.code == 404 => {
                    latest = Some("replacement pod not created yet".to_owned());
                }
                Err(error) => panic!("could not observe replacement pod {name}: {error}"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!(
            "timed out waiting for replacement UID for {name}; previous={previous_uid} latest={latest:?}"
        );
    }

    async fn assert_faulted_node_lags_or_is_unavailable(
        &self,
        ordinal: u32,
        expected_pod_uid: &str,
        baseline_epoch: u64,
        baseline_applied: u64,
    ) {
        assert_eq!(
            self.pod_uid(ordinal)
                .await
                .unwrap_or_else(|error| panic!("W5 target identity disappeared: {error}")),
            expected_pod_uid,
            "IOChaos target pod was replaced before the mutation observation"
        );
        let sequence = SCALE_ADMIN_PROBE_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let probe = self
            .ensure_network_probe_pod(3_000_u32.saturating_add(sequence as u32))
            .await
            .unwrap_or_else(|error| panic!("W5 faulted-node probe could not start: {error}"));
        let status = self.scale_admin_status_from_probe(&probe, ordinal).await;
        let compaction = if status.is_ok() {
            Some(self.raft_compaction_from_probe(&probe, ordinal).await)
        } else {
            None
        };
        self.delete_network_probe_pod(&probe).await;

        assert_eq!(
            self.pod_uid(ordinal)
                .await
                .unwrap_or_else(|error| panic!("W5 target identity disappeared: {error}")),
            expected_pod_uid,
            "IOChaos target pod was replaced during the mutation observation"
        );
        let Ok(status) = status else {
            eprintln!(
                "HC-W5-IOCHAOS target={} uid={expected_pod_uid} admin status unavailable while Raft-log writes are faulted",
                pod_name(&self.cluster, ordinal),
            );
            return;
        };
        assert_eq!(
            status.source, "live",
            "faulted node returned a modeled status"
        );
        assert_eq!(
            (status.epoch, status.members, status.voters),
            (baseline_epoch, 3, 3),
            "faulted node partially exposed the 3->4 membership mutation: {status:?}"
        );
        match compaction.expect("status success always triggers compaction observation") {
            Ok(compaction) => assert!(
                compaction
                    .applied_index
                    .is_some_and(|index| index <= baseline_applied),
                "faulted node advanced durable applied_index through the IOChaos boundary: {compaction:?}"
            ),
            Err(error) => eprintln!(
                "HC-W5-IOCHAOS target={} retained baseline admin authority; durable progress endpoint unavailable under fault: {error}",
                pod_name(&self.cluster, ordinal)
            ),
        }
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
        match self.inject_slow_disk_receipt(ordinal).await {
            Some(receipt) => {
                eprintln!(
                    "HC-W5-IOCHAOS injected uid={} target={}/{} pod_uid={} container={SERVER_CONTAINER}",
                    receipt.chaos_uid,
                    receipt.target.namespace,
                    receipt.target.pod,
                    receipt.target.pod_uid
                );
                ChaosInjection::Applied("chaos-mesh IOChaos")
            }
            None => slow_disk_plan_for_crd_present(false),
        }
    }

    async fn inject_slow_disk_receipt(&self, ordinal: u32) -> Option<IoChaosReceipt> {
        let Some(api_resource) = self.iochaos_api_resource().await else {
            let _ = slow_disk_plan_for_crd_present(false);
            return None;
        };
        let iochaos: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), &self.namespace, &api_resource);
        let name = slow_disk_chaos_name(&self.cluster, ordinal);
        self.delete_dynamic_object_and_wait(&iochaos, &name, "stale IOChaos cleanup")
            .await;

        let target = IoChaosTarget {
            namespace: self.namespace.clone(),
            pod: pod_name(&self.cluster, ordinal),
            pod_uid: self
                .stable_pod_uid(ordinal)
                .await
                .unwrap_or_else(|error| panic!("kind slow-disk target is invalid: {error}")),
            ordinal,
        };
        let manifest = iochaos_manifest(&self.cluster, &self.namespace, ordinal);
        iochaos
            .patch(
                &name,
                &PatchParams::apply(FIELD_MANAGER).force(),
                &Patch::Apply(&manifest),
            )
            .await
            .expect("kind slow-disk injector should apply chaos-mesh IOChaos");
        Some(self.wait_iochaos_injected(&iochaos, &name, &target).await)
    }

    async fn wait_iochaos_injected(
        &self,
        iochaos: &Api<DynamicObject>,
        name: &str,
        target: &IoChaosTarget,
    ) -> IoChaosReceipt {
        let mut latest = None;
        for _ in 0..30 {
            match iochaos.get(name).await {
                Ok(object) => {
                    let value = serde_json::to_value(&object)
                        .expect("DynamicObject should serialize as Kubernetes JSON");
                    let current_pod_uid = self
                        .pod_uid(target.ordinal)
                        .await
                        .unwrap_or_else(|error| panic!("IOChaos target disappeared: {error}"));
                    match iochaos_injection_receipt(&value, target, &current_pod_uid) {
                        Ok(receipt) => return receipt,
                        Err(error) => latest = Some(error),
                    }
                }
                Err(kube::Error::Api(error)) if error.code == 404 => {
                    latest = Some("IOChaos object is not visible yet".to_owned());
                }
                Err(error) => panic!("could not observe IOChaos {name}: {error}"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!("IOChaos {name} never reached exact AllInjected state: latest={latest:?}");
    }

    async fn delete_slow_disk(&self, ordinal: u32) {
        let Some(api_resource) = self.iochaos_api_resource().await else {
            return;
        };
        let iochaos: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), &self.namespace, &api_resource);
        let name = slow_disk_chaos_name(&self.cluster, ordinal);
        let _ = iochaos.delete(&name, &DeleteParams::default()).await;
        let mut latest = None;
        for _ in 0..30 {
            match iochaos.get(&name).await {
                Ok(object) => {
                    let value = serde_json::to_value(&object)
                        .expect("DynamicObject should serialize as Kubernetes JSON");
                    latest = Some(if iochaos_condition_is_true(&value, "AllRecovered") {
                        "AllRecovered=True; waiting for deletion".to_owned()
                    } else {
                        "waiting for AllRecovered/deletion".to_owned()
                    });
                }
                Err(kube::Error::Api(error)) if error.code == 404 => return,
                Err(error) => panic!("could not observe IOChaos recovery for {name}: {error}"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!("IOChaos {name} did not recover/delete: latest={latest:?}");
    }

    async fn delete_dynamic_object_and_wait(
        &self,
        api: &Api<DynamicObject>,
        name: &str,
        stage: &str,
    ) {
        match api.delete(name, &DeleteParams::default()).await {
            Ok(_) => {}
            Err(kube::Error::Api(error)) if error.code == 404 => return,
            Err(error) => panic!("{stage} could not delete {name}: {error}"),
        }
        for _ in 0..30 {
            match api.get(name).await {
                Err(kube::Error::Api(error)) if error.code == 404 => return,
                Ok(_) => tokio::time::sleep(Duration::from_secs(1)).await,
                Err(error) => panic!("{stage} could not observe {name}: {error}"),
            }
        }
        panic!("{stage} timed out waiting for {name} deletion");
    }

    async fn iochaos_api_resource(&self) -> Option<ApiResource> {
        let gvk = GroupVersionKind::gvk("chaos-mesh.org", "v1alpha1", "IOChaos");
        discovery::pinned_kind(&self.client, &gvk)
            .await
            .ok()
            .map(|(resource, _)| resource)
    }

    async fn wait_ready(&self, desired: u32, stage: &'static str) -> SoakObservation {
        let mut latest = None;
        for _ in 0..KIND_WAIT_ATTEMPTS {
            let observation = match self.observe(stage).await {
                Ok(observation) => observation,
                Err(kube::Error::Api(error)) if error.code == 404 => {
                    latest = Some(format!("waiting for owned resources: {}", error.message));
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
                Err(error) => panic!("kind soak should observe cluster resources: {error}"),
            };
            if observation.desired_replicas == desired
                && observation.ready_replicas >= desired
                && observation.leader.is_some()
            {
                observation.assert_quorum();
                return observation;
            }
            latest = Some(format!("{observation:?}"));
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        panic!("timed out waiting for {stage} readiness; latest={latest:?}");
    }

    async fn observe(&self, stage: &'static str) -> kube::Result<SoakObservation> {
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

#[test]
fn replica_churn_under_partition_keeps_voters_and_committed_metadata() {
    let mut model = ScaleChaosModel::with_replicas(3);
    model.commit_metadata("metadata-before-partition");
    model.partition(2);
    model.commit_metadata("metadata-under-three-voter-partition");

    let scale_up_target = scale_target("scale-chaos", 4);
    let scale_up = plan_scale(&scale_up_target, &scale_observation(3, 2, 3, 3));
    assert_eq!(scale_up.effective_replicas, 4);
    assert_eq!(scale_up.conditions[0].reason, "ScaleUpCreatingPods");
    assert!(scale_up.admin_actions.is_empty());

    model.add_voter(3);
    model.commit_metadata("metadata-after-scale-up");
    assert_eq!(model.voters, BTreeSet::from([0, 1, 2, 3]));

    let scale_down_target = scale_target("scale-chaos", 3);
    let scale_down = plan_scale(&scale_down_target, &scale_observation(4, 4, 4, 4));
    assert_eq!(scale_down.effective_replicas, 4);
    assert_eq!(scale_down.conditions[0].reason, "DrainBeforeRemove");
    assert_eq!(
        scale_down.admin_actions,
        vec![
            AdminAction::Reshard { ordinal: 3 },
            AdminAction::Drain { ordinal: 3 }
        ]
    );

    model.drain(3);
    model.commit_metadata("metadata-after-scale-down");
    let mut drain_committed = scale_observation(4, 3, 3, 3);
    drain_committed.drain_requested_for = Some("scale-chaos-3".to_owned());
    let completed = plan_scale(&scale_down_target, &drain_committed);
    assert_eq!(completed.effective_replicas, 3);
    assert_eq!(completed.conditions[0].reason, "DrainComplete");

    model.heal(2);
    assert_eq!(model.voters, BTreeSet::from([0, 1, 2]));
    assert_eq!(model.committed_metadata.len(), 4);
    model.assert_all_authoritative_voters_caught_up();
}

#[test]
fn drained_pod_leaves_voters_but_crashed_pod_does_not_implicitly_shrink() {
    let mut model = ScaleChaosModel::with_replicas(3);
    model.commit_metadata("metadata-before-crash");
    model.crash(2);

    let steady_target = scale_target("scale-chaos", 3);
    let crash_observed = plan_scale(&steady_target, &scale_observation(3, 2, 3, 3));
    assert_eq!(crash_observed.effective_replicas, 3);
    assert_eq!(
        crash_observed.conditions[0].reason,
        "WaitingForReadyReplicas"
    );
    assert!(crash_observed.admin_actions.is_empty());
    assert_eq!(
        model.voters,
        BTreeSet::from([0, 1, 2]),
        "process crash must not be interpreted as committed voter removal"
    );

    model.restart(2);
    model.commit_metadata("metadata-after-restart");
    let scale_down_target = scale_target("scale-chaos", 2);
    let drain = plan_scale(&scale_down_target, &scale_observation(3, 3, 3, 3));
    assert_eq!(drain.conditions[0].reason, "DrainBeforeRemove");
    assert_eq!(
        drain.admin_actions,
        vec![
            AdminAction::Reshard { ordinal: 2 },
            AdminAction::Drain { ordinal: 2 }
        ]
    );

    model.drain(2);
    let mut committed = scale_observation(3, 2, 2, 2);
    committed.drain_requested_for = Some("scale-chaos-2".to_owned());
    let completed = plan_scale(&scale_down_target, &committed);
    assert_eq!(completed.effective_replicas, 2);
    assert_eq!(completed.conditions[0].reason, "DrainComplete");
    assert_eq!(model.voters, BTreeSet::from([0, 1]));
    model.assert_all_authoritative_voters_caught_up();
}

#[test]
fn canary_scale_chaos_accepts_a_ghost_voter() {
    let mut model = ScaleChaosModel::with_replicas(3);
    if std::env::var("HYDRACACHE_CANARY_DEFECT").as_deref() == Ok("W11") {
        model.running.remove(&2);
    } else {
        model.drain(2);
    }

    assert!(
        !model.voters.contains(&2),
        "HC-CANARY-RED:W11 drained pod remained as a ghost voter"
    );
}

#[test]
fn scale_chaos_capability_rejects_a_non_enforcing_cni() {
    let error = require_scale_partition_capability(&ChaosInjection::Skipped(
        NETWORK_POLICY_SKIP.to_owned(),
    ))
    .unwrap_err();
    assert!(error.contains(KIND_ENV));
    assert!(error.contains("enforcing CNI"));
    assert!(error.contains("NetworkPolicy"));
}

#[test]
fn operator_release_evidence_rejects_empty_kubectl_output() {
    let empty_logs = validate_kubectl_stdout(
        OPERATOR_POD_LOG_ARTIFACT,
        " \n",
        KubectlStdoutRequirement::NonEmpty,
    )
    .unwrap_err();
    assert!(empty_logs.contains("empty stdout"));

    let no_resources = validate_kubectl_stdout(
        OPERATOR_POD_LOG_ARTIFACT,
        "No resources found in default namespace.",
        KubectlStdoutRequirement::NonEmpty,
    )
    .unwrap_err();
    assert!(no_resources.contains("selected no resources"));

    let missing_cluster = validate_kubectl_stdout(
        OPERATOR_RESOURCES_ARTIFACT,
        "NAMESPACE NAME READY",
        KubectlStdoutRequirement::Contains("hydracache-066"),
    )
    .unwrap_err();
    assert!(missing_cluster.contains("hydracache-066"));
}

#[test]
fn operator_release_evidence_requires_current_controller_runtime_output() {
    let nonce = "release-066-test-nonce";
    let stale = validate_controller_runtime_log(
        "HC-OPERATOR-CONTROLLER-START nonce=older-run binary=target/debug/hydracache-operator\n\
         HC-OPERATOR-CONTROLLER-RUNTIME nonce=older-run identity=old namespace=default\n",
        nonce,
    )
    .unwrap_err();
    assert!(stale.contains("current-run start marker"));

    let marker_only = validate_controller_runtime_log(
        "HC-OPERATOR-CONTROLLER-START nonce=release-066-test-nonce binary=target/debug/hydracache-operator\n",
        nonce,
    )
    .unwrap_err();
    assert!(marker_only.contains("no controller runtime output"));

    validate_controller_runtime_log(
        "HC-OPERATOR-CONTROLLER-START nonce=release-066-test-nonce binary=target/debug/hydracache-operator\n\
         HC-OPERATOR-CONTROLLER-RUNTIME nonce=release-066-test-nonce identity=current namespace=default\n",
        nonce,
    )
    .unwrap();
}

#[tokio::test]
#[ignore = "kind/Chaos-Mesh-gated W5 lane: set HYDRACACHE_OPERATOR_KIND=1 with IOChaos installed"]
async fn iochaos_fault_blocks_real_raft_persistence_then_recovers() {
    let _proof = LIVE_KIND_PROOF_LOCK.lock().await;
    let Some(kind) =
        KindHarness::try_start("iochaos_fault_blocks_real_raft_persistence_then_recovers").await
    else {
        return;
    };

    let installed = kind.apply_cluster(soak_kind_spec(3)).await;
    kind.wait_ready(installed.spec.replicas, "w5-install").await;
    let initial = kind.wait_raft_nodes(3, 3, 1, 1, None, "w5-install").await;
    let initial_epoch = initial[0].status.epoch;
    let initial_applied = initial
        .iter()
        .map(RaftNodeObservation::applied_index)
        .max()
        .expect("three-node cluster must expose applied progress");
    let leader = initial[0]
        .status
        .leader
        .as_deref()
        .expect("converged initial cluster must report a leader");
    let target = initial
        .iter()
        .find(|observation| {
            observation.ordinal != 0 && pod_name(&kind.cluster, observation.ordinal) != leader
        })
        .expect("one of ordinals 1 or 2 must be a non-leader");
    let target_ordinal = target.ordinal;
    let target_baseline_applied = target.applied_index();

    let Some(receipt) = kind.inject_slow_disk_receipt(target_ordinal).await else {
        eprintln!("skipping W5 live IOChaos proof: {IOCHAOS_SKIP}");
        return;
    };
    eprintln!(
        "HC-W5-CAPABILITY runtime=kubernetes iochaos=AllInjected uid={} target={}/{} pod_uid={} container={SERVER_CONTAINER}",
        receipt.chaos_uid,
        receipt.target.namespace,
        receipt.target.pod,
        receipt.target.pod_uid
    );
    if iochaos_required() {
        write_operator_capability_artifact(
            OPERATOR_W5_CAPABILITY_ARTIFACT,
            &format!(
                "release=0.66.0\nproof=W5\nruntime=kubernetes\niochaos=AllInjected\nuid={}\ntarget={}/{}\npod_uid={}\ncontainer={SERVER_CONTAINER}\nreceipt_marker=HC-W5-CAPABILITY\n",
                receipt.chaos_uid,
                receipt.target.namespace,
                receipt.target.pod,
                receipt.target.pod_uid
            ),
        )
        .unwrap_or_else(|error| panic!("could not record W5 capability evidence: {error}"));
    }
    let proof = AssertUnwindSafe(async {
        let injected_majority = kind
            .wait_raft_nodes(
                3,
                3,
                initial_epoch,
                1,
                Some(target_ordinal),
                "w5-injected-non-leader",
            )
            .await;
        assert_ne!(
            injected_majority[0].status.leader.as_deref(),
            Some(receipt.target.pod.as_str()),
            "IOChaos target must still be a non-leader before the real mutation starts"
        );

        kind.apply_cluster(soak_kind_spec(4)).await;
        let committed_majority = kind
            .wait_raft_nodes(
                4,
                4,
                initial_epoch.saturating_add(1),
                initial_applied.saturating_add(1),
                Some(target_ordinal),
                "w5-faulted-scale-up-majority",
            )
            .await;
        let committed_epoch = committed_majority[0].status.epoch;
        let committed_applied = committed_majority
            .iter()
            .map(RaftNodeObservation::applied_index)
            .max()
            .expect("healthy majority must expose applied progress");
        kind.assert_faulted_node_lags_or_is_unavailable(
            target_ordinal,
            &receipt.target.pod_uid,
            initial_epoch,
            target_baseline_applied,
        )
        .await;

        kind.delete_slow_disk(target_ordinal).await;
        // An EIO fault may leave the embedded Sled process unable to resume
        // writes safely in place. The supported operational recovery is to
        // remove the fault, replace that exact pod, and prove the durable Raft
        // state catches up from the healthy majority.
        kind.delete_pod_with_uid(target_ordinal, &receipt.target.pod_uid)
            .await;
        let recovered_uid = kind
            .wait_for_replacement_pod_uid(target_ordinal, &receipt.target.pod_uid)
            .await;
        kind.wait_ready(4, "w5-iochaos-recovered").await;
        assert_ne!(
            recovered_uid, receipt.target.pod_uid,
            "IOChaos recovery must observe a newly created target pod"
        );
        let expected_recovered_epoch = committed_epoch.saturating_add(1);
        let recovered = kind
            .wait_raft_nodes(
                4,
                4,
                expected_recovered_epoch,
                committed_applied,
                None,
                "w5-iochaos-recovered",
            )
            .await;
        assert_eq!(
            recovered[0].status.epoch, expected_recovered_epoch,
            "replacing the IOChaos target must fence its old process generation with exactly one epoch"
        );
        let recovered_epoch = recovered[0].status.epoch;
        let recovered_target = recovered
            .iter()
            .find(|observation| observation.ordinal == target_ordinal)
            .expect("the healed target must be part of the exact four-pod observation");
        assert!(recovered_target.applied_index() >= committed_applied);

        kind.apply_cluster(soak_kind_spec(3)).await;
        kind.wait_ready(3, "w5-scale-down-restored").await;
        let restored = kind
            .wait_raft_nodes(
                3,
                3,
                recovered_epoch.saturating_add(1),
                committed_applied.saturating_add(1),
                None,
                "w5-scale-down-restored",
            )
            .await;
        assert_eq!(restored.len(), 3);
    })
    .catch_unwind()
    .await;

    let proof_failed = proof.is_err();
    let heal_cleanup = AssertUnwindSafe(async {
        kind.delete_slow_disk(target_ordinal).await;
        // A failed proof must not leak an EIO-poisoned pod into the next
        // serialized Kind test.
        if proof_failed {
            if let Ok(uid) = kind.pod_uid(target_ordinal).await {
                if uid == receipt.target.pod_uid {
                    kind.delete_pod_with_uid(target_ordinal, &uid).await;
                }
            }
            kind.wait_for_replacement_pod_uid(target_ordinal, &receipt.target.pod_uid)
                .await;
        }
    })
    .catch_unwind()
    .await;
    let scale_cleanup = AssertUnwindSafe(async {
        kind.apply_cluster(soak_kind_spec(3)).await;
        kind.wait_ready(3, "w5-final-cleanup").await;
    })
    .catch_unwind()
    .await;
    let mut failure = proof.err();
    for cleanup_failure in [heal_cleanup.err(), scale_cleanup.err()]
        .into_iter()
        .flatten()
    {
        if failure.is_none() {
            failure = Some(cleanup_failure);
        } else {
            eprintln!("W5 cleanup also panicked after the primary failure");
        }
    }
    if let Some(failure) = failure {
        resume_unwind(failure);
    }
}

#[tokio::test]
#[ignore = "kind/CNI-gated W11 lane: set HYDRACACHE_OPERATOR_KIND=1 with a NetworkPolicy-enforcing CNI"]
async fn operator_scale_chaos_kind_lane_records_voters_and_metadata_epoch() {
    let _proof = LIVE_KIND_PROOF_LOCK.lock().await;
    let Some(kind) =
        KindHarness::try_start("operator_scale_chaos_kind_lane_records_voters_and_metadata_epoch")
            .await
    else {
        return;
    };

    let installed = kind.apply_cluster(soak_kind_spec(3)).await;
    kind.wait_ready(installed.spec.replicas, "w11-install")
        .await;
    let initial = kind.wait_raft_nodes(3, 3, 1, 1, None, "w11-install").await;
    let initial_epoch = initial[0].status.epoch;

    let fault = ChaosFault::NetworkPartition { ordinal: 2 };
    let injection = kind.inject(fault).await;
    let injector =
        require_scale_partition_capability(&injection).unwrap_or_else(|error| panic!("{error}"));
    eprintln!(
        "HC-W11-CAPABILITY runtime=kubernetes cni_network_policy=enforced injector={injector}"
    );
    if iochaos_required() {
        write_operator_capability_artifact(
            OPERATOR_W11_CAPABILITY_ARTIFACT,
            &format!(
                "release=0.66.0\nproof=W11\nruntime=kubernetes\nnamespace={}\ncluster={}\ncni_network_policy=enforced\ninjector={injector}\nreceipt_marker=HC-W11-CAPABILITY\n",
                kind.namespace, kind.cluster
            ),
        )
        .unwrap_or_else(|error| panic!("could not record W11 capability evidence: {error}"));
    }

    let proof = AssertUnwindSafe(async {
        let scaled_up = kind.apply_cluster(soak_kind_spec(4)).await;
        kind.wait_ready(scaled_up.spec.replicas, "w11-scale-up-partitioned")
            .await;
        let after_scale_up = kind
            .wait_raft_nodes(
                4,
                4,
                initial_epoch.saturating_add(1),
                1,
                Some(2),
                "w11-scale-up-partitioned",
            )
            .await;
        let after_scale_up_epoch = after_scale_up[0].status.epoch;

        let scaled_down = kind.apply_cluster(soak_kind_spec(3)).await;
        kind.wait_ready(scaled_down.spec.replicas, "w11-scale-down-partitioned")
            .await;
        let after_scale_down = kind
            .wait_raft_nodes(
                3,
                3,
                after_scale_up_epoch.saturating_add(1),
                1,
                Some(2),
                "w11-scale-down-partitioned",
            )
            .await;
        let after_scale_down_epoch = after_scale_down[0].status.epoch;
        let after_scale_down_applied = after_scale_down
            .iter()
            .map(RaftNodeObservation::applied_index)
            .max()
            .expect("partition majority must expose applied progress");

        kind.heal(fault, &injection).await;
        kind.wait_ready(3, "w11-partition-healed").await;
        let healed = kind
            .wait_raft_nodes(
                3,
                3,
                after_scale_down_epoch,
                after_scale_down_applied,
                None,
                "w11-partition-healed",
            )
            .await;
        assert_eq!(
            healed[0].status.epoch, after_scale_down_epoch,
            "partition heal must catch up to the committed epoch without inventing membership"
        );
        let healed_applied = healed
            .iter()
            .map(RaftNodeObservation::applied_index)
            .max()
            .expect("healed cluster must expose applied progress");

        let crashed_uid = kind
            .pod_uid(2)
            .await
            .expect("pod-2 must have a UID before crash");
        kind.delete_pod_with_uid(2, &crashed_uid).await;
        let replacement_uid = kind.wait_for_replacement_pod_uid(2, &crashed_uid).await;
        assert_ne!(replacement_uid, crashed_uid);
        let recovered = kind.wait_ready(3, "w11-crash-recovered").await;
        recovered.assert_quorum();
        let expected_crash_epoch = after_scale_down_epoch.saturating_add(1);
        let after_crash = kind
            .wait_raft_nodes(
                3,
                3,
                expected_crash_epoch,
                healed_applied,
                None,
                "w11-crash-recovered",
            )
            .await;
        assert_eq!(
            after_crash[0].status.epoch, expected_crash_epoch,
            "pod replacement must fence its old process generation with exactly one epoch"
        );
    })
    .catch_unwind()
    .await;

    let heal_cleanup = AssertUnwindSafe(kind.heal(fault, &injection))
        .catch_unwind()
        .await;
    let scale_cleanup = AssertUnwindSafe(async {
        kind.apply_cluster(soak_kind_spec(3)).await;
        kind.wait_ready(3, "w11-final-cleanup").await;
    })
    .catch_unwind()
    .await;
    let mut failure = proof.err();
    for cleanup_failure in [heal_cleanup.err(), scale_cleanup.err()]
        .into_iter()
        .flatten()
    {
        if failure.is_none() {
            failure = Some(cleanup_failure);
        } else {
            eprintln!("W11 cleanup also panicked after the primary failure");
        }
    }
    if iochaos_required() {
        if let Err(error) = capture_operator_kind_release_evidence() {
            if failure.is_none() {
                panic!("operator-kind release evidence capture failed: {error}");
            }
            eprintln!(
                "operator-kind evidence capture also failed after the primary failure: {error}"
            );
        }
    }
    if let Some(failure) = failure {
        resume_unwind(failure);
    }
}

#[tokio::test]
#[ignore = "kind/nightly soak: set HYDRACACHE_OPERATOR_KIND=1"]
async fn multi_node_chaos_soak_preserves_quorum_and_leadership() {
    let _proof = LIVE_KIND_PROOF_LOCK.lock().await;
    let Some(kind) =
        KindHarness::try_start("multi_node_chaos_soak_preserves_quorum_and_leadership").await
    else {
        return;
    };
    eprintln!("{SCOPE_DISCLOSURE}");

    let cluster = kind.apply_cluster(soak_kind_spec(3)).await;
    let install = kind.wait_ready(cluster.spec.replicas, "install").await;
    install.assert_quorum();
    install.assert_leader();

    for fault in rolling_chaos_schedule() {
        let injection = kind.inject(fault).await;
        if let ChaosInjection::Skipped(reason) = &injection {
            eprintln!("skipping {fault:?}: {reason}");
        }
        let observed = kind.wait_ready(cluster.spec.replicas, "fault-window").await;
        observed.assert_quorum();
        observed.assert_leader();
        kind.heal(fault, &injection).await;
        let recovered = kind.wait_ready(cluster.spec.replicas, "recovered").await;
        recovered.assert_quorum();
        recovered.assert_leader();
    }
}

#[tokio::test]
#[ignore = "kind/nightly soak: set HYDRACACHE_OPERATOR_KIND=1"]
async fn leader_is_always_reestablished_after_pod_crash() {
    let _proof = LIVE_KIND_PROOF_LOCK.lock().await;
    let Some(kind) = KindHarness::try_start("leader_is_always_reestablished_after_pod_crash").await
    else {
        return;
    };
    eprintln!("{SCOPE_DISCLOSURE}");

    let cluster = kind.apply_cluster(soak_kind_spec(3)).await;
    let ready = kind.wait_ready(cluster.spec.replicas, READY_PHASE).await;
    ready.assert_leader();

    let injection = kind.inject(ChaosFault::PodCrash { ordinal: 0 }).await;
    let recovered = kind
        .wait_ready(cluster.spec.replicas, "pod-crash-recovered")
        .await;
    recovered.assert_quorum();
    recovered.assert_leader();
    kind.heal(ChaosFault::PodCrash { ordinal: 0 }, &injection)
        .await;
}

#[tokio::test]
#[ignore = "kind/calico-gated: set HYDRACACHE_OPERATOR_KIND=1 with a NetworkPolicy-enforcing CNI"]
async fn kind_partition_injection_isolates_and_heals() {
    let _proof = LIVE_KIND_PROOF_LOCK.lock().await;
    let Some(kind) = KindHarness::try_start("kind_partition_injection_isolates_and_heals").await
    else {
        return;
    };
    eprintln!("{SCOPE_DISCLOSURE}");

    let cluster = kind.apply_cluster(soak_kind_spec(3)).await;
    let ready = kind.wait_ready(cluster.spec.replicas, READY_PHASE).await;
    ready.assert_quorum();
    ready.assert_leader();

    let fault = ChaosFault::NetworkPartition { ordinal: 1 };
    let injection = kind.inject(fault).await;
    if let ChaosInjection::Skipped(reason) = &injection {
        eprintln!("skipping partition assertion: {reason}");
        return;
    }

    let observed = kind
        .wait_ready(cluster.spec.replicas, "partition-window")
        .await;
    observed.assert_quorum();
    observed.assert_leader();
    kind.heal(fault, &injection).await;

    let recovered = kind
        .wait_ready(cluster.spec.replicas, "partition-healed")
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
    kind.wait_ready(cluster.spec.replicas, READY_PHASE).await;
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
fn pod_crash_delete_is_uid_preconditioned() {
    let params = pod_crash_delete_params("pod-uid-2");
    let preconditions = params
        .preconditions
        .expect("pod crash deletion must carry preconditions");
    assert_eq!(preconditions.uid.as_deref(), Some("pod-uid-2"));
    assert!(preconditions.resource_version.is_none());
}

#[test]
fn raft_observation_requires_every_expected_current_pod() {
    let observation = |ordinal, epoch, voters| {
        let member_ids = (0..voters)
            .map(|member| format!("chaos-{member}"))
            .collect::<Vec<_>>();
        let voter_ids = member_ids
            .iter()
            .map(|member| stable_raft_node_id(member))
            .collect();
        RaftNodeObservation {
            ordinal,
            status: ScaleChaosAdminStatus {
                source: "live".to_owned(),
                leader: Some("chaos-0".to_owned()),
                epoch,
                quorum_ok: true,
                members: voters,
                member_ids,
                voters,
                voter_ids,
            },
            compaction: RaftCompactionObservation {
                available: true,
                applied_index: Some(12),
            },
        }
    };
    let expected = BTreeSet::from([0, 1, 2]);
    let expected_members = BTreeSet::from([
        "chaos-0".to_owned(),
        "chaos-1".to_owned(),
        "chaos-2".to_owned(),
    ]);
    let expected_voters = expected_members
        .iter()
        .map(|member| stable_raft_node_id(member))
        .collect::<BTreeSet<_>>();
    let mut observations = vec![
        observation(0, 7, 3),
        observation(1, 7, 3),
        observation(2, 7, 3),
    ];
    assert!(raft_observations_converged(
        &observations,
        &expected,
        &expected_members,
        &expected_voters,
        3,
        7,
        12
    ));

    observations[2].status.epoch = 6;
    assert!(
        !raft_observations_converged(
            &observations,
            &expected,
            &expected_members,
            &expected_voters,
            3,
            7,
            12
        ),
        "one good pod must not hide a stale current pod"
    );
    observations[2] = observation(2, 7, 3);
    observations[2].status.member_ids[2] = "chaos-3".to_owned();
    observations[2].status.voter_ids[2] = stable_raft_node_id("chaos-3");
    assert!(
        !raft_observations_converged(
            &observations,
            &expected,
            &expected_members,
            &expected_voters,
            3,
            7,
            12
        ),
        "matching counts must not hide a ghost voter or missing expected member"
    );
    observations.pop();
    assert!(
        !raft_observations_converged(
            &observations,
            &expected,
            &expected_members,
            &expected_voters,
            3,
            7,
            12
        ),
        "a partial response set must not pass"
    );
}

#[test]
fn current_pod_filter_requires_exact_uid_and_statefulset_revision() {
    let ready_pod = |ordinal: u32, uid: &str, revision: &str| Pod {
        metadata: ObjectMeta {
            name: Some(format!("chaos-{ordinal}")),
            uid: Some(uid.to_owned()),
            labels: Some(BTreeMap::from([(
                STATEFULSET_REVISION_LABEL.to_owned(),
                revision.to_owned(),
            )])),
            ..Default::default()
        },
        status: Some(PodStatus {
            conditions: Some(vec![PodCondition {
                status: "True".to_owned(),
                type_: "Ready".to_owned(),
                ..Default::default()
            }]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut pods = vec![
        ready_pod(0, "uid-0", "revision-a"),
        ready_pod(1, "uid-1", "revision-a"),
        ready_pod(2, "uid-2", "revision-a"),
    ];
    let identities = exact_current_pod_identities("chaos", 3, "revision-a", None, &pods)
        .expect("exact current pod set should be accepted");
    assert_eq!(identities[2].uid, "uid-2");

    pods[2] = ready_pod(2, "replacement-uid-2", "revision-b");
    assert!(
        exact_current_pod_identities("chaos", 3, "revision-a", None, &pods)
            .unwrap_err()
            .contains("expected revision-a"),
        "a Ready pod from a stale/different revision must not count"
    );
    pods[2] = ready_pod(2, "replacement-uid-2", "revision-a");
    let replacement = exact_current_pod_identities("chaos", 3, "revision-a", None, &pods)
        .expect("replacement at the current revision should be observed");
    assert_eq!(replacement[2].uid, "replacement-uid-2");
}

#[test]
fn slow_disk_uses_iochaos_only_when_crd_present() {
    assert_eq!(
        slow_disk_plan_for_capability(false, false).unwrap(),
        ChaosInjection::Skipped(IOCHAOS_SKIP.to_owned())
    );
    assert_eq!(
        slow_disk_plan_for_capability(true, false).unwrap(),
        ChaosInjection::Applied("chaos-mesh IOChaos")
    );

    let manifest = iochaos_manifest("chaos", "testing", 1);
    assert_eq!(manifest["kind"], "IOChaos");
    assert_eq!(manifest["metadata"]["name"], "chaos-slow-disk-1");
    assert_eq!(
        manifest["spec"]["selector"]["pods"]["testing"][0],
        "chaos-1"
    );
    assert_eq!(manifest["spec"]["action"], "fault");
    assert_eq!(manifest["spec"]["errno"], 5);
    assert_eq!(manifest["spec"]["percent"], 100);
    assert_eq!(manifest["spec"]["containerNames"][0], SERVER_CONTAINER);
    assert_eq!(
        manifest["spec"]["methods"],
        json!(["WRITE", "FLUSH", "FSYNC"])
    );
    assert_eq!(manifest["spec"]["volumePath"], "/var/lib/hydracache");
    assert_eq!(
        manifest["spec"]["path"],
        "/var/lib/hydracache/raft-log/**/*"
    );
    assert_eq!(manifest["spec"]["duration"], "10m");
}

#[test]
fn iochaos_receipt_requires_controller_injection_and_exact_target() {
    let target = IoChaosTarget {
        namespace: "testing".to_owned(),
        pod: "chaos-1".to_owned(),
        pod_uid: "pod-uid-1".to_owned(),
        ordinal: 1,
    };
    let mut object = iochaos_manifest("chaos", "testing", 1);
    object["metadata"]["uid"] = json!("iochaos-uid-1");
    object["status"] = json!({
        "conditions": [
            { "type": "Selected", "status": "True" },
            { "type": "AllInjected", "status": "True" }
        ],
        "experiment": {
            "containerRecords": [{
                "id": "testing/chaos-1/hydracache",
                "selectorKey": ".",
                "phase": "Injected"
            }]
        },
        "instances": { "testing/chaos-1/hydracache": 1 }
    });

    let receipt = iochaos_injection_receipt(&object, &target, "pod-uid-1")
        .expect("exact controller-confirmed target should produce a receipt");
    assert_eq!(receipt.chaos_uid, "iochaos-uid-1");

    let mut not_injected = object.clone();
    not_injected["status"]["conditions"][1]["status"] = json!("False");
    assert!(
        iochaos_injection_receipt(&not_injected, &target, "pod-uid-1")
            .unwrap_err()
            .contains("AllInjected")
    );

    let mut wrong_target = object.clone();
    wrong_target["spec"]["selector"]["pods"] = json!({ "testing": ["chaos-2"] });
    assert!(
        iochaos_injection_receipt(&wrong_target, &target, "pod-uid-1")
            .unwrap_err()
            .contains("exact target")
    );

    let mut read_only_fault = object.clone();
    read_only_fault["spec"]["methods"] = json!(["READ"]);
    assert!(
        iochaos_injection_receipt(&read_only_fault, &target, "pod-uid-1")
            .unwrap_err()
            .contains("Raft-log fault boundary")
    );

    assert!(
        iochaos_injection_receipt(&object, &target, "replacement-pod-uid")
            .unwrap_err()
            .contains("replaced during injection")
    );
}

#[test]
fn slow_disk_release_capability_is_fail_closed_when_required() {
    let error = slow_disk_plan_for_capability(false, true)
        .expect_err("the release-kind lane must reject a missing IOChaos CRD");
    assert!(error.contains(REQUIRE_IOCHAOS_ENV));
    assert!(error.contains("iochaos.chaos-mesh.org"));
    assert_eq!(
        slow_disk_plan_for_capability(true, true).unwrap(),
        ChaosInjection::Applied("chaos-mesh IOChaos")
    );
}
