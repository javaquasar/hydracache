//! Deterministic Management Center health evaluation.
//!
//! Evaluators consume immutable typed observations. They never read a clock,
//! perform I/O, or infer healthy state from process liveness.

use serde::{Deserialize, Serialize};

use crate::management::{
    AdmissionState, CatchUpState, ClusterFormationSnapshot, ConsensusProgressSnapshot,
    DurableRecoveryStatus, FormationReasonCode, ManagementCompleteness, ManagementContractError,
    ManagementObservationSource, PlacementDecisionTrace, PlacementOutcome, RecoveryOutcome,
    RecoveryPhase, RepairState, ServingState, TransportState, MANAGEMENT_API_SCHEMA_VERSION,
};

/// Version of the stable 0.72 health rule semantics.
pub const MANAGEMENT_HEALTH_EVALUATION_VERSION: u16 = 1;
/// Maximum number of health results in one response.
pub const MAX_MANAGEMENT_HEALTH_CHECKS: usize = 64;
/// Maximum bounded evidence rows attached to one result.
pub const MAX_MANAGEMENT_HEALTH_EVIDENCE: usize = 16;

/// Reviewed, unit-bearing thresholds used by the initial rule catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementHealthThresholds {
    pub raft_apply_lag_warn_entries: u64,
    pub raft_apply_lag_fail_entries: u64,
    pub source: HealthThresholdSource,
    pub evaluation_version: u16,
}

impl Default for ManagementHealthThresholds {
    fn default() -> Self {
        Self {
            raft_apply_lag_warn_entries: 100,
            raft_apply_lag_fail_entries: 1_000,
            source: HealthThresholdSource::ReviewedDefault,
            evaluation_version: MANAGEMENT_HEALTH_EVALUATION_VERSION,
        }
    }
}

/// Provenance of a health threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthThresholdSource {
    ReviewedDefault,
    ReviewedConfiguration,
    #[serde(other)]
    Unknown,
}

/// Closed status domain. Missing evidence maps to `Unknown`, never `Pass`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ManagementHealthStatus {
    Pass,
    Warn,
    Fail,
    Unknown,
    Disabled,
}

/// Stable category used by server-side filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagementHealthCategory {
    Authority,
    Formation,
    Consensus,
    Placement,
    Recovery,
    Membership,
    Partitions,
    Replication,
    Repair,
    Reshard,
    Resource,
    Expiry,
    Clients,
    Persistence,
    Audit,
    History,
    #[serde(other)]
    Unknown,
}

/// Stable, privacy-safe evidence code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManagementHealthEvidenceCode {
    RequiredInputMissing,
    SourceNotLive,
    ObservationPartial,
    MixedAuthorityEpoch,
    MemberRejected,
    MemberPending,
    MemberNotServing,
    ApplyLag,
    ThresholdWarn,
    ThresholdFail,
    PlacementIncomplete,
    PlacementRejected,
    PlacementApplied,
    RecoveryClean,
    RecoveryDebt,
    RecoveryFailed,
    CorruptionOrLoss,
    ReconciliationPending,
    ReconciliationComplete,
    #[serde(other)]
    Unknown,
}

/// One bounded numeric or categorical evidence reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagementHealthEvidence {
    pub code: ManagementHealthEvidenceCode,
    pub value: Option<u64>,
    pub unit: Option<&'static str>,
}

/// Result of one stable health evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagementHealthCheck {
    pub id: &'static str,
    pub category: ManagementHealthCategory,
    pub status: ManagementHealthStatus,
    pub title: &'static str,
    pub evidence: Vec<ManagementHealthEvidence>,
    pub affected_count: Option<u64>,
    pub remediation_code: &'static str,
    pub remediation_link: &'static str,
    pub source: ManagementObservationSource,
    pub observation_seq: u64,
    pub evaluation_version: u16,
}

/// Status counts retain UNKNOWN independently of the known headline status.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementHealthCounts {
    pub pass: u64,
    pub warn: u64,
    pub fail: u64,
    pub unknown: u64,
    pub disabled: u64,
}

/// Complete deterministic health catalogue response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagementHealthReport {
    pub schema_version: u16,
    pub aggregate: ManagementHealthStatus,
    pub counts: ManagementHealthCounts,
    pub thresholds: ManagementHealthThresholds,
    pub checks: Vec<ManagementHealthCheck>,
}

impl ManagementHealthReport {
    /// Build a report after a stable server-side filter has selected checks.
    pub fn from_checks(
        thresholds: ManagementHealthThresholds,
        mut checks: Vec<ManagementHealthCheck>,
    ) -> Self {
        checks.sort_by_key(|check| check.id);
        Self {
            schema_version: MANAGEMENT_API_SCHEMA_VERSION,
            aggregate: known_aggregate(&checks),
            counts: count_statuses(&checks),
            thresholds,
            checks,
        }
    }

    /// Validate stable order, cardinality, IDs, evidence bounds, and summary counts.
    pub fn validate(&self) -> Result<(), ManagementContractError> {
        if self.schema_version != MANAGEMENT_API_SCHEMA_VERSION {
            return Err(ManagementContractError::UnsupportedSchema {
                found: self.schema_version,
                expected: MANAGEMENT_API_SCHEMA_VERSION,
            });
        }
        if self.checks.len() > MAX_MANAGEMENT_HEALTH_CHECKS {
            return Err(ManagementContractError::LimitExceeded {
                field: "health.checks",
                limit: MAX_MANAGEMENT_HEALTH_CHECKS,
                actual: self.checks.len(),
            });
        }
        if self.checks.windows(2).any(|pair| pair[0].id >= pair[1].id) {
            return Err(ManagementContractError::UnstableOrder {
                field: "health.checks",
            });
        }
        if self
            .checks
            .iter()
            .any(|check| check.evidence.len() > MAX_MANAGEMENT_HEALTH_EVIDENCE)
        {
            return Err(ManagementContractError::LimitExceeded {
                field: "health.evidence",
                limit: MAX_MANAGEMENT_HEALTH_EVIDENCE,
                actual: self
                    .checks
                    .iter()
                    .map(|check| check.evidence.len())
                    .max()
                    .unwrap_or(0),
            });
        }
        let counts = count_statuses(&self.checks);
        if counts != self.counts || known_aggregate(&self.checks) != self.aggregate {
            return Err(ManagementContractError::Incoherent {
                field: "health.summary",
                reason: "health counts or aggregate disagree with check results",
            });
        }
        Ok(())
    }
}

/// Recovery observation fenced independently from the recovery DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryHealthObservation {
    pub authority_epoch: Option<u64>,
    pub observation_seq: u64,
    pub status: DurableRecoveryStatus,
}

/// Immutable input for all stable 0.72 health rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagementHealthInput {
    pub authority_epoch: Option<u64>,
    pub observation_seq: u64,
    pub source: ManagementObservationSource,
    pub completeness: ManagementCompleteness,
    pub formation: Vec<ClusterFormationSnapshot>,
    pub consensus: Vec<ConsensusProgressSnapshot>,
    pub placement: Option<PlacementDecisionTrace>,
    pub recovery: Option<RecoveryHealthObservation>,
    pub operational: OperationalHealthSignals,
    pub thresholds: ManagementHealthThresholds,
}

/// Raw operational signals for the non-NATS-derived catalogue rules.
///
/// Limits are captured with the observation. An absent value is unknown and
/// cannot be substituted with zero or a UI default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OperationalHealthSignals {
    pub quorum_ok: Option<bool>,
    pub leader_present: Option<bool>,
    pub unreachable_members: Option<u64>,
    pub incompatible_members: Option<u64>,
    pub config_skew_members: Option<u64>,
    pub unassigned_partitions: Option<u64>,
    pub under_replicated_partitions: Option<u64>,
    pub zone_underspread_partitions: Option<u64>,
    pub replication_failures: Option<u64>,
    pub replication_backpressure: Option<u64>,
    pub repair_debt: Option<u64>,
    pub degraded_mode: Option<bool>,
    pub reshard_active: Option<bool>,
    pub reshard_backfill_lag: Option<u64>,
    pub reshard_backfill_lag_limit: Option<u64>,
    pub retained_bytes: Option<u64>,
    pub retained_bytes_limit: Option<u64>,
    pub admission_queue_depth: Option<u64>,
    pub admission_queue_limit: Option<u64>,
    pub expiry_backlog: Option<u64>,
    pub expiry_backlog_limit: Option<u64>,
    pub active_clients: Option<u64>,
    pub active_clients_limit: Option<u64>,
    pub buffered_client_bytes: Option<u64>,
    pub buffered_client_bytes_limit: Option<u64>,
    pub persistence_enabled: Option<bool>,
    pub disk_available: Option<bool>,
    pub backup_age_seconds: Option<u64>,
    pub backup_age_limit_seconds: Option<u64>,
    pub audit_enabled: Option<bool>,
    pub audit_sink_healthy: Option<bool>,
    pub history_enabled: Option<bool>,
    pub history_source_available: Option<bool>,
}

/// Evaluate all stable rules in byte-stable check-ID order.
pub fn evaluate_management_health(input: &ManagementHealthInput) -> ManagementHealthReport {
    let mut checks = vec![
        evaluate_audit(input),
        evaluate_authority(input),
        evaluate_clients(input),
        evaluate_expiry(input),
        evaluate_formation(input),
        evaluate_history(input),
        evaluate_membership(input),
        evaluate_memory(input),
        evaluate_partitions(input),
        evaluate_persistence(input),
        evaluate_placement(input),
        evaluate_raft(input),
        evaluate_repair(input),
        evaluate_replication(input),
        evaluate_recovery_outcome(input),
        evaluate_recovery_corruption(input),
        evaluate_reconciliation(input),
        evaluate_reshard(input),
    ];
    checks.sort_by_key(|check| check.id);
    ManagementHealthReport::from_checks(input.thresholds, checks)
}

fn evaluate_authority(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-AUTH-001",
        ManagementHealthCategory::Authority,
        "Authority quorum and leader are accessible",
        "inspect-authority",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    match (
        input.operational.quorum_ok,
        input.operational.leader_present,
    ) {
        (Some(true), Some(true)) => result,
        (Some(false), _) | (_, Some(false)) => failed(result, 1),
        _ => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
    }
}

fn evaluate_membership(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-MEMBER-001",
        ManagementHealthCategory::Membership,
        "Member reachability, version and configuration agree",
        "inspect-member-skew",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    let values = [
        input.operational.unreachable_members,
        input.operational.incompatible_members,
        input.operational.config_skew_members,
    ];
    if values.iter().any(Option::is_none) {
        return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
    }
    let incompatible = values[1]
        .unwrap_or(0)
        .saturating_add(values[2].unwrap_or(0));
    if incompatible > 0 {
        failed(result, incompatible)
    } else if values[0].unwrap_or(0) > 0 {
        warned(result, values[0].unwrap_or(0))
    } else {
        result
    }
}

fn evaluate_partitions(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-PART-001",
        ManagementHealthCategory::Partitions,
        "Partitions are assigned, replicated and zone-spread",
        "inspect-partitions",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    let values = [
        input.operational.unassigned_partitions,
        input.operational.under_replicated_partitions,
        input.operational.zone_underspread_partitions,
    ];
    if values.iter().any(Option::is_none) {
        return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
    }
    let unsafe_count = values[0]
        .unwrap_or(0)
        .saturating_add(values[1].unwrap_or(0));
    if unsafe_count > 0 {
        failed(result, unsafe_count)
    } else if values[2].unwrap_or(0) > 0 {
        warned(result, values[2].unwrap_or(0))
    } else {
        result
    }
}

fn evaluate_replication(input: &ManagementHealthInput) -> ManagementHealthCheck {
    if !coherent_live_input(input) {
        return unknown(
            base_check(
                input,
                "HC-REPL-001",
                ManagementHealthCategory::Replication,
                "Replication has no failures or backpressure",
                "inspect-replication",
            ),
            missing_evidence(input),
        );
    }
    numeric_pair_check(
        input,
        "HC-REPL-001",
        ManagementHealthCategory::Replication,
        "Replication has no failures or backpressure",
        "inspect-replication",
        input.operational.replication_failures,
        input.operational.replication_backpressure,
    )
}

fn evaluate_repair(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-REPAIR-001",
        ManagementHealthCategory::Repair,
        "Repair debt is clear and degraded mode is inactive",
        "inspect-repair-debt",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    match (
        input.operational.repair_debt,
        input.operational.degraded_mode,
    ) {
        (_, Some(true)) => failed(result, input.operational.repair_debt.unwrap_or(1).max(1)),
        (Some(0), Some(false)) => result,
        (Some(debt), Some(false)) => warned(result, debt),
        _ => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
    }
}

fn evaluate_reshard(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-RESHARD-001",
        ManagementHealthCategory::Reshard,
        "Reshard progress is not stalled",
        "inspect-reshard-progress",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    match input.operational.reshard_active {
        Some(false) => result,
        Some(true) => limit_check(
            result,
            input.operational.reshard_backfill_lag,
            input.operational.reshard_backfill_lag_limit,
        ),
        None => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
    }
}

fn evaluate_memory(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-MEM-001",
        ManagementHealthCategory::Resource,
        "Retained memory and admission queue are within bounds",
        "inspect-memory-admission",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    combined_limit_check(
        result,
        input.operational.retained_bytes,
        input.operational.retained_bytes_limit,
        input.operational.admission_queue_depth,
        input.operational.admission_queue_limit,
    )
}

fn evaluate_expiry(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-EXPIRY-001",
        ManagementHealthCategory::Expiry,
        "Expiry backlog is within its reviewed bound",
        "inspect-expiry-backlog",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    limit_check(
        result,
        input.operational.expiry_backlog,
        input.operational.expiry_backlog_limit,
    )
}

fn evaluate_clients(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-CLIENT-001",
        ManagementHealthCategory::Clients,
        "Client sessions and buffered bytes are within bounds",
        "inspect-client-pressure",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    combined_limit_check(
        result,
        input.operational.active_clients,
        input.operational.active_clients_limit,
        input.operational.buffered_client_bytes,
        input.operational.buffered_client_bytes_limit,
    )
}

fn evaluate_persistence(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-PERSIST-001",
        ManagementHealthCategory::Persistence,
        "Persistence disk and backup freshness are healthy",
        "inspect-persistence",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    match input.operational.persistence_enabled {
        Some(false) => disabled(result),
        Some(true) => match input.operational.disk_available {
            Some(false) => failed(result, 1),
            Some(true) => limit_check(
                result,
                input.operational.backup_age_seconds,
                input.operational.backup_age_limit_seconds,
            ),
            None => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
        },
        None => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
    }
}

fn evaluate_audit(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-AUDIT-001",
        ManagementHealthCategory::Audit,
        "Audit sink is available",
        "inspect-audit-sink",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    explicit_optional_check(
        result,
        input.operational.audit_enabled,
        input.operational.audit_sink_healthy,
    )
}

fn evaluate_history(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-HISTORY-001",
        ManagementHealthCategory::History,
        "Optional history source is available",
        "inspect-history-source",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    explicit_optional_check(
        result,
        input.operational.history_enabled,
        input.operational.history_source_available,
    )
}

/// Accept a recovery observation only when it is coherent and strictly newer.
///
/// Equal-sequence conflicts retain the already published value so arrival order
/// cannot let an unproven improvement replace a verdict.
pub fn retain_newer_recovery_observation(
    current: Option<&RecoveryHealthObservation>,
    candidate: RecoveryHealthObservation,
    authority_epoch: Option<u64>,
) -> Option<RecoveryHealthObservation> {
    let candidate_valid = candidate.authority_epoch == authority_epoch
        && candidate.status.validate().is_ok()
        && candidate.status.source == ManagementObservationSource::Live;
    if !candidate_valid {
        return current.cloned();
    }
    match current {
        Some(current)
            if current.authority_epoch == authority_epoch
                && current.observation_seq >= candidate.observation_seq =>
        {
            Some(current.clone())
        }
        _ => Some(candidate),
    }
}

fn evaluate_formation(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let mut result = base_check(
        input,
        "HC-FORM-001",
        ManagementHealthCategory::Formation,
        "Committed member formation is complete",
        "inspect-formation-blockers",
    );
    if !coherent_live_input(input) || input.formation.is_empty() {
        return unknown(result, missing_evidence(input));
    }
    if input.formation.iter().any(|item| {
        item.validate().is_err()
            || item.authority_epoch != input.authority_epoch
            || item.candidate_observation_seq != Some(input.observation_seq)
    }) {
        return unknown(result, ManagementHealthEvidenceCode::MixedAuthorityEpoch);
    }
    let failed = input
        .formation
        .iter()
        .filter(|item| {
            item.admission == AdmissionState::Rejected
                || matches!(
                    item.blocker,
                    Some(
                        FormationReasonCode::ClusterIdentityMismatch
                            | FormationReasonCode::DuplicateNodeIdentity
                            | FormationReasonCode::GenerationRegression
                            | FormationReasonCode::PeerUnauthenticated
                            | FormationReasonCode::ProtocolIncompatible
                    )
                )
        })
        .count();
    if failed > 0 {
        result.status = ManagementHealthStatus::Fail;
        result.affected_count = Some(failed as u64);
        result.evidence.push(evidence(
            ManagementHealthEvidenceCode::MemberRejected,
            Some(failed as u64),
            Some("members"),
        ));
        return result;
    }
    let unknown_count = input
        .formation
        .iter()
        .filter(|item| {
            item.completeness != ManagementCompleteness::Complete
                || item.source != ManagementObservationSource::Live
                || item.transport == TransportState::Unknown
                || item.admission == AdmissionState::Unknown
                || item.catch_up == CatchUpState::Unknown
                || item.serving == ServingState::Unknown
        })
        .count();
    if unknown_count > 0 {
        result.affected_count = Some(unknown_count as u64);
        return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
    }
    let pending = input
        .formation
        .iter()
        .filter(|item| {
            item.transport != TransportState::Authenticated
                || item.admission != AdmissionState::Admitted
                || item.catch_up != CatchUpState::Current
                || item.serving != ServingState::Serving
        })
        .count();
    if pending > 0 {
        result.status = ManagementHealthStatus::Warn;
        result.affected_count = Some(pending as u64);
        result.evidence.push(evidence(
            ManagementHealthEvidenceCode::MemberPending,
            Some(pending as u64),
            Some("members"),
        ));
    }
    result
}

fn evaluate_raft(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let mut result = base_check(
        input,
        "HC-RAFT-001",
        ManagementHealthCategory::Consensus,
        "Raft apply progress is within bounds",
        "inspect-raft-apply-lag",
    );
    if !coherent_live_input(input) || input.consensus.is_empty() {
        return unknown(result, missing_evidence(input));
    }
    if input.consensus.iter().any(|item| {
        item.authority_epoch != input.authority_epoch
            || item.source != ManagementObservationSource::Live
            || item.completeness != ManagementCompleteness::Complete
            || item.validate().is_err()
            || item.apply_lag().is_none()
    }) {
        return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
    }
    let lag = input
        .consensus
        .iter()
        .filter_map(ConsensusProgressSnapshot::apply_lag)
        .max()
        .unwrap_or(0);
    result.evidence.push(evidence(
        ManagementHealthEvidenceCode::ApplyLag,
        Some(lag),
        Some("entries"),
    ));
    result.evidence.push(evidence(
        ManagementHealthEvidenceCode::ThresholdWarn,
        Some(input.thresholds.raft_apply_lag_warn_entries),
        Some("entries"),
    ));
    result.evidence.push(evidence(
        ManagementHealthEvidenceCode::ThresholdFail,
        Some(input.thresholds.raft_apply_lag_fail_entries),
        Some("entries"),
    ));
    result.status = if lag >= input.thresholds.raft_apply_lag_fail_entries {
        ManagementHealthStatus::Fail
    } else if lag >= input.thresholds.raft_apply_lag_warn_entries {
        ManagementHealthStatus::Warn
    } else {
        ManagementHealthStatus::Pass
    };
    result
}

fn evaluate_placement(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-PLACE-001",
        ManagementHealthCategory::Placement,
        "Placement constraints are satisfiable and applied",
        "inspect-placement-trace",
    );
    if !coherent_live_input(input) {
        return unknown(result, missing_evidence(input));
    }
    let Some(trace) = input.placement.as_ref() else {
        return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
    };
    if trace.validate().is_err()
        || Some(trace.topology_epoch) != input.authority_epoch
        || trace.source != ManagementObservationSource::Live
        || trace.completeness != ManagementCompleteness::Complete
        || (trace.candidates.truncated
            && matches!(
                trace.outcome,
                PlacementOutcome::Unknown | PlacementOutcome::Stale
            ))
    {
        return unknown(result, ManagementHealthEvidenceCode::PlacementIncomplete);
    }
    let mut result = result;
    match trace.outcome {
        PlacementOutcome::Applied
            if trace.selected.len() == usize::from(trace.requested_replicas) =>
        {
            result.evidence.push(evidence(
                ManagementHealthEvidenceCode::PlacementApplied,
                Some(trace.selected.len() as u64),
                Some("replicas"),
            ));
        }
        PlacementOutcome::Rejected => {
            result.status = ManagementHealthStatus::Fail;
            result.evidence.push(evidence(
                ManagementHealthEvidenceCode::PlacementRejected,
                Some(trace.selected.len() as u64),
                Some("replicas"),
            ));
        }
        PlacementOutcome::Proposed | PlacementOutcome::Committed => {
            result.status = ManagementHealthStatus::Warn;
            result.evidence.push(evidence(
                ManagementHealthEvidenceCode::PlacementIncomplete,
                Some(trace.selected.len() as u64),
                Some("replicas"),
            ));
        }
        PlacementOutcome::Stale | PlacementOutcome::Unknown | PlacementOutcome::Applied => {
            return unknown(result, ManagementHealthEvidenceCode::PlacementIncomplete);
        }
    }
    result
}

fn evaluate_recovery_outcome(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-RECOVERY-001",
        ManagementHealthCategory::Recovery,
        "Durable recovery completed safely",
        "inspect-durable-recovery",
    );
    let Some(observation) = coherent_recovery(input) else {
        return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
    };
    let mut result = result;
    match observation.status.outcome {
        RecoveryOutcome::Clean => result.evidence.push(evidence(
            ManagementHealthEvidenceCode::RecoveryClean,
            Some(observation.status.records_checked.value),
            Some("records"),
        )),
        RecoveryOutcome::Repaired => {
            result.status = ManagementHealthStatus::Warn;
            result.evidence.push(evidence(
                ManagementHealthEvidenceCode::RecoveryDebt,
                Some(observation.status.records_recovered.value),
                Some("records"),
            ));
        }
        RecoveryOutcome::Degraded | RecoveryOutcome::Refused => {
            result.status = ManagementHealthStatus::Fail;
            result.evidence.push(evidence(
                ManagementHealthEvidenceCode::RecoveryFailed,
                None,
                None,
            ));
        }
        RecoveryOutcome::Unknown => {
            return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
        }
    }
    result
}

fn evaluate_recovery_corruption(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-RECOVERY-002",
        ManagementHealthCategory::Recovery,
        "No unaccounted corruption or data loss was verified",
        "inspect-corruption-evidence",
    );
    let Some(observation) = coherent_recovery(input) else {
        return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
    };
    let status = &observation.status;
    if status.records_checked.truncated
        || status.records_discarded.truncated
        || status.corrupt_records.truncated
    {
        return unknown(result, ManagementHealthEvidenceCode::ObservationPartial);
    }
    let affected = status
        .records_discarded
        .value
        .saturating_add(status.corrupt_records.value);
    let mut result = result;
    if affected > 0 {
        result.status = ManagementHealthStatus::Fail;
        result.affected_count = Some(affected);
        result.evidence.push(evidence(
            ManagementHealthEvidenceCode::CorruptionOrLoss,
            Some(affected),
            Some("records"),
        ));
    }
    result
}

fn evaluate_reconciliation(input: &ManagementHealthInput) -> ManagementHealthCheck {
    let result = base_check(
        input,
        "HC-RECOVERY-003",
        ManagementHealthCategory::Recovery,
        "Recovery reconciliation is complete",
        "inspect-reconciliation",
    );
    let Some(observation) = coherent_recovery(input) else {
        return unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing);
    };
    let mut result = result;
    if matches!(
        observation.status.outcome,
        RecoveryOutcome::Degraded | RecoveryOutcome::Refused
    ) || observation.status.repair == RepairState::Failed
    {
        result.status = ManagementHealthStatus::Fail;
        result.evidence.push(evidence(
            ManagementHealthEvidenceCode::RecoveryFailed,
            None,
            None,
        ));
    } else if observation.status.phase == RecoveryPhase::Complete
        && matches!(
            observation.status.repair,
            RepairState::NotRequired | RepairState::Complete
        )
    {
        result.evidence.push(evidence(
            ManagementHealthEvidenceCode::ReconciliationComplete,
            None,
            None,
        ));
    } else {
        result.status = ManagementHealthStatus::Warn;
        result.evidence.push(evidence(
            ManagementHealthEvidenceCode::ReconciliationPending,
            None,
            None,
        ));
    }
    result
}

fn coherent_recovery(input: &ManagementHealthInput) -> Option<&RecoveryHealthObservation> {
    let observation = input.recovery.as_ref()?;
    (coherent_live_input(input)
        && observation.authority_epoch == input.authority_epoch
        && observation.observation_seq == input.observation_seq
        && observation.status.source == ManagementObservationSource::Live
        && observation.status.completeness == ManagementCompleteness::Complete
        && observation.status.validate().is_ok())
    .then_some(observation)
}

fn coherent_live_input(input: &ManagementHealthInput) -> bool {
    input.authority_epoch.is_some()
        && input.source == ManagementObservationSource::Live
        && input.completeness == ManagementCompleteness::Complete
}

fn missing_evidence(input: &ManagementHealthInput) -> ManagementHealthEvidenceCode {
    if input.source != ManagementObservationSource::Live {
        ManagementHealthEvidenceCode::SourceNotLive
    } else if input.completeness != ManagementCompleteness::Complete {
        ManagementHealthEvidenceCode::ObservationPartial
    } else {
        ManagementHealthEvidenceCode::RequiredInputMissing
    }
}

fn base_check(
    input: &ManagementHealthInput,
    id: &'static str,
    category: ManagementHealthCategory,
    title: &'static str,
    remediation_code: &'static str,
) -> ManagementHealthCheck {
    ManagementHealthCheck {
        id,
        category,
        status: ManagementHealthStatus::Pass,
        title,
        evidence: Vec::new(),
        affected_count: None,
        remediation_code,
        remediation_link: "/docs/operations/management-healthchecks",
        source: input.source,
        observation_seq: input.observation_seq,
        evaluation_version: MANAGEMENT_HEALTH_EVALUATION_VERSION,
    }
}

fn unknown(
    mut result: ManagementHealthCheck,
    code: ManagementHealthEvidenceCode,
) -> ManagementHealthCheck {
    result.status = ManagementHealthStatus::Unknown;
    result.evidence.push(evidence(code, None, None));
    result
}

fn warned(mut result: ManagementHealthCheck, affected: u64) -> ManagementHealthCheck {
    result.status = ManagementHealthStatus::Warn;
    result.affected_count = Some(affected);
    result
}

fn failed(mut result: ManagementHealthCheck, affected: u64) -> ManagementHealthCheck {
    result.status = ManagementHealthStatus::Fail;
    result.affected_count = Some(affected);
    result
}

fn disabled(mut result: ManagementHealthCheck) -> ManagementHealthCheck {
    result.status = ManagementHealthStatus::Disabled;
    result
}

fn numeric_pair_check(
    input: &ManagementHealthInput,
    id: &'static str,
    category: ManagementHealthCategory,
    title: &'static str,
    remediation: &'static str,
    failure_count: Option<u64>,
    warning_count: Option<u64>,
) -> ManagementHealthCheck {
    let result = base_check(input, id, category, title, remediation);
    match (failure_count, warning_count) {
        (Some(failures), Some(_)) if failures > 0 => failed(result, failures),
        (Some(_), Some(warnings)) if warnings > 0 => warned(result, warnings),
        (Some(_), Some(_)) => result,
        _ => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
    }
}

fn limit_check(
    result: ManagementHealthCheck,
    value: Option<u64>,
    limit: Option<u64>,
) -> ManagementHealthCheck {
    match (value, limit) {
        (Some(value), Some(limit)) if value >= limit => failed(result, value),
        (Some(_), Some(_)) => result,
        _ => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
    }
}

fn combined_limit_check(
    result: ManagementHealthCheck,
    first: Option<u64>,
    first_limit: Option<u64>,
    second: Option<u64>,
    second_limit: Option<u64>,
) -> ManagementHealthCheck {
    match (first, first_limit, second, second_limit) {
        (Some(first), Some(first_limit), Some(second), Some(second_limit))
            if first >= first_limit || second >= second_limit =>
        {
            failed(
                result,
                u64::from(first >= first_limit) + u64::from(second >= second_limit),
            )
        }
        (Some(_), Some(_), Some(_), Some(_)) => result,
        _ => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
    }
}

fn explicit_optional_check(
    result: ManagementHealthCheck,
    enabled: Option<bool>,
    healthy: Option<bool>,
) -> ManagementHealthCheck {
    match (enabled, healthy) {
        (Some(false), _) => disabled(result),
        (Some(true), Some(true)) => result,
        (Some(true), Some(false)) => failed(result, 1),
        _ => unknown(result, ManagementHealthEvidenceCode::RequiredInputMissing),
    }
}

fn evidence(
    code: ManagementHealthEvidenceCode,
    value: Option<u64>,
    unit: Option<&'static str>,
) -> ManagementHealthEvidence {
    ManagementHealthEvidence { code, value, unit }
}

fn count_statuses(checks: &[ManagementHealthCheck]) -> ManagementHealthCounts {
    let mut counts = ManagementHealthCounts::default();
    for check in checks {
        match check.status {
            ManagementHealthStatus::Pass => counts.pass = counts.pass.saturating_add(1),
            ManagementHealthStatus::Warn => counts.warn = counts.warn.saturating_add(1),
            ManagementHealthStatus::Fail => counts.fail = counts.fail.saturating_add(1),
            ManagementHealthStatus::Unknown => counts.unknown = counts.unknown.saturating_add(1),
            ManagementHealthStatus::Disabled => {
                counts.disabled = counts.disabled.saturating_add(1);
            }
        }
    }
    counts
}

fn known_aggregate(checks: &[ManagementHealthCheck]) -> ManagementHealthStatus {
    if checks
        .iter()
        .any(|check| check.status == ManagementHealthStatus::Fail)
    {
        ManagementHealthStatus::Fail
    } else if checks
        .iter()
        .any(|check| check.status == ManagementHealthStatus::Warn)
    {
        ManagementHealthStatus::Warn
    } else if checks
        .iter()
        .any(|check| check.status == ManagementHealthStatus::Pass)
    {
        ManagementHealthStatus::Pass
    } else if checks
        .iter()
        .any(|check| check.status == ManagementHealthStatus::Unknown)
    {
        ManagementHealthStatus::Unknown
    } else {
        ManagementHealthStatus::Disabled
    }
}
