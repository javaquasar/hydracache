//! In-process adapter for the callable `/client/v1/data` Axum router.
//!
//! This target deliberately never binds a socket and never starts
//! `hydracache-server`. Every request is framed with the public client
//! protocol and dispatched through `Router::oneshot`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use hydracache_client_protocol::{
    BatchPutEntry, ClientFrame, ClientRequest, ClientRequestEnvelope, ClientResponse,
    ClientResponseEnvelope, ClientWireMessage, Namespace, StructuredKey, PROTOCOL_VERSION,
};
use hydracache_client_transport_axum::{
    AxumClientSurface, ClientSurfaceLimits, CLIENT_DATA_PATH, HYDRACACHE_CLIENT_ID_HEADER,
    HYDRACACHE_TENANT_HEADER,
};
use sha2::{Digest, Sha256};
use tokio::sync::{Barrier, RwLock};
use tower::ServiceExt;

use crate::target::{PreloadOutcome, Target, TargetError, TargetOutcome, TargetRequest};

const STATE_DIGEST_VERSION: &str = "hydracache-client-surface-state-v1";
const CLIENT_ID: &str = "loadgen-client";
const TENANT: &str = "loadgen-tenant";
const NAMESPACE: &str = "performance";

/// Deterministic YCSB-shaped mix supported by the client router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientSurfaceOperationMix {
    pub get_percent: u8,
    pub put_percent: u8,
    pub batch_get_percent: u8,
    pub batch_put_percent: u8,
}

impl ClientSurfaceOperationMix {
    pub const WORKLOAD_A: Self = Self {
        get_percent: 45,
        put_percent: 45,
        batch_get_percent: 5,
        batch_put_percent: 5,
    };

    pub const WORKLOAD_B: Self = Self {
        get_percent: 90,
        put_percent: 4,
        batch_get_percent: 5,
        batch_put_percent: 1,
    };

    pub const WORKLOAD_C: Self = Self {
        get_percent: 90,
        put_percent: 0,
        batch_get_percent: 10,
        batch_put_percent: 0,
    };

    pub const fn total_percent(self) -> u16 {
        self.get_percent as u16
            + self.put_percent as u16
            + self.batch_get_percent as u16
            + self.batch_put_percent as u16
    }

    pub fn operation_for(self, sequence: u64) -> ClientSurfaceOperation {
        let percentile = (sequence % 100) as u16;
        let get_end = self.get_percent as u16;
        let put_end = get_end + self.put_percent as u16;
        let batch_get_end = put_end + self.batch_get_percent as u16;
        if percentile < get_end {
            ClientSurfaceOperation::Get
        } else if percentile < put_end {
            ClientSurfaceOperation::Put
        } else if percentile < batch_get_end {
            ClientSurfaceOperation::BatchGet
        } else {
            ClientSurfaceOperation::BatchPut
        }
    }
}

/// Concrete supported client request shape selected for one logical operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientSurfaceOperation {
    Get,
    Put,
    BatchGet,
    BatchPut,
}

/// Configuration for the real in-process Axum client surface.
#[derive(Debug, Clone)]
pub struct ClientSurfaceTargetConfig {
    pub limits: ClientSurfaceLimits,
    pub preload_entries: u64,
    pub key_space: u64,
    pub payload_bytes: usize,
    pub batch_size: usize,
    pub operation_mix: ClientSurfaceOperationMix,
    pub key_schedule: Arc<Vec<u64>>,
    /// Development-loadgen defect seam; the product router remains unchanged.
    pub injected_dispatch_delay: Duration,
}

impl ClientSurfaceTargetConfig {
    pub fn validate(&self) -> Result<(), String> {
        self.limits.validate().map_err(|error| error.to_string())?;
        if self.key_space == 0 || self.payload_bytes == 0 || self.batch_size == 0 {
            return Err(
                "client-surface key space, payload, and batch size must be non-zero".into(),
            );
        }
        if self.preload_entries > self.key_space {
            return Err("client-surface preload entries cannot exceed key space".into());
        }
        if self.operation_mix.total_percent() != 100 {
            return Err(format!(
                "client-surface operation percentages must total 100, got {}",
                self.operation_mix.total_percent()
            ));
        }
        if self.key_schedule.is_empty() {
            return Err("client-surface key schedule must be non-empty".into());
        }
        if self.key_schedule.iter().any(|key| *key >= self.key_space) {
            return Err("client-surface key schedule contains an out-of-range key".into());
        }
        Ok(())
    }
}

/// Auditable counters exposed by the real surface state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientSurfaceTargetSnapshot {
    pub dispatch_attempts: u64,
    pub state_mutations: u64,
    pub rejected_oversized: u64,
    pub active_router_requests: u64,
    pub router_request_high_water: u64,
}

/// One complete in-process route result, including decoded response evidence.
#[derive(Debug)]
pub struct ClientSurfaceDispatch {
    pub status: StatusCode,
    pub outcome: TargetOutcome,
    pub encoded_request_bytes: usize,
    pub response: Option<ClientResponseEnvelope>,
}

struct ActiveRouterRequest<'a> {
    active: &'a AtomicU64,
}

impl Drop for ActiveRouterRequest<'_> {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Real `AxumClientSurface` target. Reset replaces the complete router state.
#[derive(Debug)]
pub struct ClientSurfaceTarget {
    config: ClientSurfaceTargetConfig,
    surface: RwLock<AxumClientSurface>,
    dispatch_rendezvous: RwLock<Option<Arc<Barrier>>>,
    active_router_requests: AtomicU64,
    router_request_high_water: AtomicU64,
}

impl ClientSurfaceTarget {
    pub fn new(config: ClientSurfaceTargetConfig) -> Result<Self, TargetError> {
        config.validate().map_err(TargetError::Reset)?;
        let surface = AxumClientSurface::new(config.limits)
            .map_err(|error| TargetError::Reset(error.to_string()))?;
        Ok(Self {
            config,
            surface: RwLock::new(surface),
            dispatch_rendezvous: RwLock::new(None),
            active_router_requests: AtomicU64::new(0),
            router_request_high_water: AtomicU64::new(0),
        })
    }

    pub fn config(&self) -> &ClientSurfaceTargetConfig {
        &self.config
    }

    pub async fn snapshot(&self) -> ClientSurfaceTargetSnapshot {
        let surface = self.surface.read().await;
        let state = surface.state();
        ClientSurfaceTargetSnapshot {
            dispatch_attempts: state.dispatch_attempts(),
            state_mutations: state.state_mutations(),
            rejected_oversized: state.rejected_oversized(),
            active_router_requests: self.active_router_requests.load(Ordering::SeqCst),
            router_request_high_water: self.router_request_high_water.load(Ordering::SeqCst),
        }
    }

    /// Configure a loadgen-only rendezvous at the actual router request
    /// boundary and reset the observed request-lifetime high-water mark.
    pub async fn configure_dispatch_rendezvous(
        &self,
        parties: Option<usize>,
    ) -> Result<(), TargetError> {
        if self.active_router_requests.load(Ordering::SeqCst) != 0 {
            return Err(TargetError::Measurement(
                "cannot reconfigure dispatch rendezvous with active router requests".into(),
            ));
        }
        let rendezvous = match parties {
            Some(0) => {
                return Err(TargetError::Measurement(
                    "dispatch rendezvous requires at least one party".into(),
                ));
            }
            Some(parties) => Some(Arc::new(Barrier::new(parties))),
            None => None,
        };
        *self.dispatch_rendezvous.write().await = rendezvous;
        self.router_request_high_water.store(0, Ordering::SeqCst);
        Ok(())
    }

    pub async fn execute_operation(
        &self,
        operation: ClientSurfaceOperation,
        sequence: u64,
    ) -> TargetOutcome {
        match self.dispatch_operation(operation, sequence).await {
            Ok(dispatch) => dispatch.outcome,
            Err(_) => TargetOutcome::Error,
        }
    }

    pub async fn dispatch_operation(
        &self,
        operation: ClientSurfaceOperation,
        sequence: u64,
    ) -> Result<ClientSurfaceDispatch, TargetError> {
        let key = self.scheduled_key(sequence);
        let request = match operation {
            ClientSurfaceOperation::Get => ClientRequest::Get {
                ns: namespace()?,
                key: structured_key(key)?,
            },
            ClientSurfaceOperation::Put => ClientRequest::Put {
                ns: namespace()?,
                key: structured_key(key)?,
                value: payload(sequence, self.config.payload_bytes),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
            ClientSurfaceOperation::BatchGet => ClientRequest::BatchGet {
                ns: namespace()?,
                keys: self.batch_keys(key)?,
            },
            ClientSurfaceOperation::BatchPut => ClientRequest::BatchPut {
                ns: namespace()?,
                entries: self
                    .batch_keys(key)?
                    .into_iter()
                    .enumerate()
                    .map(|(index, key)| BatchPutEntry {
                        key,
                        value: payload(
                            sequence.saturating_add(index as u64),
                            self.config.payload_bytes,
                        ),
                    })
                    .collect(),
            },
        };
        self.dispatch_envelope(ClientRequestEnvelope::new(
            format!("w2-{sequence}"),
            request,
        ))
        .await
    }

    /// Send a value of an exact size through the real framed route.
    pub async fn dispatch_payload_put(
        &self,
        payload_bytes: usize,
        sequence: u64,
    ) -> Result<ClientSurfaceDispatch, TargetError> {
        self.dispatch_envelope(ClientRequestEnvelope::new(
            format!("w2-payload-{payload_bytes}-{sequence}"),
            ClientRequest::Put {
                ns: namespace()?,
                key: structured_key(sequence % self.config.key_space)?,
                value: payload(sequence, payload_bytes),
                ttl_ms: None,
                dimensions: Vec::new(),
            },
        ))
        .await
    }

    /// Encode and dispatch one public envelope through `Router::oneshot`.
    pub async fn dispatch_envelope(
        &self,
        envelope: ClientRequestEnvelope,
    ) -> Result<ClientSurfaceDispatch, TargetError> {
        let frame = ClientFrame::from_message_with_version(
            PROTOCOL_VERSION,
            &ClientWireMessage::Request(envelope),
        )
        .map_err(|error| TargetError::Measurement(error.to_string()))?
        .encode()
        .map_err(|error| TargetError::Measurement(error.to_string()))?;
        self.dispatch_encoded(frame.to_vec()).await
    }

    /// Dispatch already encoded frame bytes. This is used to price codec and
    /// admission paths separately while preserving the same real router edge.
    pub async fn dispatch_encoded(
        &self,
        encoded: Vec<u8>,
    ) -> Result<ClientSurfaceDispatch, TargetError> {
        if !self.config.injected_dispatch_delay.is_zero() {
            tokio::time::sleep(self.config.injected_dispatch_delay).await;
        }
        let encoded_request_bytes = encoded.len();
        let router = {
            let surface = self.surface.read().await;
            surface.routes().clone()
        };
        let request = Request::builder()
            .method("POST")
            .uri(CLIENT_DATA_PATH)
            .header(HYDRACACHE_CLIENT_ID_HEADER, CLIENT_ID)
            .header(HYDRACACHE_TENANT_HEADER, TENANT)
            .body(Body::from(encoded))
            .map_err(|error| TargetError::Measurement(error.to_string()))?;
        let rendezvous = self.dispatch_rendezvous.read().await.clone();
        let active = self
            .active_router_requests
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        self.router_request_high_water
            .fetch_max(active, Ordering::SeqCst);
        let active_request = ActiveRouterRequest {
            active: &self.active_router_requests,
        };
        if let Some(rendezvous) = rendezvous {
            rendezvous.wait().await;
        }
        let response = router
            .oneshot(request)
            .await
            .map_err(|error| TargetError::Measurement(error.to_string()))?;
        drop(active_request);
        let status = response.status();
        if status == StatusCode::PAYLOAD_TOO_LARGE {
            return Ok(ClientSurfaceDispatch {
                status,
                outcome: TargetOutcome::Rejected,
                encoded_request_bytes,
                response: None,
            });
        }
        if status != StatusCode::OK {
            return Ok(ClientSurfaceDispatch {
                status,
                outcome: TargetOutcome::Error,
                encoded_request_bytes,
                response: None,
            });
        }
        let body = to_bytes(response.into_body(), self.config.limits.max_frame_bytes)
            .await
            .map_err(|error| TargetError::Measurement(error.to_string()))?;
        let frame = ClientFrame::decode(&body, self.config.limits.max_frame_bytes)
            .map_err(|error| TargetError::Measurement(error.to_string()))?;
        let message = frame
            .decode_message()
            .map_err(|error| TargetError::Measurement(error.to_string()))?;
        let ClientWireMessage::Response(response) = message else {
            return Err(TargetError::Measurement(
                "client surface returned a non-response wire message".into(),
            ));
        };
        let outcome = if response.result.is_ok() {
            TargetOutcome::Success
        } else {
            TargetOutcome::Rejected
        };
        Ok(ClientSurfaceDispatch {
            status,
            outcome,
            encoded_request_bytes,
            response: Some(response),
        })
    }

    fn scheduled_key(&self, sequence: u64) -> u64 {
        self.config.key_schedule[sequence as usize % self.config.key_schedule.len()]
    }

    fn batch_keys(&self, first: u64) -> Result<Vec<StructuredKey>, TargetError> {
        (0..self.config.batch_size)
            .map(|offset| {
                structured_key(first.saturating_add(offset as u64) % self.config.key_space)
            })
            .collect()
    }

    async fn observed_value(&self, logical_key: u64) -> Result<Option<Vec<u8>>, TargetError> {
        let dispatch = self
            .dispatch_envelope(ClientRequestEnvelope::new(
                format!("w2-observe-{logical_key}"),
                ClientRequest::Get {
                    ns: namespace()?,
                    key: structured_key(logical_key)?,
                },
            ))
            .await?;
        let Some(response) = dispatch.response else {
            return Err(TargetError::Measurement(format!(
                "state observation returned HTTP {}",
                dispatch.status
            )));
        };
        match response.result {
            Ok(ClientResponse::Value { value }) => Ok(value),
            other => Err(TargetError::Measurement(format!(
                "state observation returned unexpected response {other:?}"
            ))),
        }
    }

    async fn observed_state_digest(&self) -> Result<String, TargetError> {
        let mut hasher = Sha256::new();
        hasher.update(STATE_DIGEST_VERSION.as_bytes());
        hasher.update(self.config.key_space.to_le_bytes());
        for logical_key in 0..self.config.key_space {
            hasher.update(logical_key.to_le_bytes());
            match self.observed_value(logical_key).await? {
                Some(value) => {
                    hasher.update([1]);
                    hasher.update((value.len() as u64).to_le_bytes());
                    hasher.update(value);
                }
                None => hasher.update([0]),
            }
        }
        Ok(hex_digest(hasher.finalize().as_ref()))
    }
}

#[async_trait]
impl Target for ClientSurfaceTarget {
    async fn reset(&self) -> Result<String, TargetError> {
        if self.active_router_requests.load(Ordering::SeqCst) != 0 {
            return Err(TargetError::Reset(
                "cannot reset with active router requests".into(),
            ));
        }
        *self.dispatch_rendezvous.write().await = None;
        self.router_request_high_water.store(0, Ordering::SeqCst);
        let replacement = AxumClientSurface::new(self.config.limits)
            .map_err(|error| TargetError::Reset(error.to_string()))?;
        *self.surface.write().await = replacement;
        self.observed_state_digest()
            .await
            .map_err(|error| TargetError::Reset(error.to_string()))
    }

    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        for logical_key in 0..self.config.preload_entries {
            let expected = payload(logical_key, self.config.payload_bytes);
            let dispatch = self
                .dispatch_envelope(ClientRequestEnvelope::new(
                    format!("w2-preload-put-{logical_key}"),
                    ClientRequest::Put {
                        ns: namespace()?,
                        key: structured_key(logical_key)?,
                        value: expected.clone(),
                        ttl_ms: None,
                        dimensions: Vec::new(),
                    },
                ))
                .await?;
            if dispatch.outcome != TargetOutcome::Success
                || !matches!(
                    dispatch.response.as_ref().map(|value| &value.result),
                    Some(Ok(ClientResponse::Stored))
                )
            {
                return Err(TargetError::Preload(format!(
                    "preload put {logical_key} failed: {dispatch:?}"
                )));
            }
            if self.observed_value(logical_key).await? != Some(expected) {
                return Err(TargetError::Preload(format!(
                    "preload get verification failed for key {logical_key}"
                )));
            }
        }
        Ok(PreloadOutcome {
            operations: self.config.preload_entries,
            state_digest: self.observed_state_digest().await?,
        })
    }

    async fn state_digest(&self) -> Result<String, TargetError> {
        self.observed_state_digest().await
    }

    async fn execute(&self, request: TargetRequest) -> TargetOutcome {
        let operation = self.config.operation_mix.operation_for(request.sequence);
        self.execute_operation(operation, request.sequence).await
    }
}

fn namespace() -> Result<Namespace, TargetError> {
    Namespace::new(NAMESPACE).map_err(|error| TargetError::Measurement(error.to_string()))
}

fn structured_key(logical_key: u64) -> Result<StructuredKey, TargetError> {
    StructuredKey::new(vec!["w2".to_owned(), logical_key.to_string()])
        .map_err(|error| TargetError::Measurement(error.to_string()))
}

fn payload(sequence: u64, bytes: usize) -> Vec<u8> {
    let byte = (sequence % 251) as u8;
    vec![byte; bytes]
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
