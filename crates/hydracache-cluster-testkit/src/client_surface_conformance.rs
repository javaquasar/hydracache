//! Protocol-only conformance contract for client-surface backends.
//!
//! The contract deliberately depends on wire request and response types rather
//! than any concrete transport. A backend adapter supplies construction,
//! dispatch, and a deterministic clock; the assertions can then be reused by
//! the in-memory surface, Redis compatibility, or a future distributed store.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{bail, ensure, Context, Result};
use async_trait::async_trait;
use hydracache_client_protocol::{
    BatchPutEntry, ClientErrorCode, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, CompareValueExpireMode, ConditionalPutCondition, Namespace,
    StructuredKey, TtlState, PROTOCOL_VERSION,
};
use tokio::sync::Barrier;

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Result type used by conformance factories and assertions.
pub type ConformanceResult<T> = Result<T>;

/// Verified identity supplied to a backend dispatch seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceIdentity {
    /// External consumer id.
    pub client_id: String,
    /// Claimed tenant id.
    pub tenant_id: String,
}

impl ConformanceIdentity {
    /// Construct an identity.
    pub fn new(client_id: impl Into<String>, tenant_id: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            tenant_id: tenant_id.into(),
        }
    }
}

/// One namespace quota installed for a conformance tenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceNamespace {
    /// Namespace name.
    pub name: String,
    /// Maximum live bytes.
    pub max_bytes: u64,
    /// Maximum live entries.
    pub max_entries: u64,
}

impl ConformanceNamespace {
    /// Construct a namespace quota.
    pub fn new(name: impl Into<String>, max_bytes: u64, max_entries: u64) -> Self {
        Self {
            name: name.into(),
            max_bytes,
            max_entries,
        }
    }
}

/// Tenant roster entry installed by a conformance factory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceTenant {
    /// Stable tenant id.
    pub tenant_id: String,
    /// Consumer ids authorized for the tenant.
    pub client_ids: Vec<String>,
    /// Tenant namespace quotas.
    pub namespaces: Vec<ConformanceNamespace>,
}

impl ConformanceTenant {
    /// Construct a tenant with one client and one namespace.
    pub fn single_namespace(
        tenant_id: impl Into<String>,
        client_id: impl Into<String>,
        namespace: ConformanceNamespace,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            client_ids: vec![client_id.into()],
            namespaces: vec![namespace],
        }
    }
}

/// Surface-level request limits exercised by the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConformanceLimits {
    /// Maximum bytes in one value.
    pub max_value_bytes: usize,
    /// Maximum entries in one batch.
    pub max_batch_entries: usize,
    /// Maximum aggregate stable-key and value bytes in one batch.
    pub max_batch_bytes: usize,
}

impl Default for ConformanceLimits {
    fn default() -> Self {
        Self {
            max_value_bytes: 64,
            max_batch_entries: 16,
            max_batch_bytes: 512,
        }
    }
}

/// Backend construction inputs owned by the protocol-only contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientSurfaceConformanceConfig {
    /// Surface request limits.
    pub limits: ConformanceLimits,
    /// Tenant roster and namespace quotas.
    pub tenants: Vec<ConformanceTenant>,
    /// Initial deterministic clock value.
    pub now_ms: u64,
}

impl ClientSurfaceConformanceConfig {
    /// Construct the common one-tenant setup.
    pub fn single_tenant(limits: ConformanceLimits, max_bytes: u64, max_entries: u64) -> Self {
        Self {
            limits,
            tenants: vec![ConformanceTenant::single_namespace(
                "tenant-a",
                "client-a",
                ConformanceNamespace::new("users", max_bytes, max_entries),
            )],
            now_ms: 1_000_000,
        }
    }
}

impl Default for ClientSurfaceConformanceConfig {
    fn default() -> Self {
        Self::single_tenant(ConformanceLimits::default(), 1_024, 64)
    }
}

/// Async backend controlled only through protocol requests and a test clock.
#[async_trait]
pub trait ClientSurfaceBackend: Send + Sync {
    /// Execute one verified protocol request.
    async fn execute(
        &self,
        identity: &ConformanceIdentity,
        request: ClientRequestEnvelope,
    ) -> ConformanceResult<ClientResponseEnvelope>;

    /// Set the deterministic backend clock.
    async fn freeze_time(&self, now_ms: u64) -> ConformanceResult<()>;

    /// Advance the deterministic backend clock.
    async fn advance_time(&self, millis: u64) -> ConformanceResult<()>;
}

/// Factory that installs the requested limits and tenant roster in a fresh backend.
#[async_trait]
pub trait ClientSurfaceBackendFactory: Send + Sync {
    /// Create an isolated backend instance.
    async fn create(
        &self,
        config: ClientSurfaceConformanceConfig,
    ) -> ConformanceResult<Arc<dyn ClientSurfaceBackend>>;
}

fn identity_a() -> ConformanceIdentity {
    ConformanceIdentity::new("client-a", "tenant-a")
}

fn namespace() -> Result<Namespace> {
    Namespace::new("users").context("valid conformance namespace")
}

fn key(value: &str) -> Result<StructuredKey> {
    StructuredKey::new(vec![value.to_owned()]).context("valid conformance key")
}

fn request(operation: ClientRequest) -> ClientRequestEnvelope {
    let id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    ClientRequestEnvelope::new(format!("conformance-{id}"), operation)
}

async fn fresh<F>(
    factory: &F,
    config: ClientSurfaceConformanceConfig,
) -> Result<Arc<dyn ClientSurfaceBackend>>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let now_ms = config.now_ms;
    let backend = factory.create(config).await?;
    backend.freeze_time(now_ms).await?;
    Ok(backend)
}

async fn execute_ok(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    operation: ClientRequest,
) -> Result<ClientResponse> {
    let response = backend.execute(identity, request(operation)).await?;
    ensure!(
        response.protocol_version == PROTOCOL_VERSION,
        "backend returned protocol version {}, expected {PROTOCOL_VERSION}",
        response.protocol_version
    );
    match response.result {
        Ok(response) => Ok(response),
        Err(error) => bail!(
            "unexpected protocol error {:?}: {}",
            error.code,
            error.message
        ),
    }
}

async fn execute_error(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    operation: ClientRequest,
) -> Result<ClientErrorCode> {
    let response = backend.execute(identity, request(operation)).await?;
    match response.result {
        Ok(response) => bail!("expected protocol error, got {response:?}"),
        Err(error) => Ok(error.code),
    }
}

async fn put(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    key_name: &str,
    value: &[u8],
    ttl_ms: Option<u64>,
) -> Result<()> {
    let response = execute_ok(
        backend,
        identity,
        ClientRequest::Put {
            ns: namespace()?,
            key: key(key_name)?,
            value: value.to_vec(),
            ttl_ms,
            dimensions: Vec::new(),
        },
    )
    .await?;
    ensure!(matches!(response, ClientResponse::Stored));
    Ok(())
}

async fn get(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    key_name: &str,
) -> Result<Option<Vec<u8>>> {
    match execute_ok(
        backend,
        identity,
        ClientRequest::Get {
            ns: namespace()?,
            key: key(key_name)?,
        },
    )
    .await?
    {
        ClientResponse::Value { value } => Ok(value),
        response => bail!("expected value response, got {response:?}"),
    }
}

async fn conditional_put(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    key_name: &str,
    value: &[u8],
    ttl_ms: Option<u64>,
    condition: ConditionalPutCondition,
) -> Result<bool> {
    match execute_ok(
        backend,
        identity,
        ClientRequest::ConditionalPut {
            ns: namespace()?,
            key: key(key_name)?,
            value: value.to_vec(),
            ttl_ms,
            condition,
        },
    )
    .await?
    {
        ClientResponse::ConditionalStored { stored } => Ok(stored),
        response => bail!("expected conditional response, got {response:?}"),
    }
}

async fn compare_invalidate(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    key_name: &str,
    expected: &[u8],
) -> Result<bool> {
    match execute_ok(
        backend,
        identity,
        ClientRequest::CompareValueAndInvalidate {
            ns: namespace()?,
            key: key(key_name)?,
            expected_value: expected.to_vec(),
        },
    )
    .await?
    {
        ClientResponse::CompareValueApplied { applied } => Ok(applied),
        response => bail!("expected compare-value response, got {response:?}"),
    }
}

async fn compare_expire(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    key_name: &str,
    expected: &[u8],
    ttl_ms: u64,
    mode: CompareValueExpireMode,
) -> Result<bool> {
    match execute_ok(
        backend,
        identity,
        ClientRequest::CompareValueAndExpire {
            ns: namespace()?,
            key: key(key_name)?,
            expected_value: expected.to_vec(),
            ttl_ms,
            mode,
        },
    )
    .await?
    {
        ClientResponse::CompareValueApplied { applied } => Ok(applied),
        response => bail!("expected compare-value response, got {response:?}"),
    }
}

async fn ttl(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    key_name: &str,
) -> Result<TtlState> {
    match execute_ok(
        backend,
        identity,
        ClientRequest::GetTtl {
            ns: namespace()?,
            key: key(key_name)?,
        },
    )
    .await?
    {
        ClientResponse::Ttl { state } => Ok(state),
        response => bail!("expected ttl response, got {response:?}"),
    }
}

async fn batch_put(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    entries: &[(&str, &[u8])],
) -> Result<()> {
    let entries = entries
        .iter()
        .map(|(key_name, value)| {
            Ok(BatchPutEntry {
                key: key(key_name)?,
                value: value.to_vec(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let response = execute_ok(
        backend,
        identity,
        ClientRequest::BatchPut {
            ns: namespace()?,
            entries,
        },
    )
    .await?;
    ensure!(matches!(response, ClientResponse::Batch { .. }));
    Ok(())
}

async fn batch_get(
    backend: &dyn ClientSurfaceBackend,
    identity: &ConformanceIdentity,
    keys: &[&str],
) -> Result<Vec<Option<Vec<u8>>>> {
    let keys = keys
        .iter()
        .map(|key_name| key(key_name))
        .collect::<Result<Vec<_>>>()?;
    match execute_ok(
        backend,
        identity,
        ClientRequest::BatchGet {
            ns: namespace()?,
            keys,
        },
    )
    .await?
    {
        ClientResponse::Batch { items } => items
            .into_iter()
            .enumerate()
            .map(|(expected_index, item)| {
                ensure!(
                    item.index == expected_index,
                    "batch response reordered items"
                );
                item.result
                    .map_err(|error| anyhow::anyhow!("batch item failed: {:?}", error.code))
            })
            .collect(),
        response => bail!("expected batch response, got {response:?}"),
    }
}

/// Assert atomic `IfAbsent` behavior under concurrent acquisition attempts.
pub async fn assert_conditional_put_if_absent_is_atomic_under_n_concurrent_acquirers<F>(
    factory: &F,
) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let backend = fresh(factory, ClientSurfaceConformanceConfig::default()).await?;
    let identity = identity_a();
    let contenders = 16;
    let barrier = Arc::new(Barrier::new(contenders));
    let mut tasks = Vec::with_capacity(contenders);
    for contender in 0..contenders {
        let backend = Arc::clone(&backend);
        let identity = identity.clone();
        let barrier = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            let token = format!("token-{contender}").into_bytes();
            barrier.wait().await;
            let stored = conditional_put(
                backend.as_ref(),
                &identity,
                "lock",
                &token,
                Some(1_000),
                ConditionalPutCondition::IfAbsent,
            )
            .await?;
            let observed = if stored {
                None
            } else {
                get(backend.as_ref(), &identity, "lock").await?
            };
            Ok::<_, anyhow::Error>((stored, token, observed))
        }));
    }

    let mut winners = Vec::new();
    let mut loser_observations = Vec::new();
    for task in tasks {
        let (stored, token, observed) = task.await.context("concurrent acquirer panicked")??;
        if stored {
            winners.push(token);
        } else {
            loser_observations.push(observed);
        }
    }
    ensure!(
        winners.len() == 1,
        "expected one winner, got {}",
        winners.len()
    );
    let winner = winners.remove(0);
    ensure!(
        loser_observations
            .into_iter()
            .all(|observed| observed == Some(winner.clone())),
        "every loser must observe the winning token"
    );
    ensure!(get(backend.as_ref(), &identity, "lock").await? == Some(winner));
    Ok(())
}

/// Assert that an expired entry satisfies `IfAbsent`.
pub async fn assert_conditional_put_treats_expired_key_as_absent<F>(factory: &F) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let backend = fresh(factory, ClientSurfaceConformanceConfig::default()).await?;
    let identity = identity_a();
    put(backend.as_ref(), &identity, "lock", b"old", Some(10)).await?;
    backend.advance_time(10).await?;
    ensure!(
        conditional_put(
            backend.as_ref(),
            &identity,
            "lock",
            b"new",
            None,
            ConditionalPutCondition::IfAbsent,
        )
        .await?
    );
    ensure!(get(backend.as_ref(), &identity, "lock").await? == Some(b"new".to_vec()));
    Ok(())
}

/// Assert token-safe compare-and-invalidate semantics.
pub async fn assert_compare_value_invalidate_is_token_safe_and_returns_applied_count<F>(
    factory: &F,
) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let backend = fresh(factory, ClientSurfaceConformanceConfig::default()).await?;
    let identity = identity_a();
    put(backend.as_ref(), &identity, "lock", b"owner", None).await?;
    ensure!(!compare_invalidate(backend.as_ref(), &identity, "lock", b"stranger").await?);
    ensure!(get(backend.as_ref(), &identity, "lock").await? == Some(b"owner".to_vec()));
    ensure!(compare_invalidate(backend.as_ref(), &identity, "lock", b"owner").await?);
    ensure!(get(backend.as_ref(), &identity, "lock").await?.is_none());
    ensure!(!compare_invalidate(backend.as_ref(), &identity, "lock", b"owner").await?);
    Ok(())
}

/// Assert compare-value expiry modes and persistent-key guards.
pub async fn assert_compare_value_expire_add_to_remaining_and_replace_if_expiring_and_persistent_guard<
    F,
>(
    factory: &F,
) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let backend = fresh(factory, ClientSurfaceConformanceConfig::default()).await?;
    let identity = identity_a();
    put(backend.as_ref(), &identity, "lock", b"owner", Some(100)).await?;
    ensure!(
        compare_expire(
            backend.as_ref(),
            &identity,
            "lock",
            b"owner",
            50,
            CompareValueExpireMode::AddToRemaining,
        )
        .await?
    );
    ensure!(ttl(backend.as_ref(), &identity, "lock").await? == TtlState::ExpiresIn { ttl_ms: 150 });
    ensure!(
        compare_expire(
            backend.as_ref(),
            &identity,
            "lock",
            b"owner",
            20,
            CompareValueExpireMode::ReplaceIfExpiring,
        )
        .await?
    );
    ensure!(ttl(backend.as_ref(), &identity, "lock").await? == TtlState::ExpiresIn { ttl_ms: 20 });
    let response = execute_ok(
        backend.as_ref(),
        &identity,
        ClientRequest::Persist {
            ns: namespace()?,
            key: key("lock")?,
        },
    )
    .await?;
    ensure!(matches!(response, ClientResponse::Expiry { applied: true }));
    ensure!(
        !compare_expire(
            backend.as_ref(),
            &identity,
            "lock",
            b"owner",
            20,
            CompareValueExpireMode::ReplaceIfExpiring,
        )
        .await?
    );
    ensure!(
        !compare_expire(
            backend.as_ref(),
            &identity,
            "lock",
            b"owner",
            20,
            CompareValueExpireMode::AddToRemaining,
        )
        .await?
    );
    ensure!(ttl(backend.as_ref(), &identity, "lock").await? == TtlState::Persistent);
    Ok(())
}

/// Assert that batch prevalidation cannot leave a partial write.
pub async fn assert_batch_put_is_all_or_nothing_under_prevalidation_failure<F>(
    factory: &F,
) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let config = ClientSurfaceConformanceConfig::single_tenant(
        ConformanceLimits {
            max_value_bytes: 4,
            ..ConformanceLimits::default()
        },
        64,
        8,
    );
    let backend = fresh(factory, config).await?;
    let identity = identity_a();
    let error = execute_error(
        backend.as_ref(),
        &identity,
        ClientRequest::BatchPut {
            ns: namespace()?,
            entries: vec![
                BatchPutEntry {
                    key: key("first")?,
                    value: vec![1; 4],
                },
                BatchPutEntry {
                    key: key("invalid")?,
                    value: vec![2; 5],
                },
            ],
        },
    )
    .await?;
    ensure!(error == ClientErrorCode::TooLarge);
    ensure!(
        batch_get(backend.as_ref(), &identity, &["first", "invalid"]).await? == vec![None, None]
    );
    Ok(())
}

/// Assert exact missing, persistent, and expiring TTL states.
pub async fn assert_ttl_states_missing_persistent_expiring_round_trip<F>(factory: &F) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let backend = fresh(factory, ClientSurfaceConformanceConfig::default()).await?;
    let identity = identity_a();
    ensure!(ttl(backend.as_ref(), &identity, "missing").await? == TtlState::Missing);
    put(backend.as_ref(), &identity, "persistent", b"v", None).await?;
    ensure!(ttl(backend.as_ref(), &identity, "persistent").await? == TtlState::Persistent);
    put(backend.as_ref(), &identity, "expiring", b"v", Some(50)).await?;
    ensure!(
        ttl(backend.as_ref(), &identity, "expiring").await? == TtlState::ExpiresIn { ttl_ms: 50 }
    );
    backend.advance_time(20).await?;
    ensure!(
        ttl(backend.as_ref(), &identity, "expiring").await? == TtlState::ExpiresIn { ttl_ms: 30 }
    );
    Ok(())
}

/// Assert that reads never expose expired entries.
pub async fn assert_expired_key_absent_for_get_and_batch_get<F>(factory: &F) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let backend = fresh(factory, ClientSurfaceConformanceConfig::default()).await?;
    let identity = identity_a();
    put(backend.as_ref(), &identity, "one", b"v", Some(5)).await?;
    put(backend.as_ref(), &identity, "two", b"v", Some(5)).await?;
    backend.advance_time(5).await?;
    ensure!(get(backend.as_ref(), &identity, "one").await?.is_none());
    ensure!(batch_get(backend.as_ref(), &identity, &["two", "missing"]).await? == vec![None, None]);
    Ok(())
}

/// Assert per-value, batch-count, and tenant quota limits.
pub async fn assert_enforces_value_bytes_batch_and_tenant_quota_limits<F>(factory: &F) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let config = ClientSurfaceConformanceConfig::single_tenant(
        ConformanceLimits {
            max_value_bytes: 4,
            max_batch_entries: 2,
            max_batch_bytes: 128,
        },
        6,
        2,
    );
    let backend = fresh(factory, config).await?;
    let identity = identity_a();
    ensure!(
        execute_error(
            backend.as_ref(),
            &identity,
            ClientRequest::Put {
                ns: namespace()?,
                key: key("oversized")?,
                value: vec![0; 5],
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        )
        .await?
            == ClientErrorCode::TooLarge
    );
    ensure!(
        execute_error(
            backend.as_ref(),
            &identity,
            ClientRequest::BatchPut {
                ns: namespace()?,
                entries: vec![
                    BatchPutEntry {
                        key: key("a")?,
                        value: vec![1]
                    },
                    BatchPutEntry {
                        key: key("b")?,
                        value: vec![1]
                    },
                    BatchPutEntry {
                        key: key("c")?,
                        value: vec![1]
                    },
                ],
            },
        )
        .await?
            == ClientErrorCode::TooLarge
    );
    put(backend.as_ref(), &identity, "quota-a", &[1; 4], None).await?;
    ensure!(
        execute_error(
            backend.as_ref(),
            &identity,
            ClientRequest::Put {
                ns: namespace()?,
                key: key("quota-b")?,
                value: vec![2; 3],
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        )
        .await?
            == ClientErrorCode::TenantQuota
    );
    Ok(())
}

/// Assert failed conditional and batch requests do not reserve live quota.
pub async fn assert_rejected_conditionals_and_batches_do_not_reserve_quota<F>(
    factory: &F,
) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let limits = ConformanceLimits {
        max_value_bytes: 6,
        ..ConformanceLimits::default()
    };
    let backend = fresh(
        factory,
        ClientSurfaceConformanceConfig::single_tenant(limits, 2, 1),
    )
    .await?;
    let identity = identity_a();
    put(backend.as_ref(), &identity, "lock", &[1; 2], None).await?;
    ensure!(
        !conditional_put(
            backend.as_ref(),
            &identity,
            "lock",
            &[2; 6],
            None,
            ConditionalPutCondition::IfAbsent,
        )
        .await?
    );
    ensure!(matches!(
        execute_ok(
            backend.as_ref(),
            &identity,
            ClientRequest::Invalidate {
                ns: namespace()?,
                key: key("lock")?,
            },
        )
        .await?,
        ClientResponse::Invalidated
    ));
    put(backend.as_ref(), &identity, "reused", &[3; 2], None).await?;

    let backend = fresh(
        factory,
        ClientSurfaceConformanceConfig::single_tenant(limits, 8, 2),
    )
    .await?;
    put(backend.as_ref(), &identity, "existing", &[1; 4], None).await?;
    ensure!(
        execute_error(
            backend.as_ref(),
            &identity,
            ClientRequest::BatchPut {
                ns: namespace()?,
                entries: vec![
                    BatchPutEntry {
                        key: key("rejected-a")?,
                        value: vec![2; 3]
                    },
                    BatchPutEntry {
                        key: key("rejected-b")?,
                        value: vec![3; 3]
                    },
                ],
            },
        )
        .await?
            == ClientErrorCode::TenantQuota
    );
    put(backend.as_ref(), &identity, "remaining", &[4; 4], None).await?;
    Ok(())
}

/// Assert delete and lazy expiry release both byte and entry quota.
pub async fn assert_delete_and_expiry_release_tenant_quota<F>(factory: &F) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let config =
        || ClientSurfaceConformanceConfig::single_tenant(ConformanceLimits::default(), 4, 1);
    let identity = identity_a();

    let backend = fresh(factory, config()).await?;
    put(backend.as_ref(), &identity, "first", &[1; 4], None).await?;
    let response = execute_ok(
        backend.as_ref(),
        &identity,
        ClientRequest::Invalidate {
            ns: namespace()?,
            key: key("first")?,
        },
    )
    .await?;
    ensure!(matches!(response, ClientResponse::Invalidated));
    put(backend.as_ref(), &identity, "second", &[2; 4], None).await?;

    let backend = fresh(factory, config()).await?;
    put(backend.as_ref(), &identity, "first", &[1; 4], None).await?;
    ensure!(compare_invalidate(backend.as_ref(), &identity, "first", &[1; 4]).await?);
    put(backend.as_ref(), &identity, "second", &[2; 4], None).await?;

    let backend = fresh(factory, config()).await?;
    put(backend.as_ref(), &identity, "first", &[1; 4], Some(5)).await?;
    backend.advance_time(5).await?;
    ensure!(get(backend.as_ref(), &identity, "first").await?.is_none());
    put(backend.as_ref(), &identity, "second", &[2; 4], None).await?;
    Ok(())
}

/// Assert duplicate batch keys use last-write-wins quota accounting.
pub async fn assert_duplicate_batch_keys_account_last_write_only<F>(factory: &F) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let config = ClientSurfaceConformanceConfig::single_tenant(
        ConformanceLimits {
            max_value_bytes: 4,
            ..ConformanceLimits::default()
        },
        6,
        2,
    );
    let backend = fresh(factory, config).await?;
    let identity = identity_a();
    batch_put(
        backend.as_ref(),
        &identity,
        &[("same", &[1; 4]), ("same", &[2; 2])],
    )
    .await?;
    put(backend.as_ref(), &identity, "other", &[3; 4], None).await?;
    ensure!(get(backend.as_ref(), &identity, "same").await? == Some(vec![2; 2]));
    Ok(())
}

/// Assert tenant binding and same-name namespace/key isolation.
pub async fn assert_tenant_binding_and_same_namespace_keys_are_isolated<F>(
    factory: &F,
) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let mut config =
        ClientSurfaceConformanceConfig::single_tenant(ConformanceLimits::default(), 64, 8);
    config.tenants.push(ConformanceTenant::single_namespace(
        "tenant-b",
        "client-b",
        ConformanceNamespace::new("users", 64, 8),
    ));
    let backend = fresh(factory, config).await?;
    let identity_a = identity_a();
    let identity_b = ConformanceIdentity::new("client-b", "tenant-b");
    put(backend.as_ref(), &identity_a, "same", b"a", None).await?;
    put(backend.as_ref(), &identity_b, "same", b"b", None).await?;
    ensure!(get(backend.as_ref(), &identity_a, "same").await? == Some(b"a".to_vec()));
    ensure!(get(backend.as_ref(), &identity_b, "same").await? == Some(b"b".to_vec()));

    let mismatched = ConformanceIdentity::new("client-a", "tenant-b");
    ensure!(
        execute_error(
            backend.as_ref(),
            &mismatched,
            ClientRequest::Put {
                ns: namespace()?,
                key: key("same")?,
                value: b"forged".to_vec(),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        )
        .await?
            == ClientErrorCode::Unauthorized
    );
    ensure!(get(backend.as_ref(), &identity_a, "same").await? == Some(b"a".to_vec()));
    ensure!(get(backend.as_ref(), &identity_b, "same").await? == Some(b"b".to_vec()));
    Ok(())
}

/// Assert batch count and byte boundaries at exactly N and N+1.
pub async fn assert_batch_entry_and_byte_limits_reject_at_boundary_plus_one_without_mutation<F>(
    factory: &F,
) -> Result<()>
where
    F: ClientSurfaceBackendFactory + ?Sized,
{
    let identity = identity_a();
    let backend = fresh(
        factory,
        ClientSurfaceConformanceConfig::single_tenant(
            ConformanceLimits {
                max_value_bytes: 8,
                max_batch_entries: 2,
                max_batch_bytes: 128,
            },
            128,
            16,
        ),
    )
    .await?;
    batch_put(backend.as_ref(), &identity, &[("a", b"1"), ("b", b"2")]).await?;
    ensure!(
        batch_get(backend.as_ref(), &identity, &["a", "b"])
            .await?
            .len()
            == 2
    );
    ensure!(
        execute_error(
            backend.as_ref(),
            &identity,
            ClientRequest::BatchPut {
                ns: namespace()?,
                entries: vec![
                    BatchPutEntry {
                        key: key("x")?,
                        value: b"1".to_vec()
                    },
                    BatchPutEntry {
                        key: key("y")?,
                        value: b"2".to_vec()
                    },
                    BatchPutEntry {
                        key: key("z")?,
                        value: b"3".to_vec()
                    },
                ],
            },
        )
        .await?
            == ClientErrorCode::TooLarge
    );
    ensure!(
        execute_error(
            backend.as_ref(),
            &identity,
            ClientRequest::BatchGet {
                ns: namespace()?,
                keys: vec![key("x")?, key("y")?, key("z")?],
            },
        )
        .await?
            == ClientErrorCode::TooLarge
    );
    ensure!(get(backend.as_ref(), &identity, "x").await?.is_none());
    ensure!(get(backend.as_ref(), &identity, "y").await?.is_none());
    ensure!(get(backend.as_ref(), &identity, "z").await?.is_none());

    let backend = fresh(
        factory,
        ClientSurfaceConformanceConfig::single_tenant(
            ConformanceLimits {
                max_value_bytes: 8,
                max_batch_entries: 8,
                max_batch_bytes: 6,
            },
            128,
            16,
        ),
    )
    .await?;
    batch_put(backend.as_ref(), &identity, &[("a", b"12"), ("b", b"34")]).await?;
    ensure!(
        batch_get(backend.as_ref(), &identity, &["abc", "def"])
            .await?
            .len()
            == 2
    );
    ensure!(
        execute_error(
            backend.as_ref(),
            &identity,
            ClientRequest::BatchPut {
                ns: namespace()?,
                entries: vec![BatchPutEntry {
                    key: key("c")?,
                    value: vec![0; 6]
                }],
            },
        )
        .await?
            == ClientErrorCode::TooLarge
    );
    ensure!(
        execute_error(
            backend.as_ref(),
            &identity,
            ClientRequest::BatchGet {
                ns: namespace()?,
                keys: vec![key("1234567")?],
            },
        )
        .await?
            == ClientErrorCode::TooLarge
    );
    ensure!(get(backend.as_ref(), &identity, "c").await?.is_none());
    Ok(())
}
