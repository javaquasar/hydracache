//! Java/Spring migration contract for legacy Hazelcast consumers.
//!
//! The real Java artifacts live outside the Cargo workspace, but their public
//! behavior is anchored here so the Rust protocol gate can enforce stable error
//! mapping, safe codec registration, and fail-loud unsupported Hazelcast APIs.

use std::collections::BTreeMap;

use thiserror::Error;

use crate::{ClientErrorCode, ClientErrorEnvelope};

/// Current Java migration contract version.
pub const JAVA_MIGRATION_CONTRACT_VERSION: u16 = 1;

/// Checked-in manifest of Hazelcast APIs that HydraCache refuses to emulate.
pub const UNSUPPORTED_HAZELCAST_APIS_MANIFEST: &str =
    include_str!("../manifests/unsupported_hazelcast_apis.txt");

/// Supported Spring Boot generations for the 0.49 Java migration contract.
pub const SUPPORTED_SPRING_BOOT_GENERATIONS: &[u8] = &[2, 3, 4];

/// Java exception kind documented for the Java client and Spring toolkit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaExceptionKind {
    /// No common protocol or an unsupported protocol feature was used.
    Protocol,
    /// The request did not carry an accepted identity.
    Authentication,
    /// The identity is known but not allowed to perform the operation.
    Authorization,
    /// Tenant quota was exceeded.
    QuotaExceeded,
    /// The caller is over its rate or fairness budget.
    RateLimited,
    /// Data-residency governance refused the operation.
    ResidencyDenied,
    /// Request frame or value bytes exceeded a configured bound.
    PayloadTooLarge,
    /// Request deadline expired.
    Timeout,
    /// Conditional or optimistic operation conflicted.
    Conflict,
    /// The backend is temporarily unavailable.
    BackendUnavailable,
    /// The client sent a malformed binary frame.
    MalformedFrame,
}

impl JavaExceptionKind {
    /// Stable Java class name for this exception kind.
    pub const fn class_name(self) -> &'static str {
        match self {
            Self::Protocol => "HydraCacheProtocolException",
            Self::Authentication => "HydraCacheAuthenticationException",
            Self::Authorization => "HydraCacheAuthorizationException",
            Self::QuotaExceeded => "HydraCacheQuotaExceededException",
            Self::RateLimited => "HydraCacheRateLimitedException",
            Self::ResidencyDenied => "HydraCacheResidencyDeniedException",
            Self::PayloadTooLarge => "HydraCachePayloadTooLargeException",
            Self::Timeout => "HydraCacheTimeoutException",
            Self::Conflict => "HydraCacheConflictException",
            Self::BackendUnavailable => "HydraCacheBackendUnavailableException",
            Self::MalformedFrame => "HydraCacheMalformedFrameException",
        }
    }
}

/// Java-facing mapping for one stable protocol error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaExceptionMapping {
    /// Documented exception kind.
    pub kind: JavaExceptionKind,
    /// Java class name the SDK must expose.
    pub class_name: &'static str,
    /// Retryability copied from the protocol envelope.
    pub retryable: bool,
    /// Whether the Java exception must retain the request id.
    pub preserves_request_id: bool,
    /// Whether the Java exception must retain retry-after metadata when present.
    pub preserves_retry_after: bool,
}

/// Return the Java exception mapping for a stable protocol error.
pub fn java_exception_mapping(error: &ClientErrorEnvelope) -> JavaExceptionMapping {
    let kind = match error.code {
        ClientErrorCode::IncompatibleVersion => JavaExceptionKind::Protocol,
        ClientErrorCode::Unauthenticated => JavaExceptionKind::Authentication,
        ClientErrorCode::Unauthorized => JavaExceptionKind::Authorization,
        ClientErrorCode::TenantQuota => JavaExceptionKind::QuotaExceeded,
        ClientErrorCode::RateLimited => JavaExceptionKind::RateLimited,
        ClientErrorCode::ResidencyDenied => JavaExceptionKind::ResidencyDenied,
        ClientErrorCode::TooLarge => JavaExceptionKind::PayloadTooLarge,
        ClientErrorCode::DeadlineExceeded => JavaExceptionKind::Timeout,
        ClientErrorCode::Conflict => JavaExceptionKind::Conflict,
        ClientErrorCode::BackendUnavailable => JavaExceptionKind::BackendUnavailable,
        ClientErrorCode::MalformedFrame => JavaExceptionKind::MalformedFrame,
    };

    JavaExceptionMapping {
        kind,
        class_name: kind.class_name(),
        retryable: error.retryable,
        preserves_request_id: true,
        preserves_retry_after: error.retry_after_ms.is_some(),
    }
}

/// Java application topology mode for migration from Hazelcast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaClientTopology {
    /// Application JVM connects as a client.
    Client,
    /// Application JVM tries to become a data-owning member.
    Member,
    /// Toolkit is disabled and does not create a client.
    None,
}

/// Public identity source for Java clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaClientIdentityMode {
    /// Bearer/API token supplied by the application.
    Token,
    /// Client certificate identity over mTLS.
    Mtls,
}

/// Retry/backoff defaults for Java clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JavaRetryBackoff {
    /// Maximum attempts including the original call.
    pub max_attempts: u8,
    /// Initial backoff in milliseconds.
    pub initial_backoff_ms: u64,
    /// Maximum backoff in milliseconds.
    pub max_backoff_ms: u64,
}

impl Default for JavaRetryBackoff {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 25,
            max_backoff_ms: 1_000,
        }
    }
}

/// Java client runtime settings shared by the Boot 2/3/4 starters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaClientRuntimeConfig {
    /// Client endpoints.
    pub endpoints: Vec<String>,
    /// Tenant id carried to W4 isolation.
    pub tenant: String,
    /// Caller-visible client name.
    pub client_name: String,
    /// Whether the Java client may use server-provided routing hints.
    pub smart_routing: bool,
    /// Identity source.
    pub identity: JavaClientIdentityMode,
    /// Application topology mode.
    pub topology: JavaClientTopology,
    /// Retry/backoff policy.
    pub retry: JavaRetryBackoff,
    /// Default request deadline in milliseconds.
    pub deadline_ms: u64,
    /// Whether customizers are allowed to mutate the transport settings.
    pub customizer_hooks_enabled: bool,
}

impl JavaClientRuntimeConfig {
    /// Build client-first defaults.
    pub fn client_first(
        endpoints: impl IntoIterator<Item = impl Into<String>>,
        tenant: impl Into<String>,
        client_name: impl Into<String>,
    ) -> Self {
        Self {
            endpoints: endpoints.into_iter().map(Into::into).collect(),
            tenant: tenant.into(),
            client_name: client_name.into(),
            smart_routing: true,
            identity: JavaClientIdentityMode::Token,
            topology: JavaClientTopology::Client,
            retry: JavaRetryBackoff::default(),
            deadline_ms: 5_000,
            customizer_hooks_enabled: true,
        }
    }

    /// Validate release-0.49 Java client defaults.
    pub fn validate(&self) -> Result<(), JavaMigrationContractError> {
        if self.endpoints.is_empty() {
            return Err(JavaMigrationContractError::InvalidField("endpoints"));
        }
        if self.tenant.trim().is_empty() {
            return Err(JavaMigrationContractError::InvalidField("tenant"));
        }
        if self.client_name.trim().is_empty() {
            return Err(JavaMigrationContractError::InvalidField("client_name"));
        }
        if self.deadline_ms == 0 {
            return Err(JavaMigrationContractError::InvalidField("deadline_ms"));
        }
        if self.retry.max_attempts == 0 {
            return Err(JavaMigrationContractError::InvalidField(
                "retry.max_attempts",
            ));
        }
        if self.topology == JavaClientTopology::Member {
            return Err(JavaMigrationContractError::UnsupportedClientTopology(
                "member",
            ));
        }
        Ok(())
    }
}

/// Spring Cache integration mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpringCacheMode {
    /// Lazy map-backed cache names for legacy `CacheUtil` style code.
    Native,
    /// Bind to JCache when the optional JCache provider is present.
    JCache,
    /// Do not create a Spring `CacheManager`.
    None,
}

impl SpringCacheMode {
    /// Return whether this mode must lazily resolve dynamic cache names.
    pub const fn lazy_dynamic_cache_names(self) -> bool {
        matches!(self, Self::Native)
    }

    /// Return whether this mode requires a JCache provider.
    pub const fn requires_jcache_provider(self) -> bool {
        matches!(self, Self::JCache)
    }
}

/// Java map facade operation exposed by the migration toolkit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaMapOperation {
    /// `HydraCacheMap.get`.
    Get,
    /// `HydraCacheMap.put`.
    Put,
    /// `HydraCacheMap.putIfAbsent`.
    PutIfAbsent,
    /// `HydraCacheMap.remove`.
    Remove,
    /// `HydraCacheMap.containsKey`.
    ContainsKey,
    /// `HydraCacheMap.getAll`.
    GetAll,
    /// `HydraCacheMap.putAll`.
    PutAll,
    /// Key invalidation.
    Invalidate,
    /// Namespace clear for the map.
    ClearNamespace,
    /// Region/namespace eviction.
    EvictRegion,
}

/// Protocol-level operation family for a Java map facade method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaMapProtocolFamily {
    /// Maps to protocol-v1 get.
    Get,
    /// Maps to protocol-v1 put.
    Put,
    /// Maps to protocol-v1 conflict-aware conditional put-if-absent.
    ConditionalPutIfAbsent,
    /// Maps to protocol-v1 invalidation.
    Invalidate,
    /// Maps to protocol-v1 batch get.
    BatchGet,
    /// Maps to protocol-v1 batch put.
    BatchPut,
    /// Maps to protocol-v1 namespace/region eviction.
    EvictRegion,
}

impl JavaMapOperation {
    /// Return the protocol family that backs this facade operation.
    pub const fn protocol_family(self) -> JavaMapProtocolFamily {
        match self {
            Self::Get | Self::ContainsKey => JavaMapProtocolFamily::Get,
            Self::Put => JavaMapProtocolFamily::Put,
            Self::PutIfAbsent => JavaMapProtocolFamily::ConditionalPutIfAbsent,
            Self::Remove | Self::Invalidate => JavaMapProtocolFamily::Invalidate,
            Self::GetAll => JavaMapProtocolFamily::BatchGet,
            Self::PutAll => JavaMapProtocolFamily::BatchPut,
            Self::ClearNamespace | Self::EvictRegion => JavaMapProtocolFamily::EvictRegion,
        }
    }
}

/// Serializer registration kind accepted or rejected by the Java toolkit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaCodecKind {
    /// Explicit codec instance or generated schema.
    Explicit,
    /// Package-scanned `@HydraCacheCodec`.
    CodecAnnotation,
    /// Package-scanned `@HydraCacheSchema`.
    SchemaAnnotation,
    /// Legacy serializer bridge enabled for migration only.
    LegacySerializerBridge,
    /// Reflective fallback serializer.
    ReflectiveFallback,
    /// Java native serialization.
    JavaNativeSerialization,
}

impl JavaCodecKind {
    /// Stable config label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::CodecAnnotation => "codec-annotation",
            Self::SchemaAnnotation => "schema-annotation",
            Self::LegacySerializerBridge => "legacy-serializer-bridge",
            Self::ReflectiveFallback => "reflective-fallback",
            Self::JavaNativeSerialization => "java-native-serialization",
        }
    }
}

/// Codec/schema descriptor registered by the Java migration toolkit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaCodecDescriptor {
    /// Stable codec id.
    pub codec_id: String,
    /// Fully qualified Java type name.
    pub java_type: String,
    /// Schema version for reviewable migrations.
    pub schema_version: u32,
    /// Registration kind.
    pub kind: JavaCodecKind,
}

impl JavaCodecDescriptor {
    /// Create a descriptor.
    pub fn new(
        codec_id: impl Into<String>,
        java_type: impl Into<String>,
        schema_version: u32,
        kind: JavaCodecKind,
    ) -> Result<Self, JavaMigrationContractError> {
        let descriptor = Self {
            codec_id: codec_id.into(),
            java_type: java_type.into(),
            schema_version,
            kind,
        };
        descriptor.validate()?;
        Ok(descriptor)
    }

    /// Create an explicit codec descriptor.
    pub fn explicit(
        codec_id: impl Into<String>,
        java_type: impl Into<String>,
        schema_version: u32,
    ) -> Result<Self, JavaMigrationContractError> {
        Self::new(codec_id, java_type, schema_version, JavaCodecKind::Explicit)
    }

    fn validate(&self) -> Result<(), JavaMigrationContractError> {
        if self.codec_id.trim().is_empty() {
            return Err(JavaMigrationContractError::InvalidField("codec_id"));
        }
        if self.java_type.trim().is_empty() {
            return Err(JavaMigrationContractError::InvalidField("java_type"));
        }
        if self.schema_version == 0 {
            return Err(JavaMigrationContractError::InvalidField("schema_version"));
        }
        Ok(())
    }
}

/// Safe codec registry contract for Java clients.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JavaCodecRegistryContract {
    codecs: BTreeMap<String, JavaCodecDescriptor>,
    legacy_serializer_bridge_enabled: bool,
}

impl JavaCodecRegistryContract {
    /// Create a registry with safe defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Explicitly allow the migration-only legacy serializer bridge.
    pub fn with_legacy_serializer_bridge_enabled(mut self) -> Self {
        self.legacy_serializer_bridge_enabled = true;
        self
    }

    /// Register a descriptor, failing loud on ambiguous or unsafe serializers.
    pub fn register(
        &mut self,
        descriptor: JavaCodecDescriptor,
    ) -> Result<(), JavaMigrationContractError> {
        match descriptor.kind {
            JavaCodecKind::ReflectiveFallback | JavaCodecKind::JavaNativeSerialization => {
                return Err(JavaMigrationContractError::UnsupportedSerializer(
                    descriptor.kind.as_str(),
                ));
            }
            JavaCodecKind::LegacySerializerBridge if !self.legacy_serializer_bridge_enabled => {
                return Err(JavaMigrationContractError::LegacySerializerBridgeDisabled(
                    descriptor.codec_id,
                ));
            }
            _ => {}
        }

        if self.codecs.contains_key(&descriptor.codec_id) {
            return Err(JavaMigrationContractError::AmbiguousCodec {
                codec_id: descriptor.codec_id,
            });
        }

        self.codecs.insert(descriptor.codec_id.clone(), descriptor);
        Ok(())
    }

    /// Return a descriptor by codec id.
    pub fn get(&self, codec_id: &str) -> Option<&JavaCodecDescriptor> {
        self.codecs.get(codec_id)
    }

    /// Number of registered descriptors.
    pub fn len(&self) -> usize {
        self.codecs.len()
    }

    /// Return whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.codecs.is_empty()
    }
}

/// One unsupported Hazelcast API and its migration hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedHazelcastApi {
    /// Hazelcast API surface.
    pub api: String,
    /// Human migration hint.
    pub migration_hint: String,
}

/// Versioned manifest of unsupported Hazelcast APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedHazelcastApiManifest {
    /// Manifest version.
    pub version: u16,
    /// Unsupported APIs.
    pub entries: Vec<UnsupportedHazelcastApi>,
}

impl UnsupportedHazelcastApiManifest {
    /// Parse a manifest and reject unknown future versions.
    pub fn parse(contents: &str) -> Result<Self, JavaMigrationContractError> {
        let mut version = None;
        let mut entries = Vec::new();

        for raw in contents.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(value) = line.strip_prefix("version=") {
                let parsed = value
                    .parse()
                    .map_err(|_| JavaMigrationContractError::InvalidManifest("version"))?;
                version = Some(parsed);
                continue;
            }

            let Some((api, migration_hint)) = line.split_once('|') else {
                return Err(JavaMigrationContractError::InvalidManifest("entry"));
            };
            if api.trim().is_empty() || migration_hint.trim().is_empty() {
                return Err(JavaMigrationContractError::InvalidManifest("entry"));
            }
            entries.push(UnsupportedHazelcastApi {
                api: api.trim().to_owned(),
                migration_hint: migration_hint.trim().to_owned(),
            });
        }

        let version = version.ok_or(JavaMigrationContractError::InvalidManifest("version"))?;
        if version != JAVA_MIGRATION_CONTRACT_VERSION {
            return Err(JavaMigrationContractError::UnsupportedManifestVersion {
                actual: version,
                supported: JAVA_MIGRATION_CONTRACT_VERSION,
            });
        }
        if entries.is_empty() {
            return Err(JavaMigrationContractError::InvalidManifest("entries"));
        }

        Ok(Self { version, entries })
    }

    /// Parse the checked-in manifest.
    pub fn checked_in() -> Result<Self, JavaMigrationContractError> {
        Self::parse(UNSUPPORTED_HAZELCAST_APIS_MANIFEST)
    }

    /// Find a manifest entry by API name.
    pub fn find(&self, api: &str) -> Option<&UnsupportedHazelcastApi> {
        self.entries.iter().find(|entry| entry.api == api)
    }
}

/// Java migration contract errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum JavaMigrationContractError {
    /// A field is missing or invalid.
    #[error("invalid java migration contract field: {0}")]
    InvalidField(&'static str),
    /// Application JVM member mode is unsupported in release 0.49.
    #[error("unsupported java client topology for release 0.49: {0}")]
    UnsupportedClientTopology(&'static str),
    /// Codec id is ambiguous.
    #[error("ambiguous java codec id: {codec_id}")]
    AmbiguousCodec {
        /// Duplicate codec id.
        codec_id: String,
    },
    /// Serializer kind is unsupported.
    #[error("unsupported java serializer kind: {0}")]
    UnsupportedSerializer(&'static str),
    /// Legacy serializer bridge is disabled by default.
    #[error("legacy serializer bridge is disabled for codec id: {0}")]
    LegacySerializerBridgeDisabled(String),
    /// Manifest is malformed.
    #[error("invalid unsupported Hazelcast API manifest: {0}")]
    InvalidManifest(&'static str),
    /// Manifest version is newer than this library supports.
    #[error(
        "unsupported unsupported-Hazelcast-API manifest version {actual}; supported {supported}"
    )]
    UnsupportedManifestVersion {
        /// Actual manifest version.
        actual: u16,
        /// Supported manifest version.
        supported: u16,
    },
}
