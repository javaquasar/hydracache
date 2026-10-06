use crate::manifest::CampaignManifest;
use crate::process_identity::{
    identity_from_snapshot, inspect_cgroup_processes, ProcessIdentityError, ProcessSnapshot,
};
use crate::spawn::{SpawnBackend, SpawnIntent, SpawnMismatch, SpawnObservation};
use crate::systemd_unit::{
    build_transient_unit_spec, inspect_unit_optional, start_transient_unit, TransientUnitSpec,
    UnitError, UnitSnapshot,
};
use crate::ProcessIdentity;
use std::ffi::CString;
use std::fmt::Display;
use std::fs::{self, File};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use thiserror::Error;

const SERVICE_ACCOUNT: &str = "hydracache-perf";
const OBSERVATION_ATTEMPTS: u32 = 300;
const OBSERVATION_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub struct UnitProcessObservation {
    pub unit: UnitSnapshot,
    pub processes: Vec<ProcessSnapshot>,
}

pub trait TransientUnitManager {
    type Error: Display;

    fn start(&mut self, spec: &TransientUnitSpec) -> Result<(), Self::Error>;
    fn observe(&mut self, unit_name: &str) -> Result<Option<UnitProcessObservation>, Self::Error>;
}

pub struct LiveTransientUnitManager;

impl TransientUnitManager for LiveTransientUnitManager {
    type Error = LiveManagerError;

    fn start(&mut self, spec: &TransientUnitSpec) -> Result<(), Self::Error> {
        start_transient_unit(spec)?;
        Ok(())
    }

    fn observe(&mut self, unit_name: &str) -> Result<Option<UnitProcessObservation>, Self::Error> {
        let Some(unit) = inspect_unit_optional(unit_name)? else {
            return Ok(None);
        };
        let processes = if unit.main_pid == 0 || unit.control_group.is_empty() {
            Vec::new()
        } else {
            match inspect_cgroup_processes(&unit.control_group) {
                Ok(processes) => processes,
                Err(ProcessIdentityError::Io(error))
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    Vec::new()
                }
                Err(error) => return Err(error.into()),
            }
        };
        Ok(Some(UnitProcessObservation { unit, processes }))
    }
}

#[derive(Debug, Error)]
pub enum LiveManagerError {
    #[error("systemd manager operation failed: {0}")]
    Unit(#[from] UnitError),
    #[error("systemd cgroup process inspection failed: {0}")]
    Process(#[from] ProcessIdentityError),
}

#[derive(Debug, Error)]
pub enum SystemdSpawnError {
    #[error("systemd spawn policy failed: {0}")]
    Unit(#[from] UnitError),
    #[error("systemd spawn process identity failed: {0}")]
    Process(#[from] ProcessIdentityError),
    #[error("systemd spawn manager failed: {0}")]
    Manager(String),
    #[error("systemd spawn directory ownership or layout is unsafe")]
    Directory,
    #[error("systemd spawn directory I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub struct SystemdSpawnBackend<M = LiveTransientUnitManager> {
    manifest: CampaignManifest,
    campaign_directory: PathBuf,
    manager: M,
    observation_attempts: u32,
    observation_interval: Duration,
    prepare_production_directories: bool,
}

impl SystemdSpawnBackend<LiveTransientUnitManager> {
    pub fn new(manifest: CampaignManifest, campaign_directory: PathBuf) -> Self {
        Self {
            manifest,
            campaign_directory,
            manager: LiveTransientUnitManager,
            observation_attempts: OBSERVATION_ATTEMPTS,
            observation_interval: OBSERVATION_INTERVAL,
            prepare_production_directories: true,
        }
    }
}

impl<M: TransientUnitManager> SystemdSpawnBackend<M> {
    pub fn with_manager(
        manifest: CampaignManifest,
        campaign_directory: PathBuf,
        manager: M,
    ) -> Self {
        Self {
            manifest,
            campaign_directory,
            manager,
            observation_attempts: 1,
            observation_interval: Duration::ZERO,
            prepare_production_directories: false,
        }
    }

    fn observe_exact(&mut self, unit_name: &str) -> Result<SpawnObservation, SystemdSpawnError> {
        let mut last = None;
        for attempt in 0..self.observation_attempts {
            let observed = self
                .manager
                .observe(unit_name)
                .map_err(|error| SystemdSpawnError::Manager(error.to_string()))?;
            match observed {
                None => last = None,
                Some(observation) => match classify_observation(
                    &self.manifest.boot_id,
                    &self.manifest.isolated_cpuset,
                    unit_name,
                    &observation.unit,
                    observation.processes,
                )? {
                    ObservationDisposition::Ready(observation) => return Ok(observation),
                    ObservationDisposition::Pending => last = Some(()),
                },
            }
            if attempt + 1 < self.observation_attempts && !self.observation_interval.is_zero() {
                thread::sleep(self.observation_interval);
            }
        }
        Ok(if last.is_some() {
            SpawnObservation::Mismatch {
                reason: SpawnMismatch::Identity,
                harness: None,
                daemon: None,
            }
        } else {
            SpawnObservation::Absent
        })
    }
}

impl<M: TransientUnitManager> SpawnBackend for SystemdSpawnBackend<M> {
    type Error = SystemdSpawnError;

    fn start_once(&mut self, intent: &SpawnIntent) -> Result<SpawnObservation, Self::Error> {
        if self.prepare_production_directories {
            prepare_role_directory(&self.campaign_directory, intent)?;
        }
        let spec = build_transient_unit_spec(&self.manifest, &self.campaign_directory, intent)?;
        self.manager
            .start(&spec)
            .map_err(|error| SystemdSpawnError::Manager(error.to_string()))?;
        self.observe_exact(&intent.unit_name)
    }

    fn observe(&mut self, unit_name: &str) -> Result<SpawnObservation, Self::Error> {
        self.observe_exact(unit_name)
    }
}

enum ObservationDisposition {
    Ready(SpawnObservation),
    Pending,
}

fn classify_observation(
    expected_boot_id: &str,
    expected_cpuset: &str,
    unit_name: &str,
    unit: &UnitSnapshot,
    processes: Vec<ProcessSnapshot>,
) -> Result<ObservationDisposition, SystemdSpawnError> {
    let expected_cgroup = format!("/system.slice/{unit_name}");
    if unit.unit_name != unit_name
        || unit.control_group != expected_cgroup
        || !matches!(unit.active_state.as_str(), "activating" | "active")
        || (unit.active_state == "active" && unit.sub_state != "running")
        || (unit.active_state == "active" && unit.result != "success")
    {
        return Ok(ObservationDisposition::Ready(SpawnObservation::Mismatch {
            reason: SpawnMismatch::Identity,
            harness: None,
            daemon: None,
        }));
    }
    if unit.main_pid == 0 || processes.len() < 2 {
        return Ok(ObservationDisposition::Pending);
    }
    if processes.len() > 2 {
        return Ok(ObservationDisposition::Ready(SpawnObservation::Mismatch {
            reason: SpawnMismatch::MultipleExecutors,
            harness: process_by_pid(&processes, unit.main_pid, unit_name).ok(),
            daemon: None,
        }));
    }
    let Some(harness_snapshot) = processes
        .iter()
        .find(|process| process.pid == unit.main_pid)
        .cloned()
    else {
        return Ok(ObservationDisposition::Ready(SpawnObservation::Mismatch {
            reason: SpawnMismatch::Identity,
            harness: None,
            daemon: None,
        }));
    };
    let Some(daemon_snapshot) = processes
        .into_iter()
        .find(|process| process.pid != unit.main_pid)
    else {
        return Ok(ObservationDisposition::Ready(SpawnObservation::Mismatch {
            reason: SpawnMismatch::Identity,
            harness: None,
            daemon: None,
        }));
    };
    if harness_snapshot.boot_id != expected_boot_id
        || daemon_snapshot.boot_id != expected_boot_id
        || harness_snapshot.cgroup_path != expected_cgroup
        || daemon_snapshot.cgroup_path != expected_cgroup
        || harness_snapshot.cgroup_inode != daemon_snapshot.cgroup_inode
        || harness_snapshot.process_group != daemon_snapshot.process_group
        || harness_snapshot.cpus_allowed_list != expected_cpuset
        || daemon_snapshot.cpus_allowed_list != expected_cpuset
    {
        return Ok(ObservationDisposition::Ready(SpawnObservation::Mismatch {
            reason: SpawnMismatch::Identity,
            harness: identity_from_snapshot(harness_snapshot, unit_name).ok(),
            daemon: identity_from_snapshot(daemon_snapshot, unit_name).ok(),
        }));
    }
    Ok(ObservationDisposition::Ready(SpawnObservation::Exact {
        harness: identity_from_snapshot(harness_snapshot, unit_name)?,
        daemon: identity_from_snapshot(daemon_snapshot, unit_name)?,
    }))
}

fn process_by_pid(
    processes: &[ProcessSnapshot],
    pid: u32,
    unit_name: &str,
) -> Result<ProcessIdentity, ProcessIdentityError> {
    let snapshot = processes
        .iter()
        .find(|process| process.pid == pid)
        .cloned()
        .ok_or(ProcessIdentityError::Document)?;
    identity_from_snapshot(snapshot, unit_name)
}

fn prepare_role_directory(
    campaign_directory: &Path,
    intent: &SpawnIntent,
) -> Result<(), SystemdSpawnError> {
    let campaign_directory = fs::canonicalize(campaign_directory)?;
    if campaign_directory
        .file_name()
        .and_then(|name| name.to_str())
        != Some(intent.campaign_id.as_str())
    {
        return Err(SystemdSpawnError::Directory);
    }
    let (uid, gid) = service_account_ids()?;
    set_owner_and_mode(&campaign_directory, 0, gid, 0o750)?;
    prepare_manifest_head(&campaign_directory, intent, gid)?;
    let roles = create_or_validate_directory(&campaign_directory, "roles", 0, gid)?;
    let role = match intent.role {
        crate::Role::I74 => "i74",
        crate::Role::C74 => "c74",
    };
    let role_directory = create_or_validate_directory(&roles, role, uid, gid)?;
    File::open(&role_directory)?.sync_all()?;
    File::open(&roles)?.sync_all()?;
    File::open(&campaign_directory)?.sync_all()?;
    Ok(())
}

fn prepare_manifest_head(
    campaign_directory: &Path,
    intent: &SpawnIntent,
    service_gid: libc::gid_t,
) -> Result<(), SystemdSpawnError> {
    use std::os::unix::fs::MetadataExt;

    let path = campaign_directory.join("campaign-start.sha256");
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.len() != 65
        || fs::read(&path)? != format!("{}\n", intent.manifest_sha256).as_bytes()
    {
        return Err(SystemdSpawnError::Directory);
    }
    set_owner_and_mode(&path, 0, service_gid, 0o440)?;
    File::open(campaign_directory)?.sync_all()?;
    Ok(())
}

fn create_or_validate_directory(
    parent: &Path,
    name: &str,
    uid: libc::uid_t,
    gid: libc::gid_t,
) -> Result<PathBuf, SystemdSpawnError> {
    let path = parent.join(name);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err(SystemdSpawnError::Directory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            builder.mode(0o750).create(&path)?;
        }
        Err(error) => return Err(error.into()),
    }
    let canonical = fs::canonicalize(&path)?;
    if canonical.parent() != Some(parent) {
        return Err(SystemdSpawnError::Directory);
    }
    set_owner_and_mode(&canonical, uid, gid, 0o750)?;
    Ok(canonical)
}

fn service_account_ids() -> Result<(libc::uid_t, libc::gid_t), SystemdSpawnError> {
    let name = CString::new(SERVICE_ACCOUNT).map_err(|_| SystemdSpawnError::Directory)?;
    // SAFETY: getpwnam receives a valid nul-terminated account name. The single-threaded
    // supervisor copies both scalar fields before making another account database call.
    let record = unsafe { libc::getpwnam(name.as_ptr()) };
    if record.is_null() {
        return Err(SystemdSpawnError::Directory);
    }
    // SAFETY: a non-null getpwnam result points to a passwd record valid until the next lookup.
    let record = unsafe { &*record };
    Ok((record.pw_uid, record.pw_gid))
}

fn set_owner_and_mode(
    path: &Path,
    uid: libc::uid_t,
    gid: libc::gid_t,
    mode: u32,
) -> Result<(), SystemdSpawnError> {
    let bytes =
        CString::new(path.as_os_str().as_bytes()).map_err(|_| SystemdSpawnError::Directory)?;
    // SAFETY: bytes is a valid nul-terminated path and uid/gid came from the account database.
    if unsafe { libc::chown(bytes.as_ptr(), uid, gid) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{classify_observation, ObservationDisposition};
    use crate::process_identity::ProcessSnapshot;
    use crate::spawn::{SpawnMismatch, SpawnObservation};
    use crate::systemd_unit::UnitSnapshot;

    const UNIT: &str = "hydracache-performance-074-i74-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.service";

    fn unit(main_pid: u32) -> UnitSnapshot {
        UnitSnapshot {
            unit_name: UNIT.to_owned(),
            active_state: "active".to_owned(),
            sub_state: "running".to_owned(),
            main_pid,
            control_group: format!("/system.slice/{UNIT}"),
            result: "success".to_owned(),
        }
    }

    fn process(pid: u32) -> ProcessSnapshot {
        ProcessSnapshot {
            boot_id: "boot-a".to_owned(),
            pid,
            start_ticks: u64::from(pid) * 100,
            process_group: 100,
            cgroup_path: format!("/system.slice/{UNIT}"),
            cgroup_inode: 500,
            cpus_allowed_list: "1-2".to_owned(),
        }
    }

    #[test]
    fn exact_original_process_pair_is_admitted() {
        let ObservationDisposition::Ready(SpawnObservation::Exact { harness, daemon }) =
            classify_observation(
                "boot-a",
                "1-2",
                UNIT,
                &unit(100),
                vec![process(100), process(101)],
            )
            .unwrap()
        else {
            panic!("expected exact process identities");
        };
        assert_eq!(harness.pid, 100);
        assert_eq!(daemon.pid, 101);
        assert_eq!(harness.cgroup_inode, daemon.cgroup_inode);
    }

    #[test]
    fn pending_observation_never_hides_a_terminal_unit_mismatch() {
        let mut foreign = unit(0);
        foreign.control_group = "/system.slice/foreign.service".to_owned();
        assert!(matches!(
            classify_observation("boot-a", "1-2", UNIT, &foreign, vec![]).unwrap(),
            ObservationDisposition::Ready(SpawnObservation::Mismatch {
                reason: SpawnMismatch::Identity,
                ..
            })
        ));
        assert!(matches!(
            classify_observation("boot-a", "1-2", UNIT, &unit(0), vec![]).unwrap(),
            ObservationDisposition::Pending
        ));
        assert!(matches!(
            classify_observation("boot-a", "1-2", UNIT, &unit(100), vec![process(100)]).unwrap(),
            ObservationDisposition::Pending
        ));
    }

    #[test]
    fn startup_is_pending_but_extra_executor_and_identity_drift_are_terminal() {
        assert!(matches!(
            classify_observation("boot-a", "1-2", UNIT, &unit(100), vec![process(100)]).unwrap(),
            ObservationDisposition::Pending
        ));
        assert!(matches!(
            classify_observation(
                "boot-a",
                "1-2",
                UNIT,
                &unit(100),
                vec![process(100), process(101), process(102)]
            )
            .unwrap(),
            ObservationDisposition::Ready(SpawnObservation::Mismatch {
                reason: SpawnMismatch::MultipleExecutors,
                ..
            })
        ));
        let mut drifted = process(101);
        drifted.cpus_allowed_list = "0-3".to_owned();
        assert!(matches!(
            classify_observation(
                "boot-a",
                "1-2",
                UNIT,
                &unit(100),
                vec![process(100), drifted]
            )
            .unwrap(),
            ObservationDisposition::Ready(SpawnObservation::Mismatch {
                reason: SpawnMismatch::Identity,
                ..
            })
        ));
    }
}
