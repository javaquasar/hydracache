use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

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
}

impl Default for NamespacePersistenceSettings {
    fn default() -> Self {
        Self::ram_only()
    }
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
