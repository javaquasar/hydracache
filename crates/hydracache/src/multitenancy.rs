use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Bounded tenant identifier from the configured roster.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TenantId(String);

impl TenantId {
    /// Create a tenant id.
    pub fn new(value: impl Into<String>) -> Result<Self, MultitenancyError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(MultitenancyError::InvalidTenant);
        }
        Ok(Self(value))
    }

    /// Return the stable tenant label.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Quota for one tenant-owned namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespaceQuota {
    /// Maximum stored bytes in this namespace.
    pub max_bytes: u64,
    /// Maximum entries in this namespace.
    pub max_entries: u64,
}

impl NamespaceQuota {
    /// Create a namespace quota.
    pub const fn new(max_bytes: u64, max_entries: u64) -> Self {
        Self {
            max_bytes,
            max_entries,
        }
    }
}

/// Configured tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tenant {
    id: TenantId,
    client_ids: BTreeSet<String>,
    namespaces: BTreeMap<String, NamespaceQuota>,
    rate_limit_per_window: u64,
    fair_share_per_window: u64,
    max_subscriptions: u64,
}

impl Tenant {
    /// Create a tenant with conservative defaults.
    pub fn new(id: impl Into<String>) -> Result<Self, MultitenancyError> {
        Ok(Self {
            id: TenantId::new(id)?,
            client_ids: BTreeSet::new(),
            namespaces: BTreeMap::new(),
            rate_limit_per_window: u64::MAX,
            fair_share_per_window: u64::MAX,
            max_subscriptions: u64::MAX,
        })
    }

    /// Return tenant id.
    pub fn id(&self) -> &TenantId {
        &self.id
    }

    /// Allow a client identity to resolve to this tenant.
    pub fn allow_client(mut self, client_id: impl Into<String>) -> Self {
        self.client_ids.insert(client_id.into());
        self
    }

    /// Add a namespace quota.
    pub fn namespace(mut self, namespace: impl Into<String>, quota: NamespaceQuota) -> Self {
        self.namespaces.insert(namespace.into(), quota);
        self
    }

    /// Set the per-window rate limit.
    pub fn rate_limit_per_window(mut self, limit: u64) -> Self {
        self.rate_limit_per_window = limit;
        self
    }

    /// Set the per-window fair-share limit.
    pub fn fair_share_per_window(mut self, limit: u64) -> Self {
        self.fair_share_per_window = limit;
        self
    }

    /// Set maximum active subscriptions.
    pub fn max_subscriptions(mut self, limit: u64) -> Self {
        self.max_subscriptions = limit;
        self
    }

    fn quota(&self, namespace: &str) -> Option<NamespaceQuota> {
        self.namespaces.get(namespace).copied()
    }
}

/// Configured tenant roster. Unknown tenants are refused before metrics labels.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantRoster {
    tenants: BTreeMap<TenantId, Tenant>,
    client_to_tenant: BTreeMap<String, TenantId>,
}

impl TenantRoster {
    /// Build a bounded roster.
    pub fn new(tenants: Vec<Tenant>) -> Result<Self, MultitenancyError> {
        let mut roster = Self::default();
        for tenant in tenants {
            if tenant.client_ids.is_empty() {
                return Err(MultitenancyError::TenantWithoutClients(
                    tenant.id.as_str().to_owned(),
                ));
            }
            if tenant.namespaces.is_empty() {
                return Err(MultitenancyError::TenantWithoutNamespaces(
                    tenant.id.as_str().to_owned(),
                ));
            }
            for client_id in &tenant.client_ids {
                if roster
                    .client_to_tenant
                    .insert(client_id.clone(), tenant.id.clone())
                    .is_some()
                {
                    return Err(MultitenancyError::DuplicateClient(client_id.clone()));
                }
            }
            if roster.tenants.insert(tenant.id.clone(), tenant).is_some() {
                return Err(MultitenancyError::DuplicateTenant);
            }
        }
        Ok(roster)
    }

    /// Return tenant by id.
    pub fn tenant(&self, id: &TenantId) -> Option<&Tenant> {
        self.tenants.get(id)
    }
}

/// Resolve a client identity to a bounded tenant id.
pub trait TenantResolver: Send + Sync {
    /// Resolve client id to tenant id.
    fn resolve(&self, client_id: &str) -> Option<TenantId>;
}

impl TenantResolver for TenantRoster {
    fn resolve(&self, client_id: &str) -> Option<TenantId> {
        self.client_to_tenant.get(client_id).cloned()
    }
}

/// Process-global and tenant admission limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerIsolationConfig {
    /// Process-health value limit checked before tenant quota.
    pub max_value_bytes: u64,
    /// Process-health request bytes checked before tenant quota.
    pub max_request_bytes: u64,
    /// Process-health batch item limit checked before tenant quota.
    pub max_batch_items: usize,
}

impl Default for ConsumerIsolationConfig {
    fn default() -> Self {
        Self {
            max_value_bytes: 16 * 1024 * 1024,
            max_request_bytes: 8 * 1024 * 1024,
            max_batch_items: 128,
        }
    }
}

/// Tenant isolation state.
#[derive(Debug, Clone)]
pub struct ConsumerIsolation {
    roster: TenantRoster,
    config: ConsumerIsolationConfig,
    entries: BTreeMap<(TenantId, String, String), u64>,
    usage: BTreeMap<(TenantId, String), NamespaceUsage>,
    request_counts: BTreeMap<TenantId, u64>,
    fair_share_counts: BTreeMap<TenantId, u64>,
    subscriptions: BTreeMap<TenantId, u64>,
    metric_labels: BTreeSet<TenantId>,
    rejected_total: BTreeMap<TenantId, u64>,
}

impl ConsumerIsolation {
    /// Create isolation state from a bounded roster.
    pub fn new(roster: TenantRoster, config: ConsumerIsolationConfig) -> Self {
        Self {
            roster,
            config,
            entries: BTreeMap::new(),
            usage: BTreeMap::new(),
            request_counts: BTreeMap::new(),
            fair_share_counts: BTreeMap::new(),
            subscriptions: BTreeMap::new(),
            metric_labels: BTreeSet::new(),
            rejected_total: BTreeMap::new(),
        }
    }

    /// Resolve a client id to a tenant.
    pub fn resolve_tenant(&self, client_id: &str) -> Result<TenantId, AdmissionRejection> {
        self.roster
            .resolve(client_id)
            .ok_or(AdmissionRejection::UnknownTenant)
    }

    /// Admit one hot-path request against tenant rate and fair-share limits.
    pub fn admit_request(&mut self, client_id: &str) -> Result<TenantId, AdmissionRejection> {
        let tenant_id = self.resolve_tenant(client_id)?;
        self.check_rate(&tenant_id)?;
        self.check_fair_share(&tenant_id)?;
        self.metric_labels.insert(tenant_id.clone());
        Ok(tenant_id)
    }

    /// Store one value if quota permits.
    pub fn admit_put(
        &mut self,
        client_id: &str,
        namespace: &str,
        key: &str,
        value_bytes: u64,
    ) -> Result<(), AdmissionRejection> {
        if value_bytes > self.config.max_value_bytes {
            return Err(AdmissionRejection::GlobalLimit {
                reason: "max_value_bytes",
            });
        }
        let tenant_id = self.admit_request(client_id)?;
        let tenant = self
            .roster
            .tenant(&tenant_id)
            .expect("tenant id came from roster");
        let quota =
            tenant
                .quota(namespace)
                .ok_or_else(|| AdmissionRejection::UnknownNamespace {
                    tenant: tenant_id.clone(),
                    namespace: namespace.to_owned(),
                })?;
        let entry_key = (tenant_id.clone(), namespace.to_owned(), key.to_owned());
        let old_bytes = self.entries.get(&entry_key).copied();
        let usage_key = (tenant_id.clone(), namespace.to_owned());
        let current = self.usage.get(&usage_key).copied().unwrap_or_default();
        let projected = current.project(old_bytes, value_bytes);

        if projected.bytes > quota.max_bytes || projected.entries > quota.max_entries {
            self.record_rejection(&tenant_id);
            return Err(AdmissionRejection::RejectQuota {
                tenant: tenant_id,
                namespace: namespace.to_owned(),
                retry_after: Duration::from_millis(100),
            });
        }

        self.entries.insert(entry_key, value_bytes);
        self.usage.insert(usage_key, projected);
        Ok(())
    }

    /// Atomically admit a batch of puts.
    pub fn admit_batch_put(
        &mut self,
        client_id: &str,
        namespace: &str,
        entries: &[(String, u64)],
    ) -> Result<(), AdmissionRejection> {
        if entries.len() > self.config.max_batch_items {
            return Err(AdmissionRejection::GlobalLimit {
                reason: "max_batch_items",
            });
        }
        let request_bytes = entries.iter().map(|(_, bytes)| *bytes).sum::<u64>();
        if request_bytes > self.config.max_request_bytes {
            return Err(AdmissionRejection::GlobalLimit {
                reason: "max_request_bytes",
            });
        }

        let tenant_id = self.admit_request(client_id)?;
        let tenant = self
            .roster
            .tenant(&tenant_id)
            .expect("tenant id came from roster");
        let quota =
            tenant
                .quota(namespace)
                .ok_or_else(|| AdmissionRejection::UnknownNamespace {
                    tenant: tenant_id.clone(),
                    namespace: namespace.to_owned(),
                })?;

        let usage_key = (tenant_id.clone(), namespace.to_owned());
        let mut projected = self.usage.get(&usage_key).copied().unwrap_or_default();
        for (key, value_bytes) in entries {
            if *value_bytes > self.config.max_value_bytes {
                return Err(AdmissionRejection::GlobalLimit {
                    reason: "max_value_bytes",
                });
            }
            let entry_key = (tenant_id.clone(), namespace.to_owned(), key.clone());
            projected = projected.project(self.entries.get(&entry_key).copied(), *value_bytes);
        }
        if projected.bytes > quota.max_bytes || projected.entries > quota.max_entries {
            self.record_rejection(&tenant_id);
            return Err(AdmissionRejection::RejectQuota {
                tenant: tenant_id,
                namespace: namespace.to_owned(),
                retry_after: Duration::from_millis(100),
            });
        }

        for (key, value_bytes) in entries {
            self.entries.insert(
                (tenant_id.clone(), namespace.to_owned(), key.clone()),
                *value_bytes,
            );
        }
        self.usage.insert(usage_key, projected);
        Ok(())
    }

    /// Begin a tenant-scoped subscription.
    pub fn begin_subscription(&mut self, client_id: &str) -> Result<(), AdmissionRejection> {
        let tenant_id = self.admit_request(client_id)?;
        let tenant = self
            .roster
            .tenant(&tenant_id)
            .expect("tenant id came from roster");
        let current = self
            .subscriptions
            .get(&tenant_id)
            .copied()
            .unwrap_or_default();
        if current >= tenant.max_subscriptions {
            self.record_rejection(&tenant_id);
            return Err(AdmissionRejection::RejectRate {
                tenant: tenant_id,
                retry_after: Duration::from_millis(50),
            });
        }
        self.subscriptions
            .insert(tenant_id, current.saturating_add(1));
        Ok(())
    }

    /// Evict all entries in one tenant namespace.
    pub fn evict_namespace(
        &mut self,
        client_id: &str,
        namespace: &str,
    ) -> Result<u64, AdmissionRejection> {
        let tenant_id = self.admit_request(client_id)?;
        let before = self.entries.len();
        self.entries.retain(|(entry_tenant, entry_ns, _), _| {
            entry_tenant != &tenant_id || entry_ns != namespace
        });
        let removed = before.saturating_sub(self.entries.len()) as u64;
        self.usage
            .insert((tenant_id, namespace.to_owned()), NamespaceUsage::default());
        Ok(removed)
    }

    /// Return whether an entry exists.
    pub fn contains_entry(&self, tenant: &str, namespace: &str, key: &str) -> bool {
        let Ok(tenant_id) = TenantId::new(tenant) else {
            return false;
        };
        self.entries
            .contains_key(&(tenant_id, namespace.to_owned(), key.to_owned()))
    }

    /// Snapshot bounded-label metrics.
    pub fn metrics_snapshot(&self) -> TenantMetricsSnapshot {
        let mut tenant_bytes = BTreeMap::new();
        let mut tenant_entries = BTreeMap::new();
        let mut tenant_admission_rejected_total = BTreeMap::new();

        for tenant_id in &self.metric_labels {
            let bytes = self
                .usage
                .iter()
                .filter(|((usage_tenant, _), _)| usage_tenant == tenant_id)
                .map(|(_, usage)| usage.bytes)
                .sum();
            let entries = self
                .usage
                .iter()
                .filter(|((usage_tenant, _), _)| usage_tenant == tenant_id)
                .map(|(_, usage)| usage.entries)
                .sum();
            tenant_bytes.insert(tenant_id.as_str().to_owned(), bytes);
            tenant_entries.insert(tenant_id.as_str().to_owned(), entries);
            tenant_admission_rejected_total.insert(
                tenant_id.as_str().to_owned(),
                self.rejected_total
                    .get(tenant_id)
                    .copied()
                    .unwrap_or_default(),
            );
        }

        TenantMetricsSnapshot {
            tenant_bytes,
            tenant_entries,
            tenant_admission_rejected_total,
        }
    }

    fn check_rate(&mut self, tenant_id: &TenantId) -> Result<(), AdmissionRejection> {
        let tenant = self
            .roster
            .tenant(tenant_id)
            .expect("tenant id came from roster");
        let count = self
            .request_counts
            .get(tenant_id)
            .copied()
            .unwrap_or_default();
        if count >= tenant.rate_limit_per_window {
            self.record_rejection(tenant_id);
            return Err(AdmissionRejection::RejectRate {
                tenant: tenant_id.clone(),
                retry_after: Duration::from_millis(50),
            });
        }
        self.request_counts
            .insert(tenant_id.clone(), count.saturating_add(1));
        Ok(())
    }

    fn check_fair_share(&mut self, tenant_id: &TenantId) -> Result<(), AdmissionRejection> {
        let tenant = self
            .roster
            .tenant(tenant_id)
            .expect("tenant id came from roster");
        let count = self
            .fair_share_counts
            .get(tenant_id)
            .copied()
            .unwrap_or_default();
        if count >= tenant.fair_share_per_window {
            self.record_rejection(tenant_id);
            return Err(AdmissionRejection::RejectRate {
                tenant: tenant_id.clone(),
                retry_after: Duration::from_millis(50),
            });
        }
        self.fair_share_counts
            .insert(tenant_id.clone(), count.saturating_add(1));
        Ok(())
    }

    fn record_rejection(&mut self, tenant_id: &TenantId) {
        self.metric_labels.insert(tenant_id.clone());
        *self.rejected_total.entry(tenant_id.clone()).or_insert(0) += 1;
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
struct NamespaceUsage {
    bytes: u64,
    entries: u64,
}

impl NamespaceUsage {
    fn project(self, old_bytes: Option<u64>, new_bytes: u64) -> Self {
        let bytes = self
            .bytes
            .saturating_sub(old_bytes.unwrap_or_default())
            .saturating_add(new_bytes);
        let entries = if old_bytes.is_some() {
            self.entries
        } else {
            self.entries.saturating_add(1)
        };
        Self { bytes, entries }
    }
}

/// Structured admission rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionRejection {
    /// Identity did not resolve to a configured tenant.
    UnknownTenant,
    /// Namespace is not owned by the tenant.
    UnknownNamespace {
        /// Tenant id.
        tenant: TenantId,
        /// Namespace.
        namespace: String,
    },
    /// Namespace quota rejected the write.
    RejectQuota {
        /// Tenant id.
        tenant: TenantId,
        /// Namespace.
        namespace: String,
        /// Retry-after hint.
        retry_after: Duration,
    },
    /// Rate or fair-share rejected the request.
    RejectRate {
        /// Tenant id.
        tenant: TenantId,
        /// Retry-after hint.
        retry_after: Duration,
    },
    /// Process-global guardrail rejected the request.
    GlobalLimit {
        /// Limit name.
        reason: &'static str,
    },
}

impl AdmissionRejection {
    /// Return whether this rejection is retryable backpressure.
    pub fn retryable(&self) -> bool {
        matches!(self, Self::RejectQuota { .. } | Self::RejectRate { .. })
    }

    /// Return retry-after if available.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RejectQuota { retry_after, .. } | Self::RejectRate { retry_after, .. } => {
                Some(*retry_after)
            }
            Self::UnknownTenant | Self::UnknownNamespace { .. } | Self::GlobalLimit { .. } => None,
        }
    }
}

/// Bounded tenant metrics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantMetricsSnapshot {
    /// Bytes by roster tenant.
    pub tenant_bytes: BTreeMap<String, u64>,
    /// Entries by roster tenant.
    pub tenant_entries: BTreeMap<String, u64>,
    /// Admission rejections by roster tenant.
    pub tenant_admission_rejected_total: BTreeMap<String, u64>,
}

/// Configuration errors for tenant isolation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MultitenancyError {
    /// Tenant id is empty.
    #[error("tenant id is empty")]
    InvalidTenant,
    /// Duplicate tenant id.
    #[error("duplicate tenant id")]
    DuplicateTenant,
    /// Duplicate client identity.
    #[error("duplicate client identity: {0}")]
    DuplicateClient(String),
    /// Tenant has no client identities.
    #[error("tenant has no client identities: {0}")]
    TenantWithoutClients(String),
    /// Tenant has no namespaces.
    #[error("tenant has no namespaces: {0}")]
    TenantWithoutNamespaces(String),
}
