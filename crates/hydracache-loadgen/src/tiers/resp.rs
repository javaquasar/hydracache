//! Release-0.67 W3 characterization of one selected RESP endpoint.
//!
//! The fast path below is deliberately a loopback product-facade fixture. It
//! proves TCP/parser/open-loop plumbing and can never be promoted to daemon
//! capacity evidence. The reference entry point fails closed until W7 supplies
//! a receipt-bound prebuilt `hydracache-server` process context.

use std::collections::{BTreeMap, BTreeSet};
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use hydracache_cache_sim::{
    GeneratedKeySchedule, KeyDistribution, KeyScheduleSpec, KEY_SCHEDULE_GENERATOR_VERSION,
};
use hydracache_client_transport_axum::{ClientSurfaceLimits, ClientSurfaceState};
use hydracache_redis_compat::{RedisListenerConfig, RedisRespServer, DEFAULT_REDIS_NAMESPACE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};

use crate::report::{
    derived_identity_digest, BuildIdentity, DimensionValue, EvidenceRunMode,
    KeyDistributionIdentity, LoadClaim, LoadCurveEvidence, MeasurementEvidence, PerfReport,
    Quantity, RespEndpointCapability, ScalarEvidence, ScalarPoint, SourceIdentity, SurfaceIdentity,
    WeightedOperation, WeightedPayload, WorkloadIdentity,
};
use crate::resp_external::{
    run_redis_benchmark, ExternalToolPrebuildReceipt, ExternalToolProvenanceRegistry,
    ExternalToolRunOutcome, MissingToolPolicy, RedisBenchmarkContract, RedisBenchmarkEndpoint,
    RedisBenchmarkEvidence, RedisBenchmarkRunContext, RespOpenLoopEndpointCapability,
    SelectedDaemonReceipt, SelectedDaemonReceiptPayload, SystemToolExecutor,
    SELECTED_DAEMON_RECEIPT_VERSION,
};
use crate::runner::run_scenario;
use crate::scenario::Scenario;
use crate::target::{Target, TargetError};
use crate::targets::resp::{
    Resp2Limits, RespEndpointIdentity, RespOperationMix, RespTargetConfig, RespTcpEvidence,
    RespTcpTarget,
};
use crate::{
    run_open_loop, KneeResult, OpenLoopConfig, OpenLoopObservation, PerformanceProfile,
    RunnerFingerprint,
};

use super::resp_reference::{
    ReferencePrerequisites, RespDaemonEvidence, RespDaemonFixture, ValidatedRespReferenceContext,
    RESP_PING_FRAME, RESP_PONG_DISPLAY, RESP_PONG_FRAME, SERVER_BINARY_ID,
};

pub const RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH: &str =
    "target/test-evidence/0.67/resp-reference-run-inputs.json";

const SMOKE_KEY_COUNT: u64 = 32;
const SMOKE_PRELOAD: u64 = 16;
const SMOKE_OPERATIONS: u64 = 100;
const SMOKE_REPEATS: u32 = 3;
const SMOKE_SPREAD_LIMIT: f64 = 1_000.0;

const WORKLOAD_A_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/resp-open-loop-a-v1.toml");
const WORKLOAD_B_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/resp-open-loop-b-v1.toml");
const WORKLOAD_C_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/resp-open-loop-c-v1.toml");
const MATRIX_SCENARIO: &[u8] =
    include_bytes!("../../../../docs/testing/perf-scenarios/0.67/resp-connection-pipeline-v1.toml");

/// Required W3 measurements in the coordinated-omission-safe report. The
/// external closed-loop result is emitted to a different artifact and type.
pub const REQUIRED_RESP_OPEN_LOOP_MEASUREMENTS: [&str; 3] = [
    "resp_open_loop_get_set_knee_at_slo",
    "resp_open_loop_connection_and_pipeline_sweeps",
    "resp_open_loop_stall_is_visible_in_scheduled_latency",
];

#[derive(Debug, thiserror::Error)]
pub enum RespTierError {
    #[error(transparent)]
    Target(#[from] TargetError),
    #[error("RESP tier runtime failed: {0}")]
    Runtime(String),
    #[error("RESP tier report failed: {0}")]
    Report(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub struct RespReferenceSuiteEvidence {
    pub open_loop: PerfReport,
    pub external: RedisBenchmarkEvidence,
    pub daemon: RespDaemonEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespReferenceSuiteReceiptPayload {
    pub schema_version: u32,
    pub source_commit: String,
    pub prebuild_manifest_sha256: String,
    pub selected_endpoint: String,
    pub endpoint_capability_sha256: String,
    pub open_loop_report_sha256: String,
    pub external_report_sha256: String,
    pub daemon_lifecycle_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespReferenceSuiteReceipt {
    pub payload: RespReferenceSuiteReceiptPayload,
    pub receipt_sha256: String,
}

impl RespReferenceSuiteReceipt {
    pub fn seal(
        evidence: &RespReferenceSuiteEvidence,
        open_loop_bytes: &[u8],
        external_bytes: &[u8],
        lifecycle_bytes: &[u8],
    ) -> Result<Self, RespTierError> {
        let capability = evidence
            .open_loop
            .resp_endpoint_capability
            .as_ref()
            .ok_or_else(|| {
                RespTierError::Report(
                    "reference suite open-loop report lost its typed endpoint capability"
                        .to_owned(),
                )
            })?;
        let endpoint_capability_sha256 = capability
            .digest()
            .map_err(|error| RespTierError::Report(error.to_string()))?;
        let payload = RespReferenceSuiteReceiptPayload {
            schema_version: 1,
            source_commit: evidence.open_loop.source.git_commit.clone(),
            prebuild_manifest_sha256: evidence.open_loop.build.prebuild_manifest_sha256.clone(),
            selected_endpoint: capability.selected_endpoint.clone(),
            endpoint_capability_sha256,
            open_loop_report_sha256: digest_bytes(open_loop_bytes),
            external_report_sha256: digest_bytes(external_bytes),
            daemon_lifecycle_sha256: digest_bytes(lifecycle_bytes),
        };
        let receipt_sha256 = digest_bytes(
            &serde_json::to_vec(&payload)
                .map_err(|error| RespTierError::Report(error.to_string()))?,
        );
        let receipt = Self {
            payload,
            receipt_sha256,
        };
        receipt.validate(evidence, open_loop_bytes, external_bytes, lifecycle_bytes)?;
        Ok(receipt)
    }

    pub fn validate(
        &self,
        evidence: &RespReferenceSuiteEvidence,
        open_loop_bytes: &[u8],
        external_bytes: &[u8],
        lifecycle_bytes: &[u8],
    ) -> Result<(), RespTierError> {
        let capability = evidence
            .open_loop
            .resp_endpoint_capability
            .as_ref()
            .ok_or_else(|| RespTierError::Report("typed W3 capability is absent".to_owned()))?;
        let capability_digest = capability
            .digest()
            .map_err(|error| RespTierError::Report(error.to_string()))?;
        let external_context = &evidence.external.run_context;
        let expected_receipt = digest_bytes(
            &serde_json::to_vec(&self.payload)
                .map_err(|error| RespTierError::Report(error.to_string()))?,
        );
        let exact = self.payload.schema_version == 1
            && self.payload.source_commit == evidence.open_loop.source.git_commit
            && self.payload.prebuild_manifest_sha256
                == evidence.open_loop.build.prebuild_manifest_sha256
            && external_context.source == evidence.open_loop.source
            && external_context.build == evidence.open_loop.build
            && self.payload.selected_endpoint == capability.selected_endpoint
            && self.payload.endpoint_capability_sha256 == capability_digest
            && external_context.open_loop_endpoint.endpoint.host
                == capability.config.redis_addr.ip().to_string()
            && external_context.open_loop_endpoint.endpoint.port
                == capability.config.redis_addr.port()
            && external_context
                .open_loop_endpoint
                .endpoint_capability_sha256
                == capability_digest
            && external_context.selected_daemon.payload.endpoint
                == external_context.open_loop_endpoint.endpoint
            && external_context
                .selected_daemon
                .payload
                .prebuild_manifest_sha256
                == capability.prebuild_manifest_sha256
            && external_context
                .selected_daemon
                .payload
                .daemon_binary_sha256
                == capability.server_binary_sha256
            && external_context
                .selected_daemon
                .payload
                .open_loop_endpoint_capability_sha256
                == capability_digest
            && external_evidence_is_ship_eligible(&evidence.external)
            && lifecycle_matches_capability(capability, &evidence.daemon, &capability_digest)
            && self.payload.open_loop_report_sha256 == digest_bytes(open_loop_bytes)
            && self.payload.external_report_sha256 == digest_bytes(external_bytes)
            && self.payload.daemon_lifecycle_sha256 == digest_bytes(lifecycle_bytes)
            && self.receipt_sha256 == expected_receipt;
        if !exact {
            return Err(RespTierError::Report(
                "W3 suite receipt does not cross-bind the exact open-loop, external-tool, lifecycle, and endpoint-capability evidence"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

fn external_ship_status_is_eligible(
    measurements_stable: bool,
    ship_evidence_eligible: bool,
    stability_reasons: &[String],
) -> bool {
    measurements_stable && ship_evidence_eligible && stability_reasons.is_empty()
}

pub fn external_evidence_is_ship_eligible(evidence: &RedisBenchmarkEvidence) -> bool {
    external_ship_status_is_eligible(
        evidence.measurements_stable,
        evidence.ship_evidence_eligible,
        &evidence.stability_reasons,
    )
}

fn lifecycle_matches_capability(
    capability: &RespEndpointCapability,
    daemon: &RespDaemonEvidence,
    capability_digest: &str,
) -> bool {
    daemon.endpoint_capability_digest == capability_digest
        && daemon.selected_endpoint == capability.selected_endpoint
        && daemon.direct_prebuilt_exec
        && daemon.pid == capability.pid
        && daemon.repeat_index == capability.repeat_index
        && daemon.resp_endpoint == capability.config.redis_addr
        && daemon.admin_endpoint == capability.config.admin_addr
        && daemon.data_dir == capability.config.storage_dir
        && daemon.server_binary_sha256 == capability.server_binary_sha256
        && daemon.loadgen_binary_sha256 == capability.loadgen_binary_sha256
        && daemon.readiness.selected_endpoint == daemon.resp_endpoint
        && daemon.readiness.attempts > 0
        && daemon.readiness.exact_response == RESP_PONG_DISPLAY
        && daemon.readiness.request_sha256 == digest_bytes(RESP_PING_FRAME)
        && daemon.readiness.response_sha256 == digest_bytes(RESP_PONG_FRAME)
        && daemon.binaries_verified_after_measurement
        && daemon.killed_and_waited
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RespReferenceRunInputs {
    pub prerequisites: ReferencePrerequisites,
    pub external_tool_prebuild: ExternalToolPrebuildReceipt,
}

impl RespReferenceRunInputs {
    pub fn load(repo_root: &Path) -> Result<Self, RespTierError> {
        let path = repo_root.join(RESP_REFERENCE_RUN_INPUTS_RELATIVE_PATH);
        let bytes = std::fs::read(&path).map_err(|error| {
            RespTierError::Report(format!(
                "reference run inputs {} are unavailable: {error}",
                path.display()
            ))
        })?;
        if bytes.is_empty() || bytes.len() > 1024 * 1024 {
            return Err(RespTierError::Report(format!(
                "reference run inputs {} must be a bounded non-empty JSON file",
                path.display()
            )));
        }
        serde_json::from_slice(&bytes).map_err(|error| {
            RespTierError::Report(format!(
                "reference run inputs {} are invalid: {error}",
                path.display()
            ))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationInput {
    operation: String,
    weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RespInputs {
    workload: String,
    key_count: u64,
    payload_bytes: u64,
    batch_size: usize,
    connections: usize,
    pipeline: usize,
    repeat_isolation: String,
    daemon_reused_across_repeats: bool,
    operation_mix: Vec<OperationInput>,
}

#[derive(Debug, Clone)]
struct BoundRespScenario {
    scenario: Scenario,
    resp: RespInputs,
    source_digest: String,
}

#[derive(Debug, Clone)]
struct RespRunEndpoint {
    identity: RespEndpointIdentity,
    capability_digest: String,
}

#[derive(Debug, Clone, Copy)]
struct RespTargetShape {
    preload_entries: u64,
    key_space: u64,
    connections: usize,
    pipeline_depth: usize,
    injected_dispatch_delay: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RespRunShape {
    Smoke,
    Reference,
}

impl RespRunShape {
    fn scenario(self, binding: &BoundRespScenario) -> Result<Scenario, RespTierError> {
        match self {
            Self::Smoke => smoke_scenario(binding),
            Self::Reference => {
                binding
                    .scenario
                    .validate()
                    .map_err(RespTierError::Runtime)?;
                Ok(binding.scenario.clone())
            }
        }
    }

    fn key_count(self, binding: &BoundRespScenario) -> u64 {
        match self {
            Self::Smoke => SMOKE_KEY_COUNT,
            Self::Reference => binding.resp.key_count,
        }
    }

    fn schedule(
        self,
        binding: &BoundRespScenario,
        scenario: &Scenario,
    ) -> Result<GeneratedKeySchedule, RespTierError> {
        KeyScheduleSpec::uniform(
            scenario.seed,
            self.key_count(binding),
            scenario
                .steady_operations
                .max(scenario.warmup_operations)
                .max(scenario.preload_operations),
        )
        .generate()
        .map_err(RespTierError::Runtime)
    }
}

impl RespRunEndpoint {
    fn fixture(address: SocketAddr) -> Self {
        Self {
            identity: RespEndpointIdentity {
                address,
                selected_endpoint: format!("resp-loopback-fixture@{address}"),
                endpoint_kind: "resp-loopback-fixture".to_owned(),
                state_scope: "fixture-process-local".to_owned(),
            },
            capability_digest: fixture_capability_digest(address),
        }
    }

    fn reference(
        identity: RespEndpointIdentity,
        capability_digest: String,
    ) -> Result<Self, RespTierError> {
        if identity.endpoint_kind != "node-resp"
            || identity.state_scope != "node-local"
            || !identity.address.ip().is_loopback()
            || !identity.selected_endpoint.starts_with("hydracache-server@")
            || !is_lower_sha256(&capability_digest)
        {
            return Err(RespTierError::Report(
                "reference RESP endpoint identity/capability is incomplete or overclaims its surface"
                    .to_owned(),
            ));
        }
        Ok(Self {
            identity,
            capability_digest,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatrixInput {
    schema_version: u32,
    id: String,
    seed: u64,
    operations_per_repeat: u64,
    repeats: usize,
    preload_entries: u64,
    key_count: u64,
    payload_bytes: usize,
    batch_size: usize,
    connections: Vec<usize>,
    pipelines: Vec<usize>,
    metric: String,
    methodology: String,
    state_scope: String,
    selected_endpoint_only: bool,
    repeat_isolation: String,
    daemon_reused_across_repeats: bool,
    robust_spread_tolerance: f64,
}

/// In-process owner of a real loopback TCP listener backed by the product RESP
/// facade. Its identity is fixture-only; it is not a daemon substitute.
struct LoopbackRespFixture {
    address: SocketAddr,
    shutdown: watch::Sender<bool>,
    task: Option<JoinHandle<()>>,
}

impl LoopbackRespFixture {
    async fn start() -> Result<Self, RespTierError> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let address = listener.local_addr()?;
        let state = Arc::new(
            ClientSurfaceState::new(ClientSurfaceLimits::default())
                .map_err(|error| RespTierError::Runtime(error.to_string()))?,
        );
        let server = Arc::new(
            RedisRespServer::new(
                state,
                RedisListenerConfig {
                    tenant: DEFAULT_REDIS_NAMESPACE.to_owned(),
                    ..RedisListenerConfig::default()
                },
            )
            .map_err(|error| RespTierError::Runtime(error.to_string()))?,
        );
        let (shutdown, mut receiver) = watch::channel(false);
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    changed = receiver.changed() => {
                        if changed.is_err() || *receiver.borrow() {
                            break;
                        }
                    }
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break };
                        let server = Arc::clone(&server);
                        connections.spawn(async move {
                            let _ = server.serve_connection(stream).await;
                        });
                    }
                    joined = connections.join_next(), if !connections.is_empty() => {
                        let _ = joined;
                    }
                }
            }
            connections.abort_all();
            while connections.join_next().await.is_some() {}
        });
        Ok(Self {
            address,
            shutdown,
            task: Some(task),
        })
    }

    fn address(&self) -> SocketAddr {
        self.address
    }

    async fn stop(mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for LoopbackRespFixture {
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

/// Build the complete short W3 open-loop report against a real loopback TCP
/// boundary while retaining a fixture-only surface claim.
pub async fn resp_smoke_report(profile_name: &str) -> Result<PerfReport, RespTierError> {
    if profile_name != "smoke-v1" {
        return Err(RespTierError::Report(format!(
            "profile {profile_name:?} cannot be attached to fixture-only RESP smoke evidence"
        )));
    }
    let fixture = LoopbackRespFixture::start().await?;
    let result = resp_smoke_report_on(fixture.address(), profile_name).await;
    fixture.stop().await;
    result
}

/// Measure one already-started receipt-bound daemon. The caller owns the
/// lifecycle so the exact same endpoint capability can also be consumed by
/// the supplemental external-tool leg before it is stopped and hashed.
pub async fn resp_reference_report_on(
    context: &ValidatedRespReferenceContext,
    daemon: &RespDaemonFixture,
) -> Result<PerfReport, RespTierError> {
    let endpoint = RespRunEndpoint::reference(
        daemon.endpoint_identity(),
        daemon
            .endpoint_capability_digest()
            .map_err(|error| RespTierError::Report(error.to_string()))?,
    )?;
    resp_reference_report_for_endpoint(context, &endpoint, daemon.endpoint_capability()).await
}

async fn resp_reference_report_for_endpoint(
    context: &ValidatedRespReferenceContext,
    endpoint: &RespRunEndpoint,
    capability: &RespEndpointCapability,
) -> Result<PerfReport, RespTierError> {
    let mut measurements =
        resp_knee_measurements(endpoint, Duration::ZERO, RespRunShape::Reference).await?;
    measurements.push(resp_matrix_measurement(endpoint, RespRunShape::Reference).await?);
    measurements.push(resp_stall_visibility_measurement(endpoint).await?);
    let report = PerfReport::new(
        "node-resp-open-loop-reference-v1",
        "resp-w3-open-loop-reference-v1",
        "derived-from-reference-measurements",
        EvidenceRunMode::ReferenceEvidence,
        context.surface.clone(),
        context.profile.clone(),
        context.runner.clone(),
        context.source.clone(),
        context.build.clone(),
        Some(capability.clone()),
        measurements,
        Vec::new(),
    );
    report.to_pretty_json().map_err(|error| {
        RespTierError::Report(format!(
            "reference report failed canonical validation: {error}; problems={:?}",
            report.validation_problems()
        ))
    })?;
    Ok(report)
}

/// Run both W3 artifacts against one owned daemon capability, then always reap
/// that process and retain lifecycle evidence. A mandatory reference suite may
/// neither skip the external tool nor substitute a second RESP endpoint.
pub async fn run_resp_reference_suite(
    context: ValidatedRespReferenceContext,
    daemon: RespDaemonFixture,
    mut external_contract: RedisBenchmarkContract,
    provenance_registry: ExternalToolProvenanceRegistry,
    external_tool_prebuild: ExternalToolPrebuildReceipt,
) -> Result<RespReferenceSuiteEvidence, RespTierError> {
    let committed_contract_sha256 = external_contract.committed_digest();
    external_contract.endpoint = RedisBenchmarkEndpoint {
        host: "127.0.0.1".to_owned(),
        port: daemon.resp_endpoint().port(),
    };
    external_contract
        .validate()
        .map_err(|error| RespTierError::Report(error.to_string()))?;
    let capability_digest = daemon
        .endpoint_capability_digest()
        .map_err(|error| RespTierError::Report(error.to_string()))?;
    let endpoint = external_contract.endpoint.clone();
    let selected_daemon = SelectedDaemonReceipt::seal(SelectedDaemonReceiptPayload {
        schema_version: SELECTED_DAEMON_RECEIPT_VERSION,
        node_id: "local-reference-daemon".to_owned(),
        endpoint: endpoint.clone(),
        daemon_binary_id: SERVER_BINARY_ID.to_owned(),
        daemon_binary_sha256: context.server.sha256.clone(),
        prebuild_manifest_sha256: context.manifest_sha256.clone(),
        open_loop_endpoint_capability_sha256: capability_digest.clone(),
        capability_source: "real-daemon-resp-readiness".to_owned(),
        daemon_processes: true,
        resp_listener_capability: true,
        state_scope: "node-local".to_owned(),
        selected_endpoint_only: true,
        automatic_failover: false,
    });
    let run_context = RedisBenchmarkRunContext {
        runner_profile: context.profile.clone(),
        observed_runner: context.runner.clone(),
        source: context.source.clone(),
        build: context.build.clone(),
        open_loop_endpoint: RespOpenLoopEndpointCapability {
            endpoint,
            endpoint_capability_sha256: capability_digest,
        },
        selected_daemon,
        external_tool_prebuild,
        committed_contract_sha256,
    };

    let measurement = async {
        let open_loop = resp_reference_report_on(&context, &daemon).await?;
        let external = tokio::task::spawn_blocking(move || {
            run_redis_benchmark(
                &external_contract,
                MissingToolPolicy::MandatoryFailClosed,
                &SystemToolExecutor,
                &provenance_registry,
                &run_context,
            )
        })
        .await
        .map_err(|error| RespTierError::Runtime(format!("redis-benchmark worker failed: {error}")))?
        .map_err(|error| RespTierError::Report(error.to_string()))?;
        let ExternalToolRunOutcome::Completed(external) = external else {
            return Err(RespTierError::Report(
                "mandatory reference redis-benchmark run skipped instead of failing closed"
                    .to_owned(),
            ));
        };
        if !external_evidence_is_ship_eligible(&external) {
            return Err(RespTierError::Report(format!(
                "mandatory reference redis-benchmark run is unstable and cannot be promoted: {:?}",
                external.stability_reasons
            )));
        }
        Ok::<_, RespTierError>((open_loop, *external))
    }
    .await;
    let lifecycle = daemon
        .stop()
        .await
        .map_err(|error| RespTierError::Runtime(error.to_string()));
    match (measurement, lifecycle) {
        (Ok((open_loop, external)), Ok(daemon)) => Ok(RespReferenceSuiteEvidence {
            open_loop,
            external,
            daemon,
        }),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(measurement), Err(lifecycle)) => Err(RespTierError::Runtime(format!(
            "reference measurement failed ({measurement}); daemon cleanup also failed ({lifecycle})"
        ))),
    }
}

async fn resp_smoke_report_on(
    endpoint: SocketAddr,
    profile_name: &str,
) -> Result<PerfReport, RespTierError> {
    let endpoint = RespRunEndpoint::fixture(endpoint);
    let mut measurements =
        resp_knee_measurements(&endpoint, Duration::ZERO, RespRunShape::Smoke).await?;
    measurements.push(resp_matrix_measurement(&endpoint, RespRunShape::Smoke).await?);
    measurements.push(resp_stall_visibility_measurement(&endpoint).await?);
    let state_digest = measurements
        .iter()
        .find_map(|measurement| match measurement {
            MeasurementEvidence::LoadCurve(curve) => curve
                .knee
                .as_ref()
                .and_then(|knee| knee.evaluated.first())
                .and_then(|point| point.repeats.first())
                .map(|repeat| repeat.state_digest.clone()),
            _ => None,
        })
        .ok_or_else(|| RespTierError::Report("W3 report has no state digest".to_owned()))?;
    let fingerprint = smoke_fingerprint();
    let profile = smoke_profile(profile_name, &fingerprint);
    let report = PerfReport::new(
        "resp-loopback-fixture-smoke",
        "resp-w3-open-loop-fixture-smoke",
        state_digest,
        EvidenceRunMode::Smoke,
        SurfaceIdentity {
            surface_kind: "resp-loopback-fixture".to_owned(),
            execution_mode: "in-process-product-resp-listener".to_owned(),
            state_scope: "fixture-process-local".to_owned(),
            network_boundary: "loopback-tcp".to_owned(),
            claim_scope: "plumbing-only".to_owned(),
        },
        profile,
        fingerprint,
        SourceIdentity {
            git_commit: "smoke-unclaimed-working-tree".to_owned(),
            cargo_lock_sha256: digest_bytes(include_bytes!("../../../../Cargo.lock")),
            toolchain: "smoke-current-toolchain".to_owned(),
            build_flags: vec!["smoke-debug".to_owned()],
        },
        BuildIdentity {
            prebuild_contract_digest: "smoke-no-prebuild-contract".to_owned(),
            prebuild_manifest_sha256: "smoke-no-prebuild-manifest".to_owned(),
            binary_sha256: vec![(
                "hydracache-loadgen".to_owned(),
                "smoke-unclaimed-binary".to_owned(),
            )],
        },
        None,
        measurements,
        vec!["loopback RESP fixture smoke is not prebuilt-daemon capacity evidence".to_owned()],
    );
    report.to_pretty_json().map_err(|error| {
        RespTierError::Report(format!(
            "fixture smoke failed canonical validation: {error}; problems={:?}",
            report.validation_problems()
        ))
    })?;
    Ok(report)
}

/// Profile dispatch never turns a reference request into fixture evidence.
pub async fn run_resp_profile(profile: &str) -> Result<PerfReport, RespTierError> {
    match profile {
        "smoke-v1" => resp_smoke_report(profile).await,
        "reference-v1" => Err(RespTierError::Report(
            "reference-v1 requires the W7 profile, receipt-bound prebuilt hydracache-server binary, fresh data directory, and daemon readiness proof; refusing fixture evidence"
                .to_owned(),
        )),
        _ => Err(RespTierError::Report(format!(
            "unknown RESP performance profile {profile:?}"
        ))),
    }
}

pub async fn write_resp_report(profile: &str, path: &Path) -> Result<(), RespTierError> {
    let report = run_resp_profile(profile).await?;
    let bytes = report
        .to_pretty_json()
        .map_err(|error| RespTierError::Report(error.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Registered W3 defect canary. The injected delay is owned by the load tool,
/// while the request still traverses a real loopback TCP/product RESP boundary.
pub async fn resp_listener_knee(
    injected_delay: Duration,
) -> Result<crate::KneeResult, RespTierError> {
    let fixture = LoopbackRespFixture::start().await?;
    let result = async {
        let endpoint = RespRunEndpoint::fixture(fixture.address());
        let binding = parse_resp_scenario(WORKLOAD_A_SCENARIO)?;
        let schedule = smoke_schedule(binding.scenario.seed)?;
        let target = Arc::new(resp_target(
            &endpoint,
            &binding,
            &schedule,
            RespTargetShape {
                preload_entries: SMOKE_PRELOAD,
                key_space: SMOKE_KEY_COUNT,
                connections: 1,
                pipeline_depth: 1,
                injected_dispatch_delay: injected_delay,
            },
        )?);
        let mut scenario = smoke_scenario(&binding)?;
        scenario.offered_rates_per_second = vec![1_000, 2_000];
        scenario.p99_slo_us = 5_000;
        scenario.min_achieved_ratio = 0.90;
        let knee = run_scenario(Arc::clone(&target), &scenario).await?;
        validate_knee_target_counters(&target, &knee, 1)?;
        Ok(knee)
    }
    .await;
    fixture.stop().await;
    result
}

async fn resp_knee_measurements(
    endpoint: &RespRunEndpoint,
    injected_delay: Duration,
    shape: RespRunShape,
) -> Result<Vec<MeasurementEvidence>, RespTierError> {
    let mut measurements = Vec::new();
    let mut aggregate_points = Vec::new();
    let mut scenario_inputs = Vec::new();
    let mut workload_inputs = Vec::new();
    let mut aggregate_key_count = None;
    let mut aggregate_spread_limit = None;
    for source in [
        WORKLOAD_A_SCENARIO,
        WORKLOAD_B_SCENARIO,
        WORKLOAD_C_SCENARIO,
    ] {
        let binding = parse_resp_scenario(source)?;
        let scenario = shape.scenario(&binding)?;
        let key_count = shape.key_count(&binding);
        if aggregate_key_count
            .replace(key_count)
            .is_some_and(|seen| seen != key_count)
            || aggregate_spread_limit
                .replace(scenario.robust_spread_tolerance)
                .is_some_and(|seen| seen != scenario.robust_spread_tolerance)
        {
            return Err(RespTierError::Runtime(
                "RESP A/B/C aggregate requires one exact key-count and spread contract".to_owned(),
            ));
        }
        let schedule = shape.schedule(&binding, &scenario)?;
        let target = Arc::new(resp_target(
            endpoint,
            &binding,
            &schedule,
            RespTargetShape {
                preload_entries: scenario.preload_operations,
                key_space: key_count,
                connections: binding.resp.connections,
                pipeline_depth: binding.resp.pipeline,
                injected_dispatch_delay: injected_delay,
            },
        )?);
        let criteria = scenario.sustainability_criteria();
        let knee = run_scenario(Arc::clone(&target), &scenario).await?;
        validate_knee_target_counters(&target, &knee, binding.resp.pipeline)?;
        let tcp = target
            .tcp_evidence()
            .await
            .map_err(|error| RespTierError::Runtime(error.to_string()))?;
        validate_tcp_evidence(&tcp, endpoint, binding.resp.connections)?;
        let sustainable = knee.sustainable_rate_per_second.ok_or_else(|| {
            RespTierError::Runtime(format!(
                "RESP workload {} smoke knee has no sustainable point",
                binding.resp.workload
            ))
        })?;
        let selected = knee
            .evaluated
            .iter()
            .find(|point| point.sample.offered_rate_per_second == sustainable)
            .ok_or_else(|| RespTierError::Runtime("selected RESP knee disappeared".to_owned()))?;
        aggregate_points.push(scalar_point(
            BTreeMap::from([(
                "workload".to_owned(),
                DimensionValue::Text(binding.resp.workload.clone()),
            )]),
            "operations_per_second_at_slo",
            selected
                .repeats
                .iter()
                .map(|repeat| repeat.steady.achieved_rate_per_second)
                .collect(),
        ));
        let id = format!(
            "resp_open_loop_get_set_knee_at_slo_workload_{}",
            binding.resp.workload.to_ascii_lowercase()
        );
        let workload = workload_identity(&schedule, &binding.resp);
        let scenario_digest =
            effective_digest(&binding, &scenario, endpoint, shape.key_count(&binding));
        let selected_endpoint = endpoint.identity.selected_endpoint.clone();
        let capability_digest = endpoint.capability_digest.clone();
        scenario_inputs.push((id.clone(), scenario_digest.clone()));
        workload_inputs.push((id.clone(), workload.digest.clone()));
        measurements.push(MeasurementEvidence::LoadCurve(LoadCurveEvidence {
            id,
            scenario_digest,
            dimensions: BTreeMap::from([
                (
                    "workload".to_owned(),
                    DimensionValue::Text(binding.resp.workload.clone()),
                ),
                (
                    "methodology".to_owned(),
                    DimensionValue::Text("open-loop-scheduled-send".to_owned()),
                ),
                (
                    "selected_endpoint".to_owned(),
                    DimensionValue::Text(selected_endpoint),
                ),
                (
                    "state_scope".to_owned(),
                    DimensionValue::Text(endpoint.identity.state_scope.clone()),
                ),
                (
                    "endpoint_kind".to_owned(),
                    DimensionValue::Text(endpoint.identity.endpoint_kind.clone()),
                ),
                (
                    "endpoint_capability_digest".to_owned(),
                    DimensionValue::Text(capability_digest.clone()),
                ),
                ("real_tcp".to_owned(), DimensionValue::Bool(true)),
                (
                    "connections".to_owned(),
                    DimensionValue::U64(binding.resp.connections as u64),
                ),
                (
                    "pipeline_depth".to_owned(),
                    DimensionValue::U64(binding.resp.pipeline as u64),
                ),
                (
                    "verified_pipeline_exchanges_per_repeat".to_owned(),
                    DimensionValue::U64(scenario.steady_operations),
                ),
                (
                    "preload_operations".to_owned(),
                    DimensionValue::U64(scenario.preload_operations),
                ),
                (
                    "warmup_operations".to_owned(),
                    DimensionValue::U64(scenario.warmup_operations),
                ),
                (
                    "steady_operations".to_owned(),
                    DimensionValue::U64(scenario.steady_operations),
                ),
                (
                    "repeats".to_owned(),
                    DimensionValue::U64(u64::from(scenario.repeats)),
                ),
                ("key_count".to_owned(), DimensionValue::U64(key_count)),
                (
                    "repeat_isolation".to_owned(),
                    DimensionValue::Text(binding.resp.repeat_isolation.clone()),
                ),
                (
                    "daemon_reused_across_repeats".to_owned(),
                    DimensionValue::Bool(binding.resp.daemon_reused_across_repeats),
                ),
                (
                    "verified_commands_and_replies_per_repeat".to_owned(),
                    DimensionValue::U64(
                        scenario
                            .steady_operations
                            .saturating_mul(binding.resp.pipeline as u64),
                    ),
                ),
            ]),
            workload,
            criteria: Some(criteria),
            knee: Some(knee),
            claim: LoadClaim::CapacityKnee,
        }));
    }
    measurements.push(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "resp_open_loop_get_set_knee_at_slo".to_owned(),
        scenario_digest: derived_identity_digest(&scenario_inputs),
        workload: WorkloadIdentity {
            generator: "hydracache-resp-abc-matrix".to_owned(),
            generator_version: "1".to_owned(),
            seed: None,
            key_distribution: Some(KeyDistributionIdentity {
                kind: "uniform".to_owned(),
                theta: None,
            }),
            key_count: aggregate_key_count,
            operation_mix: vec![
                WeightedOperation {
                    operation: "workload_a".to_owned(),
                    weight: 1.0 / 3.0,
                },
                WeightedOperation {
                    operation: "workload_b".to_owned(),
                    weight: 1.0 / 3.0,
                },
                WeightedOperation {
                    operation: "workload_c".to_owned(),
                    weight: 1.0 / 3.0,
                },
            ],
            payload_mix: vec![WeightedPayload {
                bytes: 256,
                weight: 1.0,
            }],
            digest: derived_identity_digest(&workload_inputs),
        },
        points: aggregate_points,
        derived_from: scenario_inputs.iter().map(|(id, _)| id.clone()).collect(),
        max_robust_spread_ratio: aggregate_spread_limit.ok_or_else(|| {
            RespTierError::Runtime("RESP A/B/C aggregate has no spread contract".to_owned())
        })?,
    }));
    Ok(measurements)
}

async fn resp_matrix_measurement(
    endpoint: &RespRunEndpoint,
    shape: RespRunShape,
) -> Result<MeasurementEvidence, RespTierError> {
    let input: MatrixInput = parse_toml(MATRIX_SCENARIO)?;
    validate_matrix(&input)?;
    let (operations_per_repeat, repeats, preload_entries, key_count, spread_tolerance) = match shape
    {
        RespRunShape::Smoke => (
            60,
            SMOKE_REPEATS,
            SMOKE_PRELOAD,
            SMOKE_KEY_COUNT,
            SMOKE_SPREAD_LIMIT,
        ),
        RespRunShape::Reference => (
            input.operations_per_repeat,
            u32::try_from(input.repeats).map_err(|_| {
                RespTierError::Runtime("RESP matrix repeat count overflowed u32".to_owned())
            })?,
            input.preload_entries,
            input.key_count,
            input.robust_spread_tolerance,
        ),
    };
    let schedule = KeyScheduleSpec::uniform(input.seed, key_count, operations_per_repeat)
        .generate()
        .map_err(RespTierError::Runtime)?;
    let binding = parse_resp_scenario(WORKLOAD_A_SCENARIO)?;
    let mut points = Vec::new();
    for connections in &input.connections {
        for pipeline in &input.pipelines {
            let mut samples = Vec::new();
            let mut logical_rates = Vec::new();
            for _ in 0..repeats {
                let target = Arc::new(resp_target(
                    endpoint,
                    &binding,
                    &schedule,
                    RespTargetShape {
                        preload_entries,
                        key_space: key_count,
                        connections: *connections,
                        pipeline_depth: *pipeline,
                        injected_dispatch_delay: Duration::ZERO,
                    },
                )?);
                Target::reset(target.as_ref()).await?;
                Target::preload(target.as_ref()).await?;
                let observation = run_open_loop(
                    Arc::clone(&target),
                    &OpenLoopConfig {
                        offered_rate_per_second: 1_000,
                        operations: operations_per_repeat,
                        highest_trackable_latency: Duration::from_secs(5),
                        significant_figures: 3,
                        p999_min_samples: 1_000,
                        drain_timeout: Duration::from_secs(5),
                    },
                )
                .await
                .map_err(RespTierError::Runtime)?;
                if observation.errors != 0
                    || observation.timeouts != 0
                    || observation.rejections != 0
                    || !observation.backlog_drained
                {
                    return Err(RespTierError::Runtime(
                        "RESP connection/pipeline sweep returned unsuccessful work".to_owned(),
                    ));
                }
                let tcp = target
                    .tcp_evidence()
                    .await
                    .map_err(|error| RespTierError::Runtime(error.to_string()))?;
                validate_tcp_evidence(&tcp, endpoint, *connections)?;
                validate_target_counters(&target, 0, 0, 0, 0, &observation, *pipeline)?;
                samples.push(observation.latency.p99_us.unwrap_or(u64::MAX) as f64);
                logical_rates.push(observation.achieved_rate_per_second * *pipeline as f64);
            }
            let logical_rate = median(&logical_rates);
            points.push(scalar_point(
                BTreeMap::from([
                    (
                        "connections".to_owned(),
                        DimensionValue::U64(*connections as u64),
                    ),
                    ("pipeline".to_owned(), DimensionValue::U64(*pipeline as u64)),
                    (
                        "methodology".to_owned(),
                        DimensionValue::Text("open-loop-scheduled-send".to_owned()),
                    ),
                    ("real_tcp".to_owned(), DimensionValue::Bool(true)),
                    (
                        "endpoint_kind".to_owned(),
                        DimensionValue::Text(endpoint.identity.endpoint_kind.clone()),
                    ),
                    (
                        "selected_endpoint".to_owned(),
                        DimensionValue::Text(endpoint.identity.selected_endpoint.clone()),
                    ),
                    (
                        "endpoint_capability_digest".to_owned(),
                        DimensionValue::Text(endpoint.capability_digest.clone()),
                    ),
                    ("key_count".to_owned(), DimensionValue::U64(key_count)),
                    (
                        "preload_entries".to_owned(),
                        DimensionValue::U64(preload_entries),
                    ),
                    (
                        "repeats".to_owned(),
                        DimensionValue::U64(u64::from(repeats)),
                    ),
                    (
                        "repeat_isolation".to_owned(),
                        DimensionValue::Text(input.repeat_isolation.clone()),
                    ),
                    (
                        "daemon_reused_across_repeats".to_owned(),
                        DimensionValue::Bool(input.daemon_reused_across_repeats),
                    ),
                    (
                        "state_scope".to_owned(),
                        DimensionValue::Text(endpoint.identity.state_scope.clone()),
                    ),
                    (
                        "latency_scope".to_owned(),
                        DimensionValue::Text(
                            "pipeline-batch-completion-from-scheduled-send".to_owned(),
                        ),
                    ),
                    (
                        "verified_pipeline_exchanges_per_repeat".to_owned(),
                        DimensionValue::U64(operations_per_repeat),
                    ),
                    (
                        "verified_commands_and_replies_per_repeat".to_owned(),
                        DimensionValue::U64(operations_per_repeat.saturating_mul(*pipeline as u64)),
                    ),
                    (
                        "logical_operations_per_second".to_owned(),
                        DimensionValue::F64(logical_rate),
                    ),
                ]),
                "scheduled_send_p99_microseconds",
                samples,
            ));
        }
    }
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "resp_open_loop_connection_and_pipeline_sweeps".to_owned(),
        scenario_digest: custom_effective_digest(
            MATRIX_SCENARIO,
            &serde_json::json!({
                "operations_per_repeat": operations_per_repeat,
                "repeats": repeats,
                "preload_entries": preload_entries,
                "key_count": key_count,
                "payload_bytes": binding.resp.payload_bytes,
                "batch_size": binding.resp.batch_size,
                "connections": input.connections,
                "pipelines": input.pipelines,
                "surface_kind": endpoint.identity.endpoint_kind,
                "state_scope": endpoint.identity.state_scope,
            }),
        ),
        workload: workload_identity(&schedule, &binding.resp),
        points,
        derived_from: Vec::new(),
        max_robust_spread_ratio: spread_tolerance,
    }))
}

async fn resp_stall_visibility_measurement(
    endpoint: &RespRunEndpoint,
) -> Result<MeasurementEvidence, RespTierError> {
    let binding = parse_resp_scenario(WORKLOAD_A_SCENARIO)?;
    let schedule = smoke_schedule(binding.scenario.seed)?;
    let mut points = Vec::new();
    for delay in [Duration::ZERO, Duration::from_millis(10)] {
        let mut samples = Vec::new();
        for _ in 0..SMOKE_REPEATS {
            let target = Arc::new(resp_target(
                endpoint,
                &binding,
                &schedule,
                RespTargetShape {
                    preload_entries: SMOKE_PRELOAD,
                    key_space: SMOKE_KEY_COUNT,
                    connections: 1,
                    pipeline_depth: 1,
                    injected_dispatch_delay: delay,
                },
            )?);
            Target::reset(target.as_ref()).await?;
            Target::preload(target.as_ref()).await?;
            let observation = run_open_loop(
                Arc::clone(&target),
                &OpenLoopConfig {
                    offered_rate_per_second: 1_000,
                    operations: 30,
                    highest_trackable_latency: Duration::from_secs(5),
                    significant_figures: 3,
                    p999_min_samples: 1_000,
                    drain_timeout: Duration::from_secs(5),
                },
            )
            .await
            .map_err(RespTierError::Runtime)?;
            validate_target_counters(&target, 0, 0, 0, 0, &observation, 1)?;
            samples.push(observation.latency.p99_us.unwrap_or(u64::MAX) as f64);
        }
        points.push(scalar_point(
            BTreeMap::from([
                (
                    "injected_loadgen_delay_us".to_owned(),
                    DimensionValue::U64(delay.as_micros() as u64),
                ),
                (
                    "methodology".to_owned(),
                    DimensionValue::Text("open-loop-scheduled-send".to_owned()),
                ),
                ("real_tcp".to_owned(), DimensionValue::Bool(true)),
                (
                    "endpoint_kind".to_owned(),
                    DimensionValue::Text(endpoint.identity.endpoint_kind.clone()),
                ),
                (
                    "selected_endpoint".to_owned(),
                    DimensionValue::Text(endpoint.identity.selected_endpoint.clone()),
                ),
                (
                    "endpoint_capability_digest".to_owned(),
                    DimensionValue::Text(endpoint.capability_digest.clone()),
                ),
                (
                    "state_scope".to_owned(),
                    DimensionValue::Text(endpoint.identity.state_scope.clone()),
                ),
                (
                    "instrument_scope".to_owned(),
                    DimensionValue::Text("bounded-falsifiability-probe-not-capacity".to_owned()),
                ),
            ]),
            "scheduled_send_p99_microseconds",
            samples,
        ));
    }
    if points[1].quantity.value <= points[0].quantity.value {
        return Err(RespTierError::Runtime(
            "scheduled-send latency failed to expose the injected RESP delay".to_owned(),
        ));
    }
    Ok(MeasurementEvidence::Scalar(ScalarEvidence {
        id: "resp_open_loop_stall_is_visible_in_scheduled_latency".to_owned(),
        scenario_digest: digest_parts(&[
            digest_bytes(WORKLOAD_A_SCENARIO).as_bytes(),
            b"resp-stall-visibility-v1",
        ]),
        workload: workload_identity(&schedule, &binding.resp),
        points,
        derived_from: Vec::new(),
        max_robust_spread_ratio: SMOKE_SPREAD_LIMIT,
    }))
}

fn resp_target(
    endpoint: &RespRunEndpoint,
    binding: &BoundRespScenario,
    schedule: &GeneratedKeySchedule,
    shape: RespTargetShape,
) -> Result<RespTcpTarget, RespTierError> {
    let config = RespTargetConfig {
        endpoint: endpoint.identity.clone(),
        require_loopback: true,
        connections: shape.connections,
        pipeline_depth: shape.pipeline_depth,
        preload_entries: shape.preload_entries,
        key_space: shape.key_space,
        payload_bytes: binding.resp.payload_bytes as usize,
        batch_size: binding.resp.batch_size,
        reset_batch_entries: 128,
        operation_mix: parsed_operation_mix(&binding.resp)?,
        key_schedule: Arc::new(schedule.keys.clone()),
        connect_timeout: Duration::from_secs(2),
        io_timeout: Duration::from_secs(2),
        parser_limits: Resp2Limits::default(),
        injected_dispatch_delay: shape.injected_dispatch_delay,
    };
    RespTcpTarget::new(config).map_err(|error| RespTierError::Runtime(error.to_string()))
}

fn validate_tcp_evidence(
    evidence: &RespTcpEvidence,
    endpoint: &RespRunEndpoint,
    expected_connections: usize,
) -> Result<(), RespTierError> {
    if !evidence.real_tcp
        || evidence.connection_count != expected_connections
        || evidence.peer_addresses.len() != expected_connections
        || evidence.local_addresses.len() != expected_connections
        || evidence
            .peer_addresses
            .iter()
            .any(|peer| *peer != endpoint.identity.address)
        || evidence
            .local_addresses
            .iter()
            .any(|local| !local.ip().is_loopback())
        || evidence
            .local_addresses
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != expected_connections
        || evidence.selected_endpoint != endpoint.identity.selected_endpoint
    {
        return Err(RespTierError::Runtime(format!(
            "RESP run did not retain the exact selected real-TCP endpoint: {evidence:?}"
        )));
    }
    Ok(())
}

fn validate_knee_target_counters(
    target: &RespTcpTarget,
    knee: &KneeResult,
    pipeline_depth: usize,
) -> Result<(), RespTierError> {
    let repeat = knee
        .evaluated
        .last()
        .and_then(|point| point.repeats.last())
        .ok_or_else(|| RespTierError::Runtime("RESP knee has no final repeat".to_owned()))?;
    validate_target_counters(
        target,
        repeat.phase.warmup_successes,
        repeat.phase.warmup_errors,
        repeat.phase.warmup_timeouts,
        repeat.phase.warmup_rejections,
        &repeat.steady,
        pipeline_depth,
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_target_counters(
    target: &RespTcpTarget,
    warmup_successes: u64,
    warmup_errors: u64,
    warmup_timeouts: u64,
    warmup_rejections: u64,
    steady: &OpenLoopObservation,
    pipeline_depth: usize,
) -> Result<(), RespTierError> {
    let snapshot = target.snapshot();
    let warmup_exchanges = warmup_successes
        .saturating_add(warmup_errors)
        .saturating_add(warmup_timeouts)
        .saturating_add(warmup_rejections);
    let minimum_exchanges = warmup_exchanges.saturating_add(steady.completed);
    let maximum_exchanges = warmup_exchanges.saturating_add(steady.started);
    let expected_commands = snapshot
        .pipeline_exchanges
        .checked_mul(pipeline_depth as u64)
        .ok_or_else(|| RespTierError::Runtime("RESP counter expectation overflowed".to_owned()))?;
    let expected_successes = warmup_successes.saturating_add(steady.successes);
    let expected_rejections = warmup_rejections.saturating_add(steady.rejections);
    let expected_failures = warmup_errors
        .saturating_add(warmup_timeouts)
        .saturating_add(steady.errors)
        .saturating_add(steady.timeouts);
    let expected_completed = warmup_exchanges.saturating_add(steady.completed);
    let accounted_outcomes = snapshot
        .successful_exchanges
        .saturating_add(snapshot.rejected_exchanges)
        .saturating_add(snapshot.failed_exchanges);
    let minimum_replies = expected_successes
        .saturating_add(expected_rejections)
        .saturating_mul(pipeline_depth as u64);
    let maximum_replies = expected_completed.saturating_mul(pipeline_depth as u64);
    if !(minimum_exchanges..=maximum_exchanges).contains(&snapshot.pipeline_exchanges)
        || snapshot.commands_sent != expected_commands
        || snapshot.successful_exchanges != expected_successes
        || snapshot.rejected_exchanges != expected_rejections
        || snapshot.failed_exchanges != expected_failures
        || accounted_outcomes != expected_completed
        || !(minimum_replies..=maximum_replies).contains(&snapshot.replies_received)
    {
        return Err(RespTierError::Runtime(format!(
            "RESP counter conservation failed: exchanges={minimum_exchanges}..={maximum_exchanges}, commands={expected_commands}, successes={expected_successes}, rejections={expected_rejections}, failures={expected_failures}, completed={expected_completed}, replies={minimum_replies}..={maximum_replies}; got {snapshot:?}"
        )));
    }
    Ok(())
}

fn parse_resp_scenario(source: &[u8]) -> Result<BoundRespScenario, RespTierError> {
    let text =
        std::str::from_utf8(source).map_err(|error| RespTierError::Runtime(error.to_string()))?;
    let mut root = text
        .parse::<toml::Table>()
        .map_err(|error| RespTierError::Runtime(error.to_string()))?;
    let resp: RespInputs = root
        .remove("resp")
        .ok_or_else(|| RespTierError::Runtime("missing [resp] scenario section".to_owned()))?
        .try_into()
        .map_err(|error| RespTierError::Runtime(error.to_string()))?;
    validate_resp_inputs(&resp)?;
    let scenario: Scenario = toml::Value::Table(root)
        .try_into()
        .map_err(|error| RespTierError::Runtime(error.to_string()))?;
    scenario.validate().map_err(RespTierError::Runtime)?;
    Ok(BoundRespScenario {
        scenario,
        resp,
        source_digest: digest_bytes(source),
    })
}

fn validate_resp_inputs(input: &RespInputs) -> Result<(), RespTierError> {
    if !matches!(input.workload.as_str(), "A" | "B" | "C")
        || input.key_count == 0
        || input.payload_bytes == 0
        || input.batch_size == 0
        || input.connections == 0
        || input.pipeline == 0
        || input.repeat_isolation != "logical-keyspace-reset-and-counter-zero"
        || !input.daemon_reused_across_repeats
        || input.operation_mix.is_empty()
        || input.operation_mix.iter().any(|entry| {
            !matches!(entry.operation.as_str(), "get" | "set" | "mget" | "mset")
                || !entry.weight.is_finite()
                || entry.weight <= 0.0
        })
    {
        return Err(RespTierError::Runtime(
            "RESP scenario contract is incomplete".to_owned(),
        ));
    }
    let total = input
        .operation_mix
        .iter()
        .map(|entry| entry.weight)
        .sum::<f64>();
    if (total - 1.0).abs() > 1e-9 {
        return Err(RespTierError::Runtime(format!(
            "RESP operation weights must total 1.0, got {total}"
        )));
    }
    let expected = match input.workload.as_str() {
        "A" => RespOperationMix::WORKLOAD_A,
        "B" => RespOperationMix::WORKLOAD_B,
        "C" => RespOperationMix::WORKLOAD_C,
        _ => unreachable!(),
    };
    if parsed_operation_mix(input)? != expected {
        return Err(RespTierError::Runtime(format!(
            "RESP workload {} does not match the committed A/B/C taxonomy",
            input.workload
        )));
    }
    Ok(())
}

fn parsed_operation_mix(input: &RespInputs) -> Result<RespOperationMix, RespTierError> {
    let mut mix = RespOperationMix {
        get_percent: 0,
        set_percent: 0,
        mget_percent: 0,
        mset_percent: 0,
    };
    for entry in &input.operation_mix {
        let percentage = (entry.weight * 100.0).round() as u8;
        match entry.operation.as_str() {
            "get" => mix.get_percent = percentage,
            "set" => mix.set_percent = percentage,
            "mget" => mix.mget_percent = percentage,
            "mset" => mix.mset_percent = percentage,
            operation => {
                return Err(RespTierError::Runtime(format!(
                    "unsupported RESP operation {operation}"
                )))
            }
        }
    }
    Ok(mix)
}

fn smoke_scenario(binding: &BoundRespScenario) -> Result<Scenario, RespTierError> {
    let mut scenario = binding.scenario.clone();
    scenario.id = format!("{}-smoke", scenario.id);
    scenario.offered_rates_per_second = vec![250, 1_000];
    scenario.preload_operations = SMOKE_PRELOAD;
    scenario.warmup_operations = 0;
    scenario.steady_operations = SMOKE_OPERATIONS;
    scenario.repeats = SMOKE_REPEATS;
    scenario.p99_slo_us = 500_000;
    scenario.p999_slo_us = None;
    scenario.p999_min_samples = 1;
    scenario.min_achieved_ratio = 0.50;
    scenario.robust_spread_tolerance = SMOKE_SPREAD_LIMIT;
    scenario.validate().map_err(RespTierError::Runtime)?;
    Ok(scenario)
}

fn smoke_schedule(seed: u64) -> Result<GeneratedKeySchedule, RespTierError> {
    KeyScheduleSpec::uniform(seed, SMOKE_KEY_COUNT, SMOKE_OPERATIONS)
        .generate()
        .map_err(RespTierError::Runtime)
}

fn validate_matrix(input: &MatrixInput) -> Result<(), RespTierError> {
    if input.schema_version != 1
        || input.id.is_empty()
        || input.operations_per_repeat == 0
        || input.repeats < 3
        || input.preload_entries == 0
        || input.preload_entries > input.key_count
        || input.payload_bytes == 0
        || input.batch_size == 0
        || input.connections != [1, 10, 100]
        || input.pipelines != [1, 10]
        || input.metric != "scheduled_send_p99_microseconds"
        || input.methodology != "open-loop"
        || input.state_scope != "node-local"
        || !input.selected_endpoint_only
        || input.repeat_isolation != "logical-keyspace-reset-and-counter-zero"
        || !input.daemon_reused_across_repeats
        || !input.robust_spread_tolerance.is_finite()
        || input.robust_spread_tolerance < 0.0
    {
        return Err(RespTierError::Runtime(
            "RESP connection/pipeline matrix contract is incomplete".to_owned(),
        ));
    }
    Ok(())
}

fn workload_identity(schedule: &GeneratedKeySchedule, input: &RespInputs) -> WorkloadIdentity {
    let operations = input
        .operation_mix
        .iter()
        .map(|entry| WeightedOperation {
            operation: entry.operation.clone(),
            weight: entry.weight,
        })
        .collect::<Vec<_>>();
    let payloads = vec![WeightedPayload {
        bytes: input.payload_bytes,
        weight: 1.0,
    }];
    let (kind, theta) = match schedule.spec.distribution {
        KeyDistribution::Uniform => ("uniform", None),
        KeyDistribution::Zipfian { theta } => ("zipfian", Some(theta)),
    };
    let operation_bytes = serde_json::to_vec(&operations).expect("RESP operations serialize");
    let payload_bytes = serde_json::to_vec(&payloads).expect("RESP payloads serialize");
    WorkloadIdentity {
        generator: "hydracache-cache-sim-key-schedule".to_owned(),
        generator_version: KEY_SCHEDULE_GENERATOR_VERSION.to_string(),
        seed: Some(schedule.spec.seed),
        key_distribution: Some(KeyDistributionIdentity {
            kind: kind.to_owned(),
            theta,
        }),
        key_count: Some(schedule.spec.key_count),
        operation_mix: operations,
        payload_mix: payloads,
        digest: digest_parts(&[
            schedule.digest.as_bytes(),
            b"hydracache-resp-open-loop-workload-v1",
            &operation_bytes,
            &payload_bytes,
        ]),
    }
}

fn effective_digest(
    binding: &BoundRespScenario,
    scenario: &Scenario,
    endpoint: &RespRunEndpoint,
    key_count: u64,
) -> String {
    let resp = serde_json::to_vec(&binding.resp).expect("RESP scenario inputs serialize");
    let effective_target = serde_json::to_vec(&serde_json::json!({
        "key_count": key_count,
        "preload_entries": scenario.preload_operations,
        "warmup_operations": scenario.warmup_operations,
        "steady_operations": scenario.steady_operations,
        "repeats": scenario.repeats,
        "connections": binding.resp.connections,
        "pipeline_depth": binding.resp.pipeline,
        "payload_bytes": binding.resp.payload_bytes,
        "batch_size": binding.resp.batch_size,
        "surface_kind": endpoint.identity.endpoint_kind,
        "state_scope": endpoint.identity.state_scope,
    }))
    .expect("RESP effective target inputs serialize");
    let serialized_scenario = serde_json::to_vec(scenario).expect("Scenario serializes");
    digest_parts(&[
        binding.source_digest.as_bytes(),
        b"hydracache-resp-open-loop-effective-v1",
        &resp,
        &serialized_scenario,
        &effective_target,
    ])
}

fn custom_effective_digest(source: &[u8], effective: &serde_json::Value) -> String {
    let effective = serde_json::to_vec(effective).expect("RESP effective inputs serialize");
    digest_parts(&[
        digest_bytes(source).as_bytes(),
        b"hydracache-resp-open-loop-effective-v1",
        &effective,
    ])
}

fn scalar_point(
    dimensions: BTreeMap<String, DimensionValue>,
    unit: &str,
    samples: Vec<f64>,
) -> ScalarPoint {
    let mut ordered = samples.clone();
    ordered.sort_by(f64::total_cmp);
    let value = ordered[ordered.len() / 2];
    let min = ordered[0];
    let max = ordered[ordered.len() - 1];
    let robust_spread_ratio = if value > 0.0 {
        (max - min) / value
    } else if max == min {
        0.0
    } else {
        f64::INFINITY
    };
    ScalarPoint {
        dimensions,
        quantity: Quantity {
            value,
            unit: unit.to_owned(),
        },
        sample_count: samples.len() as u64,
        samples,
        min,
        max,
        robust_spread_ratio,
    }
}

fn parse_toml<T>(source: &[u8]) -> Result<T, RespTierError>
where
    T: for<'de> Deserialize<'de>,
{
    let text =
        std::str::from_utf8(source).map_err(|error| RespTierError::Runtime(error.to_string()))?;
    toml::from_str(text).map_err(|error| RespTierError::Runtime(error.to_string()))
}

fn median(values: &[f64]) -> f64 {
    let mut values = values.to_vec();
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn smoke_fingerprint() -> RunnerFingerprint {
    RunnerFingerprint {
        runner_class: "smoke-local".to_owned(),
        fingerprint: "smoke-local-unclaimed".to_owned(),
        cpu_model: "smoke-unclaimed".to_owned(),
        logical_cores: 1,
        ram_bytes: 1,
        os: std::env::consts::OS.to_owned(),
        kernel: "smoke-unclaimed".to_owned(),
        cpu_affinity: "smoke-unpinned".to_owned(),
        cgroup_cpu_quota: "smoke-unclaimed".to_owned(),
        governor: "smoke-unclaimed".to_owned(),
        turbo: "smoke-unclaimed".to_owned(),
        shared_hardware: true,
        calibration_score: 0.0,
    }
}

fn smoke_profile(name: &str, fingerprint: &RunnerFingerprint) -> PerformanceProfile {
    PerformanceProfile {
        name: name.to_owned(),
        required_runner_class: fingerprint.runner_class.clone(),
        allowed_fingerprints: vec![fingerprint.fingerprint.clone()],
        minimum_logical_cores: 1,
        required_cpu_affinity: fingerprint.cpu_affinity.clone(),
        required_cgroup_cpu_quota: fingerprint.cgroup_cpu_quota.clone(),
        require_dedicated: true,
        maximum_calibration_score: 1.0,
    }
}

fn digest_bytes(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes).as_ref())
}

fn digest_parts(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hex_digest(hasher.finalize().as_ref())
}

fn fixture_capability_digest(endpoint: SocketAddr) -> String {
    digest_parts(&[
        b"hydracache-resp-loopback-fixture-capability-v1",
        endpoint.to_string().as_bytes(),
    ])
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::RespDaemonConfigIdentity;
    use crate::tiers::resp_reference::{LogEvidence, RespPingEvidence};

    fn lifecycle_fixture() -> (RespEndpointCapability, RespDaemonEvidence, String) {
        let resp_endpoint = SocketAddr::from((Ipv4Addr::LOCALHOST, 16_701));
        let admin_endpoint = SocketAddr::from((Ipv4Addr::LOCALHOST, 16_702));
        let data_dir = std::env::current_dir()
            .unwrap()
            .join("target/w3-data-fixture");
        let server_sha = digest_bytes(b"server");
        let loadgen_sha = digest_bytes(b"loadgen");
        let capability = RespEndpointCapability {
            schema_version: 1,
            pid: 67,
            started_unix_nanos: 1,
            repeat_index: 0,
            direct_prebuilt_exec: true,
            fresh_data_dir: true,
            config: RespDaemonConfigIdentity {
                role: "local".to_owned(),
                listen_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                cluster_addr: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
                storage_dir: data_dir.clone(),
                admin_enabled: true,
                admin_addr: admin_endpoint,
                redis_enabled: true,
                redis_addr: resp_endpoint,
                redis_auth_required: false,
                rediss_enabled: false,
            },
            selected_endpoint: format!("hydracache-server@{resp_endpoint}"),
            server_binary_sha256: server_sha.clone(),
            loadgen_binary_sha256: loadgen_sha.clone(),
            prebuild_manifest_sha256: digest_bytes(b"manifest"),
            prebuild_contract_digest: digest_bytes(b"contract"),
            source_commit: "ab".repeat(20),
        };
        let capability_digest = capability.digest().unwrap();
        let log = LogEvidence {
            canonical_path: std::env::current_dir()
                .unwrap()
                .join("target/w3-daemon.log"),
            bytes: 0,
            sha256: digest_bytes(b""),
        };
        let daemon = RespDaemonEvidence {
            repeat_index: 0,
            pid: 67,
            direct_prebuilt_exec: true,
            server_binary_path: std::env::current_dir()
                .unwrap()
                .join("target/release/server"),
            server_binary_sha256: server_sha,
            loadgen_binary_path: std::env::current_dir()
                .unwrap()
                .join("target/release/loadgen"),
            loadgen_binary_sha256: loadgen_sha,
            binaries_verified_after_measurement: true,
            resp_endpoint,
            admin_endpoint,
            selected_endpoint: capability.selected_endpoint.clone(),
            endpoint_capability_digest: capability_digest.clone(),
            data_dir,
            readiness: RespPingEvidence {
                request_sha256: digest_bytes(RESP_PING_FRAME),
                response_sha256: digest_bytes(RESP_PONG_FRAME),
                attempts: 1,
                selected_endpoint: resp_endpoint,
                exact_response: RESP_PONG_DISPLAY.to_owned(),
            },
            killed_and_waited: true,
            exit_code: None,
            stdout_log: log.clone(),
            stderr_log: log,
        };
        (capability, daemon, capability_digest)
    }

    #[test]
    fn suite_lifecycle_binding_rejects_failed_cleanup_and_identity_tampering() {
        let (capability, daemon, digest) = lifecycle_fixture();
        assert!(lifecycle_matches_capability(&capability, &daemon, &digest));

        let mut tampered = daemon.clone();
        tampered.killed_and_waited = false;
        assert!(!lifecycle_matches_capability(
            &capability,
            &tampered,
            &digest
        ));

        let mut tampered = daemon.clone();
        tampered.binaries_verified_after_measurement = false;
        assert!(!lifecycle_matches_capability(
            &capability,
            &tampered,
            &digest
        ));

        let mut tampered = daemon.clone();
        tampered.pid += 1;
        assert!(!lifecycle_matches_capability(
            &capability,
            &tampered,
            &digest
        ));

        let mut tampered = daemon;
        tampered.server_binary_sha256 = digest_bytes(b"other server");
        assert!(!lifecycle_matches_capability(
            &capability,
            &tampered,
            &digest
        ));
    }

    #[test]
    fn suite_rejects_completed_but_unstable_external_evidence() {
        assert!(external_ship_status_is_eligible(true, true, &[]));
        assert!(!external_ship_status_is_eligible(
            false,
            false,
            &["spread exceeded".to_owned()]
        ));
        assert!(!external_ship_status_is_eligible(true, false, &[]));
        assert!(!external_ship_status_is_eligible(
            true,
            true,
            &["unexpected instability reason".to_owned()]
        ));
    }
}
