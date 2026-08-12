//! Generated-envelope transport boundary.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::wire::{ClientEnvelope, ServerEnvelope};
use crate::{ClientConfig, ClientError, TransportKind};

/// Idempotent transport shutdown control.
pub trait ConnectionControl: Send + Sync + 'static {
    /// Stop transport I/O and close the authenticated stream.
    fn close(&self);
}

/// One authenticated adapter stream. Adapters translate transport lifecycle
/// only; the shared SDK runtime owns HC/2 semantics.
pub struct AdapterConnection {
    pub(crate) outbound: mpsc::Sender<ClientEnvelope>,
    pub(crate) inbound: mpsc::Receiver<Result<ServerEnvelope, ClientError>>,
    pub(crate) control: Arc<dyn ConnectionControl>,
}

impl AdapterConnection {
    /// Construct adapter parts from generated envelopes.
    #[doc(hidden)]
    pub fn new(
        outbound: mpsc::Sender<ClientEnvelope>,
        inbound: mpsc::Receiver<Result<ServerEnvelope, ClientError>>,
        control: Arc<dyn ConnectionControl>,
    ) -> Self {
        Self {
            outbound,
            inbound,
            control,
        }
    }
}

/// HC/2 transport adapter. Security/protocol failures must be returned as
/// terminal errors and must never trigger fallback inside an adapter.
#[async_trait]
pub trait TransportAdapter: Send + Sync {
    /// Stable adapter identity.
    fn kind(&self) -> TransportKind;

    /// Establish one authenticated generated-envelope stream.
    async fn connect(&self, config: &ClientConfig) -> Result<AdapterConnection, ClientError>;
}
