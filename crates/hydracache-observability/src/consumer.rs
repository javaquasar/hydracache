use hydracache::TenantMetricsSnapshot;
use serde::Serialize;

/// Tenant status JSON schema registered in `docs/COMPAT.md`.
pub const TENANT_STATUS_SCHEMA_VERSION: u32 = 1;

/// Per-namespace consumer status scoped to a single tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TenantNamespaceStatus {
    /// Namespace.
    pub namespace: String,
    /// Stored bytes for this tenant namespace.
    pub bytes: u64,
    /// Stored entries for this tenant namespace.
    pub entries: u64,
    /// Configured byte quota.
    pub max_bytes: u64,
    /// Configured entry quota.
    pub max_entries: u64,
}

/// Tenant rate/fair-share state for a modeled window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TenantRateLimitStatus {
    /// Requests admitted in the current modeled window.
    pub request_count: u64,
    /// Request limit per modeled window.
    pub rate_limit_per_window: u64,
    /// Fair-share count in the current modeled window.
    pub fair_share_count: u64,
    /// Fair-share limit per modeled window.
    pub fair_share_per_window: u64,
    /// Admission rejections observed for this tenant.
    pub admission_rejected_total: u64,
}

/// Near-cache/subscription health visible to a consumer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConsumerNearCacheStatus {
    /// Active subscription streams for this tenant/status owner.
    pub active_subscriptions: u64,
    /// Maximum configured subscription streams.
    pub max_subscriptions: u64,
    /// Near-cache repair actions observed by the SDK/client surface.
    pub repairs_total: u64,
    /// Whether the consumer-facing cache health is currently OK.
    pub healthy: bool,
}

/// Read-only status scoped to the caller's tenant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TenantStatus {
    /// Status schema version.
    pub schema_version: u32,
    /// Tenant id.
    pub tenant: String,
    /// Namespace usage and quotas for this tenant only.
    pub namespaces: Vec<TenantNamespaceStatus>,
    /// Rate/fair-share state.
    pub rate_limit: TenantRateLimitStatus,
    /// Near-cache/subscription health.
    pub near_cache: ConsumerNearCacheStatus,
}

impl TenantStatus {
    /// Build a tenant status from bounded W4 metrics.
    pub fn from_metrics(
        tenant: impl Into<String>,
        metrics: &TenantMetricsSnapshot,
        active_subscriptions: u64,
        repairs_total: u64,
    ) -> Self {
        let tenant = tenant.into();
        let namespace_bytes = metrics
            .tenant_namespace_bytes
            .get(&tenant)
            .cloned()
            .unwrap_or_default();
        let namespace_entries = metrics
            .tenant_namespace_entries
            .get(&tenant)
            .cloned()
            .unwrap_or_default();
        let quota_bytes = metrics
            .tenant_namespace_quota_bytes
            .get(&tenant)
            .cloned()
            .unwrap_or_default();
        let quota_entries = metrics
            .tenant_namespace_quota_entries
            .get(&tenant)
            .cloned()
            .unwrap_or_default();

        let mut namespaces = quota_bytes
            .keys()
            .chain(namespace_bytes.keys())
            .cloned()
            .collect::<Vec<_>>();
        namespaces.sort();
        namespaces.dedup();
        let namespaces = namespaces
            .into_iter()
            .map(|namespace| TenantNamespaceStatus {
                bytes: namespace_bytes.get(&namespace).copied().unwrap_or_default(),
                entries: namespace_entries
                    .get(&namespace)
                    .copied()
                    .unwrap_or_default(),
                max_bytes: quota_bytes.get(&namespace).copied().unwrap_or_default(),
                max_entries: quota_entries.get(&namespace).copied().unwrap_or_default(),
                namespace,
            })
            .collect();

        let max_subscriptions = metrics
            .tenant_max_subscriptions
            .get(&tenant)
            .copied()
            .unwrap_or_default();
        Self {
            schema_version: TENANT_STATUS_SCHEMA_VERSION,
            rate_limit: TenantRateLimitStatus {
                request_count: metrics
                    .tenant_request_count
                    .get(&tenant)
                    .copied()
                    .unwrap_or_default(),
                rate_limit_per_window: metrics
                    .tenant_rate_limit_per_window
                    .get(&tenant)
                    .copied()
                    .unwrap_or_default(),
                fair_share_count: metrics
                    .tenant_fair_share_count
                    .get(&tenant)
                    .copied()
                    .unwrap_or_default(),
                fair_share_per_window: metrics
                    .tenant_fair_share_per_window
                    .get(&tenant)
                    .copied()
                    .unwrap_or_default(),
                admission_rejected_total: metrics
                    .tenant_admission_rejected_total
                    .get(&tenant)
                    .copied()
                    .unwrap_or_default(),
            },
            near_cache: ConsumerNearCacheStatus {
                active_subscriptions,
                max_subscriptions,
                repairs_total,
                healthy: active_subscriptions <= max_subscriptions || max_subscriptions == 0,
            },
            namespaces,
            tenant,
        }
    }
}

/// Consumer-facing metric names.
pub fn consumer_metric_names() -> &'static [&'static str] {
    &[
        "hydracache_tenant_bytes",
        "hydracache_tenant_entries",
        "hydracache_tenant_admission_rejected_total",
        "hydracache_client_auth_rejected_total",
        "hydracache_residency_rejected_placement_total",
        "hydracache_residency_refused_crossing_total",
        "hydracache_audit_sink_failures_total",
        "hydracache_audit_mandatory_fail_closed_total",
    ]
}

/// Consumer alert metrics shipped with W6 artifacts.
pub fn consumer_alert_metric_names() -> &'static [&'static str] {
    &[
        "hydracache_tenant_admission_rejected_total",
        "hydracache_client_auth_rejected_total",
        "hydracache_residency_refused_crossing_total",
        "hydracache_audit_mandatory_fail_closed_total",
    ]
}
