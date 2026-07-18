use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// One scheduled operation passed to a target adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetRequest {
    /// Stable sequence number within the phase.
    pub sequence: u64,
}

/// Auditable result of establishing a target's declared preloaded state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreloadOutcome {
    pub operations: u64,
    pub state_digest: String,
}

/// Normalized target outcome used by every surface adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetOutcome {
    /// The operation completed successfully.
    Success,
    /// Admission or an explicit capacity boundary rejected the operation.
    Rejected,
    /// The target returned a protocol or application error.
    Error,
    /// The operation exceeded its target-level timeout.
    Timeout,
}

/// Failure in target lifecycle setup rather than an operation outcome.
#[derive(Debug, thiserror::Error)]
pub enum TargetError {
    /// Reset could not establish the requested initial state.
    #[error("target reset failed: {0}")]
    Reset(String),
    /// Preload could not establish the declared data set.
    #[error("target preload failed: {0}")]
    Preload(String),
    /// The common measurement runner rejected an invalid execution contract.
    #[error("target measurement failed: {0}")]
    Measurement(String),
}

/// Pluggable callable boundary measured by the common open-loop runner.
#[async_trait]
pub trait Target: Send + Sync + 'static {
    /// Reset the target and return a digest of the resulting initial state.
    async fn reset(&self) -> Result<String, TargetError>;

    /// Deterministically preload the target for the scenario.
    async fn preload(&self) -> Result<PreloadOutcome, TargetError> {
        Ok(PreloadOutcome {
            operations: 0,
            state_digest: "state:preload-none:v1".to_owned(),
        })
    }

    /// Execute one scheduled request.
    async fn execute(&self, request: TargetRequest) -> TargetOutcome;
}
