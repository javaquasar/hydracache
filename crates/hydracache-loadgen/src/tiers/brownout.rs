//! Executable reference producers for the three honest W5 brownout surfaces.
//!
//! Every entry point receives explicit repository, predecessor, evidence-root,
//! and output paths. Reference reports are landed only after process cleanup,
//! with create-new atomic publication; stale evidence is never overwritten.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use hydracache_cache_sim::{KeyDistribution, KeyScheduleSpec, KEY_SCHEDULE_GENERATOR_VERSION};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::rate::{run_open_loop, OpenLoopConfig, OpenLoopObservation};
use crate::report::{
    PerfReport, RespDaemonConfigIdentity, RespEndpointCapability, WorkloadIdentity,
};
use crate::target::{Target, TargetError, TargetOutcome, TargetRequest};
use crate::targets::brownout::{
    run_control_plane_reference as derive_control_plane_reference, run_grid_model_reference,
    run_resp_reference as derive_resp_reference, BrownoutError, ControlPlaneActionReceipt,
    ControlPlaneBrownoutDriver, ControlPlaneBrownoutLoadPlan, ControlPlaneBrownoutReport,
    ControlPlaneBrownoutScenario, ControlPlaneExecutionCapability, ControlPlaneFinalCleanupReceipt,
    ControlPlanePredecessor, GridModelBrownoutReport, GridModelBrownoutScenario,
    GridModelPredecessor, IndependentRespRawWindow, ObservedProcessImage, RawControlPlaneEvent,
    RawRespEndpointEvent, RespBrownoutDriver, RespBrownoutLoadPlan, RespBrownoutReport,
    RespBrownoutScenario, RespCapacityPredecessor, RespExecutionCapability,
    RespSelectedCapacityContract, SocketUnavailableReceipt, WaitedProcessTermination,
};
use crate::targets::control_plane as w4a;
use crate::targets::grid_model as w4b;
use crate::targets::resp::{
    Resp2Limits, RespEndpointIdentity, RespOperationMix, RespTargetConfig, RespTcpTarget,
};
use crate::tiers::control_plane::{ControlPlaneReferenceError, LiveControlPlaneProcessHarness};
use crate::tiers::resp_reference::{
    RespDaemonEvidence, RespReferencePorts, ValidatedRespReferenceContext,
};

const MAX_INPUT_BYTES: u64 = 128 * 1024 * 1024;
const CONTROL_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const CONTROL_TRANSITION_POLL: Duration = Duration::from_millis(50);
const RESP_STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const RESP_POLL_INTERVAL: Duration = Duration::from_millis(25);
const RESP_PING_FRAME: &[u8] = b"*1\r\n$4\r\nPING\r\n";
const RESP_PONG_FRAME: &[u8] = b"+PONG\r\n";

static ARTIFACT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static RESP_PROCESS_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(Debug, thiserror::Error)]
pub enum BrownoutProducerError {
    #[error("W5 producer contract rejected: {0}")]
    Contract(String),
    #[error("W5 producer system operation failed: {0}")]
    System(String),
    #[error(transparent)]
    Evidence(#[from] BrownoutError),
    #[error("W5 control-plane harness failed: {0}")]
    ControlPlane(String),
}

impl From<ControlPlaneReferenceError> for BrownoutProducerError {
    fn from(error: ControlPlaneReferenceError) -> Self {
        Self::ControlPlane(error.to_string())
    }
}

/// Produce W5C from exact persisted W4B bytes. This is a real reference run of
/// the exported in-process replication primitive; it makes no daemon claim.
pub async fn produce_grid_model_reference(
    repo_root: &Path,
    context: &ValidatedRespReferenceContext,
    w5_scenario_path: &Path,
    w4_scenario_path: &Path,
    w4_report_path: &Path,
    report_path: &Path,
) -> Result<GridModelBrownoutReport, BrownoutProducerError> {
    validate_explicit_repo(repo_root, Some(context))?;
    require_absolute_paths(&[
        w5_scenario_path,
        w4_scenario_path,
        w4_report_path,
        report_path,
    ])?;
    let w5 = GridModelBrownoutScenario::load(w5_scenario_path)
        .map_err(|error| BrownoutProducerError::Contract(error.to_string()))?;
    let w4 = w4b::GridModelScenario::load(w4_scenario_path)
        .map_err(|error| BrownoutProducerError::Contract(error.to_string()))?;
    let (w4_report, w4_bytes) = read_typed::<w4b::GridModelReport>(w4_report_path)?;
    let predecessor = GridModelPredecessor::from_w4b_reference(&w4, &w4_report, &w4_bytes)?;
    let sequence = ARTIFACT_SEQUENCE
        .fetch_add(1, Ordering::SeqCst)
        .checked_add(1)
        .ok_or_else(|| {
            BrownoutProducerError::System("W5C execution sequence overflowed".to_owned())
        })?;
    let execution = predecessor.fresh_execution_receipt(context, sequence, unix_nanos_now()?)?;
    let report = run_grid_model_reference(&w5, predecessor, execution).await?;
    context.verify_binaries_unchanged().map_err(|error| {
        BrownoutProducerError::Contract(format!(
            "prebuilt binaries changed during W5C measurement: {error}"
        ))
    })?;
    write_new_json_atomic(report_path, &report)?;
    Ok(report)
}

/// Produce all four W5A events on one fresh shared W4 process harness. The
/// report is written only after every still-live original/restarted child has
/// been killed, waited, and post-run binary verification has succeeded.
#[allow(clippy::too_many_arguments)]
pub async fn produce_control_plane_reference(
    repo_root: &Path,
    context: &ValidatedRespReferenceContext,
    w5_scenario_path: &Path,
    w4_scenario_path: &Path,
    w4_report_path: &Path,
    evidence_root: &Path,
    report_path: &Path,
) -> Result<ControlPlaneBrownoutReport, BrownoutProducerError> {
    validate_explicit_repo(repo_root, Some(context))?;
    require_absolute_paths(&[
        w5_scenario_path,
        w4_scenario_path,
        w4_report_path,
        evidence_root,
        report_path,
    ])?;
    let w5 = ControlPlaneBrownoutScenario::load(w5_scenario_path)?;
    let w4 = w4a::ControlPlaneScenario::load(w4_scenario_path)
        .map_err(|error| BrownoutProducerError::Contract(error.to_string()))?;
    let (w4_report, w4_bytes) = read_typed::<w4a::ControlPlaneReport>(w4_report_path)?;
    let predecessor = ControlPlanePredecessor::from_w4a_reference(
        &w4,
        &w4_report,
        &w4_bytes,
        w5.load.fixed_rate_fraction_millionths,
        w5.reference.predecessor_node_count,
    )?;
    let harness = LiveControlPlaneProcessHarness::stage(
        context,
        &w4,
        w5.reference.predecessor_node_count,
        evidence_root,
    )
    .await?;
    let execution =
        ControlPlaneExecutionCapability::from_fresh_w4a(&w4, harness.capability().clone())?;
    let driver = LiveControlPlaneBrownoutDriver::new(harness);
    let report = match derive_control_plane_reference(&w5, predecessor, &execution, &driver).await {
        Ok(report) => report,
        Err(run) => match driver.shutdown().await {
            Ok(_) => return Err(run.into()),
            Err(cleanup) => {
                return Err(BrownoutProducerError::ControlPlane(format!(
                    "W5A run failed: {run}; fallback cleanup also failed: {cleanup}"
                )))
            }
        },
    };
    write_new_json_atomic(report_path, &report)?;
    Ok(report)
}

/// Produce W5B from exact W3 capacity/lifecycle artifacts on fresh direct
/// children. One selected process and all independent controls are reaped
/// before the create-new report publication.
#[allow(clippy::too_many_arguments)]
pub async fn produce_resp_reference(
    repo_root: &Path,
    context: &ValidatedRespReferenceContext,
    w5_scenario_path: &Path,
    w3_report_path: &Path,
    w3_lifecycle_path: &Path,
    evidence_root: &Path,
    report_path: &Path,
) -> Result<RespBrownoutReport, BrownoutProducerError> {
    validate_explicit_repo(repo_root, Some(context))?;
    require_absolute_paths(&[
        w5_scenario_path,
        w3_report_path,
        w3_lifecycle_path,
        evidence_root,
        report_path,
    ])?;
    let scenario = RespBrownoutScenario::load(w5_scenario_path)?;
    let (w3_report, w3_bytes) = read_typed::<PerfReport>(w3_report_path)?;
    let (w3_lifecycle, lifecycle_bytes) = read_typed::<RespDaemonEvidence>(w3_lifecycle_path)?;
    let predecessor = RespCapacityPredecessor::from_w3_reference(
        &w3_report,
        &w3_bytes,
        &w3_lifecycle,
        &lifecycle_bytes,
        scenario.load.fixed_rate_fraction_millionths,
    )?;
    let process_guard = RESP_PROCESS_LOCK.lock().await;
    let selected = LiveRespDaemon::start(context, 50_000, evidence_root, "selected").await?;
    let selected_execution = RespExecutionCapability::from_fresh_w3_launch(
        &predecessor,
        context,
        selected.capability().clone(),
        selected.process_image()?,
    )?;
    let mut controls =
        Vec::with_capacity(usize::from(scenario.event.independent_control_endpoints));
    let mut control_executions = Vec::with_capacity(controls.capacity());
    for index in 0..scenario.event.independent_control_endpoints {
        let daemon = LiveRespDaemon::start(
            context,
            50_001 + u32::from(index),
            evidence_root,
            &format!("control-{index}"),
        )
        .await?;
        control_executions.push(RespExecutionCapability::from_fresh_w3_launch(
            &predecessor,
            context,
            daemon.capability().clone(),
            daemon.process_image()?,
        )?);
        controls.push(daemon);
    }
    let driver = LiveRespBrownoutDriver::new(selected, controls, process_guard);
    let report = derive_resp_reference(
        &scenario,
        predecessor,
        selected_execution,
        control_executions,
        &driver,
    )
    .await?;
    if !driver.consumed().await {
        return Err(BrownoutProducerError::System(
            "W5B process driver did not consume and reap its process set".to_owned(),
        ));
    }
    write_new_json_atomic(report_path, &report)?;
    Ok(report)
}

fn validate_explicit_repo(
    repo_root: &Path,
    context: Option<&ValidatedRespReferenceContext>,
) -> Result<PathBuf, BrownoutProducerError> {
    if !repo_root.is_absolute() {
        return Err(BrownoutProducerError::Contract(format!(
            "repository root must be absolute: {}",
            repo_root.display()
        )));
    }
    let canonical = fs::canonicalize(repo_root).map_err(|error| {
        BrownoutProducerError::System(format!(
            "cannot canonicalize repository root {}: {error}",
            repo_root.display()
        ))
    })?;
    if let Some(context) = context {
        if canonical != context.repo_root {
            return Err(BrownoutProducerError::Contract(format!(
                "explicit repository root {} differs from validated context {}",
                canonical.display(),
                context.repo_root.display()
            )));
        }
        context.verify_binaries_unchanged().map_err(|error| {
            BrownoutProducerError::Contract(format!("prebuilt binaries changed: {error}"))
        })?;
    }
    Ok(canonical)
}

fn require_absolute_paths(paths: &[&Path]) -> Result<(), BrownoutProducerError> {
    if let Some(path) = paths.iter().find(|path| !path.is_absolute()) {
        return Err(BrownoutProducerError::Contract(format!(
            "W5 producer path must be absolute: {}",
            path.display()
        )));
    }
    Ok(())
}

fn read_typed<T: DeserializeOwned>(path: &Path) -> Result<(T, Vec<u8>), BrownoutProducerError> {
    let metadata = fs::metadata(path).map_err(|error| {
        BrownoutProducerError::System(format!("cannot stat {}: {error}", path.display()))
    })?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_INPUT_BYTES {
        return Err(BrownoutProducerError::Contract(format!(
            "typed predecessor {} must be a 1..={MAX_INPUT_BYTES}-byte regular file",
            path.display()
        )));
    }
    let bytes = fs::read(path).map_err(|error| {
        BrownoutProducerError::System(format!("cannot read {}: {error}", path.display()))
    })?;
    let value = serde_json::from_slice(&bytes).map_err(|error| {
        BrownoutProducerError::Contract(format!(
            "typed predecessor {} is invalid JSON: {error}",
            path.display()
        ))
    })?;
    Ok((value, bytes))
}

fn write_new_json_atomic<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), BrownoutProducerError> {
    if !path.is_absolute() {
        return Err(BrownoutProducerError::Contract(format!(
            "W5 report path must be absolute: {}",
            path.display()
        )));
    }
    if path.exists() {
        return Err(BrownoutProducerError::Contract(format!(
            "refusing to overwrite stale W5 evidence {}",
            path.display()
        )));
    }
    let parent = path.parent().ok_or_else(|| {
        BrownoutProducerError::Contract("W5 report path has no parent".to_owned())
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        BrownoutProducerError::System(format!(
            "cannot create W5 report parent {}: {error}",
            parent.display()
        ))
    })?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        BrownoutProducerError::System(format!("cannot serialize W5 report: {error}"))
    })?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            BrownoutProducerError::Contract("W5 report file name must be UTF-8".to_owned())
        })?;
    let sequence = ARTIFACT_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let temporary = parent.join(format!(
        ".{name}.{}.{}-atomic.tmp",
        std::process::id(),
        sequence
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                BrownoutProducerError::System(format!(
                    "cannot create temporary W5 report {}: {error}",
                    temporary.display()
                ))
            })?;
        file.write_all(&bytes).map_err(|error| {
            BrownoutProducerError::System(format!(
                "cannot write temporary W5 report {}: {error}",
                temporary.display()
            ))
        })?;
        file.sync_all().map_err(|error| {
            BrownoutProducerError::System(format!(
                "cannot sync temporary W5 report {}: {error}",
                temporary.display()
            ))
        })?;
        fs::rename(&temporary, path).map_err(|error| {
            BrownoutProducerError::System(format!(
                "cannot atomically land W5 report {}: {error}",
                path.display()
            ))
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Debug)]
struct LiveMetadataTarget {
    endpoint: w4a::ControlPlaneEndpoint,
    timeout: Duration,
}

impl LiveMetadataTarget {
    async fn snapshot(&self) -> Result<w4a::PublicControlPlaneSnapshot, w4a::ControlPlaneError> {
        w4a::probe_public_snapshot(self.endpoint.clone(), self.timeout).await
    }
}

#[async_trait]
impl Target for LiveMetadataTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        self.state_digest().await
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        let snapshot = self
            .snapshot()
            .await
            .map_err(|error| TargetError::Warmup(error.to_string()))?;
        let bytes = serde_json::to_vec(&snapshot)
            .map_err(|error| TargetError::Warmup(error.to_string()))?;
        Ok(format!("sha256:{}", sha256_hex(&bytes)))
    }

    async fn execute(&self, _request: TargetRequest) -> TargetOutcome {
        match self.snapshot().await {
            Ok(snapshot) if usable_dynamic_metadata_snapshot(&snapshot) => TargetOutcome::Success,
            Ok(_) => TargetOutcome::Error,
            Err(w4a::ControlPlaneError::Timeout { .. }) => TargetOutcome::Timeout,
            Err(_) => TargetOutcome::Error,
        }
    }
}

fn usable_dynamic_metadata_snapshot(snapshot: &w4a::PublicControlPlaneSnapshot) -> bool {
    let members = snapshot
        .admin_status
        .member_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let overview = snapshot
        .cluster_overview
        .members
        .iter()
        .map(|member| member.node_id.clone())
        .collect::<BTreeSet<_>>();
    let leader = snapshot.admin_status.leader.as_deref();
    snapshot.admin_status.source == w4a::ControlPlaneSource::Live
        && snapshot.cluster_overview.source == w4a::ControlPlaneSource::Live
        && snapshot.admin_status.quorum_ok
        && snapshot.admin_status.term > 0
        && snapshot.admin_status.epoch > 0
        && snapshot.admin_status.members as usize == members.len()
        && leader.is_some_and(|leader| members.contains(leader))
        && overview == members
        && snapshot
            .cluster_overview
            .leader
            .as_ref()
            .map(|value| value.node_id.as_str())
            == leader
}

struct LiveControlPlaneDriverState {
    harness: LiveControlPlaneProcessHarness,
    active_node_ids: BTreeSet<String>,
    initial_processes: BTreeMap<String, w4a::DaemonNodeProcessReceipt>,
    transient_node_id: Option<String>,
}

impl LiveControlPlaneDriverState {
    fn new(harness: LiveControlPlaneProcessHarness) -> Self {
        let initial_processes = harness
            .capability()
            .attestation
            .nodes
            .iter()
            .cloned()
            .map(|node| (node.node_id.clone(), node))
            .collect::<BTreeMap<_, _>>();
        let active_node_ids = initial_processes.keys().cloned().collect();
        Self {
            harness,
            active_node_ids,
            initial_processes,
            transient_node_id: None,
        }
    }

    fn active_endpoints(&self) -> Result<Vec<w4a::ControlPlaneEndpoint>, BrownoutError> {
        self.active_node_ids
            .iter()
            .map(|node_id| {
                self.harness
                    .endpoint(node_id)
                    .map_err(|error| BrownoutError::Driver(error.to_string()))
            })
            .collect()
    }
}

struct LiveControlPlaneBrownoutDriver {
    state: Mutex<Option<LiveControlPlaneDriverState>>,
}

impl LiveControlPlaneBrownoutDriver {
    fn new(harness: LiveControlPlaneProcessHarness) -> Self {
        Self {
            state: Mutex::new(Some(LiveControlPlaneDriverState::new(harness))),
        }
    }

    async fn shutdown(
        &self,
    ) -> Result<Vec<w4a::DaemonNodeLifecycleEvidence>, BrownoutProducerError> {
        let state = self.state.lock().await.take().ok_or_else(|| {
            BrownoutProducerError::ControlPlane(
                "W5A process state was already consumed before cleanup".to_owned(),
            )
        })?;
        state.harness.shutdown().map_err(Into::into)
    }
}

#[async_trait]
impl ControlPlaneBrownoutDriver for LiveControlPlaneBrownoutDriver {
    async fn observe_leader_failover(
        &self,
        plan: &ControlPlaneBrownoutLoadPlan,
    ) -> Result<RawControlPlaneEvent, BrownoutError> {
        let mut guard = self.state.lock().await;
        let state = guard
            .as_mut()
            .ok_or_else(|| BrownoutError::Driver("W5A process state is unavailable".to_owned()))?;
        let baseline =
            wait_for_control_baseline(state.active_endpoints()?, Duration::from_secs(15)).await?;
        let before = baseline.snapshots().to_vec();
        let old_leader = baseline.authority_node_id().to_owned();
        let old_term = before[0].admin_status.term;
        let expected_members = state.active_node_ids.clone();
        let load_endpoint = state
            .harness
            .endpoint(&old_leader)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let before_window = run_control_window(load_endpoint.clone(), plan.clone()).await?;
        let action_started = Instant::now();
        let disruption = tokio::spawn(run_control_window(load_endpoint, plan.clone()));
        tokio::task::yield_now().await;
        let waited = state
            .harness
            .kill_and_wait(&old_leader)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let survivors = state
            .active_endpoints()?
            .into_iter()
            .filter(|endpoint| endpoint.node_id != old_leader)
            .collect::<Vec<_>>();
        let committed = wait_for_new_leader(
            survivors,
            &old_leader,
            old_term,
            &expected_members,
            Duration::from_secs(15),
        )
        .await?;
        state
            .harness
            .restart(&old_leader)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let after_baseline =
            wait_for_control_baseline(state.active_endpoints()?, Duration::from_secs(15)).await?;
        let after = after_baseline.snapshots().to_vec();
        let recovered_endpoint = state
            .harness
            .endpoint(after_baseline.authority_node_id())
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let recovery = tokio::spawn(run_control_window(recovered_endpoint, plan.clone()));
        let (disruption_window, recovered_window) = tokio::try_join!(
            join_window(disruption, "leader-failover disruption"),
            join_window(recovery, "leader-failover recovery")
        )?;
        let recovered = Instant::now();
        let termination =
            WaitedProcessTermination::from_wait_status(waited.process.pid, waited.exit_status)?;
        let receipt = ControlPlaneActionReceipt::leader_failover(&waited.process, termination)?;
        RawControlPlaneEvent::from_observed(
            receipt,
            before,
            after,
            before_window,
            disruption_window,
            recovered_window,
            action_started,
            committed,
            recovered,
        )
    }

    async fn observe_member_add(
        &self,
        plan: &ControlPlaneBrownoutLoadPlan,
    ) -> Result<RawControlPlaneEvent, BrownoutError> {
        let mut guard = self.state.lock().await;
        let state = guard
            .as_mut()
            .ok_or_else(|| BrownoutError::Driver("W5A process state is unavailable".to_owned()))?;
        if state.transient_node_id.is_some() {
            return Err(BrownoutError::Driver(
                "W5A transient member already exists before add".to_owned(),
            ));
        }
        let baseline =
            wait_for_control_baseline(state.active_endpoints()?, Duration::from_secs(15)).await?;
        let before = baseline.snapshots().to_vec();
        let authority = baseline.authority_node_id().to_owned();
        let load_endpoint = state
            .harness
            .endpoint(&authority)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let before_window = run_control_window(load_endpoint.clone(), plan.clone()).await?;
        let action_started = Instant::now();
        let disruption = tokio::spawn(run_control_window(load_endpoint, plan.clone()));
        tokio::task::yield_now().await;
        let action = state
            .harness
            .spawn_transient_member(&authority)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let target_node_id = action.payload.target_node_id.clone();
        state.active_node_ids.insert(target_node_id.clone());
        let invocation = w4a::begin_daemon_add_transition(baseline, action.clone(), action_started)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let transition = w4a::observe_membership_transition(
            invocation,
            state.active_endpoints()?,
            Duration::from_secs(15),
            CONTROL_TRANSITION_POLL,
        )
        .await
        .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let committed = instant_after(action_started, transition.commit_latency_nanos)?;
        let after = transition.post_transition_snapshots.clone();
        let recovered_endpoint = state
            .harness
            .endpoint(&transition.authority_node_id)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let recovery = tokio::spawn(run_control_window(recovered_endpoint, plan.clone()));
        let (disruption_window, recovered_window) = tokio::try_join!(
            join_window(disruption, "member-add disruption"),
            join_window(recovery, "member-add recovery")
        )?;
        let recovered = Instant::now();
        state.transient_node_id = Some(target_node_id);
        let receipt = ControlPlaneActionReceipt::member_add(action)?;
        RawControlPlaneEvent::from_observed(
            receipt,
            before,
            after,
            before_window,
            disruption_window,
            recovered_window,
            action_started,
            committed,
            recovered,
        )
    }

    async fn observe_member_drain(
        &self,
        plan: &ControlPlaneBrownoutLoadPlan,
    ) -> Result<RawControlPlaneEvent, BrownoutError> {
        let mut guard = self.state.lock().await;
        let state = guard
            .as_mut()
            .ok_or_else(|| BrownoutError::Driver("W5A process state is unavailable".to_owned()))?;
        let target_node_id = state.transient_node_id.clone().ok_or_else(|| {
            BrownoutError::Driver("W5A drain has no preceding transient add".to_owned())
        })?;
        let target_process = state
            .harness
            .process_receipt(&target_node_id)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let baseline =
            wait_for_control_baseline(state.active_endpoints()?, Duration::from_secs(15)).await?;
        let before = baseline.snapshots().to_vec();
        let authority = baseline.authority_node_id().to_owned();
        let load_endpoint = state
            .harness
            .endpoint(&authority)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let before_window = run_control_window(load_endpoint.clone(), plan.clone()).await?;
        let action_started = Instant::now();
        let disruption = tokio::spawn(run_control_window(load_endpoint, plan.clone()));
        tokio::task::yield_now().await;
        let invocation =
            w4a::begin_admin_drain_transition(baseline, &target_node_id, CONTROL_PROBE_TIMEOUT)
                .await
                .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        state.active_node_ids.remove(&target_node_id);
        let transition = w4a::observe_membership_transition(
            invocation,
            state.active_endpoints()?,
            Duration::from_secs(15),
            CONTROL_TRANSITION_POLL,
        )
        .await
        .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let action = match transition.action_receipt.clone() {
            w4a::MembershipActionReceipt::AdminDrain(receipt) => receipt,
            w4a::MembershipActionReceipt::DaemonAdd(_) => {
                return Err(BrownoutError::Driver(
                    "W5A drain transition returned an add receipt".to_owned(),
                ))
            }
        };
        let committed = instant_after(action_started, transition.commit_latency_nanos)?;
        let waited = state
            .harness
            .kill_and_wait(&target_node_id)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let cleanup =
            WaitedProcessTermination::from_wait_status(waited.process.pid, waited.exit_status)?;
        state.transient_node_id = None;
        let after = transition.post_transition_snapshots.clone();
        let recovered_endpoint = state
            .harness
            .endpoint(&transition.authority_node_id)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let recovery = tokio::spawn(run_control_window(recovered_endpoint, plan.clone()));
        let (disruption_window, recovered_window) = tokio::try_join!(
            join_window(disruption, "member-drain disruption"),
            join_window(recovery, "member-drain recovery")
        )?;
        let recovered = Instant::now();
        let receipt = ControlPlaneActionReceipt::member_drain(action, &target_process, cleanup)?;
        RawControlPlaneEvent::from_observed(
            receipt,
            before,
            after,
            before_window,
            disruption_window,
            recovered_window,
            action_started,
            committed,
            recovered,
        )
    }

    async fn observe_node_kill_rejoin(
        &self,
        plan: &ControlPlaneBrownoutLoadPlan,
    ) -> Result<RawControlPlaneEvent, BrownoutError> {
        let mut guard = self.state.lock().await;
        let state = guard
            .as_mut()
            .ok_or_else(|| BrownoutError::Driver("W5A process state is unavailable".to_owned()))?;
        if state.transient_node_id.is_some() {
            return Err(BrownoutError::Driver(
                "W5A transient member was not drained before node kill/rejoin".to_owned(),
            ));
        }
        let baseline =
            wait_for_control_baseline(state.active_endpoints()?, Duration::from_secs(15)).await?;
        let before = baseline.snapshots().to_vec();
        let authority = baseline.authority_node_id().to_owned();
        let target_node_id = state
            .initial_processes
            .iter()
            .filter(|(node_id, _)| **node_id != authority)
            .find_map(|(node_id, original)| {
                state
                    .harness
                    .process_receipt(node_id)
                    .ok()
                    .filter(|current| current.pid == original.pid)
                    .map(|_| node_id.clone())
            })
            .ok_or_else(|| {
                BrownoutError::Driver(
                    "W5A has no untouched non-leader capability PID for kill/rejoin".to_owned(),
                )
            })?;
        let load_endpoint = state
            .harness
            .endpoint(&authority)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let before_window = run_control_window(load_endpoint.clone(), plan.clone()).await?;
        let action_started = Instant::now();
        let disruption = tokio::spawn(run_control_window(load_endpoint, plan.clone()));
        tokio::task::yield_now().await;
        let waited = state
            .harness
            .kill_and_wait(&target_node_id)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let committed = Instant::now();
        let restarted = state
            .harness
            .restart(&target_node_id)
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let after_baseline =
            wait_for_control_baseline(state.active_endpoints()?, Duration::from_secs(15)).await?;
        let after = after_baseline.snapshots().to_vec();
        let recovered_endpoint = state
            .harness
            .endpoint(after_baseline.authority_node_id())
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let recovery = tokio::spawn(run_control_window(recovered_endpoint, plan.clone()));
        let (disruption_window, recovered_window) = tokio::try_join!(
            join_window(disruption, "node-kill-rejoin disruption"),
            join_window(recovery, "node-kill-rejoin recovery")
        )?;
        let recovered = Instant::now();
        let termination =
            WaitedProcessTermination::from_wait_status(waited.process.pid, waited.exit_status)?;
        let restarted = ObservedProcessImage::from_w4a_receipt(&restarted)?;
        let receipt =
            ControlPlaneActionReceipt::node_kill_rejoin(&waited.process, termination, restarted)?;
        RawControlPlaneEvent::from_observed(
            receipt,
            before,
            after,
            before_window,
            disruption_window,
            recovered_window,
            action_started,
            committed,
            recovered,
        )
    }

    async fn finalize_cleanup(&self) -> Result<ControlPlaneFinalCleanupReceipt, BrownoutError> {
        let state = self.state.lock().await.take().ok_or_else(|| {
            BrownoutError::Driver("W5A process state was already finalized".to_owned())
        })?;
        let nodes = state
            .harness
            .shutdown()
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        ControlPlaneFinalCleanupReceipt::from_observed(nodes)
    }
}

async fn run_control_window(
    endpoint: w4a::ControlPlaneEndpoint,
    plan: ControlPlaneBrownoutLoadPlan,
) -> Result<OpenLoopObservation, BrownoutError> {
    let target = Arc::new(LiveMetadataTarget {
        endpoint,
        timeout: CONTROL_PROBE_TIMEOUT,
    });
    target
        .state_digest()
        .await
        .map_err(|error| BrownoutError::Driver(error.to_string()))?;
    run_open_loop(
        target,
        &open_loop_config(plan.offered_rate_per_second, plan.observation_window_millis)?,
    )
    .await
    .map_err(BrownoutError::Driver)
}

async fn join_window(
    handle: tokio::task::JoinHandle<Result<OpenLoopObservation, BrownoutError>>,
    label: &str,
) -> Result<OpenLoopObservation, BrownoutError> {
    handle
        .await
        .map_err(|error| BrownoutError::Driver(format!("{label} task failed: {error}")))?
}

async fn wait_for_control_baseline(
    endpoints: Vec<w4a::ControlPlaneEndpoint>,
    timeout: Duration,
) -> Result<w4a::MembershipBaseline, BrownoutError> {
    let deadline = Instant::now() + timeout;
    loop {
        let latest = match w4a::capture_membership_baseline_from_live(
            endpoints.clone(),
            CONTROL_PROBE_TIMEOUT,
        )
        .await
        {
            Ok(baseline) => return Ok(baseline),
            Err(error) => error.to_string(),
        };
        if Instant::now() >= deadline {
            return Err(BrownoutError::Driver(format!(
                "control-plane baseline did not converge in {timeout:?}: {latest}"
            )));
        }
        tokio::time::sleep(CONTROL_TRANSITION_POLL).await;
    }
}

async fn wait_for_new_leader(
    survivors: Vec<w4a::ControlPlaneEndpoint>,
    old_leader: &str,
    old_term: u64,
    expected_members: &BTreeSet<String>,
    timeout: Duration,
) -> Result<Instant, BrownoutError> {
    let deadline = Instant::now() + timeout;
    let mut latest = "no survivor exposed a new leader".to_owned();
    loop {
        for endpoint in &survivors {
            match w4a::probe_public_snapshot(endpoint.clone(), CONTROL_PROBE_TIMEOUT).await {
                Ok(snapshot) => {
                    let members = snapshot
                        .admin_status
                        .member_ids
                        .iter()
                        .cloned()
                        .collect::<BTreeSet<_>>();
                    let leader = snapshot.admin_status.leader.as_deref();
                    if snapshot.admin_status.source == w4a::ControlPlaneSource::Live
                        && snapshot.admin_status.quorum_ok
                        && snapshot.admin_status.term > old_term
                        && leader.is_some_and(|leader| leader != old_leader)
                        && members == *expected_members
                        && snapshot
                            .cluster_overview
                            .leader
                            .as_ref()
                            .map(|value| value.node_id.as_str())
                            == leader
                    {
                        return Ok(Instant::now());
                    }
                    latest = format!(
                        "{} still reports leader={leader:?} term={} members={members:?}",
                        endpoint.node_id, snapshot.admin_status.term
                    );
                }
                Err(error) => latest = error.to_string(),
            }
        }
        if Instant::now() >= deadline {
            return Err(BrownoutError::Driver(format!(
                "leader failover did not commit in {timeout:?}: {latest}"
            )));
        }
        tokio::time::sleep(CONTROL_TRANSITION_POLL).await;
    }
}

fn instant_after(start: Instant, nanos: u64) -> Result<Instant, BrownoutError> {
    start
        .checked_add(Duration::from_nanos(nanos))
        .ok_or_else(|| {
            BrownoutError::Driver("control-plane transition instant overflowed".to_owned())
        })
}

struct LiveRespDaemon {
    context: ValidatedRespReferenceContext,
    node_id: String,
    run_root: PathBuf,
    config: RespDaemonConfigIdentity,
    config_path: PathBuf,
    generation: u32,
    current_pid: Option<u32>,
    child: Option<Child>,
    capability: RespEndpointCapability,
}

impl LiveRespDaemon {
    async fn start(
        context: &ValidatedRespReferenceContext,
        repeat_index: u32,
        evidence_root: &Path,
        label: &str,
    ) -> Result<Self, BrownoutProducerError> {
        context.verify_binaries_unchanged().map_err(|error| {
            BrownoutProducerError::Contract(format!("pre-launch binary check failed: {error}"))
        })?;
        let ports = RespReferencePorts::select_available()
            .map_err(|error| BrownoutProducerError::System(error.to_string()))?;
        let run_root = create_resp_run_root(evidence_root, label)?;
        let data_dir = run_root.join("data");
        fs::create_dir(&data_dir).map_err(|error| {
            BrownoutProducerError::System(format!(
                "cannot create W5B data directory {}: {error}",
                data_dir.display()
            ))
        })?;
        let data_dir = fs::canonicalize(&data_dir).map_err(|error| {
            BrownoutProducerError::System(format!(
                "cannot canonicalize W5B data directory {}: {error}",
                data_dir.display()
            ))
        })?;
        let config = RespDaemonConfigIdentity {
            role: "local".to_owned(),
            listen_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            cluster_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            storage_dir: data_dir,
            admin_enabled: true,
            admin_addr: ports.admin,
            redis_enabled: true,
            redis_addr: ports.resp,
            redis_auth_required: false,
            rediss_enabled: false,
        };
        let config_path = run_root.join("direct-launch-config.json");
        write_new_json_file(&config_path, &config)?;
        let config_path = fs::canonicalize(&config_path).map_err(|error| {
            BrownoutProducerError::System(format!(
                "cannot canonicalize W5B config {}: {error}",
                config_path.display()
            ))
        })?;
        let node_id = format!("resp-{label}");
        let mut daemon = Self {
            context: context.clone(),
            node_id,
            run_root,
            config,
            config_path,
            generation: 0,
            current_pid: None,
            child: None,
            capability: RespEndpointCapability {
                schema_version: 0,
                pid: 0,
                started_unix_nanos: 0,
                repeat_index,
                direct_prebuilt_exec: false,
                fresh_data_dir: false,
                config: RespDaemonConfigIdentity {
                    role: String::new(),
                    listen_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                    cluster_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                    storage_dir: PathBuf::new(),
                    admin_enabled: false,
                    admin_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                    redis_enabled: false,
                    redis_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                    redis_auth_required: false,
                    rediss_enabled: false,
                },
                selected_endpoint: String::new(),
                server_binary_sha256: String::new(),
                loadgen_binary_sha256: String::new(),
                prebuild_manifest_sha256: String::new(),
                prebuild_contract_digest: String::new(),
                source_commit: String::new(),
            },
        };
        let started_unix_nanos = unix_nanos_now()?;
        let pid = daemon.spawn_child().await?;
        daemon.capability = RespEndpointCapability {
            schema_version: 1,
            pid,
            started_unix_nanos,
            repeat_index,
            direct_prebuilt_exec: true,
            fresh_data_dir: true,
            config: daemon.config.clone(),
            selected_endpoint: format!("hydracache-server@{}", daemon.config.redis_addr),
            server_binary_sha256: context.server.sha256.clone(),
            loadgen_binary_sha256: context.loadgen.sha256.clone(),
            prebuild_manifest_sha256: context.manifest_sha256.clone(),
            prebuild_contract_digest: context.build.prebuild_contract_digest.clone(),
            source_commit: context.source.git_commit.clone(),
        };
        daemon
            .capability
            .digest()
            .map_err(|error| BrownoutProducerError::Contract(error.to_string()))?;
        Ok(daemon)
    }

    fn capability(&self) -> &RespEndpointCapability {
        &self.capability
    }

    fn endpoint(&self) -> SocketAddr {
        self.config.redis_addr
    }

    fn process_image(&self) -> Result<ObservedProcessImage, BrownoutProducerError> {
        let pid = self.current_pid.ok_or_else(|| {
            BrownoutProducerError::Contract(format!(
                "{} has no live PID for process image",
                self.node_id
            ))
        })?;
        ObservedProcessImage::from_observed_paths(
            self.node_id.clone(),
            pid,
            &self.context.server.canonical_path,
            &self.config_path,
        )
        .map_err(Into::into)
    }

    async fn spawn_child(&mut self) -> Result<u32, BrownoutProducerError> {
        if self.child.is_some() || self.current_pid.is_some() {
            return Err(BrownoutProducerError::Contract(format!(
                "{} cannot spawn over a live child",
                self.node_id
            )));
        }
        self.context.verify_binaries_unchanged().map_err(|error| {
            BrownoutProducerError::Contract(format!("pre-spawn binary check failed: {error}"))
        })?;
        let stdout_path = self
            .run_root
            .join(format!("generation-{}.stdout.log", self.generation));
        let stderr_path = self
            .run_root
            .join(format!("generation-{}.stderr.log", self.generation));
        let stdout = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stdout_path)
            .map_err(|error| {
                BrownoutProducerError::System(format!(
                    "cannot create W5B stdout {}: {error}",
                    stdout_path.display()
                ))
            })?;
        let stderr = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&stderr_path)
            .map_err(|error| {
                BrownoutProducerError::System(format!(
                    "cannot create W5B stderr {}: {error}",
                    stderr_path.display()
                ))
            })?;
        let mut command = Command::new(&self.context.server.canonical_path);
        command
            .current_dir(&self.context.repo_root)
            .env_clear()
            .env("HYDRACACHE_ROLE", &self.config.role)
            .env(
                "HYDRACACHE_LISTEN_ADDR",
                self.config.listen_addr.to_string(),
            )
            .env(
                "HYDRACACHE_CLUSTER_ADDR",
                self.config.cluster_addr.to_string(),
            )
            .env("HYDRACACHE_STORAGE_DIR", &self.config.storage_dir)
            .env(
                "HYDRACACHE_ADMIN_API_ENABLED",
                self.config.admin_enabled.to_string(),
            )
            .env("HYDRACACHE_ADMIN_ADDR", self.config.admin_addr.to_string())
            .env(
                "HYDRACACHE_REDIS_API_ENABLED",
                self.config.redis_enabled.to_string(),
            )
            .env("HYDRACACHE_REDIS_ADDR", self.config.redis_addr.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        let mut child = command.spawn().map_err(|error| {
            BrownoutProducerError::System(format!(
                "cannot direct-exec W5B server {}: {error}",
                self.context.server.canonical_path.display()
            ))
        })?;
        let pid = child.id();
        if pid == 0 {
            let _ = child.kill();
            let _ = child.wait();
            return Err(BrownoutProducerError::System(
                "W5B direct child returned PID 0".to_owned(),
            ));
        }
        if let Err(error) = wait_for_resp_ping(&mut child, self.config.redis_addr).await {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        self.generation = self.generation.saturating_add(1);
        self.current_pid = Some(pid);
        self.child = Some(child);
        Ok(pid)
    }

    fn kill_and_wait(&mut self) -> Result<(u32, ExitStatus), BrownoutProducerError> {
        let pid = self.current_pid.ok_or_else(|| {
            BrownoutProducerError::Contract(format!("{} has no live PID", self.node_id))
        })?;
        let mut child = self.child.take().ok_or_else(|| {
            BrownoutProducerError::Contract(format!("{} has no owned child", self.node_id))
        })?;
        match child.try_wait().map_err(|error| {
            BrownoutProducerError::System(format!("cannot inspect PID {pid}: {error}"))
        })? {
            None => {}
            Some(status) => {
                self.current_pid = None;
                return Err(BrownoutProducerError::System(format!(
                    "W5B PID {pid} exited before requested kill: {status}"
                )));
            }
        }
        child.kill().map_err(|error| {
            BrownoutProducerError::System(format!("cannot kill W5B PID {pid}: {error}"))
        })?;
        let status = child.wait().map_err(|error| {
            BrownoutProducerError::System(format!("cannot wait W5B PID {pid}: {error}"))
        })?;
        self.current_pid = None;
        Ok((pid, status))
    }

    async fn restart(&mut self) -> Result<ObservedProcessImage, BrownoutProducerError> {
        self.spawn_child().await?;
        self.process_image()
    }
}

impl Drop for LiveRespDaemon {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
            self.current_pid = None;
        }
    }
}

struct LiveRespProcessState {
    selected: LiveRespDaemon,
    controls: Vec<LiveRespDaemon>,
    // Keep the lane guard last so child fields reap before another run starts.
    _process_guard: tokio::sync::MutexGuard<'static, ()>,
}

struct LiveRespBrownoutDriver {
    state: Mutex<Option<LiveRespProcessState>>,
}

impl LiveRespBrownoutDriver {
    fn new(
        selected: LiveRespDaemon,
        controls: Vec<LiveRespDaemon>,
        process_guard: tokio::sync::MutexGuard<'static, ()>,
    ) -> Self {
        Self {
            state: Mutex::new(Some(LiveRespProcessState {
                selected,
                controls,
                _process_guard: process_guard,
            })),
        }
    }

    async fn consumed(&self) -> bool {
        self.state.lock().await.is_none()
    }
}

#[async_trait]
impl RespBrownoutDriver for LiveRespBrownoutDriver {
    async fn observe_selected_endpoint_kill_restart(
        &self,
        plan: &RespBrownoutLoadPlan,
    ) -> Result<RawRespEndpointEvent, BrownoutError> {
        let mut state = self.state.lock().await.take().ok_or_else(|| {
            BrownoutError::Driver("W5B process state was already consumed".to_owned())
        })?;
        if state.controls.len() != usize::from(plan.independent_control_endpoints) {
            return Err(BrownoutError::Driver(
                "W5B process set does not match the committed control count".to_owned(),
            ));
        }
        let selected_endpoint = state.selected.endpoint();
        let original = state
            .selected
            .process_image()
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let before_target = prepare_resp_target(selected_endpoint, &plan.selected_capacity).await?;
        let disruption_target =
            prepare_resp_target(selected_endpoint, &plan.selected_capacity).await?;
        let mut control_targets = Vec::with_capacity(state.controls.len());
        let mut control_processes = Vec::with_capacity(state.controls.len());
        for control in &state.controls {
            control_targets
                .push(prepare_resp_target(control.endpoint(), &plan.selected_capacity).await?);
            control_processes.push(
                control
                    .process_image()
                    .map_err(|error| BrownoutError::Driver(error.to_string()))?,
            );
        }
        let before_window = run_resp_window(before_target, plan.clone()).await?;
        let kill_started = Instant::now();
        let disruption = tokio::spawn(run_resp_window(disruption_target, plan.clone()));
        let mut control_tasks = Vec::with_capacity(control_targets.len());
        for target in control_targets {
            control_tasks.push(tokio::spawn(run_resp_window(target, plan.clone())));
        }
        tokio::task::yield_now().await;
        let (original_pid, original_status) = state
            .selected
            .kill_and_wait()
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let termination =
            WaitedProcessTermination::from_wait_status(original_pid, original_status)?;
        let (socket_unavailable, socket_down) =
            wait_for_socket_down(selected_endpoint, Duration::from_secs(2)).await?;
        let restarted = state
            .selected
            .restart()
            .await
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let recovered_target =
            prepare_resp_target(selected_endpoint, &plan.selected_capacity).await?;
        let recovery = tokio::spawn(run_resp_window(recovered_target, plan.clone()));
        let disruption_window = join_resp_window(disruption, "selected disruption").await?;
        let recovered_window = join_resp_window(recovery, "selected recovery").await?;
        let mut control_windows = Vec::with_capacity(control_tasks.len());
        for task in control_tasks {
            control_windows.push(join_resp_window(task, "independent control").await?);
        }
        let recovered = Instant::now();
        let (restarted_pid, restarted_status) = state
            .selected
            .kill_and_wait()
            .map_err(|error| BrownoutError::Driver(error.to_string()))?;
        let restarted_cleanup =
            WaitedProcessTermination::from_wait_status(restarted_pid, restarted_status)?;
        let mut independent_controls = Vec::with_capacity(state.controls.len());
        for ((control, process), window) in state
            .controls
            .iter_mut()
            .zip(control_processes)
            .zip(control_windows)
        {
            let endpoint = control.endpoint().to_string();
            let (pid, status) = control
                .kill_and_wait()
                .map_err(|error| BrownoutError::Driver(error.to_string()))?;
            independent_controls.push(IndependentRespRawWindow {
                endpoint,
                process,
                cleanup: WaitedProcessTermination::from_wait_status(pid, status)?,
                window,
            });
        }
        RawRespEndpointEvent::from_observed(
            selected_endpoint.to_string(),
            original,
            termination,
            restarted,
            restarted_cleanup,
            socket_unavailable,
            before_window,
            disruption_window,
            recovered_window,
            independent_controls,
            kill_started,
            socket_down,
            recovered,
        )
    }
}

async fn prepare_resp_target(
    endpoint: SocketAddr,
    contract: &RespSelectedCapacityContract,
) -> Result<Arc<RespTcpTarget>, BrownoutError> {
    let seed = contract.workload.seed.ok_or_else(|| {
        BrownoutError::Driver("selected W3 workload has no deterministic seed".to_owned())
    })?;
    if contract.workload.generator_version != KEY_SCHEDULE_GENERATOR_VERSION.to_string() {
        return Err(BrownoutError::Driver(
            "selected W3 key-schedule generator version changed".to_owned(),
        ));
    }
    let distribution = match contract.workload.key_distribution.as_ref() {
        Some(identity) if identity.kind == "uniform" && identity.theta.is_none() => {
            KeyDistribution::Uniform
        }
        _ => {
            return Err(BrownoutError::Driver(
                "W5B currently consumes the exact uniform W3 A/B/C schedule only".to_owned(),
            ))
        }
    };
    let schedule_operations = contract
        .preload_operations
        .max(contract.warmup_operations)
        .max(contract.steady_operations);
    let schedule = KeyScheduleSpec {
        generator_version: KEY_SCHEDULE_GENERATOR_VERSION,
        seed,
        key_count: contract.key_count,
        operations: schedule_operations,
        distribution,
    }
    .generate()
    .map_err(BrownoutError::Driver)?;
    if resp_workload_digest(&schedule.digest, &contract.workload)? != contract.workload.digest {
        return Err(BrownoutError::Driver(
            "W5B reconstructed key schedule differs from selected W3 workload digest".to_owned(),
        ));
    }
    let payload_bytes = usize::try_from(contract.workload.payload_mix[0].bytes)
        .map_err(|_| BrownoutError::Driver("RESP payload size overflows usize".to_owned()))?;
    let target = Arc::new(
        RespTcpTarget::new(RespTargetConfig {
            endpoint: RespEndpointIdentity {
                address: endpoint,
                selected_endpoint: format!("hydracache-server@{endpoint}"),
                endpoint_kind: "node-resp".to_owned(),
                state_scope: "node-local".to_owned(),
            },
            require_loopback: true,
            connections: usize::try_from(contract.connections).map_err(|_| {
                BrownoutError::Driver("RESP connection count overflows usize".to_owned())
            })?,
            pipeline_depth: usize::try_from(contract.pipeline_depth).map_err(|_| {
                BrownoutError::Driver("RESP pipeline depth overflows usize".to_owned())
            })?,
            preload_entries: contract.preload_operations,
            key_space: contract.key_count,
            payload_bytes,
            batch_size: usize::try_from(contract.multi_key_width).map_err(|_| {
                BrownoutError::Driver("RESP multi-key width overflows usize".to_owned())
            })?,
            reset_batch_entries: usize::try_from(contract.reset_batch_entries).map_err(|_| {
                BrownoutError::Driver("RESP reset batch overflows usize".to_owned())
            })?,
            operation_mix: resp_operation_mix(&contract.workload)?,
            key_schedule: Arc::new(schedule.keys),
            connect_timeout: Duration::from_secs(2),
            io_timeout: Duration::from_secs(2),
            parser_limits: Resp2Limits::default(),
            injected_dispatch_delay: Duration::ZERO,
        })
        .map_err(|error| BrownoutError::Driver(error.to_string()))?,
    );
    target
        .reset()
        .await
        .map_err(|error| BrownoutError::Driver(error.to_string()))?;
    let preload = target
        .preload()
        .await
        .map_err(|error| BrownoutError::Driver(error.to_string()))?;
    if preload.operations != contract.preload_operations {
        return Err(BrownoutError::Driver(
            "W5B target did not retain the selected W3 preload count".to_owned(),
        ));
    }
    Ok(target)
}

async fn run_resp_window(
    target: Arc<RespTcpTarget>,
    plan: RespBrownoutLoadPlan,
) -> Result<OpenLoopObservation, BrownoutError> {
    run_open_loop(
        target,
        &open_loop_config(plan.offered_rate_per_second, plan.observation_window_millis)?,
    )
    .await
    .map_err(BrownoutError::Driver)
}

async fn join_resp_window(
    handle: tokio::task::JoinHandle<Result<OpenLoopObservation, BrownoutError>>,
    label: &str,
) -> Result<OpenLoopObservation, BrownoutError> {
    handle
        .await
        .map_err(|error| BrownoutError::Driver(format!("W5B {label} task failed: {error}")))?
}

fn resp_operation_mix(workload: &WorkloadIdentity) -> Result<RespOperationMix, BrownoutError> {
    let mut percentages = BTreeMap::new();
    for operation in &workload.operation_mix {
        if !operation.weight.is_finite() || operation.weight <= 0.0 {
            return Err(BrownoutError::Driver(
                "W3 operation weight is non-positive or non-finite".to_owned(),
            ));
        }
        let percent = (operation.weight * 100.0).round();
        if !(0.0..=100.0).contains(&percent) || (percent / 100.0 - operation.weight).abs() > 1e-9 {
            return Err(BrownoutError::Driver(
                "W3 operation weights do not map to exact integer percentages".to_owned(),
            ));
        }
        percentages.insert(operation.operation.as_str(), percent as u8);
    }
    let mix = RespOperationMix {
        get_percent: *percentages.get("get").unwrap_or(&0),
        set_percent: *percentages.get("set").unwrap_or(&0),
        mget_percent: *percentages.get("mget").unwrap_or(&0),
        mset_percent: *percentages.get("mset").unwrap_or(&0),
    };
    if percentages
        .keys()
        .any(|operation| !matches!(*operation, "get" | "set" | "mget" | "mset"))
        || mix.total_percent() != 100
        || !matches!(
            mix,
            RespOperationMix::WORKLOAD_A
                | RespOperationMix::WORKLOAD_B
                | RespOperationMix::WORKLOAD_C
        )
    {
        return Err(BrownoutError::Driver(
            "selected workload is outside the committed W3 A/B/C taxonomy".to_owned(),
        ));
    }
    Ok(mix)
}

fn resp_workload_digest(
    schedule_digest: &str,
    workload: &WorkloadIdentity,
) -> Result<String, BrownoutError> {
    let operations = serde_json::to_vec(&workload.operation_mix)
        .map_err(|error| BrownoutError::Driver(error.to_string()))?;
    let payloads = serde_json::to_vec(&workload.payload_mix)
        .map_err(|error| BrownoutError::Driver(error.to_string()))?;
    Ok(digest_parts(&[
        schedule_digest.as_bytes(),
        b"hydracache-resp-open-loop-workload-v1",
        &operations,
        &payloads,
    ]))
}

async fn wait_for_resp_ping(
    child: &mut Child,
    endpoint: SocketAddr,
) -> Result<(), BrownoutProducerError> {
    let deadline = Instant::now() + RESP_STARTUP_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            BrownoutProducerError::System(format!("cannot inspect W5B child: {error}"))
        })? {
            return Err(BrownoutProducerError::System(format!(
                "W5B child exited before readiness: {status}"
            )));
        }
        let latest = match strict_resp_ping(endpoint).await {
            Ok(()) => return Ok(()),
            Err(error) => error.to_string(),
        };
        if Instant::now() >= deadline {
            return Err(BrownoutProducerError::System(format!(
                "W5B RESP readiness timed out: {latest}"
            )));
        }
        tokio::time::sleep(RESP_POLL_INTERVAL).await;
    }
}

async fn strict_resp_ping(endpoint: SocketAddr) -> Result<(), std::io::Error> {
    let mut stream = TcpStream::connect(endpoint).await?;
    stream.write_all(RESP_PING_FRAME).await?;
    stream.flush().await?;
    let mut response = [0_u8; 7];
    stream.read_exact(&mut response).await?;
    if response != RESP_PONG_FRAME {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "RESP PING returned a non-PONG frame",
        ));
    }
    Ok(())
}

async fn wait_for_socket_down(
    endpoint: SocketAddr,
    timeout: Duration,
) -> Result<(SocketUnavailableReceipt, Instant), BrownoutError> {
    let deadline = Instant::now() + timeout;
    loop {
        match TcpStream::connect(endpoint).await {
            Ok(stream) => drop(stream),
            Err(error) => {
                let observed = Instant::now();
                let receipt =
                    SocketUnavailableReceipt::from_connect_error(endpoint.to_string(), &error)?;
                return Ok((receipt, observed));
            }
        }
        if Instant::now() >= deadline {
            return Err(BrownoutError::Driver(format!(
                "W5B selected socket {endpoint} never became unavailable"
            )));
        }
        tokio::time::sleep(RESP_POLL_INTERVAL).await;
    }
}

fn open_loop_config(
    offered_rate_per_second: u64,
    observation_window_millis: u64,
) -> Result<OpenLoopConfig, BrownoutError> {
    let operations = u128::from(offered_rate_per_second)
        .checked_mul(u128::from(observation_window_millis))
        .and_then(|value| value.checked_div(1_000))
        .and_then(|value| u64::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| BrownoutError::Driver("W5 open-loop operation count overflow".to_owned()))?;
    Ok(OpenLoopConfig {
        offered_rate_per_second,
        operations,
        highest_trackable_latency: Duration::from_secs(10),
        significant_figures: 3,
        p999_min_samples: 1,
        drain_timeout: Duration::from_secs(10),
    })
}

fn create_resp_run_root(
    evidence_root: &Path,
    label: &str,
) -> Result<PathBuf, BrownoutProducerError> {
    if !evidence_root.is_absolute()
        || label.is_empty()
        || !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(BrownoutProducerError::Contract(format!(
            "invalid explicit W5B evidence root/label: {} {label:?}",
            evidence_root.display()
        )));
    }
    fs::create_dir_all(evidence_root).map_err(|error| {
        BrownoutProducerError::System(format!(
            "cannot create W5B evidence root {}: {error}",
            evidence_root.display()
        ))
    })?;
    let root = fs::canonicalize(evidence_root).map_err(|error| {
        BrownoutProducerError::System(format!(
            "cannot canonicalize W5B evidence root {}: {error}",
            evidence_root.display()
        ))
    })?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| BrownoutProducerError::System(error.to_string()))?
        .as_nanos();
    let sequence = ARTIFACT_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let run_root = root.join(format!(
        "w5b-{label}-pid-{}-nanos-{nonce}-seq-{sequence}",
        std::process::id()
    ));
    fs::create_dir(&run_root).map_err(|error| {
        BrownoutProducerError::System(format!(
            "cannot create unique W5B run root {}: {error}",
            run_root.display()
        ))
    })?;
    fs::canonicalize(&run_root).map_err(|error| {
        BrownoutProducerError::System(format!(
            "cannot canonicalize W5B run root {}: {error}",
            run_root.display()
        ))
    })
}

fn write_new_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), BrownoutProducerError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        BrownoutProducerError::System(format!("cannot serialize {}: {error}", path.display()))
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            BrownoutProducerError::System(format!(
                "cannot create strict JSON {}: {error}",
                path.display()
            ))
        })?;
    file.write_all(&bytes).map_err(|error| {
        BrownoutProducerError::System(format!("cannot write {}: {error}", path.display()))
    })?;
    file.sync_all().map_err(|error| {
        BrownoutProducerError::System(format!("cannot sync {}: {error}", path.display()))
    })
}

fn unix_nanos_now() -> Result<u64, BrownoutProducerError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| BrownoutProducerError::System(error.to_string()))?
        .as_nanos();
    u64::try_from(nanos)
        .map_err(|_| BrownoutProducerError::System("system nanoseconds overflow u64".to_owned()))
}

fn digest_parts(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    sha256_digest_hex(&hasher.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    sha256_digest_hex(&Sha256::digest(bytes))
}

fn sha256_digest_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn producer_window_is_exact_rate_times_duration() {
        let config = open_loop_config(600, 10_000).expect("valid exact window");
        assert_eq!(config.offered_rate_per_second, 600);
        assert_eq!(config.operations, 6_000);
    }

    #[test]
    fn report_writer_refuses_stale_evidence() {
        let sequence = ARTIFACT_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "hydracache-w5-writer-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("create isolated test directory");
        let path = root.join("report.json");
        write_new_json_atomic(&path, &serde_json::json!({"run": 1})).expect("first report lands");
        let error = write_new_json_atomic(&path, &serde_json::json!({"run": 2}))
            .expect_err("stale report must not be overwritten");
        assert!(error.to_string().contains("refusing to overwrite"));
        fs::remove_dir_all(&root).expect("remove isolated test directory");
    }
}
