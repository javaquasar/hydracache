//! Byte-only P0 reconciliation. No producer, process, filesystem or live route.
use crate::diagnostic_artifacts::{ArtifactDigest, ArtifactError, VerifiedBuild};
use crate::diagnostic_lease::{CellIntent, MAX_RECEIPT_BYTES, SOURCE_COMMIT};
use crate::{canonical_json, sha256_hex};
use serde::{de, Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use thiserror::Error;

pub const STREAM_BYTES: usize = MAX_RECEIPT_BYTES as usize / 2;
const PROFILE: &str = "unprofiled-timing-controls-074-v1";
const CPU_SCOPE: &str = "whole-process scheduled-driver+byte-oracles+task/wire-drain+observation-projection; not server-only";
const CONFIGS: [&[u8]; 4] = [
    include_bytes!("../../../docs/testing/performance/0.74/rental-pilot-draft/embedded.json"),
    include_bytes!("../../../docs/testing/performance/0.74/rental-pilot-draft/direct.json"),
    include_bytes!("../../../docs/testing/performance/0.74/rental-pilot-draft/resp2.json"),
    include_bytes!("../../../docs/testing/performance/0.74/rental-pilot-draft/resp3.json"),
];

/// A caller assertion, NOT authenticated systemd/recursive-cgroup evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSummary {
    pub unit_name: String,
    pub boot_id: String,
    pub cgroup_path: String,
    pub cgroup_inode: u64,
    pub populated: bool,
    pub exit_code: Option<i32>,
    pub term_signal: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    ValidCpuUsable,
    ValidCpuUnusable,
    FailedProcess,
    FailedReport,
    InvalidContent,
    TerminalUnproven,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Packet {
    pub schema_version: u32,
    pub contract_id: String,
    pub intent: CellIntent,
    pub build_receipt_sha256: String,
    pub terminal: TerminalSummary,
    pub stdout: ArtifactDigest,
    pub stderr: ArtifactDigest,
    pub decision: Decision,
    pub promotable: bool,
    pub admission_allowed: bool,
    pub product_numeric_claims_allowed: bool,
    pub durable_spool_proven: bool,
    pub live_cgroup_proven: bool,
}

#[derive(Debug, Error)]
pub enum ReceiptError {
    #[error("raw stream exceeds complete-packet budget; preserve bounded prefix separately")]
    Overflow,
    #[error("diagnostic receipt identity refused: {0}")]
    Identity(#[from] ArtifactError),
    #[error("packet does not match recomputed canonical receipt envelope")]
    Packet,
    #[error("packet serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

/// Bounded failures are retained by digest just like successes. The caller owns
/// the original bytes and must actually persist them before releasing any fence.
pub fn packet(
    build: &VerifiedBuild,
    intent: &CellIntent,
    terminal: &TerminalSummary,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<Packet, ReceiptError> {
    build.bind_intent(intent)?;
    if stdout.len() > STREAM_BYTES || stderr.len() > STREAM_BYTES {
        return Err(ReceiptError::Overflow);
    }
    if terminal.unit_name.len() > 256
        || terminal.boot_id.len() > 36
        || terminal.cgroup_path.len() > 256
    {
        return Err(ReceiptError::Packet);
    }
    let terminal_matches = terminal.unit_name == intent.unit_name
        && terminal.boot_id == intent.boot_id
        && terminal.cgroup_path == intent.cgroup_path
        && terminal.cgroup_inode != 0
        && !terminal.populated
        && matches!(
            (terminal.exit_code, terminal.term_signal),
            (Some(0..=255), None) | (None, Some(1..=64))
        );
    let decision = if !terminal_matches {
        Decision::TerminalUnproven
    } else {
        match reconcile(build, intent, stdout) {
            Err(()) => Decision::InvalidContent,
            Ok(None) => Decision::FailedReport,
            Ok(Some(_)) if terminal.exit_code != Some(0) => Decision::FailedProcess,
            Ok(Some(true)) => Decision::ValidCpuUsable,
            Ok(Some(false)) => Decision::ValidCpuUnusable,
        }
    };
    // A config/argument/startup failure has stderr only and no report envelope.
    let decision = if terminal_matches && stdout.is_empty() && terminal.exit_code != Some(0) {
        Decision::FailedProcess
    } else {
        decision
    };
    Ok(Packet {
        schema_version: 1,
        contract_id: "diagnostic-receipts-local-074-v1".into(),
        intent: intent.clone(),
        build_receipt_sha256: build.identity().build_provenance_sha256.clone(),
        terminal: terminal.clone(),
        stdout: ArtifactDigest::of(stdout),
        stderr: ArtifactDigest::of(stderr),
        decision,
        promotable: false,
        admission_allowed: false,
        product_numeric_claims_allowed: false,
        durable_spool_proven: false,
        live_cgroup_proven: false,
    })
}

pub fn packet_bytes(packet: &Packet) -> Result<Vec<u8>, ReceiptError> {
    let mut bytes = canonical_json(packet)?;
    bytes.push(b'\n');
    if bytes.len() > 65_536 {
        return Err(ReceiptError::Packet);
    }
    Ok(bytes)
}

/// Offline byte verification with independently supplied intent/terminal/build.
/// Does not open files, trust embedded decisions or authenticate the terminal.
pub fn verify_packet(
    bytes: &[u8],
    build: &VerifiedBuild,
    intent: &CellIntent,
    terminal: &TerminalSummary,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<Packet, ReceiptError> {
    if bytes.len() > 65_536 {
        return Err(ReceiptError::Packet);
    }
    let expected = packet(build, intent, terminal, stdout, stderr)?;
    if packet_bytes(&expected)? != bytes {
        return Err(ReceiptError::Packet);
    }
    Ok(expected)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    source_commit_from_coordinator: String,
    source_git_identity_verified_by_binary: bool,
    binary_sha256: String,
    compiled_root_lock_sha256: String,
    compiled_observer_lock_sha256: String,
    report: Option<Report>,
    error: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u32,
    profile_id: String,
    surface: String,
    operation: String,
    seed: u64,
    keyspace: usize,
    payload_bytes: usize,
    dataset_sha256: String,
    slots: usize,
    pipeline_depth: usize,
    warmup_calls: u64,
    minimum_usable_cpu_ns: u64,
    minimum_usable_measurement_wall_ns: u64,
    schedule: Schedule,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Schedule {
    operations: u64,
    offered_rate_per_second: u64,
    concurrency: usize,
    maximum_queued: usize,
    operation_timeout_ns: u64,
    drain_timeout_ns: u64,
    slo_ns: u64,
    highest_trackable_ns: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    schema_version: u32,
    profile_id: String,
    input: Input,
    workload_sha256: String,
    get_owner_feature: bool,
    runtime: String,
    allocator: String,
    admission_allowed: bool,
    product_numeric_claims_allowed: bool,
    cross_surface_numeric_comparison_allowed: bool,
    secure_fresh_process_material_parity_proven: bool,
    warmup_completed: u64,
    transport_security: Option<Value>,
    cpu: Option<Cpu>,
    observation: Option<Boundary>,
    final_dataset_verified: bool,
    shutdown_verified: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    tag = "boundary",
    content = "observed",
    rename_all = "snake_case"
)]
enum Boundary {
    Native(Box<Observation>),
    Resp(Box<WireObservation>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cpu {
    clock: Clock,
    process_cpu_ns: u64,
    wall_elapsed_ns: u64,
    cpu_ns_per_offer: f64,
    cpu_ns_per_success: Option<f64>,
    usable_for_ratio: bool,
    unusable_reasons: Vec<String>,
    scope: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Clock {
    provider: String,
    scope: String,
    unit_resolution_ns: u64,
    resolution_is_accuracy_claim: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Success,
    Error,
    Timeout,
    QueueTimeout,
    TargetRejected,
    AdmissionRejected,
    Incomplete,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    sequence: u64,
    scheduled_ns: u64,
    started_ns: Option<u64>,
    terminal_ns: u64,
    outcome: Outcome,
    scheduled_latency_ns: Option<u64>,
    service_latency_ns: Option<u64>,
    incomplete_lower_bound_ns: Option<u64>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Latency {
    unit: String,
    samples: u64,
    p50_ns: Option<u64>,
    p95_ns: Option<u64>,
    p99_ns: Option<u64>,
    overflow_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    profile_id: String,
    promotable: bool,
    product_performance_claim: bool,
    config: Schedule,
    offered: u64,
    target_started: u64,
    target_completed: u64,
    successes: u64,
    errors: u64,
    timeouts: u64,
    queue_timeouts: u64,
    target_rejections: u64,
    admission_rejections: u64,
    incomplete: u64,
    late_successes: u64,
    good_successes: u64,
    elapsed_ns: u64,
    goodput_operations_per_second: f64,
    good_fraction_of_all_offers: f64,
    pending_high_water: usize,
    execution_slots: usize,
    owned_tasks_drained: bool,
    scheduled_response_latency: Latency,
    service_response_latency: Latency,
    samples: Vec<Sample>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObservation {
    dialect: String,
    hello_connections: usize,
    hello_server_version: Option<String>,
    operations: Observation,
    wire_samples: Vec<WireSample>,
    pipeline_limit: usize,
    physical_connections: usize,
    operation: String,
    batch_size: usize,
    product_performance_claim: bool,
    authenticated_connections: usize,
    transport_security: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSample {
    dialect: String,
    sequence: u64,
    connection_id: usize,
    wire_ordinal: u64,
    scheduled_ns: u64,
    accepted_ns: u64,
    write_completed_ns: Option<u64>,
    response_complete_ns: Option<u64>,
    scheduled_frame_latency_ns: Option<u64>,
    frame_kind: Option<String>,
    response_items: Option<usize>,
    protocol_error_bytes: Option<Vec<u8>>,
    byte_oracle_verified: bool,
    waiting_caller_cancelled: bool,
    transport_failure: Option<String>,
    operation_outcome: Outcome,
}

fn require(condition: bool) -> Result<(), ()> {
    if condition {
        Ok(())
    } else {
        Err(())
    }
}

fn reconcile(build: &VerifiedBuild, intent: &CellIntent, bytes: &[u8]) -> Result<Option<bool>, ()> {
    require(bytes.last() == Some(&b'\n'))?;
    let value = strict_value(bytes)?;
    require(value.is_object())?;
    let e: Envelope = serde_json::from_value(value.clone()).map_err(|_| ())?;
    // Includes null-valued fields: omission cannot silently default an Option.
    require(serde_json::to_value(&e).map_err(|_| ())? == value)?;
    let s = build.statement();
    require(
        e.source_commit_from_coordinator == SOURCE_COMMIT
            && !e.source_git_identity_verified_by_binary
            && e.binary_sha256 == s.binary.sha256
            && e.compiled_root_lock_sha256 == s.root_lock.sha256
            && e.compiled_observer_lock_sha256 == s.observer_lock.sha256,
    )?;
    let Some(r) = e.report else {
        require(e.error.as_ref().is_some_and(|s| !s.is_empty()))?;
        return Ok(None);
    };
    let index = crate::diagnostic_lease::SURFACES
        .iter()
        .position(|s| *s == intent.surface)
        .ok_or(())?;
    let input: Input = serde_json::from_slice(CONFIGS[index]).map_err(|_| ())?;
    require(
        r.schema_version == 1
            && r.profile_id == PROFILE
            && r.input == input
            && r.workload_sha256 == sha256_hex(&serde_json::to_vec(&input).map_err(|_| ())?)
            && !r.get_owner_feature
            && r.runtime == "current-thread-required-by-executable"
            && r.allocator == "System-without-counting-wrapper-required-by-executable"
            && !r.admission_allowed
            && !r.product_numeric_claims_allowed
            && !r.cross_surface_numeric_comparison_allowed
            && !r.secure_fresh_process_material_parity_proven
            && r.transport_security.is_none()
            && r.warmup_completed <= input.warmup_calls,
    )?;
    if e.error.is_some() || r.error.is_some() {
        require(
            e.error.as_ref().is_none_or(|s| !s.is_empty())
                && r.error.as_ref().is_none_or(|s| !s.is_empty()),
        )?;
        return Ok(None);
    }
    require(
        r.warmup_completed == input.warmup_calls && r.final_dataset_verified && r.shutdown_verified,
    )?;
    let o = match r.observation.as_ref().ok_or(())? {
        Boundary::Native(o) => {
            require(index < 2)?;
            o.as_ref()
        }
        Boundary::Resp(w) => {
            require(index >= 2)?;
            validate_wire(w, &input)?;
            &w.operations
        }
    };
    validate_observation(o, &input.schedule, intent.maximum_runtime_seconds)?;
    // The frozen binary emits a report error if any offer fails to succeed.
    require(o.successes == input.schedule.operations && o.owned_tasks_drained)?;
    let c = r.cpu.as_ref().ok_or(())?;
    require(
        c.clock.provider == "CLOCK_PROCESS_CPUTIME_ID"
            && c.clock.scope == "whole-process-all-threads"
            && c.clock.unit_resolution_ns > 0
            && !c.clock.resolution_is_accuracy_claim
            && c.scope == CPU_SCOPE
            && c.wall_elapsed_ns >= o.elapsed_ns
            && c.wall_elapsed_ns <= intent.maximum_runtime_seconds * 1_000_000_000
            && c.process_cpu_ns <= c.wall_elapsed_ns.saturating_mul(256)
            && same_float(
                c.cpu_ns_per_offer,
                c.process_cpu_ns as f64 / o.offered as f64,
            )
            && c.cpu_ns_per_success
                .is_some_and(|v| same_float(v, c.process_cpu_ns as f64 / o.successes as f64)),
    )?;
    let mut reasons = Vec::new();
    if c.process_cpu_ns < input.minimum_usable_cpu_ns {
        reasons.push("CPU-below-predeclared-minimum");
    }
    if c.wall_elapsed_ns < input.minimum_usable_measurement_wall_ns {
        reasons.push("wall-below-predeclared-minimum");
    }
    if o.successes != o.offered
        || !o.owned_tasks_drained
        || o.scheduled_response_latency.overflow_count != 0
        || o.service_response_latency.overflow_count != 0
    {
        reasons.push("incomplete-error-or-overflow-observation");
    }
    require(c.unusable_reasons == reasons && c.usable_for_ratio == reasons.is_empty())?;
    Ok(Some(c.usable_for_ratio))
}

fn same_float(actual: f64, expected: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && (actual - expected).abs() <= expected.abs().max(1.0) * 4.0 * f64::EPSILON
}

fn validate_observation(o: &Observation, config: &Schedule, runtime: u64) -> Result<(), ()> {
    require(
        o.profile_id == "get-owner-scheduled-controls-074-v1"
            && !o.promotable
            && !o.product_performance_claim
            && o.config == *config
            && o.offered == config.operations
            && o.samples.len() == config.operations as usize
            && o.elapsed_ns > 0
            && o.elapsed_ns <= runtime * 1_000_000_000
            && o.pending_high_water <= config.concurrency + config.maximum_queued
            && o.execution_slots == config.concurrency
            && o.owned_tasks_drained,
    )?;
    for (i, s) in o.samples.iter().enumerate() {
        require(
            s.sequence == i as u64
                && s.scheduled_ns == i as u64 * 200_000
                && s.terminal_ns >= s.scheduled_ns
                && s.terminal_ns <= o.elapsed_ns,
        )?;
        match s.outcome {
            Outcome::AdmissionRejected | Outcome::QueueTimeout => require(
                s.started_ns.is_none()
                    && s.scheduled_latency_ns.is_none()
                    && s.service_latency_ns.is_none()
                    && s.incomplete_lower_bound_ns.is_none()
                    && (s.outcome != Outcome::QueueTimeout
                        || s.terminal_ns - s.scheduled_ns >= config.operation_timeout_ns),
            )?,
            Outcome::Incomplete => require(
                s.started_ns
                    .is_none_or(|t| t >= s.scheduled_ns && t <= s.terminal_ns)
                    && s.scheduled_latency_ns.is_none()
                    && s.service_latency_ns.is_none()
                    && s.incomplete_lower_bound_ns == Some(s.terminal_ns - s.scheduled_ns),
            )?,
            _ => {
                let start = s.started_ns.ok_or(())?;
                require(
                    start >= s.scheduled_ns
                        && start <= s.terminal_ns
                        && s.scheduled_latency_ns == Some(s.terminal_ns - s.scheduled_ns)
                        && s.service_latency_ns == Some(s.terminal_ns - start)
                        && s.incomplete_lower_bound_ns.is_none(),
                )?;
            }
        }
    }
    let count = |v| o.samples.iter().filter(|s| s.outcome == v).count() as u64;
    let successes = count(Outcome::Success);
    let late = o
        .samples
        .iter()
        .filter(|s| {
            s.outcome == Outcome::Success
                && s.scheduled_latency_ns.is_some_and(|v| v > config.slo_ns)
        })
        .count() as u64;
    let good = successes - late;
    require(
        o.successes == successes
            && o.errors == count(Outcome::Error)
            && o.timeouts == count(Outcome::Timeout)
            && o.queue_timeouts == count(Outcome::QueueTimeout)
            && o.target_rejections == count(Outcome::TargetRejected)
            && o.admission_rejections == count(Outcome::AdmissionRejected)
            && o.incomplete == count(Outcome::Incomplete)
            && o.late_successes == late
            && o.good_successes == good
            && o.target_started
                == o.samples.iter().filter(|s| s.started_ns.is_some()).count() as u64
            && o.target_completed == successes + o.errors + o.timeouts + o.target_rejections
            && same_float(
                o.goodput_operations_per_second,
                good as f64 * 1e9 / o.elapsed_ns as f64,
            )
            && same_float(
                o.good_fraction_of_all_offers,
                good as f64 / config.operations as f64,
            ),
    )?;
    require(
        o.scheduled_response_latency
            == latency(
                o.samples.iter().filter_map(|s| s.scheduled_latency_ns),
                config.highest_trackable_ns,
            )
            && o.service_response_latency
                == latency(
                    o.samples.iter().filter_map(|s| s.service_latency_ns),
                    config.highest_trackable_ns,
                ),
    )
}

/// Specialized pinned HDR 7.6.0 bounds(1, highest, 3), not a general histogram.
fn latency(values: impl Iterator<Item = u64>, highest: u64) -> Latency {
    let mut overflow = 0;
    let mut values: Vec<_> = values
        .map(|v| {
            overflow += u64::from(v > highest);
            let v = v.clamp(1, highest);
            let shift = (63 - v.leading_zeros()).saturating_sub(10);
            v | ((1u64 << shift) - 1)
        })
        .collect();
    values.sort_unstable();
    let q = |q: f64| {
        (!values.is_empty()).then(|| values[(q * values.len() as f64).ceil() as usize - 1])
    };
    Latency {
        unit: "nanosecond-histogram-not-clock-resolution".into(),
        samples: values.len() as u64,
        p50_ns: q(0.5),
        p95_ns: q(0.95),
        p99_ns: q(0.99),
        overflow_count: overflow,
    }
}

fn validate_wire(w: &WireObservation, input: &Input) -> Result<(), ()> {
    require(
        w.dialect == input.surface
            && w.pipeline_limit == input.pipeline_depth
            && w.physical_connections == input.slots
            && w.operation == "get"
            && w.batch_size == 1
            && !w.product_performance_claim
            && w.authenticated_connections == 0
            && w.transport_security.is_none()
            && w.wire_samples.len() <= input.schedule.operations as usize,
    )?;
    match w.dialect.as_str() {
        "resp2" => require(w.hello_connections == 0 && w.hello_server_version.is_none())?,
        "resp3" => require(
            w.hello_connections == input.slots
                && w.hello_server_version.as_ref().is_some_and(|v| {
                    !v.is_empty()
                        && v.len() <= 64
                        && v.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
                }),
        )?,
        _ => return Err(()),
    }
    let mut seen = BTreeSet::new();
    let mut ordinals = vec![None; input.slots];
    let mut responses = vec![None; input.slots];
    for s in &w.wire_samples {
        require(
            s.connection_id < input.slots
                && s.dialect == w.dialect
                && s.sequence % input.slots as u64 == s.connection_id as u64
                && seen.insert(s.sequence),
        )?;
        let operation = w.operations.samples.get(s.sequence as usize).ok_or(())?;
        require(
            s.scheduled_ns == operation.scheduled_ns
                && s.operation_outcome == operation.outcome
                && s.accepted_ns >= s.scheduled_ns
                && s.accepted_ns <= w.operations.elapsed_ns
                && operation.started_ns.is_some_and(|t| s.accepted_ns >= t)
                && ordinals[s.connection_id].is_none_or(|v| s.wire_ordinal > v),
        )?;
        ordinals[s.connection_id] = Some(s.wire_ordinal);
        require(
            s.write_completed_ns
                .is_none_or(|t| t >= s.accepted_ns && t <= w.operations.elapsed_ns),
        )?;
        match s.response_complete_ns {
            Some(t) => {
                require(
                    s.write_completed_ns.is_some_and(|v| t >= v)
                        && t <= w.operations.elapsed_ns
                        && responses[s.connection_id].is_none_or(|v| t >= v)
                        && s.scheduled_frame_latency_ns == Some(t - s.scheduled_ns)
                        && s.response_items == Some(1)
                        && s.transport_failure.is_none()
                        && matches!(
                            s.frame_kind.as_deref(),
                            Some("bulk" | "null" | "simple" | "integer" | "error")
                        )
                        && (s.frame_kind.as_deref() == Some("error"))
                            == s.protocol_error_bytes.is_some()
                        && s.protocol_error_bytes
                            .as_ref()
                            .is_none_or(|v| v.len() <= 128),
                )?;
                responses[s.connection_id] = Some(t);
            }
            None => require(
                !s.byte_oracle_verified
                    && s.frame_kind.is_none()
                    && s.response_items.is_none()
                    && s.protocol_error_bytes.is_none()
                    && s.scheduled_frame_latency_ns.is_none()
                    && s.transport_failure.as_ref().is_some_and(|v| !v.is_empty()),
            )?,
        }
        if operation.outcome == Outcome::Success {
            require(
                s.byte_oracle_verified
                    && !s.waiting_caller_cancelled
                    && s.frame_kind.as_deref() == Some("bulk")
                    && s.response_complete_ns
                        .is_some_and(|v| v <= operation.terminal_ns),
            )?;
        }
    }
    require(
        w.operations
            .samples
            .iter()
            .all(|s| s.outcome != Outcome::Success || seen.contains(&s.sequence)),
    )
}

// serde_json::Value normally overwrites duplicate object keys. Do not use it
// before this recursive visitor, including inside nullable opaque fields.
struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|v| StrictValue(v.into()))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(StrictValue(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut v = Vec::new();
                while let Some(StrictValue(item)) = a.next_element()? {
                    v.push(item);
                }
                Ok(StrictValue(v.into()))
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut v = serde_json::Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    if v.contains_key(&key) {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                    let StrictValue(item) = a.next_value()?;
                    v.insert(key, item);
                }
                Ok(StrictValue(v.into()))
            }
        }
        d.deserialize_any(Visitor)
    }
}
fn strict_value(bytes: &[u8]) -> Result<Value, ()> {
    serde_json::from_slice::<StrictValue>(bytes)
        .map(|v| v.0)
        .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_hdr_projection_checks_bucket_edges_and_ceil_rank() {
        for (v, expected) in [
            (1, 1),
            (2047, 2047),
            (2048, 2049),
            (8180, 8183),
            (8193, 8199),
            (9995, 9999),
            (10008, 10015),
            (10_000_000_000, 10_007_609_343),
        ] {
            assert_eq!(
                latency([v].into_iter(), 10_000_000_000).p99_ns,
                Some(expected)
            );
        }
        assert_eq!(latency([1, 2, 3].into_iter(), 10).p50_ns, Some(2));
        assert_eq!(latency([0, 11].into_iter(), 10).overflow_count, 1);
        assert_eq!(latency([].into_iter(), 10).p99_ns, None);
    }

    #[test]
    fn seeded_hdr_projection_preserves_rank_bounds_and_overflow_accounting() {
        let mut seed = 740074u64;
        for length in [1, 2, 3, 8, 31, 100, 1000] {
            let mut raw = Vec::new();
            for _ in 0..length {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                raw.push(seed % 12_000_000_000);
            }
            let projected = latency(raw.iter().copied(), 10_000_000_000);
            assert_eq!(projected.samples, length as u64);
            assert_eq!(
                projected.overflow_count,
                raw.iter().filter(|v| **v > 10_000_000_000).count() as u64
            );
            raw.sort_unstable();
            for (q, projected) in [
                (0.5, projected.p50_ns),
                (0.95, projected.p95_ns),
                (0.99, projected.p99_ns),
            ] {
                let rank = raw[(q * length as f64).ceil() as usize - 1].clamp(1, 10_000_000_000);
                let upper = projected.unwrap();
                assert!(upper >= rank);
                assert!(upper - rank <= rank / 1024);
                let width = 1u64 << (63 - rank.leading_zeros()).saturating_sub(10);
                assert_eq!(upper % width, width - 1);
            }
        }
    }

    #[test]
    fn strict_json_rejects_nested_duplicates_and_depth_without_tail_repair() {
        assert!(strict_value(br#"{"report":{"error":null,"error":"override"}}"#).is_err());
        assert!(strict_value(br#"{"samples":[{"sequence":0,"sequence":1}]}"#).is_err());
        let nested = format!("{}null{}", "[".repeat(129), "]".repeat(129));
        assert!(strict_value(nested.as_bytes()).is_err());
        for tail in [b"x".as_slice(), b"{}", b"\0"] {
            let mut bytes = b"{}".to_vec();
            bytes.extend(tail);
            assert!(strict_value(&bytes).is_err());
        }
    }
}
