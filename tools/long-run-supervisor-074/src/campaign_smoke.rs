use crate::abort_backend::SystemdAbortBackend;
use crate::abort_lifecycle::drive_abort_request;
use crate::artifact::PacketResult;
use crate::checkpoint_evidence::observe_live_checkpoint_evidence;
use crate::event::request_sha256;
use crate::host_execution::HostExecutionClaim;
use crate::host_receipt::{
    collect_fixture_host_observation, verify_fixture_host_receipt_evidence,
    write_receipt_for_admission,
};
use crate::lease_expiry::{drive_lease_expiry, LeaseExpiryOutcome};
use crate::manifest::{
    frozen_identity_from_manifest, CampaignManifest, ExpectedOutputSchemaSha256s, InstalledBinary,
    OutputLimits, PhaseDurationsSeconds, RoleArgvTemplates,
};
use crate::measurement_loss::{
    drive_measurement_loss, MeasurementLossOutcome, MeasurementLossReason,
};
use crate::mutation::{begin_attach, reconcile_campaign, BeginAttach};
use crate::process_identity::{
    identity_from_snapshot, inspect_cgroup_processes, inspect_process, verify_process_cpuset,
    verify_process_identity, IdentityMismatch, ProcessIdentityError,
};
use crate::progress_loss::{drive_progress_loss, ProgressLossOutcome};
use crate::protocol::{ControllerIdentity, Operation, Request};
use crate::seal_input::{InventoryGuardEvidence, SealInputInventory, SEAL_INPUT_INVENTORY_NAME};
use crate::spawn::{SpawnBackend, SpawnObservation};
use crate::start_lifecycle::drive_i74_start_request;
use crate::state::{apply_attach, AttachRequest, CampaignState};
use crate::state_store::CampaignLock;
use crate::systemd_spawn::SystemdSpawnBackend;
use crate::systemd_unit::{
    expected_command_environment_sha256, inspect_unit_optional, verify_unit_identity,
};
use crate::{
    append_record, build_record, canonical_json, sha256_hex, CheckpointPayload, Phase, Role,
    GENESIS_HASH,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CAMPAIGN_ROOT: &str = "/var/lib/hydracache-performance/campaigns";
const FIXTURE_BINARY: &str = "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture";
const CONTEXT_PATH: &str = "/var/lib/hydracache-performance/campaign-lifecycle-smoke-v1.json";
const SUPERVISOR_SERVICE: &str = "hydracache-performance-supervisor-074.service";
const FIXTURE_DAEMON_DRIFT_AFTER_SECONDS: u64 = 210;
const FIXTURE_DAEMON_DRIFT_SECONDS: u64 = 20;
const FIXTURE_DAEMON_AFTER_DRIFT_SECONDS: u64 = 80;
const START_REHEARSAL_DAEMON_SECONDS: u64 = 840;
const START_REHEARSAL_CHECKPOINT_SECONDS: u64 = 30;
const START_REHEARSAL_COMPLETION_TIMEOUT_SECONDS: u64 = 30;
const START_REHEARSAL_READY_MARKER: &str = ".seal-ready";
const START_REHEARSAL_RELEASE_MARKER: &str = ".seal-release";
const CAMPAIGN_MANIFEST_HEAD_NAME: &str = "campaign-start.sha256";
const START_REHEARSAL_GUARDS: [&str; 2] = [
    "non-product-protected-start-rehearsal-only",
    "product-candidate-started-false",
];
const FIXTURE_LEASE_SECONDS: u64 = 250;
const MAX_CONTEXT_BYTES: u64 = 64 * 1024;
const MAX_FIXTURE_DIAGNOSTIC_BYTES: u64 = 4 * 1024;
const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const GIT_SHA: &str = "0000000000000000000000000000000000000000";

#[derive(Debug, Serialize)]
struct StartRehearsalSealProof<'a> {
    schema_version: u32,
    release: &'static str,
    campaign_id: &'a str,
    role: Role,
    fixture_binary: &'static str,
    product_candidate_started: bool,
    promotable: bool,
    outcome: &'static str,
}

#[derive(Debug, Serialize)]
struct StartRehearsalGuardDocument<'a> {
    schema_version: u32,
    release: &'static str,
    campaign_id: &'a str,
    role: Role,
    guard_id: &'a str,
    passed: bool,
    evidence_relative_paths: &'a [PathBuf],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignSmokeContext {
    schema_version: u32,
    product_candidate_started: bool,
    campaign_id: String,
    manifest_sha256: String,
    start_request: Request,
    controller_pid: u32,
    controller_start_ticks: u64,
    controller_boot_id: String,
}

#[derive(Debug, Serialize)]
pub struct CampaignSmokeStartReceipt {
    pub schema_version: &'static str,
    pub product_candidate_started: bool,
    pub campaign_id: String,
    pub manifest_sha256: String,
    pub controller_pid: u32,
    pub state_revision: u64,
    pub unit_name: String,
    pub harness_pid: u32,
    pub daemon_pid: u32,
    pub checkpoint_sequence: u64,
    pub supervisor_service_inactive: bool,
}

#[derive(Debug, Serialize)]
pub struct CampaignSmokeResumeReceipt {
    pub schema_version: &'static str,
    pub product_candidate_started: bool,
    pub campaign_id: String,
    pub original_controller_exited: bool,
    pub exact_process_pair_recovered: bool,
    pub start_response_replayed: bool,
    pub replay_spawn_calls: u32,
    pub checkpoint_sequence: u64,
    pub attached_revision: u64,
    pub aborted_revision: u64,
    pub active_campaign_released: bool,
}

#[derive(Debug, Serialize)]
pub struct CampaignProgressLossResumeReceipt {
    pub schema_version: &'static str,
    pub product_candidate_started: bool,
    pub campaign_id: String,
    pub original_controller_exited: bool,
    pub exact_process_pair_observed: bool,
    pub checkpoint_sequence: u64,
    pub useful_progress_unix_seconds: u64,
    pub rejection_gap_seconds: u64,
    pub rejection_deadline_unix_seconds: u64,
    pub observed_unix_seconds: u64,
    pub completed_revision: u64,
    pub failed_incomplete: bool,
    pub recorded_failure: bool,
    pub execution_fields_cleared: bool,
    pub diagnostic_sha256: String,
    pub diagnostic_bytes: u64,
    pub unit_stopped: bool,
    pub active_campaign_released: bool,
}

#[derive(Debug, Serialize)]
pub struct CampaignMeasurementLossResumeReceipt {
    pub schema_version: &'static str,
    pub product_candidate_started: bool,
    pub campaign_id: String,
    pub original_controller_exited: bool,
    pub retained_harness_observed: bool,
    pub missing_daemon_observed: bool,
    pub daemon_cpuset_drift_observed: bool,
    pub reason: MeasurementLossReason,
    pub checkpoint_sequence: u64,
    pub observed_unix_seconds: u64,
    pub completed_revision: u64,
    pub failed_incomplete: bool,
    pub recorded_failure: bool,
    pub execution_fields_cleared: bool,
    pub diagnostic_sha256: String,
    pub diagnostic_bytes: u64,
    pub unit_stopped: bool,
    pub active_campaign_released: bool,
}

#[derive(Debug, Serialize)]
pub struct CampaignLeaseExpiryResumeReceipt {
    pub schema_version: &'static str,
    pub product_candidate_started: bool,
    pub campaign_id: String,
    pub original_controller_exited: bool,
    pub exact_process_pair_observed: bool,
    pub checkpoint_sequence: u64,
    pub lease_id: String,
    pub lease_deadline_unix_seconds: u64,
    pub observed_unix_seconds: u64,
    pub completed_revision: u64,
    pub lease_expired_incomplete: bool,
    pub recorded_failure: bool,
    pub execution_fields_cleared: bool,
    pub diagnostic_sha256: String,
    pub diagnostic_bytes: u64,
    pub unit_stopped: bool,
    pub active_campaign_released: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CampaignMeasurementFault {
    MissingDaemon,
    DaemonCpuSetDrift,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CampaignSmokeResumeOutcome {
    ControllerLoss(CampaignSmokeResumeReceipt),
    ProgressLoss(CampaignProgressLossResumeReceipt),
    MeasurementLoss(CampaignMeasurementLossResumeReceipt),
    LeaseExpiry(CampaignLeaseExpiryResumeReceipt),
}

pub fn start_campaign_lifecycle_smoke() -> Result<CampaignSmokeStartReceipt, String> {
    require_root()?;
    require_supervisor_inactive()?;
    ensure_absent(Path::new(CONTEXT_PATH))?;
    let campaign_root = Path::new(CAMPAIGN_ROOT);
    let root = fs::canonicalize(campaign_root).map_err(display)?;
    if HostExecutionClaim::recover_active(&root)
        .map_err(display)?
        .is_some()
    {
        return Err("host already has an active campaign".to_owned());
    }
    let now = unix_seconds()?;
    let controller = inspect_process(std::process::id()).map_err(display)?;
    let campaign_id = sha256_hex(
        format!(
            "campaign-lifecycle-smoke-v1:{}:{}:{}",
            controller.boot_id, controller.start_ticks, now
        )
        .as_bytes(),
    );
    let campaign_directory = root.join(&campaign_id);
    fs::create_dir(&campaign_directory).map_err(display)?;
    fs::set_permissions(&campaign_directory, fs::Permissions::from_mode(0o750)).map_err(display)?;

    let fixture_freeze = campaign_directory.join("fixture-host-freeze.json");
    write_new_file(
        &fixture_freeze,
        br#"{"schema_version":"hydracache-w11-fixture-host-freeze-v1","product_candidate_started":false,"promotable":false}"#,
        0o400,
    )?;
    let host =
        collect_fixture_host_observation(&campaign_directory, &fixture_freeze).map_err(display)?;
    let host_receipt_sha256 =
        write_receipt_for_admission(&host, &campaign_directory).map_err(display)?;
    let mut manifest = build_manifest(
        &campaign_id,
        now,
        &host.machine_id,
        &host.boot_id,
        &host.mount_identity,
        &host.isolated_cpuset,
        &host.housekeeping_cpuset,
        host_receipt_sha256,
    )?;
    manifest.command_environment_sha256 =
        expected_command_environment_sha256(&manifest, &campaign_directory).map_err(display)?;
    let manifest_bytes = canonical_json(&manifest).map_err(display)?;
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    write_new_file(
        &campaign_directory.join("campaign-start.json"),
        &manifest_bytes,
        0o400,
    )?;
    write_new_file(
        &campaign_directory.join("campaign-start.sha256"),
        format!("{manifest_sha256}\n").as_bytes(),
        0o400,
    )?;

    let controller_identity = ControllerIdentity {
        repository_id: 1,
        run_id: 74,
        run_attempt: 1,
        actor_id: 1,
        authorization_sha256: HASH_B.to_owned(),
    };
    let start_request = Request {
        schema_version: 1,
        request_id: uuid_from_hash(&sha256_hex(format!("{campaign_id}:start").as_bytes())),
        operation: Operation::Start,
        campaign_id: campaign_id.clone(),
        expected_state_revision: 0,
        manifest_path: Some(
            campaign_directory
                .join("campaign-start.json")
                .to_string_lossy()
                .into_owned(),
        ),
        manifest_sha256: manifest_sha256.clone(),
        controller: controller_identity,
        abort_reason: None,
        approval_nonce_sha256: None,
    };
    let identity = frozen_identity_from_manifest(&manifest, &manifest_sha256).map_err(display)?;
    let host_claim = HostExecutionClaim::acquire(&root, &campaign_id).map_err(display)?;
    let lock = CampaignLock::acquire(&root, &campaign_id).map_err(display)?;
    let mut backend = SystemdSpawnBackend::new(manifest, campaign_directory.clone());
    let response = drive_i74_start_request(
        &host_claim,
        &lock,
        &start_request,
        identity,
        HASH_A.to_owned(),
        now,
        &mut backend,
    )
    .map_err(display)?;
    if !response.body.ok {
        return Err("campaign lifecycle fixture start was rejected".to_owned());
    }
    let state = reconcile_campaign(&lock).map_err(display)?;
    let (harness, daemon) = state
        .harness
        .as_ref()
        .zip(state.daemon.as_ref())
        .ok_or_else(|| "started fixture has no exact process pair".to_owned())?;
    let (_, checkpoint) = wait_for_checkpoint(&campaign_directory, &state).map_err(|error| {
        let diagnostic = fixture_diagnostic(&campaign_directory);
        format!("{error}; fixture diagnostic: {diagnostic}")
    })?;
    let context = CampaignSmokeContext {
        schema_version: 1,
        product_candidate_started: false,
        campaign_id: campaign_id.clone(),
        manifest_sha256: manifest_sha256.clone(),
        start_request,
        controller_pid: controller.pid,
        controller_start_ticks: controller.start_ticks,
        controller_boot_id: controller.boot_id,
    };
    write_context(Path::new(CONTEXT_PATH), &context)?;
    Ok(CampaignSmokeStartReceipt {
        schema_version: "hydracache-w11-campaign-lifecycle-start-v1",
        product_candidate_started: false,
        campaign_id,
        manifest_sha256,
        controller_pid: context.controller_pid,
        state_revision: state.revision,
        unit_name: harness.unit_name.clone(),
        harness_pid: harness.pid,
        daemon_pid: daemon.pid,
        checkpoint_sequence: checkpoint.sequence,
        supervisor_service_inactive: true,
    })
}

pub fn resume_campaign_lifecycle_smoke() -> Result<CampaignSmokeResumeOutcome, String> {
    require_root()?;
    require_supervisor_inactive()?;
    if let Some(fault) = campaign_measurement_fault()? {
        resume_campaign_measurement_loss_smoke(fault)
            .map(CampaignSmokeResumeOutcome::MeasurementLoss)
    } else if campaign_lease_is_due()? {
        resume_campaign_lease_expiry_smoke().map(CampaignSmokeResumeOutcome::LeaseExpiry)
    } else if campaign_progress_is_due()? {
        resume_campaign_progress_loss_smoke().map(CampaignSmokeResumeOutcome::ProgressLoss)
    } else {
        resume_campaign_controller_loss_smoke().map(CampaignSmokeResumeOutcome::ControllerLoss)
    }
}

fn campaign_measurement_fault() -> Result<Option<CampaignMeasurementFault>, String> {
    let context = read_context(Path::new(CONTEXT_PATH))?;
    if context.schema_version != 1 || context.product_candidate_started {
        return Err("campaign lifecycle context differs".to_owned());
    }
    if original_controller_still_running(&context)? {
        return Err("original controller is still running".to_owned());
    }
    let root = fs::canonicalize(CAMPAIGN_ROOT).map_err(display)?;
    let campaign_directory = root.join(&context.campaign_id);
    let manifest_bytes =
        fs::read(campaign_directory.join("campaign-start.json")).map_err(display)?;
    let manifest = crate::manifest::parse_stored_and_validate(
        &manifest_bytes,
        &context.manifest_sha256,
        &context.campaign_id,
    )
    .map_err(display)?;
    let lock = CampaignLock::acquire(&root, &context.campaign_id).map_err(display)?;
    let state = reconcile_campaign(&lock).map_err(display)?;
    verify_fixture_host_receipt_evidence(
        &campaign_directory,
        &campaign_directory.join("fixture-host-freeze.json"),
        &manifest,
        &state,
    )
    .map_err(display)?;
    let (harness, daemon) = state
        .harness
        .as_ref()
        .zip(state.daemon.as_ref())
        .ok_or_else(|| "started fixture has no exact process pair".to_owned())?;
    let Some(unit) = inspect_unit_optional(&harness.unit_name).map_err(display)? else {
        return Err("campaign fixture unit disappeared".to_owned());
    };
    verify_unit_identity(harness, daemon, &unit).map_err(display)?;
    verify_process_identity(harness).map_err(display)?;
    verify_process_cpuset(harness, &state.identity.isolated_cpuset).map_err(display)?;
    match verify_process_identity(daemon) {
        Ok(()) => match verify_process_cpuset(daemon, &state.identity.isolated_cpuset) {
            Ok(()) => Ok(None),
            Err(ProcessIdentityError::Mismatch(mismatches))
                if mismatches == vec![IdentityMismatch::CpuSet] =>
            {
                verify_exact_retained_pair(harness, daemon)?;
                Ok(Some(CampaignMeasurementFault::DaemonCpuSetDrift))
            }
            Err(error) => Err(format!("fixture daemon cpuset is ambiguous: {error}")),
        },
        Err(ProcessIdentityError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            verify_only_retained_harness(harness)?;
            Ok(Some(CampaignMeasurementFault::MissingDaemon))
        }
        Err(error) => Err(format!("fixture daemon identity is ambiguous: {error}")),
    }
}

fn verify_exact_retained_pair(
    harness: &crate::ProcessIdentity,
    daemon: &crate::ProcessIdentity,
) -> Result<(), String> {
    let processes = inspect_cgroup_processes(&harness.cgroup_path).map_err(display)?;
    let mut observed_pids = processes
        .iter()
        .map(|process| process.pid)
        .collect::<Vec<_>>();
    observed_pids.sort_unstable();
    let mut expected_pids = vec![harness.pid, daemon.pid];
    expected_pids.sort_unstable();
    if observed_pids != expected_pids {
        return Err("fixture cgroup does not contain the exact retained pair".to_owned());
    }
    verify_process_identity(harness).map_err(display)?;
    verify_process_identity(daemon).map_err(display)
}

fn campaign_lease_is_due() -> Result<bool, String> {
    let context = read_context(Path::new(CONTEXT_PATH))?;
    if context.schema_version != 1 || context.product_candidate_started {
        return Err("campaign lifecycle context differs".to_owned());
    }
    if original_controller_still_running(&context)? {
        return Err("original controller is still running".to_owned());
    }
    let root = fs::canonicalize(CAMPAIGN_ROOT).map_err(display)?;
    let campaign_directory = root.join(&context.campaign_id);
    let lock = CampaignLock::acquire(&root, &context.campaign_id).map_err(display)?;
    let state = reconcile_campaign(&lock).map_err(display)?;
    let manifest_bytes =
        fs::read(campaign_directory.join("campaign-start.json")).map_err(display)?;
    let manifest = crate::manifest::parse_stored_and_validate(
        &manifest_bytes,
        &context.manifest_sha256,
        &context.campaign_id,
    )
    .map_err(display)?;
    if state.identity.lease_deadline_unix_seconds != manifest.product_lease_deadline_unix_seconds {
        return Err("campaign lease deadline differs from the manifest".to_owned());
    }
    Ok(lease_deadline_elapsed(
        state.identity.lease_deadline_unix_seconds,
        unix_seconds()?,
    ))
}

fn lease_deadline_elapsed(deadline_unix_seconds: u64, now_unix_seconds: u64) -> bool {
    now_unix_seconds > deadline_unix_seconds
}

fn verify_only_retained_harness(harness: &crate::ProcessIdentity) -> Result<(), String> {
    let processes = inspect_cgroup_processes(&harness.cgroup_path).map_err(display)?;
    if processes.len() != 1 || processes[0].pid != harness.pid {
        return Err("fixture cgroup does not contain only the retained harness".to_owned());
    }
    verify_process_identity(harness).map_err(display)
}

fn campaign_progress_is_due() -> Result<bool, String> {
    let context = read_context(Path::new(CONTEXT_PATH))?;
    if context.schema_version != 1 || context.product_candidate_started {
        return Err("campaign lifecycle context differs".to_owned());
    }
    if original_controller_still_running(&context)? {
        return Err("original controller is still running".to_owned());
    }
    let root = fs::canonicalize(CAMPAIGN_ROOT).map_err(display)?;
    let campaign_directory = root.join(&context.campaign_id);
    let manifest_bytes =
        fs::read(campaign_directory.join("campaign-start.json")).map_err(display)?;
    let manifest = crate::manifest::parse_stored_and_validate(
        &manifest_bytes,
        &context.manifest_sha256,
        &context.campaign_id,
    )
    .map_err(display)?;
    let lock = CampaignLock::acquire(&root, &context.campaign_id).map_err(display)?;
    let state = reconcile_campaign(&lock).map_err(display)?;
    let (_, checkpoint) =
        observe_live_checkpoint_evidence(&campaign_directory, &state).map_err(display)?;
    let deadline = checkpoint
        .useful_progress_unix_seconds
        .checked_add(manifest.progress_rejection_gap_seconds)
        .ok_or_else(|| "progress deadline overflow".to_owned())?;
    Ok(progress_deadline_elapsed(deadline, unix_seconds()?))
}

fn progress_deadline_elapsed(deadline_unix_seconds: u64, now_unix_seconds: u64) -> bool {
    now_unix_seconds > deadline_unix_seconds
}

fn resume_campaign_controller_loss_smoke() -> Result<CampaignSmokeResumeReceipt, String> {
    require_root()?;
    let context_path = Path::new(CONTEXT_PATH);
    let context = read_context(context_path)?;
    if context.schema_version != 1 || context.product_candidate_started {
        return Err("campaign lifecycle context differs".to_owned());
    }
    if original_controller_still_running(&context)? {
        return Err("original controller is still running".to_owned());
    }
    let root = fs::canonicalize(CAMPAIGN_ROOT).map_err(display)?;
    let campaign_directory = root.join(&context.campaign_id);
    let manifest_bytes =
        fs::read(campaign_directory.join("campaign-start.json")).map_err(display)?;
    let manifest = crate::manifest::parse_stored_and_validate(
        &manifest_bytes,
        &context.manifest_sha256,
        &context.campaign_id,
    )
    .map_err(display)?;
    let host_claim = HostExecutionClaim::recover(&root, &context.campaign_id).map_err(display)?;
    let lock = CampaignLock::acquire(&root, &context.campaign_id).map_err(display)?;
    let state = reconcile_campaign(&lock).map_err(display)?;
    let expected_pair = state
        .harness
        .as_ref()
        .zip(state.daemon.as_ref())
        .ok_or_else(|| "durable start has no process pair".to_owned())?;
    let mut observer = SystemdSpawnBackend::new(manifest.clone(), campaign_directory.clone());
    let observed = observer
        .observe(&expected_pair.0.unit_name)
        .map_err(display)?;
    if !matches!(
        observed,
        SpawnObservation::Exact { ref harness, ref daemon }
            if harness == expected_pair.0 && daemon == expected_pair.1
    ) {
        return Err("production spawn backend did not recover the exact process pair".to_owned());
    }

    let identity =
        frozen_identity_from_manifest(&manifest, &context.manifest_sha256).map_err(display)?;
    let mut no_spawn = NoSpawnBackend::default();
    let replay = drive_i74_start_request(
        &host_claim,
        &lock,
        &context.start_request,
        identity,
        HASH_A.to_owned(),
        unix_seconds()?,
        &mut no_spawn,
    )
    .map_err(display)?;
    let start_response_replayed = replay.body.state_revision == state.revision && replay.body.ok;
    if !start_response_replayed || no_spawn.calls != 0 {
        return Err("start replay attempted a second spawn".to_owned());
    }

    let now = unix_seconds()?;
    let (_, checkpoint) =
        observe_live_checkpoint_evidence(&campaign_directory, &state).map_err(display)?;
    let attach_request = Request {
        schema_version: 1,
        request_id: uuid_from_hash(&sha256_hex(
            format!("{}:attach", context.campaign_id).as_bytes(),
        )),
        operation: Operation::Attach,
        campaign_id: context.campaign_id.clone(),
        expected_state_revision: state.revision,
        manifest_path: None,
        manifest_sha256: context.manifest_sha256.clone(),
        controller: context.start_request.controller.clone(),
        abort_reason: None,
        approval_nonce_sha256: None,
    };
    let transaction = match begin_attach(&lock, &attach_request, now).map_err(display)? {
        BeginAttach::New(transaction) => transaction,
        BeginAttach::Replayed(_) => return Err("unexpected attach replay".to_owned()),
    };
    let mut observed_state = transaction.state().clone();
    observed_state.checkpoint = Some(checkpoint.clone());
    let attach = AttachRequest {
        request_id: attach_request.request_id.clone(),
        request_sha256: request_sha256(&attach_request).map_err(display)?,
        expected_revision: attach_request.expected_state_revision,
        authorization_sha256: attach_request.controller.authorization_sha256.clone(),
        repository_id: attach_request.controller.repository_id,
        run_id: attach_request.controller.run_id,
        actor_id: attach_request.controller.actor_id,
        identity: observed_state.identity.clone(),
        harness: observed_state.harness.clone().ok_or("missing harness")?,
        daemon: observed_state.daemon.clone().ok_or("missing daemon")?,
        checkpoint,
        now_unix_seconds: now,
        requested_controller_lease_seconds: 60,
    };
    let attached = apply_attach(
        &observed_state,
        &attach,
        manifest.progress_rejection_gap_seconds,
    )
    .map_err(|decision| format!("attach rejected: {:?}", decision.failures))?;
    let attach_response = transaction.accept(attached).map_err(display)?;
    if !attach_response.body.ok {
        return Err("attach response was not accepted".to_owned());
    }

    let abort_request = Request {
        schema_version: 1,
        request_id: uuid_from_hash(&sha256_hex(
            format!("{}:abort", context.campaign_id).as_bytes(),
        )),
        operation: Operation::Abort,
        campaign_id: context.campaign_id.clone(),
        expected_state_revision: attach_response.body.state_revision,
        manifest_path: None,
        manifest_sha256: context.manifest_sha256.clone(),
        controller: context.start_request.controller,
        abort_reason: Some("operator-request".to_owned()),
        approval_nonce_sha256: Some(HASH_A.to_owned()),
    };
    let mut abort_backend = SystemdAbortBackend::new();
    abort_backend.bind_manifest(manifest);
    let aborted = drive_abort_request(
        &host_claim,
        &lock,
        &abort_request,
        unix_seconds()?,
        &mut abort_backend,
    )
    .map_err(display)?;
    if !aborted.body.ok {
        return Err("abort response was not accepted".to_owned());
    }
    drop(lock);
    drop(host_claim);
    let active_campaign_released = HostExecutionClaim::recover_active(&root)
        .map_err(display)?
        .is_none();
    if !active_campaign_released {
        return Err("terminal abort did not release the host claim".to_owned());
    }
    remove_context(context_path)?;
    Ok(CampaignSmokeResumeReceipt {
        schema_version: "hydracache-w11-campaign-lifecycle-resume-v1",
        product_candidate_started: false,
        campaign_id: context.campaign_id,
        original_controller_exited: true,
        exact_process_pair_recovered: true,
        start_response_replayed,
        replay_spawn_calls: no_spawn.calls,
        checkpoint_sequence: attach.checkpoint.sequence,
        attached_revision: attach_response.body.state_revision,
        aborted_revision: aborted.body.state_revision,
        active_campaign_released,
    })
}

fn resume_campaign_progress_loss_smoke() -> Result<CampaignProgressLossResumeReceipt, String> {
    require_root()?;
    let context_path = Path::new(CONTEXT_PATH);
    let context = read_context(context_path)?;
    if context.schema_version != 1 || context.product_candidate_started {
        return Err("campaign progress-loss context differs".to_owned());
    }
    if original_controller_still_running(&context)? {
        return Err("original controller is still running".to_owned());
    }

    let root = fs::canonicalize(CAMPAIGN_ROOT).map_err(display)?;
    let campaign_directory = root.join(&context.campaign_id);
    let manifest_bytes =
        fs::read(campaign_directory.join("campaign-start.json")).map_err(display)?;
    let manifest = crate::manifest::parse_stored_and_validate(
        &manifest_bytes,
        &context.manifest_sha256,
        &context.campaign_id,
    )
    .map_err(display)?;
    let host_claim = HostExecutionClaim::recover(&root, &context.campaign_id).map_err(display)?;
    let lock = CampaignLock::acquire(&root, &context.campaign_id).map_err(display)?;
    let state = reconcile_campaign(&lock).map_err(display)?;
    let expected_pair = state
        .harness
        .as_ref()
        .zip(state.daemon.as_ref())
        .ok_or_else(|| "started fixture has no exact process pair".to_owned())?;
    let unit_name = expected_pair.0.unit_name.clone();
    let mut observer = SystemdSpawnBackend::new(manifest.clone(), campaign_directory.clone());
    let observed = observer.observe(&unit_name).map_err(display)?;
    if !matches!(
        observed,
        SpawnObservation::Exact { ref harness, ref daemon }
            if harness == expected_pair.0 && daemon == expected_pair.1
    ) {
        return Err("production spawn backend did not observe the exact stalled pair".to_owned());
    }

    let (_, checkpoint) =
        observe_live_checkpoint_evidence(&campaign_directory, &state).map_err(display)?;
    let rejection_deadline_unix_seconds = checkpoint
        .useful_progress_unix_seconds
        .checked_add(manifest.progress_rejection_gap_seconds)
        .ok_or_else(|| "progress deadline overflow".to_owned())?;
    let observed_unix_seconds = unix_seconds()?;
    if !progress_deadline_elapsed(rejection_deadline_unix_seconds, observed_unix_seconds) {
        return Err(format!(
            "progress deadline is not due: wait at least {} seconds",
            rejection_deadline_unix_seconds - observed_unix_seconds + 1
        ));
    }

    verify_fixture_host_receipt_evidence(
        &campaign_directory,
        &campaign_directory.join("fixture-host-freeze.json"),
        &manifest,
        &state,
    )
    .map_err(display)?;
    let mut backend = SystemdAbortBackend::new();
    backend.bind_manifest(manifest.clone());
    let outcome = drive_progress_loss(
        &host_claim,
        &lock,
        observed_unix_seconds,
        Some(checkpoint.clone()),
        None,
        manifest.progress_rejection_gap_seconds,
        &mut backend,
    )
    .map_err(display)?;
    let completed_revision = match outcome {
        ProgressLossOutcome::Completed { state_revision } => state_revision,
        ProgressLossOutcome::NotDue => return Err("progress loss remained not due".to_owned()),
    };
    let completed = reconcile_campaign(&lock).map_err(display)?;
    let execution_fields_cleared = completed.harness.is_none()
        && completed.daemon.is_none()
        && completed.checkpoint.is_none()
        && completed.controller_lease.is_none();
    if completed.revision != completed_revision
        || completed.campaign_state != CampaignState::FailedIncomplete
        || completed.recorded_failure
        || !execution_fields_cleared
    {
        return Err("progress-loss terminal state differs".to_owned());
    }
    let report = crate::event::verify_event_journal(
        &campaign_directory.join(crate::event::EVENT_JOURNAL_NAME),
        &campaign_directory.join(crate::event::EVENT_HEAD_NAME),
    )
    .map_err(display)?;
    let lifecycle = report
        .latest_lifecycle
        .filter(|event| event.transition == crate::event::LifecycleEvent::ProgressLossCompleted)
        .ok_or_else(|| "progress-loss completion event is missing".to_owned())?;
    let diagnostic_path = campaign_directory
        .join("roles")
        .join("i74")
        .join("diagnostics")
        .join(format!(
            "progress-loss-{}.json",
            lifecycle.cause_request_sha256
        ));
    let diagnostic_metadata = fs::symlink_metadata(&diagnostic_path).map_err(display)?;
    if !diagnostic_metadata.is_file()
        || diagnostic_metadata.file_type().is_symlink()
        || diagnostic_metadata.nlink() != 1
        || diagnostic_metadata.uid() != 0
        || diagnostic_metadata.gid() != 0
        || diagnostic_metadata.len() == 0
        || diagnostic_metadata.len() > manifest.output_limits.diagnostic_bytes
    {
        return Err("progress-loss diagnostic is unsafe".to_owned());
    }
    let diagnostic = fs::read(&diagnostic_path).map_err(display)?;
    let diagnostic_sha256 = sha256_hex(&diagnostic);
    let unit_stopped = crate::systemd_unit::inspect_unit_optional(&unit_name)
        .map_err(display)?
        .is_none_or(|unit| unit.active_state != "active");
    if !unit_stopped {
        return Err("stalled fixture unit is still active".to_owned());
    }

    drop(lock);
    drop(host_claim);
    let active_campaign_released = HostExecutionClaim::recover_active(&root)
        .map_err(display)?
        .is_none();
    if !active_campaign_released {
        return Err("progress-loss completion did not release the host claim".to_owned());
    }
    remove_context(context_path)?;
    Ok(CampaignProgressLossResumeReceipt {
        schema_version: "hydracache-w11-campaign-progress-loss-resume-v1",
        product_candidate_started: false,
        campaign_id: context.campaign_id,
        original_controller_exited: true,
        exact_process_pair_observed: true,
        checkpoint_sequence: checkpoint.sequence,
        useful_progress_unix_seconds: checkpoint.useful_progress_unix_seconds,
        rejection_gap_seconds: manifest.progress_rejection_gap_seconds,
        rejection_deadline_unix_seconds,
        observed_unix_seconds,
        completed_revision,
        failed_incomplete: true,
        recorded_failure: completed.recorded_failure,
        execution_fields_cleared,
        diagnostic_sha256,
        diagnostic_bytes: diagnostic_metadata.len(),
        unit_stopped,
        active_campaign_released,
    })
}

fn resume_campaign_measurement_loss_smoke(
    expected_fault: CampaignMeasurementFault,
) -> Result<CampaignMeasurementLossResumeReceipt, String> {
    let context_path = Path::new(CONTEXT_PATH);
    let context = read_context(context_path)?;
    if context.schema_version != 1 || context.product_candidate_started {
        return Err("campaign measurement-loss context differs".to_owned());
    }
    if original_controller_still_running(&context)? {
        return Err("original controller is still running".to_owned());
    }

    let root = fs::canonicalize(CAMPAIGN_ROOT).map_err(display)?;
    let campaign_directory = root.join(&context.campaign_id);
    let manifest_bytes =
        fs::read(campaign_directory.join("campaign-start.json")).map_err(display)?;
    let manifest = crate::manifest::parse_stored_and_validate(
        &manifest_bytes,
        &context.manifest_sha256,
        &context.campaign_id,
    )
    .map_err(display)?;
    let host_claim = HostExecutionClaim::recover(&root, &context.campaign_id).map_err(display)?;
    let lock = CampaignLock::acquire(&root, &context.campaign_id).map_err(display)?;
    let state = reconcile_campaign(&lock).map_err(display)?;
    verify_fixture_host_receipt_evidence(
        &campaign_directory,
        &campaign_directory.join("fixture-host-freeze.json"),
        &manifest,
        &state,
    )
    .map_err(display)?;
    let (harness, daemon) = state
        .harness
        .as_ref()
        .zip(state.daemon.as_ref())
        .ok_or_else(|| "started fixture has no exact process pair".to_owned())?;
    let harness = harness.clone();
    let daemon = daemon.clone();
    let (_, observed_checkpoint) =
        observe_live_checkpoint_evidence(&campaign_directory, &state).map_err(display)?;
    let checkpoint_sequence = observed_checkpoint.sequence;
    let Some(unit) = inspect_unit_optional(&harness.unit_name).map_err(display)? else {
        return Err("measurement-loss fixture unit disappeared".to_owned());
    };
    verify_unit_identity(&harness, &daemon, &unit).map_err(display)?;
    verify_process_identity(&harness).map_err(display)?;
    verify_process_cpuset(&harness, &state.identity.isolated_cpuset).map_err(display)?;
    match expected_fault {
        CampaignMeasurementFault::MissingDaemon => {
            match verify_process_identity(&daemon) {
                Err(ProcessIdentityError::Io(error))
                    if error.kind() == std::io::ErrorKind::NotFound => {}
                Ok(()) => return Err("measurement-loss fixture daemon is still running".to_owned()),
                Err(error) => return Err(format!("fixture daemon identity is ambiguous: {error}")),
            }
            verify_only_retained_harness(&harness)?;
        }
        CampaignMeasurementFault::DaemonCpuSetDrift => {
            verify_process_identity(&daemon).map_err(display)?;
            match verify_process_cpuset(&daemon, &state.identity.isolated_cpuset) {
                Err(ProcessIdentityError::Mismatch(mismatches))
                    if mismatches == vec![IdentityMismatch::CpuSet] => {}
                Ok(()) => return Err("measurement-loss fixture daemon cpuset recovered".to_owned()),
                Err(error) => return Err(format!("fixture daemon cpuset is ambiguous: {error}")),
            }
            verify_exact_retained_pair(&harness, &daemon)?;
        }
    }

    let observed_unix_seconds = unix_seconds()?;
    let reason = MeasurementLossReason::ProcessIdentityDrift;
    let mut backend = SystemdAbortBackend::new();
    backend.bind_manifest(manifest.clone());
    let outcome = drive_measurement_loss(
        &host_claim,
        &lock,
        observed_unix_seconds,
        Some(reason),
        Some(observed_unix_seconds),
        &mut backend,
    )
    .map_err(display)?;
    let MeasurementLossOutcome::Completed { state_revision } = outcome;
    let completed = reconcile_campaign(&lock).map_err(display)?;
    let execution_fields_cleared = completed.harness.is_none()
        && completed.daemon.is_none()
        && completed.checkpoint.is_none()
        && completed.controller_lease.is_none();
    if completed.revision != state_revision
        || completed.campaign_state != CampaignState::FailedIncomplete
        || !completed.recorded_failure
        || !execution_fields_cleared
    {
        return Err("measurement-loss terminal state differs".to_owned());
    }
    let report = crate::event::verify_event_journal(
        &campaign_directory.join(crate::event::EVENT_JOURNAL_NAME),
        &campaign_directory.join(crate::event::EVENT_HEAD_NAME),
    )
    .map_err(display)?;
    let lifecycle = report
        .latest_lifecycle
        .filter(|event| event.transition == crate::event::LifecycleEvent::MeasurementLossCompleted)
        .ok_or_else(|| "measurement-loss completion event is missing".to_owned())?;
    let diagnostic_path = campaign_directory
        .join("roles")
        .join("i74")
        .join("diagnostics")
        .join(format!(
            "measurement-loss-{}.json",
            lifecycle.cause_request_sha256
        ));
    let diagnostic_metadata = fs::symlink_metadata(&diagnostic_path).map_err(display)?;
    if !diagnostic_metadata.is_file()
        || diagnostic_metadata.file_type().is_symlink()
        || diagnostic_metadata.nlink() != 1
        || diagnostic_metadata.uid() != 0
        || diagnostic_metadata.gid() != 0
        || diagnostic_metadata.len() == 0
        || diagnostic_metadata.len() > manifest.output_limits.diagnostic_bytes
    {
        return Err("measurement-loss diagnostic is unsafe".to_owned());
    }
    let diagnostic = fs::read(&diagnostic_path).map_err(display)?;
    let diagnostic_sha256 = sha256_hex(&diagnostic);
    let unit_stopped = inspect_unit_optional(&harness.unit_name)
        .map_err(display)?
        .is_none_or(|unit| unit.active_state != "active");
    if !unit_stopped {
        return Err("measurement-loss fixture unit is still active".to_owned());
    }

    drop(lock);
    drop(host_claim);
    let active_campaign_released = HostExecutionClaim::recover_active(&root)
        .map_err(display)?
        .is_none();
    if !active_campaign_released {
        return Err("measurement-loss completion did not release the host claim".to_owned());
    }
    remove_context(context_path)?;
    Ok(CampaignMeasurementLossResumeReceipt {
        schema_version: "hydracache-w11-campaign-measurement-loss-resume-v1",
        product_candidate_started: false,
        campaign_id: context.campaign_id,
        original_controller_exited: true,
        retained_harness_observed: true,
        missing_daemon_observed: expected_fault == CampaignMeasurementFault::MissingDaemon,
        daemon_cpuset_drift_observed: expected_fault == CampaignMeasurementFault::DaemonCpuSetDrift,
        reason,
        checkpoint_sequence,
        observed_unix_seconds,
        completed_revision: state_revision,
        failed_incomplete: true,
        recorded_failure: completed.recorded_failure,
        execution_fields_cleared,
        diagnostic_sha256,
        diagnostic_bytes: diagnostic_metadata.len(),
        unit_stopped,
        active_campaign_released,
    })
}

fn resume_campaign_lease_expiry_smoke() -> Result<CampaignLeaseExpiryResumeReceipt, String> {
    let context_path = Path::new(CONTEXT_PATH);
    let context = read_context(context_path)?;
    if context.schema_version != 1 || context.product_candidate_started {
        return Err("campaign lease-expiry context differs".to_owned());
    }
    if original_controller_still_running(&context)? {
        return Err("original controller is still running".to_owned());
    }

    let root = fs::canonicalize(CAMPAIGN_ROOT).map_err(display)?;
    let campaign_directory = root.join(&context.campaign_id);
    let manifest_bytes =
        fs::read(campaign_directory.join("campaign-start.json")).map_err(display)?;
    let manifest = crate::manifest::parse_stored_and_validate(
        &manifest_bytes,
        &context.manifest_sha256,
        &context.campaign_id,
    )
    .map_err(display)?;
    let host_claim = HostExecutionClaim::recover(&root, &context.campaign_id).map_err(display)?;
    let lock = CampaignLock::acquire(&root, &context.campaign_id).map_err(display)?;
    let state = reconcile_campaign(&lock).map_err(display)?;
    verify_fixture_host_receipt_evidence(
        &campaign_directory,
        &campaign_directory.join("fixture-host-freeze.json"),
        &manifest,
        &state,
    )
    .map_err(display)?;
    let (harness, daemon) = state
        .harness
        .as_ref()
        .zip(state.daemon.as_ref())
        .ok_or_else(|| "started fixture has no exact process pair".to_owned())?;
    let harness = harness.clone();
    let daemon = daemon.clone();
    let unit_name = harness.unit_name.clone();
    let mut observer = SystemdSpawnBackend::new(manifest.clone(), campaign_directory.clone());
    let observed = observer.observe(&unit_name).map_err(display)?;
    if !matches!(
        observed,
        SpawnObservation::Exact { harness: ref observed_harness, daemon: ref observed_daemon }
            if observed_harness == &harness && observed_daemon == &daemon
    ) {
        return Err("production spawn backend did not observe the exact leased pair".to_owned());
    }
    let (_, checkpoint) =
        observe_live_checkpoint_evidence(&campaign_directory, &state).map_err(display)?;
    let observed_unix_seconds = unix_seconds()?;
    if !lease_deadline_elapsed(
        state.identity.lease_deadline_unix_seconds,
        observed_unix_seconds,
    ) {
        return Err(format!(
            "lease deadline is not due: wait at least {} seconds",
            state.identity.lease_deadline_unix_seconds - observed_unix_seconds + 1
        ));
    }

    let lease_id = state.identity.lease_id.clone();
    let lease_deadline_unix_seconds = state.identity.lease_deadline_unix_seconds;
    let mut backend = SystemdAbortBackend::new();
    backend.bind_manifest(manifest.clone());
    let outcome = drive_lease_expiry(&host_claim, &lock, observed_unix_seconds, &mut backend)
        .map_err(display)?;
    let completed_revision = match outcome {
        LeaseExpiryOutcome::Completed { state_revision } => state_revision,
        LeaseExpiryOutcome::NotDue => return Err("lease expiry remained not due".to_owned()),
    };
    let completed = reconcile_campaign(&lock).map_err(display)?;
    let execution_fields_cleared = completed.harness.is_none()
        && completed.daemon.is_none()
        && completed.checkpoint.is_none()
        && completed.controller_lease.is_none();
    if completed.revision != completed_revision
        || completed.campaign_state != CampaignState::LeaseExpiredIncomplete
        || completed.recorded_failure
        || !execution_fields_cleared
    {
        return Err("lease-expiry terminal state differs".to_owned());
    }
    let diagnostic_path = campaign_directory
        .join("roles")
        .join("i74")
        .join("diagnostics")
        .join(format!("lease-expiry-{lease_id}.json"));
    let diagnostic_metadata = fs::symlink_metadata(&diagnostic_path).map_err(display)?;
    if !diagnostic_metadata.is_file()
        || diagnostic_metadata.file_type().is_symlink()
        || diagnostic_metadata.nlink() != 1
        || diagnostic_metadata.uid() != 0
        || diagnostic_metadata.gid() != 0
        || diagnostic_metadata.len() == 0
        || diagnostic_metadata.len() > manifest.output_limits.diagnostic_bytes
    {
        return Err("lease-expiry diagnostic is unsafe".to_owned());
    }
    let diagnostic_sha256 = sha256_hex(&fs::read(&diagnostic_path).map_err(display)?);
    let unit_stopped = inspect_unit_optional(&unit_name)
        .map_err(display)?
        .is_none_or(|unit| unit.active_state != "active");
    if !unit_stopped {
        return Err("lease-expiry fixture unit is still active".to_owned());
    }

    drop(lock);
    drop(host_claim);
    let active_campaign_released = HostExecutionClaim::recover_active(&root)
        .map_err(display)?
        .is_none();
    if !active_campaign_released {
        return Err("lease-expiry completion did not release the host claim".to_owned());
    }
    remove_context(context_path)?;
    Ok(CampaignLeaseExpiryResumeReceipt {
        schema_version: "hydracache-w11-campaign-lease-expiry-resume-v1",
        product_candidate_started: false,
        campaign_id: context.campaign_id,
        original_controller_exited: true,
        exact_process_pair_observed: true,
        checkpoint_sequence: checkpoint.sequence,
        lease_id,
        lease_deadline_unix_seconds,
        observed_unix_seconds,
        completed_revision,
        lease_expired_incomplete: true,
        recorded_failure: completed.recorded_failure,
        execution_fields_cleared,
        diagnostic_sha256,
        diagnostic_bytes: diagnostic_metadata.len(),
        unit_stopped,
        active_campaign_released,
    })
}

pub fn run_fixture_harness() -> Result<(), String> {
    let campaign_id = std::env::var("HYDRACACHE_CAMPAIGN_ID").map_err(display)?;
    let role = std::env::var("HYDRACACHE_ROLE").map_err(display)?;
    let evidence_directory =
        PathBuf::from(std::env::var("HYDRACACHE_EVIDENCE_DIRECTORY").map_err(display)?);
    let isolated_cpuset = std::env::var("HYDRACACHE_ISOLATED_CPUSET").map_err(display)?;
    let housekeeping_cpuset = std::env::var("HYDRACACHE_HOUSEKEEPING_CPUSET").map_err(display)?;
    if role != "i74" || campaign_id.len() != 64 {
        return Err("fixture environment differs".to_owned());
    }
    let executable = std::env::current_exe().map_err(display)?;
    if executable != Path::new(FIXTURE_BINARY) {
        return Err("fixture executable path differs".to_owned());
    }
    set_current_thread_affinity(&isolated_cpuset)?;
    let mut child =
        spawn_daemon_on_housekeeping(&executable, "campaign-fixture-daemon", &housekeeping_cpuset)?;
    let own_snapshot = inspect_process(std::process::id()).map_err(display)?;
    let unit_name = Path::new(&own_snapshot.cgroup_path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "fixture unit name is unavailable".to_owned())?
        .to_owned();
    let harness = identity_from_snapshot(own_snapshot, &unit_name).map_err(display)?;
    let daemon = identity_from_snapshot(inspect_process(child.id()).map_err(display)?, &unit_name)
        .map_err(display)?;
    let now = unix_seconds()?;
    let record = build_record(
        1,
        GENESIS_HASH,
        CheckpointPayload {
            campaign_id,
            role: Role::I74,
            phase: Phase::Startup,
            phase_epoch: 1,
            monotonic_elapsed_ns: 1,
            wall_clock_utc: format!("{now:020}"),
            observed_unix_seconds: now,
            useful_progress_unix_seconds: now,
            completed: 0,
            failed: 0,
            rejected: 0,
            timed_out: 0,
            outstanding: 1,
            telemetry_sequence: 1,
            milestone: "non-product-campaign-lifecycle-fixture-ready".to_owned(),
            surface_counters: BTreeMap::new(),
            resource_counters: BTreeMap::new(),
            owner_counters: BTreeMap::new(),
            harness,
            daemon,
        },
    )
    .map_err(display)?;
    append_record_on_housekeeping(&evidence_directory, &housekeeping_cpuset, record)?;
    let status = child.wait().map_err(display)?;
    if status.success() {
        Ok(())
    } else {
        Err("fixture daemon failed".to_owned())
    }
}

pub fn run_fixture_daemon() -> Result<(), String> {
    let isolated_cpuset = std::env::var("HYDRACACHE_ISOLATED_CPUSET").map_err(display)?;
    set_current_thread_affinity(&isolated_cpuset)?;
    let mut original = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    let cpu_set_bytes = std::mem::size_of::<libc::cpu_set_t>();
    if unsafe { libc::sched_getaffinity(0, cpu_set_bytes, &mut original) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let available = (0..libc::CPU_SETSIZE as usize)
        .filter(|cpu| unsafe { libc::CPU_ISSET(*cpu, &original) })
        .collect::<Vec<_>>();
    if available.len() < 2 {
        return Err("fixture daemon requires at least two isolated CPUs".to_owned());
    }
    let mut narrowed = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_ZERO(&mut narrowed);
        libc::CPU_SET(available[0], &mut narrowed);
    }

    thread::sleep(Duration::from_secs(FIXTURE_DAEMON_DRIFT_AFTER_SECONDS));
    if unsafe { libc::sched_setaffinity(0, cpu_set_bytes, &narrowed) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    thread::sleep(Duration::from_secs(FIXTURE_DAEMON_DRIFT_SECONDS));
    if unsafe { libc::sched_setaffinity(0, cpu_set_bytes, &original) } != 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    thread::sleep(Duration::from_secs(FIXTURE_DAEMON_AFTER_DRIFT_SECONDS));
    Ok(())
}

pub fn run_start_rehearsal_harness() -> Result<(), String> {
    let campaign_id = std::env::var("HYDRACACHE_CAMPAIGN_ID").map_err(display)?;
    let role = start_rehearsal_role(&std::env::var("HYDRACACHE_ROLE").map_err(display)?)?;
    let evidence_directory =
        PathBuf::from(std::env::var("HYDRACACHE_EVIDENCE_DIRECTORY").map_err(display)?);
    let isolated_cpuset = std::env::var("HYDRACACHE_ISOLATED_CPUSET").map_err(display)?;
    let housekeeping_cpuset = std::env::var("HYDRACACHE_HOUSEKEEPING_CPUSET").map_err(display)?;
    if campaign_id.len() != 64 {
        return Err("start rehearsal fixture environment differs".to_owned());
    }
    let executable = std::env::current_exe().map_err(display)?;
    if executable != Path::new(FIXTURE_BINARY) {
        return Err("start rehearsal fixture executable path differs".to_owned());
    }

    set_current_thread_affinity(&isolated_cpuset)?;
    let mut child = spawn_daemon_on_housekeeping(
        &executable,
        "campaign-start-rehearsal-daemon",
        &housekeeping_cpuset,
    )?;
    let own_snapshot = inspect_process(std::process::id()).map_err(display)?;
    let unit_name = Path::new(&own_snapshot.cgroup_path)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "start rehearsal fixture unit name is unavailable".to_owned())?
        .to_owned();
    let harness = identity_from_snapshot(own_snapshot, &unit_name).map_err(display)?;
    let daemon = identity_from_snapshot(inspect_process(child.id()).map_err(display)?, &unit_name)
        .map_err(display)?;

    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (finish_tx, finish_rx) = mpsc::channel();
    let writer_evidence_directory = evidence_directory.clone();
    let writer = thread::spawn(move || {
        let result = run_start_rehearsal_checkpoint_writer(
            writer_evidence_directory,
            housekeeping_cpuset,
            campaign_id,
            role,
            harness,
            daemon,
            finish_rx,
            &ready_tx,
        );
        if let Err(error) = &result {
            let _ = ready_tx.try_send(Err(error.clone()));
        }
        result
    });
    match ready_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = writer.join();
            return Err(error);
        }
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = writer.join();
            return Err(error.to_string());
        }
    }

    let ready_marker = evidence_directory.join(START_REHEARSAL_READY_MARKER);
    let release_marker = evidence_directory.join(START_REHEARSAL_RELEASE_MARKER);
    if let Err(error) = wait_for_start_rehearsal_ready(&mut child, &ready_marker) {
        let _ = finish_tx.send(false);
        let _ = child.kill();
        let _ = child.wait();
        let _ = writer.join();
        return Err(error);
    }
    finish_tx
        .send(true)
        .map_err(|error| format!("start rehearsal writer completion failed: {error}"))?;
    if let Err(error) = writer
        .join()
        .map_err(|_| "start rehearsal checkpoint writer panicked".to_owned())?
    {
        let _ = child.kill();
        let _ = child.wait();
        let _ = remove_start_rehearsal_marker(&ready_marker);
        return Err(error);
    }
    write_new_file(&release_marker, b"release\n", 0o600)?;
    let status = child.wait().map_err(display)?;
    remove_start_rehearsal_marker(&ready_marker)?;
    remove_start_rehearsal_marker(&release_marker)?;
    if !status.success() {
        return Err("start rehearsal fixture daemon failed".to_owned());
    }
    Ok(())
}

pub fn run_start_rehearsal_daemon() -> Result<(), String> {
    let isolated_cpuset = std::env::var("HYDRACACHE_ISOLATED_CPUSET").map_err(display)?;
    let evidence_directory =
        PathBuf::from(std::env::var("HYDRACACHE_EVIDENCE_DIRECTORY").map_err(display)?);
    set_current_thread_affinity(&isolated_cpuset)?;
    thread::sleep(Duration::from_secs(START_REHEARSAL_DAEMON_SECONDS));
    write_new_file(
        &evidence_directory.join(START_REHEARSAL_READY_MARKER),
        b"ready\n",
        0o600,
    )?;
    wait_for_start_rehearsal_release(&evidence_directory.join(START_REHEARSAL_RELEASE_MARKER))
}

fn wait_for_start_rehearsal_ready(
    child: &mut std::process::Child,
    marker: &Path,
) -> Result<(), String> {
    let deadline = Instant::now()
        + Duration::from_secs(
            START_REHEARSAL_DAEMON_SECONDS + START_REHEARSAL_COMPLETION_TIMEOUT_SECONDS,
        );
    loop {
        if marker_matches(marker, b"ready\n")? {
            return Ok(());
        }
        if let Some(status) = child.try_wait().map_err(display)? {
            return Err(format!(
                "start rehearsal fixture daemon exited before terminal publication: {status}"
            ));
        }
        if Instant::now() >= deadline {
            return Err("start rehearsal fixture daemon readiness timed out".to_owned());
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn wait_for_start_rehearsal_release(marker: &Path) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(START_REHEARSAL_COMPLETION_TIMEOUT_SECONDS);
    loop {
        if marker_matches(marker, b"release\n")? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("start rehearsal terminal publication timed out".to_owned());
        }
        thread::sleep(Duration::from_millis(50));
    }
}

fn marker_matches(path: &Path, expected: &[u8]) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.gid() != unsafe { libc::getegid() }
        || metadata.mode() & 0o7777 != 0o600
        || metadata.len() != expected.len() as u64
        || fs::read(path).map_err(display)? != expected
    {
        return Err("start rehearsal completion marker is unsafe".to_owned());
    }
    Ok(true)
}

fn remove_start_rehearsal_marker(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "start rehearsal marker parent is unavailable".to_owned())?;
    fs::remove_file(path).map_err(display)?;
    File::open(parent)
        .map_err(display)?
        .sync_all()
        .map_err(display)
}

fn run_start_rehearsal_checkpoint_writer(
    evidence_directory: PathBuf,
    housekeeping_cpuset: String,
    campaign_id: String,
    role: Role,
    harness: crate::ProcessIdentity,
    daemon: crate::ProcessIdentity,
    finish_rx: mpsc::Receiver<bool>,
    ready_tx: &mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    set_current_thread_affinity(&housekeeping_cpuset)?;
    let started = Instant::now();
    let mut sequence = 1_u64;
    let mut previous = GENESIS_HASH.to_owned();
    loop {
        let now = unix_seconds()?;
        let record = build_record(
            sequence,
            &previous,
            CheckpointPayload {
                campaign_id: campaign_id.clone(),
                role: role.clone(),
                phase: Phase::Startup,
                phase_epoch: sequence,
                monotonic_elapsed_ns: u64::try_from(started.elapsed().as_nanos())
                    .map_err(display)?,
                wall_clock_utc: format!("{now:020}"),
                observed_unix_seconds: now,
                useful_progress_unix_seconds: now,
                completed: sequence.saturating_sub(1),
                failed: 0,
                rejected: 0,
                timed_out: 0,
                outstanding: 1,
                telemetry_sequence: sequence,
                milestone: format!("non-product-signed-start-rehearsal-{sequence}"),
                surface_counters: BTreeMap::new(),
                resource_counters: BTreeMap::new(),
                owner_counters: BTreeMap::new(),
                harness: harness.clone(),
                daemon: daemon.clone(),
            },
        )
        .map_err(display)?;
        append_record(
            &evidence_directory.join("checkpoints.jsonl"),
            &evidence_directory.join("checkpoints.head"),
            &record,
        )
        .map_err(display)?;
        previous = record.record_sha256;
        if sequence == 1 {
            ready_tx.send(Ok(())).map_err(|error| error.to_string())?;
        }
        match finish_rx.recv_timeout(Duration::from_secs(START_REHEARSAL_CHECKPOINT_SECONDS)) {
            Ok(true) => {
                return publish_start_rehearsal_terminal_evidence(
                    &evidence_directory,
                    &campaign_id,
                    &role,
                    &harness,
                    &daemon,
                    sequence,
                    &previous,
                    started.elapsed(),
                    unix_seconds()?,
                );
            }
            Ok(false) => return Err("start rehearsal fixture daemon failed".to_owned()),
            Err(RecvTimeoutError::Disconnected) => {
                return Err("start rehearsal completion channel disconnected".to_owned());
            }
            Err(RecvTimeoutError::Timeout) => {
                sequence = sequence
                    .checked_add(1)
                    .ok_or_else(|| "start rehearsal checkpoint sequence overflow".to_owned())?;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn publish_start_rehearsal_terminal_evidence(
    evidence_directory: &Path,
    campaign_id: &str,
    role: &Role,
    harness: &crate::ProcessIdentity,
    daemon: &crate::ProcessIdentity,
    sequence: u64,
    previous_record_sha256: &str,
    elapsed: Duration,
    now: u64,
) -> Result<(), String> {
    let roles_directory = evidence_directory
        .parent()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some("roles"))
        .ok_or_else(|| "start rehearsal role directory differs".to_owned())?;
    let campaign_directory = roles_directory
        .parent()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some(campaign_id))
        .ok_or_else(|| "start rehearsal campaign directory differs".to_owned())?;
    if evidence_directory
        .file_name()
        .and_then(|name| name.to_str())
        != Some(start_rehearsal_role_name(role))
    {
        return Err("start rehearsal evidence role differs".to_owned());
    }

    let manifest_sha256 = read_start_rehearsal_manifest_head(campaign_directory)?;
    publish_start_rehearsal_terminal_documents(
        campaign_directory,
        evidence_directory,
        campaign_id,
        &manifest_sha256,
        role,
        harness,
        daemon,
        sequence,
        previous_record_sha256,
        elapsed,
        now,
    )
}

#[allow(clippy::too_many_arguments)]
fn publish_start_rehearsal_terminal_documents(
    campaign_directory: &Path,
    evidence_directory: &Path,
    campaign_id: &str,
    manifest_sha256: &str,
    role: &Role,
    harness: &crate::ProcessIdentity,
    daemon: &crate::ProcessIdentity,
    sequence: u64,
    previous_record_sha256: &str,
    elapsed: Duration,
    now: u64,
) -> Result<(), String> {
    if !crate::is_hash(campaign_id)
        || !crate::is_hash(manifest_sha256)
        || campaign_directory
            .file_name()
            .and_then(|name| name.to_str())
            != Some(campaign_id)
    {
        return Err("start rehearsal terminal manifest differs".to_owned());
    }

    let terminal_sequence = sequence
        .checked_add(1)
        .ok_or_else(|| "start rehearsal terminal sequence overflow".to_owned())?;
    let terminal = build_record(
        terminal_sequence,
        previous_record_sha256,
        CheckpointPayload {
            campaign_id: campaign_id.to_owned(),
            role: role.clone(),
            phase: Phase::Terminal,
            phase_epoch: terminal_sequence,
            monotonic_elapsed_ns: u64::try_from(elapsed.as_nanos()).map_err(display)?,
            wall_clock_utc: format!("{now:020}"),
            observed_unix_seconds: now,
            useful_progress_unix_seconds: now,
            completed: sequence,
            failed: 0,
            rejected: 0,
            timed_out: 0,
            outstanding: 0,
            telemetry_sequence: terminal_sequence,
            milestone: "non-product-signed-seal-rehearsal-terminal".to_owned(),
            surface_counters: BTreeMap::new(),
            resource_counters: BTreeMap::new(),
            owner_counters: BTreeMap::new(),
            harness: harness.clone(),
            daemon: daemon.clone(),
        },
    )
    .map_err(display)?;
    append_record(
        &evidence_directory.join("checkpoints.jsonl"),
        &evidence_directory.join("checkpoints.head"),
        &terminal,
    )
    .map_err(display)?;

    let role_root = PathBuf::from("roles").join(start_rehearsal_role_name(role));
    let proof_relative = role_root.join("non-product-seal-proof.json");
    let proof = canonical_json(&StartRehearsalSealProof {
        schema_version: 1,
        release: "0.74",
        campaign_id,
        role: role.clone(),
        fixture_binary: FIXTURE_BINARY,
        product_candidate_started: false,
        promotable: false,
        outcome: "complete-non-product-fixture",
    })
    .map_err(display)?;
    write_new_file(&campaign_directory.join(&proof_relative), &proof, 0o600)?;

    let guard_directory = evidence_directory.join("guards");
    fs::create_dir(&guard_directory).map_err(display)?;
    File::open(evidence_directory)
        .map_err(display)?
        .sync_all()
        .map_err(display)?;
    let evidence_paths = vec![proof_relative.clone()];
    let mut inventory_guards = Vec::with_capacity(START_REHEARSAL_GUARDS.len());
    let mut raw_files = BTreeSet::from([
        PathBuf::from("campaign-start.json"),
        role_root.join("checkpoints.jsonl"),
        proof_relative,
    ]);
    for guard_id in START_REHEARSAL_GUARDS {
        let guard_relative = role_root
            .join("guards")
            .join(format!("{}.json", sha256_hex(guard_id.as_bytes())));
        let document = canonical_json(&StartRehearsalGuardDocument {
            schema_version: 1,
            release: "0.74",
            campaign_id,
            role: role.clone(),
            guard_id,
            passed: true,
            evidence_relative_paths: &evidence_paths,
        })
        .map_err(display)?;
        write_new_file(&campaign_directory.join(&guard_relative), &document, 0o600)?;
        raw_files.insert(guard_relative.clone());
        inventory_guards.push(InventoryGuardEvidence {
            id: guard_id.to_owned(),
            passed: true,
            source_relative_path: guard_relative,
        });
    }

    let inventory_relative = role_root.join(SEAL_INPUT_INVENTORY_NAME);
    raw_files.insert(inventory_relative.clone());
    let inventory = SealInputInventory {
        schema_version: 1,
        release: "0.74".to_owned(),
        campaign_id: campaign_id.to_owned(),
        campaign_manifest_sha256: manifest_sha256.to_owned(),
        role: role.clone(),
        result: PacketResult::Complete,
        terminal_reason: None,
        journal_relative_path: role_root.join("checkpoints.jsonl"),
        guard_evidence: inventory_guards,
        raw_files: raw_files.into_iter().collect(),
    };
    let inventory_bytes = canonical_json(&inventory).map_err(display)?;
    write_new_file(
        &campaign_directory.join(&inventory_relative),
        &inventory_bytes,
        0o600,
    )?;

    Ok(())
}

fn start_rehearsal_role(value: &str) -> Result<Role, String> {
    match value {
        "i74" => Ok(Role::I74),
        "c74" => Ok(Role::C74),
        _ => Err("start rehearsal fixture environment differs".to_owned()),
    }
}

fn start_rehearsal_role_name(role: &Role) -> &'static str {
    match role {
        Role::I74 => "i74",
        Role::C74 => "c74",
    }
}

fn read_start_rehearsal_manifest_head(campaign_directory: &Path) -> Result<String, String> {
    let campaign_metadata = fs::symlink_metadata(campaign_directory).map_err(display)?;
    let path = campaign_directory.join(CAMPAIGN_MANIFEST_HEAD_NAME);
    let metadata = fs::symlink_metadata(&path).map_err(display)?;
    if !campaign_metadata.is_dir()
        || campaign_metadata.file_type().is_symlink()
        || campaign_metadata.mode() & 0o7777 != 0o750
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != campaign_metadata.uid()
        || metadata.gid() != campaign_metadata.gid()
        || metadata.gid() != unsafe { libc::getegid() }
        || metadata.mode() & 0o7777 != 0o440
        || metadata.len() != 65
    {
        return Err("start rehearsal manifest head is unsafe".to_owned());
    }
    let bytes = fs::read(&path).map_err(display)?;
    let value = bytes
        .strip_suffix(b"\n")
        .and_then(|value| std::str::from_utf8(value).ok())
        .filter(|value| crate::is_hash(value))
        .ok_or_else(|| "start rehearsal manifest head is unsafe".to_owned())?;
    Ok(value.to_owned())
}

fn append_record_on_housekeeping(
    evidence_directory: &Path,
    housekeeping_cpuset: &str,
    record: crate::RecordEnvelope,
) -> Result<(), String> {
    let directory = evidence_directory.to_owned();
    let cpuset = housekeeping_cpuset.to_owned();
    thread::spawn(move || {
        set_current_thread_affinity(&cpuset)?;
        append_record(
            &directory.join("checkpoints.jsonl"),
            &directory.join("checkpoints.head"),
            &record,
        )
        .map_err(display)
    })
    .join()
    .map_err(|_| "fixture checkpoint writer panicked".to_owned())?
}

fn spawn_daemon_on_housekeeping(
    executable: &Path,
    command: &str,
    housekeeping_cpuset: &str,
) -> Result<std::process::Child, String> {
    use std::os::unix::process::CommandExt;

    let cpus = parse_cpuset(housekeeping_cpuset)?;
    let mut child = Command::new(executable);
    child.arg(command);
    unsafe {
        child.pre_exec(move || set_affinity(&cpus).map_err(std::io::Error::other));
    }
    child.spawn().map_err(display)
}

fn set_current_thread_affinity(cpuset: &str) -> Result<(), String> {
    set_affinity(&parse_cpuset(cpuset)?)
}

fn set_affinity(cpus: &[usize]) -> Result<(), String> {
    let mut affinity = unsafe { std::mem::zeroed::<libc::cpu_set_t>() };
    unsafe {
        libc::CPU_ZERO(&mut affinity);
        for cpu in cpus {
            libc::CPU_SET(*cpu, &mut affinity);
        }
    }
    if unsafe { libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &affinity) } != 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

fn parse_cpuset(value: &str) -> Result<Vec<usize>, String> {
    if value.is_empty() || value.len() > 256 || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err("fixture cpuset is invalid".to_owned());
    }
    let mut cpus = Vec::new();
    for item in value.split(',') {
        let mut bounds = item.split('-');
        let first = bounds
            .next()
            .ok_or_else(|| "fixture cpuset is invalid".to_owned())?
            .parse::<usize>()
            .map_err(display)?;
        let last = bounds
            .next()
            .map_or(Ok(first), |value| value.parse::<usize>().map_err(display))?;
        if bounds.next().is_some() || first > last || last >= libc::CPU_SETSIZE as usize {
            return Err("fixture cpuset is invalid".to_owned());
        }
        cpus.extend(first..=last);
    }
    cpus.sort_unstable();
    let original_len = cpus.len();
    cpus.dedup();
    if cpus.is_empty() || cpus.len() != original_len {
        return Err("fixture cpuset is invalid".to_owned());
    }
    Ok(cpus)
}

#[derive(Default)]
struct NoSpawnBackend {
    calls: u32,
}

impl SpawnBackend for NoSpawnBackend {
    type Error = String;

    fn start_once(
        &mut self,
        _intent: &crate::spawn::SpawnIntent,
    ) -> Result<SpawnObservation, Self::Error> {
        self.calls += 1;
        Err("start replay invoked start_once".to_owned())
    }

    fn observe(&mut self, _unit_name: &str) -> Result<SpawnObservation, Self::Error> {
        self.calls += 1;
        Err("start replay invoked observe".to_owned())
    }
}

#[allow(clippy::too_many_arguments)]
fn build_manifest(
    campaign_id: &str,
    now: u64,
    machine_id: &str,
    boot_id: &str,
    mount_identity: &str,
    isolated_cpuset: &str,
    housekeeping_cpuset: &str,
    host_receipt_sha256: String,
) -> Result<CampaignManifest, String> {
    let metadata = fs::symlink_metadata(FIXTURE_BINARY).map_err(display)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("fixture binary is unsafe".to_owned());
    }
    let binary = InstalledBinary {
        role: "i74".to_owned(),
        path: FIXTURE_BINARY.to_owned(),
        sha256: sha256_hex(&fs::read(FIXTURE_BINARY).map_err(display)?),
        size: metadata.len(),
        inode: metadata.ino(),
        device: metadata.dev(),
        uid: u64::from(metadata.uid()),
        gid: u64::from(metadata.gid()),
        mode: u64::from(metadata.mode() & 0o7777),
    };
    let mut companion = binary.clone();
    companion.role = "c74".to_owned();
    Ok(CampaignManifest {
        schema_version: 1,
        repository_id: 1,
        authorization_identity: "non-product-root-smoke".to_owned(),
        contract_sha256: HASH_A.to_owned(),
        tooling_sha: GIT_SHA.to_owned(),
        i74_source_sha: GIT_SHA.to_owned(),
        c74_source_sha: GIT_SHA.to_owned(),
        i74_tree_sha: GIT_SHA.to_owned(),
        c74_tree_sha: GIT_SHA.to_owned(),
        i74_cargo_lock_sha256: HASH_A.to_owned(),
        c74_cargo_lock_sha256: HASH_A.to_owned(),
        i74_dirty: false,
        c74_dirty: false,
        scenario_sha256: HASH_A.to_owned(),
        workload_sha256: HASH_A.to_owned(),
        offered_load_sha256: HASH_A.to_owned(),
        estimator_sha256: HASH_A.to_owned(),
        thresholds_sha256: HASH_A.to_owned(),
        host_receipt_sha256,
        lease_id: "00000000-0000-4000-8000-000000000074".to_owned(),
        machine_id: machine_id.to_owned(),
        boot_id: boot_id.to_owned(),
        mount_identity: mount_identity.to_owned(),
        isolated_cpuset: isolated_cpuset.to_owned(),
        housekeeping_cpuset: housekeeping_cpuset.to_owned(),
        seed: 74,
        checkpoint_cadence_seconds: 30,
        progress_warning_gap_seconds: 90,
        progress_rejection_gap_seconds: 180,
        diagnostic_grace_seconds: 30,
        product_lease_deadline_unix_seconds: now.saturating_add(FIXTURE_LEASE_SECONDS),
        maximum_campaign_bytes: 21_474_836_480,
        maximum_campaign_files: 20_000,
        installed_binaries: vec![binary, companion],
        argv_templates: RoleArgvTemplates {
            i74: vec![
                FIXTURE_BINARY.to_owned(),
                "campaign-fixture-harness".to_owned(),
            ],
            c74: vec![
                FIXTURE_BINARY.to_owned(),
                "campaign-fixture-harness".to_owned(),
            ],
        },
        command_environment_sha256: HASH_A.to_owned(),
        role_order: vec!["i74".to_owned(), "c74".to_owned()],
        phase_durations_seconds: PhaseDurationsSeconds {
            warmup: 55,
            measured: 55,
            drain: 55,
            durable_companion: 55,
            post_work_idle: 55,
            reconciliation: 55,
        },
        output_limits: OutputLimits {
            stdout_bytes: 1_048_576,
            stderr_bytes: 1_048_576,
            diagnostic_bytes: 1_048_576,
            final_artifact_bytes: 1_048_576,
            files: 100,
        },
        expected_output_schema_sha256s: ExpectedOutputSchemaSha256s {
            checkpoint: HASH_A.to_owned(),
            measurement: HASH_A.to_owned(),
            reconciliation: HASH_A.to_owned(),
            raw_manifest: HASH_A.to_owned(),
            packet_manifest: HASH_A.to_owned(),
        },
        required_final_guards: vec!["non-product-campaign-lifecycle-smoke".to_owned()],
        secret_identifiers: Vec::new(),
        release: "0.74".to_owned(),
        campaign_id: campaign_id.to_owned(),
        nonce_sha256: HASH_A.to_owned(),
        dirty: false,
        controller_history: Vec::new(),
        state: "PREPARED".to_owned(),
    })
}

fn wait_for_checkpoint(
    campaign_directory: &Path,
    state: &crate::state::DurableCampaignState,
) -> Result<(crate::VerificationReport, crate::state::CheckpointHead), String> {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        match observe_live_checkpoint_evidence(campaign_directory, state) {
            Ok(observed) => return Ok(observed),
            Err(_) if std::time::Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn fixture_diagnostic(campaign_directory: &Path) -> String {
    let path = campaign_directory
        .join("roles")
        .join("i74")
        .join("stderr.log");
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => return format!("stderr metadata unavailable: {error}"),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_FIXTURE_DIAGNOSTIC_BYTES
    {
        return "stderr is unsafe or oversized".to_owned();
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    if let Err(error) = File::open(&path).and_then(|file| {
        file.take(MAX_FIXTURE_DIAGNOSTIC_BYTES + 1)
            .read_to_end(&mut bytes)
    }) {
        return format!("stderr read failed: {error}");
    }
    if bytes.len() as u64 > MAX_FIXTURE_DIAGNOSTIC_BYTES {
        return "stderr is oversized".to_owned();
    }
    match String::from_utf8(bytes) {
        Ok(value) if value.trim().is_empty() => "stderr is empty".to_owned(),
        Ok(value) => value.trim().to_owned(),
        Err(_) => "stderr is not UTF-8".to_owned(),
    }
}

fn original_controller_still_running(context: &CampaignSmokeContext) -> Result<bool, String> {
    match inspect_process(context.controller_pid) {
        Ok(process) => Ok(process.start_ticks == context.controller_start_ticks
            && process.boot_id == context.controller_boot_id),
        Err(crate::process_identity::ProcessIdentityError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(false)
        }
        Err(error) => Err(error.to_string()),
    }
}

fn write_context(path: &Path, context: &CampaignSmokeContext) -> Result<(), String> {
    let bytes = serde_json::to_vec(context).map_err(display)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_CONTEXT_BYTES {
        return Err("campaign lifecycle context is oversized".to_owned());
    }
    write_new_file(path, &bytes, 0o600)
}

fn read_context(path: &Path) -> Result<CampaignSmokeContext, String> {
    let metadata = fs::symlink_metadata(path).map_err(display)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != 0
        || metadata.gid() != 0
        || metadata.mode() & 0o7777 != 0o600
        || metadata.len() == 0
        || metadata.len() > MAX_CONTEXT_BYTES
    {
        return Err("campaign lifecycle context is unsafe".to_owned());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .map_err(display)?
        .take(MAX_CONTEXT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(display)?;
    let context: CampaignSmokeContext = serde_json::from_slice(&bytes).map_err(display)?;
    if serde_json::to_vec(&context).map_err(display)? != bytes {
        return Err("campaign lifecycle context is non-canonical".to_owned());
    }
    Ok(context)
}

fn write_new_file(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "output parent is unavailable".to_owned())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .map_err(display)?;
    file.write_all(bytes).map_err(display)?;
    file.sync_all().map_err(display)?;
    File::open(parent)
        .map_err(display)?
        .sync_all()
        .map_err(display)?;
    Ok(())
}

fn remove_context(path: &Path) -> Result<(), String> {
    let parent = path.parent().ok_or("context parent is unavailable")?;
    fs::remove_file(path).map_err(display)?;
    File::open(parent)
        .map_err(display)?
        .sync_all()
        .map_err(display)?;
    Ok(())
}

fn ensure_absent(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err("campaign lifecycle context already exists".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

fn require_root() -> Result<(), String> {
    if unsafe { libc::geteuid() } == 0 {
        Ok(())
    } else {
        Err("root is required".to_owned())
    }
}

fn require_supervisor_inactive() -> Result<(), String> {
    let output = Command::new("/usr/bin/systemctl")
        .args(["is-active", SUPERVISOR_SERVICE])
        .output()
        .map_err(display)?;
    let state = std::str::from_utf8(&output.stdout).map_err(display)?.trim();
    if state == "inactive" {
        Ok(())
    } else {
        Err(format!(
            "campaign fixture requires the production supervisor to be inactive, observed {state:?}"
        ))
    }
}

fn unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(display)
}

fn uuid_from_hash(hash: &str) -> String {
    let mut bytes = hash.as_bytes()[..32].to_vec();
    bytes[12] = b'4';
    bytes[16] = b'8';
    let value = String::from_utf8(bytes).expect("hash is ASCII");
    format!(
        "{}-{}-{}-{}-{}",
        &value[..8],
        &value[8..12],
        &value[12..16],
        &value[16..20],
        &value[20..32]
    )
}

fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        fixture_diagnostic, lease_deadline_elapsed, parse_cpuset, progress_deadline_elapsed,
        publish_start_rehearsal_terminal_evidence, start_rehearsal_role, start_rehearsal_role_name,
        uuid_from_hash, FIXTURE_DAEMON_AFTER_DRIFT_SECONDS, FIXTURE_DAEMON_DRIFT_AFTER_SECONDS,
        FIXTURE_DAEMON_DRIFT_SECONDS, FIXTURE_LEASE_SECONDS, HASH_A,
    };
    use crate::manifest::{
        CampaignManifest, ExpectedOutputSchemaSha256s, InstalledBinary, OutputLimits,
        PhaseDurationsSeconds, RoleArgvTemplates,
    };
    use crate::{
        append_record, build_record, canonical_json, sha256_hex, verify_journal, CheckpointPayload,
        Phase, ProcessIdentity, Role, GENESIS_HASH,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::Duration;

    fn process(pid: u32) -> ProcessIdentity {
        ProcessIdentity {
            boot_id: "1".repeat(36),
            pid,
            start_ticks: u64::from(pid) + 100,
            process_group: 40,
            cgroup_path: "/system.slice/hydracache-performance-074-test.service".to_owned(),
            cgroup_inode: 50,
            unit_name: "hydracache-performance-074-test.service".to_owned(),
        }
    }

    fn terminal_manifest(campaign_id: &str) -> CampaignManifest {
        let installed = |role: &str| InstalledBinary {
            role: role.to_owned(),
            path: "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture".to_owned(),
            sha256: "a".repeat(64),
            size: 1,
            inode: 2,
            device: 3,
            uid: 0,
            gid: 0,
            mode: 0o755,
        };
        CampaignManifest {
            schema_version: 1,
            repository_id: 1,
            authorization_identity: "performance-reference-074/non-product-start-v1".to_owned(),
            contract_sha256: "a".repeat(64),
            tooling_sha: "a".repeat(40),
            i74_source_sha: "a".repeat(40),
            c74_source_sha: "a".repeat(40),
            i74_tree_sha: "a".repeat(40),
            c74_tree_sha: "a".repeat(40),
            i74_cargo_lock_sha256: "a".repeat(64),
            c74_cargo_lock_sha256: "a".repeat(64),
            i74_dirty: false,
            c74_dirty: false,
            scenario_sha256: "a".repeat(64),
            workload_sha256: "a".repeat(64),
            offered_load_sha256: "a".repeat(64),
            estimator_sha256: "a".repeat(64),
            thresholds_sha256: "a".repeat(64),
            host_receipt_sha256: "a".repeat(64),
            lease_id: "00000000-0000-4000-8000-000000000074".to_owned(),
            machine_id: "machine".to_owned(),
            boot_id: "boot".to_owned(),
            mount_identity: "a".repeat(64),
            isolated_cpuset: "1-4".to_owned(),
            housekeeping_cpuset: "0,5-7".to_owned(),
            seed: 740074,
            checkpoint_cadence_seconds: 30,
            progress_warning_gap_seconds: 90,
            progress_rejection_gap_seconds: 180,
            diagnostic_grace_seconds: 30,
            product_lease_deadline_unix_seconds: 3_600,
            maximum_campaign_bytes: 21_474_836_480,
            maximum_campaign_files: 20_000,
            installed_binaries: vec![installed("i74"), installed("c74")],
            argv_templates: RoleArgvTemplates {
                i74: vec![
                    "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture".to_owned(),
                    "campaign-start-rehearsal-harness".to_owned(),
                ],
                c74: vec![
                    "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture".to_owned(),
                    "campaign-start-rehearsal-harness".to_owned(),
                ],
            },
            command_environment_sha256: "a".repeat(64),
            role_order: vec!["i74".to_owned(), "c74".to_owned()],
            phase_durations_seconds: PhaseDurationsSeconds {
                warmup: 145,
                measured: 145,
                drain: 145,
                durable_companion: 145,
                post_work_idle: 145,
                reconciliation: 145,
            },
            output_limits: OutputLimits {
                stdout_bytes: 1_048_576,
                stderr_bytes: 1_048_576,
                diagnostic_bytes: 1_048_576,
                final_artifact_bytes: 1_048_576,
                files: 100,
            },
            expected_output_schema_sha256s: ExpectedOutputSchemaSha256s {
                checkpoint: "a".repeat(64),
                measurement: "a".repeat(64),
                reconciliation: "a".repeat(64),
                raw_manifest: "a".repeat(64),
                packet_manifest: "a".repeat(64),
            },
            required_final_guards: vec![
                "non-product-protected-start-rehearsal-only".to_owned(),
                "product-candidate-started-false".to_owned(),
            ],
            secret_identifiers: Vec::new(),
            release: "0.74".to_owned(),
            campaign_id: campaign_id.to_owned(),
            nonce_sha256: "a".repeat(64),
            dirty: false,
            controller_history: Vec::new(),
            state: "PREPARED".to_owned(),
        }
    }

    fn prepare_terminal_fixture(
        role: Role,
    ) -> (
        tempfile::TempDir,
        PathBuf,
        PathBuf,
        CampaignManifest,
        String,
        ProcessIdentity,
        ProcessIdentity,
        String,
    ) {
        let temporary = tempfile::tempdir().unwrap();
        let campaign_id = "1".repeat(64);
        let campaign = temporary.path().join("campaigns").join(&campaign_id);
        let role_directory = campaign
            .join("roles")
            .join(start_rehearsal_role_name(&role));
        fs::create_dir_all(&role_directory).unwrap();
        fs::create_dir(temporary.path().join("seals")).unwrap();
        let manifest = terminal_manifest(&campaign_id);
        let manifest_bytes = canonical_json(&manifest).unwrap();
        let manifest_sha256 = sha256_hex(&manifest_bytes);
        fs::write(campaign.join("campaign-start.json"), manifest_bytes).unwrap();
        fs::write(
            campaign.join("campaign-start.sha256"),
            format!("{manifest_sha256}\n"),
        )
        .unwrap();
        fs::set_permissions(&campaign, fs::Permissions::from_mode(0o750)).unwrap();
        fs::set_permissions(
            campaign.join("campaign-start.sha256"),
            fs::Permissions::from_mode(0o440),
        )
        .unwrap();
        let harness = process(41);
        let daemon = process(42);
        let first = build_record(
            1,
            GENESIS_HASH,
            CheckpointPayload {
                campaign_id,
                role,
                phase: Phase::Startup,
                phase_epoch: 1,
                monotonic_elapsed_ns: 1,
                wall_clock_utc: format!("{:020}", 1),
                observed_unix_seconds: 1,
                useful_progress_unix_seconds: 1,
                completed: 0,
                failed: 0,
                rejected: 0,
                timed_out: 0,
                outstanding: 1,
                telemetry_sequence: 1,
                milestone: "fixture-start".to_owned(),
                surface_counters: BTreeMap::new(),
                resource_counters: BTreeMap::new(),
                owner_counters: BTreeMap::new(),
                harness: harness.clone(),
                daemon: daemon.clone(),
            },
        )
        .unwrap();
        append_record(
            &role_directory.join("checkpoints.jsonl"),
            &role_directory.join("checkpoints.head"),
            &first,
        )
        .unwrap();
        (
            temporary,
            campaign,
            role_directory,
            manifest,
            manifest_sha256,
            harness,
            daemon,
            first.record_sha256,
        )
    }

    #[test]
    fn derived_request_id_is_a_lowercase_v4_uuid() {
        let value = uuid_from_hash(HASH_A);
        assert_eq!(value.len(), 36);
        assert_eq!(&value[14..15], "4");
        assert_eq!(&value[19..20], "8");
    }

    #[test]
    fn fixture_cpuset_parser_is_exact_and_rejects_overlap() {
        assert_eq!(parse_cpuset("0,5-7").unwrap(), vec![0, 5, 6, 7]);
        assert_eq!(parse_cpuset("1-4").unwrap(), vec![1, 2, 3, 4]);
        for invalid in ["", "1,1", "2-1", "1-2,2", " 1", "1,"] {
            assert!(parse_cpuset(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn fixture_diagnostic_reads_only_the_bounded_role_stderr() {
        let root = tempfile::tempdir().unwrap();
        let role = root.path().join("roles").join("i74");
        fs::create_dir_all(&role).unwrap();
        fs::write(role.join("stderr.log"), b"fixture failed\n").unwrap();
        assert_eq!(fixture_diagnostic(root.path()), "fixture failed");

        fs::write(role.join("stderr.log"), vec![b'x'; 4097]).unwrap();
        assert_eq!(
            fixture_diagnostic(root.path()),
            "stderr is unsafe or oversized"
        );
    }

    #[test]
    fn lifecycle_resume_switches_only_after_the_frozen_progress_deadline() {
        assert!(!progress_deadline_elapsed(1_000, 999));
        assert!(!progress_deadline_elapsed(1_000, 1_000));
        assert!(progress_deadline_elapsed(1_000, 1_001));
    }

    #[test]
    fn lifecycle_resume_switches_only_after_the_frozen_lease_deadline() {
        assert!(!lease_deadline_elapsed(1_000, 999));
        assert!(!lease_deadline_elapsed(1_000, 1_000));
        assert!(lease_deadline_elapsed(1_000, 1_001));
    }

    #[test]
    fn measurement_fault_window_follows_the_progress_rehearsal_and_is_bounded() {
        const PROGRESS_REJECTION_GAP_SECONDS: u64 = 180;
        const ROLE_RUNTIME_SECONDS: u64 = 360;
        const PROGRESS_RESUME_SECONDS: u64 = PROGRESS_REJECTION_GAP_SECONDS + 2;
        const MEASUREMENT_RESUME_SECONDS: u64 = 212;
        const LEASE_RESUME_SECONDS: u64 = 252;
        let drift_end = FIXTURE_DAEMON_DRIFT_AFTER_SECONDS + FIXTURE_DAEMON_DRIFT_SECONDS;
        let daemon_end = drift_end + FIXTURE_DAEMON_AFTER_DRIFT_SECONDS;
        assert!(PROGRESS_RESUME_SECONDS < FIXTURE_DAEMON_DRIFT_AFTER_SECONDS);
        assert!(MEASUREMENT_RESUME_SECONDS >= FIXTURE_DAEMON_DRIFT_AFTER_SECONDS);
        assert!(MEASUREMENT_RESUME_SECONDS < drift_end);
        assert!(drift_end < FIXTURE_LEASE_SECONDS);
        assert!(LEASE_RESUME_SECONDS > FIXTURE_LEASE_SECONDS);
        assert!(LEASE_RESUME_SECONDS < daemon_end);
        assert!(daemon_end < ROLE_RUNTIME_SECONDS);
    }

    #[test]
    fn start_rehearsal_terminal_publication_is_resolver_ready_and_non_promotable() {
        let (_temporary, campaign, role, manifest, digest, harness, daemon, previous) =
            prepare_terminal_fixture(Role::I74);
        publish_start_rehearsal_terminal_evidence(
            &role,
            &manifest.campaign_id,
            &Role::I74,
            &harness,
            &daemon,
            1,
            &previous,
            Duration::from_secs(2),
            2,
        )
        .unwrap();

        let report = verify_journal(&role.join("checkpoints.jsonl")).unwrap();
        assert_eq!(report.records, 2);
        assert_eq!(report.last_phase, Phase::Terminal);
        let inventory: crate::seal_input::SealInputInventory = serde_json::from_slice(
            &fs::read(role.join(crate::seal_input::SEAL_INPUT_INVENTORY_NAME)).unwrap(),
        )
        .unwrap();
        assert_eq!(inventory.result, crate::artifact::PacketResult::Complete);
        assert_eq!(inventory.guard_evidence.len(), 2);
        assert!(inventory.guard_evidence.iter().all(|guard| guard.passed));
        assert!(campaign
            .join("roles/i74/non-product-seal-proof.json")
            .is_file());
        let seal_root = campaign.parent().unwrap().parent().unwrap().join("seals");
        let plan = crate::seal_input::resolve_packet_plan(
            &campaign,
            &seal_root,
            &manifest,
            &digest,
            Role::I74,
        )
        .unwrap();
        assert_eq!(plan.result, crate::artifact::PacketResult::Complete);
        assert!(!plan.promotable);
    }

    #[test]
    fn start_rehearsal_terminal_publication_preserves_c74_role_and_paths() {
        let (_temporary, campaign, role, manifest, _digest, harness, daemon, previous) =
            prepare_terminal_fixture(Role::C74);
        publish_start_rehearsal_terminal_evidence(
            &role,
            &manifest.campaign_id,
            &Role::C74,
            &harness,
            &daemon,
            1,
            &previous,
            Duration::from_secs(2),
            2,
        )
        .unwrap();

        let report = verify_journal(&role.join("checkpoints.jsonl")).unwrap();
        assert_eq!(report.records, 2);
        assert_eq!(report.last_phase, Phase::Terminal);
        let inventory: crate::seal_input::SealInputInventory = serde_json::from_slice(
            &fs::read(role.join(crate::seal_input::SEAL_INPUT_INVENTORY_NAME)).unwrap(),
        )
        .unwrap();
        assert_eq!(inventory.role, Role::C74);
        assert_eq!(
            inventory.journal_relative_path,
            PathBuf::from("roles/c74/checkpoints.jsonl")
        );
        assert!(inventory
            .raw_files
            .iter()
            .all(|path| !path.starts_with("roles/i74")));
        let proof: serde_json::Value = serde_json::from_slice(
            &fs::read(campaign.join("roles/c74/non-product-seal-proof.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(proof["role"], "c74");
    }

    #[test]
    fn start_rehearsal_role_is_exact_and_bounded_to_campaign_roles() {
        assert_eq!(start_rehearsal_role("i74").unwrap(), Role::I74);
        assert_eq!(start_rehearsal_role("c74").unwrap(), Role::C74);
        for invalid in ["", "I74", "c73", "i74 ", "c74/../i74"] {
            assert!(start_rehearsal_role(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn start_rehearsal_terminal_publication_rejects_unsafe_manifest_head_before_append() {
        let (_temporary, campaign, role, manifest, _digest, harness, daemon, previous) =
            prepare_terminal_fixture(Role::I74);
        fs::set_permissions(
            campaign.join("campaign-start.sha256"),
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        assert!(publish_start_rehearsal_terminal_evidence(
            &role,
            &manifest.campaign_id,
            &Role::I74,
            &harness,
            &daemon,
            1,
            &previous,
            Duration::from_secs(2),
            2,
        )
        .is_err());
        assert_eq!(
            verify_journal(&role.join("checkpoints.jsonl"))
                .unwrap()
                .records,
            1
        );
        assert!(!role
            .join(crate::seal_input::SEAL_INPUT_INVENTORY_NAME)
            .exists());
    }
}
