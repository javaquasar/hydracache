use crate::config::ServerConfig;
use crate::host_receipt::verify_host_receipt_evidence;
use crate::manifest_evidence::verify_manifest_evidence;
use crate::mutation::{begin_attach, reconcile_campaign, BeginAttach, MutationError};
use crate::process_identity::{verify_process_cpuset, verify_process_identity};
use crate::protocol::{
    parse_wire_request, sign_response, Operation, Response, ResponseBody, WireRequest,
};
use crate::service::{authorize_wire, AuthorizedRequest, ServiceError, ServicePolicy};
use crate::state::{apply_attach, AttachRequest};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::systemd_unit::{inspect_unit, verify_unit_identity};
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
    #[error("systemd readiness notification failed: {0}")]
    Notify(#[from] std::io::Error),
    #[error("durable supervisor mutation failed: {0}")]
    Mutation(#[from] MutationError),
}

pub struct SupervisorServer {
    listener: SeqpacketListener,
    policy: ServicePolicy,
    config: ServerConfig,
}

impl SupervisorServer {
    pub fn bind(config: ServerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let policy = config.policy()?;
        let listener = SeqpacketListener::bind(
            &config.socket_path,
            config.socket_mode,
            config.required_client_gid,
        )?;
        Ok(Self {
            listener,
            policy,
            config,
        })
    }

    pub fn serve(self) -> Result<(), ServerError> {
        crate::systemd_notify::ready()?;
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
                Ok(authorized) => self.dispatch(&authorized, now),
                Err(error) => error_response(&wire, now, service_error_code(&error)),
            },
            Err(_) => error_response(&wire, now, 3),
        }?;
        connection.send_packet(&serde_json::to_vec(&response)?)?;
        Ok(())
    }

    fn dispatch(&self, authorized: &AuthorizedRequest, now: u64) -> Result<Response, ServerError> {
        let request = &authorized.request;
        if request.operation == Operation::Attach {
            return self.dispatch_attach(authorized, now);
        }
        if request.operation != Operation::Status {
            return Ok(error_response_from_request(request, now, 11)?);
        }
        let result = match CampaignLock::acquire(&self.config.campaign_root, &request.campaign_id) {
            Ok(lock) => match reconcile_campaign(&lock) {
                Ok(state)
                    if state.identity.manifest_sha256 == request.manifest_sha256
                        && state.revision == request.expected_state_revision =>
                {
                    success_response(request, now, state.revision, serde_json::to_value(state)?)
                }
                Ok(state) => error_response_with_revision(request, now, state.revision, 5),
                Err(error) => {
                    error_response_from_request(request, now, mutation_error_code(&error))
                }
            },
            Err(error) => error_response_from_request(request, now, state_error_code(&error)),
        }?;
        Ok(result)
    }

    fn dispatch_attach(
        &self,
        authorized: &AuthorizedRequest,
        now: u64,
    ) -> Result<Response, ServerError> {
        let request = &authorized.request;
        let lock = match CampaignLock::acquire(&self.config.campaign_root, &request.campaign_id) {
            Ok(lock) => lock,
            Err(error) => {
                return Ok(error_response_from_request(
                    request,
                    now,
                    state_error_code(&error),
                )?)
            }
        };
        let transaction = match begin_attach(&lock, request, now) {
            Ok(BeginAttach::Replayed(response)) => return Ok(response),
            Ok(BeginAttach::New(transaction)) => transaction,
            Err(error) => {
                return Ok(error_response_from_request(
                    request,
                    now,
                    mutation_error_code(&error),
                )?)
            }
        };
        let state = transaction.state().clone();
        let mut failures = Vec::new();
        let manifest =
            match verify_manifest_evidence(lock.campaign_directory(), request, &state, now) {
                Ok(manifest) => Some(manifest),
                Err(error) => {
                    failures.push(format!("manifest:{error}"));
                    None
                }
            };
        match inspect_unit(&state.harness.unit_name)
            .and_then(|snapshot| verify_unit_identity(&state.harness, &state.daemon, &snapshot))
        {
            Ok(()) => {}
            Err(error) => failures.push(format!("systemd-unit:{error}")),
        }
        if let Err(error) = verify_process_identity(&state.harness) {
            failures.push(format!("harness-process:{error}"));
        }
        if let Err(error) = verify_process_identity(&state.daemon) {
            failures.push(format!("daemon-process:{error}"));
        }
        if let Err(error) = verify_process_cpuset(&state.harness, &state.identity.isolated_cpuset) {
            failures.push(format!("harness-cpuset:{error}"));
        }
        if let Err(error) = verify_process_cpuset(&state.daemon, &state.identity.isolated_cpuset) {
            failures.push(format!("daemon-cpuset:{error}"));
        }
        if let Err(error) = crate::checkpoint_evidence::verify_checkpoint_evidence(
            lock.campaign_directory(),
            &state,
        ) {
            failures.push(format!("checkpoint:{error}"));
        }
        match manifest.as_ref() {
            Some(manifest) => {
                if let Err(error) =
                    verify_host_receipt_evidence(lock.campaign_directory(), manifest, &state)
                {
                    failures.push(format!("host:{error}"));
                }
            }
            None => failures.push("host:manifest-unavailable".to_owned()),
        }
        let authorization = authorized
            .authorization
            .as_ref()
            .ok_or(MutationError::Operation)?;
        let attach = AttachRequest {
            request_id: request.request_id.clone(),
            request_sha256: crate::event::request_sha256(request).map_err(MutationError::from)?,
            expected_revision: request.expected_state_revision,
            authorization_sha256: request.controller.authorization_sha256.clone(),
            identity: state.identity.clone(),
            harness: state.harness.clone(),
            daemon: state.daemon.clone(),
            checkpoint: state.checkpoint.clone(),
            now_unix_seconds: now,
            requested_controller_lease_seconds: authorization
                .expires_at_unix_seconds
                .saturating_sub(now),
        };
        let progress_rejection_gap_seconds = manifest
            .as_ref()
            .map_or(180, |manifest| manifest.progress_rejection_gap_seconds);
        let next = match apply_attach(&state, &attach, progress_rejection_gap_seconds) {
            Ok(next) => Some(next),
            Err(decision) => {
                failures.extend(
                    decision
                        .failures
                        .into_iter()
                        .map(|failure| format!("state:{failure:?}")),
                );
                None
            }
        };
        if !failures.is_empty() {
            return Ok(transaction
                .reject_with_result(6, Some(serde_json::json!({"failures": failures})))?);
        }
        Ok(transaction.accept(next.ok_or(MutationError::Diverged)?)?)
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

fn mutation_error_code(error: &MutationError) -> u32 {
    match error {
        MutationError::Event(crate::event::EventError::ReplayConflict { .. }) => 3,
        MutationError::Diverged
        | MutationError::Event(_)
        | MutationError::State(StateStoreError::Document) => 9,
        MutationError::State(error) => state_error_code(error),
        MutationError::Operation | MutationError::Json(_) | MutationError::Io(_) => 11,
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
