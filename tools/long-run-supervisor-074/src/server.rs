use crate::config::ServerConfig;
use crate::protocol::{
    parse_wire_request, sign_response, Operation, Response, ResponseBody, WireRequest,
};
use crate::service::{authorize_wire, ServiceError, ServicePolicy};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::unix_transport::{SeqpacketListener, TransportError};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("supervisor transport failed: {0}")]
    Transport(#[from] TransportError),
    #[error("supervisor response serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct SupervisorServer {
    listener: SeqpacketListener,
    policy: ServicePolicy,
    config: ServerConfig,
}

impl SupervisorServer {
    pub fn bind(config: ServerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let policy = config.policy()?;
        let listener = SeqpacketListener::bind(&config.socket_path, config.socket_mode)?;
        Ok(Self {
            listener,
            policy,
            config,
        })
    }

    pub fn serve(self) -> Result<(), ServerError> {
        loop {
            self.serve_one()?;
        }
    }

    pub fn serve_one(&self) -> Result<(), ServerError> {
        let connection = self.listener.accept()?;
        let packet = connection.receive_packet()?;
        let wire = match parse_wire_request(&packet) {
            Ok(wire) => wire,
            Err(_) => return Ok(()),
        };
        let now = unix_seconds();
        let response = match connection.peer_credentials() {
            Ok(peer) => match authorize_wire(wire.clone(), &peer, now, &self.policy) {
                Ok(authorized) => self.dispatch(&authorized.request, now),
                Err(error) => error_response(&wire, now, service_error_code(&error)),
            },
            Err(_) => error_response(&wire, now, 3),
        }?;
        connection.send_packet(&serde_json::to_vec(&response)?)?;
        Ok(())
    }

    fn dispatch(
        &self,
        request: &crate::protocol::Request,
        now: u64,
    ) -> Result<Response, ServerError> {
        if request.operation != Operation::Status {
            return Ok(error_response_from_request(request, now, 11)?);
        }
        let result = match CampaignLock::acquire(&self.config.campaign_root, &request.campaign_id) {
            Ok(lock) => match lock.read() {
                Ok(state)
                    if state.identity.manifest_sha256 == request.manifest_sha256
                        && state.revision == request.expected_state_revision =>
                {
                    success_response(request, now, state.revision, serde_json::to_value(state)?)
                }
                Ok(state) => error_response_with_revision(request, now, state.revision, 5),
                Err(error) => error_response_from_request(request, now, state_error_code(&error)),
            },
            Err(error) => error_response_from_request(request, now, state_error_code(&error)),
        }?;
        Ok(result)
    }
}

fn success_response(
    request: &crate::protocol::Request,
    now: u64,
    revision: u64,
    result: Value,
) -> Result<Response, ServerError> {
    Ok(sign_response(ResponseBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        campaign_id: request.campaign_id.clone(),
        ok: true,
        state_revision: revision,
        server_time_unix_seconds: now,
        result: Some(result),
        error_code: None,
    })?)
}

fn error_response(wire: &WireRequest, now: u64, code: u32) -> Result<Response, ServerError> {
    error_response_from_request(&wire.request, now, code)
}

fn error_response_from_request(
    request: &crate::protocol::Request,
    now: u64,
    code: u32,
) -> Result<Response, ServerError> {
    error_response_with_revision(request, now, request.expected_state_revision, code)
}

fn error_response_with_revision(
    request: &crate::protocol::Request,
    now: u64,
    revision: u64,
    code: u32,
) -> Result<Response, ServerError> {
    Ok(sign_response(ResponseBody {
        schema_version: 1,
        request_id: request.request_id.clone(),
        campaign_id: request.campaign_id.clone(),
        ok: false,
        state_revision: revision,
        server_time_unix_seconds: now,
        result: None,
        error_code: Some(code),
    })?)
}

fn service_error_code(error: &ServiceError) -> u32 {
    match error {
        ServiceError::Protocol(_) => 2,
        ServiceError::Peer | ServiceError::Principal | ServiceError::Authorization(_) => 3,
    }
}

fn state_error_code(error: &StateStoreError) -> u32 {
    match error {
        StateStoreError::Busy => 10,
        StateStoreError::Path => 4,
        StateStoreError::Document => 9,
        StateStoreError::Revision { .. } => 5,
        StateStoreError::Io(error) if error.kind() == std::io::ErrorKind::NotFound => 4,
        StateStoreError::Io(_) | StateStoreError::Json(_) => 11,
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
