use crate::abort_backend::SystemdAbortBackend;
use crate::abort_lifecycle::drive_abort_request;
use crate::checkpoint_evidence::observe_live_checkpoint_evidence;
use crate::event::request_sha256;
use crate::host_execution::HostExecutionClaim;
use crate::host_receipt::{collect_fixture_host_observation, write_receipt_for_admission};
use crate::manifest::{
    frozen_identity_from_manifest, CampaignManifest, ExpectedOutputSchemaSha256s, InstalledBinary,
    OutputLimits, PhaseDurationsSeconds, RoleArgvTemplates,
};
use crate::mutation::{begin_attach, reconcile_campaign, BeginAttach};
use crate::process_identity::{identity_from_snapshot, inspect_process};
use crate::protocol::{ControllerIdentity, Operation, Request};
use crate::spawn::{SpawnBackend, SpawnObservation};
use crate::start_lifecycle::drive_i74_start_request;
use crate::state::{apply_attach, AttachRequest};
use crate::state_store::CampaignLock;
use crate::systemd_spawn::SystemdSpawnBackend;
use crate::systemd_unit::expected_command_environment_sha256;
use crate::{
    append_record, build_record, canonical_json, sha256_hex, CheckpointPayload, Phase, Role,
    GENESIS_HASH,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CAMPAIGN_ROOT: &str = "/var/lib/hydracache-performance/campaigns";
const FIXTURE_BINARY: &str = "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture";
const CONTEXT_PATH: &str = "/run/hydracache-perf/campaign-lifecycle-smoke-v1.json";
const FIXTURE_SECONDS: u64 = 75;
const MAX_CONTEXT_BYTES: u64 = 64 * 1024;
const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const GIT_SHA: &str = "0000000000000000000000000000000000000000";

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

pub fn start_campaign_lifecycle_smoke() -> Result<CampaignSmokeStartReceipt, String> {
    require_root()?;
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
    let (_, checkpoint) = wait_for_checkpoint(&campaign_directory, &state)?;
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
    })
}

pub fn resume_campaign_lifecycle_smoke() -> Result<CampaignSmokeResumeReceipt, String> {
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

pub fn run_fixture_harness() -> Result<(), String> {
    let campaign_id = std::env::var("HYDRACACHE_CAMPAIGN_ID").map_err(display)?;
    let role = std::env::var("HYDRACACHE_ROLE").map_err(display)?;
    let evidence_directory =
        PathBuf::from(std::env::var("HYDRACACHE_EVIDENCE_DIRECTORY").map_err(display)?);
    if role != "i74" || campaign_id.len() != 64 {
        return Err("fixture environment differs".to_owned());
    }
    let executable = std::env::current_exe().map_err(display)?;
    if executable != Path::new(FIXTURE_BINARY) {
        return Err("fixture executable path differs".to_owned());
    }
    let mut child = Command::new(&executable)
        .arg("campaign-fixture-daemon")
        .spawn()
        .map_err(display)?;
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
    append_record(
        &evidence_directory.join("checkpoints.jsonl"),
        &evidence_directory.join("checkpoints.head"),
        &record,
    )
    .map_err(display)?;
    let status = child.wait().map_err(display)?;
    if status.success() {
        Ok(())
    } else {
        Err("fixture daemon failed".to_owned())
    }
}

pub fn run_fixture_daemon() {
    thread::sleep(Duration::from_secs(FIXTURE_SECONDS));
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
        product_lease_deadline_unix_seconds: now.saturating_add(300),
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
            warmup: 10,
            measured: 10,
            drain: 10,
            durable_companion: 10,
            post_work_idle: 10,
            reconciliation: 10,
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
    use super::{uuid_from_hash, HASH_A};

    #[test]
    fn derived_request_id_is_a_lowercase_v4_uuid() {
        let value = uuid_from_hash(HASH_A);
        assert_eq!(value.len(), 36);
        assert_eq!(&value[14..15], "4");
        assert_eq!(&value[19..20], "8");
    }
}
