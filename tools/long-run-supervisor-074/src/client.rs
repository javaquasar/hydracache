use crate::protocol::{parse_wire_request, verify_response, ProtocolError, Response};
use crate::unix_transport::{SeqpacketConnection, TransportError};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("client request or response protocol failed: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("client transport failed: {0}")]
    Transport(#[from] TransportError),
    #[error("client response JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("response identity does not match the request")]
    Identity,
}

pub fn exchange(socket: &Path, packet: &[u8]) -> Result<Response, ClientError> {
    let wire = parse_wire_request(packet)?;
    let connection = SeqpacketConnection::connect(socket)?;
    connection.send_packet(packet)?;
    let response_packet = connection.receive_packet()?;
    let mut deserializer = serde_json::Deserializer::from_slice(&response_packet);
    let response = Response::deserialize(&mut deserializer)?;
    deserializer.end()?;
    verify_response(&response)?;
    if response.body.request_id != wire.request.request_id
        || response.body.campaign_id != wire.request.campaign_id
    {
        return Err(ClientError::Identity);
    }
    Ok(response)
}

use serde::Deserialize;
