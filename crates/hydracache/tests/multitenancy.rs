use hydracache::{
    AdmissionRejection, ConsumerIsolation, ConsumerIsolationConfig, NamespaceQuota, Tenant,
    TenantRoster,
};
use proptest::prelude::*;

fn roster() -> TenantRoster {
    TenantRoster::new(vec![
        Tenant::new("tenant-a")
            .unwrap()
            .allow_client("client-a")
            .namespace("users", NamespaceQuota::new(8, 2))
            .namespace("orders", NamespaceQuota::new(32, 4))
            .rate_limit_per_window(100)
            .fair_share_per_window(100)
            .max_subscriptions(1),
        Tenant::new("tenant-b")
            .unwrap()
            .allow_client("client-b")
            .namespace("users", NamespaceQuota::new(32, 4))
            .rate_limit_per_window(100)
            .fair_share_per_window(100)
            .max_subscriptions(2),
    ])
    .unwrap()
}

fn isolation() -> ConsumerIsolation {
    ConsumerIsolation::new(roster(), ConsumerIsolationConfig::default())
}

#[test]
fn multitenancy_over_quota_put_is_rejected_not_silently_evicting_others() {
    let mut isolation = isolation();
    isolation
        .admit_put("client-b", "users", "user:7", 4)
        .expect("tenant b put");
    isolation
        .admit_put("client-a", "users", "user:1", 4)
        .expect("tenant a first put");

    let rejection = isolation
        .admit_put("client-a", "users", "user:2", 5)
        .expect_err("tenant a quota should reject");
    assert!(matches!(rejection, AdmissionRejection::RejectQuota { .. }));
    assert!(rejection.retryable());

    assert!(isolation.contains_entry("tenant-b", "users", "user:7"));
    assert!(isolation.contains_entry("tenant-a", "users", "user:1"));
    assert!(!isolation.contains_entry("tenant-a", "users", "user:2"));
}

#[test]
fn multitenancy_tenant_eviction_is_namespace_scoped() {
    let mut isolation = isolation();
    isolation
        .admit_put("client-a", "users", "user:1", 4)
        .expect("users put");
    isolation
        .admit_put("client-a", "orders", "order:1", 4)
        .expect("orders put");
    isolation
        .admit_put("client-b", "users", "user:9", 4)
        .expect("tenant b put");

    assert_eq!(
        isolation
            .evict_namespace("client-a", "users")
            .expect("evict namespace"),
        1
    );
    assert!(!isolation.contains_entry("tenant-a", "users", "user:1"));
    assert!(isolation.contains_entry("tenant-a", "orders", "order:1"));
    assert!(isolation.contains_entry("tenant-b", "users", "user:9"));
}

#[test]
fn multitenancy_rate_limit_returns_retryable_backpressure() {
    let roster = TenantRoster::new(vec![Tenant::new("tenant-a")
        .unwrap()
        .allow_client("client-a")
        .namespace("users", NamespaceQuota::new(32, 4))
        .rate_limit_per_window(1)])
    .unwrap();
    let mut isolation = ConsumerIsolation::new(roster, ConsumerIsolationConfig::default());

    isolation.admit_request("client-a").expect("first request");
    let rejection = isolation
        .admit_request("client-a")
        .expect_err("second request should be rate limited");
    assert!(matches!(rejection, AdmissionRejection::RejectRate { .. }));
    assert!(rejection.retryable());
    assert!(rejection.retry_after().is_some());
}

proptest! {
    #[test]
    fn multitenancy_fair_share_prevents_one_tenant_starving_replication(
        requests in prop::collection::vec(0usize..2, 1..64)
    ) {
        let roster = TenantRoster::new(vec![
            Tenant::new("tenant-a").unwrap()
                .allow_client("client-a")
                .namespace("users", NamespaceQuota::new(1024, 1024))
                .fair_share_per_window(5),
            Tenant::new("tenant-b").unwrap()
                .allow_client("client-b")
                .namespace("users", NamespaceQuota::new(1024, 1024))
                .fair_share_per_window(5),
        ]).unwrap();
        let mut isolation = ConsumerIsolation::new(roster, ConsumerIsolationConfig::default());
        let mut admitted = [0u64, 0u64];

        for tenant in requests {
            let client = if tenant == 0 { "client-a" } else { "client-b" };
            if isolation.admit_request(client).is_ok() {
                admitted[tenant] += 1;
            }
        }

        prop_assert!(admitted[0] <= 5);
        prop_assert!(admitted[1] <= 5);
    }
}

#[test]
fn multitenancy_tenant_resolved_from_identity() {
    let mut isolation = isolation();
    let tenant = isolation
        .admit_request("client-a")
        .expect("known client resolves");
    assert_eq!(tenant.as_str(), "tenant-a");
    assert_eq!(
        isolation.admit_request("unknown-client"),
        Err(AdmissionRejection::UnknownTenant)
    );
}

#[test]
fn multitenancy_oversized_payload_rejected_before_cache_mutation() {
    let mut isolation = ConsumerIsolation::new(
        roster(),
        ConsumerIsolationConfig {
            max_value_bytes: 4,
            ..ConsumerIsolationConfig::default()
        },
    );

    let rejection = isolation
        .admit_put("client-a", "users", "user:too-large", 5)
        .expect_err("oversized value should reject");
    assert!(matches!(
        rejection,
        AdmissionRejection::GlobalLimit {
            reason: "max_value_bytes"
        }
    ));
    assert!(!isolation.contains_entry("tenant-a", "users", "user:too-large"));
}

#[test]
fn multitenancy_batch_cannot_bypass_namespace_quota() {
    let mut isolation = isolation();
    let batch = vec![("user:1".to_owned(), 4), ("user:2".to_owned(), 5)];

    let rejection = isolation
        .admit_batch_put("client-a", "users", &batch)
        .expect_err("batch should be checked before mutation");
    assert!(matches!(rejection, AdmissionRejection::RejectQuota { .. }));
    assert!(!isolation.contains_entry("tenant-a", "users", "user:1"));
    assert!(!isolation.contains_entry("tenant-a", "users", "user:2"));
}

#[test]
fn multitenancy_conditional_noop_does_not_reserve_quota() {
    let mut isolation = isolation();
    isolation
        .admit_put("client-a", "users", "lock", 4)
        .expect("initial lock accounting");

    assert!(!isolation
        .admit_put_if_committed("client-a", "users", "lock", 9, false, || {
            panic!("lost condition must not invoke the commit callback")
        })
        .expect("failed condition remains an admitted no-op"));
    isolation
        .admit_put("client-a", "users", "other", 4)
        .expect("failed condition must not consume the remaining quota");

    let metrics = isolation.metrics_snapshot_for_tenant(
        &hydracache::TenantId::new("tenant-a").expect("valid tenant id"),
    );
    let metrics = metrics.expect("known tenant metrics");
    assert_eq!(metrics.tenant_bytes["tenant-a"], 8);
    assert_eq!(metrics.tenant_entries["tenant-a"], 2);
}

#[test]
fn multitenancy_rejected_batch_callback_leaves_accounting_unchanged() {
    let mut isolation = isolation();
    let entries = vec![("user:1".to_owned(), 4), ("user:2".to_owned(), 4)];

    assert!(!isolation
        .admit_batch_put_if_committed("client-a", "users", &entries, || false)
        .expect("prevalidated batch may still abort before commit"));
    assert!(!isolation.contains_entry("tenant-a", "users", "user:1"));
    assert!(!isolation.contains_entry("tenant-a", "users", "user:2"));
    isolation
        .admit_batch_put("client-a", "users", &entries)
        .expect("aborted batch must leave the full quota available");
}

#[test]
fn multitenancy_duplicate_batch_keys_account_only_the_last_value() {
    let roster = TenantRoster::new(vec![Tenant::new("tenant-a")
        .unwrap()
        .allow_client("client-a")
        .namespace("users", NamespaceQuota::new(6, 2))])
    .unwrap();
    let mut isolation = ConsumerIsolation::new(roster, ConsumerIsolationConfig::default());

    isolation
        .admit_batch_put(
            "client-a",
            "users",
            &[("same".to_owned(), 4), ("same".to_owned(), 2)],
        )
        .expect("duplicate batch uses last-write accounting");
    isolation
        .admit_put("client-a", "users", "other", 4)
        .expect("only the final two-byte value is charged");

    let metrics = isolation
        .metrics_snapshot_for_tenant(&hydracache::TenantId::new("tenant-a").unwrap())
        .unwrap();
    assert_eq!(metrics.tenant_bytes["tenant-a"], 6);
    assert_eq!(metrics.tenant_entries["tenant-a"], 2);
}

#[test]
fn multitenancy_remove_entry_releases_bytes_and_entry_quota() {
    let mut isolation = isolation();
    isolation
        .admit_put("client-a", "users", "user:1", 8)
        .expect("fill tenant quota");
    assert!(isolation
        .remove_entry("client-a", "users", "user:1")
        .expect("remove existing accounting"));
    assert!(!isolation
        .remove_entry("client-a", "users", "user:1")
        .expect("removal is idempotent"));
    isolation
        .admit_put("client-a", "users", "user:2", 8)
        .expect("released quota can be reused");
}

#[test]
fn multitenancy_subscription_flood_is_rate_limited_per_tenant() {
    let mut isolation = isolation();
    isolation
        .begin_subscription("client-a")
        .expect("first subscription");
    let rejection = isolation
        .begin_subscription("client-a")
        .expect_err("second subscription should reject");
    assert!(matches!(rejection, AdmissionRejection::RejectRate { .. }));
    assert!(rejection.retryable());
}

proptest! {
    #[test]
    fn multitenancy_hot_key_hammering_does_not_starve_other_tenants(
        hammer_count in 0usize..64
    ) {
        let roster = TenantRoster::new(vec![
            Tenant::new("tenant-a").unwrap()
                .allow_client("client-a")
                .namespace("users", NamespaceQuota::new(1024, 1024))
                .fair_share_per_window(3),
            Tenant::new("tenant-b").unwrap()
                .allow_client("client-b")
                .namespace("users", NamespaceQuota::new(1024, 1024))
                .fair_share_per_window(3),
        ]).unwrap();
        let mut isolation = ConsumerIsolation::new(roster, ConsumerIsolationConfig::default());

        for index in 0..hammer_count {
            let _ = isolation.admit_put("client-a", "users", "hot-key", (index % 2 + 1) as u64);
        }

        prop_assert!(isolation.admit_put("client-b", "users", "own-key", 1).is_ok());
    }
}

#[test]
fn multitenancy_unknown_tenant_never_creates_metric_label() {
    let mut isolation = isolation();
    assert_eq!(
        isolation.admit_request("unknown-client"),
        Err(AdmissionRejection::UnknownTenant)
    );
    let metrics = isolation.metrics_snapshot();
    assert!(metrics.tenant_bytes.is_empty());
    assert!(metrics.tenant_entries.is_empty());
    assert!(metrics.tenant_admission_rejected_total.is_empty());
}

#[test]
fn multitenancy_metrics_snapshot_reports_every_roster_limit_and_usage() {
    let mut isolation = isolation();
    isolation
        .admit_put("client-a", "users", "user:1", 4)
        .unwrap();
    isolation.admit_request("client-a").unwrap();
    isolation.begin_subscription("client-a").unwrap();
    let _ = isolation.admit_put("client-a", "users", "user:2", 8);
    isolation.admit_request("client-b").unwrap();

    let metrics = isolation.metrics_snapshot();
    assert_eq!(metrics.tenant_bytes["tenant-a"], 4);
    assert_eq!(metrics.tenant_entries["tenant-a"], 1);
    assert_eq!(metrics.tenant_namespace_bytes["tenant-a"]["users"], 4);
    assert_eq!(metrics.tenant_namespace_entries["tenant-a"]["users"], 1);
    assert_eq!(metrics.tenant_namespace_quota_bytes["tenant-a"]["users"], 8);
    assert_eq!(
        metrics.tenant_namespace_quota_entries["tenant-a"]["users"],
        2
    );
    assert!(metrics.tenant_request_count["tenant-a"] >= 1);
    assert_eq!(metrics.tenant_rate_limit_per_window["tenant-a"], 100);
    assert!(metrics.tenant_fair_share_count["tenant-a"] >= 1);
    assert_eq!(metrics.tenant_fair_share_per_window["tenant-a"], 100);
    assert_eq!(metrics.tenant_subscriptions["tenant-a"], 1);
    assert_eq!(metrics.tenant_max_subscriptions["tenant-a"], 1);
    assert!(metrics.tenant_admission_rejected_total["tenant-a"] >= 1);
    assert_eq!(metrics.tenant_bytes["tenant-b"], 0);
}
