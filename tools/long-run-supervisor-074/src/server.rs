use crate::abort_backend::SystemdAbortBackend;
use crate::abort_lifecycle::{drive_abort_request, AbortBackend, AbortLifecycleError};
use crate::archive::ArchiveLimits;
use crate::artifact::PacketLimits;
use crate::checkpoint_evidence::{observe_live_checkpoint_evidence, CheckpointEvidenceError};
use crate::config::ServerConfig;
use crate::event::{verify_event_journal, LifecycleEvent, EVENT_HEAD_NAME, EVENT_JOURNAL_NAME};
use crate::host_execution::{HostExecutionClaim, HostExecutionError};
use crate::host_receipt::{
    collect_host_observation, verify_host_receipt_evidence, verify_live_observation,
    verify_receipt_manifest_binding, HostObservationReceipt,
};
use crate::lease_expiry::{drive_lease_expiry, LeaseExpiryBackend, LeaseExpiryOutcome};
use crate::manifest::{frozen_identity_from_manifest, CampaignManifest};
use crate::manifest_evidence::{verify_manifest_evidence, verify_stored_manifest_evidence};
use crate::measurement_loss::{
    drive_measurement_loss, MeasurementLossBackend, MeasurementLossOutcome, MeasurementObservation,
};
use crate::mutation::{begin_attach, reconcile_campaign, BeginAttach, MutationError};
use crate::process_identity::{verify_process_cpuset, verify_process_identity};
use crate::progress_loss::{drive_progress_loss, ProgressLossBackend, ProgressLossOutcome};
use crate::protocol::{
    parse_wire_request, sign_response, HostObservationResult, Operation, Response, ResponseBody,
    WireRequest,
};
use crate::seal_input::resolve_packet_plan;
use crate::seal_lifecycle::{drive_seal_request, SealLifecycleError};
use crate::service::{authorize_wire, AuthorizedRequest, ServiceError, ServicePolicy};
use crate::spawn::SpawnBackend;
use crate::start_evidence::{
    load_campaign_evidence, prepare_campaign_evidence, publish_received_start_evidence,
    PreparedCampaignEvidence, StartEvidenceError,
};
use crate::start_lifecycle::{
    drive_c74_start_request, drive_i74_start_request, replay_start_response, StartLifecycleError,
};
use crate::state::{
    apply_attach, AttachRequest, CampaignState, DurableCampaignState, FrozenIdentity,
};
use crate::state_store::{CampaignLock, StateStoreError};
use crate::systemd_spawn::SystemdSpawnBackend;
use crate::systemd_unit::{inspect_unit, verify_unit_identity, UnitSnapshot};
use crate::unix_transport::{SeqpacketConnection, SeqpacketListener, TransportError};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(1);
const START_EVIDENCE_PACKET_TIMEOUT: Duration = Duration::from_secs(5);

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
    #[error("supervisor lease-expiry maintenance failed: {0}")]
    LeaseExpiry(String),
    #[error("supervisor progress-loss maintenance failed: {0}")]
    ProgressLoss(String),
    #[error("supervisor measurement-loss maintenance failed: {0}")]
    MeasurementLoss(String),
}

pub struct SupervisorServer {
    listener: SeqpacketListener,
    policy: ServicePolicy,
    config: ServerConfig,
}

pub trait SealObservationBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String>;

    fn inspect_terminal(
        &mut self,
        harness: &crate::ProcessIdentity,
    ) -> Result<UnitSnapshot, String>;
}

pub trait AbortObservationBackend: AbortBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String>;
}

pub trait LeaseExpiryObservationBackend: LeaseExpiryBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String>;
}

pub trait ProgressLossObservationBackend: ProgressLossBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String>;
}

pub trait MeasurementLossObservationBackend: MeasurementLossBackend {
    fn prepare_measurement(
        &mut self,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String>;

    fn observe_measurement(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<MeasurementObservation, String>;
}

pub trait StartObservationBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        admitted: &HostObservationReceipt,
    ) -> Result<(), String>;
}

pub trait HostObservationBackend {
    fn collect(&mut self, campaign_root: &Path) -> Result<HostObservationReceipt, String>;
}

struct LiveHostObservationBackend;

impl HostObservationBackend for LiveHostObservationBackend {
    fn collect(&mut self, campaign_root: &Path) -> Result<HostObservationReceipt, String> {
        collect_host_observation(campaign_root).map_err(|error| error.to_string())
    }
}

struct LiveStartObservationBackend;

impl StartObservationBackend for LiveStartObservationBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        admitted: &HostObservationReceipt,
    ) -> Result<(), String> {
        verify_receipt_manifest_binding(admitted, manifest).map_err(|error| error.to_string())?;
        let observed =
            collect_host_observation(campaign_directory).map_err(|error| error.to_string())?;
        verify_live_observation(admitted, &observed).map_err(|error| error.to_string())
    }
}

impl StartObservationBackend for SystemdSpawnBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        admitted: &HostObservationReceipt,
    ) -> Result<(), String> {
        let mut observer = LiveStartObservationBackend;
        observer.verify_host(campaign_directory, manifest, admitted)
    }
}

impl AbortObservationBackend for SystemdAbortBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        verify_host_receipt_evidence(campaign_directory, manifest, state)
            .map_err(|error| error.to_string())?;
        self.bind_manifest(manifest.clone());
        Ok(())
    }
}

impl LeaseExpiryObservationBackend for SystemdAbortBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        verify_host_receipt_evidence(campaign_directory, manifest, state)
            .map_err(|error| error.to_string())?;
        self.bind_manifest(manifest.clone());
        Ok(())
    }
}

impl ProgressLossObservationBackend for SystemdAbortBackend {
    fn verify_host(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        verify_host_receipt_evidence(campaign_directory, manifest, state)
            .map_err(|error| error.to_string())?;
        self.bind_manifest(manifest.clone());
        Ok(())
    }
}

impl MeasurementLossObservationBackend for SystemdAbortBackend {
    fn prepare_measurement(
        &mut self,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<(), String> {
        if manifest.campaign_id != state.identity.campaign_id {
            return Err("measurement manifest identity differs".to_owned());
        }
        self.bind_manifest(manifest.clone());
        Ok(())
    }

    fn observe_measurement(
        &mut self,
        campaign_directory: &Path,
        manifest: &CampaignManifest,
        state: &DurableCampaignState,
    ) -> Result<MeasurementObservation, String> {
        if verify_host_receipt_evidence(campaign_directory, manifest, state).is_err() {
            return Ok(MeasurementObservation::Lost(
                crate::measurement_loss::MeasurementLossReason::HostIdentityDrift,
            ));
        }
        let harness = state
            .harness
            .as_ref()
            .ok_or_else(|| "measurement harness identity is missing".to_owned())?;
        let daemon = state
            .daemon
            .as_ref()
            .ok_or_else(|| "measurement daemon identity is missing".to_owned())?;
        let Some(snapshot) = crate::systemd_unit::inspect_unit_optional(&harness.unit_name)
            .map_err(|error| error.to_string())?
        else {
            return Ok(MeasurementObservation::Lost(
                crate::measurement_loss::MeasurementLossReason::UnitAbsent,
            ));
        };
        if crate::systemd_unit::verify_unit_terminal(harness, daemon, &snapshot).is_ok() {
            return Ok(MeasurementObservation::Terminal);
        }
        if verify_unit_identity(harness, daemon, &snapshot).is_err() {
            return Ok(MeasurementObservation::Lost(
                crate::measurement_loss::MeasurementLossReason::UnitIdentityDrift,
            ));
        }
        if verify_process_identity(harness).is_err()
            || verify_process_identity(daemon).is_err()
            || verify_process_cpuset(harness, &state.identity.isolated_cpuset).is_err()
            || verify_process_cpuset(daemon, &state.identity.isolated_cpuset).is_err()
        {
            return Ok(MeasurementObservation::Lost(
                crate::measurement_loss::MeasurementLossReason::ProcessIdentityDrift,
            ));
        }
        Ok(MeasurementObservation::Healthy)
    }
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
        let mut lease_backend = SystemdAbortBackend::new();
        loop {
            self.maintain_lease_expiry_with_backend(unix_seconds(), &mut lease_backend)?;
            self.maintain_measurement_loss_with_backend(unix_seconds(), &mut lease_backend)?;
            self.maintain_progress_loss_with_backend(unix_seconds(), &mut lease_backend)?;
            if let Some(connection) = self.listener.accept_timeout(MAINTENANCE_INTERVAL)? {
                self.handle_connection_inner::<SystemdSpawnBackend>(
                    connection, None, None, None, None,
                )?;
            }
        }
    }

    pub fn maintain_lease_expiry_with_backend(
        &self,
        now: u64,
        backend: &mut dyn LeaseExpiryObservationBackend,
    ) -> Result<Option<LeaseExpiryOutcome>, ServerError> {
        let Some(host_claim) = HostExecutionClaim::recover_active(&self.config.campaign_root)
            .map_err(|error| ServerError::LeaseExpiry(error.to_string()))?
        else {
            return Ok(None);
        };
        let lock = CampaignLock::acquire(&self.config.campaign_root, host_claim.campaign_id())
            .map_err(|error| ServerError::LeaseExpiry(error.to_string()))?;
        let state = reconcile_campaign(&lock)
            .map_err(|error| ServerError::LeaseExpiry(error.to_string()))?;
        // A committed failure intent owns recovery for this terminal state. Do not let an
        // expired controller lease relabel or interrupt progress- or measurement-loss recovery.
        if state.campaign_state == CampaignState::FailedIncomplete {
            return Ok(Some(LeaseExpiryOutcome::NotDue));
        }
        if now <= state.identity.lease_deadline_unix_seconds {
            return Ok(Some(LeaseExpiryOutcome::NotDue));
        }
        let manifest = verify_stored_manifest_evidence(lock.campaign_directory(), &state)
            .map_err(|error| ServerError::LeaseExpiry(error.to_string()))?;
        backend
            .verify_host(lock.campaign_directory(), &manifest, &state)
            .map_err(ServerError::LeaseExpiry)?;
        let outcome = drive_lease_expiry(&host_claim, &lock, now, backend)
            .map_err(|error| ServerError::LeaseExpiry(error.to_string()))?;
        Ok(Some(outcome))
    }

    pub fn maintain_measurement_loss_with_backend(
        &self,
        now: u64,
        backend: &mut dyn MeasurementLossObservationBackend,
    ) -> Result<Option<MeasurementLossOutcome>, ServerError> {
        let Some(host_claim) = HostExecutionClaim::recover_active(&self.config.campaign_root)
            .map_err(|error| ServerError::MeasurementLoss(error.to_string()))?
        else {
            return Ok(None);
        };
        let lock = CampaignLock::acquire(&self.config.campaign_root, host_claim.campaign_id())
            .map_err(|error| ServerError::MeasurementLoss(error.to_string()))?;
        let state = reconcile_campaign(&lock)
            .map_err(|error| ServerError::MeasurementLoss(error.to_string()))?;
        let recovery_pending =
            if state.campaign_state == CampaignState::FailedIncomplete && state.recorded_failure {
                let report = verify_event_journal(
                    &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
                    &lock.campaign_directory().join(EVENT_HEAD_NAME),
                )
                .map_err(|error| ServerError::MeasurementLoss(error.to_string()))?;
                report.latest_lifecycle.as_ref().is_some_and(|latest| {
                    matches!(
                        latest.transition,
                        LifecycleEvent::MeasurementLossRequested
                            | LifecycleEvent::MeasurementLossCompleted
                    )
                })
            } else {
                false
            };
        if !matches!(
            state.campaign_state,
            CampaignState::I74Running | CampaignState::C74Running
        ) && !recovery_pending
        {
            return Ok(None);
        }
        let manifest = verify_stored_manifest_evidence(lock.campaign_directory(), &state)
            .map_err(|error| ServerError::MeasurementLoss(error.to_string()))?;
        backend
            .prepare_measurement(&manifest, &state)
            .map_err(ServerError::MeasurementLoss)?;
        let (reason, observed_at) = if recovery_pending {
            (None, None)
        } else {
            match backend
                .observe_measurement(lock.campaign_directory(), &manifest, &state)
                .map_err(ServerError::MeasurementLoss)?
            {
                MeasurementObservation::Healthy | MeasurementObservation::Terminal => {
                    return Ok(None);
                }
                MeasurementObservation::Lost(reason) => (Some(reason), Some(now)),
            }
        };
        let outcome = drive_measurement_loss(&host_claim, &lock, now, reason, observed_at, backend)
            .map_err(|error| ServerError::MeasurementLoss(error.to_string()))?;
        Ok(Some(outcome))
    }

    pub fn maintain_progress_loss_with_backend(
        &self,
        now: u64,
        backend: &mut dyn ProgressLossObservationBackend,
    ) -> Result<Option<ProgressLossOutcome>, ServerError> {
        let Some(host_claim) = HostExecutionClaim::recover_active(&self.config.campaign_root)
            .map_err(|error| ServerError::ProgressLoss(error.to_string()))?
        else {
            return Ok(None);
        };
        let lock = CampaignLock::acquire(&self.config.campaign_root, host_claim.campaign_id())
            .map_err(|error| ServerError::ProgressLoss(error.to_string()))?;
        let state = reconcile_campaign(&lock)
            .map_err(|error| ServerError::ProgressLoss(error.to_string()))?;
        let manifest = verify_stored_manifest_evidence(lock.campaign_directory(), &state)
            .map_err(|error| ServerError::ProgressLoss(error.to_string()))?;
        let (observed_checkpoint, startup_progress_unix_seconds) = if state.campaign_state
            == CampaignState::FailedIncomplete
        {
            (state.checkpoint.clone(), None)
        } else if state.campaign_state.is_live() {
            match observe_live_checkpoint_evidence(lock.campaign_directory(), &state) {
                Ok((report, checkpoint)) => {
                    if report.last_observed_unix_seconds > now {
                        return Err(ServerError::ProgressLoss(
                            "checkpoint observation is in the future".to_owned(),
                        ));
                    }
                    if report.last_phase == crate::Phase::Terminal {
                        return Ok(Some(ProgressLossOutcome::NotDue));
                    }
                    (Some(checkpoint), None)
                }
                Err(error)
                    if state.checkpoint.is_none() && checkpoint_evidence_is_absent(&error) =>
                {
                    let report = verify_event_journal(
                        &lock.campaign_directory().join(EVENT_JOURNAL_NAME),
                        &lock.campaign_directory().join(EVENT_HEAD_NAME),
                    )
                    .map_err(|error| ServerError::ProgressLoss(error.to_string()))?;
                    if report.latest_state_after.as_ref() != Some(&state) {
                        return Err(ServerError::ProgressLoss(
                            "startup progress state is not bound to the event journal".to_owned(),
                        ));
                    }
                    let latest = report.latest_lifecycle.as_ref().ok_or_else(|| {
                        ServerError::ProgressLoss(
                            "startup progress lifecycle event is missing".to_owned(),
                        )
                    })?;
                    let expected_start = match state.campaign_state {
                        CampaignState::I74Running => matches!(
                            latest.transition,
                            LifecycleEvent::I74Started | LifecycleEvent::I74Adopted
                        ),
                        CampaignState::C74Running => matches!(
                            latest.transition,
                            LifecycleEvent::C74Started | LifecycleEvent::C74Adopted
                        ),
                        _ => false,
                    };
                    if !expected_start
                        || latest.occurred_at_unix_seconds == 0
                        || latest.occurred_at_unix_seconds > now
                    {
                        return Err(ServerError::ProgressLoss(
                            "startup progress lifecycle event is invalid".to_owned(),
                        ));
                    }
                    (None, Some(latest.occurred_at_unix_seconds))
                }
                Err(error) => return Err(ServerError::ProgressLoss(error.to_string())),
            }
        } else {
            return Ok(Some(ProgressLossOutcome::NotDue));
        };
        if state.campaign_state.is_live() {
            let last_useful_progress_unix_seconds = observed_checkpoint
                .as_ref()
                .map(|checkpoint| checkpoint.useful_progress_unix_seconds)
                .or(startup_progress_unix_seconds)
                .ok_or_else(|| {
                    ServerError::ProgressLoss("progress anchor is missing".to_owned())
                })?;
            let rejection_deadline = last_useful_progress_unix_seconds
                .checked_add(manifest.progress_rejection_gap_seconds)
                .ok_or_else(|| {
                    ServerError::ProgressLoss("progress deadline overflow".to_owned())
                })?;
            if now <= rejection_deadline {
                return Ok(Some(ProgressLossOutcome::NotDue));
            }
        }
        backend
            .verify_host(lock.campaign_directory(), &manifest, &state)
            .map_err(ServerError::ProgressLoss)?;
        let outcome = drive_progress_loss(
            &host_claim,
            &lock,
            now,
            observed_checkpoint,
            startup_progress_unix_seconds,
            manifest.progress_rejection_gap_seconds,
            backend,
        )
        .map_err(|error| ServerError::ProgressLoss(error.to_string()))?;
        Ok(Some(outcome))
    }

    pub fn serve_one(&self) -> Result<(), ServerError> {
        self.serve_one_inner::<SystemdSpawnBackend>(None, None, None, None)
    }

    pub fn serve_one_with_start_backend<B: SpawnBackend + StartObservationBackend>(
        &self,
        backend: &mut B,
    ) -> Result<(), ServerError> {
        self.serve_one_inner(Some(backend), None, None, None)
    }

    pub fn serve_one_with_seal_backend(
        &self,
        backend: &mut dyn SealObservationBackend,
    ) -> Result<(), ServerError> {
        self.serve_one_inner::<SystemdSpawnBackend>(None, Some(backend), None, None)
    }

    pub fn serve_one_with_abort_backend(
        &self,
        backend: &mut dyn AbortObservationBackend,
    ) -> Result<(), ServerError> {
        self.serve_one_inner::<SystemdSpawnBackend>(None, None, Some(backend), None)
    }

    pub fn serve_one_with_host_observation_backend(
        &self,
        backend: &mut dyn HostObservationBackend,
    ) -> Result<(), ServerError> {
        self.serve_one_inner::<SystemdSpawnBackend>(None, None, None, Some(backend))
    }

    fn serve_one_inner<B: SpawnBackend + StartObservationBackend>(
        &self,
        start_backend: Option<&mut B>,
        seal_backend: Option<&mut dyn SealObservationBackend>,
        abort_backend: Option<&mut dyn AbortObservationBackend>,
        host_observation_backend: Option<&mut dyn HostObservationBackend>,
    ) -> Result<(), ServerError> {
        let connection = self.listener.accept()?;
        self.handle_connection_inner(
            connection,
            start_backend,
            seal_backend,
            abort_backend,
            host_observation_backend,
        )
    }

    fn handle_connection_inner<B: SpawnBackend + StartObservationBackend>(
        &self,
        connection: SeqpacketConnection,
        start_backend: Option<&mut B>,
        seal_backend: Option<&mut dyn SealObservationBackend>,
        abort_backend: Option<&mut dyn AbortObservationBackend>,
        host_observation_backend: Option<&mut dyn HostObservationBackend>,
    ) -> Result<(), ServerError> {
        match self.serve_connection_inner(
            connection,
            start_backend,
            seal_backend,
            abort_backend,
            host_observation_backend,
        ) {
            Err(ServerError::Transport(_)) => Ok(()),
            result => result,
        }
    }

    fn serve_connection_inner<B: SpawnBackend + StartObservationBackend>(
        &self,
        connection: SeqpacketConnection,
        start_backend: Option<&mut B>,
        seal_backend: Option<&mut dyn SealObservationBackend>,
        abort_backend: Option<&mut dyn AbortObservationBackend>,
        host_observation_backend: Option<&mut dyn HostObservationBackend>,
    ) -> Result<(), ServerError> {
        let packet = connection.receive_packet()?;
        let wire = match parse_wire_request(&packet) {
            Ok(wire) => wire,
            Err(_) => return Ok(()),
        };
        let now = unix_seconds();
        let response = match connection.peer_credentials() {
            Ok(peer) => match authorize_wire(wire.clone(), &peer, now, &self.policy) {
                Ok(authorized) => {
                    if self.requires_start_evidence_upload(&authorized)
                        && self
                            .receive_start_evidence(&connection, &authorized, now)
                            .is_err()
                    {
                        Ok(error_response_from_request(&authorized.request, now, 9)?)
                    } else {
                        self.dispatch(
                            &authorized,
                            now,
                            start_backend,
                            seal_backend,
                            abort_backend,
                            host_observation_backend,
                        )
                    }
                }
                Err(error) => error_response(&wire, now, service_error_code(&error)),
            },
            Err(_) => error_response(&wire, now, 3),
        }?;
        connection.send_packet(&serde_json::to_vec(&response)?)?;
        Ok(())
    }

    fn requires_start_evidence_upload(&self, authorized: &AuthorizedRequest) -> bool {
        authorized.request.operation == Operation::Start
            && authorized.request.expected_state_revision == 0
            && !self
                .config
                .staging_root
                .join(&authorized.request.campaign_id)
                .exists()
    }

    fn receive_start_evidence(
        &self,
        connection: &SeqpacketConnection,
        authorized: &AuthorizedRequest,
        now: u64,
    ) -> Result<(), StartEvidenceError> {
        let manifest = connection
            .receive_packet_timeout(START_EVIDENCE_PACKET_TIMEOUT)
            .map_err(|error| StartEvidenceError::Io(std::io::Error::other(error.to_string())))?
            .ok_or(StartEvidenceError::Input)?;
        let host_receipt = connection
            .receive_packet_timeout(START_EVIDENCE_PACKET_TIMEOUT)
            .map_err(|error| StartEvidenceError::Io(std::io::Error::other(error.to_string())))?
            .ok_or(StartEvidenceError::Input)?;
        publish_received_start_evidence(
            &self.config.staging_root,
            &authorized.request,
            now,
            &manifest,
            &host_receipt,
        )?;
        Ok(())
    }

    fn dispatch<B: SpawnBackend + StartObservationBackend>(
        &self,
        authorized: &AuthorizedRequest,
        now: u64,
        start_backend: Option<&mut B>,
        seal_backend: Option<&mut dyn SealObservationBackend>,
        abort_backend: Option<&mut dyn AbortObservationBackend>,
        host_observation_backend: Option<&mut dyn HostObservationBackend>,
    ) -> Result<Response, ServerError> {
        let request = &authorized.request;
        if request.operation == Operation::Start {
            return match start_backend {
                Some(backend) => self.dispatch_start(authorized, now, backend),
                None => self.dispatch_systemd_start(authorized, now),
            };
        }
        if request.operation == Operation::Attach {
            return self.dispatch_attach(authorized, now);
        }
        if request.operation == Operation::Seal {
            return self.dispatch_seal(authorized, now, seal_backend);
        }
        if request.operation == Operation::Abort {
            return match abort_backend {
                Some(backend) => self.dispatch_abort(authorized, now, backend),
                None => {
                    let mut backend = SystemdAbortBackend::new();
                    self.dispatch_abort(authorized, now, &mut backend)
                }
            };
        }
        if request.operation == Operation::HostObservation {
            let observed = match host_observation_backend {
                Some(backend) => backend.collect(&self.config.campaign_root),
                None => LiveHostObservationBackend.collect(&self.config.campaign_root),
            };
            return match observed {
                Ok(receipt) => {
                    let receipt_sha256 = crate::sha256_hex(&crate::canonical_json(&receipt)?);
                    let result = HostObservationResult {
                        schema_version: 1,
                        receipt_sha256,
                        receipt,
                    };
                    Ok(success_response(
                        request,
                        now,
                        0,
                        serde_json::to_value(result)?,
                    )?)
                }
                Err(_) => Ok(error_response_from_request(request, now, 6)?),
            };
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

    fn dispatch_start<B: SpawnBackend + StartObservationBackend>(
        &self,
        authorized: &AuthorizedRequest,
        now: u64,
        backend: &mut B,
    ) -> Result<Response, ServerError> {
        match self.admit_start(authorized, now, backend)? {
            StartAdmission::Rejected(response) => Ok(response),
            StartAdmission::Ready(admission) => finish_start(authorized, now, admission, backend),
        }
    }

    fn dispatch_systemd_start(
        &self,
        authorized: &AuthorizedRequest,
        now: u64,
    ) -> Result<Response, ServerError> {
        let mut observer = LiveStartObservationBackend;
        match self.admit_start(authorized, now, &mut observer)? {
            StartAdmission::Rejected(response) => Ok(response),
            StartAdmission::Ready(admission) => {
                let mut backend = SystemdSpawnBackend::new(
                    admission.evidence.manifest.clone(),
                    admission.evidence.campaign_directory.clone(),
                );
                finish_start(authorized, now, admission, &mut backend)
            }
        }
    }

    fn admit_start(
        &self,
        authorized: &AuthorizedRequest,
        now: u64,
        observer: &mut dyn StartObservationBackend,
    ) -> Result<StartAdmission, ServerError> {
        let request = &authorized.request;
        let campaign_directory = self.config.campaign_root.join(&request.campaign_id);
        let evidence = if campaign_directory.exists() {
            load_campaign_evidence(&self.config.campaign_root, request, now)
        } else {
            prepare_campaign_evidence(
                &self.config.campaign_root,
                &self.config.staging_root,
                request,
                now,
            )
        };
        let evidence = match evidence {
            Ok(evidence) => evidence,
            Err(error) => {
                return Ok(StartAdmission::Rejected(error_response_from_request(
                    request,
                    now,
                    start_evidence_error_code(&error),
                )?))
            }
        };
        let identity =
            match frozen_identity_from_manifest(&evidence.manifest, &request.manifest_sha256) {
                Ok(identity) => identity,
                Err(_) => {
                    return Ok(StartAdmission::Rejected(error_response_from_request(
                        request, now, 9,
                    )?))
                }
            };
        let lock = match CampaignLock::acquire(&self.config.campaign_root, &request.campaign_id) {
            Ok(lock) => lock,
            Err(error) => {
                return Ok(StartAdmission::Rejected(error_response_from_request(
                    request,
                    now,
                    state_error_code(&error),
                )?))
            }
        };
        match replay_start_response(&lock, request) {
            Ok(Some(response)) => return Ok(StartAdmission::Rejected(response)),
            Ok(None) => {}
            Err(error) => {
                return Ok(StartAdmission::Rejected(error_response_from_request(
                    request,
                    now,
                    start_lifecycle_error_code(&error),
                )?))
            }
        }
        if observer
            .verify_host(
                &evidence.campaign_directory,
                &evidence.manifest,
                &evidence.host_receipt,
            )
            .is_err()
        {
            return Ok(StartAdmission::Rejected(error_response_from_request(
                request, now, 6,
            )?));
        }
        let host_claim =
            match HostExecutionClaim::acquire(&self.config.campaign_root, &request.campaign_id) {
                Ok(claim) => claim,
                Err(error) => {
                    return Ok(StartAdmission::Rejected(error_response_from_request(
                        request,
                        now,
                        host_execution_error_code(&error),
                    )?))
                }
            };
        Ok(StartAdmission::Ready(AdmittedStart {
            evidence,
            host_claim,
            lock,
            identity,
        }))
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
        let execution = match (state.harness.as_ref(), state.daemon.as_ref()) {
            (Some(harness), Some(daemon)) => {
                match inspect_unit(&harness.unit_name)
                    .and_then(|snapshot| verify_unit_identity(harness, daemon, &snapshot))
                {
                    Ok(()) => {}
                    Err(error) => failures.push(format!("systemd-unit:{error}")),
                }
                if let Err(error) = verify_process_identity(harness) {
                    failures.push(format!("harness-process:{error}"));
                }
                if let Err(error) = verify_process_identity(daemon) {
                    failures.push(format!("daemon-process:{error}"));
                }
                if let Err(error) = verify_process_cpuset(harness, &state.identity.isolated_cpuset)
                {
                    failures.push(format!("harness-cpuset:{error}"));
                }
                if let Err(error) = verify_process_cpuset(daemon, &state.identity.isolated_cpuset) {
                    failures.push(format!("daemon-cpuset:{error}"));
                }
                Some((harness.clone(), daemon.clone()))
            }
            _ => {
                failures.push("process:execution-identity-unavailable".to_owned());
                None
            }
        };
        let observed_checkpoint =
            match observe_live_checkpoint_evidence(lock.campaign_directory(), &state) {
                Ok((report, checkpoint)) if report.last_observed_unix_seconds <= now => {
                    Some(checkpoint)
                }
                Ok(_) => {
                    failures.push("checkpoint:observation-in-the-future".to_owned());
                    None
                }
                Err(error) => {
                    failures.push(format!("checkpoint:{error}"));
                    None
                }
            };
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
        let progress_rejection_gap_seconds = manifest
            .as_ref()
            .map_or(180, |manifest| manifest.progress_rejection_gap_seconds);
        let next = match (execution, observed_checkpoint) {
            (Some((harness, daemon)), Some(checkpoint)) => {
                let mut observed_state = state.clone();
                observed_state.checkpoint = Some(checkpoint.clone());
                let attach = AttachRequest {
                    request_id: request.request_id.clone(),
                    request_sha256: crate::event::request_sha256(request)
                        .map_err(MutationError::from)?,
                    expected_revision: request.expected_state_revision,
                    authorization_sha256: request.controller.authorization_sha256.clone(),
                    repository_id: request.controller.repository_id,
                    run_id: request.controller.run_id,
                    actor_id: request.controller.actor_id,
                    identity: state.identity.clone(),
                    harness,
                    daemon,
                    checkpoint,
                    now_unix_seconds: now,
                    requested_controller_lease_seconds: authorization
                        .expires_at_unix_seconds
                        .saturating_sub(now),
                };
                match apply_attach(&observed_state, &attach, progress_rejection_gap_seconds) {
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
                }
            }
            _ => None,
        };
        if !failures.is_empty() {
            return Ok(transaction
                .reject_with_result(6, Some(serde_json::json!({"failures": failures})))?);
        }
        Ok(transaction.accept(next.ok_or(MutationError::Diverged)?)?)
    }

    fn dispatch_seal(
        &self,
        authorized: &AuthorizedRequest,
        now: u64,
        mut seal_backend: Option<&mut dyn SealObservationBackend>,
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
        let state = match reconcile_campaign(&lock) {
            Ok(state) => state,
            Err(error) => {
                return Ok(error_response_from_request(
                    request,
                    now,
                    mutation_error_code(&error),
                )?)
            }
        };
        let manifest =
            match verify_manifest_evidence(lock.campaign_directory(), request, &state, now) {
                Ok(manifest) => manifest,
                Err(_) => {
                    return Ok(error_response_with_revision(
                        request,
                        now,
                        state.revision,
                        5,
                    )?)
                }
            };
        let host_verified = match seal_backend.as_deref_mut() {
            Some(backend) => backend
                .verify_host(lock.campaign_directory(), &manifest, &state)
                .is_ok(),
            None => {
                verify_host_receipt_evidence(lock.campaign_directory(), &manifest, &state).is_ok()
            }
        };
        if !host_verified {
            return Ok(error_response_with_revision(
                request,
                now,
                state.revision,
                5,
            )?);
        }
        let role = match state.campaign_state {
            CampaignState::I74Running | CampaignState::I74Terminal | CampaignState::I74Sealed => {
                crate::Role::I74
            }
            CampaignState::C74Running
            | CampaignState::C74Terminal
            | CampaignState::CompleteSealed => crate::Role::C74,
            _ => {
                return Ok(error_response_with_revision(
                    request,
                    now,
                    state.revision,
                    5,
                )?)
            }
        };
        let plan = match resolve_packet_plan(
            lock.campaign_directory(),
            &self.config.seal_root,
            &manifest,
            &request.manifest_sha256,
            role,
        ) {
            Ok(plan) => plan,
            Err(_) => {
                return Ok(error_response_with_revision(
                    request,
                    now,
                    state.revision,
                    5,
                )?)
            }
        };
        let host_claim = if state.campaign_state == CampaignState::CompleteSealed {
            HostExecutionClaim::acquire(&self.config.campaign_root, &request.campaign_id)
        } else {
            HostExecutionClaim::recover(&self.config.campaign_root, &request.campaign_id)
        };
        let host_claim = match host_claim {
            Ok(claim) => claim,
            Err(error) => {
                return Ok(error_response_with_revision(
                    request,
                    now,
                    state.revision,
                    host_execution_error_code(&error),
                )?)
            }
        };
        let observed = match state.harness.as_ref() {
            Some(harness) => match seal_backend.as_deref_mut() {
                Some(backend) => match backend.inspect_terminal(harness) {
                    Ok(snapshot) => snapshot,
                    Err(_) => {
                        return Ok(error_response_with_revision(
                            request,
                            now,
                            state.revision,
                            11,
                        )?)
                    }
                },
                None => match inspect_unit(&harness.unit_name) {
                    Ok(snapshot) => snapshot,
                    Err(_) => {
                        return Ok(error_response_with_revision(
                            request,
                            now,
                            state.revision,
                            11,
                        )?)
                    }
                },
            },
            None => UnitSnapshot {
                unit_name: String::new(),
                active_state: String::new(),
                sub_state: String::new(),
                main_pid: 0,
                control_group: String::new(),
                result: String::new(),
            },
        };
        let maximum_files = match usize::try_from(manifest.output_limits.files) {
            Ok(value) => value,
            Err(_) => {
                return Ok(error_response_with_revision(
                    request,
                    now,
                    state.revision,
                    5,
                )?)
            }
        };
        let archive_files = match maximum_files.checked_add(2) {
            Some(value) => value,
            None => {
                return Ok(error_response_with_revision(
                    request,
                    now,
                    state.revision,
                    5,
                )?)
            }
        };
        let result = drive_seal_request(
            &host_claim,
            &lock,
            request,
            now,
            &observed,
            &plan,
            &self.config.seal_root,
            PacketLimits {
                maximum_files,
                maximum_bytes: manifest.output_limits.final_artifact_bytes,
            },
            ArchiveLimits {
                maximum_files: archive_files,
                maximum_uncompressed_bytes: manifest.output_limits.final_artifact_bytes,
                maximum_archive_bytes: manifest.output_limits.final_artifact_bytes,
            },
        );
        match result {
            Ok(response) => Ok(response),
            Err(error) => {
                let revision = reconcile_campaign(&lock)
                    .map(|current| current.revision)
                    .unwrap_or(state.revision);
                Ok(error_response_with_revision(
                    request,
                    now,
                    revision,
                    seal_lifecycle_error_code(&error),
                )?)
            }
        }
    }

    fn dispatch_abort(
        &self,
        authorized: &AuthorizedRequest,
        now: u64,
        backend: &mut dyn AbortObservationBackend,
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
        let state = match reconcile_campaign(&lock) {
            Ok(state) => state,
            Err(error) => {
                return Ok(error_response_from_request(
                    request,
                    now,
                    mutation_error_code(&error),
                )?)
            }
        };
        let manifest =
            match verify_manifest_evidence(lock.campaign_directory(), request, &state, now) {
                Ok(manifest) => manifest,
                Err(_) => {
                    return Ok(error_response_with_revision(
                        request,
                        now,
                        state.revision,
                        5,
                    )?)
                }
            };
        if backend
            .verify_host(lock.campaign_directory(), &manifest, &state)
            .is_err()
        {
            return Ok(error_response_with_revision(
                request,
                now,
                state.revision,
                5,
            )?);
        }
        let completed = state.campaign_state == CampaignState::AbortedIncomplete
            && state.harness.is_none()
            && state.daemon.is_none()
            && state.checkpoint.is_none()
            && state.controller_lease.is_none();
        let host_claim = if completed {
            HostExecutionClaim::acquire(&self.config.campaign_root, &request.campaign_id)
        } else {
            HostExecutionClaim::recover(&self.config.campaign_root, &request.campaign_id)
        };
        let host_claim = match host_claim {
            Ok(claim) => claim,
            Err(error) => {
                return Ok(error_response_with_revision(
                    request,
                    now,
                    state.revision,
                    host_execution_error_code(&error),
                )?)
            }
        };
        match drive_abort_request(&host_claim, &lock, request, now, backend) {
            Ok(response) => Ok(response),
            Err(error) => {
                let revision = reconcile_campaign(&lock)
                    .map(|state| state.revision)
                    .unwrap_or(state.revision);
                Ok(error_response_with_revision(
                    request,
                    now,
                    revision,
                    abort_lifecycle_error_code(&error),
                )?)
            }
        }
    }
}

fn checkpoint_evidence_is_absent(error: &CheckpointEvidenceError) -> bool {
    match error {
        CheckpointEvidenceError::Chain(crate::ChainError::Empty) => true,
        CheckpointEvidenceError::Chain(crate::ChainError::Io(error))
        | CheckpointEvidenceError::Io(error) => error.kind() == std::io::ErrorKind::NotFound,
        _ => false,
    }
}

enum StartAdmission {
    Ready(AdmittedStart),
    Rejected(Response),
}

struct AdmittedStart {
    evidence: PreparedCampaignEvidence,
    host_claim: HostExecutionClaim,
    lock: CampaignLock,
    identity: FrozenIdentity,
}

fn finish_start<B: SpawnBackend>(
    authorized: &AuthorizedRequest,
    now: u64,
    admission: AdmittedStart,
    backend: &mut B,
) -> Result<Response, ServerError> {
    let request = &authorized.request;
    let result = if request.expected_state_revision == 0 {
        drive_i74_start_request(
            &admission.host_claim,
            &admission.lock,
            request,
            admission.identity,
            admission.evidence.manifest.nonce_sha256,
            now,
            backend,
        )
    } else {
        drive_c74_start_request(
            &admission.host_claim,
            &admission.lock,
            request,
            admission.identity,
            admission.evidence.manifest.nonce_sha256,
            now,
            backend,
        )
    };
    match result {
        Ok(response) => Ok(response),
        Err(error) => Ok(error_response_from_request(
            request,
            now,
            start_lifecycle_error_code(&error),
        )?),
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

fn start_evidence_error_code(error: &StartEvidenceError) -> u32 {
    match error {
        StartEvidenceError::Path | StartEvidenceError::Input | StartEvidenceError::Head => 4,
        StartEvidenceError::Manifest(_) | StartEvidenceError::Host(_) => 9,
        StartEvidenceError::Io(_) => 11,
    }
}

fn host_execution_error_code(error: &HostExecutionError) -> u32 {
    match error {
        HostExecutionError::Busy => 10,
        HostExecutionError::Conflict { .. } => 5,
        HostExecutionError::Path => 4,
        HostExecutionError::Io(_) => 11,
    }
}

fn start_lifecycle_error_code(error: &StartLifecycleError) -> u32 {
    match error {
        StartLifecycleError::Event(crate::event::EventError::ReplayConflict { .. }) => 3,
        StartLifecycleError::Binding => 5,
        StartLifecycleError::State(error) => state_error_code(error),
        StartLifecycleError::Mutation(error) => mutation_error_code(error),
        StartLifecycleError::Event(_) | StartLifecycleError::Spawn(_) => 11,
    }
}

fn seal_lifecycle_error_code(error: &SealLifecycleError) -> u32 {
    match error {
        SealLifecycleError::Event(crate::event::EventError::ReplayConflict { .. }) => 3,
        SealLifecycleError::Binding => 5,
        SealLifecycleError::State(error) => state_error_code(error),
        SealLifecycleError::Mutation(error) => mutation_error_code(error),
        SealLifecycleError::Event(_)
        | SealLifecycleError::Checkpoint(_)
        | SealLifecycleError::Unit(_)
        | SealLifecycleError::Artifact(_)
        | SealLifecycleError::Host(_)
        | SealLifecycleError::Json(_) => 11,
    }
}

fn abort_lifecycle_error_code(error: &AbortLifecycleError) -> u32 {
    match error {
        AbortLifecycleError::Event(crate::event::EventError::ReplayConflict { .. }) => 3,
        AbortLifecycleError::Binding => 5,
        AbortLifecycleError::State(error) => state_error_code(error),
        AbortLifecycleError::Mutation(error) => mutation_error_code(error),
        AbortLifecycleError::Event(_)
        | AbortLifecycleError::Host(_)
        | AbortLifecycleError::Backend(_)
        | AbortLifecycleError::Json(_) => 11,
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
