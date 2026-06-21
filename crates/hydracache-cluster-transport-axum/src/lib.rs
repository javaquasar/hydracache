//! Axum/HTTP peer-fetch transport for HydraCache cluster members.
//!
//! The base `hydracache` crate exposes the transport-neutral
//! [`ClusterPeerFetch`] seam. This crate provides
//! an opt-in HTTP implementation so local-only applications do not inherit
//! HTTP client/server dependencies.
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use hydracache::{
//!     CacheOptions, ClusterCandidate, ClusterGeneration, ClusterPeerFetch,
//!     ClusterPeerFetchRequest, HydraCache, InMemoryCluster,
//! };
//! use hydracache_cluster_transport_axum::{AxumPeerFetchService, HttpPeerFetch, PeerFetchRouter};
//!
//! # async fn example() -> hydracache::CacheResult<()> {
//! let owner_cache = HydraCache::local().build();
//! owner_cache.put("user:42", 42_u64, CacheOptions::new()).await?;
//!
//! let routes = AxumPeerFetchService::new(
//!     "member-a",
//!     ClusterGeneration::new(1),
//!     Arc::new(owner_cache),
//! )
//! .routes();
//! # let _ = routes;
//!
//! let peer_fetch = HttpPeerFetch::for_base_url("http://127.0.0.1:3000");
//! let response = peer_fetch
//!     .fetch(
//!         ClusterPeerFetchRequest::new("member-a", "user:42")
//!             .generation(ClusterGeneration::new(1)),
//!     )
//!     .await;
//! # let _ = response;
//!
//! let cluster = InMemoryCluster::new("orders");
//! cluster.join_member(
//!     ClusterCandidate::member("member-a")
//!         .generation(ClusterGeneration::new(1))
//!         .peer_fetch_base_url("http://127.0.0.1:3000"),
//! )?;
//! let routed = PeerFetchRouter::new()
//!     .fetch_owner_value(cluster.owner_for_key("user:42"))
//!     .await;
//! # let _ = routed;
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use bytes::Bytes;
use futures_util::future::{BoxFuture, Shared};
use futures_util::FutureExt;
use hydracache::{
    CacheError, CacheOptions, CacheResult, ClusterGeneration, ClusterNodeId,
    ClusterOwnershipDecision, ClusterPeerFetch, ClusterPeerFetchRequest, ClusterPeerFetchResponse,
    HydraCache,
};
use hydracache_core::CacheCodec;
use serde::{Deserialize, Serialize};

/// Default HTTP path used by the peer-fetch route and client.
pub const DEFAULT_PEER_FETCH_PATH: &str = "/cluster/peer-fetch";

/// Default HTTP path reserved for owner-side load requests.
pub const DEFAULT_OWNER_LOAD_PATH: &str = "/cluster/owner-load";

/// Current HydraCache HTTP transport wire version.
pub const HYDRACACHE_HTTP_WIRE_VERSION: u16 = 1;

/// HTTP header that carries the HydraCache transport wire version.
pub const HYDRACACHE_WIRE_VERSION_HEADER: &str = "x-hydracache-wire-version";

/// Default token header for simple non-browser cluster transport auth.
pub const HYDRACACHE_TOKEN_HEADER: &str = "x-hydracache-token";

/// Optional authentication boundary for HydraCache HTTP transport routes.
///
/// This is intentionally small: HydraCache does not manage TLS, certificates,
/// identities, or token rotation. Put this behind HTTPS/mTLS or a trusted
/// private network when evaluating cluster transport in staging.
///
/// # Example
///
/// ```
/// use hydracache_cluster_transport_axum::HttpTransportAuth;
///
/// let bearer = HttpTransportAuth::bearer("staging-token");
/// assert_eq!(bearer.header_name(), "authorization");
/// assert_eq!(bearer.header_value(), "Bearer staging-token");
///
/// let token = HttpTransportAuth::token("secret");
/// assert_eq!(token.header_name(), "x-hydracache-token");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpTransportAuth {
    header_name: String,
    header_value: String,
}

impl HttpTransportAuth {
    /// Require `Authorization: Bearer <token>`.
    pub fn bearer(token: impl AsRef<str>) -> Self {
        Self::header("authorization", format!("Bearer {}", token.as_ref()))
            .expect("static authorization header name is valid")
    }

    /// Require `x-hydracache-token: <token>`.
    pub fn token(token: impl AsRef<str>) -> Self {
        Self::header(HYDRACACHE_TOKEN_HEADER, token.as_ref())
            .expect("static token header name is valid")
    }

    /// Require an explicit header/value pair.
    pub fn header(name: impl AsRef<str>, value: impl AsRef<str>) -> CacheResult<Self> {
        let name = name.as_ref().trim().to_ascii_lowercase();
        let value = value.as_ref().to_owned();
        HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
            CacheError::Backend(format!("invalid HydraCache auth header name: {error}"))
        })?;
        HeaderValue::from_str(&value).map_err(|error| {
            CacheError::Backend(format!("invalid HydraCache auth header value: {error}"))
        })?;

        Ok(Self {
            header_name: name,
            header_value: value,
        })
    }

    /// Header name expected by routes and attached by clients.
    pub fn header_name(&self) -> &str {
        &self.header_name
    }

    /// Header value expected by routes and attached by clients.
    pub fn header_value(&self) -> &str {
        &self.header_value
    }

    fn matches(&self, headers: &HeaderMap) -> bool {
        headers
            .get(self.header_name.as_str())
            .and_then(|value| value.to_str().ok())
            == Some(self.header_value.as_str())
    }

    fn apply(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request.header(self.header_name.as_str(), self.header_value.as_str())
    }
}

/// Wire-version compatibility policy for HydraCache HTTP transport.
///
/// By default routes accept missing wire headers for compatibility with older
/// local tests and demos, but they reject incompatible versions when a header is
/// present. Use [`HttpWireCompatibility::strict_current`] when staging routes
/// should require an explicit version header from every caller.
///
/// # Example
///
/// ```
/// use hydracache_cluster_transport_axum::{
///     HttpWireCompatibility, HYDRACACHE_HTTP_WIRE_VERSION,
/// };
///
/// let strict = HttpWireCompatibility::strict_current();
/// assert_eq!(strict.wire_version(), HYDRACACHE_HTTP_WIRE_VERSION);
/// assert!(strict.requires_header());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpWireCompatibility {
    wire_version: u16,
    require_header: bool,
}

impl Default for HttpWireCompatibility {
    fn default() -> Self {
        Self::current()
    }
}

impl HttpWireCompatibility {
    /// Current wire version, accepting missing headers for compatibility.
    pub const fn current() -> Self {
        Self {
            wire_version: HYDRACACHE_HTTP_WIRE_VERSION,
            require_header: false,
        }
    }

    /// Current wire version, requiring the version header.
    pub const fn strict_current() -> Self {
        Self {
            wire_version: HYDRACACHE_HTTP_WIRE_VERSION,
            require_header: true,
        }
    }

    /// Custom wire version policy, primarily useful for compatibility tests.
    pub const fn version(wire_version: u16) -> Self {
        Self {
            wire_version,
            require_header: false,
        }
    }

    /// Set whether a missing wire-version header is rejected.
    pub const fn require_header(mut self, require_header: bool) -> Self {
        self.require_header = require_header;
        self
    }

    /// Wire version expected by routes and sent by clients.
    pub const fn wire_version(&self) -> u16 {
        self.wire_version
    }

    /// Whether routes reject requests that omit the wire-version header.
    pub const fn requires_header(&self) -> bool {
        self.require_header
    }

    fn apply(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request.header(
            HYDRACACHE_WIRE_VERSION_HEADER,
            self.wire_version.to_string(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpTransportRejection {
    status: StatusCode,
    code: &'static str,
    message: String,
}

fn validate_http_transport(
    headers: &HeaderMap,
    auth: Option<&HttpTransportAuth>,
    wire: HttpWireCompatibility,
) -> Result<(), HttpTransportRejection> {
    if let Some(auth) = auth {
        if !auth.matches(headers) {
            return Err(HttpTransportRejection {
                status: StatusCode::UNAUTHORIZED,
                code: "unauthorized",
                message: format!(
                    "missing or invalid HydraCache transport auth header '{}'",
                    auth.header_name()
                ),
            });
        }
    }

    match headers.get(HYDRACACHE_WIRE_VERSION_HEADER) {
        Some(value) => {
            let value = value.to_str().map_err(|error| HttpTransportRejection {
                status: StatusCode::BAD_REQUEST,
                code: "invalid-wire-version",
                message: format!("invalid HydraCache wire-version header: {error}"),
            })?;
            let requested = value
                .parse::<u16>()
                .map_err(|error| HttpTransportRejection {
                    status: StatusCode::BAD_REQUEST,
                    code: "invalid-wire-version",
                    message: format!("invalid HydraCache wire-version value '{value}': {error}"),
                })?;
            if requested != wire.wire_version() {
                return Err(HttpTransportRejection {
                    status: StatusCode::UPGRADE_REQUIRED,
                    code: "wire-version-mismatch",
                    message: format!(
                        "requested HydraCache wire version {requested} does not match supported version {}",
                        wire.wire_version()
                    ),
                });
            }
        }
        None if wire.requires_header() => {
            return Err(HttpTransportRejection {
                status: StatusCode::UPGRADE_REQUIRED,
                code: "missing-wire-version",
                message: format!(
                    "missing required HydraCache wire-version header '{HYDRACACHE_WIRE_VERSION_HEADER}'"
                ),
            });
        }
        None => {}
    }

    Ok(())
}

/// One typed argument passed to a registered owner-side loader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum OwnerLoadArg {
    /// UTF-8 string argument.
    String(String),
    /// Signed integer argument.
    I64(i64),
    /// Unsigned integer argument.
    U64(u64),
    /// Boolean argument.
    Bool(bool),
}

impl From<String> for OwnerLoadArg {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for OwnerLoadArg {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<i64> for OwnerLoadArg {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<u64> for OwnerLoadArg {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

impl From<bool> for OwnerLoadArg {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

/// Named argument bag for owner-side loaders.
///
/// # Example
///
/// ```
/// use hydracache_cluster_transport_axum::OwnerLoadArgs;
///
/// let args = OwnerLoadArgs::new()
///     .arg("id", 42_i64)
///     .arg("tenant", "acme");
///
/// assert_eq!(args.get_i64("id"), Some(42));
/// assert_eq!(args.get_str("tenant"), Some("acme"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerLoadArgs {
    values: BTreeMap<String, OwnerLoadArg>,
}

impl OwnerLoadArgs {
    /// Create an empty argument bag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace one named argument.
    pub fn arg(mut self, name: impl Into<String>, value: impl Into<OwnerLoadArg>) -> Self {
        self.values.insert(name.into(), value.into());
        self
    }

    /// Return one raw argument by name.
    pub fn get(&self, name: &str) -> Option<&OwnerLoadArg> {
        self.values.get(name)
    }

    /// Return one string argument by name.
    pub fn get_str(&self, name: &str) -> Option<&str> {
        match self.values.get(name) {
            Some(OwnerLoadArg::String(value)) => Some(value.as_str()),
            _ => None,
        }
    }

    /// Return one signed integer argument by name.
    pub fn get_i64(&self, name: &str) -> Option<i64> {
        match self.values.get(name) {
            Some(OwnerLoadArg::I64(value)) => Some(*value),
            Some(OwnerLoadArg::U64(value)) => i64::try_from(*value).ok(),
            _ => None,
        }
    }

    /// Return one unsigned integer argument by name.
    pub fn get_u64(&self, name: &str) -> Option<u64> {
        match self.values.get(name) {
            Some(OwnerLoadArg::U64(value)) => Some(*value),
            Some(OwnerLoadArg::I64(value)) => u64::try_from(*value).ok(),
            _ => None,
        }
    }

    /// Return one boolean argument by name.
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        match self.values.get(name) {
            Some(OwnerLoadArg::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    /// Return the number of arguments.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Return whether no arguments are present.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Application-level description of a load that may be routed to a key owner.
///
/// The descriptor is intentionally data-only. It names a registered loader and
/// carries key, tags, TTL, and serializable arguments. It never carries a Rust
/// closure or raw SQL string for arbitrary remote execution.
///
/// # Example
///
/// ```
/// use std::time::Duration;
///
/// use hydracache_cluster_transport_axum::OwnerLoadDescriptor;
///
/// let descriptor = OwnerLoadDescriptor::new("users.by-id")
///     .key("user:42")
///     .tag("user:42")
///     .arg("id", 42_i64)
///     .ttl(Duration::from_secs(60));
///
/// assert_eq!(descriptor.loader(), "users.by-id");
/// assert_eq!(descriptor.key_value(), Some("user:42"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerLoadDescriptor {
    loader: String,
    key: Option<String>,
    tags: Vec<String>,
    ttl_ms: Option<u64>,
    args: OwnerLoadArgs,
}

impl OwnerLoadDescriptor {
    /// Create a descriptor for a registered owner-side loader.
    pub fn new(loader: impl Into<String>) -> Self {
        Self {
            loader: loader.into(),
            key: None,
            tags: Vec::new(),
            ttl_ms: None,
            args: OwnerLoadArgs::new(),
        }
    }

    /// Set the logical cache key.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Attach one invalidation tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Attach invalidation tags.
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Set the owner-side cache TTL.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl_ms = Some(duration_to_millis(ttl));
        self
    }

    /// Set the owner-side cache TTL in milliseconds.
    pub fn ttl_millis(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = Some(ttl_ms);
        self
    }

    /// Add one named loader argument.
    pub fn arg(mut self, name: impl Into<String>, value: impl Into<OwnerLoadArg>) -> Self {
        self.args = self.args.arg(name, value);
        self
    }

    /// Return the registered loader name.
    pub fn loader(&self) -> &str {
        &self.loader
    }

    /// Return the logical cache key, if configured.
    pub fn key_value(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// Return configured tags.
    pub fn tags_value(&self) -> &[String] {
        &self.tags
    }

    /// Return the configured TTL in milliseconds.
    pub fn ttl_millis_value(&self) -> Option<u64> {
        self.ttl_ms
    }

    /// Return loader arguments.
    pub fn args(&self) -> &OwnerLoadArgs {
        &self.args
    }

    /// Convert descriptor metadata into local cache options.
    pub fn cache_options(&self) -> CacheOptions {
        let mut options = CacheOptions::new().tags(self.tags.clone());
        if let Some(ttl_ms) = self.ttl_ms {
            options = options.ttl(Duration::from_millis(ttl_ms));
        }
        options
    }

    /// Build an owner-load request from an ownership decision.
    pub fn into_request(
        self,
        decision: ClusterOwnershipDecision,
        request_id: impl Into<String>,
    ) -> Result<OwnerLoadRequest, OwnerLoadRequestBuildError> {
        OwnerLoadRequest::from_descriptor(decision, self, request_id)
    }
}

/// Error returned when a descriptor cannot become an owner-load request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerLoadRequestBuildError {
    /// The ownership decision had no eligible owner.
    NoOwner { key: String },
    /// The descriptor did not include a cache key.
    MissingKey { loader: String },
}

impl fmt::Display for OwnerLoadRequestBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoOwner { key } => {
                write!(
                    formatter,
                    "cannot build owner-load request for '{key}': no owner"
                )
            }
            Self::MissingKey { loader } => {
                write!(
                    formatter,
                    "owner-load descriptor '{loader}' is missing a key"
                )
            }
        }
    }
}

impl Error for OwnerLoadRequestBuildError {}

/// Transport-neutral owner-side load request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerLoadRequest {
    /// Expected owner member id.
    pub owner: String,
    /// Logical cache key to read or load on the owner.
    pub key: String,
    /// Registered owner-side loader name.
    pub loader: String,
    /// Invalidation tags to apply if the owner stores a loaded value.
    pub tags: Vec<String>,
    /// Per-entry TTL in milliseconds.
    pub ttl_ms: Option<u64>,
    /// Typed loader arguments.
    pub args: OwnerLoadArgs,
    /// Owner generation observed by the caller.
    pub generation: Option<u64>,
    /// Caller-generated request id for logs and diagnostics.
    pub request_id: String,
}

impl OwnerLoadRequest {
    /// Build a request from an ownership decision and descriptor.
    pub fn from_descriptor(
        decision: ClusterOwnershipDecision,
        descriptor: OwnerLoadDescriptor,
        request_id: impl Into<String>,
    ) -> Result<Self, OwnerLoadRequestBuildError> {
        let key = descriptor
            .key
            .clone()
            .ok_or_else(|| OwnerLoadRequestBuildError::MissingKey {
                loader: descriptor.loader.clone(),
            })?;
        let owner = decision
            .owner
            .ok_or_else(|| OwnerLoadRequestBuildError::NoOwner { key: key.clone() })?;

        Ok(Self {
            owner: owner.node_id.as_str().to_owned(),
            key,
            loader: descriptor.loader,
            tags: descriptor.tags,
            ttl_ms: descriptor.ttl_ms,
            args: descriptor.args,
            generation: Some(owner.generation.value()),
            request_id: request_id.into(),
        })
    }

    /// Replace the request argument bag.
    pub fn with_args(mut self, args: OwnerLoadArgs) -> Self {
        self.args = args;
        self
    }

    /// Return a required signed integer argument.
    pub fn arg_i64(&self, name: &str) -> Result<i64, OwnerLoadRequestArgError> {
        self.args
            .get_i64(name)
            .ok_or_else(|| OwnerLoadRequestArgError::missing_or_wrong_type(name, "i64"))
    }

    /// Return a required unsigned integer argument.
    pub fn arg_u64(&self, name: &str) -> Result<u64, OwnerLoadRequestArgError> {
        self.args
            .get_u64(name)
            .ok_or_else(|| OwnerLoadRequestArgError::missing_or_wrong_type(name, "u64"))
    }

    /// Return a required string argument.
    pub fn arg_str(&self, name: &str) -> Result<&str, OwnerLoadRequestArgError> {
        self.args
            .get_str(name)
            .ok_or_else(|| OwnerLoadRequestArgError::missing_or_wrong_type(name, "string"))
    }

    /// Return a required boolean argument.
    pub fn arg_bool(&self, name: &str) -> Result<bool, OwnerLoadRequestArgError> {
        self.args
            .get_bool(name)
            .ok_or_else(|| OwnerLoadRequestArgError::missing_or_wrong_type(name, "bool"))
    }

    /// Convert request tags and TTL into local cache options.
    pub fn cache_options(&self) -> CacheOptions {
        let mut options = CacheOptions::new().tags(self.tags.clone());
        if let Some(ttl_ms) = self.ttl_ms {
            options = options.ttl(Duration::from_millis(ttl_ms));
        }
        options
    }
}

/// Error returned by typed owner-load argument accessors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerLoadRequestArgError {
    name: String,
    expected: &'static str,
}

impl OwnerLoadRequestArgError {
    fn missing_or_wrong_type(name: &str, expected: &'static str) -> Self {
        Self {
            name: name.to_owned(),
            expected,
        }
    }

    /// Return the argument name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the expected argument type.
    pub fn expected(&self) -> &'static str {
        self.expected
    }
}

impl fmt::Display for OwnerLoadRequestArgError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "missing owner-load argument '{}' with expected type {}",
            self.name, self.expected
        )
    }
}

impl Error for OwnerLoadRequestArgError {}

/// Successful owner-load value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerLoadHit {
    /// Owner that served the value.
    pub owner: String,
    /// Logical cache key that was served.
    pub key: String,
    /// Registered loader name associated with this response.
    pub loader: String,
    /// Base64-encoded cache bytes.
    pub value_base64: String,
}

impl OwnerLoadHit {
    /// Create a hit from encoded bytes.
    pub fn new(
        owner: impl Into<String>,
        key: impl Into<String>,
        loader: impl Into<String>,
        value: Bytes,
    ) -> Self {
        Self {
            owner: owner.into(),
            key: key.into(),
            loader: loader.into(),
            value_base64: BASE64_STANDARD.encode(value.as_ref()),
        }
    }

    /// Decode the base64 payload into bytes.
    pub fn decode_value(&self) -> CacheResult<Bytes> {
        BASE64_STANDARD
            .decode(&self.value_base64)
            .map(Bytes::from)
            .map_err(|error| CacheError::Decode(format!("invalid owner-load payload: {error}")))
    }
}

/// Owner-load miss response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerLoadMiss {
    /// Owner that served the miss.
    pub owner: String,
    /// Logical cache key that missed.
    pub key: String,
    /// Registered loader name associated with this miss.
    pub loader: String,
}

impl OwnerLoadMiss {
    /// Create a miss response.
    pub fn new(
        owner: impl Into<String>,
        key: impl Into<String>,
        loader: impl Into<String>,
    ) -> Self {
        Self {
            owner: owner.into(),
            key: key.into(),
            loader: loader.into(),
        }
    }
}

/// Stable owner-load rejection code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OwnerLoadRejectionCode {
    /// The ownership decision had no owner.
    NoOwner,
    /// The request reached a member that is not the requested owner.
    WrongOwner,
    /// The request generation is stale or does not match the owner.
    StaleGeneration,
    /// No loader is registered for the requested name.
    MissingLoader,
    /// The request was malformed or incomplete.
    InvalidRequest,
}

/// Owner-load rejection response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerLoadRejection {
    /// Machine-readable rejection code.
    pub code: OwnerLoadRejectionCode,
    /// Human-readable detail.
    pub message: String,
    /// Owner requested by the caller, if available.
    pub requested_owner: Option<String>,
    /// Owner serving the request, if available.
    pub current_owner: Option<String>,
    /// Generation observed by the caller, if available.
    pub requested_generation: Option<u64>,
    /// Current owner generation, if available.
    pub current_generation: Option<u64>,
}

impl OwnerLoadRejection {
    /// Create a rejection with a stable code and message.
    pub fn new(code: OwnerLoadRejectionCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            requested_owner: None,
            current_owner: None,
            requested_generation: None,
            current_generation: None,
        }
    }

    /// Attach requested/current owner metadata.
    pub fn owners(
        mut self,
        requested_owner: impl Into<String>,
        current_owner: impl Into<String>,
    ) -> Self {
        self.requested_owner = Some(requested_owner.into());
        self.current_owner = Some(current_owner.into());
        self
    }

    /// Attach requested/current generation metadata.
    pub fn generations(mut self, requested: Option<u64>, current: Option<u64>) -> Self {
        self.requested_generation = requested;
        self.current_generation = current;
        self
    }
}

/// Owner-load failure response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerLoadFailure {
    /// Stable machine-readable failure code.
    pub code: String,
    /// Human-readable detail.
    pub message: String,
    /// Logical cache key being loaded.
    pub key: String,
    /// Registered loader name that failed.
    pub loader: String,
}

impl OwnerLoadFailure {
    /// Create a failure response.
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        key: impl Into<String>,
        loader: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            key: key.into(),
            loader: loader.into(),
        }
    }
}

/// Transport-neutral owner-load response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "body", rename_all = "kebab-case")]
pub enum OwnerLoadResponse {
    /// Owner already had encoded bytes for the key.
    Hit(OwnerLoadHit),
    /// Owner executed a registered loader and stored the encoded value.
    Loaded(OwnerLoadHit),
    /// Owner or loader intentionally produced no value.
    Miss(OwnerLoadMiss),
    /// Owner rejected the request before running a loader.
    Rejected(OwnerLoadRejection),
    /// Loader or codec failed.
    Failed(OwnerLoadFailure),
}

impl OwnerLoadResponse {
    /// Return whether the response contains encoded bytes.
    pub fn is_hit(&self) -> bool {
        matches!(self, Self::Hit(_) | Self::Loaded(_))
    }

    /// Return whether the response came from an owner-side loader execution.
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    /// Return whether the response is a miss.
    pub fn is_miss(&self) -> bool {
        matches!(self, Self::Miss(_))
    }

    /// Return whether the request was rejected before loader execution.
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected(_))
    }

    /// Return whether the loader or codec failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Decode the response value when this is `Hit` or `Loaded`.
    pub fn decode_value(&self) -> CacheResult<Option<Bytes>> {
        match self {
            Self::Hit(hit) | Self::Loaded(hit) => hit.decode_value().map(Some),
            Self::Miss(_) | Self::Rejected(_) | Self::Failed(_) => Ok(None),
        }
    }
}

/// Encoded value produced by an owner-side loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerLoadValue {
    value: Bytes,
    options: CacheOptions,
}

impl OwnerLoadValue {
    /// Create a loader value from encoded bytes and cache options.
    pub fn encoded(value: impl Into<Bytes>, options: CacheOptions) -> Self {
        Self {
            value: value.into(),
            options,
        }
    }

    /// Return encoded bytes.
    pub fn value(&self) -> &Bytes {
        &self.value
    }

    /// Return cache options used when storing the value on the owner.
    pub fn options(&self) -> &CacheOptions {
        &self.options
    }

    fn into_parts(self) -> (Bytes, CacheOptions) {
        (self.value, self.options)
    }
}

type OwnerLoadFuture = BoxFuture<'static, CacheResult<Option<OwnerLoadValue>>>;
type OwnerLoadHandler = Arc<dyn Fn(OwnerLoadRequest) -> OwnerLoadFuture + Send + Sync>;
type SharedOwnerLoadFuture = Shared<BoxFuture<'static, OwnerLoadResponse>>;

/// Registry of explicitly named owner-side loaders.
///
/// Applications register stable loader names on member nodes. Clients then send
/// data-only [`OwnerLoadDescriptor`] values that reference those names. This
/// keeps remote owner loading explicit and avoids sending closures or arbitrary
/// executable code over the network.
///
/// # Example
///
/// ```no_run
/// use bytes::Bytes;
/// use hydracache::CacheOptions;
/// use hydracache_cluster_transport_axum::{
///     OwnerLoadRegistry, OwnerLoadValue,
/// };
///
/// let registry = OwnerLoadRegistry::new()
///     .register("users.by-id", |request| async move {
///         let id = request.arg_i64("id").unwrap();
///         let encoded = Bytes::from(format!("encoded-user-{id}"));
///         Ok(Some(OwnerLoadValue::encoded(encoded, request.cache_options())))
///     });
///
/// assert!(registry.contains_loader("users.by-id"));
/// ```
#[derive(Clone, Default)]
pub struct OwnerLoadRegistry {
    loaders: BTreeMap<String, OwnerLoadHandler>,
}

impl fmt::Debug for OwnerLoadRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerLoadRegistry")
            .field("loader_count", &self.loaders.len())
            .field("loader_names", &self.loader_names())
            .finish()
    }
}

impl OwnerLoadRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace one named loader.
    pub fn register<F, Fut>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(OwnerLoadRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CacheResult<Option<OwnerLoadValue>>> + Send + 'static,
    {
        let handler = Arc::new(move |request| handler(request).boxed()) as OwnerLoadHandler;
        self.loaders.insert(name.into(), handler);
        self
    }

    /// Return whether a loader name is registered.
    pub fn contains_loader(&self, name: &str) -> bool {
        self.loaders.contains_key(name)
    }

    /// Return the number of registered loaders.
    pub fn len(&self) -> usize {
        self.loaders.len()
    }

    /// Return whether no loaders are registered.
    pub fn is_empty(&self) -> bool {
        self.loaders.is_empty()
    }

    /// Return registered loader names in deterministic order.
    pub fn loader_names(&self) -> Vec<&str> {
        self.loaders.keys().map(String::as_str).collect()
    }

    async fn load(&self, request: OwnerLoadRequest) -> CacheResult<Option<OwnerLoadValue>> {
        let Some(handler) = self.loaders.get(&request.loader).cloned() else {
            return Err(CacheError::Backend(format!(
                "owner-load loader '{}' is not registered",
                request.loader
            )));
        };

        handler(request).await
    }
}

/// Point-in-time counters for [`OwnerLoadService`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OwnerLoadDiagnostics {
    /// Total owner-load requests observed.
    pub attempts: u64,
    /// Requests that hit the owner's local cache.
    pub owner_hits: u64,
    /// Requests that missed the owner's local cache.
    pub owner_misses: u64,
    /// Registered loader executions.
    pub loader_executions: u64,
    /// Requests that joined an already-running owner-side load for the key.
    pub in_flight_joins: u64,
    /// Loader results stored on the owner and returned to callers.
    pub loaded: u64,
    /// Loader results that intentionally produced no value.
    pub misses: u64,
    /// Requests rejected before loader execution.
    pub rejections: u64,
    /// Loader, codec, or local cache failures.
    pub failures: u64,
    /// Successful owner cache stores after loader completion.
    pub stores: u64,
}

impl OwnerLoadDiagnostics {
    /// Return owner hits plus successful owner loads.
    pub fn total_successes(&self) -> u64 {
        self.owner_hits.saturating_add(self.loaded)
    }

    /// Return whether any request failed or was rejected.
    pub fn has_failures(&self) -> bool {
        self.rejections.saturating_add(self.failures) > 0
    }
}

/// Owner-side load service backed by a local [`HydraCache`] and loader registry.
///
/// # Example
///
/// ```no_run
/// use bytes::Bytes;
/// use hydracache::{CacheOptions, ClusterGeneration, HydraCache};
/// use hydracache_cluster_transport_axum::{
///     OwnerLoadRegistry, OwnerLoadRequest, OwnerLoadService, OwnerLoadValue,
/// };
///
/// # async fn example() -> hydracache::CacheResult<()> {
/// let cache = HydraCache::local().build();
/// let registry = OwnerLoadRegistry::new()
///     .register("users.by-id", |request| async move {
///         let id = request.arg_i64("id").unwrap();
///         Ok(Some(OwnerLoadValue::encoded(
///             Bytes::from(format!("encoded-user-{id}")),
///             request.cache_options(),
///         )))
///     });
///
/// let service = OwnerLoadService::new(
///     "member-a",
///     ClusterGeneration::new(1),
///     cache,
///     registry,
/// );
///
/// let request = OwnerLoadRequest {
///     owner: "member-a".to_owned(),
///     key: "user:42".to_owned(),
///     loader: "users.by-id".to_owned(),
///     tags: vec!["user:42".to_owned()],
///     ttl_ms: None,
///     args: Default::default(),
///     generation: Some(1),
///     request_id: "req-1".to_owned(),
/// };
///
/// let _response = service.load(request).await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct OwnerLoadService<C = hydracache::PostcardCodec>
where
    C: CacheCodec,
{
    owner: ClusterNodeId,
    generation: ClusterGeneration,
    cache: HydraCache<C>,
    registry: OwnerLoadRegistry,
    diagnostics: Arc<Mutex<OwnerLoadDiagnostics>>,
    in_flight: Arc<Mutex<BTreeMap<String, SharedOwnerLoadFuture>>>,
}

impl<C> fmt::Debug for OwnerLoadService<C>
where
    C: CacheCodec,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerLoadService")
            .field("owner", &self.owner)
            .field("generation", &self.generation)
            .field("registry", &self.registry)
            .finish_non_exhaustive()
    }
}

impl<C> OwnerLoadService<C>
where
    C: CacheCodec + Send + Sync + 'static,
{
    /// Create an owner-load service for one member.
    pub fn new(
        owner: impl Into<ClusterNodeId>,
        generation: ClusterGeneration,
        cache: HydraCache<C>,
        registry: OwnerLoadRegistry,
    ) -> Self {
        Self {
            owner: owner.into(),
            generation,
            cache,
            registry,
            diagnostics: Arc::new(Mutex::new(OwnerLoadDiagnostics::default())),
            in_flight: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Return the owner node id served by this service.
    pub fn owner(&self) -> &ClusterNodeId {
        &self.owner
    }

    /// Return the owner generation served by this service.
    pub fn generation(&self) -> ClusterGeneration {
        self.generation
    }

    /// Return the local owner cache.
    pub fn cache(&self) -> &HydraCache<C> {
        &self.cache
    }

    /// Return the owner loader registry.
    pub fn registry(&self) -> &OwnerLoadRegistry {
        &self.registry
    }

    /// Return current owner-load diagnostics.
    pub fn diagnostics(&self) -> OwnerLoadDiagnostics {
        *self
            .diagnostics
            .lock()
            .expect("owner-load diagnostics poisoned")
    }

    /// Serve one owner-load request.
    pub async fn load(&self, request: OwnerLoadRequest) -> OwnerLoadResponse {
        Self::record(&self.diagnostics, |diagnostics| {
            diagnostics.attempts = diagnostics.attempts.saturating_add(1);
        });

        if request.owner != self.owner.as_str() {
            return self.reject(
                OwnerLoadRejection::new(
                    OwnerLoadRejectionCode::WrongOwner,
                    format!(
                        "owner-load service serves owner '{}', not '{}'",
                        self.owner, request.owner
                    ),
                )
                .owners(request.owner, self.owner.as_str()),
            );
        }

        if let Some(requested_generation) = request.generation {
            if ClusterGeneration::new(requested_generation) != self.generation {
                return self.reject(
                    OwnerLoadRejection::new(
                        OwnerLoadRejectionCode::StaleGeneration,
                        format!(
                            "requested owner generation {} does not match current generation {}",
                            requested_generation,
                            self.generation.value()
                        ),
                    )
                    .generations(Some(requested_generation), Some(self.generation.value())),
                );
            }
        }

        match self.cache.get_encoded(&request.key).await {
            Ok(Some(value)) => {
                Self::record(&self.diagnostics, |diagnostics| {
                    diagnostics.owner_hits = diagnostics.owner_hits.saturating_add(1);
                });
                return OwnerLoadResponse::Hit(OwnerLoadHit::new(
                    self.owner.as_str(),
                    request.key,
                    request.loader,
                    value,
                ));
            }
            Ok(None) => {
                Self::record(&self.diagnostics, |diagnostics| {
                    diagnostics.owner_misses = diagnostics.owner_misses.saturating_add(1);
                });
            }
            Err(error) => {
                return self.fail(
                    "cache-read-error",
                    error.to_string(),
                    request.key,
                    request.loader,
                );
            }
        }

        if !self.registry.contains_loader(&request.loader) {
            return self.reject(OwnerLoadRejection::new(
                OwnerLoadRejectionCode::MissingLoader,
                format!("owner-load loader '{}' is not registered", request.loader),
            ));
        }

        self.load_shared(request).await
    }

    async fn load_shared(&self, request: OwnerLoadRequest) -> OwnerLoadResponse {
        let key = request.key.clone();
        let shared = {
            let mut in_flight = self
                .in_flight
                .lock()
                .expect("owner-load in-flight map poisoned");
            if let Some(shared) = in_flight.get(&key) {
                Self::record(&self.diagnostics, |diagnostics| {
                    diagnostics.in_flight_joins = diagnostics.in_flight_joins.saturating_add(1);
                });
                shared.clone()
            } else {
                let cache = self.cache.clone();
                let registry = self.registry.clone();
                let diagnostics = self.diagnostics.clone();
                let owner = self.owner.clone();
                let shared = async move {
                    Self::load_once(cache, registry, diagnostics, owner, request).await
                }
                .boxed()
                .shared();
                in_flight.insert(key.clone(), shared.clone());
                shared
            }
        };

        let response = shared.await;
        self.in_flight
            .lock()
            .expect("owner-load in-flight map poisoned")
            .remove(&key);
        response
    }

    async fn load_once(
        cache: HydraCache<C>,
        registry: OwnerLoadRegistry,
        diagnostics: Arc<Mutex<OwnerLoadDiagnostics>>,
        owner: ClusterNodeId,
        request: OwnerLoadRequest,
    ) -> OwnerLoadResponse {
        Self::record(&diagnostics, |diagnostics| {
            diagnostics.loader_executions = diagnostics.loader_executions.saturating_add(1);
        });

        match registry.load(request.clone()).await {
            Ok(Some(value)) => {
                let (bytes, options) = value.into_parts();
                match cache
                    .put_encoded(&request.key, bytes.clone(), options)
                    .await
                {
                    Ok(()) => {
                        Self::record(&diagnostics, |diagnostics| {
                            diagnostics.loaded = diagnostics.loaded.saturating_add(1);
                            diagnostics.stores = diagnostics.stores.saturating_add(1);
                        });
                        OwnerLoadResponse::Loaded(OwnerLoadHit::new(
                            owner.as_str(),
                            request.key,
                            request.loader,
                            bytes,
                        ))
                    }
                    Err(error) => {
                        Self::record(&diagnostics, |diagnostics| {
                            diagnostics.failures = diagnostics.failures.saturating_add(1);
                        });
                        OwnerLoadResponse::Failed(OwnerLoadFailure::new(
                            "cache-store-error",
                            error.to_string(),
                            request.key,
                            request.loader,
                        ))
                    }
                }
            }
            Ok(None) => {
                Self::record(&diagnostics, |diagnostics| {
                    diagnostics.misses = diagnostics.misses.saturating_add(1);
                });
                OwnerLoadResponse::Miss(OwnerLoadMiss::new(
                    owner.as_str(),
                    request.key,
                    request.loader,
                ))
            }
            Err(error) => {
                Self::record(&diagnostics, |diagnostics| {
                    diagnostics.failures = diagnostics.failures.saturating_add(1);
                });
                OwnerLoadResponse::Failed(OwnerLoadFailure::new(
                    "loader-error",
                    error.to_string(),
                    request.key,
                    request.loader,
                ))
            }
        }
    }

    fn reject(&self, rejection: OwnerLoadRejection) -> OwnerLoadResponse {
        Self::record(&self.diagnostics, |diagnostics| {
            diagnostics.rejections = diagnostics.rejections.saturating_add(1);
        });
        OwnerLoadResponse::Rejected(rejection)
    }

    fn fail(
        &self,
        code: impl Into<String>,
        message: impl Into<String>,
        key: String,
        loader: String,
    ) -> OwnerLoadResponse {
        Self::record(&self.diagnostics, |diagnostics| {
            diagnostics.failures = diagnostics.failures.saturating_add(1);
        });
        OwnerLoadResponse::Failed(OwnerLoadFailure::new(code, message, key, loader))
    }

    fn record(
        diagnostics: &Arc<Mutex<OwnerLoadDiagnostics>>,
        update: impl FnOnce(&mut OwnerLoadDiagnostics),
    ) {
        let mut diagnostics = diagnostics.lock().expect("owner-load diagnostics poisoned");
        update(&mut diagnostics);
    }
}

#[derive(Clone)]
struct OwnerLoadHttpState<C>
where
    C: CacheCodec,
{
    service: OwnerLoadService<C>,
    auth: Option<HttpTransportAuth>,
    wire: HttpWireCompatibility,
}

/// Axum route factory for serving owner-side load requests from one member.
#[derive(Clone)]
pub struct AxumOwnerLoadService<C = hydracache::PostcardCodec>
where
    C: CacheCodec,
{
    service: OwnerLoadService<C>,
    auth: Option<HttpTransportAuth>,
    wire: HttpWireCompatibility,
}

impl<C> fmt::Debug for AxumOwnerLoadService<C>
where
    C: CacheCodec,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AxumOwnerLoadService")
            .field("owner", &self.service.owner())
            .field("generation", &self.service.generation())
            .field("auth_enabled", &self.auth.is_some())
            .field("wire", &self.wire)
            .finish_non_exhaustive()
    }
}

impl<C> AxumOwnerLoadService<C>
where
    C: CacheCodec + Send + Sync + 'static,
{
    /// Create an Axum owner-load route factory.
    pub fn new(service: OwnerLoadService<C>) -> Self {
        Self {
            service,
            auth: None,
            wire: HttpWireCompatibility::default(),
        }
    }

    /// Require an auth header before owner-load requests reach the service.
    pub fn with_auth(self, auth: HttpTransportAuth) -> Self {
        Self {
            service: self.service,
            auth: Some(auth),
            wire: self.wire,
        }
    }

    /// Set the HTTP wire compatibility policy for this route factory.
    pub fn with_wire_compatibility(self, wire: HttpWireCompatibility) -> Self {
        Self {
            service: self.service,
            auth: self.auth,
            wire,
        }
    }

    /// Return the underlying owner-load service.
    pub fn service(&self) -> &OwnerLoadService<C> {
        &self.service
    }

    /// Return the configured auth boundary, if any.
    pub fn auth(&self) -> Option<&HttpTransportAuth> {
        self.auth.as_ref()
    }

    /// Return the configured wire compatibility policy.
    pub fn wire(&self) -> HttpWireCompatibility {
        self.wire
    }

    /// Build the Axum router with `POST /cluster/owner-load`.
    pub fn routes(&self) -> Router {
        Router::new()
            .route(DEFAULT_OWNER_LOAD_PATH, post(handle_owner_load::<C>))
            .with_state(OwnerLoadHttpState {
                service: self.service.clone(),
                auth: self.auth.clone(),
                wire: self.wire,
            })
    }
}

async fn handle_owner_load<C>(
    State(state): State<OwnerLoadHttpState<C>>,
    headers: HeaderMap,
    Json(request): Json<OwnerLoadRequest>,
) -> Response
where
    C: CacheCodec + Send + Sync + 'static,
{
    if let Err(rejection) = validate_http_transport(&headers, state.auth.as_ref(), state.wire) {
        return owner_load_transport_rejection(rejection);
    }

    let response = state.service.load(request).await;
    (owner_load_http_status(&response), Json(response)).into_response()
}

fn owner_load_transport_rejection(rejection: HttpTransportRejection) -> Response {
    let response = OwnerLoadResponse::Rejected(OwnerLoadRejection::new(
        OwnerLoadRejectionCode::InvalidRequest,
        rejection.message,
    ));
    (rejection.status, Json(response)).into_response()
}

fn owner_load_http_status(response: &OwnerLoadResponse) -> StatusCode {
    match response {
        OwnerLoadResponse::Hit(_) | OwnerLoadResponse::Loaded(_) | OwnerLoadResponse::Miss(_) => {
            StatusCode::OK
        }
        OwnerLoadResponse::Rejected(rejection) => match rejection.code {
            OwnerLoadRejectionCode::StaleGeneration => StatusCode::CONFLICT,
            OwnerLoadRejectionCode::InvalidRequest => StatusCode::BAD_REQUEST,
            OwnerLoadRejectionCode::NoOwner
            | OwnerLoadRejectionCode::WrongOwner
            | OwnerLoadRejectionCode::MissingLoader => StatusCode::NOT_FOUND,
        },
        OwnerLoadResponse::Failed(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// HTTP client for owner-side load requests.
///
/// # Example
///
/// ```no_run
/// use hydracache_cluster_transport_axum::{HttpOwnerLoad, OwnerLoadRequest};
///
/// # async fn example(request: OwnerLoadRequest) -> hydracache::CacheResult<()> {
/// let client = HttpOwnerLoad::for_base_url("http://127.0.0.1:3000");
/// let _response = client.load(request).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct HttpOwnerLoad {
    endpoint: String,
    client: reqwest::Client,
    auth: Option<HttpTransportAuth>,
    wire: HttpWireCompatibility,
}

impl HttpOwnerLoad {
    /// Create an owner-load client for a full endpoint URL.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::new(),
            auth: None,
            wire: HttpWireCompatibility::default(),
        }
    }

    /// Create an owner-load client from a member base URL.
    ///
    /// `DEFAULT_OWNER_LOAD_PATH` is appended after trimming a trailing slash.
    pub fn for_base_url(base_url: impl AsRef<str>) -> Self {
        let base_url = base_url.as_ref().trim_end_matches('/');
        Self::new(format!("{base_url}{DEFAULT_OWNER_LOAD_PATH}"))
    }

    /// Return the endpoint URL used by this client.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Attach an auth header to every owner-load request.
    pub fn with_auth(mut self, auth: HttpTransportAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set the wire version header sent with owner-load requests.
    pub fn with_wire_compatibility(mut self, wire: HttpWireCompatibility) -> Self {
        self.wire = wire;
        self
    }

    /// Send one owner-load request.
    pub async fn load(&self, request: OwnerLoadRequest) -> CacheResult<OwnerLoadResponse> {
        let expected_owner = request.owner.clone();
        let expected_key = request.key.clone();
        let mut builder = self.client.post(&self.endpoint).json(&request);
        builder = self.wire.apply(builder);
        if let Some(auth) = &self.auth {
            builder = auth.apply(builder);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| CacheError::Backend(format!("owner-load request failed: {error}")))?;

        let status = response.status();
        let response = response
            .json::<OwnerLoadResponse>()
            .await
            .map_err(|error| {
                CacheError::Decode(format!(
                    "invalid owner-load response with HTTP status {status}: {error}"
                ))
            })?;

        validate_owner_load_identity(&response, &expected_owner, &expected_key)?;
        Ok(response)
    }
}

fn validate_owner_load_identity(
    response: &OwnerLoadResponse,
    expected_owner: &str,
    expected_key: &str,
) -> CacheResult<()> {
    match response {
        OwnerLoadResponse::Hit(hit) | OwnerLoadResponse::Loaded(hit) => {
            if hit.owner != expected_owner || hit.key != expected_key {
                return Err(CacheError::Backend(format!(
                    "owner-load response identity mismatch: expected owner/key '{expected_owner}/{expected_key}', got '{}/{}'",
                    hit.owner, hit.key
                )));
            }
        }
        OwnerLoadResponse::Miss(miss) => {
            if miss.owner != expected_owner || miss.key != expected_key {
                return Err(CacheError::Backend(format!(
                    "owner-load miss identity mismatch: expected owner/key '{expected_owner}/{expected_key}', got '{}/{}'",
                    miss.owner, miss.key
                )));
            }
        }
        OwnerLoadResponse::Rejected(_) | OwnerLoadResponse::Failed(_) => {}
    }

    Ok(())
}

fn duration_to_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

/// Outcome status produced by [`PeerFetchRouter`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerFetchRouterStatus {
    /// The ownership decision had no eligible owner.
    NoOwner,
    /// The owner did not advertise a peer-fetch endpoint.
    MissingEndpoint,
    /// The owner returned encoded bytes.
    Hit,
    /// The owner was reachable but did not have the value.
    Miss,
    /// The owner rejected the request because the observed generation is stale.
    GenerationMismatch,
    /// The transport request failed or returned an unexpected response.
    TransportError,
}

/// Result of routing one ownership decision through a peer-fetch transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerFetchRouterOutcome {
    /// Logical cache key being fetched.
    pub key: String,
    /// Owner selected by the ownership resolver, when one exists.
    pub owner: Option<ClusterNodeId>,
    /// Full peer-fetch endpoint used by the router, when available.
    pub endpoint: Option<String>,
    /// Terminal route status.
    pub status: PeerFetchRouterStatus,
    /// Encoded value returned by the owner on hit.
    pub value: Option<Bytes>,
    /// Human-readable transport or routing error detail.
    pub error: Option<String>,
}

impl PeerFetchRouterOutcome {
    fn new(
        key: String,
        owner: Option<ClusterNodeId>,
        endpoint: Option<String>,
        status: PeerFetchRouterStatus,
        value: Option<Bytes>,
        error: Option<String>,
    ) -> Self {
        Self {
            key,
            owner,
            endpoint,
            status,
            value,
            error,
        }
    }

    /// Return whether the routed request returned a value.
    pub fn is_hit(&self) -> bool {
        self.status == PeerFetchRouterStatus::Hit
    }

    /// Return whether the owner was reached but did not have the value.
    pub fn is_miss(&self) -> bool {
        self.status == PeerFetchRouterStatus::Miss
    }

    /// Return whether the router did not issue an HTTP request.
    pub fn did_not_route(&self) -> bool {
        matches!(
            self.status,
            PeerFetchRouterStatus::NoOwner | PeerFetchRouterStatus::MissingEndpoint
        )
    }
}

/// Point-in-time counters for [`PeerFetchRouter`].
///
/// These counters are intentionally small and copyable so they can be exported
/// through application diagnostics, actuator endpoints, or sandbox reports
/// without holding the router lock.
///
/// # Example
///
/// ```no_run
/// use hydracache::{ClusterCandidate, InMemoryCluster};
/// use hydracache_cluster_transport_axum::PeerFetchRouter;
///
/// # async fn example() -> hydracache::CacheResult<()> {
/// let router = PeerFetchRouter::new();
///
/// let empty = InMemoryCluster::new("orders");
/// let no_owner = router.fetch_owner_value(empty.owner_for_key("user:42")).await;
/// assert!(no_owner.did_not_route());
///
/// let cluster = InMemoryCluster::new("orders");
/// cluster.join_member(ClusterCandidate::member("member-a"))?;
/// let missing_endpoint = router
///     .fetch_owner_value(cluster.owner_for_key("user:42"))
///     .await;
/// assert!(missing_endpoint.did_not_route());
///
/// let diagnostics = router.diagnostics();
/// assert_eq!(diagnostics.attempts, 2);
/// assert_eq!(diagnostics.no_owner, 1);
/// assert_eq!(diagnostics.missing_endpoint, 1);
/// assert_eq!(diagnostics.routed_requests(), 0);
/// assert!(diagnostics.has_failures());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PeerFetchRouterDiagnostics {
    /// Total routing calls observed.
    pub attempts: u64,
    /// Routed requests that returned encoded bytes.
    pub hits: u64,
    /// Routed requests that reached the owner and missed.
    pub misses: u64,
    /// Calls where ownership had no eligible member.
    pub no_owner: u64,
    /// Calls where the owner did not advertise a peer-fetch endpoint.
    pub missing_endpoint: u64,
    /// Calls rejected due to stale owner generation.
    pub generation_mismatches: u64,
    /// Calls that failed at the HTTP transport layer.
    pub transport_errors: u64,
}

impl PeerFetchRouterDiagnostics {
    /// Return hit + miss routed requests.
    pub fn routed_requests(&self) -> u64 {
        self.hits.saturating_add(self.misses)
    }

    /// Return whether any routing failures were observed.
    pub fn has_failures(&self) -> bool {
        self.no_owner
            .saturating_add(self.missing_endpoint)
            .saturating_add(self.generation_mismatches)
            .saturating_add(self.transport_errors)
            > 0
    }
}

/// Routes ownership decisions to an advertised HTTP peer-fetch endpoint.
#[derive(Debug, Clone, Default)]
pub struct PeerFetchRouter {
    diagnostics: Arc<Mutex<PeerFetchRouterDiagnostics>>,
}

impl PeerFetchRouter {
    /// Create a router with empty diagnostics.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hydracache::{ClusterCandidate, ClusterGeneration, InMemoryCluster};
    /// use hydracache_cluster_transport_axum::{
    ///     PeerFetchRouter, PeerFetchRouterStatus,
    /// };
    ///
    /// # async fn example() -> hydracache::CacheResult<()> {
    /// let cluster = InMemoryCluster::new("orders");
    /// cluster.join_member(
    ///     ClusterCandidate::member("member-a")
    ///         .generation(ClusterGeneration::new(1))
    ///         .peer_fetch_base_url("http://127.0.0.1:3000"),
    /// )?;
    ///
    /// let outcome = PeerFetchRouter::new()
    ///     .fetch_owner_value(cluster.owner_for_key("user:42"))
    ///     .await;
    ///
    /// assert!(matches!(
    ///     outcome.status,
    ///     PeerFetchRouterStatus::Hit
    ///         | PeerFetchRouterStatus::Miss
    ///         | PeerFetchRouterStatus::TransportError
    /// ));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Route an ownership decision through the owner's advertised endpoint.
    pub async fn fetch_owner_value(
        &self,
        decision: ClusterOwnershipDecision,
    ) -> PeerFetchRouterOutcome {
        self.record(|diagnostics| {
            diagnostics.attempts = diagnostics.attempts.saturating_add(1);
        });

        let key = decision.key.clone();
        let Some(owner) = decision.owner.clone() else {
            self.record(|diagnostics| {
                diagnostics.no_owner = diagnostics.no_owner.saturating_add(1);
            });
            return PeerFetchRouterOutcome::new(
                key,
                None,
                None,
                PeerFetchRouterStatus::NoOwner,
                None,
                Some("ownership decision did not select an owner".to_owned()),
            );
        };

        let Some(base_url) = owner.peer_fetch_base_url() else {
            self.record(|diagnostics| {
                diagnostics.missing_endpoint = diagnostics.missing_endpoint.saturating_add(1);
            });
            return PeerFetchRouterOutcome::new(
                key,
                Some(owner.node_id),
                None,
                PeerFetchRouterStatus::MissingEndpoint,
                None,
                Some("owner did not advertise a peer-fetch base URL".to_owned()),
            );
        };

        let peer_fetch = HttpPeerFetch::for_base_url(base_url);
        let endpoint = peer_fetch.endpoint().to_owned();
        let request = ClusterPeerFetchRequest::new(owner.node_id.clone(), decision.key)
            .generation(owner.generation);

        match peer_fetch.fetch(request).await {
            Ok(response) if response.is_hit() => {
                self.record(|diagnostics| {
                    diagnostics.hits = diagnostics.hits.saturating_add(1);
                });
                PeerFetchRouterOutcome::new(
                    key,
                    Some(response.owner),
                    Some(endpoint),
                    PeerFetchRouterStatus::Hit,
                    response.value,
                    None,
                )
            }
            Ok(response) => {
                self.record(|diagnostics| {
                    diagnostics.misses = diagnostics.misses.saturating_add(1);
                });
                PeerFetchRouterOutcome::new(
                    key,
                    Some(response.owner),
                    Some(endpoint),
                    PeerFetchRouterStatus::Miss,
                    None,
                    None,
                )
            }
            Err(error) => {
                let message = error.to_string();
                let status = if message.contains("generation-mismatch") {
                    self.record(|diagnostics| {
                        diagnostics.generation_mismatches =
                            diagnostics.generation_mismatches.saturating_add(1);
                    });
                    PeerFetchRouterStatus::GenerationMismatch
                } else {
                    self.record(|diagnostics| {
                        diagnostics.transport_errors =
                            diagnostics.transport_errors.saturating_add(1);
                    });
                    PeerFetchRouterStatus::TransportError
                };
                PeerFetchRouterOutcome::new(
                    key,
                    Some(owner.node_id),
                    Some(endpoint),
                    status,
                    None,
                    Some(message),
                )
            }
        }
    }

    /// Return current router diagnostics.
    pub fn diagnostics(&self) -> PeerFetchRouterDiagnostics {
        *self.diagnostics.lock().expect("peer-fetch router poisoned")
    }

    fn record(&self, update: impl FnOnce(&mut PeerFetchRouterDiagnostics)) {
        let mut diagnostics = self.diagnostics.lock().expect("peer-fetch router poisoned");
        update(&mut diagnostics);
    }
}

/// Read-through policy for local/near-cache and owner peer-fetch ordering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PeerFetchReadThroughPolicy {
    /// Check the local cache first, then route to the owner on miss.
    #[default]
    LocalThenOwner,
    /// Route to the owner first, then fall back to the local cache on miss/error.
    OwnerThenLocal,
    /// Route only to the owner. Remote hits may still hydrate the local cache.
    OwnerOnly,
}

/// Bounded policy for remote values hydrated into a read-through near-cache.
///
/// This policy is intentionally scoped to [`PeerFetchReadThrough`]. The base
/// [`HydraCache`] runtime does not know whether a caller inserted an owned
/// value or remote owner bytes; the read-through helper does, so it owns this
/// pressure-control surface.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use hydracache_cluster_transport_axum::HotRemoteCachePolicy;
///
/// let policy = HotRemoteCachePolicy::new()
///     .ttl(Duration::from_secs(5))
///     .max_entries(1_000);
///
/// assert!(policy.is_enabled());
/// assert_eq!(policy.ttl_value(), Some(Duration::from_secs(5)));
/// assert_eq!(policy.max_entries_value(), Some(1_000));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotRemoteCachePolicy {
    enabled: bool,
    ttl: Option<Duration>,
    max_entries: Option<usize>,
}

impl Default for HotRemoteCachePolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl HotRemoteCachePolicy {
    /// Create a policy that preserves the previous read-through behavior:
    /// hydrate remote hits, keep caller-provided TTLs, and do not enforce an
    /// additional remote-entry capacity limit.
    pub const fn new() -> Self {
        Self {
            enabled: true,
            ttl: None,
            max_entries: None,
        }
    }

    /// Create a policy that never hydrates remote owner values locally.
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            ttl: None,
            max_entries: None,
        }
    }

    /// Enable or disable remote hydration.
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set a TTL override used only for remote hydrated entries.
    pub const fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Set a maximum number of tracked remote entries.
    ///
    /// `0` is treated as disabled hydration because a near-cache with no
    /// remote slots would immediately evict every hydrated value.
    pub const fn max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = Some(max_entries);
        self
    }

    /// Return whether remote hydration is enabled and has at least one slot.
    pub const fn is_enabled(&self) -> bool {
        self.enabled && !matches!(self.max_entries, Some(0))
    }

    /// Return the optional remote-entry TTL override.
    pub const fn ttl_value(&self) -> Option<Duration> {
        self.ttl
    }

    /// Return the optional remote-entry capacity limit.
    pub const fn max_entries_value(&self) -> Option<usize> {
        self.max_entries
    }

    fn cache_options(&self, options: CacheOptions) -> CacheOptions {
        match self.ttl {
            Some(ttl) => options.ttl(ttl),
            None => options,
        }
    }
}

/// Point-in-time diagnostics for the hot-remote layer inside
/// [`PeerFetchReadThrough`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HotRemoteCacheDiagnostics {
    /// Whether remote hydration is currently allowed.
    pub enabled: bool,
    /// Optional TTL override in milliseconds for remote hydrated entries.
    pub ttl_millis: Option<u64>,
    /// Optional maximum number of tracked remote entries.
    pub max_entries: Option<usize>,
    /// Number of remote keys currently tracked by this helper.
    pub tracked_entries: usize,
    /// Total remote values stored into the local near-cache.
    pub hydrations: u64,
    /// Hydrations skipped because remote hydration was disabled.
    pub skipped_hydrations: u64,
    /// Remote entries removed because the hot-remote capacity was exceeded.
    pub pressure_evictions: u64,
}

impl HotRemoteCacheDiagnostics {
    /// Return whether the policy is currently enforcing a finite capacity.
    pub fn is_bounded(&self) -> bool {
        self.max_entries.is_some()
    }

    /// Return whether the helper has had to evict remote entries under
    /// pressure.
    pub fn has_pressure_evictions(&self) -> bool {
        self.pressure_evictions > 0
    }
}

#[derive(Debug, Default)]
struct HotRemoteCacheState {
    tracked: BTreeMap<String, u64>,
    next_sequence: u64,
    hydrations: u64,
    skipped_hydrations: u64,
    pressure_evictions: u64,
}

impl HotRemoteCacheState {
    fn snapshot(&self, policy: HotRemoteCachePolicy) -> HotRemoteCacheDiagnostics {
        HotRemoteCacheDiagnostics {
            enabled: policy.is_enabled(),
            ttl_millis: policy
                .ttl_value()
                .map(|ttl| ttl.as_millis().min(u128::from(u64::MAX)) as u64),
            max_entries: policy.max_entries_value(),
            tracked_entries: self.tracked.len(),
            hydrations: self.hydrations,
            skipped_hydrations: self.skipped_hydrations,
            pressure_evictions: self.pressure_evictions,
        }
    }

    fn record_skipped_hydration(&mut self) {
        self.skipped_hydrations = self.skipped_hydrations.saturating_add(1);
    }

    fn record_hydration(&mut self, key: String, policy: HotRemoteCachePolicy) -> Vec<String> {
        self.hydrations = self.hydrations.saturating_add(1);
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.tracked.insert(key, self.next_sequence);

        let Some(max_entries) = policy.max_entries_value() else {
            return Vec::new();
        };

        let mut evicted = Vec::new();
        while self.tracked.len() > max_entries {
            let Some((oldest_key, _)) = self
                .tracked
                .iter()
                .min_by_key(|(_, sequence)| **sequence)
                .map(|(key, sequence)| (key.clone(), *sequence))
            else {
                break;
            };

            self.tracked.remove(&oldest_key);
            self.pressure_evictions = self.pressure_evictions.saturating_add(1);
            evicted.push(oldest_key);
        }

        evicted
    }
}

/// Outcome status produced by [`PeerFetchReadThrough`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerFetchReadThroughStatus {
    /// The value was already available in the local/near cache.
    LocalHit,
    /// The owner returned encoded bytes.
    RemoteHit,
    /// The owner was reachable but did not have the value.
    RemoteMiss,
    /// The ownership decision had no eligible owner.
    NoOwner,
    /// The owner did not advertise a peer-fetch endpoint.
    MissingEndpoint,
    /// The owner rejected the request because the observed generation is stale.
    GenerationMismatch,
    /// The transport request failed or returned an unexpected response.
    TransportError,
}

/// Result of one read-through attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerFetchReadThroughOutcome {
    /// Logical cache key being fetched.
    pub key: String,
    /// Policy used for this read-through attempt.
    pub policy: PeerFetchReadThroughPolicy,
    /// Terminal read-through status.
    pub status: PeerFetchReadThroughStatus,
    /// Owner selected by the ownership resolver, when one exists.
    pub owner: Option<ClusterNodeId>,
    /// Full peer-fetch endpoint used by the router, when available.
    pub endpoint: Option<String>,
    /// Encoded value returned by local cache or owner on hit.
    pub value: Option<Bytes>,
    /// Whether a remote hit was stored into the local cache.
    pub hydrated: bool,
    /// Human-readable routing error detail.
    pub error: Option<String>,
}

impl PeerFetchReadThroughOutcome {
    /// Return whether the read-through attempt returned encoded bytes.
    pub fn is_hit(&self) -> bool {
        matches!(
            self.status,
            PeerFetchReadThroughStatus::LocalHit | PeerFetchReadThroughStatus::RemoteHit
        )
    }

    /// Return whether the hit came from the local/near cache.
    pub fn is_local_hit(&self) -> bool {
        self.status == PeerFetchReadThroughStatus::LocalHit
    }

    /// Return whether the hit came from the owner peer-fetch route.
    pub fn is_remote_hit(&self) -> bool {
        self.status == PeerFetchReadThroughStatus::RemoteHit
    }

    /// Return whether the owner was reached but did not have the value.
    pub fn is_remote_miss(&self) -> bool {
        self.status == PeerFetchReadThroughStatus::RemoteMiss
    }

    /// Return whether the attempt ended in a routing/transport problem.
    pub fn is_router_error(&self) -> bool {
        matches!(
            self.status,
            PeerFetchReadThroughStatus::NoOwner
                | PeerFetchReadThroughStatus::MissingEndpoint
                | PeerFetchReadThroughStatus::GenerationMismatch
                | PeerFetchReadThroughStatus::TransportError
        )
    }
}

/// Point-in-time counters for [`PeerFetchReadThrough`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PeerFetchReadThroughDiagnostics {
    /// Total read-through calls observed.
    pub attempts: u64,
    /// Calls that found the value in the local/near cache.
    pub local_hits: u64,
    /// Local cache misses before a remote route attempt.
    pub local_misses: u64,
    /// Owner-routed calls that returned encoded bytes.
    pub remote_hits: u64,
    /// Owner-routed calls that reached the owner and missed.
    pub remote_misses: u64,
    /// Remote hits stored into the local/near cache.
    pub hydrations: u64,
    /// Calls that joined an already-running same-key remote route.
    pub in_flight_joins: u64,
    /// Owner routing calls that ended in no-owner, missing-endpoint, generation,
    /// or transport errors.
    pub router_errors: u64,
    /// Reserved for future local loader fallback helpers.
    pub fallback_loads: u64,
}

impl PeerFetchReadThroughDiagnostics {
    /// Return local + remote hits.
    pub fn total_hits(&self) -> u64 {
        self.local_hits.saturating_add(self.remote_hits)
    }

    /// Return local + remote misses.
    pub fn total_misses(&self) -> u64 {
        self.local_misses.saturating_add(self.remote_misses)
    }

    /// Return whether any router errors were observed.
    pub fn has_router_errors(&self) -> bool {
        self.router_errors > 0
    }
}

/// Outcome status produced by owner-load read-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerLoadReadThroughStatus {
    /// The value was already available in the local/near cache.
    LocalHit,
    /// The owner already had encoded bytes.
    RemoteHit,
    /// The owner executed a registered loader and returned encoded bytes.
    RemoteLoaded,
    /// The owner or registered loader produced no value.
    RemoteMiss,
    /// The ownership decision had no eligible owner.
    NoOwner,
    /// The owner did not advertise an owner-load endpoint.
    MissingEndpoint,
    /// The owner rejected the request.
    Rejected,
    /// The owner reported a loader/cache failure.
    Failed,
    /// The HTTP transport failed or returned an invalid response.
    TransportError,
}

/// Result of one owner-load read-through attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerLoadReadThroughOutcome {
    /// Logical cache key being loaded.
    pub key: String,
    /// Policy used for this read-through attempt.
    pub policy: PeerFetchReadThroughPolicy,
    /// Terminal owner-load read-through status.
    pub status: OwnerLoadReadThroughStatus,
    /// Owner selected by the ownership resolver, when one exists.
    pub owner: Option<ClusterNodeId>,
    /// Full owner-load endpoint used by the helper, when available.
    pub endpoint: Option<String>,
    /// Structured owner-load response, when an owner route was reached.
    pub response: Option<OwnerLoadResponse>,
    /// Encoded bytes returned by local cache or owner on hit/load.
    pub value: Option<Bytes>,
    /// Whether remote bytes were stored into the local/near cache.
    pub hydrated: bool,
    /// Human-readable routing or transport detail.
    pub error: Option<String>,
}

impl OwnerLoadReadThroughOutcome {
    /// Return whether the read-through attempt returned encoded bytes.
    pub fn is_hit(&self) -> bool {
        matches!(
            self.status,
            OwnerLoadReadThroughStatus::LocalHit
                | OwnerLoadReadThroughStatus::RemoteHit
                | OwnerLoadReadThroughStatus::RemoteLoaded
        )
    }

    /// Return whether the value came from a remote owner-side loader.
    pub fn is_remote_loaded(&self) -> bool {
        self.status == OwnerLoadReadThroughStatus::RemoteLoaded
    }

    /// Return whether the owner route produced no value.
    pub fn is_remote_miss(&self) -> bool {
        self.status == OwnerLoadReadThroughStatus::RemoteMiss
    }

    /// Return whether this outcome represents a routing or transport problem.
    pub fn is_route_error(&self) -> bool {
        matches!(
            self.status,
            OwnerLoadReadThroughStatus::NoOwner
                | OwnerLoadReadThroughStatus::MissingEndpoint
                | OwnerLoadReadThroughStatus::TransportError
        )
    }
}

/// Point-in-time counters for owner-load read-through calls.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OwnerLoadReadThroughDiagnostics {
    /// Total owner-load read-through calls observed.
    pub attempts: u64,
    /// Calls that found the value in the local/near cache.
    pub local_hits: u64,
    /// Local cache misses before owner-load routing.
    pub local_misses: u64,
    /// Owner-routed calls that returned an existing owner value.
    pub remote_hits: u64,
    /// Owner-routed calls that executed a loader and returned a value.
    pub remote_loaded: u64,
    /// Owner-routed calls that produced no value.
    pub remote_misses: u64,
    /// Remote values stored into the local/near cache.
    pub hydrations: u64,
    /// Calls that joined an already-running same-key owner-load route.
    pub in_flight_joins: u64,
    /// Calls with no owner, missing endpoint, or malformed descriptor.
    pub routing_errors: u64,
    /// Owner-side request rejections.
    pub rejections: u64,
    /// Owner-side loader/cache failures.
    pub failures: u64,
    /// HTTP transport failures.
    pub transport_errors: u64,
}

impl OwnerLoadReadThroughDiagnostics {
    /// Return local + remote successful value outcomes.
    pub fn total_hits(&self) -> u64 {
        self.local_hits
            .saturating_add(self.remote_hits)
            .saturating_add(self.remote_loaded)
    }

    /// Return whether any non-miss problem was observed.
    pub fn has_errors(&self) -> bool {
        self.routing_errors
            .saturating_add(self.rejections)
            .saturating_add(self.failures)
            .saturating_add(self.transport_errors)
            > 0
    }
}

type SharedReadThroughFuture = Shared<BoxFuture<'static, CacheResult<PeerFetchReadThroughOutcome>>>;
type SharedOwnerLoadReadThroughFuture =
    Shared<BoxFuture<'static, CacheResult<OwnerLoadReadThroughOutcome>>>;

struct ReadThroughExecution<C>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    router: PeerFetchRouter,
    diagnostics: Arc<Mutex<PeerFetchReadThroughDiagnostics>>,
    policy: PeerFetchReadThroughPolicy,
    hydrate_remote_hits: bool,
    hot_remote_policy: HotRemoteCachePolicy,
    hot_remote: Arc<Mutex<HotRemoteCacheState>>,
}

struct OwnerLoadReadThroughExecution<C>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    diagnostics: Arc<Mutex<OwnerLoadReadThroughDiagnostics>>,
    policy: PeerFetchReadThroughPolicy,
    hydrate_remote_hits: bool,
    hot_remote_policy: HotRemoteCachePolicy,
    hot_remote: Arc<Mutex<HotRemoteCacheState>>,
}

/// Local/near-cache read-through helper backed by [`PeerFetchRouter`].
///
/// The helper checks a local cache according to a [`PeerFetchReadThroughPolicy`],
/// routes misses to the advertised owner endpoint, and hydrates the local cache
/// with encoded bytes returned by the owner.
#[derive(Debug)]
pub struct PeerFetchReadThrough<C = hydracache::PostcardCodec>
where
    C: CacheCodec,
{
    cache: HydraCache<C>,
    router: PeerFetchRouter,
    policy: PeerFetchReadThroughPolicy,
    hydrate_remote_hits: bool,
    hot_remote_policy: HotRemoteCachePolicy,
    hot_remote: Arc<Mutex<HotRemoteCacheState>>,
    diagnostics: Arc<Mutex<PeerFetchReadThroughDiagnostics>>,
    in_flight: Arc<Mutex<BTreeMap<String, SharedReadThroughFuture>>>,
    owner_load_diagnostics: Arc<Mutex<OwnerLoadReadThroughDiagnostics>>,
    owner_load_in_flight: Arc<Mutex<BTreeMap<String, SharedOwnerLoadReadThroughFuture>>>,
}

impl<C> PeerFetchReadThrough<C>
where
    C: CacheCodec + Send + Sync + 'static,
{
    /// Create a read-through helper with [`PeerFetchReadThroughPolicy::LocalThenOwner`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    ///
    /// use hydracache::{CacheOptions, ClusterCandidate, ClusterGeneration, HydraCache, InMemoryCluster};
    /// use hydracache_cluster_transport_axum::{HotRemoteCachePolicy, PeerFetchReadThrough};
    ///
    /// # async fn example() -> hydracache::CacheResult<()> {
    /// let near_cache = HydraCache::local().build();
    /// let cluster = InMemoryCluster::new("orders");
    /// cluster.join_member(
    ///     ClusterCandidate::member("member-a")
    ///         .generation(ClusterGeneration::new(1))
    ///         .peer_fetch_base_url("http://127.0.0.1:3000"),
    /// )?;
    ///
    /// let read_through = PeerFetchReadThrough::new(near_cache)
    ///     .hot_remote_policy(
    ///         HotRemoteCachePolicy::new()
    ///             .ttl(Duration::from_secs(30))
    ///             .max_entries(1_000),
    ///     );
    ///
    /// let outcome = read_through
    ///     .fetch_encoded(
    ///         cluster.owner_for_key("user:42"),
    ///         CacheOptions::new().tag("user:42"),
    ///     )
    ///     .await?;
    ///
    /// if outcome.is_remote_hit() {
    ///     assert!(outcome.hydrated);
    /// }
    ///
    /// let hot_remote = read_through.hot_remote_diagnostics();
    /// assert_eq!(hot_remote.max_entries, Some(1_000));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(cache: HydraCache<C>) -> Self {
        Self::with_router(cache, PeerFetchRouter::new())
    }

    /// Create a read-through helper with a caller-provided router.
    pub fn with_router(cache: HydraCache<C>, router: PeerFetchRouter) -> Self {
        Self {
            cache,
            router,
            policy: PeerFetchReadThroughPolicy::default(),
            hydrate_remote_hits: true,
            hot_remote_policy: HotRemoteCachePolicy::default(),
            hot_remote: Arc::new(Mutex::new(HotRemoteCacheState::default())),
            diagnostics: Arc::new(Mutex::new(PeerFetchReadThroughDiagnostics::default())),
            in_flight: Arc::new(Mutex::new(BTreeMap::new())),
            owner_load_diagnostics: Arc::new(
                Mutex::new(OwnerLoadReadThroughDiagnostics::default()),
            ),
            owner_load_in_flight: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Set the read-through policy.
    pub fn policy(mut self, policy: PeerFetchReadThroughPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Set the hot-remote near-cache policy used when remote owner values are
    /// hydrated locally.
    ///
    /// The policy is helper-local: it controls only remote copies inserted by
    /// this read-through helper and does not change owner-side cache entries.
    pub fn hot_remote_policy(mut self, policy: HotRemoteCachePolicy) -> Self {
        self.hot_remote_policy = policy;
        self
    }

    /// Enable or disable local hydration after remote hits.
    pub fn hydrate_remote_hits(mut self, enabled: bool) -> Self {
        self.hydrate_remote_hits = enabled;
        self
    }

    /// Disable local hydration after remote hits.
    pub fn without_hydration(self) -> Self {
        self.hot_remote_policy(HotRemoteCachePolicy::disabled())
            .hydrate_remote_hits(false)
    }

    /// Return the local/near cache handle used by this helper.
    pub fn cache(&self) -> &HydraCache<C> {
        &self.cache
    }

    /// Return the underlying peer-fetch router.
    pub fn router(&self) -> &PeerFetchRouter {
        &self.router
    }

    /// Return current read-through diagnostics.
    pub fn diagnostics(&self) -> PeerFetchReadThroughDiagnostics {
        *self
            .diagnostics
            .lock()
            .expect("peer-fetch read-through diagnostics poisoned")
    }

    /// Return current owner-load read-through diagnostics.
    pub fn owner_load_diagnostics(&self) -> OwnerLoadReadThroughDiagnostics {
        *self
            .owner_load_diagnostics
            .lock()
            .expect("owner-load read-through diagnostics poisoned")
    }

    /// Return current hot-remote near-cache diagnostics.
    pub fn hot_remote_diagnostics(&self) -> HotRemoteCacheDiagnostics {
        self.hot_remote
            .lock()
            .expect("hot-remote diagnostics poisoned")
            .snapshot(self.hot_remote_policy)
    }

    /// Fetch encoded bytes through the configured local/owner policy.
    pub async fn fetch_encoded(
        &self,
        decision: ClusterOwnershipDecision,
        options: CacheOptions,
    ) -> CacheResult<PeerFetchReadThroughOutcome> {
        Self::record_read_through(&self.diagnostics, |diagnostics| {
            diagnostics.attempts = diagnostics.attempts.saturating_add(1);
        });

        match self.policy {
            PeerFetchReadThroughPolicy::LocalThenOwner => {
                if let Some(outcome) = self.local_hit(&decision).await? {
                    return Ok(outcome);
                }
                self.fetch_owner_shared(decision, options).await
            }
            PeerFetchReadThroughPolicy::OwnerThenLocal => {
                let remote = self
                    .fetch_owner_shared(decision.clone(), options.clone())
                    .await?;
                if remote.is_hit() {
                    Ok(remote)
                } else if let Some(local) = self.local_hit(&decision).await? {
                    Ok(local)
                } else {
                    Ok(remote)
                }
            }
            PeerFetchReadThroughPolicy::OwnerOnly => {
                self.fetch_owner_shared(decision, options).await
            }
        }
    }

    /// Fetch encoded bytes locally, or ask the owner to execute a registered
    /// loader on miss.
    ///
    /// If the descriptor does not set an explicit cache key, the helper uses
    /// the key already present in the ownership decision. That keeps callers
    /// from spelling the same key twice in the common path.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hydracache::{CacheOptions, ClusterCandidate, ClusterGeneration, HydraCache, InMemoryCluster};
    /// use hydracache_cluster_transport_axum::{OwnerLoadDescriptor, PeerFetchReadThrough};
    ///
    /// # async fn example() -> hydracache::CacheResult<()> {
    /// let near_cache = HydraCache::local().build();
    /// let cluster = InMemoryCluster::new("users");
    /// cluster.join_member(
    ///     ClusterCandidate::member("member-a")
    ///         .generation(ClusterGeneration::new(1))
    ///         .peer_fetch_base_url("http://127.0.0.1:3000"),
    /// )?;
    ///
    /// let descriptor = OwnerLoadDescriptor::new("users.by-id")
    ///     .tag("users")
    ///     .tag("user:42")
    ///     .arg("id", 42_u64);
    ///
    /// let outcome = PeerFetchReadThrough::new(near_cache)
    ///     .get_or_load_encoded(cluster.owner_for_key("user:42"), descriptor)
    ///     .await?;
    ///
    /// if outcome.is_remote_loaded() {
    ///     assert!(outcome.hydrated);
    /// }
    /// # let _ = CacheOptions::new();
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_or_load_encoded(
        &self,
        decision: ClusterOwnershipDecision,
        descriptor: OwnerLoadDescriptor,
    ) -> CacheResult<OwnerLoadReadThroughOutcome> {
        Self::record_owner_load(&self.owner_load_diagnostics, |diagnostics| {
            diagnostics.attempts = diagnostics.attempts.saturating_add(1);
        });

        let descriptor = if descriptor.key_value().is_some() {
            descriptor
        } else {
            descriptor.key(decision.key.clone())
        };

        match self.policy {
            PeerFetchReadThroughPolicy::LocalThenOwner => {
                if let Some(outcome) = self.owner_load_local_hit(&decision, &descriptor).await? {
                    return Ok(outcome);
                }
                self.owner_load_shared(decision, descriptor).await
            }
            PeerFetchReadThroughPolicy::OwnerThenLocal => {
                let remote = self
                    .owner_load_shared(decision.clone(), descriptor.clone())
                    .await?;
                if remote.is_hit() {
                    Ok(remote)
                } else if let Some(local) =
                    self.owner_load_local_hit(&decision, &descriptor).await?
                {
                    Ok(local)
                } else {
                    Ok(remote)
                }
            }
            PeerFetchReadThroughPolicy::OwnerOnly => {
                self.owner_load_shared(decision, descriptor).await
            }
        }
    }

    async fn local_hit(
        &self,
        decision: &ClusterOwnershipDecision,
    ) -> CacheResult<Option<PeerFetchReadThroughOutcome>> {
        match self.cache.get_encoded(&decision.key).await? {
            Some(value) => {
                Self::record_read_through(&self.diagnostics, |diagnostics| {
                    diagnostics.local_hits = diagnostics.local_hits.saturating_add(1);
                });
                self.cache.record_cluster_hot_cache_hit();
                Ok(Some(PeerFetchReadThroughOutcome {
                    key: decision.key.clone(),
                    policy: self.policy,
                    status: PeerFetchReadThroughStatus::LocalHit,
                    owner: decision.owner.as_ref().map(|member| member.node_id.clone()),
                    endpoint: None,
                    value: Some(value),
                    hydrated: false,
                    error: None,
                }))
            }
            None => {
                Self::record_read_through(&self.diagnostics, |diagnostics| {
                    diagnostics.local_misses = diagnostics.local_misses.saturating_add(1);
                });
                Ok(None)
            }
        }
    }

    async fn fetch_owner_shared(
        &self,
        decision: ClusterOwnershipDecision,
        options: CacheOptions,
    ) -> CacheResult<PeerFetchReadThroughOutcome> {
        let key = decision.key.clone();
        let shared = {
            let mut in_flight = self
                .in_flight
                .lock()
                .expect("peer-fetch read-through in-flight map poisoned");
            if let Some(shared) = in_flight.get(&key) {
                Self::record_read_through(&self.diagnostics, |diagnostics| {
                    diagnostics.in_flight_joins = diagnostics.in_flight_joins.saturating_add(1);
                });
                shared.clone()
            } else {
                let cache = self.cache.clone();
                let router = self.router.clone();
                let diagnostics = self.diagnostics.clone();
                let policy = self.policy;
                let hydrate_remote_hits = self.hydrate_remote_hits;
                let hot_remote_policy = self.hot_remote_policy;
                let hot_remote = self.hot_remote.clone();
                let shared = async move {
                    let execution = ReadThroughExecution {
                        cache,
                        router,
                        diagnostics,
                        policy,
                        hydrate_remote_hits,
                        hot_remote_policy,
                        hot_remote,
                    };
                    Self::fetch_owner_once(execution, decision, options).await
                }
                .boxed()
                .shared();
                in_flight.insert(key.clone(), shared.clone());
                shared
            }
        };

        let result = shared.await;
        self.in_flight
            .lock()
            .expect("peer-fetch read-through in-flight map poisoned")
            .remove(&key);
        result
    }

    async fn owner_load_local_hit(
        &self,
        decision: &ClusterOwnershipDecision,
        descriptor: &OwnerLoadDescriptor,
    ) -> CacheResult<Option<OwnerLoadReadThroughOutcome>> {
        let key = descriptor
            .key_value()
            .unwrap_or(decision.key.as_str())
            .to_owned();
        match self.cache.get_encoded(&key).await? {
            Some(value) => {
                Self::record_owner_load(&self.owner_load_diagnostics, |diagnostics| {
                    diagnostics.local_hits = diagnostics.local_hits.saturating_add(1);
                });
                self.cache.record_cluster_hot_cache_hit();
                Ok(Some(OwnerLoadReadThroughOutcome {
                    key,
                    policy: self.policy,
                    status: OwnerLoadReadThroughStatus::LocalHit,
                    owner: decision.owner.as_ref().map(|member| member.node_id.clone()),
                    endpoint: None,
                    response: None,
                    value: Some(value),
                    hydrated: false,
                    error: None,
                }))
            }
            None => {
                Self::record_owner_load(&self.owner_load_diagnostics, |diagnostics| {
                    diagnostics.local_misses = diagnostics.local_misses.saturating_add(1);
                });
                Ok(None)
            }
        }
    }

    async fn owner_load_shared(
        &self,
        decision: ClusterOwnershipDecision,
        descriptor: OwnerLoadDescriptor,
    ) -> CacheResult<OwnerLoadReadThroughOutcome> {
        let key = descriptor
            .key_value()
            .map(str::to_owned)
            .unwrap_or_else(|| decision.key.clone());
        let shared = {
            let mut in_flight = self
                .owner_load_in_flight
                .lock()
                .expect("owner-load read-through in-flight map poisoned");
            if let Some(shared) = in_flight.get(&key) {
                Self::record_owner_load(&self.owner_load_diagnostics, |diagnostics| {
                    diagnostics.in_flight_joins = diagnostics.in_flight_joins.saturating_add(1);
                });
                shared.clone()
            } else {
                let cache = self.cache.clone();
                let diagnostics = self.owner_load_diagnostics.clone();
                let policy = self.policy;
                let hydrate_remote_hits = self.hydrate_remote_hits;
                let hot_remote_policy = self.hot_remote_policy;
                let hot_remote = self.hot_remote.clone();
                let shared = async move {
                    let execution = OwnerLoadReadThroughExecution {
                        cache,
                        diagnostics,
                        policy,
                        hydrate_remote_hits,
                        hot_remote_policy,
                        hot_remote,
                    };
                    Self::owner_load_once(execution, decision, descriptor).await
                }
                .boxed()
                .shared();
                in_flight.insert(key.clone(), shared.clone());
                shared
            }
        };

        let result = shared.await;
        self.owner_load_in_flight
            .lock()
            .expect("owner-load read-through in-flight map poisoned")
            .remove(&key);
        result
    }

    async fn fetch_owner_once(
        execution: ReadThroughExecution<C>,
        decision: ClusterOwnershipDecision,
        options: CacheOptions,
    ) -> CacheResult<PeerFetchReadThroughOutcome> {
        let routed = execution.router.fetch_owner_value(decision).await;
        let status = read_through_status_from_router(routed.status);
        let mut hydrated = false;

        match routed.status {
            PeerFetchRouterStatus::Hit => {
                Self::record_read_through(&execution.diagnostics, |diagnostics| {
                    diagnostics.remote_hits = diagnostics.remote_hits.saturating_add(1);
                });
                execution.cache.record_cluster_remote_fetch_success();
                if execution.hydrate_remote_hits {
                    if let Some(value) = routed.value.clone() {
                        hydrated = Self::hydrate_remote_entry(
                            &execution.cache,
                            &routed.key,
                            value,
                            options,
                            execution.hot_remote_policy,
                            &execution.hot_remote,
                        )
                        .await?;
                        if hydrated {
                            Self::record_read_through(&execution.diagnostics, |diagnostics| {
                                diagnostics.hydrations = diagnostics.hydrations.saturating_add(1);
                            });
                        }
                    }
                } else {
                    Self::record_hot_remote_skip(&execution.hot_remote);
                }
            }
            PeerFetchRouterStatus::Miss => {
                Self::record_read_through(&execution.diagnostics, |diagnostics| {
                    diagnostics.remote_misses = diagnostics.remote_misses.saturating_add(1);
                });
            }
            PeerFetchRouterStatus::NoOwner
            | PeerFetchRouterStatus::MissingEndpoint
            | PeerFetchRouterStatus::GenerationMismatch
            | PeerFetchRouterStatus::TransportError => {
                Self::record_read_through(&execution.diagnostics, |diagnostics| {
                    diagnostics.router_errors = diagnostics.router_errors.saturating_add(1);
                });
                execution.cache.record_cluster_remote_fetch_error();
            }
        }

        Ok(PeerFetchReadThroughOutcome {
            key: routed.key,
            policy: execution.policy,
            status,
            owner: routed.owner,
            endpoint: routed.endpoint,
            value: routed.value,
            hydrated,
            error: routed.error,
        })
    }

    async fn owner_load_once(
        execution: OwnerLoadReadThroughExecution<C>,
        decision: ClusterOwnershipDecision,
        descriptor: OwnerLoadDescriptor,
    ) -> CacheResult<OwnerLoadReadThroughOutcome> {
        let key = descriptor
            .key_value()
            .map(str::to_owned)
            .unwrap_or_else(|| decision.key.clone());
        let Some(owner) = decision.owner.clone() else {
            Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                diagnostics.routing_errors = diagnostics.routing_errors.saturating_add(1);
            });
            execution.cache.record_cluster_owner_load_error();
            return Ok(OwnerLoadReadThroughOutcome {
                key,
                policy: execution.policy,
                status: OwnerLoadReadThroughStatus::NoOwner,
                owner: None,
                endpoint: None,
                response: None,
                value: None,
                hydrated: false,
                error: Some("ownership decision did not select an owner".to_owned()),
            });
        };

        let Some(base_url) = owner.peer_fetch_base_url() else {
            Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                diagnostics.routing_errors = diagnostics.routing_errors.saturating_add(1);
            });
            execution.cache.record_cluster_owner_load_error();
            return Ok(OwnerLoadReadThroughOutcome {
                key,
                policy: execution.policy,
                status: OwnerLoadReadThroughStatus::MissingEndpoint,
                owner: Some(owner.node_id),
                endpoint: None,
                response: None,
                value: None,
                hydrated: false,
                error: Some("owner did not advertise an owner-load base URL".to_owned()),
            });
        };

        let options = descriptor.cache_options();
        let request = match descriptor.into_request(decision, format!("owner-load:{key}")) {
            Ok(request) => request,
            Err(error) => {
                Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                    diagnostics.routing_errors = diagnostics.routing_errors.saturating_add(1);
                });
                execution.cache.record_cluster_owner_load_error();
                return Ok(OwnerLoadReadThroughOutcome {
                    key,
                    policy: execution.policy,
                    status: OwnerLoadReadThroughStatus::Rejected,
                    owner: Some(owner.node_id),
                    endpoint: None,
                    response: None,
                    value: None,
                    hydrated: false,
                    error: Some(error.to_string()),
                });
            }
        };

        let client = HttpOwnerLoad::for_base_url(base_url);
        let endpoint = client.endpoint().to_owned();
        match client.load(request).await {
            Ok(response) => {
                let status = owner_load_read_through_status(&response);
                let mut value = response.decode_value()?;
                let mut hydrated = false;

                match &response {
                    OwnerLoadResponse::Hit(_) => {
                        Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                            diagnostics.remote_hits = diagnostics.remote_hits.saturating_add(1);
                        });
                        execution.cache.record_cluster_remote_fetch_success();
                    }
                    OwnerLoadResponse::Loaded(_) => {
                        Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                            diagnostics.remote_loaded = diagnostics.remote_loaded.saturating_add(1);
                        });
                        execution.cache.record_cluster_owner_load_success();
                    }
                    OwnerLoadResponse::Miss(_) => {
                        Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                            diagnostics.remote_misses = diagnostics.remote_misses.saturating_add(1);
                        });
                    }
                    OwnerLoadResponse::Rejected(_) => {
                        Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                            diagnostics.rejections = diagnostics.rejections.saturating_add(1);
                        });
                        execution.cache.record_cluster_owner_load_error();
                    }
                    OwnerLoadResponse::Failed(_) => {
                        Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                            diagnostics.failures = diagnostics.failures.saturating_add(1);
                        });
                        execution.cache.record_cluster_owner_load_error();
                    }
                }

                if execution.hydrate_remote_hits
                    && matches!(
                        status,
                        OwnerLoadReadThroughStatus::RemoteHit
                            | OwnerLoadReadThroughStatus::RemoteLoaded
                    )
                {
                    if let Some(bytes) = value.clone() {
                        hydrated = Self::hydrate_remote_entry(
                            &execution.cache,
                            &key,
                            bytes,
                            options,
                            execution.hot_remote_policy,
                            &execution.hot_remote,
                        )
                        .await?;
                        if hydrated {
                            Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                                diagnostics.hydrations = diagnostics.hydrations.saturating_add(1);
                            });
                        }
                    }
                } else if value.is_some() {
                    Self::record_hot_remote_skip(&execution.hot_remote);
                }

                Ok(OwnerLoadReadThroughOutcome {
                    key,
                    policy: execution.policy,
                    status,
                    owner: Some(owner.node_id),
                    endpoint: Some(endpoint),
                    response: Some(response),
                    value: value.take(),
                    hydrated,
                    error: None,
                })
            }
            Err(error) => {
                Self::record_owner_load(&execution.diagnostics, |diagnostics| {
                    diagnostics.transport_errors = diagnostics.transport_errors.saturating_add(1);
                });
                execution.cache.record_cluster_owner_load_error();
                Ok(OwnerLoadReadThroughOutcome {
                    key,
                    policy: execution.policy,
                    status: OwnerLoadReadThroughStatus::TransportError,
                    owner: Some(owner.node_id),
                    endpoint: Some(endpoint),
                    response: None,
                    value: None,
                    hydrated: false,
                    error: Some(error.to_string()),
                })
            }
        }
    }

    async fn hydrate_remote_entry(
        cache: &HydraCache<C>,
        key: &str,
        value: Bytes,
        options: CacheOptions,
        policy: HotRemoteCachePolicy,
        hot_remote: &Arc<Mutex<HotRemoteCacheState>>,
    ) -> CacheResult<bool> {
        if !policy.is_enabled() {
            Self::record_hot_remote_skip(hot_remote);
            return Ok(false);
        }

        cache
            .put_encoded(key, value, policy.cache_options(options))
            .await?;

        let evicted = {
            hot_remote
                .lock()
                .expect("hot-remote state poisoned")
                .record_hydration(key.to_owned(), policy)
        };

        for evicted_key in evicted {
            cache.remove(&evicted_key).await?;
        }

        Ok(true)
    }

    fn record_hot_remote_skip(hot_remote: &Arc<Mutex<HotRemoteCacheState>>) {
        hot_remote
            .lock()
            .expect("hot-remote state poisoned")
            .record_skipped_hydration();
    }

    fn record_read_through(
        diagnostics: &Arc<Mutex<PeerFetchReadThroughDiagnostics>>,
        update: impl FnOnce(&mut PeerFetchReadThroughDiagnostics),
    ) {
        let mut diagnostics = diagnostics
            .lock()
            .expect("peer-fetch read-through diagnostics poisoned");
        update(&mut diagnostics);
    }

    fn record_owner_load(
        diagnostics: &Arc<Mutex<OwnerLoadReadThroughDiagnostics>>,
        update: impl FnOnce(&mut OwnerLoadReadThroughDiagnostics),
    ) {
        let mut diagnostics = diagnostics
            .lock()
            .expect("owner-load read-through diagnostics poisoned");
        update(&mut diagnostics);
    }
}

fn read_through_status_from_router(status: PeerFetchRouterStatus) -> PeerFetchReadThroughStatus {
    match status {
        PeerFetchRouterStatus::NoOwner => PeerFetchReadThroughStatus::NoOwner,
        PeerFetchRouterStatus::MissingEndpoint => PeerFetchReadThroughStatus::MissingEndpoint,
        PeerFetchRouterStatus::Hit => PeerFetchReadThroughStatus::RemoteHit,
        PeerFetchRouterStatus::Miss => PeerFetchReadThroughStatus::RemoteMiss,
        PeerFetchRouterStatus::GenerationMismatch => PeerFetchReadThroughStatus::GenerationMismatch,
        PeerFetchRouterStatus::TransportError => PeerFetchReadThroughStatus::TransportError,
    }
}

fn owner_load_read_through_status(response: &OwnerLoadResponse) -> OwnerLoadReadThroughStatus {
    match response {
        OwnerLoadResponse::Hit(_) => OwnerLoadReadThroughStatus::RemoteHit,
        OwnerLoadResponse::Loaded(_) => OwnerLoadReadThroughStatus::RemoteLoaded,
        OwnerLoadResponse::Miss(_) => OwnerLoadReadThroughStatus::RemoteMiss,
        OwnerLoadResponse::Rejected(_) => OwnerLoadReadThroughStatus::Rejected,
        OwnerLoadResponse::Failed(_) => OwnerLoadReadThroughStatus::Failed,
    }
}

/// Owner-side store abstraction used by the HTTP route.
///
/// The store returns encoded bytes. It deliberately does not deserialize values
/// because remote peer fetch should be type-agnostic and codec-preserving.
#[async_trait::async_trait]
pub trait PeerFetchStore: Send + Sync + 'static {
    /// Return the encoded value for `key`, or `None` on miss/expiry.
    async fn get_encoded(&self, key: &str) -> CacheResult<Option<Bytes>>;
}

#[async_trait::async_trait]
impl<C> PeerFetchStore for HydraCache<C>
where
    C: CacheCodec + Send + Sync + 'static,
{
    async fn get_encoded(&self, key: &str) -> CacheResult<Option<Bytes>> {
        HydraCache::get_encoded(self, key).await
    }
}

/// Simple owner-side encoded-byte store for transport tests and examples.
#[derive(Debug, Clone, Default)]
pub struct MemoryPeerFetchStore {
    values: Arc<Mutex<BTreeMap<String, Bytes>>>,
}

impl MemoryPeerFetchStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store encoded bytes for one key.
    pub fn put(&self, key: impl Into<String>, value: impl Into<Bytes>) {
        self.values
            .lock()
            .expect("peer-fetch store poisoned")
            .insert(key.into(), value.into());
    }

    /// Remove encoded bytes for one key.
    pub fn remove(&self, key: &str) -> Option<Bytes> {
        self.values
            .lock()
            .expect("peer-fetch store poisoned")
            .remove(key)
    }

    /// Return the number of stored keys.
    pub fn len(&self) -> usize {
        self.values.lock().expect("peer-fetch store poisoned").len()
    }

    /// Return whether the store has no keys.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait::async_trait]
impl PeerFetchStore for MemoryPeerFetchStore {
    async fn get_encoded(&self, key: &str) -> CacheResult<Option<Bytes>> {
        Ok(self
            .values
            .lock()
            .expect("peer-fetch store poisoned")
            .get(key)
            .cloned())
    }
}

#[derive(Clone)]
struct PeerFetchState {
    owner: ClusterNodeId,
    generation: ClusterGeneration,
    store: Arc<dyn PeerFetchStore>,
    auth: Option<HttpTransportAuth>,
    wire: HttpWireCompatibility,
}

/// Axum route factory for serving peer-fetch requests from one member.
#[derive(Clone)]
pub struct AxumPeerFetchService {
    state: PeerFetchState,
}

impl fmt::Debug for AxumPeerFetchService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AxumPeerFetchService")
            .field("owner", &self.state.owner)
            .field("generation", &self.state.generation)
            .field("auth_enabled", &self.state.auth.is_some())
            .field("wire", &self.state.wire)
            .finish_non_exhaustive()
    }
}

impl AxumPeerFetchService {
    /// Create a peer-fetch service for one member owner.
    pub fn new(
        owner: impl Into<ClusterNodeId>,
        generation: ClusterGeneration,
        store: Arc<dyn PeerFetchStore>,
    ) -> Self {
        Self {
            state: PeerFetchState {
                owner: owner.into(),
                generation,
                store,
                auth: None,
                wire: HttpWireCompatibility::default(),
            },
        }
    }

    /// Require an auth header before peer-fetch requests reach the store.
    pub fn with_auth(mut self, auth: HttpTransportAuth) -> Self {
        self.state.auth = Some(auth);
        self
    }

    /// Set the HTTP wire compatibility policy for this route factory.
    pub fn with_wire_compatibility(mut self, wire: HttpWireCompatibility) -> Self {
        self.state.wire = wire;
        self
    }

    /// Return the owner node id served by this route.
    pub fn owner(&self) -> &ClusterNodeId {
        &self.state.owner
    }

    /// Return the owner generation served by this route.
    pub fn generation(&self) -> ClusterGeneration {
        self.state.generation
    }

    /// Return the configured auth boundary, if any.
    pub fn auth(&self) -> Option<&HttpTransportAuth> {
        self.state.auth.as_ref()
    }

    /// Return the configured wire compatibility policy.
    pub fn wire(&self) -> HttpWireCompatibility {
        self.state.wire
    }

    /// Build the Axum router with `POST /cluster/peer-fetch`.
    pub fn routes(&self) -> Router {
        Router::new()
            .route(DEFAULT_PEER_FETCH_PATH, post(handle_peer_fetch))
            .with_state(self.state.clone())
    }
}

/// JSON request body used by the HTTP transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerFetchHttpRequest {
    /// Expected owner member id.
    pub owner: String,
    /// Logical cache key requested from that owner.
    pub key: String,
    /// Optional owner generation observed by the caller.
    pub generation: Option<u64>,
}

impl PeerFetchHttpRequest {
    /// Build a transport DTO from the transport-neutral request.
    pub fn from_peer_request(request: &ClusterPeerFetchRequest) -> Self {
        Self {
            owner: request.owner.as_str().to_owned(),
            key: request.key.clone(),
            generation: request.generation.map(ClusterGeneration::value),
        }
    }
}

/// JSON response body used by the HTTP transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerFetchHttpResponse {
    /// Owner member that served the request.
    pub owner: String,
    /// Logical cache key requested from the owner.
    pub key: String,
    /// Base64-encoded cache bytes. `None` means cache miss.
    pub value_base64: Option<String>,
}

impl PeerFetchHttpResponse {
    /// Build a transport DTO from the transport-neutral response.
    pub fn from_peer_response(response: &ClusterPeerFetchResponse) -> Self {
        Self {
            owner: response.owner.as_str().to_owned(),
            key: response.key.clone(),
            value_base64: response
                .value
                .as_ref()
                .map(|value| BASE64_STANDARD.encode(value.as_ref())),
        }
    }

    /// Decode the optional base64 payload into bytes.
    pub fn decode_value(&self) -> CacheResult<Option<Bytes>> {
        self.value_base64
            .as_ref()
            .map(|value| {
                BASE64_STANDARD
                    .decode(value)
                    .map(Bytes::from)
                    .map_err(|error| {
                        CacheError::Decode(format!("invalid peer-fetch payload: {error}"))
                    })
            })
            .transpose()
    }

    fn into_peer_response(self) -> CacheResult<ClusterPeerFetchResponse> {
        let value = self.decode_value()?;
        Ok(ClusterPeerFetchResponse {
            owner: self.owner.into(),
            key: self.key,
            value,
        })
    }
}

/// JSON error body returned by the HTTP route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerFetchHttpErrorBody {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable error detail.
    pub message: String,
    /// Generation observed by the caller, if provided.
    pub requested_generation: Option<u64>,
    /// Current owner generation on this server, when relevant.
    pub current_generation: Option<u64>,
}

async fn handle_peer_fetch(
    State(state): State<PeerFetchState>,
    headers: HeaderMap,
    Json(request): Json<PeerFetchHttpRequest>,
) -> Response {
    if let Err(rejection) = validate_http_transport(&headers, state.auth.as_ref(), state.wire) {
        return error_response(
            rejection.status,
            rejection.code,
            rejection.message,
            request.generation,
            Some(state.generation.value()),
        );
    }

    if request.owner != state.owner.as_str() {
        return error_response(
            StatusCode::NOT_FOUND,
            "owner-mismatch",
            format!(
                "peer-fetch route serves owner '{}', not '{}'",
                state.owner, request.owner
            ),
            request.generation,
            Some(state.generation.value()),
        );
    }

    if let Some(requested_generation) = request.generation {
        let requested = ClusterGeneration::new(requested_generation);
        if requested != state.generation {
            return error_response(
                StatusCode::CONFLICT,
                "generation-mismatch",
                format!(
                    "requested owner generation {} does not match current generation {}",
                    requested_generation,
                    state.generation.value()
                ),
                Some(requested_generation),
                Some(state.generation.value()),
            );
        }
    }

    match state.store.get_encoded(&request.key).await {
        Ok(value) => {
            let response = PeerFetchHttpResponse {
                owner: state.owner.as_str().to_owned(),
                key: request.key,
                value_base64: value.map(|value| BASE64_STANDARD.encode(value.as_ref())),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "store-error",
            error.to_string(),
            request.generation,
            Some(state.generation.value()),
        ),
    }
}

fn error_response(
    status: StatusCode,
    code: impl Into<String>,
    message: impl Into<String>,
    requested_generation: Option<u64>,
    current_generation: Option<u64>,
) -> Response {
    (
        status,
        Json(PeerFetchHttpErrorBody {
            code: code.into(),
            message: message.into(),
            requested_generation,
            current_generation,
        }),
    )
        .into_response()
}

/// HTTP client implementation of [`ClusterPeerFetch`].
#[derive(Debug, Clone)]
pub struct HttpPeerFetch {
    endpoint: String,
    client: reqwest::Client,
    auth: Option<HttpTransportAuth>,
    wire: HttpWireCompatibility,
}

impl HttpPeerFetch {
    /// Create a peer-fetch client for a full endpoint URL.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::new(),
            auth: None,
            wire: HttpWireCompatibility::default(),
        }
    }

    /// Create a peer-fetch client from a member base URL.
    ///
    /// `DEFAULT_PEER_FETCH_PATH` is appended after trimming a trailing slash.
    pub fn for_base_url(base_url: impl AsRef<str>) -> Self {
        let base_url = base_url.as_ref().trim_end_matches('/');
        Self::new(format!("{base_url}{DEFAULT_PEER_FETCH_PATH}"))
    }

    /// Return the endpoint URL used by this client.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Attach an auth header to every peer-fetch request.
    pub fn with_auth(mut self, auth: HttpTransportAuth) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set the wire version header sent with peer-fetch requests.
    pub fn with_wire_compatibility(mut self, wire: HttpWireCompatibility) -> Self {
        self.wire = wire;
        self
    }
}

#[async_trait::async_trait]
impl ClusterPeerFetch for HttpPeerFetch {
    async fn fetch(
        &self,
        request: ClusterPeerFetchRequest,
    ) -> CacheResult<ClusterPeerFetchResponse> {
        let expected_owner = request.owner.clone();
        let expected_key = request.key.clone();
        let http_request = PeerFetchHttpRequest::from_peer_request(&request);
        let mut builder = self.client.post(&self.endpoint).json(&http_request);
        builder = self.wire.apply(builder);
        if let Some(auth) = &self.auth {
            builder = auth.apply(builder);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| CacheError::Backend(format!("peer-fetch request failed: {error}")))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|error| format!("failed to read error body: {error}"));
            return Err(CacheError::Backend(format!(
                "peer-fetch request failed with {status}: {body}"
            )));
        }

        let response = response
            .json::<PeerFetchHttpResponse>()
            .await
            .map_err(|error| CacheError::Decode(format!("invalid peer-fetch response: {error}")))?;

        if response.owner != expected_owner.as_str() || response.key != expected_key {
            return Err(CacheError::Backend(format!(
                "peer-fetch response identity mismatch: expected owner/key '{expected_owner}/{expected_key}', got '{}/{}'",
                response.owner, response.key
            )));
        }

        response.into_peer_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use hydracache::{
        ClusterCandidate, ClusterEndpoints, ClusterEpoch, ClusterMember, ClusterRole,
    };
    use serde::de::DeserializeOwned;
    use tokio::sync::oneshot;
    use tower::ServiceExt;

    #[test]
    fn owner_load_args_descriptor_and_request_roundtrip() {
        let descriptor = OwnerLoadDescriptor::new("users.by-id")
            .key("user:42")
            .tag("users")
            .tag("user:42")
            .ttl(Duration::from_secs(60))
            .arg("id", 42_i64)
            .arg("tenant", "acme")
            .arg("include_deleted", false);

        assert_eq!(descriptor.loader(), "users.by-id");
        assert_eq!(descriptor.key_value(), Some("user:42"));
        assert_eq!(
            descriptor.tags_value(),
            &["users".to_owned(), "user:42".to_owned()]
        );
        assert_eq!(descriptor.ttl_millis_value(), Some(60_000));
        assert_eq!(descriptor.args().get_i64("id"), Some(42));
        assert_eq!(descriptor.args().get_str("tenant"), Some("acme"));
        assert_eq!(descriptor.args().get_bool("include_deleted"), Some(false));

        let options = descriptor.cache_options();
        assert_eq!(options.ttl_value(), Some(Duration::from_secs(60)));
        assert_eq!(
            options.tags_value(),
            &["users".to_owned(), "user:42".to_owned()]
        );

        let request = descriptor
            .into_request(
                decision_with_endpoint("http://127.0.0.1:3000", "ignored", 7),
                "req-1",
            )
            .unwrap();

        assert_eq!(request.owner, "member-a");
        assert_eq!(request.key, "user:42");
        assert_eq!(request.loader, "users.by-id");
        assert_eq!(request.generation, Some(7));
        assert_eq!(request.request_id, "req-1");
        assert_eq!(request.arg_i64("id").unwrap(), 42);
        assert_eq!(request.arg_u64("id").unwrap(), 42);
        assert_eq!(request.arg_str("tenant").unwrap(), "acme");
        assert!(!request.arg_bool("include_deleted").unwrap());
        assert!(request
            .arg_str("missing")
            .unwrap_err()
            .to_string()
            .contains("missing"));

        let serialized = serde_json::to_string(&request).unwrap();
        let decoded: OwnerLoadRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(decoded, request);
    }

    #[test]
    fn owner_load_request_build_errors_are_explicit() {
        let missing_key = OwnerLoadDescriptor::new("users.by-id")
            .into_request(
                decision_with_endpoint("http://127.0.0.1:3000", "ignored", 7),
                "req-1",
            )
            .unwrap_err();
        assert_eq!(
            missing_key,
            OwnerLoadRequestBuildError::MissingKey {
                loader: "users.by-id".to_owned()
            }
        );
        assert!(missing_key.to_string().contains("missing a key"));

        let no_owner = OwnerLoadDescriptor::new("users.by-id")
            .key("user:42")
            .into_request(
                ClusterOwnershipDecision {
                    key: "user:42".to_owned(),
                    owner: None,
                    member_count: 0,
                    resolver: "test",
                },
                "req-1",
            )
            .unwrap_err();
        assert_eq!(
            no_owner,
            OwnerLoadRequestBuildError::NoOwner {
                key: "user:42".to_owned()
            }
        );
        assert!(no_owner.to_string().contains("no owner"));
    }

    #[test]
    fn owner_load_response_statuses_decode_and_roundtrip() {
        let hit = OwnerLoadResponse::Hit(OwnerLoadHit::new(
            "member-a",
            "user:42",
            "users.by-id",
            Bytes::from_static(b"encoded-user"),
        ));
        assert!(hit.is_hit());
        assert!(!hit.is_loaded());
        assert_eq!(
            hit.decode_value().unwrap(),
            Some(Bytes::from_static(b"encoded-user"))
        );

        let loaded = OwnerLoadResponse::Loaded(OwnerLoadHit::new(
            "member-a",
            "user:42",
            "users.by-id",
            Bytes::from_static(b"loaded-user"),
        ));
        assert!(loaded.is_hit());
        assert!(loaded.is_loaded());
        assert_eq!(
            loaded.decode_value().unwrap(),
            Some(Bytes::from_static(b"loaded-user"))
        );

        let miss =
            OwnerLoadResponse::Miss(OwnerLoadMiss::new("member-a", "missing", "users.by-id"));
        assert!(miss.is_miss());
        assert_eq!(miss.decode_value().unwrap(), None);

        let rejected = OwnerLoadResponse::Rejected(
            OwnerLoadRejection::new(OwnerLoadRejectionCode::StaleGeneration, "stale generation")
                .owners("member-a", "member-b")
                .generations(Some(6), Some(7)),
        );
        assert!(rejected.is_rejected());
        assert_eq!(rejected.decode_value().unwrap(), None);

        let failed = OwnerLoadResponse::Failed(OwnerLoadFailure::new(
            "loader-error",
            "database unavailable",
            "user:42",
            "users.by-id",
        ));
        assert!(failed.is_failed());
        assert_eq!(failed.decode_value().unwrap(), None);

        for response in [hit, loaded, miss, rejected, failed] {
            let encoded = serde_json::to_string(&response).unwrap();
            let decoded: OwnerLoadResponse = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, response);
        }
    }

    #[test]
    fn owner_load_invalid_payload_reports_decode_error() {
        let response = OwnerLoadResponse::Hit(OwnerLoadHit {
            owner: "member-a".to_owned(),
            key: "user:42".to_owned(),
            loader: "users.by-id".to_owned(),
            value_base64: "not base64".to_owned(),
        });

        let error = response.decode_value().unwrap_err();
        assert!(matches!(error, CacheError::Decode(_)));
        assert!(error.to_string().contains("invalid owner-load payload"));
    }

    #[tokio::test]
    async fn owner_load_registry_replaces_loaders_and_exposes_names() {
        let registry = OwnerLoadRegistry::new()
            .register("users.by-id", |_| async {
                Ok(Some(OwnerLoadValue::encoded(
                    Bytes::from_static(b"first"),
                    CacheOptions::new(),
                )))
            })
            .register("users.by-id", |_| async { Ok(None) });

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert_eq!(registry.loader_names(), vec!["users.by-id"]);
        assert!(registry.contains_loader("users.by-id"));
        assert!(format!("{registry:?}").contains("users.by-id"));

        let loaded = registry
            .load(owner_load_request("user:42", "users.by-id", 7))
            .await
            .unwrap();
        assert_eq!(loaded, None);

        let missing = registry
            .load(owner_load_request("user:42", "missing", 7))
            .await
            .unwrap_err();
        assert!(missing.to_string().contains("not registered"));
    }

    #[tokio::test]
    async fn owner_load_service_hits_local_cache_without_loader_execution() {
        let cache = HydraCache::local().build();
        cache
            .put("answer", 42_u64, CacheOptions::new())
            .await
            .unwrap();
        let registry = OwnerLoadRegistry::new();
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            cache.clone(),
            registry,
        );

        let response = service
            .load(owner_load_request("answer", "missing-loader", 7))
            .await;

        assert!(response.is_hit());
        assert!(!response.is_loaded());
        assert_eq!(
            response.decode_value().unwrap().unwrap(),
            encoded_u64(42).await
        );
        assert_eq!(service.owner().as_str(), "member-a");
        assert_eq!(service.generation(), ClusterGeneration::new(7));
        assert_eq!(
            service.cache().get::<u64>("answer").await.unwrap(),
            Some(42)
        );

        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.owner_hits, 1);
        assert_eq!(diagnostics.owner_misses, 0);
        assert_eq!(diagnostics.loader_executions, 0);
        assert_eq!(diagnostics.total_successes(), 1);
        assert!(!diagnostics.has_failures());
        assert!(format!("{service:?}").contains("OwnerLoadService"));
    }

    #[tokio::test]
    async fn owner_load_service_loads_miss_and_stores_value() {
        let calls = Arc::new(AtomicUsize::new(0));
        let registry = {
            let calls = calls.clone();
            OwnerLoadRegistry::new().register("answers.by-id", move |request| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(OwnerLoadValue::encoded(
                        encoded_u64(request.arg_u64("id").unwrap()).await,
                        request.cache_options(),
                    )))
                }
            })
        };
        let cache = HydraCache::local().build();
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            cache.clone(),
            registry,
        );

        let response = service
            .load(
                owner_load_request("answer", "answers.by-id", 7)
                    .with_args(OwnerLoadArgs::new().arg("id", 42_u64)),
            )
            .await;

        assert!(response.is_loaded());
        assert_eq!(
            response.decode_value().unwrap().unwrap(),
            encoded_u64(42).await
        );
        assert_eq!(cache.get::<u64>("answer").await.unwrap(), Some(42));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = service
            .load(owner_load_request("answer", "answers.by-id", 7))
            .await;
        assert!(matches!(second, OwnerLoadResponse::Hit(_)));

        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.attempts, 2);
        assert_eq!(diagnostics.owner_hits, 1);
        assert_eq!(diagnostics.owner_misses, 1);
        assert_eq!(diagnostics.loader_executions, 1);
        assert_eq!(diagnostics.loaded, 1);
        assert_eq!(diagnostics.stores, 1);
        assert_eq!(diagnostics.total_successes(), 2);
    }

    #[tokio::test]
    async fn owner_load_service_rejects_wrong_owner_generation_and_missing_loader() {
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            OwnerLoadRegistry::new(),
        );

        let wrong_owner = service
            .load(OwnerLoadRequest {
                owner: "member-b".to_owned(),
                ..owner_load_request("user:42", "users.by-id", 7)
            })
            .await;
        assert!(wrong_owner.is_rejected());
        match wrong_owner {
            OwnerLoadResponse::Rejected(rejection) => {
                assert_eq!(rejection.code, OwnerLoadRejectionCode::WrongOwner);
                assert_eq!(rejection.current_owner.as_deref(), Some("member-a"));
            }
            other => panic!("unexpected response: {other:?}"),
        }

        let stale_generation = service
            .load(owner_load_request("user:42", "users.by-id", 6))
            .await;
        assert!(stale_generation.is_rejected());
        match stale_generation {
            OwnerLoadResponse::Rejected(rejection) => {
                assert_eq!(rejection.code, OwnerLoadRejectionCode::StaleGeneration);
                assert_eq!(rejection.requested_generation, Some(6));
                assert_eq!(rejection.current_generation, Some(7));
            }
            other => panic!("unexpected response: {other:?}"),
        }

        let missing_loader = service
            .load(owner_load_request("user:42", "users.by-id", 7))
            .await;
        assert!(missing_loader.is_rejected());
        match missing_loader {
            OwnerLoadResponse::Rejected(rejection) => {
                assert_eq!(rejection.code, OwnerLoadRejectionCode::MissingLoader);
                assert!(rejection.message.contains("not registered"));
            }
            other => panic!("unexpected response: {other:?}"),
        }

        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.attempts, 3);
        assert_eq!(diagnostics.rejections, 3);
        assert_eq!(diagnostics.owner_misses, 1);
        assert_eq!(diagnostics.loader_executions, 0);
        assert!(diagnostics.has_failures());
    }

    #[tokio::test]
    async fn owner_load_service_maps_loader_miss_and_failure() {
        let miss_service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            OwnerLoadRegistry::new().register("users.optional", |_| async { Ok(None) }),
        );

        let miss = miss_service
            .load(owner_load_request("missing", "users.optional", 7))
            .await;
        assert!(miss.is_miss());
        assert_eq!(miss.decode_value().unwrap(), None);
        assert_eq!(miss_service.diagnostics().misses, 1);

        let failure_service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            OwnerLoadRegistry::new().register("users.failing", |_| async {
                Err(CacheError::Loader("database unavailable".to_owned()))
            }),
        );

        let failure = failure_service
            .load(owner_load_request("user:42", "users.failing", 7))
            .await;
        assert!(failure.is_failed());
        match failure {
            OwnerLoadResponse::Failed(failure) => {
                assert_eq!(failure.code, "loader-error");
                assert!(failure.message.contains("database unavailable"));
            }
            other => panic!("unexpected response: {other:?}"),
        }
        assert_eq!(failure_service.diagnostics().failures, 1);
    }

    #[tokio::test]
    async fn owner_load_service_shares_concurrent_loader_for_same_key() {
        let calls = Arc::new(AtomicUsize::new(0));
        let registry = {
            let calls = calls.clone();
            OwnerLoadRegistry::new().register("answers.slow", move |request| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    Ok(Some(OwnerLoadValue::encoded(
                        encoded_u64(request.arg_u64("id").unwrap()).await,
                        request.cache_options(),
                    )))
                }
            })
        };
        let service = Arc::new(OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            registry,
        ));

        let mut tasks = Vec::new();
        for _ in 0..8 {
            let service = service.clone();
            tasks.push(tokio::spawn(async move {
                service
                    .load(
                        owner_load_request("answer", "answers.slow", 7)
                            .with_args(OwnerLoadArgs::new().arg("id", 42_u64)),
                    )
                    .await
            }));
        }

        for task in tasks {
            let response = task.await.unwrap();
            assert!(response.is_loaded());
            assert_eq!(
                response.decode_value().unwrap().unwrap(),
                encoded_u64(42).await
            );
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            service.cache().get::<u64>("answer").await.unwrap(),
            Some(42)
        );

        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.attempts, 8);
        assert_eq!(diagnostics.owner_misses, 8);
        assert_eq!(diagnostics.loader_executions, 1);
        assert!(diagnostics.in_flight_joins >= 1);
        assert_eq!(diagnostics.loaded, 1);
        assert_eq!(diagnostics.stores, 1);
    }

    #[tokio::test]
    async fn memory_store_reports_hits_and_misses() {
        let store = MemoryPeerFetchStore::new();
        assert!(store.is_empty());

        store.put("user:42", Bytes::from_static(b"encoded-user"));

        assert_eq!(store.len(), 1);
        assert_eq!(
            store.get_encoded("user:42").await.unwrap(),
            Some(Bytes::from_static(b"encoded-user"))
        );
        assert_eq!(store.get_encoded("missing").await.unwrap(), None);
        assert_eq!(
            store.remove("user:42"),
            Some(Bytes::from_static(b"encoded-user"))
        );
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn route_returns_base64_hit() {
        let store = MemoryPeerFetchStore::new();
        store.put("user:42", Bytes::from_static(b"encoded-user"));
        let app = service_with_store(store).routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "user:42".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeerFetchHttpResponse = response_json(response).await;
        assert_eq!(body.owner, "member-a");
        assert_eq!(body.key, "user:42");
        assert_eq!(
            body.decode_value().unwrap(),
            Some(Bytes::from_static(b"encoded-user"))
        );
    }

    #[tokio::test]
    async fn route_returns_miss_for_missing_key() {
        let app = service_with_store(MemoryPeerFetchStore::new()).routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "missing".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: PeerFetchHttpResponse = response_json(response).await;
        assert_eq!(body.value_base64, None);
        assert_eq!(body.decode_value().unwrap(), None);
    }

    #[tokio::test]
    async fn route_rejects_wrong_owner() {
        let app = service_with_store(MemoryPeerFetchStore::new()).routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-b".to_owned(),
                key: "user:42".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body: PeerFetchHttpErrorBody = response_json(response).await;
        assert_eq!(body.code, "owner-mismatch");
        assert_eq!(body.current_generation, Some(7));
    }

    #[tokio::test]
    async fn route_rejects_stale_generation() {
        let app = service_with_store(MemoryPeerFetchStore::new()).routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "user:42".to_owned(),
                generation: Some(6),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body: PeerFetchHttpErrorBody = response_json(response).await;
        assert_eq!(body.code, "generation-mismatch");
        assert_eq!(body.requested_generation, Some(6));
        assert_eq!(body.current_generation, Some(7));
    }

    #[tokio::test]
    async fn auth_peer_fetch_route_rejects_missing_token() {
        let app = service_with_store(MemoryPeerFetchStore::new())
            .with_auth(HttpTransportAuth::token("secret"))
            .routes();

        let response = app
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "answer".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body: PeerFetchHttpErrorBody = response_json(response).await;
        assert_eq!(body.code, "unauthorized");
        assert_eq!(body.current_generation, Some(7));
    }

    #[tokio::test]
    async fn wire_peer_fetch_route_rejects_missing_and_mismatched_versions() {
        let app = service_with_store(MemoryPeerFetchStore::new())
            .with_wire_compatibility(HttpWireCompatibility::strict_current())
            .routes();

        let missing = app
            .clone()
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "answer".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::UPGRADE_REQUIRED);
        let missing: PeerFetchHttpErrorBody = response_json(missing).await;
        assert_eq!(missing.code, "missing-wire-version");

        let mut mismatched = json_request(PeerFetchHttpRequest {
            owner: "member-a".to_owned(),
            key: "answer".to_owned(),
            generation: Some(7),
        });
        mismatched.headers_mut().insert(
            HYDRACACHE_WIRE_VERSION_HEADER,
            HeaderValue::from_static("2"),
        );
        let response = app.oneshot(mismatched).await.unwrap();
        assert_eq!(response.status(), StatusCode::UPGRADE_REQUIRED);
        let body: PeerFetchHttpErrorBody = response_json(response).await;
        assert_eq!(body.code, "wire-version-mismatch");
    }

    #[tokio::test]
    async fn http_peer_fetch_round_trips_against_axum_server() {
        let store = MemoryPeerFetchStore::new();
        store.put("user:42", Bytes::from_static(b"encoded-user"));
        let app = service_with_store(store).routes();
        let (base_url, shutdown, server) = spawn_server(app).await;
        let peer_fetch = HttpPeerFetch::for_base_url(&base_url);

        let response = peer_fetch
            .fetch(
                ClusterPeerFetchRequest::new("member-a", "user:42")
                    .generation(ClusterGeneration::new(7)),
            )
            .await
            .unwrap();

        assert_eq!(
            peer_fetch.endpoint(),
            format!("{base_url}{DEFAULT_PEER_FETCH_PATH}")
        );
        assert!(response.is_hit());
        assert_eq!(response.owner.as_str(), "member-a");
        assert_eq!(response.key, "user:42");
        assert_eq!(response.value.unwrap().as_ref(), b"encoded-user");

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn auth_and_strict_wire_peer_fetch_client_round_trips() {
        let store = MemoryPeerFetchStore::new();
        store.put("answer", Bytes::from_static(b"encoded"));
        let auth = HttpTransportAuth::bearer("secret");
        let app = service_with_store(store)
            .with_auth(auth.clone())
            .with_wire_compatibility(HttpWireCompatibility::strict_current())
            .routes();
        let (base_url, shutdown, server) = spawn_server(app).await;
        let peer_fetch = HttpPeerFetch::for_base_url(&base_url).with_auth(auth);

        let response = peer_fetch
            .fetch(
                ClusterPeerFetchRequest::new("member-a", "answer")
                    .generation(ClusterGeneration::new(7)),
            )
            .await
            .unwrap();

        assert_eq!(response.value, Some(Bytes::from_static(b"encoded")));

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn http_peer_fetch_maps_miss_and_generation_error() {
        let app = service_with_store(MemoryPeerFetchStore::new()).routes();
        let (base_url, shutdown, server) = spawn_server(app).await;
        let peer_fetch = HttpPeerFetch::for_base_url(&base_url);

        let missing = peer_fetch
            .fetch(
                ClusterPeerFetchRequest::new("member-a", "missing")
                    .generation(ClusterGeneration::new(7)),
            )
            .await
            .unwrap();
        assert!(missing.is_miss());

        let error = peer_fetch
            .fetch(
                ClusterPeerFetchRequest::new("member-a", "missing")
                    .generation(ClusterGeneration::new(6)),
            )
            .await
            .unwrap_err();
        assert!(error.to_string().contains("generation-mismatch"));

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn router_reports_no_owner_without_calling_transport() {
        let router = PeerFetchRouter::new();
        let outcome = router
            .fetch_owner_value(ClusterOwnershipDecision {
                key: "user:42".to_owned(),
                owner: None,
                member_count: 0,
                resolver: "test",
            })
            .await;

        assert_eq!(outcome.status, PeerFetchRouterStatus::NoOwner);
        assert!(outcome.did_not_route());
        assert_eq!(outcome.owner, None);
        assert!(outcome.endpoint.is_none());

        let diagnostics = router.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.no_owner, 1);
        assert!(diagnostics.has_failures());
    }

    #[tokio::test]
    async fn router_reports_missing_endpoint_without_calling_transport() {
        let router = PeerFetchRouter::new();
        let outcome = router
            .fetch_owner_value(decision_with_member(member_without_endpoint(), "user:42"))
            .await;

        assert_eq!(outcome.status, PeerFetchRouterStatus::MissingEndpoint);
        assert!(outcome.did_not_route());
        assert_eq!(
            outcome.owner.as_ref().map(ClusterNodeId::as_str),
            Some("member-a")
        );
        assert!(outcome.endpoint.is_none());

        let diagnostics = router.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.missing_endpoint, 1);
        assert!(diagnostics.has_failures());
    }

    #[tokio::test]
    async fn router_fetches_hit_from_advertised_owner_endpoint() {
        let store = MemoryPeerFetchStore::new();
        store.put("user:42", Bytes::from_static(b"encoded-user"));
        let (base_url, shutdown, server) = spawn_server(service_with_store(store).routes()).await;
        let router = PeerFetchRouter::new();

        let outcome = router
            .fetch_owner_value(decision_with_endpoint(&base_url, "user:42", 7))
            .await;

        assert_eq!(outcome.status, PeerFetchRouterStatus::Hit);
        assert!(outcome.is_hit());
        assert_eq!(outcome.value.unwrap().as_ref(), b"encoded-user");
        assert_eq!(
            outcome.endpoint.as_deref(),
            Some(format!("{base_url}{DEFAULT_PEER_FETCH_PATH}").as_str())
        );

        let diagnostics = router.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.hits, 1);
        assert_eq!(diagnostics.routed_requests(), 1);
        assert!(!diagnostics.has_failures());

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn router_fetches_miss_from_advertised_owner_endpoint() {
        let (base_url, shutdown, server) =
            spawn_server(service_with_store(MemoryPeerFetchStore::new()).routes()).await;
        let router = PeerFetchRouter::new();

        let outcome = router
            .fetch_owner_value(decision_with_endpoint(&base_url, "missing", 7))
            .await;

        assert_eq!(outcome.status, PeerFetchRouterStatus::Miss);
        assert!(outcome.is_miss());
        assert!(outcome.value.is_none());

        let diagnostics = router.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.misses, 1);
        assert_eq!(diagnostics.routed_requests(), 1);
        assert!(!diagnostics.has_failures());

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn router_reports_generation_mismatch_from_owner() {
        let (base_url, shutdown, server) =
            spawn_server(service_with_store(MemoryPeerFetchStore::new()).routes()).await;
        let router = PeerFetchRouter::new();

        let outcome = router
            .fetch_owner_value(decision_with_endpoint(&base_url, "user:42", 6))
            .await;

        assert_eq!(outcome.status, PeerFetchRouterStatus::GenerationMismatch);
        assert!(outcome
            .error
            .as_deref()
            .unwrap()
            .contains("generation-mismatch"));

        let diagnostics = router.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.generation_mismatches, 1);
        assert!(diagnostics.has_failures());

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn router_reports_transport_error_for_invalid_endpoint() {
        let router = PeerFetchRouter::new();

        let outcome = router
            .fetch_owner_value(decision_with_endpoint("not a url", "user:42", 7))
            .await;

        assert_eq!(outcome.status, PeerFetchRouterStatus::TransportError);
        assert!(outcome.error.is_some());

        let diagnostics = router.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.transport_errors, 1);
        assert!(diagnostics.has_failures());
    }

    #[tokio::test]
    async fn http_response_rejects_invalid_base64() {
        let response = PeerFetchHttpResponse {
            owner: "member-a".to_owned(),
            key: "user:42".to_owned(),
            value_base64: Some("not base64".to_owned()),
        };

        let error = response.decode_value().unwrap_err();
        assert!(matches!(error, CacheError::Decode(_)));
    }

    #[test]
    fn http_response_can_be_built_from_transport_neutral_response() {
        let hit = ClusterPeerFetchResponse::hit(
            "member-a",
            "user:42",
            Bytes::from_static(b"encoded-user"),
        );
        let dto = PeerFetchHttpResponse::from_peer_response(&hit);

        assert_eq!(dto.owner, "member-a");
        assert_eq!(dto.key, "user:42");
        assert_eq!(
            dto.decode_value().unwrap(),
            Some(Bytes::from_static(b"encoded-user"))
        );

        let miss = ClusterPeerFetchResponse::miss("member-a", "missing");
        let dto = PeerFetchHttpResponse::from_peer_response(&miss);
        assert_eq!(dto.value_base64, None);
    }

    #[tokio::test]
    async fn hydracache_implements_peer_fetch_store() {
        let cache = HydraCache::local().build();
        cache
            .put("answer", 42_u64, hydracache::CacheOptions::new())
            .await
            .unwrap();

        let encoded = PeerFetchStore::get_encoded(&cache, "answer")
            .await
            .unwrap()
            .expect("stored bytes");

        assert!(!encoded.is_empty());
    }

    #[test]
    fn hot_remote_policy_and_diagnostics_describe_pressure_shape() {
        let disabled = HotRemoteCachePolicy::disabled();
        assert!(!disabled.is_enabled());
        assert_eq!(disabled.ttl_value(), None);
        assert_eq!(disabled.max_entries_value(), None);

        let policy = HotRemoteCachePolicy::new()
            .ttl(Duration::from_millis(25))
            .max_entries(2);

        assert!(policy.is_enabled());
        assert_eq!(policy.ttl_value(), Some(Duration::from_millis(25)));
        assert_eq!(policy.max_entries_value(), Some(2));

        let diagnostics = HotRemoteCacheDiagnostics {
            enabled: true,
            ttl_millis: Some(25),
            max_entries: Some(2),
            tracked_entries: 1,
            hydrations: 3,
            skipped_hydrations: 0,
            pressure_evictions: 1,
        };

        assert!(diagnostics.is_bounded());
        assert!(diagnostics.has_pressure_evictions());
    }

    #[tokio::test]
    async fn read_through_local_hit_does_not_call_router() {
        let cache = HydraCache::local().build();
        cache
            .put("answer", 42_u64, CacheOptions::new())
            .await
            .unwrap();
        let read_through = PeerFetchReadThrough::new(cache);

        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint("not a url", "answer", 7),
                CacheOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::LocalHit);
        assert!(outcome.is_hit());
        assert!(outcome.is_local_hit());
        assert!(!outcome.hydrated);
        assert_eq!(read_through.router().diagnostics().attempts, 0);

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.local_hits, 1);
        assert_eq!(diagnostics.remote_hits, 0);
        assert_eq!(diagnostics.total_hits(), 1);
    }

    #[tokio::test]
    async fn read_through_remote_hit_hydrates_near_cache() {
        let store = MemoryPeerFetchStore::new();
        store.put("answer", encoded_u64(42).await);
        let (base_url, shutdown, server) = spawn_server(service_with_store(store).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone());

        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer", 7),
                CacheOptions::new().tag("answers"),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::RemoteHit);
        assert!(outcome.is_remote_hit());
        assert!(outcome.hydrated);
        assert_eq!(cache.get::<u64>("answer").await.unwrap(), Some(42));
        assert_eq!(cache.invalidate_tag("answers").await.unwrap(), 1);
        assert!(!cache.contains_key("answer").await);

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.local_misses, 1);
        assert_eq!(diagnostics.remote_hits, 1);
        assert_eq!(diagnostics.hydrations, 1);
        assert!(!diagnostics.has_router_errors());

        let hot_remote = read_through.hot_remote_diagnostics();
        assert!(hot_remote.enabled);
        assert_eq!(hot_remote.hydrations, 1);
        assert_eq!(hot_remote.tracked_entries, 1);
        assert_eq!(hot_remote.skipped_hydrations, 0);

        let fill = cache.cluster_fill_counters();
        assert_eq!(fill.owner_load_success, 0);
        assert_eq!(fill.owner_load_errors, 0);
        assert_eq!(fill.remote_fetch_success, 1);
        assert_eq!(fill.remote_fetch_errors, 0);
        assert_eq!(fill.hot_cache_hits, 0);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn hot_remote_ttl_override_expires_hydrated_remote_entry() {
        let store = MemoryPeerFetchStore::new();
        store.put("answer", encoded_u64(42).await);
        let (base_url, shutdown, server) = spawn_server(service_with_store(store).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone()).hot_remote_policy(
            HotRemoteCachePolicy::new()
                .ttl(Duration::from_millis(20))
                .max_entries(10),
        );

        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer", 7),
                CacheOptions::new()
                    .ttl(Duration::from_secs(60))
                    .tag("answers"),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::RemoteHit);
        assert!(outcome.hydrated);
        assert_eq!(cache.get::<u64>("answer").await.unwrap(), Some(42));

        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(!cache.contains_key("answer").await);

        let hot_remote = read_through.hot_remote_diagnostics();
        assert_eq!(hot_remote.ttl_millis, Some(20));
        assert_eq!(hot_remote.max_entries, Some(10));
        assert_eq!(hot_remote.hydrations, 1);
        assert_eq!(hot_remote.pressure_evictions, 0);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn hot_remote_capacity_evicts_oldest_hydrated_remote_entry() {
        let store = MemoryPeerFetchStore::new();
        store.put("answer:a", encoded_u64(1).await);
        store.put("answer:b", encoded_u64(2).await);
        let (base_url, shutdown, server) = spawn_server(service_with_store(store).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone())
            .hot_remote_policy(HotRemoteCachePolicy::new().max_entries(1));

        let first = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer:a", 7),
                CacheOptions::new().tag("answers"),
            )
            .await
            .unwrap();
        assert_eq!(first.status, PeerFetchReadThroughStatus::RemoteHit);
        assert!(cache.contains_key("answer:a").await);

        let second = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer:b", 7),
                CacheOptions::new().tag("answers"),
            )
            .await
            .unwrap();
        assert_eq!(second.status, PeerFetchReadThroughStatus::RemoteHit);

        assert!(!cache.contains_key("answer:a").await);
        assert_eq!(cache.get::<u64>("answer:b").await.unwrap(), Some(2));

        let hot_remote = read_through.hot_remote_diagnostics();
        assert_eq!(hot_remote.max_entries, Some(1));
        assert_eq!(hot_remote.tracked_entries, 1);
        assert_eq!(hot_remote.hydrations, 2);
        assert_eq!(hot_remote.pressure_evictions, 1);
        assert!(hot_remote.has_pressure_evictions());

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn read_through_without_hydration_preserves_remote_value_only() {
        let store = MemoryPeerFetchStore::new();
        store.put("answer", encoded_u64(42).await);
        let (base_url, shutdown, server) = spawn_server(service_with_store(store).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone()).without_hydration();

        assert_eq!(read_through.cache().stats().total_requests(), 0);
        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer", 7),
                CacheOptions::new().tag("answers"),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::RemoteHit);
        assert!(outcome.is_remote_hit());
        assert_eq!(
            outcome.value.as_deref(),
            Some(encoded_u64(42).await.as_ref())
        );
        assert!(!outcome.hydrated);
        assert!(!cache.contains_key("answer").await);

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.remote_hits, 1);
        assert_eq!(diagnostics.hydrations, 0);

        let hot_remote = read_through.hot_remote_diagnostics();
        assert!(!hot_remote.enabled);
        assert_eq!(hot_remote.hydrations, 0);
        assert_eq!(hot_remote.skipped_hydrations, 1);
        assert_eq!(hot_remote.tracked_entries, 0);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn hot_remote_disabled_policy_skips_hydration_without_legacy_toggle() {
        let store = MemoryPeerFetchStore::new();
        store.put("answer", encoded_u64(42).await);
        let (base_url, shutdown, server) = spawn_server(service_with_store(store).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone())
            .hot_remote_policy(HotRemoteCachePolicy::disabled());

        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer", 7),
                CacheOptions::new().tag("answers"),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::RemoteHit);
        assert!(!outcome.hydrated);
        assert!(!cache.contains_key("answer").await);

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.remote_hits, 1);
        assert_eq!(diagnostics.hydrations, 0);

        let hot_remote = read_through.hot_remote_diagnostics();
        assert!(!hot_remote.enabled);
        assert_eq!(hot_remote.skipped_hydrations, 1);
        assert_eq!(hot_remote.tracked_entries, 0);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn read_through_remote_miss_does_not_hydrate() {
        let (base_url, shutdown, server) =
            spawn_server(service_with_store(MemoryPeerFetchStore::new()).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone());

        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "missing", 7),
                CacheOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::RemoteMiss);
        assert!(outcome.is_remote_miss());
        assert!(!outcome.hydrated);
        assert!(!cache.contains_key("missing").await);

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.remote_misses, 1);
        assert_eq!(diagnostics.hydrations, 0);
        assert_eq!(diagnostics.total_misses(), 2);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn read_through_missing_endpoint_reports_router_error_without_hydration() {
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone());

        let outcome = read_through
            .fetch_encoded(
                decision_with_member(member_without_endpoint(), "answer"),
                CacheOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::MissingEndpoint);
        assert!(outcome.is_router_error());
        assert!(!outcome.hydrated);
        assert!(!cache.contains_key("answer").await);

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.router_errors, 1);
        assert!(diagnostics.has_router_errors());
    }

    #[tokio::test]
    async fn read_through_generation_mismatch_never_hydrates_stale_value() {
        let store = MemoryPeerFetchStore::new();
        store.put("answer", encoded_u64(42).await);
        let (base_url, shutdown, server) = spawn_server(service_with_store(store).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone());

        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer", 6),
                CacheOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(
            outcome.status,
            PeerFetchReadThroughStatus::GenerationMismatch
        );
        assert!(outcome.is_router_error());
        assert!(!outcome.hydrated);
        assert!(!cache.contains_key("answer").await);

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.router_errors, 1);
        assert_eq!(diagnostics.hydrations, 0);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn read_through_owner_then_local_can_fallback_to_local_hit() {
        let (base_url, shutdown, server) =
            spawn_server(service_with_store(MemoryPeerFetchStore::new()).routes()).await;
        let cache = HydraCache::local().build();
        cache
            .put("answer", 42_u64, CacheOptions::new())
            .await
            .unwrap();
        let read_through =
            PeerFetchReadThrough::new(cache).policy(PeerFetchReadThroughPolicy::OwnerThenLocal);

        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer", 7),
                CacheOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::LocalHit);
        assert!(outcome.is_local_hit());

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.remote_misses, 1);
        assert_eq!(diagnostics.local_hits, 1);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn read_through_owner_only_skips_local_cache() {
        let (base_url, shutdown, server) =
            spawn_server(service_with_store(MemoryPeerFetchStore::new()).routes()).await;
        let cache = HydraCache::local().build();
        cache
            .put("answer", 42_u64, CacheOptions::new())
            .await
            .unwrap();
        let read_through =
            PeerFetchReadThrough::new(cache).policy(PeerFetchReadThroughPolicy::OwnerOnly);

        let outcome = read_through
            .fetch_encoded(
                decision_with_endpoint(&base_url, "answer", 7),
                CacheOptions::new(),
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, PeerFetchReadThroughStatus::RemoteMiss);
        assert_eq!(read_through.diagnostics().local_hits, 0);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_read_through_for_same_key_shares_remote_route_and_hydration() {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = DelayedPeerFetchStore {
            value: encoded_u64(42).await,
            calls: calls.clone(),
            delay: Duration::from_millis(40),
        };
        let app = AxumPeerFetchService::new("member-a", ClusterGeneration::new(7), Arc::new(store))
            .routes();
        let (base_url, shutdown, server) = spawn_server(app).await;
        let cache = HydraCache::local().build();
        let read_through = Arc::new(PeerFetchReadThrough::new(cache.clone()));

        let mut tasks = Vec::new();
        for _ in 0..8 {
            let read_through = read_through.clone();
            let decision = decision_with_endpoint(&base_url, "answer", 7);
            tasks.push(tokio::spawn(async move {
                read_through
                    .fetch_encoded(decision, CacheOptions::new().tag("answers"))
                    .await
            }));
        }

        for task in tasks {
            let outcome = task.await.unwrap().unwrap();
            assert_eq!(outcome.status, PeerFetchReadThroughStatus::RemoteHit);
            assert!(outcome.is_remote_hit());
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.get::<u64>("answer").await.unwrap(), Some(42));

        let diagnostics = read_through.diagnostics();
        assert_eq!(diagnostics.attempts, 8);
        assert_eq!(diagnostics.remote_hits, 1);
        assert_eq!(diagnostics.hydrations, 1);
        assert!(diagnostics.in_flight_joins >= 1);
        assert_eq!(read_through.router().diagnostics().attempts, 1);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn owner_load_route_returns_hit_loaded_miss_and_rejections() {
        let cache = HydraCache::local().build();
        cache
            .put("cached", 7_u64, CacheOptions::new())
            .await
            .unwrap();
        let registry = OwnerLoadRegistry::new()
            .register("answers.by-id", |request| async move {
                Ok(Some(OwnerLoadValue::encoded(
                    encoded_u64(request.arg_u64("id").unwrap()).await,
                    request.cache_options(),
                )))
            })
            .register("answers.optional", |_| async { Ok(None) });
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            cache.clone(),
            registry,
        );
        let route_factory = AxumOwnerLoadService::new(service.clone());
        assert_eq!(route_factory.service().owner().as_str(), "member-a");
        assert!(format!("{route_factory:?}").contains("AxumOwnerLoadService"));
        let app = route_factory.routes();

        let hit = app
            .clone()
            .oneshot(owner_load_json_request(owner_load_request(
                "cached", "missing", 7,
            )))
            .await
            .unwrap();
        assert_eq!(hit.status(), StatusCode::OK);
        let hit: OwnerLoadResponse = response_json(hit).await;
        assert!(matches!(hit, OwnerLoadResponse::Hit(_)));
        assert_eq!(hit.decode_value().unwrap().unwrap(), encoded_u64(7).await);

        let loaded = app
            .clone()
            .oneshot(owner_load_json_request(
                owner_load_request("answer", "answers.by-id", 7)
                    .with_args(OwnerLoadArgs::new().arg("id", 42_u64)),
            ))
            .await
            .unwrap();
        assert_eq!(loaded.status(), StatusCode::OK);
        let loaded: OwnerLoadResponse = response_json(loaded).await;
        assert!(loaded.is_loaded());
        assert_eq!(cache.get::<u64>("answer").await.unwrap(), Some(42));

        let miss = app
            .clone()
            .oneshot(owner_load_json_request(owner_load_request(
                "missing",
                "answers.optional",
                7,
            )))
            .await
            .unwrap();
        assert_eq!(miss.status(), StatusCode::OK);
        let miss: OwnerLoadResponse = response_json(miss).await;
        assert!(miss.is_miss());

        let stale = app
            .clone()
            .oneshot(owner_load_json_request(owner_load_request(
                "other",
                "answers.by-id",
                6,
            )))
            .await
            .unwrap();
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        let stale: OwnerLoadResponse = response_json(stale).await;
        assert!(stale.is_rejected());

        let missing_loader = app
            .oneshot(owner_load_json_request(owner_load_request(
                "other", "missing", 7,
            )))
            .await
            .unwrap();
        assert_eq!(missing_loader.status(), StatusCode::NOT_FOUND);
        let missing_loader: OwnerLoadResponse = response_json(missing_loader).await;
        assert!(missing_loader.is_rejected());

        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.attempts, 5);
        assert_eq!(diagnostics.owner_hits, 1);
        assert_eq!(diagnostics.loaded, 1);
        assert_eq!(diagnostics.misses, 1);
        assert_eq!(diagnostics.rejections, 2);
    }

    #[tokio::test]
    async fn http_owner_load_round_trips_against_axum_server() {
        let registry = OwnerLoadRegistry::new().register("answers.by-id", |request| async move {
            Ok(Some(OwnerLoadValue::encoded(
                encoded_u64(request.arg_u64("id").unwrap()).await,
                request.cache_options(),
            )))
        });
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            registry,
        );
        let app = AxumOwnerLoadService::new(service.clone()).routes();
        let (base_url, shutdown, server) = spawn_server(app).await;
        let client = HttpOwnerLoad::for_base_url(&base_url);

        assert_eq!(
            client.endpoint(),
            format!("{base_url}{DEFAULT_OWNER_LOAD_PATH}")
        );

        let response = client
            .load(
                owner_load_request("answer", "answers.by-id", 7)
                    .with_args(OwnerLoadArgs::new().arg("id", 42_u64)),
            )
            .await
            .unwrap();
        assert!(response.is_loaded());
        assert_eq!(
            response.decode_value().unwrap().unwrap(),
            encoded_u64(42).await
        );

        let cached = client
            .load(owner_load_request("answer", "answers.by-id", 7))
            .await
            .unwrap();
        assert!(matches!(cached, OwnerLoadResponse::Hit(_)));

        let stale = client
            .load(owner_load_request("answer", "answers.by-id", 6))
            .await
            .unwrap();
        assert!(stale.is_rejected());

        let diagnostics = service.diagnostics();
        assert_eq!(diagnostics.attempts, 3);
        assert_eq!(diagnostics.loaded, 1);
        assert_eq!(diagnostics.owner_hits, 1);
        assert_eq!(diagnostics.rejections, 1);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn auth_and_strict_wire_owner_load_route_and_client_round_trip() {
        let registry = OwnerLoadRegistry::new().register("answers.by-id", |request| async move {
            Ok(Some(OwnerLoadValue::encoded(
                Bytes::from_static(b"encoded"),
                request.cache_options(),
            )))
        });
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            registry,
        );
        let auth = HttpTransportAuth::token("secret");
        let route = AxumOwnerLoadService::new(service)
            .with_auth(auth.clone())
            .with_wire_compatibility(HttpWireCompatibility::strict_current());

        assert!(route.auth().is_some());
        assert!(route.wire().requires_header());

        let missing = route
            .routes()
            .oneshot(owner_load_json_request(owner_load_request(
                "answer",
                "answers.by-id",
                7,
            )))
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
        let missing: OwnerLoadResponse = response_json(missing).await;
        assert!(missing.is_rejected());

        let (base_url, shutdown, server) = spawn_server(route.routes()).await;
        let client = HttpOwnerLoad::for_base_url(&base_url).with_auth(auth);
        let response = client
            .load(owner_load_request("answer", "answers.by-id", 7))
            .await
            .unwrap();

        assert!(matches!(response, OwnerLoadResponse::Loaded(_)));

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn owner_load_read_through_local_hit_uses_descriptor_key_and_skips_http() {
        let cache = HydraCache::local().build();
        cache
            .put("explicit-answer", 42_u64, CacheOptions::new())
            .await
            .unwrap();
        let read_through = PeerFetchReadThrough::new(cache);

        let outcome = read_through
            .get_or_load_encoded(
                decision_with_endpoint("http://127.0.0.1:9", "decision-answer", 7),
                OwnerLoadDescriptor::new("answers.by-id").key("explicit-answer"),
            )
            .await
            .unwrap();

        assert_eq!(outcome.key, "explicit-answer");
        assert_eq!(outcome.status, OwnerLoadReadThroughStatus::LocalHit);
        assert!(outcome.is_hit());
        assert_eq!(outcome.value, Some(encoded_u64(42).await));

        let diagnostics = read_through.owner_load_diagnostics();
        assert_eq!(diagnostics.attempts, 1);
        assert_eq!(diagnostics.local_hits, 1);
        assert_eq!(diagnostics.local_misses, 0);
        assert_eq!(diagnostics.total_hits(), 1);
        assert!(!diagnostics.has_errors());
    }

    #[tokio::test]
    async fn owner_load_read_through_remote_loaded_hydrates_then_hits_local_cache() {
        let calls = Arc::new(AtomicUsize::new(0));
        let registry = OwnerLoadRegistry::new().register("answers.by-id", {
            let calls = calls.clone();
            move |request| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(OwnerLoadValue::encoded(
                        encoded_u64(request.arg_u64("id").unwrap()).await,
                        request.cache_options(),
                    )))
                }
            }
        });
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            registry,
        );
        let (base_url, shutdown, server) =
            spawn_server(AxumOwnerLoadService::new(service).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone());
        let descriptor = OwnerLoadDescriptor::new("answers.by-id")
            .tag("answers")
            .arg("id", 42_u64);

        let first = read_through
            .get_or_load_encoded(
                decision_with_endpoint(&base_url, "answer", 7),
                descriptor.clone(),
            )
            .await
            .unwrap();

        assert_eq!(first.key, "answer");
        assert_eq!(first.status, OwnerLoadReadThroughStatus::RemoteLoaded);
        assert!(first.is_remote_loaded());
        assert!(first.hydrated);
        assert!(matches!(first.response, Some(OwnerLoadResponse::Loaded(_))));
        assert_eq!(first.value, Some(encoded_u64(42).await));
        assert_eq!(cache.get::<u64>("answer").await.unwrap(), Some(42));

        let second = read_through
            .get_or_load_encoded(decision_with_endpoint(&base_url, "answer", 7), descriptor)
            .await
            .unwrap();

        assert_eq!(second.status, OwnerLoadReadThroughStatus::LocalHit);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let diagnostics = read_through.owner_load_diagnostics();
        assert_eq!(diagnostics.attempts, 2);
        assert_eq!(diagnostics.local_hits, 1);
        assert_eq!(diagnostics.local_misses, 1);
        assert_eq!(diagnostics.remote_loaded, 1);
        assert_eq!(diagnostics.hydrations, 1);
        assert_eq!(diagnostics.total_hits(), 2);

        let hot_remote = read_through.hot_remote_diagnostics();
        assert!(hot_remote.enabled);
        assert_eq!(hot_remote.hydrations, 1);
        assert_eq!(hot_remote.tracked_entries, 1);

        let fill = cache.cluster_fill_counters();
        assert_eq!(fill.owner_load_success, 1);
        assert_eq!(fill.owner_load_errors, 0);
        assert_eq!(fill.remote_fetch_success, 0);
        assert_eq!(fill.remote_fetch_errors, 0);
        assert_eq!(fill.hot_cache_hits, 1);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn owner_load_read_through_without_hydration_preserves_remote_only_flow() {
        let calls = Arc::new(AtomicUsize::new(0));
        let registry = OwnerLoadRegistry::new().register("answers.by-id", {
            let calls = calls.clone();
            move |request| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(OwnerLoadValue::encoded(
                        encoded_u64(request.arg_u64("id").unwrap()).await,
                        request.cache_options(),
                    )))
                }
            }
        });
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            registry,
        );
        let (base_url, shutdown, server) =
            spawn_server(AxumOwnerLoadService::new(service).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone()).without_hydration();
        let descriptor = OwnerLoadDescriptor::new("answers.by-id").arg("id", 42_u64);

        let first = read_through
            .get_or_load_encoded(
                decision_with_endpoint(&base_url, "answer", 7),
                descriptor.clone(),
            )
            .await
            .unwrap();
        let second = read_through
            .get_or_load_encoded(decision_with_endpoint(&base_url, "answer", 7), descriptor)
            .await
            .unwrap();

        assert_eq!(first.status, OwnerLoadReadThroughStatus::RemoteLoaded);
        assert_eq!(second.status, OwnerLoadReadThroughStatus::RemoteHit);
        assert!(!first.hydrated);
        assert!(!second.hydrated);
        assert_eq!(cache.get::<u64>("answer").await.unwrap(), None);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let diagnostics = read_through.owner_load_diagnostics();
        assert_eq!(diagnostics.local_misses, 2);
        assert_eq!(diagnostics.remote_loaded, 1);
        assert_eq!(diagnostics.remote_hits, 1);
        assert_eq!(diagnostics.hydrations, 0);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn owner_load_read_through_reports_no_owner_and_missing_endpoint() {
        let read_through = PeerFetchReadThrough::new(HydraCache::local().build());
        let descriptor = OwnerLoadDescriptor::new("answers.by-id").arg("id", 42_u64);

        let no_owner = read_through
            .get_or_load_encoded(
                ClusterOwnershipDecision {
                    key: "answer".to_owned(),
                    owner: None,
                    member_count: 0,
                    resolver: "test",
                },
                descriptor.clone(),
            )
            .await
            .unwrap();

        assert_eq!(no_owner.status, OwnerLoadReadThroughStatus::NoOwner);
        assert!(no_owner.is_route_error());

        let missing_endpoint = read_through
            .get_or_load_encoded(
                decision_with_member(member_without_endpoint(), "answer"),
                descriptor,
            )
            .await
            .unwrap();

        assert_eq!(
            missing_endpoint.status,
            OwnerLoadReadThroughStatus::MissingEndpoint
        );
        assert!(missing_endpoint.is_route_error());

        let diagnostics = read_through.owner_load_diagnostics();
        assert_eq!(diagnostics.attempts, 2);
        assert_eq!(diagnostics.routing_errors, 2);
        assert!(diagnostics.has_errors());
    }

    #[tokio::test]
    async fn owner_load_read_through_rejections_and_failures_do_not_hydrate() {
        let registry = OwnerLoadRegistry::new().register("answers.failing", |_| async {
            Err(CacheError::Backend("forced loader failure".to_owned()))
        });
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            registry,
        );
        let (base_url, shutdown, server) =
            spawn_server(AxumOwnerLoadService::new(service).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = PeerFetchReadThrough::new(cache.clone());

        let rejected = read_through
            .get_or_load_encoded(
                decision_with_endpoint(&base_url, "missing-loader", 7),
                OwnerLoadDescriptor::new("answers.missing").arg("id", 1_u64),
            )
            .await
            .unwrap();
        let failed = read_through
            .get_or_load_encoded(
                decision_with_endpoint(&base_url, "failing-loader", 7),
                OwnerLoadDescriptor::new("answers.failing").arg("id", 2_u64),
            )
            .await
            .unwrap();

        assert_eq!(rejected.status, OwnerLoadReadThroughStatus::Rejected);
        assert_eq!(failed.status, OwnerLoadReadThroughStatus::Failed);
        assert!(!rejected.hydrated);
        assert!(!failed.hydrated);
        assert_eq!(cache.get::<u64>("missing-loader").await.unwrap(), None);
        assert_eq!(cache.get::<u64>("failing-loader").await.unwrap(), None);

        let diagnostics = read_through.owner_load_diagnostics();
        assert_eq!(diagnostics.rejections, 1);
        assert_eq!(diagnostics.failures, 1);
        assert_eq!(diagnostics.hydrations, 0);
        assert!(diagnostics.has_errors());

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_owner_load_read_through_for_same_key_shares_remote_route() {
        let calls = Arc::new(AtomicUsize::new(0));
        let registry = OwnerLoadRegistry::new().register("answers.slow", {
            let calls = calls.clone();
            move |request| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(40)).await;
                    Ok(Some(OwnerLoadValue::encoded(
                        encoded_u64(request.arg_u64("id").unwrap()).await,
                        request.cache_options(),
                    )))
                }
            }
        });
        let service = OwnerLoadService::new(
            "member-a",
            ClusterGeneration::new(7),
            HydraCache::local().build(),
            registry,
        );
        let (base_url, shutdown, server) =
            spawn_server(AxumOwnerLoadService::new(service.clone()).routes()).await;
        let cache = HydraCache::local().build();
        let read_through = Arc::new(PeerFetchReadThrough::new(cache.clone()));

        let mut tasks = Vec::new();
        for _ in 0..8 {
            let read_through = read_through.clone();
            let decision = decision_with_endpoint(&base_url, "answer", 7);
            let descriptor = OwnerLoadDescriptor::new("answers.slow").arg("id", 42_u64);
            tasks.push(tokio::spawn(async move {
                read_through.get_or_load_encoded(decision, descriptor).await
            }));
        }

        for task in tasks {
            let outcome = task.await.unwrap().unwrap();
            assert_eq!(outcome.status, OwnerLoadReadThroughStatus::RemoteLoaded);
            assert_eq!(outcome.value, Some(encoded_u64(42).await));
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(service.diagnostics().attempts, 1);
        assert_eq!(cache.get::<u64>("answer").await.unwrap(), Some(42));

        let diagnostics = read_through.owner_load_diagnostics();
        assert_eq!(diagnostics.attempts, 8);
        assert_eq!(diagnostics.remote_loaded, 1);
        assert_eq!(diagnostics.hydrations, 1);
        assert!(diagnostics.in_flight_joins >= 1);

        shutdown.send(()).unwrap();
        server.await.unwrap();
    }

    #[test]
    fn http_owner_load_identity_validation_is_strict_for_values_and_misses() {
        let wrong_hit = OwnerLoadResponse::Hit(OwnerLoadHit::new(
            "member-b",
            "user:42",
            "users.by-id",
            Bytes::from_static(b"value"),
        ));
        let error = validate_owner_load_identity(&wrong_hit, "member-a", "user:42").unwrap_err();
        assert!(error.to_string().contains("identity mismatch"));

        let wrong_miss =
            OwnerLoadResponse::Miss(OwnerLoadMiss::new("member-a", "other", "users.by-id"));
        let error = validate_owner_load_identity(&wrong_miss, "member-a", "user:42").unwrap_err();
        assert!(error.to_string().contains("miss identity mismatch"));

        let rejected = OwnerLoadResponse::Rejected(OwnerLoadRejection::new(
            OwnerLoadRejectionCode::WrongOwner,
            "wrong owner",
        ));
        validate_owner_load_identity(&rejected, "member-a", "user:42").unwrap();
    }

    #[tokio::test]
    async fn service_reports_store_errors_and_exposes_owner_metadata() {
        let service = AxumPeerFetchService::new(
            "member-a",
            ClusterGeneration::new(7),
            Arc::new(FailingStore),
        );

        assert_eq!(service.owner().as_str(), "member-a");
        assert_eq!(service.generation(), ClusterGeneration::new(7));
        assert!(format!("{service:?}").contains("AxumPeerFetchService"));

        let response = service
            .routes()
            .oneshot(json_request(PeerFetchHttpRequest {
                owner: "member-a".to_owned(),
                key: "boom".to_owned(),
                generation: Some(7),
            }))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body: PeerFetchHttpErrorBody = response_json(response).await;
        assert_eq!(body.code, "store-error");
        assert!(body.message.contains("forced store failure"));
        assert_eq!(body.requested_generation, Some(7));
        assert_eq!(body.current_generation, Some(7));
    }

    fn owner_load_request(key: &str, loader: &str, generation: u64) -> OwnerLoadRequest {
        OwnerLoadRequest {
            owner: "member-a".to_owned(),
            key: key.to_owned(),
            loader: loader.to_owned(),
            tags: vec![key.to_owned()],
            ttl_ms: Some(60_000),
            args: OwnerLoadArgs::new(),
            generation: Some(generation),
            request_id: format!("request:{key}"),
        }
    }

    fn service_with_store(store: MemoryPeerFetchStore) -> AxumPeerFetchService {
        AxumPeerFetchService::new("member-a", ClusterGeneration::new(7), Arc::new(store))
    }

    async fn encoded_u64(value: u64) -> Bytes {
        let cache = HydraCache::local().build();
        cache
            .put("value", value, CacheOptions::new())
            .await
            .unwrap();
        cache
            .get_encoded("value")
            .await
            .unwrap()
            .expect("encoded value")
    }

    struct DelayedPeerFetchStore {
        value: Bytes,
        calls: Arc<AtomicUsize>,
        delay: Duration,
    }

    #[async_trait::async_trait]
    impl PeerFetchStore for DelayedPeerFetchStore {
        async fn get_encoded(&self, key: &str) -> CacheResult<Option<Bytes>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            if key == "answer" {
                Ok(Some(self.value.clone()))
            } else {
                Ok(None)
            }
        }
    }

    struct FailingStore;

    #[async_trait::async_trait]
    impl PeerFetchStore for FailingStore {
        async fn get_encoded(&self, _key: &str) -> CacheResult<Option<Bytes>> {
            Err(CacheError::Backend("forced store failure".to_owned()))
        }
    }

    fn decision_with_endpoint(
        base_url: &str,
        key: impl Into<String>,
        generation: u64,
    ) -> ClusterOwnershipDecision {
        decision_with_member(member_with_endpoint(base_url, generation), key)
    }

    fn decision_with_member(
        owner: ClusterMember,
        key: impl Into<String>,
    ) -> ClusterOwnershipDecision {
        ClusterOwnershipDecision {
            key: key.into(),
            owner: Some(owner),
            member_count: 1,
            resolver: "test",
        }
    }

    fn member_with_endpoint(base_url: &str, generation: u64) -> ClusterMember {
        let candidate = ClusterCandidate::member("member-a")
            .generation(ClusterGeneration::new(generation))
            .peer_fetch_base_url(base_url);

        ClusterMember {
            node_id: candidate.node_id,
            generation: candidate.generation,
            role: candidate.role,
            epoch: ClusterEpoch::new(1),
            endpoints: candidate.endpoints,
            metadata: candidate.metadata,
        }
    }

    fn member_without_endpoint() -> ClusterMember {
        ClusterMember {
            node_id: ClusterNodeId::from("member-a"),
            generation: ClusterGeneration::new(7),
            role: ClusterRole::Member,
            epoch: ClusterEpoch::new(1),
            endpoints: ClusterEndpoints::new(),
            metadata: Default::default(),
        }
    }

    fn json_request<T>(body: T) -> Request<Body>
    where
        T: Serialize,
    {
        json_request_to(DEFAULT_PEER_FETCH_PATH, body)
    }

    fn owner_load_json_request<T>(body: T) -> Request<Body>
    where
        T: Serialize,
    {
        json_request_to(DEFAULT_OWNER_LOAD_PATH, body)
    }

    fn json_request_to<T>(path: &str, body: T) -> Request<Body>
    where
        T: Serialize,
    {
        Request::builder()
            .method("POST")
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap()
    }

    async fn response_json<T>(response: Response) -> T
    where
        T: DeserializeOwned,
    {
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn spawn_server(
        app: Router,
    ) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .unwrap();
        });

        (format!("http://{addr}"), shutdown_tx, server)
    }
}
