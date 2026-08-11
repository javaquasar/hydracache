//! Preview transport-neutral Rust SDK for the generated HC/2 client plane.
//!
//! HC/1 remains the separate `hydracache-client` crate. This crate never
//! decodes, probes, or falls back to an HC/1 endpoint.

mod client;
mod grpc;
mod reconnect;
mod types;

/// Adapter boundary for independently gated HC/2 transports.
pub mod adapter;

/// Generated contract types for adapter authors. Native client operations do
/// not expose these types.
#[doc(hidden)]
pub mod wire {
    tonic::include_proto!("hydracache.client.v2alpha");
}

pub use client::{FencedSession, Hc2Client, Subscription};
pub use grpc::{GrpcMtlsAdapter, GrpcMtlsConfig};
pub use reconnect::{
    InvocationRetryPolicy, ReconnectEndpoint, ReconnectPolicy, RecoveringFencedSession,
    RecoveringHc2Client, RecoveringSubscription, RecoveryMetricsSnapshot,
};
pub use types::{
    BatchItemResult, BatchOperation, CacheEvent, CacheValue, Capability, ClientConfig, ClientError,
    ClientLimits, ClientMetricsSnapshot, ErrorCode, MutationResult, NodeEndpoint, RequestOptions,
    RetryAdvice, SubscriptionEvent, TopologySnapshot, TransportKind,
};

/// Oldest HC/2 generation accepted during the 0.68 preview compatibility window.
pub const HC2_MINIMUM_GENERATION: u32 = 5;

/// Current preferred HC/2 generation. It is intentionally disjoint from HC/1 versions.
pub const HC2_GENERATION: u32 = 6;

/// Return whether a generation belongs to the explicit preview compatibility window.
pub const fn is_supported_hc2_generation(generation: u32) -> bool {
    generation >= HC2_MINIMUM_GENERATION && generation <= HC2_GENERATION
}
