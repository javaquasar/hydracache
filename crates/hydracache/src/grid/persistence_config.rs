use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::grid::elasticity::RegionId;
use crate::grid::persistence_policy::{
    NamespacePersistenceRule, NamespacePersistenceSettings, PersistenceDurability,
    PersistenceEviction, PersistenceInMemoryFormat, PersistenceMaintenance, PersistencePolicy,
    PersistencePolicyError, RegionSelector,
};
use crate::grid::recovery::{RecoveryMode, RecoveryPolicy};

/// Aggregate label used when a namespace is not in the bounded metric roster.
pub const OTHER_NAMESPACE_METRIC_LABEL: &str = "other";

/// Declarative persistence configuration mirroring Hazelcast-style per-map blocks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistenceConfig {
    /// Root directory for value-plane persistence.
    pub storage_dir: Option<PathBuf>,
    /// Default snapshot interval applied to persistent namespaces that omit one.
    pub snapshot_interval_default_secs: Option<u64>,
    /// Recovery behavior used when a node restarts with persistent namespaces.
    pub recovery: PersistenceRecoveryConfig,
    /// Namespace pattern -> persistence settings.
    pub namespaces: BTreeMap<String, PersistenceNamespaceConfig>,
}

impl PersistenceConfig {
    /// Return whether any configured namespace requests persistence.
    pub fn requests_persistence(&self) -> bool {
        self.namespaces.values().any(|namespace| namespace.persist)
    }

    /// Convert declarative config into the deterministic policy resolver.
    pub fn to_policy(&self) -> Result<PersistencePolicy, PersistenceConfigError> {
        let rules = self
            .namespaces
            .iter()
            .map(|(pattern, namespace)| {
                let mut settings = namespace.to_settings();
                if settings.persist && settings.snapshot_interval.is_none() {
                    settings.snapshot_interval =
                        self.snapshot_interval_default_secs.map(Duration::from_secs);
                }
                NamespacePersistenceRule::new(pattern.clone(), settings)
                    .map_err(PersistenceConfigError::policy)
            })
            .collect::<Result<Vec<_>, _>>()?;
        PersistencePolicy::try_new(rules).map_err(PersistenceConfigError::policy)
    }

    /// Convert recovery config into the runtime recovery policy.
    pub fn to_recovery_policy(&self) -> RecoveryPolicy {
        self.recovery.to_policy()
    }

    /// Validate startup for a node with this persistence config.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn validate_startup(
        &self,
        durable_engine_available: bool,
    ) -> Result<(), PersistenceConfigError> {
        let policy = self.to_policy()?;
        policy
            .validate_engine_available(durable_engine_available)
            .map_err(|error| {
                PersistenceConfigError::new(
                    PersistenceConfigErrorKind::DurableEngineUnavailable,
                    error.to_string(),
                )
            })?;
        if !self.requests_persistence() {
            return Ok(());
        }
        let Some(storage_dir) = &self.storage_dir else {
            return Err(PersistenceConfigError::new(
                PersistenceConfigErrorKind::MissingStorageDir,
                "persistence requested but no storage_dir is configured",
            ));
        };
        validate_storage_dir(storage_dir)?;
        Ok(())
    }
}

/// Recovery settings in serde-friendly seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistenceRecoveryConfig {
    /// Strict or partial restart behavior.
    pub mode: RecoveryMode,
    /// Validation timeout in seconds.
    pub validation_timeout_secs: u64,
    /// Data-load timeout in seconds.
    pub data_load_timeout_secs: u64,
    /// Whether engines may remove stale durable records during recovery.
    pub auto_remove_stale_data: bool,
}

impl PersistenceRecoveryConfig {
    /// Convert to runtime recovery policy.
    pub fn to_policy(&self) -> RecoveryPolicy {
        RecoveryPolicy {
            mode: self.mode,
            validation_timeout: Duration::from_secs(self.validation_timeout_secs),
            data_load_timeout: Duration::from_secs(self.data_load_timeout_secs),
            auto_remove_stale_data: self.auto_remove_stale_data,
        }
    }
}

impl Default for PersistenceRecoveryConfig {
    fn default() -> Self {
        Self {
            mode: RecoveryMode::FullRecoveryOnly,
            validation_timeout_secs: 30,
            data_load_timeout_secs: 30,
            auto_remove_stale_data: false,
        }
    }
}

/// Per-namespace persistence config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistenceNamespaceConfig {
    /// Whether this namespace should persist on selected regions.
    pub persist: bool,
    /// Durability mode for persistent writes.
    pub durability: PersistenceDurabilityConfig,
    /// Optional scheduled snapshot interval in seconds.
    pub snapshot_interval_secs: Option<u64>,
    /// Optional durable eviction policy.
    pub eviction: PersistenceEviction,
    /// Desired durable backup count.
    pub backup_count: usize,
    /// RAM representation preference.
    pub in_memory_format: PersistenceInMemoryFormat,
    /// Region selector for value-plane persistence.
    pub regions: PersistenceRegionSelectorConfig,
    /// Durable maintenance cadence and bounded-cycle knobs.
    pub maintenance: PersistenceMaintenanceConfig,
}

impl PersistenceNamespaceConfig {
    /// RAM-only namespace config.
    pub fn ram_only() -> Self {
        Self::default()
    }

    /// Persistent namespace config with default async-bounded durability.
    pub fn persistent() -> Self {
        Self {
            persist: true,
            ..Self::default()
        }
    }

    fn to_settings(&self) -> NamespacePersistenceSettings {
        let mut settings = if self.persist {
            NamespacePersistenceSettings::persistent()
        } else {
            NamespacePersistenceSettings::ram_only()
        };
        settings.durability = self.durability.clone().into();
        settings.snapshot_interval = self.snapshot_interval_secs.map(Duration::from_secs);
        settings.eviction = self.eviction;
        settings.backup_count = self.backup_count;
        settings.in_memory_format = self.in_memory_format;
        settings.persist_in_regions = self.regions.clone().into();
        settings.maintenance = self.maintenance.into();
        settings
    }
}

impl Default for PersistenceNamespaceConfig {
    fn default() -> Self {
        Self {
            persist: false,
            durability: PersistenceDurabilityConfig::default(),
            snapshot_interval_secs: None,
            eviction: PersistenceEviction::None,
            backup_count: 0,
            in_memory_format: PersistenceInMemoryFormat::Binary,
            regions: PersistenceRegionSelectorConfig::All,
            maintenance: PersistenceMaintenanceConfig::default(),
        }
    }
}

/// Serde-friendly durable maintenance config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistenceMaintenanceConfig {
    /// Optional scheduled tombstone GC interval in seconds.
    pub tombstone_gc_interval_secs: Option<u64>,
    /// Optional scheduled backend compaction interval in seconds.
    pub compaction_interval_secs: Option<u64>,
    /// Maximum records scanned per GC cycle.
    pub gc_records_per_cycle: usize,
}

impl From<PersistenceMaintenanceConfig> for PersistenceMaintenance {
    fn from(value: PersistenceMaintenanceConfig) -> Self {
        PersistenceMaintenance::new(
            value.tombstone_gc_interval_secs.map(Duration::from_secs),
            value.compaction_interval_secs.map(Duration::from_secs),
            value.gc_records_per_cycle,
        )
    }
}

impl From<PersistenceMaintenance> for PersistenceMaintenanceConfig {
    fn from(value: PersistenceMaintenance) -> Self {
        Self {
            tombstone_gc_interval_secs: value
                .tombstone_gc_interval
                .map(|duration| duration.as_secs().max(1)),
            compaction_interval_secs: value
                .compaction_interval
                .map(|duration| duration.as_secs().max(1)),
            gc_records_per_cycle: value.gc_records_per_cycle,
        }
    }
}

impl Default for PersistenceMaintenanceConfig {
    fn default() -> Self {
        Self {
            tombstone_gc_interval_secs: None,
            compaction_interval_secs: None,
            gc_records_per_cycle: 128,
        }
    }
}

/// Serde-friendly persistence durability config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceDurabilityConfig {
    /// Flush durable storage before ack.
    Sync,
    /// Bounded write-behind queue.
    AsyncBounded {
        /// Maximum pending writes before backpressure.
        max_lag: usize,
    },
}

impl From<PersistenceDurabilityConfig> for PersistenceDurability {
    fn from(value: PersistenceDurabilityConfig) -> Self {
        match value {
            PersistenceDurabilityConfig::Sync => Self::Sync,
            PersistenceDurabilityConfig::AsyncBounded { max_lag } => Self::AsyncBounded { max_lag },
        }
    }
}

impl From<PersistenceDurability> for PersistenceDurabilityConfig {
    fn from(value: PersistenceDurability) -> Self {
        match value {
            PersistenceDurability::Sync => Self::Sync,
            PersistenceDurability::AsyncBounded { max_lag } => Self::AsyncBounded { max_lag },
        }
    }
}

impl Default for PersistenceDurabilityConfig {
    fn default() -> Self {
        Self::AsyncBounded { max_lag: 1024 }
    }
}

/// Serde-friendly region selector.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceRegionSelectorConfig {
    /// Persist in every placed region.
    #[default]
    All,
    /// Persist only in the home region.
    HomeRegionOnly,
    /// Persist only in listed regions.
    Only(Vec<String>),
}

impl From<PersistenceRegionSelectorConfig> for RegionSelector {
    fn from(value: PersistenceRegionSelectorConfig) -> Self {
        match value {
            PersistenceRegionSelectorConfig::All => Self::All,
            PersistenceRegionSelectorConfig::HomeRegionOnly => Self::HomeRegionOnly,
            PersistenceRegionSelectorConfig::Only(regions) => {
                Self::only(regions.into_iter().map(RegionId::new))
            }
        }
    }
}

impl From<RegionSelector> for PersistenceRegionSelectorConfig {
    fn from(value: RegionSelector) -> Self {
        match value {
            RegionSelector::All => Self::All,
            RegionSelector::HomeRegionOnly => Self::HomeRegionOnly,
            RegionSelector::Only(regions) => Self::Only(
                regions
                    .into_iter()
                    .map(|region| region.as_str().to_owned())
                    .collect(),
            ),
        }
    }
}

/// Bounded namespace labels for durability metrics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamespaceMetricLabels {
    roster: BTreeSet<String>,
}

impl NamespaceMetricLabels {
    /// Build label roster from exact persistent namespaces in a policy.
    pub fn from_policy(policy: &PersistencePolicy) -> Self {
        Self {
            roster: policy.persistable_namespace_roster().clone(),
        }
    }

    /// Return a bounded metric label for `namespace`.
    pub fn label_for<'a>(&'a self, namespace: &'a str) -> Cow<'a, str> {
        if self.roster.contains(namespace) {
            Cow::Borrowed(namespace)
        } else {
            Cow::Borrowed(OTHER_NAMESPACE_METRIC_LABEL)
        }
    }

    /// Return all registered metric labels, including the aggregate `other` bucket.
    pub fn registered_labels(&self) -> BTreeSet<String> {
        let mut labels = self.roster.clone();
        labels.insert(OTHER_NAMESPACE_METRIC_LABEL.to_owned());
        labels
    }
}

/// Persistence config validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistenceConfigError {
    kind: PersistenceConfigErrorKind,
    message: String,
}

impl PersistenceConfigError {
    fn new(kind: PersistenceConfigErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn policy(error: PersistencePolicyError) -> Self {
        Self::new(PersistenceConfigErrorKind::Policy, error.to_string())
    }

    /// Return the stable error kind.
    pub fn kind(&self) -> PersistenceConfigErrorKind {
        self.kind
    }
}

impl fmt::Display for PersistenceConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PersistenceConfigError {}

/// Stable persistence config error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceConfigErrorKind {
    /// Persistence was requested with no storage directory.
    MissingStorageDir,
    /// Storage directory could not be created or written.
    StorageDirUnavailable,
    /// Persistence was requested without an available durable engine.
    DurableEngineUnavailable,
    /// Policy conversion failed.
    Policy,
}

#[cfg(not(target_arch = "wasm32"))]
fn validate_storage_dir(path: &Path) -> Result<(), PersistenceConfigError> {
    std::fs::create_dir_all(path).map_err(|error| {
        PersistenceConfigError::new(
            PersistenceConfigErrorKind::StorageDirUnavailable,
            format!(
                "persistence storage_dir '{}' is not writable: {error}",
                path.display()
            ),
        )
    })?;
    let probe = path.join(".hydracache-persistence-write-test");
    std::fs::write(&probe, b"ok").map_err(|error| {
        PersistenceConfigError::new(
            PersistenceConfigErrorKind::StorageDirUnavailable,
            format!(
                "persistence storage_dir '{}' is not writable: {error}",
                path.display()
            ),
        )
    })?;
    let _ = std::fs::remove_file(probe);
    Ok(())
}
