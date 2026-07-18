//! Real reference producer for W4A control-plane evidence.
//!
//! The producer owns every direct child from reservation through reap. It
//! stages N-1 bootstrap daemons, observes the add of one joiner, measures one
//! leader and one follower independently, observes a real drain, then seals
//! lifecycle evidence before the report is allowed to validate or land.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::targets::control_plane::{
    begin_admin_drain_transition, begin_daemon_add_transition,
    capture_membership_baseline_from_live, observe_membership_transition,
    probe_control_plane_capability, run_control_plane_knee, ControlPlaneCapabilityAttestation,
    ControlPlaneCapabilityOutcome, ControlPlaneCapabilityReceiptPayload, ControlPlaneEndpoint,
    ControlPlaneError, ControlPlaneLifecycleReceipt, ControlPlaneLifecycleReceiptPayload,
    ControlPlaneReport, ControlPlaneScenario, ControlPlaneTarget, DaemonAddActionPayload,
    DaemonAddInvocationReceipt, DaemonNodeConfigReceipt, DaemonNodeLaunchConfig,
    DaemonNodeLifecycleEvidence, DaemonNodeProcessReceipt, DaemonReceiptSource, NodeRole,
    PrebuiltServerBinaryReceipt, ProbedControlPlaneCapability, ProcessLogReceipt,
    ReferenceCapabilityPolicy, CONTROL_PLANE_EVIDENCE_CLASS, CONTROL_PLANE_EXECUTION_MODE,
    DAEMON_CAPABILITY_RECEIPT_KIND, DAEMON_CLUSTER_PROVISIONER, NODE_CONFIG_RECEIPT_KIND,
};
use crate::tiers::resp_reference::ValidatedRespReferenceContext;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(90);
const PER_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const LIFECYCLE_RECEIPT_KIND: &str = "hydracache-daemon-cluster-lifecycle-v1";
const ADD_ACTION_RECEIPT_KIND: &str = "hydracache-daemon-add-action-v1";

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static CONTROL_PLANE_PROCESS_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Error)]
pub enum ControlPlaneReferenceError {
    #[error("W4A reference contract rejected: {0}")]
    Contract(String),
    #[error("W4A reference system operation failed: {0}")]
    System(String),
    #[error(transparent)]
    Evidence(#[from] ControlPlaneError),
    #[error("W4A daemon lifecycle failed: {0}")]
    Lifecycle(String),
}

/// Run one complete 3/5/7-node reference artifact and write it only after all
/// daemons have been killed, waited, and re-hashed.
pub async fn run_control_plane_reference(
    context: &ValidatedRespReferenceContext,
    scenario: &ControlPlaneScenario,
    node_count: u8,
    evidence_root: &Path,
    report_path: &Path,
) -> Result<ControlPlaneReport, ControlPlaneReferenceError> {
    let _process_guard = CONTROL_PLANE_PROCESS_LOCK.lock().await;
    require_mandatory_lane()?;
    scenario.validate()?;
    if !scenario.read_only.node_counts.contains(&node_count) {
        return Err(ControlPlaneReferenceError::Contract(format!(
            "W4A reference node_count must be one of 3/5/7, got {node_count}"
        )));
    }
    context
        .verify_binaries_unchanged()
        .map_err(|error| ControlPlaneReferenceError::Contract(error.to_string()))?;

    let mut cluster = ReferenceControlPlaneCluster::prepare(node_count, evidence_root)?;
    cluster.spawn_initial(context)?;
    let initial_endpoints = cluster.initial_endpoints();
    let add_baseline = wait_for_live_baseline(initial_endpoints.clone(), STARTUP_TIMEOUT).await?;

    let add_invoked_at = Instant::now();
    cluster.spawn_joiner(context)?;
    let add_receipt =
        cluster.write_add_action_receipt(add_baseline.authority_node_id(), context)?;
    let add_invocation = begin_daemon_add_transition(add_baseline, add_receipt, add_invoked_at)?;
    let full_endpoints = cluster.full_endpoints();
    let add_event = observe_membership_transition(
        add_invocation,
        full_endpoints.clone(),
        Duration::from_millis(scenario.membership_event.event_timeout_millis),
        POLL_INTERVAL,
    )
    .await?;

    let capability_receipt = cluster.seal_capability(context)?;
    let validated =
        capability_receipt.require(scenario, ReferenceCapabilityPolicy::MandatoryFailClosed)?;
    let ControlPlaneCapabilityOutcome::Ready(validated) = validated else {
        return Err(ControlPlaneReferenceError::Contract(
            "mandatory W4A capability unexpectedly skipped".to_owned(),
        ));
    };
    let probed = Arc::new(
        probe_control_plane_capability(*validated, PER_PROBE_TIMEOUT)
            .await
            .map_err(ControlPlaneReferenceError::from)?,
    );
    let leader = selected_role_node(&probed, NodeRole::Leader)?;
    let follower = selected_role_node(&probed, NodeRole::Follower)?;
    let timeout = Duration::from_millis(scenario.read_only.timeout_millis);
    let leader_target = Arc::new(ControlPlaneTarget::new(
        Arc::clone(&probed),
        &leader,
        NodeRole::Leader,
        timeout,
    )?);
    let leader_evidence = run_control_plane_knee(leader_target, scenario).await?;
    let follower_target = Arc::new(ControlPlaneTarget::new(
        Arc::clone(&probed),
        &follower,
        NodeRole::Follower,
        timeout,
    )?);
    let follower_evidence = run_control_plane_knee(follower_target, scenario).await?;

    let drain_baseline = wait_for_live_baseline(full_endpoints, STARTUP_TIMEOUT).await?;
    let drain_target = cluster.joiner_node_id().to_owned();
    let drain_invocation = begin_admin_drain_transition(
        drain_baseline,
        &drain_target,
        Duration::from_millis(scenario.read_only.timeout_millis),
    )
    .await?;
    let drain_event = observe_membership_transition(
        drain_invocation,
        initial_endpoints,
        Duration::from_millis(scenario.membership_event.event_timeout_millis),
        POLL_INTERVAL,
    )
    .await?;

    let lifecycle = cluster.stop_all(context, probed.receipt_sha256())?;
    let report = ControlPlaneReport {
        schema_version: 1,
        scenario_id: scenario.scenario_id.clone(),
        evidence_class: CONTROL_PLANE_EVIDENCE_CLASS.to_owned(),
        execution_mode: CONTROL_PLANE_EXECUTION_MODE.to_owned(),
        capability_receipt_sha256: probed.receipt_sha256().to_owned(),
        capability: probed.attestation.receipt.clone(),
        node_count,
        capacity_scope: "per-selected-admin-endpoint-and-role-no-sum".to_owned(),
        aggregate_cluster_capacity: false,
        product_data_plane: false,
        live_reshard_measured: false,
        steady_reads: vec![leader_evidence, follower_evidence],
        membership_events: vec![add_event, drain_event],
        lifecycle,
        deferred_claims: vec!["live-rebalance-reshard-performance".to_owned()],
    };
    report.validate(scenario, &probed)?;
    write_new_report(report_path, &report)?;
    Ok(report)
}

/// Shared direct-process seam for fault-oriented tiers. It deliberately
/// exposes process primitives and existing W4 receipts, not W5 report types,
/// so the W4 reference producer and the W5 brownout producer cannot drift into
/// independent daemon launch implementations.
pub struct LiveControlPlaneProcessHarness {
    context: ValidatedRespReferenceContext,
    cluster: ReferenceControlPlaneCluster,
    capability: ProbedControlPlaneCapability,
    // Fields drop in declaration order: keep the global lane guard last so
    // every child-owning cluster field is dropped/reaped before unlock.
    _process_guard: tokio::sync::MutexGuard<'static, ()>,
}

impl LiveControlPlaneProcessHarness {
    /// Stage one fresh, fully joined 3/5/7-node cluster and seal/probe the exact
    /// same typed capability used by W4. No predecessor PID is reused.
    pub async fn stage(
        context: &ValidatedRespReferenceContext,
        scenario: &ControlPlaneScenario,
        node_count: u8,
        evidence_root: &Path,
    ) -> Result<Self, ControlPlaneReferenceError> {
        let process_guard = CONTROL_PLANE_PROCESS_LOCK.lock().await;
        require_mandatory_lane()?;
        scenario.validate()?;
        if !scenario.read_only.node_counts.contains(&node_count) {
            return Err(ControlPlaneReferenceError::Contract(format!(
                "live control-plane harness node_count must be one of 3/5/7, got {node_count}"
            )));
        }
        context
            .verify_binaries_unchanged()
            .map_err(|error| ControlPlaneReferenceError::Contract(error.to_string()))?;
        let mut cluster = ReferenceControlPlaneCluster::prepare(node_count, evidence_root)?;
        cluster.spawn_initial(context)?;
        wait_for_live_baseline(cluster.initial_endpoints(), STARTUP_TIMEOUT).await?;
        cluster.spawn_joiner(context)?;
        wait_for_live_baseline(cluster.full_endpoints(), STARTUP_TIMEOUT).await?;
        let sealed = cluster.seal_capability(context)?;
        let validated = sealed.require(scenario, ReferenceCapabilityPolicy::MandatoryFailClosed)?;
        let ControlPlaneCapabilityOutcome::Ready(validated) = validated else {
            return Err(ControlPlaneReferenceError::Contract(
                "mandatory live control-plane harness unexpectedly skipped".to_owned(),
            ));
        };
        let capability = probe_control_plane_capability(*validated, PER_PROBE_TIMEOUT).await?;
        Ok(Self {
            context: context.clone(),
            cluster,
            capability,
            _process_guard: process_guard,
        })
    }

    pub fn capability(&self) -> &ProbedControlPlaneCapability {
        &self.capability
    }

    pub fn endpoints(&self) -> Vec<ControlPlaneEndpoint> {
        self.cluster.full_endpoints()
    }

    pub fn endpoint(
        &self,
        node_id: &str,
    ) -> Result<ControlPlaneEndpoint, ControlPlaneReferenceError> {
        self.cluster
            .nodes
            .iter()
            .find(|node| node.config.launch_config.node_id == node_id)
            .map(ReferenceNode::endpoint)
            .ok_or_else(|| {
                ControlPlaneReferenceError::Contract(format!(
                    "live control-plane harness has no node {node_id:?}"
                ))
            })
    }

    pub fn process_receipt(
        &self,
        node_id: &str,
    ) -> Result<DaemonNodeProcessReceipt, ControlPlaneReferenceError> {
        self.cluster
            .nodes
            .iter()
            .find(|node| node.config.launch_config.node_id == node_id)
            .ok_or_else(|| {
                ControlPlaneReferenceError::Contract(format!(
                    "live control-plane harness has no node {node_id:?}"
                ))
            })?
            .process_receipt(&self.context)
    }

    /// Kill and wait the exact currently owned PID. The node cannot restart
    /// until this method has returned an OS `ExitStatus`.
    pub fn kill_and_wait(
        &mut self,
        node_id: &str,
    ) -> Result<WaitedDaemonProcess, ControlPlaneReferenceError> {
        let node = self
            .cluster
            .nodes
            .iter_mut()
            .find(|node| node.config.launch_config.node_id == node_id)
            .ok_or_else(|| {
                ControlPlaneReferenceError::Contract(format!(
                    "live control-plane harness has no node {node_id:?}"
                ))
            })?;
        node.kill_and_wait(&self.context)
    }

    /// Restart a previously waited node with byte-identical config and server
    /// binary, under a fresh PID and separate log generation.
    pub fn restart(
        &mut self,
        node_id: &str,
    ) -> Result<DaemonNodeProcessReceipt, ControlPlaneReferenceError> {
        let run_root = self.cluster.run_root.clone();
        let node = self
            .cluster
            .nodes
            .iter_mut()
            .find(|node| node.config.launch_config.node_id == node_id)
            .ok_or_else(|| {
                ControlPlaneReferenceError::Contract(format!(
                    "live control-plane harness has no node {node_id:?}"
                ))
            })?;
        if node.child.is_some() || node.pid.is_some() {
            return Err(ControlPlaneReferenceError::Contract(format!(
                "node {node_id:?} must be killed and waited before restart"
            )));
        }
        node.prepare_restart_logs(&run_root);
        node.spawn(&self.context)?;
        node.process_receipt(&self.context)
    }

    /// Spawn one additional joiner and write the same physical typed
    /// add-action receipt format used by W4.
    pub fn spawn_transient_member(
        &mut self,
        authority_node_id: &str,
    ) -> Result<DaemonAddInvocationReceipt, ControlPlaneReferenceError> {
        let index = self.cluster.prepare_transient_joiner()?;
        let node = &mut self.cluster.nodes[index];
        node.release_reservations();
        node.spawn(&self.context)?;
        let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        self.cluster.write_add_action_receipt_for(
            index,
            authority_node_id,
            &self.context,
            &format!("daemon-add-action-transient-{sequence}.json"),
        )
    }

    /// Reap every still-live child (including restarted/transient nodes) and
    /// verify the prebuilt server/loadgen bytes once more. Drop provides the
    /// same reap guarantee on every early-return path.
    pub fn shutdown(
        mut self,
    ) -> Result<Vec<DaemonNodeLifecycleEvidence>, ControlPlaneReferenceError> {
        self.cluster.stop_all_unsealed(&self.context)
    }
}

fn require_mandatory_lane() -> Result<(), ControlPlaneReferenceError> {
    if std::env::var("HYDRACACHE_RUN_PERF_CONTROL_PLANE").as_deref() != Ok("1") {
        return Err(ControlPlaneReferenceError::Contract(
            "real W4A producer requires HYDRACACHE_RUN_PERF_CONTROL_PLANE=1".to_owned(),
        ));
    }
    Ok(())
}

fn selected_role_node(
    capability: &crate::targets::control_plane::ProbedControlPlaneCapability,
    role: NodeRole,
) -> Result<String, ControlPlaneReferenceError> {
    let matches = capability
        .baseline
        .iter()
        .filter_map(|snapshot| {
            snapshot
                .target_role()
                .ok()
                .filter(|observed| *observed == role)
                .map(|_| snapshot.endpoint.node_id.clone())
        })
        .collect::<Vec<_>>();
    match role {
        NodeRole::Leader if matches.len() == 1 => Ok(matches[0].clone()),
        NodeRole::Follower if !matches.is_empty() => Ok(matches[0].clone()),
        _ => Err(ControlPlaneReferenceError::Contract(format!(
            "live W4A capability has no unambiguous {role:?} target"
        ))),
    }
}

async fn wait_for_live_baseline(
    endpoints: Vec<ControlPlaneEndpoint>,
    timeout: Duration,
) -> Result<crate::targets::control_plane::MembershipBaseline, ControlPlaneReferenceError> {
    let deadline = Instant::now() + timeout;
    loop {
        let last_error =
            match capture_membership_baseline_from_live(endpoints.clone(), PER_PROBE_TIMEOUT).await
            {
                Ok(baseline) => return Ok(baseline),
                Err(error) => error.to_string(),
            };
        if Instant::now() >= deadline {
            return Err(ControlPlaneReferenceError::System(format!(
                "real daemon membership did not become live within {}ms; last_error={last_error}",
                timeout.as_millis()
            )));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

struct DualProtocolReservation {
    tcp: TcpListener,
    udp: UdpSocket,
    address: SocketAddr,
}

impl DualProtocolReservation {
    fn reserve() -> Result<Self, ControlPlaneReferenceError> {
        for _ in 0..100 {
            let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|error| {
                ControlPlaneReferenceError::System(format!(
                    "unable to reserve loopback TCP port: {error}"
                ))
            })?;
            let address = tcp.local_addr().map_err(|error| {
                ControlPlaneReferenceError::System(format!(
                    "unable to inspect reserved TCP port: {error}"
                ))
            })?;
            match UdpSocket::bind(address) {
                Ok(udp) => {
                    return Ok(Self { tcp, udp, address });
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::AddrInUse | std::io::ErrorKind::PermissionDenied
                    ) => {}
                Err(error) => {
                    return Err(ControlPlaneReferenceError::System(format!(
                        "unable to reserve UDP alongside {address}: {error}"
                    )));
                }
            }
        }
        Err(ControlPlaneReferenceError::System(
            "unable to reserve a dual-protocol loopback port after 100 attempts".to_owned(),
        ))
    }

    fn address(&self) -> SocketAddr {
        debug_assert_eq!(self.tcp.local_addr().ok(), Some(self.address));
        debug_assert_eq!(self.udp.local_addr().ok(), Some(self.address));
        self.address
    }
}

struct ReferenceNode {
    config: DaemonNodeConfigReceipt,
    reservations: Option<Vec<DualProtocolReservation>>,
    child: Option<Child>,
    pid: Option<u32>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

/// One exact direct child that the shared process harness killed and reaped.
/// The W5 tier converts this OS wait result into its own sealed event receipt;
/// W4 remains unaware of W5 evidence types.
#[derive(Debug)]
pub struct WaitedDaemonProcess {
    pub process: DaemonNodeProcessReceipt,
    pub exit_status: ExitStatus,
}

impl ReferenceNode {
    fn endpoint(&self) -> ControlPlaneEndpoint {
        ControlPlaneEndpoint {
            node_id: self.config.launch_config.node_id.clone(),
            admin_addr: self.config.launch_config.admin_addr,
        }
    }

    fn release_reservations(&mut self) {
        self.reservations.take();
    }

    fn spawn(
        &mut self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<(), ControlPlaneReferenceError> {
        if self.reservations.is_some() || self.child.is_some() || self.pid.is_some() {
            return Err(ControlPlaneReferenceError::Contract(format!(
                "node {} must release reservations exactly once before one direct spawn",
                self.config.launch_config.node_id
            )));
        }
        context
            .verify_binaries_unchanged()
            .map_err(|error| ControlPlaneReferenceError::Contract(error.to_string()))?;
        let stdout = File::create(&self.stdout_path).map_err(system_io("create daemon stdout"))?;
        let stderr = File::create(&self.stderr_path).map_err(system_io("create daemon stderr"))?;
        let config = &self.config.launch_config;
        let mut command = Command::new(&context.server.canonical_path);
        command
            .current_dir(&context.repo_root)
            .env_clear()
            .env("HYDRACACHE_ROLE", "member")
            .env("HYDRACACHE_NODE_ID", &config.node_id)
            .env("HYDRACACHE_LISTEN_ADDR", config.client_addr.to_string())
            .env("HYDRACACHE_CLUSTER_ADDR", config.cluster_addr.to_string())
            .env(
                "HYDRACACHE_CLUSTER_ADVERTISE_ADDR",
                config.cluster_addr.to_string(),
            )
            .env("HYDRACACHE_ADMIN_API_ENABLED", "true")
            .env("HYDRACACHE_ADMIN_ADDR", config.admin_addr.to_string())
            .env("HYDRACACHE_REDIS_API_ENABLED", "false")
            .env("HYDRACACHE_CLUSTER_START", &config.cluster_start)
            .env(
                "HYDRACACHE_SEEDS",
                config
                    .seed_cluster_addrs
                    .iter()
                    .map(SocketAddr::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            )
            .env("HYDRACACHE_STORAGE_DIR", &config.storage_dir)
            .env("HYDRACACHE_JOIN_TIMEOUT_MS", "60000")
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        let child = command.spawn().map_err(|error| {
            ControlPlaneReferenceError::System(format!(
                "unable to execute prebuilt server {} directly for {}: {error}",
                context.server.canonical_path.display(),
                config.node_id
            ))
        })?;
        let pid = child.id();
        if pid == 0 {
            let mut child = child;
            let _ = child.kill();
            let _ = child.wait();
            return Err(ControlPlaneReferenceError::System(
                "direct daemon spawn returned PID 0".to_owned(),
            ));
        }
        self.pid = Some(pid);
        self.child = Some(child);
        Ok(())
    }

    fn kill_and_wait(
        &mut self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<WaitedDaemonProcess, ControlPlaneReferenceError> {
        let process = self.process_receipt(context)?;
        let mut child = self.child.take().ok_or_else(|| {
            ControlPlaneReferenceError::Lifecycle(format!(
                "node {} has no owned child to kill",
                process.node_id
            ))
        })?;
        match child
            .try_wait()
            .map_err(system_io("inspect daemon before kill"))?
        {
            None => {}
            Some(status) => {
                self.pid = None;
                return Err(ControlPlaneReferenceError::Lifecycle(format!(
                    "node {} PID {} exited before requested kill: {}",
                    process.node_id,
                    process.pid,
                    exit_status_text(&status)
                )));
            }
        }
        child.kill().map_err(system_io("kill exact daemon child"))?;
        let exit_status = child.wait().map_err(system_io("wait exact daemon child"))?;
        self.pid = None;
        Ok(WaitedDaemonProcess {
            process,
            exit_status,
        })
    }

    fn prepare_restart_logs(&mut self, run_root: &Path) {
        let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let node_id = &self.config.launch_config.node_id;
        self.stdout_path = run_root.join(format!("{node_id}-restart-{sequence}.stdout.log"));
        self.stderr_path = run_root.join(format!("{node_id}-restart-{sequence}.stderr.log"));
    }

    fn process_receipt(
        &self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<DaemonNodeProcessReceipt, ControlPlaneReferenceError> {
        let pid = self.pid.ok_or_else(|| {
            ControlPlaneReferenceError::Contract(format!(
                "node {} has no spawned PID receipt",
                self.config.launch_config.node_id
            ))
        })?;
        if self.child.is_none() {
            return Err(ControlPlaneReferenceError::Contract(format!(
                "node {} was reaped before capability sealing",
                self.config.launch_config.node_id
            )));
        }
        Ok(DaemonNodeProcessReceipt {
            node_id: self.config.launch_config.node_id.clone(),
            pid,
            direct_prebuilt_exec: true,
            observed_executable_path: context.server.canonical_path.clone(),
            observed_executable_sha256: context.server.sha256.clone(),
            config: self.config.clone(),
        })
    }

    fn stop(
        &mut self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<DaemonNodeLifecycleEvidence, String> {
        let pid = self.pid.ok_or_else(|| "node has no PID".to_owned())?;
        let mut child = self
            .child
            .take()
            .ok_or_else(|| format!("node {pid} has no owned child"))?;
        let mut problems = Vec::new();
        let was_running = match child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                problems.push(format!(
                    "daemon exited before lifecycle kill: {}",
                    exit_status_text(&status)
                ));
                false
            }
            Err(error) => {
                problems.push(format!("pre-kill status failed: {error}"));
                true
            }
        };
        let kill_requested = true;
        if let Err(error) = child.kill() {
            problems.push(format!("kill failed: {error}"));
        }
        let status = match child.wait() {
            Ok(status) => Some(status),
            Err(error) => {
                problems.push(format!("wait/reap failed: {error}"));
                None
            }
        };
        let wait_completed = status.is_some();
        let evidence = DaemonNodeLifecycleEvidence {
            node_id: self.config.launch_config.node_id.clone(),
            pid,
            kill_requested,
            wait_completed,
            process_no_longer_running: wait_completed,
            exit_status: status
                .as_ref()
                .map(exit_status_text)
                .unwrap_or_else(|| "unavailable".to_owned()),
            stdout_log: process_log_receipt(&self.stdout_path)
                .map_err(|error| error.to_string())?,
            stderr_log: process_log_receipt(&self.stderr_path)
                .map_err(|error| error.to_string())?,
            server_binary_path_after: context.server.canonical_path.clone(),
            server_binary_sha256_after: sha256_file(&context.server.canonical_path)
                .map_err(|error| error.to_string())?,
            node_config_path_after: self.config.canonical_path.clone(),
            node_config_sha256_after: sha256_file(&self.config.canonical_path)
                .map_err(|error| error.to_string())?,
        };
        if !was_running {
            problems.push("daemon was not running at cleanup boundary".to_owned());
        }
        if problems.is_empty() {
            Ok(evidence)
        } else {
            Err(format!(
                "node {} lifecycle failed: {}; evidence={evidence:?}",
                self.config.launch_config.node_id,
                problems.join("; ")
            ))
        }
    }
}

impl Drop for ReferenceNode {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

struct ReferenceControlPlaneCluster {
    run_root: PathBuf,
    nodes: Vec<ReferenceNode>,
    initial_count: usize,
}

impl ReferenceControlPlaneCluster {
    fn prepare(node_count: u8, evidence_root: &Path) -> Result<Self, ControlPlaneReferenceError> {
        let evidence_root = ensure_directory(evidence_root)?;
        let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        let run_root = evidence_root.join(format!(
            "w4a-control-plane-{}-{}-{sequence}",
            node_count,
            std::process::id()
        ));
        fs::create_dir(&run_root).map_err(system_io("create unique W4A run directory"))?;
        let run_root = fs::canonicalize(&run_root)
            .map_err(system_io("canonicalize unique W4A run directory"))?;
        let initial_count = usize::from(node_count) - 1;
        let mut reservations = Vec::with_capacity(usize::from(node_count));
        for _ in 0..node_count {
            reservations.push(vec![
                DualProtocolReservation::reserve()?,
                DualProtocolReservation::reserve()?,
                DualProtocolReservation::reserve()?,
            ]);
        }
        let cluster_addrs = reservations
            .iter()
            .map(|node| node[1].address())
            .collect::<Vec<_>>();
        let initial_cluster_addrs = cluster_addrs[..initial_count].to_vec();
        let mut nodes = Vec::with_capacity(usize::from(node_count));
        for (index, node_reservations) in reservations.into_iter().enumerate() {
            let node_id = member_node_id_for_addr(cluster_addrs[index]);
            let storage_dir = run_root.join(format!("node-{index}-storage"));
            fs::create_dir(&storage_dir).map_err(system_io("create node storage directory"))?;
            let storage_dir = fs::canonicalize(&storage_dir)
                .map_err(system_io("canonicalize node storage directory"))?;
            let cluster_start = if index < initial_count {
                "bootstrap"
            } else {
                "join"
            };
            let seed_cluster_addrs = if index < initial_count {
                initial_cluster_addrs
                    .iter()
                    .copied()
                    .filter(|address| *address != cluster_addrs[index])
                    .collect::<Vec<_>>()
            } else {
                initial_cluster_addrs.clone()
            };
            let launch_config = DaemonNodeLaunchConfig {
                receipt_kind: NODE_CONFIG_RECEIPT_KIND.to_owned(),
                node_id,
                client_addr: node_reservations[0].address(),
                cluster_addr: node_reservations[1].address(),
                admin_addr: node_reservations[2].address(),
                redis_addr: None,
                storage_dir,
                cluster_start: cluster_start.to_owned(),
                seed_cluster_addrs,
            };
            let config_path = run_root.join(format!("node-{index}-launch-config.json"));
            write_new_json(&config_path, &launch_config)?;
            let config_path = fs::canonicalize(&config_path)
                .map_err(system_io("canonicalize node launch config"))?;
            let config = DaemonNodeConfigReceipt {
                sha256: sha256_file(&config_path)?,
                canonical_path: config_path,
                launch_config,
            };
            nodes.push(ReferenceNode {
                config,
                reservations: Some(node_reservations),
                child: None,
                pid: None,
                stdout_path: run_root.join(format!("node-{index}.stdout.log")),
                stderr_path: run_root.join(format!("node-{index}.stderr.log")),
            });
        }
        Ok(Self {
            run_root,
            nodes,
            initial_count,
        })
    }

    fn spawn_initial(
        &mut self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<(), ControlPlaneReferenceError> {
        for node in &mut self.nodes[..self.initial_count] {
            node.release_reservations();
            node.spawn(context)?;
        }
        Ok(())
    }

    fn spawn_joiner(
        &mut self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<(), ControlPlaneReferenceError> {
        let joiner = &mut self.nodes[self.initial_count];
        joiner.release_reservations();
        joiner.spawn(context)
    }

    fn initial_endpoints(&self) -> Vec<ControlPlaneEndpoint> {
        self.nodes[..self.initial_count]
            .iter()
            .map(ReferenceNode::endpoint)
            .collect()
    }

    fn full_endpoints(&self) -> Vec<ControlPlaneEndpoint> {
        self.nodes.iter().map(ReferenceNode::endpoint).collect()
    }

    fn joiner_node_id(&self) -> &str {
        &self.nodes[self.initial_count].config.launch_config.node_id
    }

    fn write_add_action_receipt(
        &self,
        authority_node_id: &str,
        context: &ValidatedRespReferenceContext,
    ) -> Result<DaemonAddInvocationReceipt, ControlPlaneReferenceError> {
        self.write_add_action_receipt_for(
            self.initial_count,
            authority_node_id,
            context,
            "daemon-add-action.json",
        )
    }

    fn write_add_action_receipt_for(
        &self,
        target_index: usize,
        authority_node_id: &str,
        context: &ValidatedRespReferenceContext,
        file_name: &str,
    ) -> Result<DaemonAddInvocationReceipt, ControlPlaneReferenceError> {
        let target = self.nodes.get(target_index).ok_or_else(|| {
            ControlPlaneReferenceError::Contract(format!(
                "add-action target index {target_index} is absent"
            ))
        })?;
        let payload = DaemonAddActionPayload {
            receipt_kind: ADD_ACTION_RECEIPT_KIND.to_owned(),
            provisioner: DAEMON_CLUSTER_PROVISIONER.to_owned(),
            authority_node_id: authority_node_id.to_owned(),
            target_node_id: target.config.launch_config.node_id.clone(),
            outcome: "process-started-and-admission-requested".to_owned(),
        };
        let path = self.run_root.join(file_name);
        write_new_json(&path, &payload)?;
        let path =
            fs::canonicalize(path).map_err(system_io("canonicalize daemon add action receipt"))?;
        Ok(DaemonAddInvocationReceipt {
            action_receipt_sha256: sha256_file(&path)?,
            canonical_action_receipt_path: path,
            payload,
            target_process: target.process_receipt(context)?,
        })
    }

    fn prepare_transient_joiner(&mut self) -> Result<usize, ControlPlaneReferenceError> {
        let index = self.nodes.len();
        let reservations = vec![
            DualProtocolReservation::reserve()?,
            DualProtocolReservation::reserve()?,
            DualProtocolReservation::reserve()?,
        ];
        let cluster_addr = reservations[1].address();
        let node_id = member_node_id_for_addr(cluster_addr);
        if self
            .nodes
            .iter()
            .any(|node| node.config.launch_config.node_id == node_id)
        {
            return Err(ControlPlaneReferenceError::Contract(format!(
                "transient member id {node_id} collides with an existing node"
            )));
        }
        let storage_dir = self.run_root.join(format!("node-{index}-storage"));
        fs::create_dir(&storage_dir).map_err(system_io("create transient node storage"))?;
        let storage_dir = fs::canonicalize(&storage_dir)
            .map_err(system_io("canonicalize transient node storage"))?;
        let seed_cluster_addrs = self
            .nodes
            .iter()
            .filter(|node| node.child.is_some())
            .map(|node| node.config.launch_config.cluster_addr)
            .collect::<Vec<_>>();
        if seed_cluster_addrs.is_empty() {
            return Err(ControlPlaneReferenceError::Contract(
                "transient member requires at least one live seed".to_owned(),
            ));
        }
        let launch_config = DaemonNodeLaunchConfig {
            receipt_kind: NODE_CONFIG_RECEIPT_KIND.to_owned(),
            node_id,
            client_addr: reservations[0].address(),
            cluster_addr,
            admin_addr: reservations[2].address(),
            redis_addr: None,
            storage_dir,
            cluster_start: "join".to_owned(),
            seed_cluster_addrs,
        };
        let config_path = self
            .run_root
            .join(format!("node-{index}-launch-config.json"));
        write_new_json(&config_path, &launch_config)?;
        let config_path = fs::canonicalize(&config_path)
            .map_err(system_io("canonicalize transient launch config"))?;
        let config = DaemonNodeConfigReceipt {
            sha256: sha256_file(&config_path)?,
            canonical_path: config_path,
            launch_config,
        };
        self.nodes.push(ReferenceNode {
            config,
            reservations: Some(reservations),
            child: None,
            pid: None,
            stdout_path: self
                .run_root
                .join(format!("node-{index}-transient.stdout.log")),
            stderr_path: self
                .run_root
                .join(format!("node-{index}-transient.stderr.log")),
        });
        Ok(index)
    }

    fn stop_all_unsealed(
        &mut self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<Vec<DaemonNodeLifecycleEvidence>, ControlPlaneReferenceError> {
        let mut evidence = Vec::with_capacity(self.nodes.len());
        let mut problems = Vec::new();
        for node in &mut self.nodes {
            if node.child.is_none() && node.pid.is_none() {
                // A fault-oriented caller already retained the exact wait
                // status for this node. W4 never takes this branch.
                continue;
            }
            match node.stop(context) {
                Ok(node_evidence) => evidence.push(node_evidence),
                Err(error) => problems.push(error),
            }
        }
        if let Err(error) = context.verify_binaries_unchanged() {
            problems.push(format!(
                "post-run prebuilt binary verification failed: {error}"
            ));
        }
        if problems.is_empty() {
            Ok(evidence)
        } else {
            Err(ControlPlaneReferenceError::Lifecycle(problems.join("; ")))
        }
    }

    fn seal_capability(
        &self,
        context: &ValidatedRespReferenceContext,
    ) -> Result<ControlPlaneCapabilityAttestation, ControlPlaneReferenceError> {
        context
            .verify_binaries_unchanged()
            .map_err(|error| ControlPlaneReferenceError::Contract(error.to_string()))?;
        let nodes = self
            .nodes
            .iter()
            .map(|node| node.process_receipt(context))
            .collect::<Result<Vec<_>, _>>()?;
        let payload = ControlPlaneCapabilityReceiptPayload {
            receipt_kind: DAEMON_CAPABILITY_RECEIPT_KIND.to_owned(),
            receipt_source: DaemonReceiptSource::ObservedProcessHarness,
            execution_mode: CONTROL_PLANE_EXECUTION_MODE.to_owned(),
            profile: context.profile.name.clone(),
            source_commit: context.source.git_commit.clone(),
            runner_fingerprint_sha256: sha256_bytes(context.runner.fingerprint.as_bytes()),
            prebuild_manifest_canonical_path: context.manifest_path.clone(),
            prebuild_manifest_sha256: context.manifest_sha256.clone(),
            prebuild_contract_sha256: context.build.prebuild_contract_digest.clone(),
            provisioner: DAEMON_CLUSTER_PROVISIONER.to_owned(),
            direct_prebuilt_exec: true,
            server_binary: PrebuiltServerBinaryReceipt {
                canonical_path: context.server.canonical_path.clone(),
                sha256: context.server.sha256.clone(),
            },
            node_count: u8::try_from(nodes.len()).map_err(|_| {
                ControlPlaneReferenceError::Contract("node count exceeds u8".to_owned())
            })?,
            nodes,
        };
        ControlPlaneCapabilityAttestation::seal(payload).map_err(Into::into)
    }

    fn stop_all(
        &mut self,
        context: &ValidatedRespReferenceContext,
        capability_receipt_sha256: &str,
    ) -> Result<ControlPlaneLifecycleReceipt, ControlPlaneReferenceError> {
        let evidence = self.stop_all_unsealed(context)?;
        ControlPlaneLifecycleReceipt::seal(ControlPlaneLifecycleReceiptPayload {
            receipt_kind: LIFECYCLE_RECEIPT_KIND.to_owned(),
            receipt_source: DaemonReceiptSource::ObservedProcessHarness,
            capability_receipt_sha256: capability_receipt_sha256.to_owned(),
            nodes: evidence,
        })
        .map_err(Into::into)
    }
}

impl Drop for ReferenceControlPlaneCluster {
    fn drop(&mut self) {
        for node in &mut self.nodes {
            if let Some(mut child) = node.child.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn ensure_directory(path: &Path) -> Result<PathBuf, ControlPlaneReferenceError> {
    fs::create_dir_all(path).map_err(system_io("create W4A evidence root"))?;
    let canonical = fs::canonicalize(path).map_err(system_io("canonicalize W4A evidence root"))?;
    if !fs::metadata(&canonical)
        .map_err(system_io("stat W4A evidence root"))?
        .is_dir()
    {
        return Err(ControlPlaneReferenceError::Contract(format!(
            "W4A evidence root is not a directory: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn member_node_id_for_addr(address: SocketAddr) -> String {
    let suffix = address
        .to_string()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("member-{suffix}")
}

fn write_new_json<T: Serialize>(path: &Path, value: &T) -> Result<(), ControlPlaneReferenceError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        ControlPlaneReferenceError::System(format!(
            "unable to serialize strict JSON {}: {error}",
            path.display()
        ))
    })?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(system_io("create strict JSON artifact"))?;
    file.write_all(&bytes)
        .map_err(system_io("write strict JSON artifact"))?;
    file.sync_all()
        .map_err(system_io("sync strict JSON artifact"))
}

fn write_new_report(
    path: &Path,
    report: &ControlPlaneReport,
) -> Result<(), ControlPlaneReferenceError> {
    if !path.is_absolute() {
        return Err(ControlPlaneReferenceError::Contract(format!(
            "W4A report path must be absolute: {}",
            path.display()
        )));
    }
    let parent = path.parent().ok_or_else(|| {
        ControlPlaneReferenceError::Contract("W4A report path has no parent".to_owned())
    })?;
    fs::create_dir_all(parent).map_err(system_io("create W4A report parent"))?;
    if path.exists() {
        return Err(ControlPlaneReferenceError::Contract(format!(
            "refusing to overwrite existing W4A report {}",
            path.display()
        )));
    }
    let bytes = serde_json::to_vec_pretty(report).map_err(|error| {
        ControlPlaneReferenceError::System(format!(
            "unable to serialize W4A report {}: {error}",
            path.display()
        ))
    })?;
    let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ControlPlaneReferenceError::Contract(
                "W4A report file name must be valid UTF-8".to_owned(),
            )
        })?;
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(system_io("create temporary W4A report"))?;
        file.write_all(&bytes)
            .map_err(system_io("write temporary W4A report"))?;
        file.sync_all()
            .map_err(system_io("sync temporary W4A report"))?;
        fs::rename(&temporary, path).map_err(system_io("atomically land W4A report"))
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

fn process_log_receipt(path: &Path) -> Result<ProcessLogReceipt, ControlPlaneReferenceError> {
    let canonical = fs::canonicalize(path).map_err(system_io("canonicalize daemon log"))?;
    let metadata = fs::metadata(&canonical).map_err(system_io("stat daemon log"))?;
    if !metadata.is_file() {
        return Err(ControlPlaneReferenceError::Lifecycle(format!(
            "daemon log is not a regular file: {}",
            canonical.display()
        )));
    }
    Ok(ProcessLogReceipt {
        sha256: sha256_file(&canonical)?,
        canonical_path: canonical,
        bytes: metadata.len(),
    })
}

fn exit_status_text(status: &ExitStatus) -> String {
    status.code().map_or_else(
        || "terminated-without-exit-code".to_owned(),
        |code| format!("exit:{code}"),
    )
}

fn sha256_file(path: &Path) -> Result<String, ControlPlaneReferenceError> {
    let bytes = fs::read(path).map_err(system_io("read artifact for SHA-256"))?;
    Ok(sha256_bytes(&bytes))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn system_io(context: &'static str) -> impl FnOnce(std::io::Error) -> ControlPlaneReferenceError {
    move |error| ControlPlaneReferenceError::System(format!("{context}: {error}"))
}
