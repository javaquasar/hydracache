//! Evolution-safe helpers for the generated HC/2 contract.

use prost::Message;
use thiserror::Error;

/// Failure to preserve a received message's unknown protobuf fields.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContractRelayError {
    /// The bytes do not decode as the expected generated message.
    #[error("invalid HC/2 protobuf message: {0}")]
    Decode(String),
    /// Known fields changed, so replaying the original bytes would hide the
    /// mutation while a normal encode would silently discard unknown fields.
    #[error("known fields changed; unknown fields require an explicit discard decision")]
    UnknownFieldPreservationRequired,
}

/// A generated protobuf message paired with the exact bytes received.
///
/// `prost` deliberately does not retain unknown fields in its generated
/// structs. This wrapper makes transparent relay exact and makes mutation of a
/// future-version message fail loudly until the caller explicitly chooses to
/// discard fields it cannot understand.
#[derive(Debug, Clone)]
pub struct PreservedMessage<M> {
    value: M,
    original: Vec<u8>,
    known_at_decode: Vec<u8>,
}

impl<M> PreservedMessage<M>
where
    M: Message + Default,
{
    /// Decode a generated message while retaining its complete wire image.
    pub fn decode(bytes: &[u8]) -> Result<Self, ContractRelayError> {
        let value =
            M::decode(bytes).map_err(|error| ContractRelayError::Decode(error.to_string()))?;
        let known_at_decode = value.encode_to_vec();
        Ok(Self {
            value,
            original: bytes.to_vec(),
            known_at_decode,
        })
    }

    /// Read the generated, known-field view.
    pub fn value(&self) -> &M {
        &self.value
    }

    /// Mutate known fields. Encoding will then require an explicit decision
    /// about unknown data rather than silently dropping it.
    pub fn value_mut(&mut self) -> &mut M {
        &mut self.value
    }

    /// Return the exact original bytes when the known-field view is unchanged.
    pub fn encode_preserving_unknown_fields(&self) -> Result<Vec<u8>, ContractRelayError> {
        if self.value.encode_to_vec() == self.known_at_decode {
            Ok(self.original.clone())
        } else {
            Err(ContractRelayError::UnknownFieldPreservationRequired)
        }
    }

    /// Explicitly discard unknown fields and encode the current known view.
    ///
    /// This deliberately verbose method is the audit point for a lossy relay.
    pub fn discard_unknown_fields_and_encode(self) -> Vec<u8> {
        self.value.encode_to_vec()
    }
}
