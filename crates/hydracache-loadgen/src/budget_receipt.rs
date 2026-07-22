//! Shared W7 macro-report envelope and canonical receipt contract.
//!
//! Operational and model tiers own their typed raw reports. This module adds
//! only the candidate/build/runner identity required by the release budget
//! checker; it does not reinterpret or bless producer measurements.

use std::collections::{BTreeMap, BTreeSet};
#[cfg(unix)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::overload::{EligibleOverloadSurface, OverloadReport, OverloadRunMode, OverloadScenario};
use crate::profile::{PerformanceProfile, RunnerFingerprint};
use crate::rate::OpenLoopObservation;
use crate::report::EvidenceRunMode;
use crate::targets::brownout::{
    BrownoutRunMode, ControlPlaneBrownoutReport, ControlPlaneBrownoutScenario,
    GridModelBrownoutReport, GridModelBrownoutScenario, RespBrownoutReport, RespBrownoutScenario,
};
use crate::targets::control_plane::{ControlPlaneReport, ControlPlaneScenario};
use crate::targets::grid_model::{GridModelReport, GridModelRunMode, GridModelScenario};
use crate::tiers::resp_reference::{
    ValidatedRespReferenceContext, LOADGEN_BINARY_ID, REFERENCE_PROFILE, SERVER_BINARY_ID,
};
use crate::{PERF_RELEASE, PERF_SCHEMA_VERSION};

/// Dedicated recovery/provenance directory for the atomic W7 macro tail.
/// Keeping it below the evidence root avoids colliding with the top-level
/// missing/extra report guard.
pub const MACRO_RAW_DIR_RELATIVE: &str = "target/test-evidence/0.67/w7-raw";
pub const MACRO_PUBLICATION_RECEIPT_RELATIVE: &str =
    "target/test-evidence/0.67/w7-raw/macro-publication-receipt.json";

/// Exact W4-W6 raw reports that must be published as one fail-closed batch.
pub const MACRO_REPORT_PATHS: [(&str, &str); 10] = [
    (
        "control-plane-3-reference-v1",
        "target/test-evidence/0.67/control-plane-3.json",
    ),
    (
        "control-plane-5-reference-v1",
        "target/test-evidence/0.67/control-plane-5.json",
    ),
    (
        "control-plane-7-reference-v1",
        "target/test-evidence/0.67/control-plane-7.json",
    ),
    (
        "grid-model-reference-v1",
        "target/test-evidence/0.67/grid-model.json",
    ),
    (
        "brownout-control-plane-reference-v1",
        "target/test-evidence/0.67/brownout-control-plane.json",
    ),
    (
        "brownout-resp-endpoint-reference-v1",
        "target/test-evidence/0.67/brownout-resp-endpoint.json",
    ),
    (
        "brownout-grid-model-reference-v1",
        "target/test-evidence/0.67/brownout-grid-model.json",
    ),
    (
        "overload-local-v1",
        "target/test-evidence/0.67/overload-local.json",
    ),
    (
        "overload-client-surface-v1",
        "target/test-evidence/0.67/overload-client-surface.json",
    ),
    (
        "overload-node-resp-v1",
        "target/test-evidence/0.67/overload-node-resp.json",
    ),
];

/// One exact prebuilt executable in a macro-report receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryDigest {
    pub id: String,
    pub sha256: String,
}

/// One producer-owned metric copied into the sealed W7 receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportMetric {
    pub id: String,
    pub value: f64,
    pub unit: String,
}

/// Canonical identity receipt for W4-W6 operational/model reports.
///
/// W7 independently derives `metrics`, `maximum_spread_ratio`, and `stable`
/// from the typed `report`. The copies here are deliberately sealed so a
/// producer cannot leave unbound metadata beside an otherwise-bound report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroReportReceipt {
    pub schema_version: u32,
    pub release: String,
    pub report_id: String,
    pub source_report_sha256: String,
    pub claim_scope: String,
    pub run_mode: EvidenceRunMode,
    pub runner_profile: String,
    pub runner_contract: PerformanceProfile,
    pub runner_contract_digest: String,
    pub observed_runner: RunnerFingerprint,
    pub runner_fingerprint: String,
    pub source_commit: String,
    pub cargo_lock_sha256: String,
    pub toolchain_identity: String,
    pub prebuild_contract_digest: String,
    pub prebuild_manifest_sha256: String,
    pub binary_sha256: Vec<BinaryDigest>,
    pub binary_set_digest: String,
    pub scenario_digest: String,
    pub workload_digest: String,
    pub slo_digest: String,
    pub methodology_digest: String,
    pub stable: bool,
    pub maximum_spread_ratio: f64,
    #[serde(rename = "metric")]
    pub metrics: Vec<ReportMetric>,
    pub receipt_sha256: String,
}

impl MacroReportReceipt {
    /// Recompute the canonical receipt digest with the seal field cleared.
    pub fn recomputed_receipt_sha256(&self) -> Result<String, MacroReceiptError> {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        canonical_json_sha256(&payload)
    }

    /// Seal this receipt after all producer-owned fields are final.
    pub fn seal(&mut self) -> Result<(), MacroReceiptError> {
        self.receipt_sha256 = self.recomputed_receipt_sha256()?;
        Ok(())
    }

    /// Validate that no field in this receipt changed after sealing.
    pub fn receipt_is_valid(&self) -> bool {
        is_sha256(&self.receipt_sha256)
            && self
                .recomputed_receipt_sha256()
                .is_ok_and(|digest| digest == self.receipt_sha256)
    }
}

/// Exact on-disk shape accepted by W7 for a macro report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroReportEnvelope<T> {
    pub report: T,
    pub budget_receipt: MacroReportReceipt,
}

impl<T> MacroReportEnvelope<T>
where
    T: Serialize,
{
    /// Revalidate the complete outer seal, including the canonical digest of
    /// the typed producer report. Suite orchestration calls this only after
    /// every raw-report consumer has finished, immediately before publishing
    /// the W7 envelope at the canonical budget path.
    pub fn validate_seal(&self) -> Result<(), MacroReceiptError> {
        if !self.budget_receipt.receipt_is_valid() {
            return Err(MacroReceiptError::Envelope(
                "macro receipt canonical seal does not recompute".to_owned(),
            ));
        }
        let observed = canonical_report_sha256(&self.report)?;
        if self.budget_receipt.source_report_sha256 != observed {
            return Err(MacroReceiptError::Envelope(
                "typed source report differs from the sealed source_report_sha256".to_owned(),
            ));
        }
        Ok(())
    }

    /// Serialize a fully sealed envelope without performing any filesystem
    /// mutation. This lets an aggregate suite prepare every W4-W6 envelope in
    /// memory first and publish them together only at the end of the suite.
    pub fn to_pretty_json(&self) -> Result<Vec<u8>, MacroReceiptError> {
        self.validate_seal()?;
        serde_json::to_vec_pretty(self)
            .map_err(|error| MacroReceiptError::Serialization(error.to_string()))
    }
}

/// One durable raw backup and its atomically landed macro envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroPublicationArtifact {
    pub report_id: String,
    pub canonical_path: String,
    pub raw_sidecar_path: String,
    pub raw_sha256: String,
    pub source_report_sha256: String,
    pub envelope_sha256: String,
}

/// Marker written only after every W4-W6 envelope reached its canonical path.
/// Its absence makes a crashed/partial batch ineligible even when some
/// individual envelopes happen to be well formed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroBatchPublicationReceipt {
    pub schema_version: u32,
    pub release: String,
    pub source_commit: String,
    pub runner_profile: String,
    pub runner_fingerprint: String,
    pub prebuild_manifest_sha256: String,
    pub artifacts: Vec<MacroPublicationArtifact>,
    pub receipt_sha256: String,
}

impl MacroBatchPublicationReceipt {
    pub fn recomputed_receipt_sha256(&self) -> Result<String, MacroReceiptError> {
        let mut payload = self.clone();
        payload.receipt_sha256.clear();
        canonical_json_sha256(&payload)
    }

    pub fn seal(&mut self) -> Result<(), MacroReceiptError> {
        self.artifacts
            .sort_by(|left, right| left.report_id.cmp(&right.report_id));
        self.receipt_sha256 = self.recomputed_receipt_sha256()?;
        Ok(())
    }

    pub fn receipt_is_valid(&self) -> bool {
        is_sha256(&self.receipt_sha256)
            && self
                .recomputed_receipt_sha256()
                .is_ok_and(|digest| digest == self.receipt_sha256)
    }
}

/// In-memory, fully validated publication input. Fields are intentionally
/// private so callers cannot substitute bytes after preparation.
#[derive(Debug)]
pub struct PreparedMacroArtifact {
    report_id: String,
    canonical_path: PathBuf,
    canonical_relative_path: String,
    raw_bytes: Vec<u8>,
    raw_sha256: String,
    envelope_bytes: Vec<u8>,
    envelope_sha256: String,
    source_report_sha256: String,
    source_commit: String,
    runner_profile: String,
    runner_fingerprint: String,
    prebuild_manifest_sha256: String,
}

/// Prepare one raw report for the final batch without mutating the filesystem.
/// The typed envelope must describe the exact JSON value already present at the
/// report's canonical path.
pub fn prepare_macro_artifact<T>(
    repo_root: &Path,
    raw_report_path: &Path,
    envelope: &MacroReportEnvelope<T>,
) -> Result<PreparedMacroArtifact, MacroReceiptError>
where
    T: Serialize,
{
    envelope.validate_seal()?;
    let canonical_root = fs::canonicalize(repo_root).map_err(|error| {
        MacroReceiptError::Publication(format!(
            "unable to canonicalize repository root {}: {error}",
            repo_root.display()
        ))
    })?;
    let canonical_path = fs::canonicalize(raw_report_path).map_err(|error| {
        MacroReceiptError::Publication(format!(
            "unable to canonicalize raw macro report {}: {error}",
            raw_report_path.display()
        ))
    })?;
    let expected_relative = expected_macro_path(&envelope.budget_receipt.report_id)?;
    let expected_path = canonical_root.join(expected_relative);
    if canonical_path != expected_path || !safe_relative_json_path(expected_relative) {
        return Err(MacroReceiptError::Publication(format!(
            "{} is not the exact canonical path for {}",
            canonical_path.display(),
            envelope.budget_receipt.report_id
        )));
    }
    let raw_bytes = read_bounded(&canonical_path)?;
    let raw_value: serde_json::Value = serde_json::from_slice(&raw_bytes).map_err(|error| {
        MacroReceiptError::Publication(format!(
            "raw macro report {} is not typed JSON: {error}",
            canonical_path.display()
        ))
    })?;
    let typed_value = serde_json::to_value(&envelope.report)
        .map_err(|error| MacroReceiptError::Serialization(error.to_string()))?;
    if raw_value != typed_value {
        return Err(MacroReceiptError::Publication(format!(
            "raw macro report {} differs from its prepared typed envelope",
            canonical_path.display()
        )));
    }
    let envelope_bytes = envelope.to_pretty_json()?;
    Ok(PreparedMacroArtifact {
        report_id: envelope.budget_receipt.report_id.clone(),
        canonical_path,
        canonical_relative_path: expected_relative.to_owned(),
        raw_sha256: sha256(&raw_bytes),
        raw_bytes,
        envelope_sha256: sha256(&envelope_bytes),
        envelope_bytes,
        source_report_sha256: envelope.budget_receipt.source_report_sha256.clone(),
        source_commit: envelope.budget_receipt.source_commit.clone(),
        runner_profile: envelope.budget_receipt.runner_profile.clone(),
        runner_fingerprint: envelope.budget_receipt.runner_fingerprint.clone(),
        prebuild_manifest_sha256: envelope.budget_receipt.prebuild_manifest_sha256.clone(),
    })
}

/// Publish the exact W4-W6 macro set as a single fail-closed tail operation.
/// Every raw report remains recoverable in `w7-raw`; the batch receipt is the
/// final atomic marker. Existing sidecars, temps, envelopes, or markers are
/// never overwritten.
pub fn publish_macro_batch(
    repo_root: &Path,
    prepared: Vec<PreparedMacroArtifact>,
) -> Result<MacroBatchPublicationReceipt, MacroReceiptError> {
    publish_macro_batch_inner(repo_root, prepared, None)
}

fn publish_macro_batch_inner(
    repo_root: &Path,
    mut prepared: Vec<PreparedMacroArtifact>,
    fail_after_landings: Option<usize>,
) -> Result<MacroBatchPublicationReceipt, MacroReceiptError> {
    let canonical_root = fs::canonicalize(repo_root).map_err(|error| {
        MacroReceiptError::Publication(format!(
            "unable to canonicalize repository root {}: {error}",
            repo_root.display()
        ))
    })?;
    prepared.sort_by(|left, right| left.report_id.cmp(&right.report_id));
    validate_prepared_batch(&canonical_root, &prepared)?;

    let recovery_dir = canonical_root.join(MACRO_RAW_DIR_RELATIVE);
    if recovery_dir.exists() {
        return Err(MacroReceiptError::Publication(format!(
            "stale W7 raw/temp/publication state already exists at {}; recover or remove it explicitly",
            recovery_dir.display()
        )));
    }
    fs::create_dir(&recovery_dir).map_err(|error| {
        MacroReceiptError::Publication(format!(
            "unable to create W7 recovery directory {}: {error}",
            recovery_dir.display()
        ))
    })?;

    let mut receipt = MacroBatchPublicationReceipt {
        schema_version: PERF_SCHEMA_VERSION,
        release: PERF_RELEASE.to_owned(),
        source_commit: prepared[0].source_commit.clone(),
        runner_profile: prepared[0].runner_profile.clone(),
        runner_fingerprint: prepared[0].runner_fingerprint.clone(),
        prebuild_manifest_sha256: prepared[0].prebuild_manifest_sha256.clone(),
        artifacts: Vec::with_capacity(prepared.len()),
        receipt_sha256: String::new(),
    };
    let mut staged_paths = BTreeMap::new();
    for artifact in &prepared {
        let stem = safe_report_stem(&artifact.report_id)?;
        let raw_relative = format!("{MACRO_RAW_DIR_RELATIVE}/{stem}.raw.json");
        let raw_path = canonical_root.join(&raw_relative);
        create_new_synced(&raw_path, &artifact.raw_bytes)?;
        let staged_path = recovery_dir.join(format!("{stem}.envelope.tmp"));
        create_new_synced(&staged_path, &artifact.envelope_bytes)?;
        staged_paths.insert(artifact.report_id.clone(), staged_path);
        receipt.artifacts.push(MacroPublicationArtifact {
            report_id: artifact.report_id.clone(),
            canonical_path: artifact.canonical_relative_path.clone(),
            raw_sidecar_path: raw_relative,
            raw_sha256: artifact.raw_sha256.clone(),
            source_report_sha256: artifact.source_report_sha256.clone(),
            envelope_sha256: artifact.envelope_sha256.clone(),
        });
    }
    receipt.seal()?;
    let marker_path = canonical_root.join(MACRO_PUBLICATION_RECEIPT_RELATIVE);
    let marker_temp = recovery_dir.join("macro-publication-receipt.tmp");
    let marker_bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| MacroReceiptError::Serialization(error.to_string()))?;
    create_new_synced(&marker_temp, &marker_bytes)?;

    let mut landed = Vec::new();
    for artifact in &prepared {
        let observed = read_bounded(&artifact.canonical_path)?;
        if sha256(&observed) != artifact.raw_sha256 || observed != artifact.raw_bytes {
            rollback_landings(&prepared, &landed)?;
            return Err(MacroReceiptError::Publication(format!(
                "raw macro report {} changed after preparation",
                artifact.canonical_path.display()
            )));
        }
        if let Err(error) = fs::remove_file(&artifact.canonical_path) {
            rollback_landings(&prepared, &landed)?;
            return Err(MacroReceiptError::Publication(format!(
                "unable to remove raw report {} before atomic landing: {error}",
                artifact.canonical_path.display()
            )));
        }
        let staged_path = staged_paths
            .get(&artifact.report_id)
            .expect("validated prepared artifact has a staged path");
        if let Err(error) = fs::rename(staged_path, &artifact.canonical_path) {
            restore_raw_create_new(artifact)?;
            rollback_landings(&prepared, &landed)?;
            return Err(MacroReceiptError::Publication(format!(
                "unable to atomically land {}: {error}",
                artifact.canonical_path.display()
            )));
        }
        landed.push(artifact.report_id.clone());
        if let Err(error) = sync_file(&artifact.canonical_path) {
            rollback_landings(&prepared, &landed)?;
            return Err(error);
        }
        if fail_after_landings == Some(landed.len()) {
            rollback_landings(&prepared, &landed)?;
            return Err(MacroReceiptError::Publication(
                "injected publication interruption before final marker".to_owned(),
            ));
        }
    }

    if marker_path.exists() {
        rollback_landings(&prepared, &landed)?;
        return Err(MacroReceiptError::Publication(format!(
            "publication marker {} already exists",
            marker_path.display()
        )));
    }
    if let Err(error) = fs::rename(&marker_temp, &marker_path) {
        rollback_landings(&prepared, &landed)?;
        return Err(MacroReceiptError::Publication(format!(
            "unable to atomically land final W7 marker {}: {error}",
            marker_path.display()
        )));
    }
    if let Err(error) = sync_file(&marker_path).and_then(|()| sync_directory(&recovery_dir)) {
        let _ = fs::remove_file(&marker_path);
        rollback_landings(&prepared, &landed)?;
        return Err(error);
    }
    Ok(receipt)
}

/// Producer-owned inputs that cannot be inferred from the validated reference
/// context. All digests must identify committed scenario/methodology material.
#[derive(Debug, Clone, PartialEq)]
pub struct MacroReceiptInputs {
    pub report_id: String,
    pub claim_scope: String,
    pub run_mode: EvidenceRunMode,
    pub scenario_digest: String,
    pub workload_digest: String,
    pub slo_digest: String,
    pub methodology_digest: String,
    pub stable: bool,
    pub maximum_spread_ratio: f64,
    pub metrics: Vec<ReportMetric>,
}

/// Construct and seal a macro envelope from a fully validated reference
/// context. No caller-supplied source, runner, build, manifest, or binary
/// identity is accepted.
pub fn build_macro_report_envelope<T>(
    context: &ValidatedRespReferenceContext,
    report: T,
    inputs: MacroReceiptInputs,
) -> Result<MacroReportEnvelope<T>, MacroReceiptError>
where
    T: Serialize,
{
    validate_context(context)?;
    validate_inputs(&inputs)?;
    // Reports cross a typed-producer -> generic-JSON checker boundary. Hash a
    // normalized JSON value so struct declaration order and object-map order
    // cannot make the producer/checker disagree about identical JSON.
    let source_report_sha256 = canonical_report_sha256(&report)?;
    let mut binary_sha256 = context
        .build
        .binary_sha256
        .iter()
        .map(|(id, sha256)| BinaryDigest {
            id: id.clone(),
            sha256: sha256.clone(),
        })
        .collect::<Vec<_>>();
    binary_sha256.sort_by(|left, right| left.id.cmp(&right.id));
    let binary_set_digest = canonical_json_sha256(&binary_sha256)?;
    let runner_contract_digest = canonical_json_sha256(&context.profile)?;
    let mut receipt = MacroReportReceipt {
        schema_version: PERF_SCHEMA_VERSION,
        release: PERF_RELEASE.to_owned(),
        report_id: inputs.report_id,
        source_report_sha256,
        claim_scope: inputs.claim_scope,
        run_mode: inputs.run_mode,
        runner_profile: context.profile.name.clone(),
        runner_contract: context.profile.clone(),
        runner_contract_digest,
        observed_runner: context.runner.clone(),
        runner_fingerprint: context.runner.fingerprint.clone(),
        source_commit: context.source.git_commit.clone(),
        cargo_lock_sha256: context.source.cargo_lock_sha256.clone(),
        toolchain_identity: context.source.toolchain.clone(),
        prebuild_contract_digest: context.build.prebuild_contract_digest.clone(),
        prebuild_manifest_sha256: context.manifest_sha256.clone(),
        binary_sha256,
        binary_set_digest,
        scenario_digest: inputs.scenario_digest,
        workload_digest: inputs.workload_digest,
        slo_digest: inputs.slo_digest,
        methodology_digest: inputs.methodology_digest,
        stable: inputs.stable,
        maximum_spread_ratio: inputs.maximum_spread_ratio,
        metrics: inputs.metrics,
        receipt_sha256: String::new(),
    };
    receipt.seal()?;
    let envelope = MacroReportEnvelope {
        report,
        budget_receipt: receipt,
    };
    envelope.validate_seal()?;
    Ok(envelope)
}

/// Typed W4A producer: validates the archived real-daemon report and derives
/// every budget field from the report/scenario pair.
pub fn build_control_plane_macro_envelope(
    context: &ValidatedRespReferenceContext,
    scenario: &ControlPlaneScenario,
    report: ControlPlaneReport,
) -> Result<MacroReportEnvelope<ControlPlaneReport>, MacroReceiptError> {
    report
        .validate_archived(scenario)
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    let maximum = report
        .membership_events
        .iter()
        .map(|event| event.convergence_latency_nanos)
        .max()
        .ok_or_else(|| MacroReceiptError::Report("W4A has no membership events".to_owned()))?
        as f64
        / 1_000_000.0;
    let spread = report
        .steady_reads
        .iter()
        .flat_map(|read| read.knee.evaluated.iter())
        .map(|point| point.sample.robust_spread_ratio)
        .fold(0.0_f64, f64::max);
    let report_id = format!("control-plane-{}-reference-v1", report.node_count);
    let inputs = MacroReceiptInputs {
        report_id,
        claim_scope: "w4a-real-daemon-control-plane".to_owned(),
        run_mode: EvidenceRunMode::ReferenceEvidence,
        scenario_digest: canonical_json_sha256(scenario)?,
        workload_digest: domain_digest("w4a-workload-v1", &scenario.read_only)?,
        slo_digest: domain_digest("w4a-slo-v1", &scenario.sustainability_criteria())?,
        methodology_digest: domain_digest(
            "w4a-methodology-v1",
            &(
                &scenario.identity,
                &scenario.membership_event,
                &scenario.reference,
            ),
        )?,
        stable: spread <= 0.05,
        maximum_spread_ratio: spread,
        metrics: vec![ReportMetric {
            id: "membership_add_drain_commit_and_convergence_latency.max_milliseconds".to_owned(),
            value: maximum,
            unit: "milliseconds".to_owned(),
        }],
    };
    build_macro_report_envelope(context, report, inputs)
}

/// Typed W4B producer for the explicitly in-process library/model tier.
pub fn build_grid_model_macro_envelope(
    context: &ValidatedRespReferenceContext,
    scenario: &GridModelScenario,
    report: GridModelReport,
) -> Result<MacroReportEnvelope<GridModelReport>, MacroReceiptError> {
    scenario
        .validate_exact_reference_shape()
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    report
        .validate(scenario)
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    if report.run_mode != GridModelRunMode::Reference {
        return Err(MacroReceiptError::Report(
            "W4B budget envelope requires reference mode".to_owned(),
        ));
    }
    let maximum = report
        .ack_requirement_cost
        .iter()
        .map(|point| point.timing.median_elapsed_nanos as f64 / point.iterations as f64)
        .max_by(f64::total_cmp)
        .ok_or_else(|| MacroReceiptError::Report("W4B has no ack-cost points".to_owned()))?;
    let spread = report
        .ack_requirement_cost
        .iter()
        .map(|point| point.timing.robust_spread_ratio_millionths)
        .chain(
            report
                .session_decision_cost
                .iter()
                .map(|point| point.timing.robust_spread_ratio_millionths),
        )
        .chain(
            report
                .replication_primitive_curve
                .iter()
                .map(|point| point.timing.robust_spread_ratio_millionths),
        )
        .chain(
            report
                .invalidation_fanout_cost
                .iter()
                .map(|point| point.timing.robust_spread_ratio_millionths),
        )
        .max()
        .unwrap_or(0) as f64
        / 1_000_000.0;
    let inputs = MacroReceiptInputs {
        report_id: "grid-model-reference-v1".to_owned(),
        claim_scope: "w4b-in-process-library-model".to_owned(),
        run_mode: EvidenceRunMode::ReferenceEvidence,
        scenario_digest: scenario.contract_sha256(),
        workload_digest: domain_digest("w4b-workload-v1", &scenario.dimensions)?,
        slo_digest: domain_digest("w4b-slo-v1", &scenario.measurement)?,
        methodology_digest: domain_digest(
            "w4b-methodology-v1",
            &(&scenario.identity, &scenario.reference),
        )?,
        stable: spread <= 0.05,
        maximum_spread_ratio: spread,
        metrics: vec![ReportMetric {
            id: "consistency_ack_requirement_cost_by_level_and_replica_shape.max_nanoseconds_per_operation"
                .to_owned(),
            value: maximum,
            unit: "nanoseconds_per_operation".to_owned(),
        }],
    };
    build_macro_report_envelope(context, report, inputs)
}

pub fn build_control_plane_brownout_macro_envelope(
    context: &ValidatedRespReferenceContext,
    scenario: &ControlPlaneBrownoutScenario,
    report: ControlPlaneBrownoutReport,
) -> Result<MacroReportEnvelope<ControlPlaneBrownoutReport>, MacroReceiptError> {
    scenario
        .validate_exact_reference_shape()
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    report
        .validate(scenario)
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    if report.run_mode != BrownoutRunMode::Reference {
        return Err(MacroReceiptError::Report(
            "W5A budget envelope requires reference mode".to_owned(),
        ));
    }
    let recovery = report
        .events
        .iter()
        .map(|event| event.transition_recovery_millis)
        .max()
        .ok_or_else(|| MacroReceiptError::Report("W5A has no events".to_owned()))?;
    let depth = report
        .events
        .iter()
        .map(|event| open_loop_availability_dip_ppm(&event.raw.disruption_window))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or_else(|| MacroReceiptError::Report("W5A has no disruption windows".to_owned()))?;
    let inputs = MacroReceiptInputs {
        report_id: "brownout-control-plane-reference-v1".to_owned(),
        claim_scope: "w5a-control-plane-metadata-brownout".to_owned(),
        run_mode: EvidenceRunMode::ReferenceEvidence,
        scenario_digest: scenario.contract_sha256(),
        workload_digest: domain_digest("w5a-workload-v1", &scenario.load)?,
        slo_digest: domain_digest("w5a-slo-v1", &scenario.events)?,
        methodology_digest: domain_digest(
            "w5a-methodology-v1",
            &(&scenario.identity, &scenario.reference),
        )?,
        stable: true,
        maximum_spread_ratio: 0.0,
        metrics: vec![
            ReportMetric {
                id: "control_plane_brownout.recovery_milliseconds".to_owned(),
                value: recovery as f64,
                unit: "milliseconds".to_owned(),
            },
            ReportMetric {
                id: "control_plane_brownout.maximum_availability_dip_ppm".to_owned(),
                value: depth as f64,
                unit: "parts_per_million".to_owned(),
            },
        ],
    };
    build_macro_report_envelope(context, report, inputs)
}

pub fn build_resp_brownout_macro_envelope(
    context: &ValidatedRespReferenceContext,
    scenario: &RespBrownoutScenario,
    report: RespBrownoutReport,
) -> Result<MacroReportEnvelope<RespBrownoutReport>, MacroReceiptError> {
    scenario
        .validate_exact_reference_shape()
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    report
        .validate(scenario)
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    if report.run_mode != BrownoutRunMode::Reference {
        return Err(MacroReceiptError::Report(
            "W5B budget envelope requires reference mode".to_owned(),
        ));
    }
    let depth = open_loop_availability_dip_ppm(&report.event.disruption_window)?;
    let inputs = MacroReceiptInputs {
        report_id: "brownout-resp-endpoint-reference-v1".to_owned(),
        claim_scope: "w5b-selected-node-local-resp-brownout".to_owned(),
        run_mode: EvidenceRunMode::ReferenceEvidence,
        scenario_digest: scenario.contract_sha256(),
        workload_digest: domain_digest("w5b-workload-v1", &scenario.load)?,
        slo_digest: domain_digest("w5b-slo-v1", &scenario.event)?,
        methodology_digest: domain_digest(
            "w5b-methodology-v1",
            &(&scenario.identity, &scenario.reference),
        )?,
        stable: true,
        maximum_spread_ratio: 0.0,
        metrics: vec![
            ReportMetric {
                id: "resp_endpoint_brownout.recovery_milliseconds".to_owned(),
                value: report.selected_endpoint_recovery_millis as f64,
                unit: "milliseconds".to_owned(),
            },
            ReportMetric {
                id: "resp_endpoint_brownout.availability_dip_ppm".to_owned(),
                value: depth as f64,
                unit: "parts_per_million".to_owned(),
            },
        ],
    };
    build_macro_report_envelope(context, report, inputs)
}

pub fn build_grid_model_brownout_macro_envelope(
    context: &ValidatedRespReferenceContext,
    scenario: &GridModelBrownoutScenario,
    report: GridModelBrownoutReport,
) -> Result<MacroReportEnvelope<GridModelBrownoutReport>, MacroReceiptError> {
    scenario
        .validate_exact_reference_shape()
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    report
        .validate(scenario)
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    if report.run_mode != BrownoutRunMode::Reference {
        return Err(MacroReceiptError::Report(
            "W5C budget envelope requires reference mode".to_owned(),
        ));
    }
    let recovery = report
        .faults
        .iter()
        .map(|fault| fault.recovery_timing.median_nanos_per_iteration)
        .max()
        .unwrap_or(0);
    let increase = report
        .faults
        .iter()
        .map(|fault| {
            fault
                .fault_timing
                .median_nanos_per_iteration
                .saturating_sub(fault.baseline_timing.median_nanos_per_iteration)
        })
        .max()
        .unwrap_or(0);
    let spread = report
        .faults
        .iter()
        .flat_map(|fault| {
            [
                fault.baseline_timing.robust_spread_ratio_millionths,
                fault.fault_timing.robust_spread_ratio_millionths,
                fault.recovery_timing.robust_spread_ratio_millionths,
            ]
        })
        .max()
        .unwrap_or(0) as f64
        / 1_000_000.0;
    let inputs = MacroReceiptInputs {
        report_id: "brownout-grid-model-reference-v1".to_owned(),
        claim_scope: "w5c-in-process-model-replica-fault".to_owned(),
        run_mode: EvidenceRunMode::ReferenceEvidence,
        scenario_digest: scenario.contract_sha256(),
        workload_digest: domain_digest("w5c-workload-v1", &scenario.work)?,
        slo_digest: domain_digest("w5c-slo-v1", &scenario.faults)?,
        methodology_digest: domain_digest(
            "w5c-methodology-v1",
            &(&scenario.identity, &scenario.reference),
        )?,
        stable: spread <= 0.05,
        maximum_spread_ratio: spread,
        metrics: vec![
            ReportMetric {
                id: "grid_model_fault.maximum_recovery_cost_nanos".to_owned(),
                value: recovery as f64,
                unit: "nanoseconds".to_owned(),
            },
            ReportMetric {
                id: "grid_model_fault.maximum_decision_cost_increase_nanos".to_owned(),
                value: increase as f64,
                unit: "nanoseconds".to_owned(),
            },
        ],
    };
    build_macro_report_envelope(context, report, inputs)
}

pub fn build_overload_macro_envelope(
    context: &ValidatedRespReferenceContext,
    scenario: &OverloadScenario,
    report: OverloadReport,
) -> Result<MacroReportEnvelope<OverloadReport>, MacroReceiptError> {
    scenario
        .validate()
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    report
        .validate(scenario)
        .map_err(|error| MacroReceiptError::Report(error.to_string()))?;
    if report.run_mode != OverloadRunMode::Reference {
        return Err(MacroReceiptError::Report(
            "W6 budget envelope requires reference mode".to_owned(),
        ));
    }
    let (report_id, surface_name) = match report.surface {
        EligibleOverloadSurface::Local => ("overload-local-v1", "local"),
        EligibleOverloadSurface::ClientSurface => ("overload-client-surface-v1", "client-surface"),
        EligibleOverloadSurface::NodeResp => ("overload-node-resp-v1", "node-resp"),
    };
    if report.report_id != report_id {
        return Err(MacroReceiptError::Report(format!(
            "W6 report id {:?} differs from its typed surface {surface_name}",
            report.report_id
        )));
    }
    let minimum = report
        .points
        .iter()
        .map(|point| point.aggregate.successful_goodput_per_second)
        .min_by(f64::total_cmp)
        .ok_or_else(|| MacroReceiptError::Report("W6 has no overload points".to_owned()))?;
    let spread = report
        .points
        .iter()
        .map(|point| point.aggregate.robust_goodput_spread_ratio)
        .fold(0.0_f64, f64::max);
    let inputs = MacroReceiptInputs {
        report_id: report_id.to_owned(),
        claim_scope: "capacity-bound-overload-goodput-recovery".to_owned(),
        run_mode: EvidenceRunMode::ReferenceEvidence,
        scenario_digest: scenario.contract_digest().map_err(|error| {
            MacroReceiptError::Report(format!("W6 scenario digest failed: {error}"))
        })?,
        workload_digest: domain_digest(
            "w6-workload-v1",
            &(&scenario.work, surface_name),
        )?,
        slo_digest: domain_digest("w6-slo-v1", &scenario.work)?,
        methodology_digest: domain_digest(
            "w6-methodology-v1",
            &(&scenario.identity, &scenario.reference, surface_name),
        )?,
        stable: spread <= 0.05,
        maximum_spread_ratio: spread,
        metrics: vec![ReportMetric {
            id: "overload_goodput_curve_1_2x_1_5x_2x_knee_per_eligible_surface.minimum_goodput_per_second"
                .to_owned(),
            value: minimum,
            unit: "operations_per_second".to_owned(),
        }],
    };
    build_macro_report_envelope(context, report, inputs)
}

fn domain_digest<T: Serialize + ?Sized>(
    domain: &str,
    value: &T,
) -> Result<String, MacroReceiptError> {
    canonical_json_sha256(&(domain, value))
}

fn open_loop_availability_dip_ppm(window: &OpenLoopObservation) -> Result<u64, MacroReceiptError> {
    let outcomes = window
        .successes
        .checked_add(window.errors)
        .and_then(|value| value.checked_add(window.timeouts))
        .and_then(|value| value.checked_add(window.rejections));
    if window.offered == 0
        || window.started != window.offered
        || window.completed != window.started
        || outcomes != Some(window.completed)
        || !window.backlog_drained
        || window.latency.samples != window.completed
        || window.latency.overflow_count != 0
    {
        return Err(MacroReceiptError::Report(
            "brownout open-loop disruption counters/backlog/latency do not balance".to_owned(),
        ));
    }
    let availability = u128::from(window.successes)
        .saturating_mul(1_000_000)
        .checked_div(u128::from(window.offered))
        .unwrap_or(0)
        .min(1_000_000);
    Ok(1_000_000_u64.saturating_sub(availability as u64))
}

fn validate_context(context: &ValidatedRespReferenceContext) -> Result<(), MacroReceiptError> {
    context
        .verify_binaries_unchanged()
        .map_err(|error| MacroReceiptError::Context(error.to_string()))?;
    let profile_problems = context.profile.contract_problems();
    if context.profile.name != REFERENCE_PROFILE
        || !profile_problems.is_empty()
        || !context.profile.validate(&context.runner).eligible
        || context.runner.fingerprint.trim().is_empty()
    {
        return Err(MacroReceiptError::Context(
            "macro ship evidence requires the eligible validated reference-v1 runner".to_owned(),
        ));
    }
    if !is_git_commit(&context.source.git_commit)
        || !is_sha256(&context.source.cargo_lock_sha256)
        || context.source.toolchain.trim().is_empty()
        || context.source.build_flags.is_empty()
        || !is_sha256(&context.manifest_sha256)
        || context.manifest_sha256 != context.build.prebuild_manifest_sha256
        || !is_sha256(&context.build.prebuild_contract_digest)
    {
        return Err(MacroReceiptError::Context(
            "validated source/build/prebuild identity is incomplete".to_owned(),
        ));
    }
    let manifest_bytes = fs::read(&context.manifest_path).map_err(|error| {
        MacroReceiptError::Context(format!(
            "unable to re-read prebuild manifest {}: {error}",
            context.manifest_path.display()
        ))
    })?;
    if sha256(&manifest_bytes) != context.manifest_sha256 {
        return Err(MacroReceiptError::Context(
            "prebuild manifest changed after reference-context validation".to_owned(),
        ));
    }
    let cargo_lock_path = context.repo_root.join("Cargo.lock");
    let cargo_lock_bytes = fs::read(&cargo_lock_path).map_err(|error| {
        MacroReceiptError::Context(format!(
            "unable to re-read {}: {error}",
            cargo_lock_path.display()
        ))
    })?;
    if sha256(&cargo_lock_bytes) != context.source.cargo_lock_sha256 {
        return Err(MacroReceiptError::Context(
            "Cargo.lock changed after reference-context validation".to_owned(),
        ));
    }
    let expected = [
        (LOADGEN_BINARY_ID, &context.loadgen.sha256),
        (SERVER_BINARY_ID, &context.server.sha256),
    ];
    let observed = context
        .build
        .binary_sha256
        .iter()
        .map(|(id, digest)| (id.as_str(), digest))
        .collect::<Vec<_>>();
    if observed.len() != expected.len()
        || expected.iter().any(|(id, digest)| {
            !is_sha256(digest)
                || !observed.iter().any(|(observed_id, observed_digest)| {
                    observed_id == id && observed_digest == digest
                })
        })
    {
        return Err(MacroReceiptError::Context(
            "validated context does not contain the exact loadgen/server binary set".to_owned(),
        ));
    }
    Ok(())
}

fn validate_inputs(inputs: &MacroReceiptInputs) -> Result<(), MacroReceiptError> {
    if inputs.report_id.trim().is_empty()
        || inputs.claim_scope.trim().is_empty()
        || inputs.run_mode != EvidenceRunMode::ReferenceEvidence
        || [
            &inputs.scenario_digest,
            &inputs.workload_digest,
            &inputs.slo_digest,
            &inputs.methodology_digest,
        ]
        .iter()
        .any(|digest| !is_sha256(digest))
        || !inputs.maximum_spread_ratio.is_finite()
        || !(0.0..=1.0).contains(&inputs.maximum_spread_ratio)
        || inputs.metrics.is_empty()
    {
        return Err(MacroReceiptError::Inputs(
            "macro receipt inputs are incomplete or are not reference evidence".to_owned(),
        ));
    }
    let mut metric_ids = BTreeSet::new();
    if inputs.metrics.iter().any(|metric| {
        metric.id.trim().is_empty()
            || metric.unit.trim().is_empty()
            || !metric.value.is_finite()
            || !metric_ids.insert(metric.id.as_str())
    }) {
        return Err(MacroReceiptError::Inputs(
            "macro receipt metrics must be finite, typed, non-empty, and unique".to_owned(),
        ));
    }
    Ok(())
}

fn validate_prepared_batch(
    canonical_root: &Path,
    prepared: &[PreparedMacroArtifact],
) -> Result<(), MacroReceiptError> {
    let expected = MACRO_REPORT_PATHS
        .iter()
        .map(|(report_id, path)| ((*report_id).to_owned(), (*path).to_owned()))
        .collect::<BTreeMap<_, _>>();
    let observed = prepared
        .iter()
        .map(|artifact| {
            (
                artifact.report_id.clone(),
                artifact.canonical_relative_path.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if prepared.len() != expected.len() || observed != expected {
        return Err(MacroReceiptError::Publication(
            "W7 publication requires the exact complete W4-W6 macro report set".to_owned(),
        ));
    }
    let first = prepared
        .first()
        .ok_or_else(|| MacroReceiptError::Publication("macro batch is empty".to_owned()))?;
    if prepared.iter().any(|artifact| {
        artifact.source_commit != first.source_commit
            || artifact.runner_profile != first.runner_profile
            || artifact.runner_fingerprint != first.runner_fingerprint
            || artifact.prebuild_manifest_sha256 != first.prebuild_manifest_sha256
            || artifact.runner_profile != REFERENCE_PROFILE
            || artifact.canonical_path != canonical_root.join(&artifact.canonical_relative_path)
            || !is_git_commit(&artifact.source_commit)
            || !is_sha256(&artifact.runner_fingerprint)
            || !is_sha256(&artifact.prebuild_manifest_sha256)
            || !is_sha256(&artifact.raw_sha256)
            || !is_sha256(&artifact.envelope_sha256)
            || !is_sha256(&artifact.source_report_sha256)
    }) {
        return Err(MacroReceiptError::Publication(
            "macro batch mixes source, runner, prebuild, path, or digest identities".to_owned(),
        ));
    }
    Ok(())
}

fn expected_macro_path(report_id: &str) -> Result<&'static str, MacroReceiptError> {
    MACRO_REPORT_PATHS
        .iter()
        .find_map(|(expected_id, path)| (*expected_id == report_id).then_some(*path))
        .ok_or_else(|| {
            MacroReceiptError::Publication(format!(
                "{report_id:?} is not an exact W4-W6 macro report id"
            ))
        })
}

fn safe_report_stem(report_id: &str) -> Result<String, MacroReceiptError> {
    if report_id.is_empty()
        || !report_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(MacroReceiptError::Publication(format!(
            "unsafe macro report id {report_id:?}"
        )));
    }
    Ok(report_id.to_owned())
}

fn safe_relative_json_path(path: &str) -> bool {
    let path = Path::new(path);
    path.extension().and_then(|value| value.to_str()) == Some("json")
        && path.starts_with("target/test-evidence/0.67")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, MacroReceiptError> {
    let metadata = fs::metadata(path).map_err(|error| {
        MacroReceiptError::Publication(format!(
            "unable to inspect macro artifact {}: {error}",
            path.display()
        ))
    })?;
    if metadata.len() > 4 * 1024 * 1024 {
        return Err(MacroReceiptError::Publication(format!(
            "macro artifact {} exceeds the 4 MiB cap",
            path.display()
        )));
    }
    fs::read(path).map_err(|error| {
        MacroReceiptError::Publication(format!(
            "unable to read macro artifact {}: {error}",
            path.display()
        ))
    })
}

fn create_new_synced(path: &Path, bytes: &[u8]) -> Result<(), MacroReceiptError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            MacroReceiptError::Publication(format!(
                "refusing to overwrite stale publication file {}: {error}",
                path.display()
            ))
        })?;
    file.write_all(bytes).map_err(|error| {
        MacroReceiptError::Publication(format!("writing {}: {error}", path.display()))
    })?;
    file.sync_all().map_err(|error| {
        MacroReceiptError::Publication(format!("syncing {}: {error}", path.display()))
    })
}

fn sync_file(path: &Path) -> Result<(), MacroReceiptError> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| {
            MacroReceiptError::Publication(format!("syncing {}: {error}", path.display()))
        })
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), MacroReceiptError> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| {
            MacroReceiptError::Publication(format!("syncing {}: {error}", path.display()))
        })
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), MacroReceiptError> {
    Ok(())
}

fn restore_raw_create_new(artifact: &PreparedMacroArtifact) -> Result<(), MacroReceiptError> {
    create_new_synced(&artifact.canonical_path, &artifact.raw_bytes)
}

fn rollback_landings(
    prepared: &[PreparedMacroArtifact],
    landed: &[String],
) -> Result<(), MacroReceiptError> {
    let landed = landed.iter().map(String::as_str).collect::<BTreeSet<_>>();
    let mut failures = Vec::new();
    for artifact in prepared
        .iter()
        .filter(|artifact| landed.contains(artifact.report_id.as_str()))
    {
        if let Err(error) = fs::remove_file(&artifact.canonical_path) {
            failures.push(format!(
                "removing partial envelope {}: {error}",
                artifact.canonical_path.display()
            ));
            continue;
        }
        if let Err(error) = restore_raw_create_new(artifact) {
            failures.push(error.to_string());
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(MacroReceiptError::Publication(format!(
            "partial publication rollback failed; raw sidecars remain recoverable: {}",
            failures.join("; ")
        )))
    }
}

/// Canonical JSON SHA-256 used by both loadgen producers and the W7 checker.
pub fn canonical_json_sha256<T: Serialize + ?Sized>(
    value: &T,
) -> Result<String, MacroReceiptError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|error| MacroReceiptError::Serialization(error.to_string()))
}

fn canonical_report_sha256<T: Serialize + ?Sized>(value: &T) -> Result<String, MacroReceiptError> {
    serde_json::to_value(value)
        .and_then(|value| serde_json::to_vec(&value))
        .map(|bytes| sha256(&bytes))
        .map_err(|error| MacroReceiptError::Serialization(error.to_string()))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[derive(Debug, thiserror::Error)]
pub enum MacroReceiptError {
    #[error("macro receipt reference context is invalid: {0}")]
    Context(String),
    #[error("macro receipt producer inputs are invalid: {0}")]
    Inputs(String),
    #[error("macro report envelope is invalid: {0}")]
    Envelope(String),
    #[error("macro report publication failed closed: {0}")]
    Publication(String),
    #[error("typed macro report validation failed: {0}")]
    Report(String),
    #[error("macro receipt serialization failed: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temporary_repo(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "hydracache-w7-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(root.join("target/test-evidence/0.67")).expect("evidence root");
        root
    }

    fn prepared_batch(root: &Path) -> Vec<PreparedMacroArtifact> {
        let canonical_root = fs::canonicalize(root).expect("canonical temp repo");
        MACRO_REPORT_PATHS
            .iter()
            .map(|(report_id, relative)| {
                let canonical_path = canonical_root.join(relative);
                let raw_bytes = serde_json::to_vec_pretty(&serde_json::json!({
                    "raw_report_id": report_id,
                }))
                .unwrap();
                let envelope_bytes = serde_json::to_vec_pretty(&serde_json::json!({
                    "report": { "raw_report_id": report_id },
                    "budget_receipt": { "report_id": report_id },
                }))
                .unwrap();
                fs::write(&canonical_path, &raw_bytes).expect("raw report");
                PreparedMacroArtifact {
                    report_id: (*report_id).to_owned(),
                    canonical_path,
                    canonical_relative_path: (*relative).to_owned(),
                    raw_sha256: sha256(&raw_bytes),
                    raw_bytes,
                    envelope_sha256: sha256(&envelope_bytes),
                    envelope_bytes,
                    source_report_sha256: sha256(report_id.as_bytes()),
                    source_commit: "a".repeat(40),
                    runner_profile: "reference-v1".to_owned(),
                    runner_fingerprint: "b".repeat(64),
                    prebuild_manifest_sha256: "c".repeat(64),
                }
            })
            .collect()
    }

    #[test]
    fn macro_batch_marker_is_last_and_existing_publication_is_not_overwritten() {
        let root = temporary_repo("success");
        let prepared = prepared_batch(&root);
        let expected_envelopes = prepared
            .iter()
            .map(|artifact| {
                (
                    artifact.canonical_path.clone(),
                    artifact.envelope_bytes.clone(),
                )
            })
            .collect::<Vec<_>>();
        let receipt = publish_macro_batch(&root, prepared).expect("publish complete batch");
        assert!(receipt.receipt_is_valid());
        assert!(root.join(MACRO_PUBLICATION_RECEIPT_RELATIVE).is_file());
        for (path, expected) in &expected_envelopes {
            assert_eq!(fs::read(path).unwrap(), *expected);
        }

        let retry = MACRO_REPORT_PATHS
            .iter()
            .map(|(report_id, relative)| PreparedMacroArtifact {
                report_id: (*report_id).to_owned(),
                canonical_path: fs::canonicalize(&root)
                    .expect("canonical temp repo")
                    .join(relative),
                canonical_relative_path: (*relative).to_owned(),
                raw_bytes: b"replacement".to_vec(),
                raw_sha256: sha256(b"replacement"),
                envelope_bytes: b"replacement-envelope".to_vec(),
                envelope_sha256: sha256(b"replacement-envelope"),
                source_report_sha256: sha256(report_id.as_bytes()),
                source_commit: "a".repeat(40),
                runner_profile: "reference-v1".to_owned(),
                runner_fingerprint: "b".repeat(64),
                prebuild_manifest_sha256: "c".repeat(64),
            })
            .collect();
        assert!(publish_macro_batch(&root, retry).is_err());
        for (path, expected) in &expected_envelopes {
            assert_eq!(fs::read(path).unwrap(), *expected);
        }
        fs::remove_dir_all(&root).expect("cleanup exact temp repo");
    }

    #[test]
    fn interrupted_macro_batch_has_no_green_marker_and_restores_raw_reports() {
        let root = temporary_repo("partial");
        let prepared = prepared_batch(&root);
        let raw = prepared
            .iter()
            .map(|artifact| (artifact.canonical_path.clone(), artifact.raw_bytes.clone()))
            .collect::<Vec<_>>();
        let result = publish_macro_batch_inner(&root, prepared, Some(1));
        assert!(result.is_err());
        assert!(!root.join(MACRO_PUBLICATION_RECEIPT_RELATIVE).exists());
        for (path, expected) in raw {
            assert_eq!(fs::read(path).unwrap(), expected);
        }
        let recovery = root.join(MACRO_RAW_DIR_RELATIVE);
        assert!(recovery.is_dir());
        assert!(MACRO_REPORT_PATHS
            .iter()
            .all(|(report_id, _)| recovery.join(format!("{report_id}.raw.json")).is_file()));
        fs::remove_dir_all(&root).expect("cleanup exact temp repo");
    }
}
