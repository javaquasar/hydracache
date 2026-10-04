use crate::protocol::{parse_wire_request, verify_response, Operation, ProtocolError, Response};
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
    #[error("start evidence can accompany only a revision-zero start request")]
    StartEvidence,
}

pub fn exchange(socket: &Path, packet: &[u8]) -> Result<Response, ClientError> {
    let wire = parse_wire_request(packet)?;
    let connection = SeqpacketConnection::connect(socket)?;
    connection.send_packet(packet)?;
    receive_response(&connection, &wire)
}

pub fn exchange_start_with_evidence(
    socket: &Path,
    packet: &[u8],
    manifest: &[u8],
    host_receipt: &[u8],
) -> Result<Response, ClientError> {
    let wire = parse_wire_request(packet)?;
    if wire.request.operation != Operation::Start || wire.request.expected_state_revision != 0 {
        return Err(ClientError::StartEvidence);
    }
    let connection = SeqpacketConnection::connect(socket)?;
    connection.send_packet(packet)?;
    connection.send_packet(manifest)?;
    connection.send_packet(host_receipt)?;
    receive_response(&connection, &wire)
}

fn receive_response(
    connection: &SeqpacketConnection,
    wire: &crate::protocol::WireRequest,
) -> Result<Response, ClientError> {
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
