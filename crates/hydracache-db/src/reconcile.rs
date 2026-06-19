use crate::{HookSchemaVersion, InvalidationOutbox, OutboxStatus, Result};

/// Policy used to decide when outbox backlog becomes drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboxLagPolicy {
    /// Maximum pending rows before reporting drift.
    pub max_pending_rows: u64,
    /// Maximum age of the oldest pending row before reporting drift.
    pub max_oldest_pending_age_ms: u64,
    /// Whether dead-lettered rows should always report drift.
    pub fail_on_dead_letters: bool,
}

impl Default for OutboxLagPolicy {
    fn default() -> Self {
        Self {
            max_pending_rows: 0,
            max_oldest_pending_age_ms: 0,
            fail_on_dead_letters: true,
        }
    }
}

/// Outbox backlog signal for reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboxLag {
    /// Raw outbox status.
    pub status: OutboxStatus,
    /// Policy used to evaluate the status.
    pub policy: OutboxLagPolicy,
}

impl OutboxLag {
    /// Evaluate raw outbox status with a policy.
    pub fn new(status: OutboxStatus, policy: OutboxLagPolicy) -> Self {
        Self { status, policy }
    }

    /// Return whether no outbox-lag drift is detected.
    pub fn is_clean(&self) -> bool {
        self.reasons().is_empty()
    }

    /// Return drift reasons contributed by outbox lag.
    pub fn reasons(&self) -> Vec<DriftReason> {
        let mut reasons = Vec::new();
        if self.status.pending > self.policy.max_pending_rows {
            reasons.push(DriftReason::OutboxPendingRows {
                pending: self.status.pending,
                max_pending_rows: self.policy.max_pending_rows,
            });
        }
        if self.status.pending > 0
            && self.status.oldest_pending_age_ms > self.policy.max_oldest_pending_age_ms
        {
            reasons.push(DriftReason::OutboxOldestPendingAge {
                oldest_pending_age_ms: self.status.oldest_pending_age_ms,
                max_oldest_pending_age_ms: self.policy.max_oldest_pending_age_ms,
            });
        }
        if self.policy.fail_on_dead_letters && self.status.dead_lettered > 0 {
            reasons.push(DriftReason::OutboxDeadLetters {
                dead_lettered: self.status.dead_lettered,
            });
        }
        reasons
    }
}

/// Hook/schema drift signal for reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookDrift {
    /// Expected hook schema version.
    pub expected: HookSchemaVersion,
    /// Installed hook schema version, if it can be read.
    pub installed: Option<HookSchemaVersion>,
}

impl HookDrift {
    /// Compare expected hook schema metadata with installed metadata.
    pub fn new(expected: HookSchemaVersion, installed: Option<HookSchemaVersion>) -> Self {
        Self {
            expected,
            installed,
        }
    }

    /// Return a missing installed-schema signal for the expected hook version.
    pub fn missing(expected: HookSchemaVersion) -> Self {
        Self::new(expected, None)
    }

    /// Return whether installed hook metadata matches the expected metadata.
    pub fn is_clean(&self) -> bool {
        self.reasons().is_empty()
    }

    /// Return drift reasons contributed by hook/schema metadata.
    pub fn reasons(&self) -> Vec<DriftReason> {
        let Some(installed) = &self.installed else {
            return vec![DriftReason::HookSchemaMissing {
                artifact: self.expected.artifact.clone(),
                expected_table: self.expected.table.clone(),
            }];
        };

        let mut reasons = Vec::new();
        if installed.artifact != self.expected.artifact {
            reasons.push(DriftReason::HookArtifactMismatch {
                expected: self.expected.artifact.clone(),
                installed: installed.artifact.clone(),
            });
        }
        if installed.version != self.expected.version {
            reasons.push(DriftReason::HookVersionMismatch {
                expected: self.expected.version,
                installed: installed.version,
            });
        }
        if installed.table != self.expected.table {
            reasons.push(DriftReason::HookTableMismatch {
                expected: self.expected.table.clone(),
                installed: installed.table.clone(),
            });
        }
        if installed.dialect != self.expected.dialect {
            reasons.push(DriftReason::HookDialectMismatch {
                expected: self.expected.dialect.to_string(),
                installed: installed.dialect.to_string(),
            });
        }
        reasons
    }
}

/// Optional CDC lag signal reserved for future releases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CdcOffsetLag {
    /// Human-readable source name.
    pub source: String,
    /// Lag in bytes/LSN units as reported by the source.
    pub lag: u64,
}

/// Optional generation-drift signal reserved for future releases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationDrift {
    /// Human-readable source name.
    pub source: String,
    /// Expected cache generation.
    pub expected: u64,
    /// Observed cache generation.
    pub observed: u64,
}

/// Reconciliation report over mandatory and extension drift signals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationReport {
    /// Mandatory outbox backlog signal.
    pub outbox_lag: OutboxLag,
    /// Mandatory hook/schema signal.
    pub hook_drift: HookDrift,
    /// Optional CDC offset lag extension.
    pub cdc_offset: Option<CdcOffsetLag>,
    /// Optional cache generation drift extension.
    pub generations: Option<GenerationDrift>,
}

impl ReconciliationReport {
    /// Build a report from raw mandatory signals.
    pub fn new(outbox_lag: OutboxLag, hook_drift: HookDrift) -> Self {
        Self {
            outbox_lag,
            hook_drift,
            cdc_offset: None,
            generations: None,
        }
    }

    /// Build a report by querying an invalidation outbox.
    pub async fn from_outbox<O>(
        outbox: &O,
        namespace: &str,
        hook_drift: HookDrift,
        policy: OutboxLagPolicy,
    ) -> Result<Self>
    where
        O: InvalidationOutbox,
    {
        let status = outbox.status(namespace).await?;
        Ok(Self::new(OutboxLag::new(status, policy), hook_drift))
    }

    /// Attach optional CDC lag.
    pub fn with_cdc_offset(mut self, cdc_offset: CdcOffsetLag) -> Self {
        self.cdc_offset = Some(cdc_offset);
        self
    }

    /// Attach optional generation drift.
    pub fn with_generations(mut self, generations: GenerationDrift) -> Self {
        self.generations = Some(generations);
        self
    }

    /// Return the aggregate reconciliation status.
    pub fn status(&self) -> DriftStatus {
        let mut reasons = Vec::new();
        reasons.extend(self.outbox_lag.reasons());
        reasons.extend(self.hook_drift.reasons());
        if let Some(cdc_offset) = &self.cdc_offset {
            reasons.push(DriftReason::CdcOffsetLag {
                source: cdc_offset.source.clone(),
                lag: cdc_offset.lag,
            });
        }
        if let Some(generation) = &self.generations {
            if generation.expected != generation.observed {
                reasons.push(DriftReason::GenerationMismatch {
                    source: generation.source.clone(),
                    expected: generation.expected,
                    observed: generation.observed,
                });
            }
        }

        if reasons.is_empty() {
            DriftStatus::Clean
        } else {
            DriftStatus::Drift(reasons)
        }
    }

    /// Return whether the aggregate status is clean.
    pub fn is_clean(&self) -> bool {
        matches!(self.status(), DriftStatus::Clean)
    }
}

/// Aggregate reconciliation status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftStatus {
    /// Mandatory and configured extension signals are clean.
    Clean,
    /// One or more drift reasons were detected.
    Drift(Vec<DriftReason>),
}

/// Reason explaining why reconciliation reports drift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftReason {
    /// Pending outbox row count exceeded the policy.
    OutboxPendingRows { pending: u64, max_pending_rows: u64 },
    /// Oldest pending outbox row exceeded the policy.
    OutboxOldestPendingAge {
        oldest_pending_age_ms: u64,
        max_oldest_pending_age_ms: u64,
    },
    /// Dead-lettered outbox rows are present.
    OutboxDeadLetters { dead_lettered: u64 },
    /// Hook schema metadata is missing.
    HookSchemaMissing {
        artifact: String,
        expected_table: String,
    },
    /// Hook artifact name differs.
    HookArtifactMismatch { expected: String, installed: String },
    /// Hook schema version differs.
    HookVersionMismatch { expected: i64, installed: i64 },
    /// Hook table differs.
    HookTableMismatch { expected: String, installed: String },
    /// Hook dialect differs.
    HookDialectMismatch { expected: String, installed: String },
    /// Optional CDC source reports lag.
    CdcOffsetLag { source: String, lag: u64 },
    /// Optional generation source reports mismatch.
    GenerationMismatch {
        source: String,
        expected: u64,
        observed: u64,
    },
}

#[cfg(feature = "sqlx-outbox")]
pub async fn sqlite_hook_drift(
    pool: &sqlx::SqlitePool,
    expected: HookSchemaVersion,
) -> Result<HookDrift> {
    use sqlx::Row;

    let row = match sqlx::query(
        "select artifact, version, table_name from hydracache_hook_schema where artifact = ?",
    )
    .bind(crate::HOOK_SCHEMA_ARTIFACT)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(error) if is_sqlite_missing_table(&error) => None,
        Err(error) => {
            return Err(hydracache::CacheError::Backend(format!(
                "sqlite hook reconciliation error: {error}"
            ))
            .into())
        }
    };

    let installed = row.map(|row| HookSchemaVersion {
        artifact: row.get("artifact"),
        version: row.get("version"),
        table: row.get("table_name"),
        dialect: crate::HookDialect::Sqlite,
    });

    Ok(HookDrift::new(expected, installed))
}

#[cfg(feature = "sqlx-outbox")]
fn is_sqlite_missing_table(error: &sqlx::Error) -> bool {
    error
        .to_string()
        .contains("no such table: hydracache_hook_schema")
}

#[cfg(test)]
mod tests {
    use crate::{HookDialect, OutboxStatus, HOOK_SCHEMA_ARTIFACT};

    use super::*;

    fn expected_hook() -> HookSchemaVersion {
        HookSchemaVersion {
            artifact: HOOK_SCHEMA_ARTIFACT.to_owned(),
            version: 1,
            table: "users".to_owned(),
            dialect: HookDialect::Sqlite,
        }
    }

    #[test]
    fn reconcile_report_status_is_clean_for_matching_signals() {
        let outbox_lag = OutboxLag::new(OutboxStatus::default(), OutboxLagPolicy::default());
        let hook = expected_hook();
        let hook_drift = HookDrift::new(hook.clone(), Some(hook));
        let report = ReconciliationReport::new(outbox_lag, hook_drift);

        assert_eq!(report.status(), DriftStatus::Clean);
        assert!(report.is_clean());
    }

    #[test]
    fn reconcile_report_status_collects_mandatory_reasons() {
        let outbox_lag = OutboxLag::new(
            OutboxStatus {
                pending: 3,
                dead_lettered: 1,
                ..OutboxStatus::default()
            },
            OutboxLagPolicy::default(),
        );
        let report = ReconciliationReport::new(outbox_lag, HookDrift::missing(expected_hook()));

        let DriftStatus::Drift(reasons) = report.status() else {
            panic!("expected drift");
        };

        assert!(reasons
            .iter()
            .any(|reason| matches!(reason, DriftReason::OutboxPendingRows { pending: 3, .. })));
        assert!(reasons
            .iter()
            .any(|reason| matches!(reason, DriftReason::OutboxDeadLetters { dead_lettered: 1 })));
        assert!(reasons
            .iter()
            .any(|reason| matches!(reason, DriftReason::HookSchemaMissing { .. })));
    }
}
