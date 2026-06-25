use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use bytes::Bytes;
use hydracache::{ClusterGeneration, MetaDataContainer, NearCacheRepairAction};
use hydracache_client::{
    stable_error_retryable, ClientError, ClientIdentity, ClientTransport, ConformanceManifest,
    HydraClient, HydraClientConfig, RemoteNearCache, RequestOptions, RetryPolicy,
};
use hydracache_client_protocol::{
    ClientErrorCode, ClientRequest, Namespace, RepairAction, StructuredKey, VersionHandshake,
    MIN_PROTOCOL_VERSION, PROTOCOL_VERSION,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, CLIENT_DATA_PATH, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use proptest::prelude::*;
use tower::ServiceExt;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn manifest_text() -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/conformance/client_v1.json"),
    )
    .expect("conformance manifest")
}

fn manifest() -> ConformanceManifest {
    serde_json::from_str(&manifest_text()).expect("valid conformance manifest")
}

fn identity() -> ClientIdentity {
    ClientIdentity::new("rust-conformance", "tenant-a").unwrap()
}

fn ns() -> Namespace {
    Namespace::new("users").unwrap()
}

fn key(id: &str) -> StructuredKey {
    StructuredKey::new(vec!["user".to_owned(), id.to_owned()]).unwrap()
}

#[derive(Debug)]
struct TwoNodeAxumTransport {
    endpoints: [AxumClientSurface; 2],
    next: AtomicUsize,
}

impl TwoNodeAxumTransport {
    fn new() -> Self {
        let primary = AxumClientSurface::new(ClientSurfaceLimits::default()).unwrap();
        let secondary = AxumClientSurface::from_state(primary.state());
        Self {
            endpoints: [primary, secondary],
            next: AtomicUsize::new(0),
        }
    }

    fn primary(&self) -> &AxumClientSurface {
        &self.endpoints[0]
    }
}

#[async_trait]
impl ClientTransport for TwoNodeAxumTransport {
    async fn send_frame(
        &self,
        identity: &ClientIdentity,
        frame: Bytes,
    ) -> Result<Bytes, ClientError> {
        let index = self.next.fetch_add(1, Ordering::SeqCst) % self.endpoints.len();
        let response = self.endpoints[index]
            .routes()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(CLIENT_DATA_PATH)
                    .header(HYDRACACHE_CLIENT_ID_HEADER, identity.client_id())
                    .header(HYDRACACHE_TENANT_HEADER, identity.tenant())
                    .body(Body::from(frame))
                    .expect("request"),
            )
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))?;

        if response.status() != StatusCode::OK {
            return Err(ClientError::Transport(format!(
                "unexpected status {}",
                response.status()
            )));
        }

        to_bytes(response.into_body(), 1024 * 1024)
            .await
            .map_err(|error| ClientError::Transport(error.to_string()))
    }
}

#[tokio::test]
async fn rust_client_passes_full_conformance() {
    let manifest = manifest();
    assert_eq!(manifest.protocol_version, MIN_PROTOCOL_VERSION);
    assert!(manifest
        .scenarios
        .iter()
        .any(|s| s.id == "get-put-invalidate-round-trip"));

    let transport = TwoNodeAxumTransport::new();
    let client = HydraClient::connect(transport, HydraClientConfig::new(identity()))
        .await
        .expect("client connects");

    client
        .put(ns(), key("42"), Bytes::from_static(b"profile"), None)
        .await
        .expect("put");
    assert_eq!(
        client.get(ns(), key("42")).await.expect("get"),
        Some(Bytes::from_static(b"profile"))
    );
    client
        .invalidate(ns(), key("42"))
        .await
        .expect("invalidate");
    assert_eq!(client.get(ns(), key("42")).await.expect("get"), None);

    let mut near_cache = client.near_cache();
    assert_eq!(near_cache.on_watermark(1, 1), RepairAction::ClearPartition);
    assert_eq!(
        near_cache.on_watermark(1, 3),
        RepairAction::InvalidateConservatively
    );
    assert_eq!(client.metrics().client_sessions_active, 1);
    assert_eq!(client.metrics().client_near_cache_repairs_total, 2);
}

proptest! {
    #[test]
    fn conformance_near_cache_reconciles_like_embedded(
        frames in prop::collection::vec((0u64..4, 0u64..20), 1..64)
    ) {
        let mut remote = RemoteNearCache::default();
        let mut embedded = MetaDataContainer::default();

        for (generation, message_id) in frames {
            let remote_action = remote.on_watermark(generation, message_id);
            let embedded_action = embedded.on_watermark(
                Some(ClusterGeneration::new(generation)),
                Some(message_id),
            );
            prop_assert_eq!(remote_action, map_embedded_action(embedded_action));
        }
    }
}

#[tokio::test]
async fn conformance_client_respects_negotiated_version() {
    let transport = TwoNodeAxumTransport::new();
    let config = HydraClientConfig {
        supported_versions: VersionHandshake::new(PROTOCOL_VERSION, PROTOCOL_VERSION),
        ..HydraClientConfig::new(identity())
    };
    let client = HydraClient::connect(transport, config)
        .await
        .expect("client connects");

    assert_eq!(client.negotiated_version(), PROTOCOL_VERSION);
}

#[tokio::test]
async fn conformance_v1_client_keeps_v1_compat_window() {
    let transport = TwoNodeAxumTransport::new();
    let config = HydraClientConfig {
        supported_versions: VersionHandshake::new(MIN_PROTOCOL_VERSION, MIN_PROTOCOL_VERSION),
        ..HydraClientConfig::new(identity())
    };
    let client = HydraClient::connect(transport, config)
        .await
        .expect("client connects");

    assert_eq!(client.negotiated_version(), MIN_PROTOCOL_VERSION);
    assert_eq!(client.get(ns(), key("missing")).await.unwrap(), None);
}

#[test]
fn conformance_client_error_mapping_matches_protocol_manifest() {
    let manifest = manifest();
    let all_codes = [
        ClientErrorCode::IncompatibleVersion,
        ClientErrorCode::Unauthenticated,
        ClientErrorCode::Unauthorized,
        ClientErrorCode::TenantQuota,
        ClientErrorCode::RateLimited,
        ClientErrorCode::ResidencyDenied,
        ClientErrorCode::TooLarge,
        ClientErrorCode::DeadlineExceeded,
        ClientErrorCode::Conflict,
        ClientErrorCode::BackendUnavailable,
        ClientErrorCode::MalformedFrame,
    ];

    assert_eq!(manifest.errors.len(), all_codes.len());
    for code in all_codes {
        let entry = manifest
            .errors
            .iter()
            .find(|entry| entry.code == code)
            .unwrap_or_else(|| panic!("manifest missing {code:?}"));
        assert_eq!(entry.retryable, stable_error_retryable(code));
    }
}

#[tokio::test]
async fn conformance_client_deadline_retry_and_idempotency_match_conformance() {
    let transport = TwoNodeAxumTransport::new();
    let primary = transport.primary().clone();
    let client = HydraClient::connect(
        transport,
        HydraClientConfig::new(identity()).with_retry(RetryPolicy {
            max_attempts: 2,
            backoff_ms: 0,
        }),
    )
    .await
    .expect("client connects");

    let expired = client
        .request(
            ClientRequest::Get {
                ns: ns(),
                key: key("deadline"),
            },
            RequestOptions::default().with_deadline_ms(0),
        )
        .await
        .expect_err("expired deadline should fail");
    let ClientError::Server(error) = expired else {
        panic!("expected stable server error");
    };
    assert_eq!(error.code, ClientErrorCode::DeadlineExceeded);
    assert!(error.retryable);

    let put = || ClientRequest::Put {
        ns: ns(),
        key: key("idem"),
        value: b"once".to_vec(),
        ttl_ms: None,
        dimensions: Vec::new(),
    };
    let options = RequestOptions::default().with_idempotency_key("idem-1");
    client
        .request(put(), options.clone())
        .await
        .expect("first put");
    client.request(put(), options).await.expect("second put");
    assert_eq!(primary.state().state_mutations(), 1);
}

#[test]
fn conformance_manifest_is_language_agnostic() {
    let raw = manifest_text();
    let manifest = manifest();

    assert_eq!(manifest.manifest_version, 1);
    assert!(manifest
        .sdks
        .iter()
        .any(|sdk| sdk.language == "python" && sdk.supported));
    assert!(manifest
        .sdks
        .iter()
        .any(|sdk| sdk.language == "rust" && sdk.supported));
    assert!(manifest.scenarios.len() >= 6);

    for scenario in &manifest.scenarios {
        let scenario_text = format!(
            "{} {} {} {}",
            scenario.id,
            scenario.kind,
            scenario.behavior,
            scenario.requires.join(" ")
        );
        for forbidden in ["cargo", ".rs", "tokio", "crate::", "pytest", ".py"] {
            assert!(
                !scenario_text.contains(forbidden),
                "scenario {} contains language-specific marker {forbidden}",
                scenario.id
            );
        }
    }

    assert!(
        !raw.contains("C:\\") && !raw.contains("/home/"),
        "manifest should not contain host-specific paths"
    );
}

#[test]
#[ignore = "nightly Docker tier: runs Python SDK runner against a live grid"]
fn non_jvm_sdk_conformance() {
    let root = repo_root();
    let pyproject =
        fs::read_to_string(root.join("sdks/python/pyproject.toml")).expect("python SDK pyproject");
    let runner = fs::read_to_string(root.join("sdks/python/hydracache_client/conformance.py"))
        .expect("python SDK conformance runner");

    assert!(pyproject.contains("name = \"hydracache-client\""));
    assert!(pyproject.contains("requires-python = \">=3.10\""));
    assert!(runner.contains("client_v1.json"));
    assert!(runner.contains("deadline-retry-idempotency"));
}

fn map_embedded_action(action: NearCacheRepairAction) -> RepairAction {
    match action {
        NearCacheRepairAction::Apply => RepairAction::Apply,
        NearCacheRepairAction::ClearPartition => RepairAction::ClearPartition,
        NearCacheRepairAction::InvalidateConservatively => RepairAction::InvalidateConservatively,
    }
}
