use std::collections::BTreeMap;
use std::fmt;
#[cfg(feature = "durable-value-store")]
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::cluster::ClusterEpoch;
#[cfg(feature = "durable-value-store")]
use crate::grid::durable_store::DurableValueStore;
use crate::grid::elasticity::RegionId;
use crate::grid::hardening::{ReplicatedValueRecord, ReplicatedValueStore, ValueStoreError};
use crate::grid::persistence_policy::{
    PersistencePolicy, PersistencePolicyError, PersistenceRegionPlacement,
};
use crate::grid::EffectiveReplicationMap;

/// Recovery strictness for persistent namespaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryMode {
    /// Any validation/load timeout or corruption refuses node start.
    FullRecoveryOnly,
    /// Best-effort recovery reports partial state and leaves repair to later phases.
    PartialAllowed,
}

/// Full-cluster-restart recovery policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPolicy {
    /// Recovery strictness.
    pub mode: RecoveryMode,
    /// Timeout budget for store/manifest validation.
    pub validation_timeout: Duration,
    /// Timeout budget for loading namespace records.
    pub data_load_timeout: Duration,
    /// Whether stale records may be compacted by an engine that supports deletion.
    pub auto_remove_stale_data: bool,
}

impl RecoveryPolicy {
    /// Create a strict recovery policy with non-zero default timeout budgets.
    pub fn full_recovery_only() -> Self {
        Self {
            mode: RecoveryMode::FullRecoveryOnly,
            validation_timeout: Duration::from_secs(30),
            data_load_timeout: Duration::from_secs(30),
            auto_remove_stale_data: false,
        }
    }

    /// Create a partial recovery policy with non-zero default timeout budgets.
    pub fn partial_allowed() -> Self {
        Self {
            mode: RecoveryMode::PartialAllowed,
            validation_timeout: Duration::from_secs(30),
            data_load_timeout: Duration::from_secs(30),
            auto_remove_stale_data: false,
        }
    }

    /// Override the validation timeout.
    pub fn with_validation_timeout(mut self, timeout: Duration) -> Self {
        self.validation_timeout = timeout;
        self
    }

    /// Override the data-load timeout.
    pub fn with_data_load_timeout(mut self, timeout: Duration) -> Self {
        self.data_load_timeout = timeout;
        self
    }

    /// Enable/disable stale durable-data removal for engines that support compaction.
    pub fn with_auto_remove_stale_data(mut self, auto_remove: bool) -> Self {
        self.auto_remove_stale_data = auto_remove;
        self
    }
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self::full_recovery_only()
    }
}

/// Namespace recovery input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryNamespace {
    /// Namespace to recover.
    pub namespace: String,
    /// Local placement used by the persistence policy resolver.
    pub placement: PersistenceRegionPlacement,
    /// Replication map used to scan owned records from the value store.
    pub replication_map: EffectiveReplicationMap,
    /// Optional physical-key prefix for this namespace.
    pub key_prefix: Option<String>,
}

impl RecoveryNamespace {
    /// Create a recovery request for one namespace.
    pub fn new(
        namespace: impl Into<String>,
        placement: PersistenceRegionPlacement,
        replication_map: EffectiveReplicationMap,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            placement,
            replication_map,
            key_prefix: None,
        }
    }

    /// Restrict recovered records to a physical-key prefix.
    pub fn with_key_prefix(mut self, key_prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(key_prefix.into());
        self
    }
}

/// Recovery result for one namespace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredNamespace {
    /// Namespace name.
    pub namespace: String,
    /// Whether the namespace was persistent on this node.
    pub persistent: bool,
    /// Records admitted after epoch fencing.
    pub records: BTreeMap<String, ReplicatedValueRecord>,
    /// Stale keys that were fenced and not served.
    pub stale_keys: Vec<String>,
    /// Whether this namespace was only partially recovered.
    pub partial: bool,
}

/// Aggregate recovery report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryReport {
    /// Per-namespace reports.
    pub namespaces: BTreeMap<String, RecoveredNamespace>,
    /// Records loaded and admitted.
    pub recovered_record_total: u64,
    /// Stale records fenced by authority epoch.
    pub stale_fenced_total: u64,
    /// RAM-only namespaces deliberately skipped.
    pub non_persistent_skipped_total: u64,
    /// Partial recovery events.
    pub partial_recovery_total: u64,
    /// Timeout events.
    pub timeout_total: u64,
    /// Whether stale data removal was requested.
    pub auto_remove_stale_data: bool,
}

impl RecoveryReport {
    /// Return a recovered record by namespace/key.
    pub fn record(&self, namespace: &str, key: &str) -> Option<&ReplicatedValueRecord> {
        self.namespaces.get(namespace)?.records.get(key)
    }

    /// Return whether a namespace was recovered as persistent.
    pub fn namespace_persistent(&self, namespace: &str) -> bool {
        self.namespaces
            .get(namespace)
            .map(|report| report.persistent)
            .unwrap_or(false)
    }
}

/// Recover persistent namespaces from an existing replicated value store.
pub fn recover_namespaces<S>(
    store: &S,
    policy: &PersistencePolicy,
    local_region: &RegionId,
    authority_epoch: ClusterEpoch,
    recovery_policy: &RecoveryPolicy,
    namespaces: impl IntoIterator<Item = RecoveryNamespace>,
) -> Result<RecoveryReport, RecoveryError>
where
    S: ReplicatedValueStore,
{
    if recovery_policy.validation_timeout.is_zero() {
        return recovery_timeout_or_partial(recovery_policy, RecoveryReport::default());
    }

    let mut report = RecoveryReport {
        auto_remove_stale_data: recovery_policy.auto_remove_stale_data,
        ..RecoveryReport::default()
    };
    for request in namespaces {
        let resolved = policy
            .resolve_for_region(&request.namespace, local_region, &request.placement)
            .map_err(RecoveryError::policy)?;
        if !resolved.persists() {
            report.non_persistent_skipped_total =
                report.non_persistent_skipped_total.saturating_add(1);
            report.namespaces.insert(
                request.namespace.clone(),
                RecoveredNamespace {
                    namespace: request.namespace,
                    persistent: false,
                    ..RecoveredNamespace::default()
                },
            );
            continue;
        }

        let scanned = store
            .scan_owned(&request.replication_map)
            .map_err(RecoveryError::store)?;
        if recovery_policy.data_load_timeout.is_zero() && !scanned.is_empty() {
            report.timeout_total = report.timeout_total.saturating_add(1);
            return recovery_timeout_or_partial(recovery_policy, report);
        }

        let mut namespace_report = RecoveredNamespace {
            namespace: request.namespace.clone(),
            persistent: true,
            ..RecoveredNamespace::default()
        };
        for (key, record) in scanned {
            if let Some(prefix) = &request.key_prefix {
                if !key.starts_with(prefix) {
                    continue;
                }
            }
            if record.epoch < authority_epoch {
                namespace_report.stale_keys.push(key);
                report.stale_fenced_total = report.stale_fenced_total.saturating_add(1);
                continue;
            }
            namespace_report.records.insert(key, record);
            report.recovered_record_total = report.recovered_record_total.saturating_add(1);
        }
        report
            .namespaces
            .insert(request.namespace.clone(), namespace_report);
    }
    Ok(report)
}

#[cfg(feature = "durable-value-store")]
/// Open a durable value store as part of recovery, preserving fail-loud store errors.
pub fn open_durable_value_store_for_recovery(
    path: impl AsRef<Path>,
    max_total_bytes: u64,
) -> Result<DurableValueStore, RecoveryError> {
    DurableValueStore::open_with_budget(path, max_total_bytes).map_err(RecoveryError::store)
}

/// Recovery error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryError {
    kind: RecoveryErrorKind,
    message: String,
}

impl RecoveryError {
    fn policy(error: PersistencePolicyError) -> Self {
        Self {
            kind: RecoveryErrorKind::Policy,
            message: error.to_string(),
        }
    }

    fn store(error: ValueStoreError) -> Self {
        Self {
            kind: RecoveryErrorKind::Store,
            message: error.to_string(),
        }
    }

    fn timeout(message: impl Into<String>) -> Self {
        Self {
            kind: RecoveryErrorKind::Timeout,
            message: message.into(),
        }
    }

    /// Return the stable error kind.
    pub fn kind(&self) -> RecoveryErrorKind {
        self.kind
    }
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RecoveryError {}

/// Stable recovery error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryErrorKind {
    /// Persistence policy validation failed.
    Policy,
    /// Value-store open/scan/read validation failed.
    Store,
    /// Recovery timed out before safe serving.
    Timeout,
}

fn recovery_timeout_or_partial(
    policy: &RecoveryPolicy,
    mut report: RecoveryReport,
) -> Result<RecoveryReport, RecoveryError> {
    match policy.mode {
        RecoveryMode::FullRecoveryOnly => Err(RecoveryError::timeout(
            "full recovery timed out before persistent namespaces were safely loaded",
        )),
        RecoveryMode::PartialAllowed => {
            report.partial_recovery_total = report.partial_recovery_total.saturating_add(1);
            for namespace in report.namespaces.values_mut() {
                namespace.partial = true;
            }
            Ok(report)
        }
    }
}
