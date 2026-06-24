//! Hibernate second-level cache contract types for external JVM providers.
//!
//! This module is intentionally protocol-only. The Java `RegionFactory` lives in
//! a Maven module outside the Cargo workspace; these types define the stable
//! mapping that provider must use over the W1 client protocol.

use serde::{Deserialize, Serialize};

use crate::{
    ClientContext, ClientProtocolError, ClientRequest, Namespace, ReadConsistency, StructuredKey,
    WriteConsistency,
};

/// Supported Hibernate major line for the first provider contract.
pub const HIBERNATE_SUPPORTED_MAJOR: u8 = 6;

/// Human-readable supported Hibernate range for docs and bootstrap checks.
pub const HIBERNATE_SUPPORTED_RANGE: &str = "Hibernate ORM 6.x";

/// Version of the Hibernate-to-HydraCache mapping contract.
pub const HIBERNATE_CONTRACT_VERSION: u16 = 1;

/// Hibernate L2 access strategy as seen by the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum L2AccessMode {
    /// Immutable data: strong reads, cache puts, no write-driven invalidation.
    ReadOnly,
    /// Best-effort invalidation on write.
    NonStrictReadWrite,
    /// Invalidate on transaction completion, driven by the consumer.
    ReadWrite,
    /// Same boundary as `ReadWrite`; HydraCache still does not join the JVM transaction.
    Transactional,
}

impl L2AccessMode {
    /// Return the documented HydraCache consistency mapping for this access mode.
    pub const fn consistency_mapping(self) -> L2ConsistencyMapping {
        match self {
            Self::ReadOnly => L2ConsistencyMapping {
                label: L2ConsistencyLabel::StrongImmutable,
                read: ReadConsistency::Strong,
                write: None,
                immutable: true,
                invalidates_on_write: false,
                invalidates_on_commit: false,
                joins_jvm_transaction: false,
            },
            Self::NonStrictReadWrite => L2ConsistencyMapping {
                label: L2ConsistencyLabel::BestEffortInvalidate,
                read: ReadConsistency::Eventual,
                write: Some(WriteConsistency::Local),
                immutable: false,
                invalidates_on_write: true,
                invalidates_on_commit: false,
                joins_jvm_transaction: false,
            },
            Self::ReadWrite | Self::Transactional => L2ConsistencyMapping {
                label: L2ConsistencyLabel::InvalidateOnCommit,
                read: ReadConsistency::Session,
                write: Some(WriteConsistency::Quorum),
                immutable: false,
                invalidates_on_write: false,
                invalidates_on_commit: true,
                joins_jvm_transaction: false,
            },
        }
    }
}

/// Stable labels used in docs and conformance output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum L2ConsistencyLabel {
    /// Read-only immutable region.
    StrongImmutable,
    /// Non-strict best-effort invalidation region.
    BestEffortInvalidate,
    /// Transaction-boundary invalidation region.
    InvalidateOnCommit,
}

impl L2ConsistencyLabel {
    /// Stable string used by Java conformance reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StrongImmutable => "strong-immutable",
            Self::BestEffortInvalidate => "best-effort-invalidate",
            Self::InvalidateOnCommit => "invalidate-on-commit",
        }
    }
}

/// Full consistency decision for one Hibernate L2 access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct L2ConsistencyMapping {
    /// Stable label.
    pub label: L2ConsistencyLabel,
    /// Read consistency sent in the W1 request context.
    pub read: ReadConsistency,
    /// Write consistency sent for puts/invalidations when relevant.
    pub write: Option<WriteConsistency>,
    /// Region contents are immutable from Hibernate's perspective.
    pub immutable: bool,
    /// Provider invalidates when it observes a write.
    pub invalidates_on_write: bool,
    /// Provider invalidates only from transaction-completion callbacks.
    pub invalidates_on_commit: bool,
    /// Always false: HydraCache never joins the JVM transaction.
    pub joins_jvm_transaction: bool,
}

impl L2ConsistencyMapping {
    /// Build the W1 client context for requests using this mapping.
    pub fn client_context(self) -> ClientContext {
        ClientContext {
            session_token: None,
            read: Some(self.read),
            write: self.write,
            preferred_region: None,
        }
    }
}

/// Hibernate region kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HibernateRegionKind {
    /// Entity region.
    Entity,
    /// Collection region.
    Collection,
    /// Natural-id region.
    NaturalId,
    /// Query result region.
    Query,
    /// Update timestamp region required for query cache correctness.
    Timestamps,
}

impl HibernateRegionKind {
    /// Stable segment used in structured keys.
    pub const fn key_segment(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Collection => "collection",
            Self::NaturalId => "natural-id",
            Self::Query => "query",
            Self::Timestamps => "timestamps",
        }
    }

    /// Query cache behavior for this region kind.
    pub const fn query_cache_behavior(self) -> QueryCacheBehavior {
        match self {
            Self::Query | Self::Timestamps => QueryCacheBehavior::TimestampBulkInvalidation,
            Self::Entity | Self::Collection | Self::NaturalId => QueryCacheBehavior::NotQueryCache,
        }
    }
}

/// Explicit query-cache support stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QueryCacheBehavior {
    /// Query cache is implemented with query-result namespaces plus timestamp invalidation.
    TimestampBulkInvalidation,
    /// This region is not a query-cache region.
    NotQueryCache,
}

/// A Hibernate region mapped to a HydraCache namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionMapping {
    /// Hibernate region name as configured by the app.
    pub region: String,
    /// HydraCache namespace used on the W1 protocol.
    pub ns: Namespace,
    /// Hibernate region kind.
    pub kind: HibernateRegionKind,
    /// Access mode and consistency contract.
    pub mode: L2AccessMode,
}

impl RegionMapping {
    /// Build an explicit region mapping.
    pub fn new(
        region: impl Into<String>,
        ns: Namespace,
        kind: HibernateRegionKind,
        mode: L2AccessMode,
    ) -> Result<Self, ClientProtocolError> {
        let region = region.into();
        if region.trim().is_empty() {
            return Err(ClientProtocolError::InvalidField("hibernate_region"));
        }
        Ok(Self {
            region,
            ns,
            kind,
            mode,
        })
    }

    /// Build the default namespace mapping for a Hibernate region.
    pub fn from_region(
        region: impl Into<String>,
        kind: HibernateRegionKind,
        mode: L2AccessMode,
    ) -> Result<Self, ClientProtocolError> {
        let region = region.into();
        let ns = Namespace::new(format!("hibernate:{}", region.trim()))?;
        Self::new(region, ns, kind, mode)
    }

    /// Return the consistency mapping for this region.
    pub const fn consistency_mapping(&self) -> L2ConsistencyMapping {
        self.mode.consistency_mapping()
    }

    /// Return a W1 request context for this region.
    pub fn client_context(&self) -> ClientContext {
        self.consistency_mapping().client_context()
    }

    /// Build a namespaced structured key for this Hibernate region.
    pub fn key<I, S>(&self, segments: I) -> Result<StructuredKey, ClientProtocolError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut key_segments = vec![self.kind.key_segment().to_owned()];
        key_segments.extend(segments.into_iter().map(Into::into));
        StructuredKey::new(key_segments)
    }

    /// Build a W1 get request for this region.
    pub fn get(&self, key: StructuredKey) -> ClientRequest {
        ClientRequest::Get {
            ns: self.ns.clone(),
            key,
        }
    }

    /// Build a W1 put request for this region.
    pub fn put(&self, key: StructuredKey, value: Vec<u8>, ttl_ms: Option<u64>) -> ClientRequest {
        ClientRequest::Put {
            ns: self.ns.clone(),
            key,
            value,
            ttl_ms,
            dimensions: vec![
                "hibernate".to_owned(),
                self.kind.key_segment().to_owned(),
                self.region.clone(),
            ],
        }
    }

    /// Build a W1 invalidation request for this region.
    pub fn invalidate(&self, key: StructuredKey) -> ClientRequest {
        ClientRequest::Invalidate {
            ns: self.ns.clone(),
            key,
        }
    }

    /// Build a W1 region eviction request for this region's namespace.
    pub fn evict_region(&self) -> ClientRequest {
        ClientRequest::EvictRegion {
            ns: self.ns.clone(),
        }
    }
}

/// Query cache region contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryCacheMapping {
    /// Region containing cached query result keys.
    pub query_region: RegionMapping,
    /// Region containing Hibernate update timestamps.
    pub timestamps_region: RegionMapping,
}

impl QueryCacheMapping {
    /// Build a query-cache mapping that uses timestamp/bulk invalidation.
    pub fn new(
        query_region: RegionMapping,
        timestamps_region: RegionMapping,
    ) -> Result<Self, ClientProtocolError> {
        if query_region.kind != HibernateRegionKind::Query {
            return Err(ClientProtocolError::InvalidField("query_region_kind"));
        }
        if timestamps_region.kind != HibernateRegionKind::Timestamps {
            return Err(ClientProtocolError::InvalidField("timestamps_region_kind"));
        }
        Ok(Self {
            query_region,
            timestamps_region,
        })
    }

    /// Build the two W1 evictions needed after a bulk update.
    pub fn bulk_update_evictions(&self) -> [ClientRequest; 2] {
        [
            self.query_region.evict_region(),
            self.timestamps_region.evict_region(),
        ]
    }
}
