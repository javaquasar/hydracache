use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::grid::elasticity::RegionId;

/// Namespace persistence durability mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceDurability {
    /// Flush durable storage before acknowledging the write.
    Sync,
    /// Write behind with a bounded lag/queue budget.
    AsyncBounded {
        /// Maximum admitted pending writes before backpressure/fail-loud.
        max_lag: usize,
    },
}

impl Default for PersistenceDurability {
    fn default() -> Self {
        Self::AsyncBounded { max_lag: 1024 }
    }
}

/// Persistence-time eviction intent for a namespace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceEviction {
    /// Keep records until explicit tombstone/compaction policy removes them.
    #[default]
    None,
    /// Allow least-recently-used eviction under durable byte pressure.
    Lru,
}

/// In-memory representation preference mirrored from Hazelcast-style configs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceInMemoryFormat {
    /// Store values in their object/runtime form in RAM.
    Object,
    /// Store sealed/binary bytes in RAM.
    #[default]
    Binary,
}

/// Region selection for persistent namespaces.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionSelector {
    /// Persist in every region where the namespace is placed.
    #[default]
    All,
    /// Persist only in the listed regions.
    Only(BTreeSet<RegionId>),
    /// Persist only in the namespace home region.
    HomeRegionOnly,
}

impl RegionSelector {
    /// Select an explicit bounded set of regions.
    pub fn only(regions: impl IntoIterator<Item = RegionId>) -> Self {
        Self::Only(regions.into_iter().collect())
    }

    /// Return whether `local_region` should persist under `placement`.
    pub fn selects(&self, local_region: &RegionId, placement: &PersistenceRegionPlacement) -> bool {
        match self {
            Self::All => placement.contains(local_region),
            Self::Only(regions) => regions.contains(local_region),
            Self::HomeRegionOnly => placement.home() == local_region,
        }
    }

    /// Validate that all explicitly selected regions are in placement.
    pub fn validate_placement(
        &self,
        pattern: &PersistenceMatcher,
        placement: &PersistenceRegionPlacement,
    ) -> Result<(), PersistencePolicyError> {
        let missing = match self {
            Self::All | Self::HomeRegionOnly => Vec::new(),
            Self::Only(regions) => regions
                .iter()
                .filter(|region| !placement.contains(region))
                .cloned()
                .collect::<Vec<_>>(),
        };
        if missing.is_empty() {
            return Ok(());
        }
        let missing = missing
            .iter()
            .map(RegionId::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        Err(PersistencePolicyError::new(format!(
            "persistence rule '{pattern}' selects region(s) outside placement: {missing}"
        )))
    }
}

/// Placement regions for a namespace used by persistence-region validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistenceRegionPlacement {
    home: RegionId,
    replicated_regions: BTreeSet<RegionId>,
}

impl PersistenceRegionPlacement {
    /// Create placement with a home region and replicated regions.
    pub fn new(
        home: impl Into<RegionId>,
        replicated_regions: impl IntoIterator<Item = RegionId>,
    ) -> Self {
        let home = home.into();
        let mut replicated_regions = replicated_regions.into_iter().collect::<BTreeSet<_>>();
        replicated_regions.insert(home.clone());
        Self {
            home,
            replicated_regions,
        }
    }

    /// Create single-home placement.
    pub fn home_region_only(home: impl Into<RegionId>) -> Self {
        Self::new(home, [])
    }

    /// Create active-active placement for home plus peers.
    pub fn active_active(
        home: impl Into<RegionId>,
        peers: impl IntoIterator<Item = RegionId>,
    ) -> Self {
        Self::new(home, peers)
    }

    /// Return the authoritative home region.
    pub fn home(&self) -> &RegionId {
        &self.home
    }

    /// Return whether the namespace is placed in `region`.
    pub fn contains(&self, region: &RegionId) -> bool {
        self.replicated_regions.contains(region)
    }

    /// Return all placed regions.
    pub fn regions(&self) -> &BTreeSet<RegionId> {
        &self.replicated_regions
    }
}

/// Concrete rule applied after namespace pattern matching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespacePersistenceSettings {
    /// Whether matching namespaces persist.
    pub persist: bool,
    /// Durability level for persistent namespaces.
    pub durability: PersistenceDurability,
    /// Optional scheduled snapshot interval.
    pub snapshot_interval: Option<Duration>,
    /// Durable eviction policy.
    pub eviction: PersistenceEviction,
    /// Requested backup count for persisted records.
    pub backup_count: usize,
    /// RAM representation preference.
    pub in_memory_format: PersistenceInMemoryFormat,
    /// Regions where this namespace persists when `persist` is true.
    pub persist_in_regions: RegionSelector,
    /// Durable maintenance cadence and bounded-cycle knobs.
    pub maintenance: PersistenceMaintenance,
}

impl NamespacePersistenceSettings {
    /// Create RAM-only settings.
    pub fn ram_only() -> Self {
        Self {
            persist: false,
            durability: PersistenceDurability::default(),
            snapshot_interval: None,
            eviction: PersistenceEviction::None,
            backup_count: 0,
            in_memory_format: PersistenceInMemoryFormat::Binary,
            persist_in_regions: RegionSelector::All,
            maintenance: PersistenceMaintenance::default(),
        }
    }

    /// Create persistent settings with default bounded async durability.
    pub fn persistent() -> Self {
        Self {
            persist: true,
            durability: PersistenceDurability::default(),
            snapshot_interval: None,
            eviction: PersistenceEviction::None,
            backup_count: 0,
            in_memory_format: PersistenceInMemoryFormat::Binary,
            persist_in_regions: RegionSelector::All,
            maintenance: PersistenceMaintenance::default(),
        }
    }

    /// Set durability.
    pub fn with_durability(mut self, durability: PersistenceDurability) -> Self {
        self.durability = durability;
        self
    }

    /// Set snapshot interval.
    pub fn with_snapshot_interval(mut self, interval: Duration) -> Self {
        self.snapshot_interval = Some(interval);
        self
    }

    /// Set region selector.
    pub fn with_region_selector(mut self, selector: RegionSelector) -> Self {
        self.persist_in_regions = selector;
        self
    }

    /// Set durable maintenance settings.
    pub fn with_maintenance(mut self, maintenance: PersistenceMaintenance) -> Self {
        self.maintenance = maintenance;
        self
    }
}

impl Default for NamespacePersistenceSettings {
    fn default() -> Self {
        Self::ram_only()
    }
}

/// Durable maintenance settings resolved from persistence config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistenceMaintenance {
    /// Optional scheduled tombstone GC interval.
    pub tombstone_gc_interval: Option<Duration>,
    /// Optional scheduled backend compaction interval.
    pub compaction_interval: Option<Duration>,
    /// Maximum records scanned per GC cycle.
    pub gc_records_per_cycle: usize,
}

impl PersistenceMaintenance {
    /// Create normalized maintenance settings.
    pub fn new(
        tombstone_gc_interval: Option<Duration>,
        compaction_interval: Option<Duration>,
        gc_records_per_cycle: usize,
    ) -> Self {
        Self {
            tombstone_gc_interval: normalize_interval(tombstone_gc_interval),
            compaction_interval: normalize_interval(compaction_interval),
            gc_records_per_cycle: gc_records_per_cycle.max(1),
        }
    }
}

impl Default for PersistenceMaintenance {
    fn default() -> Self {
        Self::new(None, None, 128)
    }
}

fn normalize_interval(interval: Option<Duration>) -> Option<Duration> {
    interval.map(|duration| {
        if duration.is_zero() {
            Duration::from_secs(1)
        } else {
            duration
        }
    })
}

/// A Hazelcast-style namespace pattern.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PersistenceMatcher {
    /// Exact namespace id, e.g. `cache.jwt.pem`.
    Exact(String),
    /// Prefix wildcard, e.g. `cache.*` stores prefix `cache.`.
    Prefix(String),
    /// Explicit default fallback.
    Default,
}

impl PersistenceMatcher {
    /// Parse `default`, exact, or `prefix.*` matcher syntax.
    pub fn parse(pattern: impl Into<String>) -> Result<Self, PersistencePolicyError> {
        let pattern = pattern.into();
        if pattern == "default" {
            return Ok(Self::Default);
        }
        if pattern.is_empty() {
            return Err(PersistencePolicyError::new(
                "persistence pattern must not be empty",
            ));
        }
        if let Some(prefix) = pattern.strip_suffix(".*") {
            if prefix.is_empty() {
                return Err(PersistencePolicyError::new(
                    "wildcard persistence pattern must have a prefix",
                ));
            }
            return Ok(Self::Prefix(format!("{prefix}.")));
        }
        if pattern.contains('*') {
            return Err(PersistencePolicyError::new(format!(
                "unsupported persistence wildcard pattern '{pattern}'"
            )));
        }
        Ok(Self::Exact(pattern))
    }

    fn matches(&self, namespace: &str) -> bool {
        match self {
            Self::Exact(exact) => exact == namespace,
            Self::Prefix(prefix) => namespace.starts_with(prefix),
            Self::Default => true,
        }
    }

    fn precedence_key(&self) -> (u8, usize) {
        match self {
            Self::Exact(exact) => (3, exact.len()),
            Self::Prefix(prefix) => (2, prefix.len()),
            Self::Default => (1, 0),
        }
    }
}

impl fmt::Display for PersistenceMatcher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(exact) => formatter.write_str(exact),
            Self::Prefix(prefix) => write!(formatter, "{prefix}*"),
            Self::Default => formatter.write_str("default"),
        }
    }
}

/// One ordered persistence rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespacePersistenceRule {
    /// Pattern used to match namespaces.
    pub matcher: PersistenceMatcher,
    /// Settings applied by this rule.
    pub settings: NamespacePersistenceSettings,
}

impl NamespacePersistenceRule {
    /// Create a rule from a pattern string and concrete settings.
    pub fn new(
        pattern: impl Into<String>,
        settings: NamespacePersistenceSettings,
    ) -> Result<Self, PersistencePolicyError> {
        Ok(Self {
            matcher: PersistenceMatcher::parse(pattern)?,
            settings,
        })
    }

    /// Create a persistent rule.
    pub fn persistent(pattern: impl Into<String>) -> Result<Self, PersistencePolicyError> {
        Self::new(pattern, NamespacePersistenceSettings::persistent())
    }

    /// Create a RAM-only rule.
    pub fn ram_only(pattern: impl Into<String>) -> Result<Self, PersistencePolicyError> {
        Self::new(pattern, NamespacePersistenceSettings::ram_only())
    }
}

/// Resolved namespace persistence decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedPersistence {
    /// Namespace that was resolved.
    pub namespace: String,
    /// Matcher that supplied the decision, or `None` for built-in RAM-only.
    pub matched_by: Option<PersistenceMatcher>,
    /// Applied settings.
    pub settings: NamespacePersistenceSettings,
}

impl ResolvedPersistence {
    /// Return whether this namespace is persistent.
    pub fn persists(&self) -> bool {
        self.settings.persist
    }

    /// Return whether this namespace persists on `local_region`.
    pub fn persists_in_region(
        &self,
        local_region: &RegionId,
        placement: &PersistenceRegionPlacement,
    ) -> bool {
        self.settings.persist
            && self
                .settings
                .persist_in_regions
                .selects(local_region, placement)
    }
}

/// Deterministic namespace persistence policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistencePolicy {
    rules: Vec<NamespacePersistenceRule>,
    persistable_namespace_roster: BTreeSet<String>,
}

impl PersistencePolicy {
    /// Build a policy and fail loud on duplicate/conflicting matchers.
    pub fn try_new(
        rules: impl IntoIterator<Item = NamespacePersistenceRule>,
    ) -> Result<Self, PersistencePolicyError> {
        let mut by_matcher = BTreeMap::<PersistenceMatcher, NamespacePersistenceSettings>::new();
        let mut retained = Vec::new();
        for rule in rules {
            match by_matcher.get(&rule.matcher) {
                Some(existing) if existing != &rule.settings => {
                    return Err(PersistencePolicyError::new(format!(
                        "conflicting persistence rules for pattern '{}'",
                        rule.matcher
                    )));
                }
                Some(_) => continue,
                None => {
                    by_matcher.insert(rule.matcher.clone(), rule.settings.clone());
                    retained.push(rule);
                }
            }
        }
        Ok(Self {
            persistable_namespace_roster: retained
                .iter()
                .filter_map(|rule| match (&rule.matcher, rule.settings.persist) {
                    (PersistenceMatcher::Exact(namespace), true) => Some(namespace.clone()),
                    _ => None,
                })
                .collect(),
            rules: retained,
        })
    }

    /// Create an empty policy where every namespace is RAM-only.
    pub fn ram_only() -> Self {
        Self {
            rules: Vec::new(),
            persistable_namespace_roster: BTreeSet::new(),
        }
    }

    /// Resolve the effective decision for `namespace`.
    pub fn resolve(&self, namespace: &str) -> ResolvedPersistence {
        let mut best: Option<&NamespacePersistenceRule> = None;
        for rule in &self.rules {
            if !rule.matcher.matches(namespace) {
                continue;
            }
            let replace = best
                .map(|current| rule.matcher.precedence_key() > current.matcher.precedence_key())
                .unwrap_or(true);
            if replace {
                best = Some(rule);
            }
        }
        match best {
            Some(rule) => ResolvedPersistence {
                namespace: namespace.to_owned(),
                matched_by: Some(rule.matcher.clone()),
                settings: rule.settings.clone(),
            },
            None => ResolvedPersistence {
                namespace: namespace.to_owned(),
                matched_by: None,
                settings: NamespacePersistenceSettings::ram_only(),
            },
        }
    }

    /// Resolve and validate a namespace for a concrete node region.
    pub fn resolve_for_region(
        &self,
        namespace: &str,
        local_region: &RegionId,
        placement: &PersistenceRegionPlacement,
    ) -> Result<ResolvedPersistence, PersistencePolicyError> {
        self.validate_namespace_placement(namespace, placement)?;
        let mut resolved = self.resolve(namespace);
        if !resolved.persists_in_region(local_region, placement) {
            resolved.settings.persist = false;
        }
        Ok(resolved)
    }

    /// Validate that matching persistent rules select only placed regions.
    pub fn validate_namespace_placement(
        &self,
        namespace: &str,
        placement: &PersistenceRegionPlacement,
    ) -> Result<(), PersistencePolicyError> {
        for rule in self
            .rules
            .iter()
            .filter(|rule| rule.settings.persist && rule.matcher.matches(namespace))
        {
            rule.settings
                .persist_in_regions
                .validate_placement(&rule.matcher, placement)?;
        }
        Ok(())
    }

    /// Return exact persistent namespaces that are safe to use as bounded metric labels.
    pub fn persistable_namespace_roster(&self) -> &BTreeSet<String> {
        &self.persistable_namespace_roster
    }

    /// Fail if any rule requests persistence while a durable engine is unavailable.
    pub fn validate_engine_available(
        &self,
        durable_engine_available: bool,
    ) -> Result<(), PersistencePolicyError> {
        if durable_engine_available {
            return Ok(());
        }
        let missing = self
            .rules
            .iter()
            .filter(|rule| rule.settings.persist)
            .map(|rule| rule.matcher.to_string())
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(PersistencePolicyError::new(format!(
                "persistence requested for {} but no durable value store is available",
                missing.join(", ")
            )))
        }
    }
}

impl Default for PersistencePolicy {
    fn default() -> Self {
        Self::ram_only()
    }
}

/// Persistence policy construction/validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistencePolicyError {
    message: String,
}

impl PersistencePolicyError {
    /// Create an error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PersistencePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for PersistencePolicyError {}
