//! Release-0.67 W9 daemon metrics honesty cross-check.
//!
//! This module deliberately does not create product metrics. It captures the
//! production daemon's existing `/metrics` response on the exact W3/W4A admin
//! endpoint, keeps the raw HTTP bytes, and compares only fields whose semantics
//! match an independent observer for the same interval. At the 0.66 product
//! boundary RESP/admin operation counters and server-side latency summaries are
//! not exported, so those rows are explicitly `not_available`. W4A topology
//! gauges are genuinely comparable to the public control-plane snapshots.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time;

use crate::report::{
    BuildIdentity, EvidenceRunMode, PerfReport, RespEndpointCapability, SourceIdentity,
    SurfaceIdentity,
};
use crate::resp_external::RedisBenchmarkEvidence;
use crate::targets::control_plane::{
    ControlPlaneEndpoint, ControlPlaneReport, ControlPlaneScenario, ControlPlaneSource,
    PublicControlPlaneSnapshot, ValidatedControlPlaneCapability,
};
use crate::tiers::resp::{RespReferenceSuiteEvidence, RespReferenceSuiteReceipt};
use crate::tiers::resp_reference::RespDaemonEvidence;
use crate::{OpenLoopConfig, OpenLoopObservation, PERF_RELEASE, PERF_SCHEMA_VERSION};

pub const W9_CANARY_MARKER: &str = "HC-CANARY-RED:W9";
pub const METRICS_HONESTY_SCENARIO_ID: &str = "metrics-honesty-daemon-v1";
pub const METRICS_PATH: &str = "/metrics";

const MAX_HTTP_HEADER_BYTES: usize = 64 * 1024;
const W3_OPERATIONS_NA: &str = "no-exported-generic-resp-command-counter";
const W3_REJECTIONS_NA: &str = "no-exported-resp-surface-rejection-counter";
const W3_LATENCY_NA: &str = "no-exported-resp-service-latency-summary";
const W3_TOPOLOGY_NA: &str = "resp-capacity-is-node-local-and-not-a-topology-claim";
const W4_OPERATIONS_NA: &str = "no-exported-admin-request-counter";
const W4_REJECTIONS_NA: &str = "no-exported-admin-request-rejection-counter";
const W4_LATENCY_NA: &str = "no-exported-admin-service-latency-summary";
const OBSERVER_PROBE_ID: &str = "w9-w0-open-loop-observer-v1";
const MAX_PREDECESSOR_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;

const W3_OPEN_LOOP_ARTIFACT: &str = "w3_open_loop_report";
const W3_EXTERNAL_ARTIFACT: &str = "w3_external_report";
const W3_LIFECYCLE_ARTIFACT: &str = "w3_daemon_lifecycle";
const W3_SUITE_RECEIPT_ARTIFACT: &str = "w3_suite_receipt";
const W4A_REPORT_ARTIFACT: &str = "w4a_control_plane_report";
const W7_RAW_DIRECTORY: &str = "w7-raw";

static REPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, thiserror::Error)]
pub enum MetricsHonestyError {
    #[error("W9 scenario contract is invalid: {0}")]
    Scenario(String),
    #[error("W9 daemon binding is invalid: {0}")]
    Binding(String),
    #[error("W9 metrics scrape failed: {0}")]
    Scrape(String),
    #[error("W9 Prometheus text is invalid: {0}")]
    Prometheus(String),
    #[error("W9 evidence is invalid: {0}")]
    Evidence(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatencyBoundaryContract {
    pub observer_metric: String,
    pub observer_includes_scheduler_queue_delay: bool,
    pub server_metric: String,
    pub server_metric_scope: String,
    pub equality_claim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceCoverageContract {
    pub operations: String,
    pub rejections: String,
    pub internal_service_latency: String,
    pub topology: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageContract {
    pub w3_node_resp: SurfaceCoverageContract,
    pub w4a_control_plane_admin: SurfaceCoverageContract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverProbeContract {
    pub probe_id: String,
    pub offered_rate_per_second: u64,
    pub operations: u64,
    pub highest_trackable_latency_micros: u64,
    pub histogram_significant_figures: u8,
    pub p999_min_samples: u64,
    pub drain_timeout_millis: u64,
}

impl ObserverProbeContract {
    fn validate(&self) -> Result<(), MetricsHonestyError> {
        if self.probe_id != OBSERVER_PROBE_ID
            || self.offered_rate_per_second != 1_000
            || self.operations != 64
            || self.highest_trackable_latency_micros != 5_000_000
            || self.histogram_significant_figures != 3
            || self.p999_min_samples != 1
            || self.drain_timeout_millis != 5_000
        {
            return Err(MetricsHonestyError::Scenario(
                "the bounded W0 observer probe contract drifted".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn open_loop_config(&self) -> OpenLoopConfig {
        OpenLoopConfig {
            offered_rate_per_second: self.offered_rate_per_second,
            operations: self.operations,
            highest_trackable_latency: Duration::from_micros(self.highest_trackable_latency_micros),
            significant_figures: self.histogram_significant_figures,
            p999_min_samples: self.p999_min_samples,
            drain_timeout: Duration::from_millis(self.drain_timeout_millis),
        }
    }

    fn digest(&self) -> Result<String, MetricsHonestyError> {
        Ok(sha256(&serde_json::to_vec(self)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsHonestyScenario {
    pub schema_version: u32,
    pub release: String,
    pub scenario_id: String,
    pub exporter_path: String,
    pub maximum_scrape_bytes: usize,
    pub connect_timeout_millis: u64,
    pub read_timeout_millis: u64,
    pub counter_absolute_tolerance: u64,
    pub counter_relative_tolerance_millionths: u32,
    pub topology_gauge_absolute_tolerance: u64,
    pub required_surfaces: Vec<String>,
    pub observer_probe: ObserverProbeContract,
    pub latency_boundary: LatencyBoundaryContract,
    pub coverage: CoverageContract,
}

impl MetricsHonestyScenario {
    pub fn load(path: &Path) -> Result<Self, MetricsHonestyError> {
        let bytes = fs::read(path)?;
        if bytes.is_empty() || bytes.len() > 1024 * 1024 {
            return Err(MetricsHonestyError::Scenario(
                "W9 scenario must be a non-empty bounded TOML artifact".to_owned(),
            ));
        }
        let text = std::str::from_utf8(&bytes).map_err(|error| {
            MetricsHonestyError::Scenario(format!("W9 scenario is not UTF-8: {error}"))
        })?;
        Self::parse_toml(text)
    }

    pub fn parse_toml(input: &str) -> Result<Self, MetricsHonestyError> {
        let scenario: Self = toml::from_str(input)?;
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn validate(&self) -> Result<(), MetricsHonestyError> {
        self.observer_probe.validate()?;
        let surfaces = self
            .required_surfaces
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let exact_surfaces = BTreeSet::from([
            "w3_node_resp".to_owned(),
            "w4a_control_plane_admin".to_owned(),
        ]);
        if self.schema_version != PERF_SCHEMA_VERSION
            || self.release != PERF_RELEASE
            || self.scenario_id != METRICS_HONESTY_SCENARIO_ID
            || self.exporter_path != METRICS_PATH
            || !(4_096..=4 * 1024 * 1024).contains(&self.maximum_scrape_bytes)
            || !(100..=30_000).contains(&self.connect_timeout_millis)
            || !(100..=60_000).contains(&self.read_timeout_millis)
            || self.counter_absolute_tolerance != 1
            || self.counter_relative_tolerance_millionths != 10_000
            || self.topology_gauge_absolute_tolerance != 0
            || surfaces != exact_surfaces
            || surfaces.len() != self.required_surfaces.len()
        {
            return Err(MetricsHonestyError::Scenario(
                "identity, bounds, tolerances, or required daemon surfaces drifted".to_owned(),
            ));
        }
        if self.latency_boundary
            != (LatencyBoundaryContract {
                observer_metric: "scheduled-send-to-completion-latency".to_owned(),
                observer_includes_scheduler_queue_delay: true,
                server_metric: "not_available".to_owned(),
                server_metric_scope: "internal-service-time-if-ever-exported".to_owned(),
                equality_claim: false,
            })
        {
            return Err(MetricsHonestyError::Scenario(
                "queue-inclusive observer latency must remain explicitly distinct from internal service time"
                    .to_owned(),
            ));
        }
        let expected_w3 = SurfaceCoverageContract {
            operations: format!("not_available:{W3_OPERATIONS_NA}"),
            rejections: format!("not_available:{W3_REJECTIONS_NA}"),
            internal_service_latency: format!("not_available:{W3_LATENCY_NA}"),
            topology: format!("not_applicable:{W3_TOPOLOGY_NA}"),
        };
        let expected_w4 = SurfaceCoverageContract {
            operations: format!("not_available:{W4_OPERATIONS_NA}"),
            rejections: format!("not_available:{W4_REJECTIONS_NA}"),
            internal_service_latency: format!("not_available:{W4_LATENCY_NA}"),
            topology: "available:hydracache_cluster_members,hydracache_cluster_leader,hydracache_cluster_epoch".to_owned(),
        };
        if self.coverage.w3_node_resp != expected_w3
            || self.coverage.w4a_control_plane_admin != expected_w4
        {
            return Err(MetricsHonestyError::Scenario(
                "coverage contract invented or relabeled a daemon metric".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, MetricsHonestyError> {
        Ok(sha256(&serde_json::to_vec(self)?))
    }

    fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_millis)
    }

    fn read_timeout(&self) -> Duration {
        Duration::from_millis(self.read_timeout_millis)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonMetricsSurface {
    W3NodeResp,
    W4aControlPlaneAdmin,
}

impl DaemonMetricsSurface {
    fn as_str(self) -> &'static str {
        match self {
            Self::W3NodeResp => "w3_node_resp",
            Self::W4aControlPlaneAdmin => "w4a_control_plane_admin",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiveDaemonMetricsBinding {
    pub schema_version: u32,
    pub surface: DaemonMetricsSurface,
    pub scenario_id: String,
    pub scenario_sha256: String,
    pub node_id: String,
    pub pid: u32,
    pub admin_endpoint: SocketAddr,
    pub source_commit: String,
    pub cargo_lock_sha256: Option<String>,
    pub runner_profile: String,
    pub runner_identity_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub prebuild_contract_sha256: String,
    pub server_binary_sha256: String,
    pub loadgen_binary_sha256: Option<String>,
    pub capability_receipt_sha256: String,
    pub direct_prebuilt_exec: bool,
}

impl LiveDaemonMetricsBinding {
    pub fn from_w3(
        scenario: &MetricsHonestyScenario,
        capability: &RespEndpointCapability,
        source: &SourceIdentity,
        build: &BuildIdentity,
        runner_profile: &str,
        runner_identity_sha256: &str,
    ) -> Result<Self, MetricsHonestyError> {
        scenario.validate()?;
        let capability_receipt_sha256 = capability
            .digest()
            .map_err(|error| MetricsHonestyError::Binding(error.to_string()))?;
        let binding = Self {
            schema_version: PERF_SCHEMA_VERSION,
            surface: DaemonMetricsSurface::W3NodeResp,
            scenario_id: scenario.scenario_id.clone(),
            scenario_sha256: scenario.digest()?,
            node_id: "selected-node-local-resp-endpoint".to_owned(),
            pid: capability.pid,
            admin_endpoint: capability.config.admin_addr,
            source_commit: source.git_commit.clone(),
            cargo_lock_sha256: Some(source.cargo_lock_sha256.clone()),
            runner_profile: runner_profile.to_owned(),
            runner_identity_sha256: runner_identity_sha256.to_owned(),
            prebuild_manifest_sha256: build.prebuild_manifest_sha256.clone(),
            prebuild_contract_sha256: build.prebuild_contract_digest.clone(),
            server_binary_sha256: capability.server_binary_sha256.clone(),
            loadgen_binary_sha256: Some(capability.loadgen_binary_sha256.clone()),
            capability_receipt_sha256,
            direct_prebuilt_exec: capability.direct_prebuilt_exec,
        };
        binding.validate(scenario)?;
        if capability.source_commit != binding.source_commit
            || capability.prebuild_manifest_sha256 != binding.prebuild_manifest_sha256
            || capability.prebuild_contract_digest != binding.prebuild_contract_sha256
        {
            return Err(MetricsHonestyError::Binding(
                "W3 endpoint capability does not cross-bind source and prebuild identity"
                    .to_owned(),
            ));
        }
        Ok(binding)
    }

    pub fn from_w4a(
        scenario: &MetricsHonestyScenario,
        capability: &ValidatedControlPlaneCapability,
        node_id: &str,
    ) -> Result<Self, MetricsHonestyError> {
        scenario.validate()?;
        let node = capability
            .nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .ok_or_else(|| {
                MetricsHonestyError::Binding(format!(
                    "W4A node {node_id:?} is absent from the validated capability"
                ))
            })?;
        let binding = Self {
            schema_version: PERF_SCHEMA_VERSION,
            surface: DaemonMetricsSurface::W4aControlPlaneAdmin,
            scenario_id: scenario.scenario_id.clone(),
            scenario_sha256: scenario.digest()?,
            node_id: node.node_id.clone(),
            pid: node.pid,
            admin_endpoint: node.config.launch_config.admin_addr,
            source_commit: capability.source_commit.clone(),
            cargo_lock_sha256: None,
            runner_profile: capability.profile.clone(),
            runner_identity_sha256: capability.runner_fingerprint_sha256.clone(),
            prebuild_manifest_sha256: capability.prebuild_manifest_sha256.clone(),
            prebuild_contract_sha256: capability.prebuild_contract_sha256.clone(),
            server_binary_sha256: capability.server_binary.sha256.clone(),
            loadgen_binary_sha256: None,
            capability_receipt_sha256: capability.receipt.receipt_sha256.clone(),
            direct_prebuilt_exec: node.direct_prebuilt_exec,
        };
        binding.validate(scenario)?;
        if node.observed_executable_sha256 != binding.server_binary_sha256
            || node.config.launch_config.node_id != binding.node_id
        {
            return Err(MetricsHonestyError::Binding(
                "W4A node executable/config differs from the validated capability".to_owned(),
            ));
        }
        Ok(binding)
    }

    pub fn validate(&self, scenario: &MetricsHonestyScenario) -> Result<(), MetricsHonestyError> {
        if self.schema_version != PERF_SCHEMA_VERSION
            || self.scenario_id != scenario.scenario_id
            || self.scenario_sha256 != scenario.digest()?
            || self.node_id.trim().is_empty()
            || self.pid == 0
            || !self.admin_endpoint.ip().is_loopback()
            || self.admin_endpoint.port() == 0
            || !is_git_commit(&self.source_commit)
            || self.runner_profile != "reference-v1"
            || !is_sha256(&self.runner_identity_sha256)
            || !is_sha256(&self.prebuild_manifest_sha256)
            || !is_sha256(&self.prebuild_contract_sha256)
            || !is_sha256(&self.server_binary_sha256)
            || !is_sha256(&self.capability_receipt_sha256)
            || !self.direct_prebuilt_exec
        {
            return Err(MetricsHonestyError::Binding(
                "live binding is not an exact loopback direct-prebuilt reference daemon identity"
                    .to_owned(),
            ));
        }
        match self.surface {
            DaemonMetricsSurface::W3NodeResp => {
                if self
                    .cargo_lock_sha256
                    .as_deref()
                    .is_none_or(|value| !is_sha256(value))
                    || self
                        .loadgen_binary_sha256
                        .as_deref()
                        .is_none_or(|value| !is_sha256(value))
                {
                    return Err(MetricsHonestyError::Binding(
                        "W3 binding must retain exact Cargo.lock and loadgen binary hashes"
                            .to_owned(),
                    ));
                }
            }
            DaemonMetricsSurface::W4aControlPlaneAdmin => {
                if self.cargo_lock_sha256.is_some() || self.loadgen_binary_sha256.is_some() {
                    return Err(MetricsHonestyError::Binding(
                        "W4A binding may not invent fields absent from its capability receipt"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, MetricsHonestyError> {
        Ok(sha256(&serde_json::to_vec(self)?))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrometheusMetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
    Untyped,
}

impl PrometheusMetricType {
    fn parse(value: &str) -> Result<Self, MetricsHonestyError> {
        match value {
            "counter" => Ok(Self::Counter),
            "gauge" => Ok(Self::Gauge),
            "histogram" => Ok(Self::Histogram),
            "summary" => Ok(Self::Summary),
            "untyped" => Ok(Self::Untyped),
            other => Err(MetricsHonestyError::Prometheus(format!(
                "unsupported metric type {other:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricInventoryEntry {
    pub name: String,
    pub metric_type: Option<PrometheusMetricType>,
    pub sample_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricSelector {
    pub name: String,
    pub labels: BTreeMap<String, String>,
}

impl MetricSelector {
    fn live_topology(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            labels: BTreeMap::from([("source".to_owned(), "live".to_owned())]),
        }
    }

    fn live_leader(node_id: &str) -> Self {
        Self {
            name: "hydracache_cluster_leader".to_owned(),
            labels: BTreeMap::from([
                ("node".to_owned(), node_id.to_owned()),
                ("source".to_owned(), "live".to_owned()),
            ]),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedMetricSample {
    selector: MetricSelector,
    value: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct PrometheusDocument {
    types: BTreeMap<String, PrometheusMetricType>,
    samples: Vec<ParsedMetricSample>,
}

impl PrometheusDocument {
    fn parse(body: &str) -> Result<Self, MetricsHonestyError> {
        if body.is_empty() || body.as_bytes().contains(&0) {
            return Err(MetricsHonestyError::Prometheus(
                "metrics body is empty or contains NUL".to_owned(),
            ));
        }
        let mut types = BTreeMap::new();
        let mut samples = Vec::new();
        for (line_index, raw) in body.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("# HELP ") {
                continue;
            }
            if let Some(rest) = line.strip_prefix("# TYPE ") {
                let mut fields = rest.split_ascii_whitespace();
                let name = fields.next().unwrap_or_default();
                let kind = fields.next().unwrap_or_default();
                if fields.next().is_some() || !valid_metric_name(name) {
                    return Err(MetricsHonestyError::Prometheus(format!(
                        "invalid TYPE declaration at line {}",
                        line_index + 1
                    )));
                }
                let kind = PrometheusMetricType::parse(kind)?;
                if types.insert(name.to_owned(), kind).is_some() {
                    return Err(MetricsHonestyError::Prometheus(format!(
                        "duplicate TYPE declaration for {name}"
                    )));
                }
                continue;
            }
            if line.starts_with('#') {
                return Err(MetricsHonestyError::Prometheus(format!(
                    "unsupported metadata line {}",
                    line_index + 1
                )));
            }
            samples.push(parse_sample(line, line_index + 1)?);
        }
        if samples.is_empty() {
            return Err(MetricsHonestyError::Prometheus(
                "metrics scrape contains no samples".to_owned(),
            ));
        }
        Ok(Self { types, samples })
    }

    fn inventory(&self) -> Vec<MetricInventoryEntry> {
        let mut counts = BTreeMap::<String, u64>::new();
        for sample in &self.samples {
            *counts.entry(sample.selector.name.clone()).or_default() += 1;
        }
        let mut inventory = counts
            .into_iter()
            .map(|(name, sample_count)| MetricInventoryEntry {
                metric_type: self.types.get(&name).copied(),
                name,
                sample_count,
            })
            .collect::<Vec<_>>();
        inventory.sort_by(|left, right| left.name.cmp(&right.name));
        inventory
    }

    fn exact_value(
        &self,
        selector: &MetricSelector,
        expected_type: PrometheusMetricType,
    ) -> Result<f64, MetricsHonestyError> {
        if self.types.get(&selector.name) != Some(&expected_type) {
            return Err(MetricsHonestyError::Evidence(format!(
                "metric {} is missing or has the wrong TYPE",
                selector.name
            )));
        }
        let matches = self
            .samples
            .iter()
            .filter(|sample| sample.selector == *selector)
            .map(|sample| sample.value)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(MetricsHonestyError::Evidence(format!(
                "selector {selector:?} matched {} samples, expected exactly one",
                matches.len()
            )));
        }
        Ok(matches[0])
    }
}

fn parse_sample(line: &str, line_number: usize) -> Result<ParsedMetricSample, MetricsHonestyError> {
    let split_at = line
        .char_indices()
        .find(|(_, character)| character.is_ascii_whitespace())
        .map(|(index, _)| index)
        .ok_or_else(|| {
            MetricsHonestyError::Prometheus(format!("sample at line {line_number} has no value"))
        })?;
    let identity = &line[..split_at];
    let value_text = line[split_at..]
        .split_ascii_whitespace()
        .next()
        .unwrap_or_default();
    let value = value_text.parse::<f64>().map_err(|error| {
        MetricsHonestyError::Prometheus(format!(
            "sample at line {line_number} has invalid value: {error}"
        ))
    })?;
    if !value.is_finite() {
        return Err(MetricsHonestyError::Prometheus(format!(
            "sample at line {line_number} is non-finite"
        )));
    }
    let (name, labels) = parse_metric_identity(identity, line_number)?;
    Ok(ParsedMetricSample {
        selector: MetricSelector { name, labels },
        value,
    })
}

fn parse_metric_identity(
    identity: &str,
    line_number: usize,
) -> Result<(String, BTreeMap<String, String>), MetricsHonestyError> {
    let Some(open) = identity.find('{') else {
        if !valid_metric_name(identity) {
            return Err(MetricsHonestyError::Prometheus(format!(
                "invalid metric name at line {line_number}"
            )));
        }
        return Ok((identity.to_owned(), BTreeMap::new()));
    };
    if !identity.ends_with('}') || identity[..open].contains('}') {
        return Err(MetricsHonestyError::Prometheus(format!(
            "invalid label braces at line {line_number}"
        )));
    }
    let name = &identity[..open];
    if !valid_metric_name(name) {
        return Err(MetricsHonestyError::Prometheus(format!(
            "invalid metric name at line {line_number}"
        )));
    }
    let labels = parse_labels(&identity[open + 1..identity.len() - 1], line_number)?;
    Ok((name.to_owned(), labels))
}

fn parse_labels(
    input: &str,
    line_number: usize,
) -> Result<BTreeMap<String, String>, MetricsHonestyError> {
    let mut labels = BTreeMap::new();
    let bytes = input.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let key_start = cursor;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
        {
            cursor += 1;
        }
        if cursor == key_start || cursor >= bytes.len() || bytes[cursor] != b'=' {
            return Err(MetricsHonestyError::Prometheus(format!(
                "invalid label key at line {line_number}"
            )));
        }
        let key = &input[key_start..cursor];
        cursor += 1;
        if cursor >= bytes.len() || bytes[cursor] != b'"' {
            return Err(MetricsHonestyError::Prometheus(format!(
                "label {key} is not quoted at line {line_number}"
            )));
        }
        cursor += 1;
        let mut value = String::new();
        let mut closed = false;
        while cursor < bytes.len() {
            match bytes[cursor] {
                b'"' => {
                    cursor += 1;
                    closed = true;
                    break;
                }
                b'\\' => {
                    cursor += 1;
                    let escaped = bytes.get(cursor).copied().ok_or_else(|| {
                        MetricsHonestyError::Prometheus(format!(
                            "truncated label escape at line {line_number}"
                        ))
                    })?;
                    match escaped {
                        b'\\' => value.push('\\'),
                        b'"' => value.push('"'),
                        b'n' => value.push('\n'),
                        _ => {
                            return Err(MetricsHonestyError::Prometheus(format!(
                                "unsupported label escape at line {line_number}"
                            )))
                        }
                    }
                    cursor += 1;
                }
                byte if byte.is_ascii_control() => {
                    return Err(MetricsHonestyError::Prometheus(format!(
                        "control byte in label at line {line_number}"
                    )))
                }
                byte => {
                    value.push(char::from(byte));
                    cursor += 1;
                }
            }
        }
        if !closed || labels.insert(key.to_owned(), value).is_some() {
            return Err(MetricsHonestyError::Prometheus(format!(
                "unterminated or duplicate label at line {line_number}"
            )));
        }
        if cursor < bytes.len() {
            if bytes[cursor] != b',' {
                return Err(MetricsHonestyError::Prometheus(format!(
                    "labels are not comma-separated at line {line_number}"
                )));
            }
            cursor += 1;
            if cursor == bytes.len() {
                return Err(MetricsHonestyError::Prometheus(format!(
                    "trailing label comma at line {line_number}"
                )));
            }
        }
    }
    Ok(labels)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawMetricsScrape {
    pub schema_version: u32,
    pub binding_sha256: String,
    pub admin_endpoint: SocketAddr,
    pub exporter_path: String,
    pub request_sha256: String,
    pub started_unix_nanos: u64,
    pub finished_unix_nanos: u64,
    pub elapsed_monotonic_nanos: u64,
    pub status_code: u16,
    pub content_type: String,
    pub raw_http_response: String,
    pub raw_http_response_sha256: String,
    pub raw_body: String,
    pub raw_body_sha256: String,
    pub inventory: Vec<MetricInventoryEntry>,
}

impl RawMetricsScrape {
    fn validate(
        &self,
        scenario: &MetricsHonestyScenario,
        binding: &LiveDaemonMetricsBinding,
    ) -> Result<PrometheusDocument, MetricsHonestyError> {
        let request = metrics_request(binding.admin_endpoint, &scenario.exporter_path);
        let parsed = parse_http_response(self.raw_http_response.as_bytes())?;
        let body = String::from_utf8(parsed.body).map_err(|error| {
            MetricsHonestyError::Scrape(format!("metrics body is not UTF-8: {error}"))
        })?;
        let document = PrometheusDocument::parse(&body)?;
        if self.schema_version != PERF_SCHEMA_VERSION
            || self.binding_sha256 != binding.digest()?
            || self.admin_endpoint != binding.admin_endpoint
            || self.exporter_path != scenario.exporter_path
            || self.request_sha256 != sha256(&request)
            || self.started_unix_nanos == 0
            || self.finished_unix_nanos < self.started_unix_nanos
            || self.elapsed_monotonic_nanos == 0
            || self.status_code != 200
            || !self.content_type.starts_with("text/plain")
            || self.raw_http_response.len() > scenario.maximum_scrape_bytes
            || self.raw_http_response_sha256 != sha256(self.raw_http_response.as_bytes())
            || self.raw_body != body
            || self.raw_body_sha256 != sha256(body.as_bytes())
            || self.inventory != document.inventory()
            || parsed.status_code != self.status_code
            || parsed.content_type != self.content_type
        {
            return Err(MetricsHonestyError::Evidence(
                "raw scrape bytes/hash/timestamps/HTTP metadata/inventory are not self-consistent"
                    .to_owned(),
            ));
        }
        Ok(document)
    }
}

struct ParsedHttpResponse {
    status_code: u16,
    content_type: String,
    body: Vec<u8>,
}

fn parse_http_response(raw: &[u8]) -> Result<ParsedHttpResponse, MetricsHonestyError> {
    let header_end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| {
            MetricsHonestyError::Scrape("HTTP header terminator is absent".to_owned())
        })?;
    if header_end > MAX_HTTP_HEADER_BYTES {
        return Err(MetricsHonestyError::Scrape(
            "HTTP response headers exceed the bound".to_owned(),
        ));
    }
    let headers = std::str::from_utf8(&raw[..header_end]).map_err(|error| {
        MetricsHonestyError::Scrape(format!("HTTP headers are not UTF-8: {error}"))
    })?;
    let mut lines = headers.split("\r\n");
    let status = lines
        .next()
        .ok_or_else(|| MetricsHonestyError::Scrape("HTTP status line is absent".to_owned()))?;
    let mut status_fields = status.split_ascii_whitespace();
    if status_fields.next() != Some("HTTP/1.1") {
        return Err(MetricsHonestyError::Scrape(
            "metrics response is not HTTP/1.1".to_owned(),
        ));
    }
    let status_code = status_fields
        .next()
        .ok_or_else(|| MetricsHonestyError::Scrape("HTTP status code is absent".to_owned()))?
        .parse::<u16>()
        .map_err(|error| MetricsHonestyError::Scrape(format!("invalid status code: {error}")))?;
    let mut content_type = None;
    let mut content_length = None;
    let mut chunked = false;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or_else(|| {
            MetricsHonestyError::Scrape("malformed HTTP response header".to_owned())
        })?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "content-type" => content_type = Some(value.to_owned()),
            "content-length" => {
                content_length = Some(value.parse::<usize>().map_err(|error| {
                    MetricsHonestyError::Scrape(format!("invalid Content-Length: {error}"))
                })?)
            }
            "transfer-encoding" if value.eq_ignore_ascii_case("chunked") => chunked = true,
            _ => {}
        }
    }
    let encoded_body = &raw[header_end + 4..];
    let body = if chunked {
        decode_chunked_body(encoded_body)?
    } else {
        if let Some(expected) = content_length {
            if encoded_body.len() != expected {
                return Err(MetricsHonestyError::Scrape(format!(
                    "Content-Length {expected} differs from {} received bytes",
                    encoded_body.len()
                )));
            }
        }
        encoded_body.to_vec()
    };
    Ok(ParsedHttpResponse {
        status_code,
        content_type: content_type
            .ok_or_else(|| MetricsHonestyError::Scrape("Content-Type is absent".to_owned()))?,
        body,
    })
}

fn decode_chunked_body(mut encoded: &[u8]) -> Result<Vec<u8>, MetricsHonestyError> {
    let mut body = Vec::new();
    loop {
        let line_end = encoded
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| MetricsHonestyError::Scrape("invalid chunk header".to_owned()))?;
        let size_text = std::str::from_utf8(&encoded[..line_end]).map_err(|error| {
            MetricsHonestyError::Scrape(format!("chunk size is not UTF-8: {error}"))
        })?;
        let size_text = size_text.split(';').next().unwrap_or_default();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|error| MetricsHonestyError::Scrape(format!("invalid chunk size: {error}")))?;
        encoded = &encoded[line_end + 2..];
        if size == 0 {
            if encoded != b"\r\n" && !encoded.starts_with(b"\r\n") {
                return Err(MetricsHonestyError::Scrape(
                    "chunked terminator is malformed".to_owned(),
                ));
            }
            return Ok(body);
        }
        if encoded.len() < size + 2 || &encoded[size..size + 2] != b"\r\n" {
            return Err(MetricsHonestyError::Scrape(
                "chunk data is truncated".to_owned(),
            ));
        }
        body.extend_from_slice(&encoded[..size]);
        encoded = &encoded[size + 2..];
    }
}

pub async fn scrape_live_daemon_metrics(
    scenario: &MetricsHonestyScenario,
    binding: &LiveDaemonMetricsBinding,
) -> Result<RawMetricsScrape, MetricsHonestyError> {
    scenario.validate()?;
    binding.validate(scenario)?;
    let request = metrics_request(binding.admin_endpoint, &scenario.exporter_path);
    let started_unix_nanos = unix_nanos()?;
    let started = Instant::now();
    let mut stream = time::timeout(
        scenario.connect_timeout(),
        TcpStream::connect(binding.admin_endpoint),
    )
    .await
    .map_err(|_| MetricsHonestyError::Scrape("metrics connect timed out".to_owned()))??;
    time::timeout(scenario.read_timeout(), stream.write_all(&request))
        .await
        .map_err(|_| MetricsHonestyError::Scrape("metrics request write timed out".to_owned()))??;
    let mut raw = Vec::new();
    let read_result = time::timeout(scenario.read_timeout(), async {
        let mut chunk = [0_u8; 16 * 1024];
        loop {
            let read = stream.read(&mut chunk).await?;
            if read == 0 {
                break;
            }
            if raw.len().saturating_add(read) > scenario.maximum_scrape_bytes {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "metrics response exceeds maximum_scrape_bytes",
                ));
            }
            raw.extend_from_slice(&chunk[..read]);
        }
        Ok::<(), std::io::Error>(())
    })
    .await
    .map_err(|_| MetricsHonestyError::Scrape("metrics response read timed out".to_owned()))?;
    read_result?;
    let elapsed = started.elapsed();
    let finished_unix_nanos = unix_nanos()?;
    let parsed = parse_http_response(&raw)?;
    if parsed.status_code != 200 || !parsed.content_type.starts_with("text/plain") {
        return Err(MetricsHonestyError::Scrape(format!(
            "metrics endpoint returned status {} and content type {:?}",
            parsed.status_code, parsed.content_type
        )));
    }
    let body = String::from_utf8(parsed.body).map_err(|error| {
        MetricsHonestyError::Scrape(format!("metrics body is not UTF-8: {error}"))
    })?;
    let document = PrometheusDocument::parse(&body)?;
    let raw_http_response = String::from_utf8(raw).map_err(|error| {
        MetricsHonestyError::Scrape(format!("raw metrics HTTP response is not UTF-8: {error}"))
    })?;
    let evidence = RawMetricsScrape {
        schema_version: PERF_SCHEMA_VERSION,
        binding_sha256: binding.digest()?,
        admin_endpoint: binding.admin_endpoint,
        exporter_path: scenario.exporter_path.clone(),
        request_sha256: sha256(&request),
        started_unix_nanos,
        finished_unix_nanos,
        elapsed_monotonic_nanos: u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX),
        status_code: parsed.status_code,
        content_type: parsed.content_type,
        raw_http_response_sha256: sha256(raw_http_response.as_bytes()),
        raw_http_response,
        raw_body_sha256: sha256(body.as_bytes()),
        raw_body: body,
        inventory: document.inventory(),
    };
    evidence.validate(scenario, binding)?;
    Ok(evidence)
}

fn metrics_request(endpoint: SocketAddr, path: &str) -> Vec<u8> {
    format!(
        "GET {path} HTTP/1.1\r\nHost: {endpoint}\r\nAccept: text/plain\r\nConnection: close\r\n\r\n"
    )
    .into_bytes()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverAccounting {
    pub attempted_operations: u64,
    pub successful_operations: u64,
    pub errors: u64,
    pub timeouts: u64,
    pub rejections: u64,
    pub scheduled_latency_sample_count: u64,
    pub scheduled_latency_evidence_sha256: String,
}

impl ObserverAccounting {
    pub fn from_open_loop(observation: &OpenLoopObservation) -> Result<Self, MetricsHonestyError> {
        let accounting = Self {
            attempted_operations: observation.offered,
            successful_operations: observation.successes,
            errors: observation.errors,
            timeouts: observation.timeouts,
            rejections: observation.rejections,
            scheduled_latency_sample_count: observation.latency.samples,
            scheduled_latency_evidence_sha256: sha256(&serde_json::to_vec(&observation.latency)?),
        };
        accounting.validate()?;
        if observation.started != observation.offered
            || observation.completed != observation.offered
            || !observation.backlog_drained
            || observation.latency.samples != observation.completed
        {
            return Err(MetricsHonestyError::Evidence(
                "W9 observer probe did not start, classify, drain, and retain latency for every scheduled offer"
                    .to_owned(),
            ));
        }
        Ok(accounting)
    }

    fn validate(&self) -> Result<(), MetricsHonestyError> {
        let classified = self
            .successful_operations
            .checked_add(self.errors)
            .and_then(|value| value.checked_add(self.timeouts))
            .and_then(|value| value.checked_add(self.rejections));
        if classified != Some(self.attempted_operations)
            || self.attempted_operations == 0
            || self.scheduled_latency_sample_count != self.attempted_operations
            || !is_sha256(&self.scheduled_latency_evidence_sha256)
        {
            return Err(MetricsHonestyError::Evidence(
                "independent observer accounting does not conserve W0 outcomes or scheduled-latency samples"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologySnapshotEvidence {
    pub endpoint: ControlPlaneEndpoint,
    pub source: String,
    pub members: u64,
    pub leader: String,
    pub epoch: u64,
    pub public_snapshot_sha256: String,
}

impl TopologySnapshotEvidence {
    pub fn from_public_snapshot(
        snapshot: &PublicControlPlaneSnapshot,
    ) -> Result<Self, MetricsHonestyError> {
        let overview_leader = snapshot
            .cluster_overview
            .leader
            .as_ref()
            .map(|leader| leader.node_id.as_str());
        let admin_leader = snapshot.admin_status.leader.as_deref();
        if snapshot.admin_status.source != ControlPlaneSource::Live
            || snapshot.cluster_overview.source != ControlPlaneSource::Live
            || admin_leader.is_none()
            || overview_leader != admin_leader
            || snapshot.admin_status.members as usize != snapshot.admin_status.member_ids.len()
            || snapshot.admin_status.members as usize != snapshot.cluster_overview.members.len()
            || snapshot
                .cluster_overview
                .leader
                .as_ref()
                .is_none_or(|leader| leader.epoch != snapshot.admin_status.epoch)
        {
            return Err(MetricsHonestyError::Evidence(
                "W4A topology observer is not a complete same-snapshot live public view".to_owned(),
            ));
        }
        Ok(Self {
            endpoint: snapshot.endpoint.clone(),
            source: "live".to_owned(),
            members: u64::from(snapshot.admin_status.members),
            leader: admin_leader.expect("checked").to_owned(),
            epoch: snapshot.admin_status.epoch,
            public_snapshot_sha256: sha256(&serde_json::to_vec(snapshot)?),
        })
    }

    fn validate(&self, binding: &LiveDaemonMetricsBinding) -> Result<(), MetricsHonestyError> {
        if self.endpoint.node_id != binding.node_id
            || self.endpoint.admin_addr != binding.admin_endpoint
            || self.source != "live"
            || self.members == 0
            || self.leader.trim().is_empty()
            || self.epoch == 0
            || !is_sha256(&self.public_snapshot_sha256)
        {
            return Err(MetricsHonestyError::Evidence(
                "topology snapshot does not bind the exact live W4A endpoint".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverIntervalEvidence {
    pub observer: String,
    pub interval_clock: String,
    pub probe_contract_sha256: String,
    pub started_unix_nanos: u64,
    pub ended_unix_nanos: u64,
    pub elapsed_monotonic_nanos: u64,
    pub raw_open_loop_json: String,
    pub raw_open_loop_sha256: String,
    pub accounting: ObserverAccounting,
    pub topology_before: Option<TopologySnapshotEvidence>,
    pub topology_after: Option<TopologySnapshotEvidence>,
}

impl ObserverIntervalEvidence {
    fn validate(
        &self,
        scenario: &MetricsHonestyScenario,
        binding: &LiveDaemonMetricsBinding,
    ) -> Result<(), MetricsHonestyError> {
        let raw: OpenLoopObservation = serde_json::from_str(&self.raw_open_loop_json)?;
        let expected_accounting = ObserverAccounting::from_open_loop(&raw)?;
        self.accounting.validate()?;
        if self.observer != "w0-open-loop-scheduled-send-independent"
            || self.interval_clock != "monotonic-elapsed-plus-unix-boundary"
            || self.probe_contract_sha256 != scenario.observer_probe.digest()?
            || self.started_unix_nanos == 0
            || self.ended_unix_nanos < self.started_unix_nanos
            || self.elapsed_monotonic_nanos == 0
            || self.raw_open_loop_json
                != String::from_utf8(serde_json::to_vec(&raw)?).expect("JSON is UTF-8")
            || self.raw_open_loop_sha256 != sha256(self.raw_open_loop_json.as_bytes())
            || self.accounting != expected_accounting
            || raw.offered != scenario.observer_probe.operations
            || raw.offered_rate_per_second != scenario.observer_probe.offered_rate_per_second as f64
            || raw.latency.p999_min_samples != scenario.observer_probe.p999_min_samples
        {
            return Err(MetricsHonestyError::Evidence(
                "observer interval lacks the exact raw bounded W0 schedule/accounting/timestamps"
                    .to_owned(),
            ));
        }
        match binding.surface {
            DaemonMetricsSurface::W3NodeResp => {
                if self.topology_before.is_some() || self.topology_after.is_some() {
                    return Err(MetricsHonestyError::Evidence(
                        "W3 node-local RESP evidence may not acquire a topology claim".to_owned(),
                    ));
                }
            }
            DaemonMetricsSurface::W4aControlPlaneAdmin => {
                let before = self.topology_before.as_ref().ok_or_else(|| {
                    MetricsHonestyError::Evidence(
                        "W4A metrics interval has no before topology snapshot".to_owned(),
                    )
                })?;
                let after = self.topology_after.as_ref().ok_or_else(|| {
                    MetricsHonestyError::Evidence(
                        "W4A metrics interval has no after topology snapshot".to_owned(),
                    )
                })?;
                before.validate(binding)?;
                after.validate(binding)?;
            }
        }
        Ok(())
    }
}

/// Non-serializable clock guard that makes observer wall boundaries and
/// monotonic elapsed time come from the harness rather than caller arithmetic.
pub struct ObserverIntervalClock {
    started_unix_nanos: u64,
    started: Instant,
}

impl ObserverIntervalClock {
    pub fn start() -> Result<Self, MetricsHonestyError> {
        Ok(Self {
            started_unix_nanos: unix_nanos()?,
            started: Instant::now(),
        })
    }

    pub fn finish(
        self,
        scenario: &MetricsHonestyScenario,
        observation: &OpenLoopObservation,
        topology_before: Option<TopologySnapshotEvidence>,
        topology_after: Option<TopologySnapshotEvidence>,
    ) -> Result<ObserverIntervalEvidence, MetricsHonestyError> {
        scenario.validate()?;
        let accounting = ObserverAccounting::from_open_loop(observation)?;
        let raw_open_loop_json =
            String::from_utf8(serde_json::to_vec(observation)?).expect("JSON is UTF-8");
        let evidence = ObserverIntervalEvidence {
            observer: "w0-open-loop-scheduled-send-independent".to_owned(),
            interval_clock: "monotonic-elapsed-plus-unix-boundary".to_owned(),
            probe_contract_sha256: scenario.observer_probe.digest()?,
            started_unix_nanos: self.started_unix_nanos,
            ended_unix_nanos: unix_nanos()?,
            elapsed_monotonic_nanos: u64::try_from(self.started.elapsed().as_nanos())
                .unwrap_or(u64::MAX),
            raw_open_loop_sha256: sha256(raw_open_loop_json.as_bytes()),
            raw_open_loop_json,
            accounting,
            topology_before,
            topology_after,
        };
        Ok(evidence)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricField {
    Operations,
    Rejections,
    InternalServiceLatency,
    TopologyMembers,
    TopologyLeader,
    TopologyEpoch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotAvailableEvidence {
    pub field: MetricField,
    pub status: String,
    pub reason: String,
    pub metric_selector: Option<MetricSelector>,
    pub agreement_claim: bool,
}

impl NotAvailableEvidence {
    fn new(field: MetricField, reason: &str) -> Self {
        Self {
            field,
            status: "not_available".to_owned(),
            reason: reason.to_owned(),
            metric_selector: None,
            agreement_claim: false,
        }
    }

    fn validate(&self) -> Result<(), MetricsHonestyError> {
        if self.status != "not_available"
            || self.reason.trim().is_empty()
            || self.metric_selector.is_some()
            || self.agreement_claim
        {
            return Err(MetricsHonestyError::Evidence(
                "an unavailable field must carry a reason and no metric/agreement claim".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterAgreement {
    pub field: MetricField,
    pub selector: MetricSelector,
    pub reported_before: u64,
    pub reported_after: u64,
    pub reported_delta: u64,
    pub observer_delta: u64,
    pub absolute_error: u64,
    pub absolute_tolerance: u64,
    pub allowed_absolute_error: u64,
    pub relative_tolerance_millionths: u32,
    pub agrees: bool,
}

impl CounterAgreement {
    pub fn from_scrapes(
        scenario: &MetricsHonestyScenario,
        field: MetricField,
        selector: MetricSelector,
        before: &RawMetricsScrape,
        after: &RawMetricsScrape,
        observer_delta: u64,
    ) -> Result<Self, MetricsHonestyError> {
        if !matches!(field, MetricField::Operations | MetricField::Rejections) {
            return Err(MetricsHonestyError::Evidence(
                "counter agreement is allowed only for operation/rejection fields".to_owned(),
            ));
        }
        let before_doc = PrometheusDocument::parse(&before.raw_body)?;
        let after_doc = PrometheusDocument::parse(&after.raw_body)?;
        let reported_before = exact_u64_metric(
            before_doc.exact_value(&selector, PrometheusMetricType::Counter)?,
            &selector.name,
        )?;
        let reported_after = exact_u64_metric(
            after_doc.exact_value(&selector, PrometheusMetricType::Counter)?,
            &selector.name,
        )?;
        counter_agreement(
            field,
            selector,
            reported_before,
            reported_after,
            observer_delta,
            scenario.counter_absolute_tolerance,
            scenario.counter_relative_tolerance_millionths,
        )
    }

    fn validate(&self) -> Result<(), MetricsHonestyError> {
        let recomputed = counter_agreement(
            self.field,
            self.selector.clone(),
            self.reported_before,
            self.reported_after,
            self.observer_delta,
            self.absolute_tolerance,
            self.relative_tolerance_millionths,
        )?;
        if *self != recomputed || !self.agrees {
            return Err(MetricsHonestyError::Evidence(
                "server counter and independent observer do not agree within tolerance".to_owned(),
            ));
        }
        Ok(())
    }
}

fn counter_agreement(
    field: MetricField,
    selector: MetricSelector,
    reported_before: u64,
    reported_after: u64,
    observer_delta: u64,
    absolute_tolerance: u64,
    relative_tolerance_millionths: u32,
) -> Result<CounterAgreement, MetricsHonestyError> {
    if reported_after < reported_before || relative_tolerance_millionths > 1_000_000 {
        return Err(MetricsHonestyError::Evidence(
            "counter reset or invalid tolerance inside comparison interval".to_owned(),
        ));
    }
    let reported_delta = reported_after - reported_before;
    let absolute_error = reported_delta.abs_diff(observer_delta);
    let relative = observer_delta
        .saturating_mul(u64::from(relative_tolerance_millionths))
        .saturating_add(999_999)
        / 1_000_000;
    let allowed_absolute_error = absolute_tolerance.max(relative);
    Ok(CounterAgreement {
        field,
        selector,
        reported_before,
        reported_after,
        reported_delta,
        observer_delta,
        absolute_error,
        absolute_tolerance,
        allowed_absolute_error,
        relative_tolerance_millionths,
        agrees: absolute_error <= allowed_absolute_error,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GaugeAgreement {
    pub field: MetricField,
    pub before_selector: MetricSelector,
    pub after_selector: MetricSelector,
    pub reported_before: u64,
    pub reported_after: u64,
    pub observer_before: u64,
    pub observer_after: u64,
    pub absolute_tolerance: u64,
    pub agrees: bool,
}

impl GaugeAgreement {
    fn derive(
        field: MetricField,
        before_selector: MetricSelector,
        after_selector: MetricSelector,
        before_doc: &PrometheusDocument,
        after_doc: &PrometheusDocument,
        observer: (u64, u64),
        absolute_tolerance: u64,
    ) -> Result<Self, MetricsHonestyError> {
        let (observer_before, observer_after) = observer;
        let reported_before = exact_u64_metric(
            before_doc.exact_value(&before_selector, PrometheusMetricType::Gauge)?,
            &before_selector.name,
        )?;
        let reported_after = exact_u64_metric(
            after_doc.exact_value(&after_selector, PrometheusMetricType::Gauge)?,
            &after_selector.name,
        )?;
        Ok(Self {
            field,
            before_selector,
            after_selector,
            reported_before,
            reported_after,
            observer_before,
            observer_after,
            absolute_tolerance,
            agrees: reported_before.abs_diff(observer_before) <= absolute_tolerance
                && reported_after.abs_diff(observer_after) <= absolute_tolerance,
        })
    }

    fn validate(
        &self,
        before_doc: &PrometheusDocument,
        after_doc: &PrometheusDocument,
    ) -> Result<(), MetricsHonestyError> {
        let recomputed = Self::derive(
            self.field,
            self.before_selector.clone(),
            self.after_selector.clone(),
            before_doc,
            after_doc,
            (self.observer_before, self.observer_after),
            self.absolute_tolerance,
        )?;
        if *self != recomputed || !self.agrees {
            return Err(MetricsHonestyError::Evidence(
                "exported topology gauge disagrees with the same-interval public snapshot"
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "availability", content = "evidence", rename_all = "snake_case")]
pub enum MetricCoverageEvidence {
    NotAvailable(NotAvailableEvidence),
    CounterCompared(CounterAgreement),
    GaugeCompared(GaugeAgreement),
}

impl MetricCoverageEvidence {
    fn field(&self) -> MetricField {
        match self {
            Self::NotAvailable(value) => value.field,
            Self::CounterCompared(value) => value.field,
            Self::GaugeCompared(value) => value.field,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatencyNonConflationEvidence {
    pub observer_metric: String,
    pub observer_includes_scheduler_queue_delay: bool,
    pub server_internal_service_metric_status: String,
    pub matching_observer_interval_required_if_available: bool,
    pub equality_with_scheduled_latency_claimed: bool,
}

impl LatencyNonConflationEvidence {
    fn exact() -> Self {
        Self {
            observer_metric: "scheduled-send-to-completion-latency".to_owned(),
            observer_includes_scheduler_queue_delay: true,
            server_internal_service_metric_status: "not_available".to_owned(),
            matching_observer_interval_required_if_available: true,
            equality_with_scheduled_latency_claimed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsWindowEvidence {
    pub schema_version: u32,
    pub window_id: String,
    pub binding: LiveDaemonMetricsBinding,
    pub before: RawMetricsScrape,
    pub observer: ObserverIntervalEvidence,
    pub after: RawMetricsScrape,
    pub coverage: Vec<MetricCoverageEvidence>,
    pub latency_boundary: LatencyNonConflationEvidence,
}

impl MetricsWindowEvidence {
    pub fn validate(&self, scenario: &MetricsHonestyScenario) -> Result<(), MetricsHonestyError> {
        self.binding.validate(scenario)?;
        let before_doc = self.before.validate(scenario, &self.binding)?;
        let after_doc = self.after.validate(scenario, &self.binding)?;
        self.observer.validate(scenario, &self.binding)?;
        if self.schema_version != PERF_SCHEMA_VERSION
            || self.window_id.trim().is_empty()
            || self.before.finished_unix_nanos > self.observer.started_unix_nanos
            || self.observer.ended_unix_nanos > self.after.started_unix_nanos
            || self.latency_boundary != LatencyNonConflationEvidence::exact()
        {
            return Err(MetricsHonestyError::Evidence(
                "metrics scrapes do not bracket the independent interval or latency semantics were conflated"
                    .to_owned(),
            ));
        }
        let fields = self
            .coverage
            .iter()
            .map(MetricCoverageEvidence::field)
            .collect::<BTreeSet<_>>();
        let expected = BTreeSet::from([
            MetricField::Operations,
            MetricField::Rejections,
            MetricField::InternalServiceLatency,
            MetricField::TopologyMembers,
            MetricField::TopologyLeader,
            MetricField::TopologyEpoch,
        ]);
        if fields != expected || self.coverage.len() != expected.len() {
            return Err(MetricsHonestyError::Evidence(
                "coverage table must contain each W9 field exactly once".to_owned(),
            ));
        }
        for row in &self.coverage {
            match row {
                MetricCoverageEvidence::NotAvailable(value) => value.validate()?,
                MetricCoverageEvidence::CounterCompared(value) => value.validate()?,
                MetricCoverageEvidence::GaugeCompared(value) => {
                    value.validate(&before_doc, &after_doc)?
                }
            }
        }
        self.validate_surface_coverage()?;
        Ok(())
    }

    fn validate_surface_coverage(&self) -> Result<(), MetricsHonestyError> {
        let row = |field| {
            self.coverage
                .iter()
                .find(|candidate| candidate.field() == field)
                .expect("complete field set checked")
        };
        let expect_na = |field, reason: &str| -> Result<(), MetricsHonestyError> {
            match row(field) {
                MetricCoverageEvidence::NotAvailable(value) if value.reason == reason => Ok(()),
                _ => Err(MetricsHonestyError::Evidence(format!(
                    "{} field {field:?} does not preserve its honest unavailable reason",
                    self.binding.surface.as_str()
                ))),
            }
        };
        match self.binding.surface {
            DaemonMetricsSurface::W3NodeResp => {
                expect_na(MetricField::Operations, W3_OPERATIONS_NA)?;
                expect_na(MetricField::Rejections, W3_REJECTIONS_NA)?;
                expect_na(MetricField::InternalServiceLatency, W3_LATENCY_NA)?;
                for field in [
                    MetricField::TopologyMembers,
                    MetricField::TopologyLeader,
                    MetricField::TopologyEpoch,
                ] {
                    expect_na(field, W3_TOPOLOGY_NA)?;
                }
            }
            DaemonMetricsSurface::W4aControlPlaneAdmin => {
                expect_na(MetricField::Operations, W4_OPERATIONS_NA)?;
                expect_na(MetricField::Rejections, W4_REJECTIONS_NA)?;
                expect_na(MetricField::InternalServiceLatency, W4_LATENCY_NA)?;
                let before = self.observer.topology_before.as_ref().expect("validated");
                let after = self.observer.topology_after.as_ref().expect("validated");
                validate_topology_row(
                    row(MetricField::TopologyMembers),
                    MetricField::TopologyMembers,
                    "hydracache_cluster_members",
                    before.members,
                    after.members,
                    "",
                    "",
                )?;
                validate_topology_row(
                    row(MetricField::TopologyLeader),
                    MetricField::TopologyLeader,
                    "hydracache_cluster_leader",
                    1,
                    1,
                    &before.leader,
                    &after.leader,
                )?;
                validate_topology_row(
                    row(MetricField::TopologyEpoch),
                    MetricField::TopologyEpoch,
                    "hydracache_cluster_epoch",
                    before.epoch,
                    after.epoch,
                    "",
                    "",
                )?;
            }
        }
        Ok(())
    }
}

fn validate_topology_row(
    row: &MetricCoverageEvidence,
    field: MetricField,
    metric_name: &str,
    observer_before: u64,
    observer_after: u64,
    leader_before: &str,
    leader_after: &str,
) -> Result<(), MetricsHonestyError> {
    let MetricCoverageEvidence::GaugeCompared(gauge) = row else {
        return Err(MetricsHonestyError::Evidence(format!(
            "W4A {field:?} must be an exported gauge comparison"
        )));
    };
    let expected_before = if field == MetricField::TopologyLeader {
        MetricSelector::live_leader(leader_before)
    } else {
        MetricSelector::live_topology(metric_name)
    };
    let expected_after = if field == MetricField::TopologyLeader {
        MetricSelector::live_leader(leader_after)
    } else {
        MetricSelector::live_topology(metric_name)
    };
    if gauge.field != field
        || gauge.before_selector != expected_before
        || gauge.after_selector != expected_after
        || gauge.observer_before != observer_before
        || gauge.observer_after != observer_after
        || gauge.absolute_tolerance != 0
        || !gauge.agrees
    {
        return Err(MetricsHonestyError::Evidence(format!(
            "W4A {field:?} comparison is not exact and same-interval"
        )));
    }
    Ok(())
}

/// Owns the first real exporter scrape. The caller runs the W0-measured
/// interval, then `finish` captures the second scrape and derives all claims.
pub struct MetricsWindowRecorder {
    scenario: MetricsHonestyScenario,
    window_id: String,
    binding: LiveDaemonMetricsBinding,
    before: RawMetricsScrape,
}

impl MetricsWindowRecorder {
    pub async fn begin(
        scenario: &MetricsHonestyScenario,
        window_id: impl Into<String>,
        binding: LiveDaemonMetricsBinding,
    ) -> Result<Self, MetricsHonestyError> {
        scenario.validate()?;
        binding.validate(scenario)?;
        let window_id = window_id.into();
        if window_id.trim().is_empty() {
            return Err(MetricsHonestyError::Evidence(
                "metrics window id is empty".to_owned(),
            ));
        }
        let before = scrape_live_daemon_metrics(scenario, &binding).await?;
        Ok(Self {
            scenario: scenario.clone(),
            window_id,
            binding,
            before,
        })
    }

    pub async fn finish(
        self,
        observer: ObserverIntervalEvidence,
    ) -> Result<MetricsWindowEvidence, MetricsHonestyError> {
        observer.validate(&self.scenario, &self.binding)?;
        if self.before.finished_unix_nanos > observer.started_unix_nanos {
            return Err(MetricsHonestyError::Evidence(
                "observer interval started before the first metrics scrape completed".to_owned(),
            ));
        }
        let after = scrape_live_daemon_metrics(&self.scenario, &self.binding).await?;
        if observer.ended_unix_nanos > after.started_unix_nanos {
            return Err(MetricsHonestyError::Evidence(
                "second metrics scrape did not start after the observer interval".to_owned(),
            ));
        }
        let before_doc = PrometheusDocument::parse(&self.before.raw_body)?;
        let after_doc = PrometheusDocument::parse(&after.raw_body)?;
        let coverage = match self.binding.surface {
            DaemonMetricsSurface::W3NodeResp => w3_coverage(),
            DaemonMetricsSurface::W4aControlPlaneAdmin => {
                w4_coverage(&self.scenario, &observer, &before_doc, &after_doc)?
            }
        };
        let evidence = MetricsWindowEvidence {
            schema_version: PERF_SCHEMA_VERSION,
            window_id: self.window_id,
            binding: self.binding,
            before: self.before,
            observer,
            after,
            coverage,
            latency_boundary: LatencyNonConflationEvidence::exact(),
        };
        evidence.validate(&self.scenario)?;
        Ok(evidence)
    }
}

fn w3_coverage() -> Vec<MetricCoverageEvidence> {
    vec![
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::Operations,
            W3_OPERATIONS_NA,
        )),
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::Rejections,
            W3_REJECTIONS_NA,
        )),
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::InternalServiceLatency,
            W3_LATENCY_NA,
        )),
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::TopologyMembers,
            W3_TOPOLOGY_NA,
        )),
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::TopologyLeader,
            W3_TOPOLOGY_NA,
        )),
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::TopologyEpoch,
            W3_TOPOLOGY_NA,
        )),
    ]
}

fn w4_coverage(
    scenario: &MetricsHonestyScenario,
    observer: &ObserverIntervalEvidence,
    before_doc: &PrometheusDocument,
    after_doc: &PrometheusDocument,
) -> Result<Vec<MetricCoverageEvidence>, MetricsHonestyError> {
    let before = observer.topology_before.as_ref().ok_or_else(|| {
        MetricsHonestyError::Evidence("W4A observer lacks before topology".to_owned())
    })?;
    let after = observer.topology_after.as_ref().ok_or_else(|| {
        MetricsHonestyError::Evidence("W4A observer lacks after topology".to_owned())
    })?;
    let members_selector = MetricSelector::live_topology("hydracache_cluster_members");
    let epoch_selector = MetricSelector::live_topology("hydracache_cluster_epoch");
    Ok(vec![
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::Operations,
            W4_OPERATIONS_NA,
        )),
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::Rejections,
            W4_REJECTIONS_NA,
        )),
        MetricCoverageEvidence::NotAvailable(NotAvailableEvidence::new(
            MetricField::InternalServiceLatency,
            W4_LATENCY_NA,
        )),
        MetricCoverageEvidence::GaugeCompared(GaugeAgreement::derive(
            MetricField::TopologyMembers,
            members_selector.clone(),
            members_selector,
            before_doc,
            after_doc,
            (before.members, after.members),
            scenario.topology_gauge_absolute_tolerance,
        )?),
        MetricCoverageEvidence::GaugeCompared(GaugeAgreement::derive(
            MetricField::TopologyLeader,
            MetricSelector::live_leader(&before.leader),
            MetricSelector::live_leader(&after.leader),
            before_doc,
            after_doc,
            (1, 1),
            scenario.topology_gauge_absolute_tolerance,
        )?),
        MetricCoverageEvidence::GaugeCompared(GaugeAgreement::derive(
            MetricField::TopologyEpoch,
            epoch_selector.clone(),
            epoch_selector,
            before_doc,
            after_doc,
            (before.epoch, after.epoch),
            scenario.topology_gauge_absolute_tolerance,
        )?),
    ])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredecessorProcessReceipt {
    pub node_id: String,
    pub pid: u32,
    pub admin_endpoint: SocketAddr,
    pub server_binary_sha256: String,
    pub loadgen_binary_sha256: Option<String>,
    pub killed_and_waited: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivedArtifactReceipt {
    pub logical_name: String,
    pub canonical_path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

impl ArchivedArtifactReceipt {
    fn capture(logical_name: &str, path: &Path) -> Result<(Self, Vec<u8>), MetricsHonestyError> {
        if logical_name.trim().is_empty() || !path.is_absolute() {
            return Err(MetricsHonestyError::Binding(
                "predecessor artifact requires a logical name and absolute path".to_owned(),
            ));
        }
        let supplied_parent = path.parent().ok_or_else(|| {
            MetricsHonestyError::Binding(
                "predecessor artifact absolute path has no parent".to_owned(),
            )
        })?;
        let canonical_parent = fs::canonicalize(supplied_parent)?;
        let canonical_path = fs::canonicalize(path)?;
        let metadata = fs::metadata(&canonical_path)?;
        if canonical_path.parent() != Some(canonical_parent.as_path())
            || canonical_path.file_name() != path.file_name()
            || !metadata.is_file()
            || metadata.len() == 0
            || metadata.len() > MAX_PREDECESSOR_ARTIFACT_BYTES
        {
            return Err(MetricsHonestyError::Binding(format!(
                "predecessor artifact {logical_name} is not a bounded canonical regular file"
            )));
        }
        let contents = fs::read(&canonical_path)?;
        if u64::try_from(contents.len()).unwrap_or(u64::MAX) != metadata.len() {
            return Err(MetricsHonestyError::Binding(format!(
                "predecessor artifact {logical_name} changed length while it was captured"
            )));
        }
        let receipt = Self {
            logical_name: logical_name.to_owned(),
            canonical_path,
            bytes: metadata.len(),
            sha256: sha256(&contents),
        };
        Ok((receipt, contents))
    }

    fn validate_archived(&self) -> Result<Vec<u8>, MetricsHonestyError> {
        let (observed, contents) = Self::capture(&self.logical_name, &self.canonical_path)?;
        if observed != *self || !is_sha256(&self.sha256) {
            return Err(MetricsHonestyError::Binding(format!(
                "archived predecessor artifact {} no longer matches path/length/SHA",
                self.logical_name
            )));
        }
        Ok(contents)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsPredecessorReceipt {
    pub schema_version: u32,
    pub surface: DaemonMetricsSurface,
    pub report_sha256: String,
    pub lifecycle_sha256: String,
    pub external_report_sha256: Option<String>,
    pub suite_receipt_sha256: Option<String>,
    pub source_commit: String,
    pub cargo_lock_sha256: Option<String>,
    pub runner_profile: String,
    pub runner_identity_sha256: String,
    pub prebuild_manifest_sha256: String,
    pub prebuild_contract_sha256: String,
    pub capability_receipt_sha256: String,
    pub processes: Vec<PredecessorProcessReceipt>,
    pub artifacts: Vec<ArchivedArtifactReceipt>,
}

impl MetricsPredecessorReceipt {
    /// Capture the exact canonical W3 four-artifact set. The stored paths are
    /// re-opened by `validate_archived`, so later replacement/removal is loud.
    pub fn from_w3_paths(
        open_loop_path: &Path,
        lifecycle_path: &Path,
        external_path: &Path,
        suite_receipt_path: &Path,
    ) -> Result<Self, MetricsHonestyError> {
        let (open_loop_artifact, open_loop_bytes) =
            ArchivedArtifactReceipt::capture(W3_OPEN_LOOP_ARTIFACT, open_loop_path)?;
        let (lifecycle_artifact, lifecycle_bytes) =
            ArchivedArtifactReceipt::capture(W3_LIFECYCLE_ARTIFACT, lifecycle_path)?;
        let (external_artifact, external_bytes) =
            ArchivedArtifactReceipt::capture(W3_EXTERNAL_ARTIFACT, external_path)?;
        let (suite_artifact, suite_receipt_bytes) =
            ArchivedArtifactReceipt::capture(W3_SUITE_RECEIPT_ARTIFACT, suite_receipt_path)?;
        let artifacts = vec![
            open_loop_artifact,
            external_artifact,
            lifecycle_artifact,
            suite_artifact,
        ];
        let receipt = Self::derive_w3(
            &open_loop_bytes,
            &lifecycle_bytes,
            &external_bytes,
            &suite_receipt_bytes,
            artifacts,
        )?;
        receipt.validate_archived(None)?;
        Ok(receipt)
    }

    fn derive_w3(
        open_loop_bytes: &[u8],
        lifecycle_bytes: &[u8],
        external_bytes: &[u8],
        suite_receipt_bytes: &[u8],
        artifacts: Vec<ArchivedArtifactReceipt>,
    ) -> Result<Self, MetricsHonestyError> {
        let report: PerfReport = serde_json::from_slice(open_loop_bytes)?;
        let lifecycle: RespDaemonEvidence = serde_json::from_slice(lifecycle_bytes)?;
        let external: RedisBenchmarkEvidence = serde_json::from_slice(external_bytes)?;
        let suite: RespReferenceSuiteReceipt = serde_json::from_slice(suite_receipt_bytes)?;
        validate_daemon_exporter_surface(&report.surface)?;
        let problems = report.validation_problems();
        if !problems.is_empty()
            || !report.stable
            || report.run_mode != EvidenceRunMode::ReferenceEvidence
            || report.surface.surface_kind != "node-resp"
        {
            return Err(MetricsHonestyError::Binding(format!(
                "W3 predecessor is not stable exact reference evidence: {problems:?}"
            )));
        }
        let capability = report.resp_endpoint_capability.as_ref().ok_or_else(|| {
            MetricsHonestyError::Binding("W3 predecessor lost endpoint capability".to_owned())
        })?;
        let capability_sha256 = capability
            .digest()
            .map_err(|error| MetricsHonestyError::Binding(error.to_string()))?;
        let suite_evidence = RespReferenceSuiteEvidence {
            open_loop: report.clone(),
            external,
            daemon: lifecycle.clone(),
        };
        suite
            .validate(
                &suite_evidence,
                open_loop_bytes,
                external_bytes,
                lifecycle_bytes,
            )
            .map_err(|error| MetricsHonestyError::Binding(error.to_string()))?;
        let expected_suite_sha256 = sha256(&serde_json::to_vec(&suite.payload)?);
        let build_binaries = report
            .build
            .binary_sha256
            .iter()
            .cloned()
            .collect::<BTreeMap<_, _>>();
        if suite.receipt_sha256 != expected_suite_sha256
            || suite.payload.schema_version != PERF_SCHEMA_VERSION
            || suite.payload.source_commit != report.source.git_commit
            || suite.payload.prebuild_manifest_sha256 != report.build.prebuild_manifest_sha256
            || suite.payload.selected_endpoint != capability.selected_endpoint
            || suite.payload.endpoint_capability_sha256 != capability_sha256
            || suite.payload.open_loop_report_sha256 != sha256(open_loop_bytes)
            || suite.payload.external_report_sha256 != sha256(external_bytes)
            || suite.payload.daemon_lifecycle_sha256 != sha256(lifecycle_bytes)
            || lifecycle.pid != capability.pid
            || lifecycle.repeat_index != capability.repeat_index
            || lifecycle.admin_endpoint != capability.config.admin_addr
            || lifecycle.resp_endpoint != capability.config.redis_addr
            || lifecycle.selected_endpoint != capability.selected_endpoint
            || lifecycle.endpoint_capability_digest != capability_sha256
            || lifecycle.server_binary_sha256 != capability.server_binary_sha256
            || lifecycle.loadgen_binary_sha256 != capability.loadgen_binary_sha256
            || build_binaries.get("hydracache-server") != Some(&lifecycle.server_binary_sha256)
            || build_binaries.get("hydracache-loadgen") != Some(&lifecycle.loadgen_binary_sha256)
            || build_binaries.len() != report.build.binary_sha256.len()
            || lifecycle.data_dir != capability.config.storage_dir
            || !lifecycle.direct_prebuilt_exec
            || !lifecycle.binaries_verified_after_measurement
            || !lifecycle.killed_and_waited
            || process_is_alive(lifecycle.pid)
            || !valid_file_sha(
                &lifecycle.server_binary_path,
                &lifecycle.server_binary_sha256,
            )
            || !valid_file_sha(
                &lifecycle.loadgen_binary_path,
                &lifecycle.loadgen_binary_sha256,
            )
            || !valid_file_receipt(
                &lifecycle.stdout_log.canonical_path,
                lifecycle.stdout_log.bytes,
                &lifecycle.stdout_log.sha256,
            )
            || !valid_file_receipt(
                &lifecycle.stderr_log.canonical_path,
                lifecycle.stderr_log.bytes,
                &lifecycle.stderr_log.sha256,
            )
        {
            return Err(MetricsHonestyError::Binding(
                "W3 suite/lifecycle receipt does not seal the exact endpoint process and artifact bytes"
                    .to_owned(),
            ));
        }
        let receipt = Self {
            schema_version: PERF_SCHEMA_VERSION,
            surface: DaemonMetricsSurface::W3NodeResp,
            report_sha256: sha256(open_loop_bytes),
            lifecycle_sha256: sha256(lifecycle_bytes),
            external_report_sha256: Some(sha256(external_bytes)),
            suite_receipt_sha256: Some(sha256(suite_receipt_bytes)),
            source_commit: report.source.git_commit.clone(),
            cargo_lock_sha256: Some(report.source.cargo_lock_sha256.clone()),
            runner_profile: report.runner_profile.clone(),
            runner_identity_sha256: sha256(&serde_json::to_vec(&report.observed_runner)?),
            prebuild_manifest_sha256: report.build.prebuild_manifest_sha256.clone(),
            prebuild_contract_sha256: report.build.prebuild_contract_digest.clone(),
            capability_receipt_sha256: capability_sha256,
            processes: vec![PredecessorProcessReceipt {
                node_id: "selected-node-local-resp-endpoint".to_owned(),
                pid: lifecycle.pid,
                admin_endpoint: lifecycle.admin_endpoint,
                server_binary_sha256: lifecycle.server_binary_sha256,
                loadgen_binary_sha256: Some(lifecycle.loadgen_binary_sha256),
                killed_and_waited: true,
            }],
            artifacts,
        };
        receipt.validate_shape()?;
        Ok(receipt)
    }

    /// Capture the W4A raw report only after W7 has moved it into its
    /// immutable recovery directory. Capturing `control-plane-N.json` before
    /// the W7 macro tail would leave this receipt pointing at the envelope
    /// that replaces the raw report.
    pub fn from_w4a_path(
        report_path: &Path,
        scenario: &ControlPlaneScenario,
    ) -> Result<Self, MetricsHonestyError> {
        let (artifact, report_bytes) =
            ArchivedArtifactReceipt::capture(W4A_REPORT_ARTIFACT, report_path)?;
        let receipt = Self::derive_w4a(&report_bytes, scenario, vec![artifact])?;
        receipt.validate_archived(Some(scenario))?;
        Ok(receipt)
    }

    fn derive_w4a(
        report_bytes: &[u8],
        scenario: &ControlPlaneScenario,
        artifacts: Vec<ArchivedArtifactReceipt>,
    ) -> Result<Self, MetricsHonestyError> {
        let report: ControlPlaneReport = serde_json::from_slice(report_bytes)?;
        let capability = report
            .validate_archived(scenario)
            .map_err(|error| MetricsHonestyError::Binding(error.to_string()))?;
        let lifecycle_bytes = serde_json::to_vec(&report.lifecycle)?;
        let mut processes = Vec::with_capacity(capability.nodes.len());
        for node in &capability.nodes {
            let lifecycle = report
                .lifecycle
                .payload
                .nodes
                .iter()
                .find(|candidate| candidate.node_id == node.node_id)
                .ok_or_else(|| {
                    MetricsHonestyError::Binding(format!(
                        "W4A lifecycle lost node {}",
                        node.node_id
                    ))
                })?;
            processes.push(PredecessorProcessReceipt {
                node_id: node.node_id.clone(),
                pid: node.pid,
                admin_endpoint: node.config.launch_config.admin_addr,
                server_binary_sha256: node.observed_executable_sha256.clone(),
                loadgen_binary_sha256: None,
                killed_and_waited: lifecycle.kill_requested
                    && lifecycle.wait_completed
                    && lifecycle.process_no_longer_running,
            });
        }
        processes.sort_by(|left, right| left.node_id.cmp(&right.node_id));
        let receipt = Self {
            schema_version: PERF_SCHEMA_VERSION,
            surface: DaemonMetricsSurface::W4aControlPlaneAdmin,
            report_sha256: sha256(report_bytes),
            lifecycle_sha256: sha256(&lifecycle_bytes),
            external_report_sha256: None,
            suite_receipt_sha256: None,
            source_commit: capability.source_commit,
            cargo_lock_sha256: None,
            runner_profile: capability.profile,
            runner_identity_sha256: capability.runner_fingerprint_sha256,
            prebuild_manifest_sha256: capability.prebuild_manifest_sha256,
            prebuild_contract_sha256: capability.prebuild_contract_sha256,
            capability_receipt_sha256: capability.receipt.receipt_sha256,
            processes,
            artifacts,
        };
        receipt.validate_shape()?;
        Ok(receipt)
    }

    fn validate_shape(&self) -> Result<(), MetricsHonestyError> {
        let mut nodes = BTreeSet::new();
        let artifact_names = self
            .artifacts
            .iter()
            .map(|artifact| artifact.logical_name.as_str())
            .collect::<Vec<_>>();
        let mut artifact_paths = BTreeSet::new();
        if self.schema_version != PERF_SCHEMA_VERSION
            || !is_sha256(&self.report_sha256)
            || !is_sha256(&self.lifecycle_sha256)
            || !is_git_commit(&self.source_commit)
            || self.runner_profile != "reference-v1"
            || !is_sha256(&self.runner_identity_sha256)
            || !is_sha256(&self.prebuild_manifest_sha256)
            || !is_sha256(&self.prebuild_contract_sha256)
            || !is_sha256(&self.capability_receipt_sha256)
            || self.processes.is_empty()
            || self.artifacts.is_empty()
            || self.artifacts.iter().any(|artifact| {
                artifact.logical_name.trim().is_empty()
                    || !artifact.canonical_path.is_absolute()
                    || artifact.bytes == 0
                    || artifact.bytes > MAX_PREDECESSOR_ARTIFACT_BYTES
                    || !is_sha256(&artifact.sha256)
                    || !artifact_paths.insert(artifact.canonical_path.clone())
            })
            || self.processes.iter().any(|process| {
                process.node_id.trim().is_empty()
                    || !nodes.insert(process.node_id.clone())
                    || process.pid == 0
                    || !process.admin_endpoint.ip().is_loopback()
                    || process.admin_endpoint.port() == 0
                    || !is_sha256(&process.server_binary_sha256)
                    || !process.killed_and_waited
            })
        {
            return Err(MetricsHonestyError::Binding(
                "predecessor receipt is incomplete, duplicated, or lacks kill/wait evidence"
                    .to_owned(),
            ));
        }
        match self.surface {
            DaemonMetricsSurface::W3NodeResp => {
                if self.processes.len() != 1
                    || artifact_names.as_slice()
                        != [
                            W3_OPEN_LOOP_ARTIFACT,
                            W3_EXTERNAL_ARTIFACT,
                            W3_LIFECYCLE_ARTIFACT,
                            W3_SUITE_RECEIPT_ARTIFACT,
                        ]
                    || self.artifacts[0]
                        .canonical_path
                        .file_name()
                        .and_then(|v| v.to_str())
                        != Some("node-resp-open-loop.json")
                    || self.artifacts[1]
                        .canonical_path
                        .file_name()
                        .and_then(|v| v.to_str())
                        != Some("node-resp-redis-benchmark.json")
                    || self.artifacts[2]
                        .canonical_path
                        .file_name()
                        .and_then(|v| v.to_str())
                        != Some("node-resp-daemon-lifecycle.json")
                    || self.artifacts[3]
                        .canonical_path
                        .file_name()
                        .and_then(|v| v.to_str())
                        != Some("node-resp-suite-receipt.json")
                    || !same_parent(&self.artifacts)
                    || self.report_sha256 != self.artifacts[0].sha256
                    || self.external_report_sha256.as_deref()
                        != Some(self.artifacts[1].sha256.as_str())
                    || self.lifecycle_sha256 != self.artifacts[2].sha256
                    || self.suite_receipt_sha256.as_deref()
                        != Some(self.artifacts[3].sha256.as_str())
                    || self
                        .suite_receipt_sha256
                        .as_deref()
                        .is_none_or(|value| !is_sha256(value))
                    || self
                        .cargo_lock_sha256
                        .as_deref()
                        .is_none_or(|value| !is_sha256(value))
                    || self.processes[0]
                        .loadgen_binary_sha256
                        .as_deref()
                        .is_none_or(|value| !is_sha256(value))
                {
                    return Err(MetricsHonestyError::Binding(
                        "W3 predecessor lacks suite/Cargo.lock/loadgen identity".to_owned(),
                    ));
                }
            }
            DaemonMetricsSurface::W4aControlPlaneAdmin => {
                let expected_report_name = format!(
                    "control-plane-{}-reference-v1.raw.json",
                    self.processes.len()
                );
                if self.suite_receipt_sha256.is_some()
                    || self.external_report_sha256.is_some()
                    || self.cargo_lock_sha256.is_some()
                    || artifact_names.as_slice() != [W4A_REPORT_ARTIFACT]
                    || self.report_sha256 != self.artifacts[0].sha256
                    || self.artifacts[0]
                        .canonical_path
                        .file_name()
                        .and_then(|value| value.to_str())
                        != Some(expected_report_name.as_str())
                    || self.artifacts[0]
                        .canonical_path
                        .parent()
                        .and_then(Path::file_name)
                        .and_then(|value| value.to_str())
                        != Some(W7_RAW_DIRECTORY)
                    || self
                        .processes
                        .iter()
                        .any(|process| process.loadgen_binary_sha256.is_some())
                {
                    return Err(MetricsHonestyError::Binding(
                        "W4A predecessor invented fields outside its capability".to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_archived(
        &self,
        control_plane_scenario: Option<&ControlPlaneScenario>,
    ) -> Result<(), MetricsHonestyError> {
        self.validate_shape()?;
        let mut bytes = BTreeMap::new();
        for artifact in &self.artifacts {
            let contents = artifact.validate_archived()?;
            if bytes
                .insert(artifact.logical_name.as_str(), contents)
                .is_some()
            {
                return Err(MetricsHonestyError::Binding(
                    "archived predecessor contains duplicate logical artifacts".to_owned(),
                ));
            }
        }
        let derived = match self.surface {
            DaemonMetricsSurface::W3NodeResp => Self::derive_w3(
                bytes[W3_OPEN_LOOP_ARTIFACT].as_slice(),
                bytes[W3_LIFECYCLE_ARTIFACT].as_slice(),
                bytes[W3_EXTERNAL_ARTIFACT].as_slice(),
                bytes[W3_SUITE_RECEIPT_ARTIFACT].as_slice(),
                self.artifacts.clone(),
            )?,
            DaemonMetricsSurface::W4aControlPlaneAdmin => {
                let scenario = control_plane_scenario.ok_or_else(|| {
                    MetricsHonestyError::Binding(
                        "archived W4A validation requires the committed control-plane scenario"
                            .to_owned(),
                    )
                })?;
                Self::derive_w4a(
                    bytes[W4A_REPORT_ARTIFACT].as_slice(),
                    scenario,
                    self.artifacts.clone(),
                )?
            }
        };
        if derived != *self {
            return Err(MetricsHonestyError::Binding(
                "persisted predecessor receipt no longer re-derives from its canonical artifacts"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    fn matches_window(&self, window: &MetricsWindowEvidence) -> bool {
        let binding = &window.binding;
        self.surface == binding.surface
            && self.source_commit == binding.source_commit
            && self.cargo_lock_sha256 == binding.cargo_lock_sha256
            && self.runner_profile == binding.runner_profile
            && self.runner_identity_sha256 == binding.runner_identity_sha256
            && self.prebuild_manifest_sha256 == binding.prebuild_manifest_sha256
            && self.prebuild_contract_sha256 == binding.prebuild_contract_sha256
            && self.capability_receipt_sha256 == binding.capability_receipt_sha256
            && self.processes.iter().any(|process| {
                process.node_id == binding.node_id
                    && process.pid == binding.pid
                    && process.admin_endpoint == binding.admin_endpoint
                    && process.server_binary_sha256 == binding.server_binary_sha256
                    && process.loadgen_binary_sha256 == binding.loadgen_binary_sha256
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsHonestyReport {
    pub schema_version: u32,
    pub release: String,
    pub scenario_id: String,
    pub scenario_sha256: String,
    pub evidence_class: String,
    pub surface: DaemonMetricsSurface,
    pub predecessor: MetricsPredecessorReceipt,
    pub windows: Vec<MetricsWindowEvidence>,
    pub no_new_product_metric: bool,
    pub no_in_process_daemon_claim: bool,
}

impl MetricsHonestyReport {
    pub fn new(
        scenario: &MetricsHonestyScenario,
        predecessor: MetricsPredecessorReceipt,
        windows: Vec<MetricsWindowEvidence>,
    ) -> Result<Self, MetricsHonestyError> {
        let surface = predecessor.surface;
        let report = Self {
            schema_version: PERF_SCHEMA_VERSION,
            release: PERF_RELEASE.to_owned(),
            scenario_id: scenario.scenario_id.clone(),
            scenario_sha256: scenario.digest()?,
            evidence_class: "real-daemon-exporter-cross-check".to_owned(),
            surface,
            predecessor,
            windows,
            no_new_product_metric: true,
            no_in_process_daemon_claim: true,
        };
        report.validate(scenario)?;
        Ok(report)
    }

    pub fn validate(&self, scenario: &MetricsHonestyScenario) -> Result<(), MetricsHonestyError> {
        scenario.validate()?;
        if self.schema_version != PERF_SCHEMA_VERSION
            || self.release != PERF_RELEASE
            || self.scenario_id != scenario.scenario_id
            || self.scenario_sha256 != scenario.digest()?
            || self.evidence_class != "real-daemon-exporter-cross-check"
            || !self.no_new_product_metric
            || !self.no_in_process_daemon_claim
            || self.surface != self.predecessor.surface
            || self.windows.len() != 1
        {
            return Err(MetricsHonestyError::Evidence(
                "W9 surface-specific artifact identity is incomplete or has a non-exact window set"
                    .to_owned(),
            ));
        }
        if self.windows[0].binding.surface != self.surface {
            return Err(MetricsHonestyError::Evidence(
                "W9 surface-specific artifact mixed RESP and control-plane evidence".to_owned(),
            ));
        }
        let mut window_ids = BTreeSet::new();
        self.predecessor.validate_shape()?;
        for window in &self.windows {
            window.validate(scenario)?;
            if !window_ids.insert(window.window_id.clone()) {
                return Err(MetricsHonestyError::Evidence(
                    "duplicate metrics window id".to_owned(),
                ));
            }
            if !self.predecessor.matches_window(window) {
                return Err(MetricsHonestyError::Evidence(
                    "metrics window does not bind the exact predecessor process/source/prebuild"
                        .to_owned(),
                ));
            }
        }
        Ok(())
    }

    /// Re-open and fully re-derive the persisted predecessor receipt. W3 is
    /// self-contained; W4A additionally needs its committed scenario because
    /// that scenario is the semantic validator for the W7-preserved raw W4
    /// report.
    pub fn validate_archived(
        &self,
        scenario: &MetricsHonestyScenario,
        control_plane_scenario: Option<&ControlPlaneScenario>,
    ) -> Result<(), MetricsHonestyError> {
        self.validate(scenario)?;
        self.predecessor.validate_archived(control_plane_scenario)
    }

    pub fn to_pretty_json(
        &self,
        scenario: &MetricsHonestyScenario,
    ) -> Result<Vec<u8>, MetricsHonestyError> {
        self.validate(scenario)?;
        Ok(serde_json::to_vec_pretty(self)?)
    }
}

/// Publish the independent W9 validator for the exact W3 suite bytes and the
/// metrics window captured before that same daemon was reaped.
pub fn publish_resp_metrics_report(
    scenario: &MetricsHonestyScenario,
    open_loop_path: &Path,
    lifecycle_path: &Path,
    external_path: &Path,
    suite_receipt_path: &Path,
    window: MetricsWindowEvidence,
    report_path: &Path,
) -> Result<MetricsHonestyReport, MetricsHonestyError> {
    if window.binding.surface != DaemonMetricsSurface::W3NodeResp {
        return Err(MetricsHonestyError::Binding(
            "RESP metrics publisher received a non-W3 window".to_owned(),
        ));
    }
    let predecessor = MetricsPredecessorReceipt::from_w3_paths(
        open_loop_path,
        lifecycle_path,
        external_path,
        suite_receipt_path,
    )?;
    let report = MetricsHonestyReport::new(scenario, predecessor, vec![window])?;
    report.validate_archived(scenario, None)?;
    write_new_report(report_path, scenario, None, &report)?;
    Ok(report)
}

/// Publish the independent W9 validator for the W7-preserved raw W4A report
/// and the metrics window captured from one of that report's exact live child
/// PIDs. This must run after the W7 macro tail has landed its raw sidecars.
pub fn publish_control_plane_metrics_report(
    scenario: &MetricsHonestyScenario,
    control_plane_scenario: &ControlPlaneScenario,
    control_plane_report_path: &Path,
    window: MetricsWindowEvidence,
    report_path: &Path,
) -> Result<MetricsHonestyReport, MetricsHonestyError> {
    if window.binding.surface != DaemonMetricsSurface::W4aControlPlaneAdmin {
        return Err(MetricsHonestyError::Binding(
            "control-plane metrics publisher received a non-W4A window".to_owned(),
        ));
    }
    let predecessor = MetricsPredecessorReceipt::from_w4a_path(
        control_plane_report_path,
        control_plane_scenario,
    )?;
    let report = MetricsHonestyReport::new(scenario, predecessor, vec![window])?;
    report.validate_archived(scenario, Some(control_plane_scenario))?;
    write_new_report(report_path, scenario, Some(control_plane_scenario), &report)?;
    Ok(report)
}

fn write_new_report(
    path: &Path,
    scenario: &MetricsHonestyScenario,
    control_plane_scenario: Option<&ControlPlaneScenario>,
    report: &MetricsHonestyReport,
) -> Result<(), MetricsHonestyError> {
    if !path.is_absolute() || path.file_name().is_none() || path.exists() {
        return Err(MetricsHonestyError::Evidence(
            "W9 report publication requires a new absolute output path".to_owned(),
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| MetricsHonestyError::Evidence("W9 report path has no parent".to_owned()))?;
    fs::create_dir_all(parent)?;
    let parent = fs::canonicalize(parent)?;
    let final_path = parent.join(path.file_name().expect("checked"));
    if final_path.exists() {
        return Err(MetricsHonestyError::Evidence(
            "W9 report output is not create-new".to_owned(),
        ));
    }
    report.validate_archived(scenario, control_plane_scenario)?;
    let bytes = report.to_pretty_json(scenario)?;
    let sequence = REPORT_SEQUENCE.fetch_add(1, Ordering::SeqCst);
    let temporary = parent.join(format!(
        ".{}.w9-tmp-{}-{sequence}",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("metrics.json"),
        std::process::id()
    ));
    let publication = (|| -> Result<(), MetricsHonestyError> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        report.validate_archived(scenario, control_plane_scenario)?;
        // A hard-link is the portable create-new atomic landing primitive here:
        // it fails if the final name appeared concurrently and never replaces it.
        fs::hard_link(&temporary, &final_path)?;
        fs::remove_file(&temporary)?;
        Ok(())
    })();
    if publication.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    publication?;
    report.validate_archived(scenario, control_plane_scenario)
}

/// Reject W2/W4B (or any future in-process/model surface) when a caller tries
/// to label it as daemon/exporter evidence.
pub fn validate_daemon_exporter_surface(
    surface: &SurfaceIdentity,
) -> Result<(), MetricsHonestyError> {
    let w3 = surface.surface_kind == "node-resp"
        && surface.execution_mode == "real-daemon-tcp-resp-open-loop"
        && surface.state_scope == "node-local"
        && surface.network_boundary == "loopback-tcp";
    let w4a = surface.surface_kind == "control-plane-admin-read-only"
        && surface.execution_mode == "real-daemon-admin-http-open-loop"
        && surface.state_scope == "consensus-metadata"
        && surface.network_boundary == "loopback-http";
    if !w3 && !w4a {
        return Err(MetricsHonestyError::Binding(
            "in-process/client-surface/grid-model evidence cannot be labeled daemon/exporter metrics evidence"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Falsifiability fixture: dropping every tenth exported increment must be
/// rejected by the same tolerance oracle used for comparable counters.
pub fn w9_counter_undercount_canary_red() -> Result<(), String> {
    let result = counter_agreement(
        MetricField::Operations,
        MetricSelector {
            name: "hydracache_cache_hits_total".to_owned(),
            labels: BTreeMap::from([("cache".to_owned(), "canary".to_owned())]),
        },
        1_000,
        1_090,
        100,
        1,
        10_000,
    )
    .map_err(|error| format!("{W9_CANARY_MARKER} {error}"))?;
    if result.agrees {
        Ok(())
    } else {
        Err(format!(
            "{W9_CANARY_MARKER} dropping every tenth counter increment was detected"
        ))
    }
}

pub fn verify_counter_agreement(
    field: MetricField,
    selector: MetricSelector,
    reported_before: u64,
    reported_after: u64,
    observer_delta: u64,
    absolute_tolerance: u64,
    relative_tolerance_millionths: u32,
) -> Result<CounterAgreement, MetricsHonestyError> {
    let agreement = counter_agreement(
        field,
        selector,
        reported_before,
        reported_after,
        observer_delta,
        absolute_tolerance,
        relative_tolerance_millionths,
    )?;
    if !agreement.agrees {
        return Err(MetricsHonestyError::Evidence(format!(
            "reported delta {} differs from independent delta {} by {}, tolerance {}",
            agreement.reported_delta,
            agreement.observer_delta,
            agreement.absolute_error,
            agreement.allowed_absolute_error
        )));
    }
    Ok(agreement)
}

fn exact_u64_metric(value: f64, name: &str) -> Result<u64, MetricsHonestyError> {
    if value < 0.0 || value.fract() != 0.0 || value > u64::MAX as f64 {
        return Err(MetricsHonestyError::Evidence(format!(
            "metric {name} is not an exact non-negative integer"
        )));
    }
    Ok(value as u64)
}

fn valid_file_receipt(path: &Path, bytes: u64, digest: &str) -> bool {
    if !path.is_absolute() || !is_sha256(digest) {
        return false;
    }
    let Ok(canonical) = fs::canonicalize(path) else {
        return false;
    };
    canonical == path
        && fs::metadata(&canonical)
            .is_ok_and(|metadata| metadata.is_file() && metadata.len() == bytes)
        && fs::read(&canonical).is_ok_and(|contents| sha256(&contents) == digest)
}

fn valid_file_sha(path: &Path, digest: &str) -> bool {
    if !path.is_absolute() || !is_sha256(digest) {
        return false;
    }
    let Ok(canonical) = fs::canonicalize(path) else {
        return false;
    };
    canonical == path
        && fs::metadata(&canonical).is_ok_and(|metadata| metadata.is_file())
        && fs::read(&canonical).is_ok_and(|contents| sha256(&contents) == digest)
}

fn same_parent(artifacts: &[ArchivedArtifactReceipt]) -> bool {
    let Some(parent) = artifacts
        .first()
        .and_then(|artifact| artifact.canonical_path.parent())
    else {
        return false;
    };
    artifacts
        .iter()
        .all(|artifact| artifact.canonical_path.parent() == Some(parent))
}

#[cfg(target_os = "linux")]
fn process_is_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(target_os = "windows")]
fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ exit 0 }} else {{ exit 1 }}"
            ),
        ])
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn process_is_alive(_pid: u32) -> bool {
    // Unknown process semantics cannot prove an archived PID is gone.
    true
}

fn valid_metric_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || matches!(first, b'_' | b':'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unix_nanos() -> Result<u64, MetricsHonestyError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            MetricsHonestyError::Evidence(format!("system clock before epoch: {error}"))
        })?
        .as_nanos();
    u64::try_from(nanos).map_err(|_| {
        MetricsHonestyError::Evidence("Unix timestamp no longer fits u64 nanoseconds".to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LatencySummary;

    const SCENARIO: &str =
        include_str!("../../../docs/testing/perf-scenarios/0.67/metrics-honesty-v1.toml");

    fn observation() -> OpenLoopObservation {
        OpenLoopObservation {
            offered: 64,
            started: 64,
            completed: 64,
            successes: 64,
            errors: 0,
            timeouts: 0,
            rejections: 0,
            backlog_high_water: 1,
            backlog_drained: true,
            drain_ms: 1,
            elapsed_ms: 64,
            offered_rate_per_second: 1_000.0,
            achieved_rate_per_second: 1_000.0,
            latency: LatencySummary {
                samples: 64,
                p50_us: Some(10),
                p90_us: Some(20),
                p99_us: Some(30),
                p999_us: Some(30),
                p999_min_samples: 1,
                p999_reportable: true,
                max_us: Some(30),
                overflow_count: 0,
            },
        }
    }

    fn w3_binding() -> LiveDaemonMetricsBinding {
        LiveDaemonMetricsBinding {
            schema_version: PERF_SCHEMA_VERSION,
            surface: DaemonMetricsSurface::W3NodeResp,
            scenario_id: METRICS_HONESTY_SCENARIO_ID.to_owned(),
            scenario_sha256: "a".repeat(64),
            node_id: "selected-node-local-resp-endpoint".to_owned(),
            pid: 1,
            admin_endpoint: "127.0.0.1:19001".parse().expect("socket"),
            source_commit: "a".repeat(40),
            cargo_lock_sha256: Some("b".repeat(64)),
            runner_profile: "reference-v1".to_owned(),
            runner_identity_sha256: "c".repeat(64),
            prebuild_manifest_sha256: "d".repeat(64),
            prebuild_contract_sha256: "e".repeat(64),
            server_binary_sha256: "f".repeat(64),
            loadgen_binary_sha256: Some("1".repeat(64)),
            capability_receipt_sha256: "2".repeat(64),
            direct_prebuilt_exec: true,
        }
    }

    #[test]
    fn raw_observer_accounting_tamper_is_rejected_after_rehash() {
        let scenario = MetricsHonestyScenario::parse_toml(SCENARIO).expect("scenario");
        let mut evidence = ObserverIntervalClock::start()
            .expect("clock")
            .finish(&scenario, &observation(), None, None)
            .expect("observer evidence");
        evidence
            .validate(&scenario, &w3_binding())
            .expect("baseline evidence");

        evidence.raw_open_loop_json = evidence
            .raw_open_loop_json
            .replace("\"successes\":64", "\"successes\":63");
        evidence.raw_open_loop_sha256 = sha256(evidence.raw_open_loop_json.as_bytes());
        assert!(evidence.validate(&scenario, &w3_binding()).is_err());
    }

    #[test]
    fn archived_log_receipt_rehash_rejects_same_length_mutation() {
        let root = std::env::temp_dir().join(format!(
            "hydracache-w9-log-receipt-{}-{}",
            std::process::id(),
            unix_nanos().expect("clock")
        ));
        fs::create_dir(&root).expect("temp root");
        let path = fs::canonicalize(&root)
            .expect("canonical root")
            .join("daemon.log");
        fs::write(&path, b"original").expect("write log");
        let digest = sha256(b"original");
        assert!(valid_file_receipt(&path, 8, &digest));
        fs::write(&path, b"tampered").expect("mutate log");
        assert!(!valid_file_receipt(&path, 8, &digest));
        fs::remove_dir_all(&root).expect("remove temp root");
    }

    #[test]
    fn persisted_predecessor_path_rejects_late_artifact_replacement() {
        let root = std::env::temp_dir().join(format!(
            "hydracache-w9-artifact-replacement-{}-{}",
            std::process::id(),
            unix_nanos().expect("clock")
        ));
        fs::create_dir(&root).expect("temp root");
        let path = fs::canonicalize(&root)
            .expect("canonical root")
            .join("node-resp-open-loop.json");
        fs::write(&path, b"original-artifact").expect("write artifact");
        let (receipt, _) = ArchivedArtifactReceipt::capture(W3_OPEN_LOOP_ARTIFACT, &path)
            .expect("capture artifact");
        receipt.validate_archived().expect("baseline archive");
        fs::write(&path, b"tampered-artifact").expect("replace artifact");
        assert!(receipt.validate_archived().is_err());
        fs::remove_dir_all(&root).expect("remove temp root");
    }

    #[test]
    fn persisted_predecessor_path_rejects_missing_artifact() {
        let root = std::env::temp_dir().join(format!(
            "hydracache-w9-artifact-missing-{}-{}",
            std::process::id(),
            unix_nanos().expect("clock")
        ));
        fs::create_dir(&root).expect("temp root");
        let raw_root = root.join(W7_RAW_DIRECTORY);
        fs::create_dir(&raw_root).expect("raw root");
        let path = fs::canonicalize(&raw_root)
            .expect("canonical raw root")
            .join("control-plane-3-reference-v1.raw.json");
        fs::write(&path, b"control-plane-artifact").expect("write artifact");
        let (receipt, _) =
            ArchivedArtifactReceipt::capture(W4A_REPORT_ARTIFACT, &path).expect("capture artifact");
        fs::remove_file(&path).expect("remove artifact");
        assert!(receipt.validate_archived().is_err());
        fs::remove_dir_all(&root).expect("remove temp root");
    }

    #[test]
    fn predecessor_capture_accepts_an_ordinary_absolute_path_and_persists_canonical_form() {
        let root = std::env::temp_dir().join(format!(
            "hydracache-w9-ordinary-path-{}-{}",
            std::process::id(),
            unix_nanos().expect("clock")
        ));
        fs::create_dir(&root).expect("temp root");
        let ordinary_path = root.join("node-resp-open-loop.json");
        assert!(ordinary_path.is_absolute());
        fs::write(&ordinary_path, b"ordinary-absolute-path").expect("write artifact");
        let expected = fs::canonicalize(&ordinary_path).expect("canonical artifact");
        let (receipt, _) = ArchivedArtifactReceipt::capture(W3_OPEN_LOOP_ARTIFACT, &ordinary_path)
            .expect("capture ordinary path");
        assert_eq!(receipt.canonical_path, expected);
        receipt.validate_archived().expect("archived receipt");
        fs::remove_dir_all(&root).expect("remove temp root");
    }
}
