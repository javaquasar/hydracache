use crate::systemd_unit::{
    inspect_unit_optional, start_transient_unit, stop_unit_and_wait, ExecCommand,
    TransientUnitSpec, UnitError, UnitProperty, UnitSnapshot,
};
use crate::ProcessIdentity;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const UNIT_NAME: &str = "hydracache-performance-074-systemd-smoke.service";
const CONTROLLER_LOSS_UNIT_NAME: &str = "hydracache-performance-074-controller-loss-smoke.service";
const CONTROLLER_LOSS_CONTEXT_PATH: &str = "/run/hydracache-perf/controller-loss-smoke-v1.json";
const OBSERVATION_TIMEOUT: Duration = Duration::from_secs(5);
const CONTROLLER_LOSS_FIXTURE_SECONDS: u64 = 30;
const MAX_CONTEXT_BYTES: u64 = 16 * 1024;

#[derive(Debug, Serialize)]
pub struct SystemdSmokeReceipt {
    pub schema_version: &'static str,
    pub unit_name: &'static str,
    pub fixture_command: &'static str,
    pub product_candidate_started: bool,
    pub saw_running: bool,
    pub running_main_pid: u32,
    pub running_control_group: String,
    pub terminal: UnitSnapshot,
    pub after_stop: Option<UnitSnapshot>,
    pub elapsed_milliseconds: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ControllerLossContext {
    schema_version: u32,
    unit_name: String,
    fixture_command: String,
    product_candidate_started: bool,
    controller_pid: u32,
    controller_start_ticks: u64,
    controller_boot_id: String,
    fixture: ProcessIdentity,
}

#[derive(Debug, Serialize)]
pub struct ControllerLossStartReceipt {
    pub schema_version: &'static str,
    pub unit_name: &'static str,
    pub fixture_command: &'static str,
    pub product_candidate_started: bool,
    pub controller_pid: u32,
    pub controller_start_ticks: u64,
    pub fixture: ProcessIdentity,
}

#[derive(Debug, Serialize)]
pub struct ControllerLossResumeReceipt {
    pub schema_version: &'static str,
    pub unit_name: &'static str,
    pub fixture_command: &'static str,
    pub product_candidate_started: bool,
    pub original_controller_exited: bool,
    pub fixture_identity_unchanged: bool,
    pub observed_before_stop: UnitSnapshot,
    pub after_stop: Option<UnitSnapshot>,
}

#[derive(Debug, Error)]
pub enum SystemdSmokeError {
    #[error("systemd smoke unit operation failed: {0}")]
    Unit(#[from] UnitError),
    #[error("systemd smoke process identity failed: {0}")]
    Process(#[from] crate::process_identity::ProcessIdentityError),
    #[error("systemd smoke context I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("systemd smoke context JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("systemd smoke precondition, transition, or cleanup failed")]
    State,
}

pub fn run_systemd_smoke() -> Result<SystemdSmokeReceipt, SystemdSmokeError> {
    if inspect_unit_optional(UNIT_NAME)?.is_some() {
        return Err(SystemdSmokeError::State);
    }
    let started = Instant::now();
    start_transient_unit(&smoke_spec())?;
    let observed = observe_running_and_terminal();
    let cleanup = stop_unit_and_wait(UNIT_NAME, 5);
    let (running, terminal) = observed?;
    cleanup?;
    let after_stop = inspect_unit_optional(UNIT_NAME)?;
    if let Some(snapshot) = &after_stop {
        if snapshot.active_state != "inactive"
            || snapshot.sub_state != "dead"
            || snapshot.main_pid != 0
        {
            return Err(SystemdSmokeError::State);
        }
    }
    Ok(SystemdSmokeReceipt {
        schema_version: "hydracache-w11-systemd-smoke-v1",
        unit_name: UNIT_NAME,
        fixture_command: "/usr/bin/sleep 1",
        product_candidate_started: false,
        saw_running: true,
        running_main_pid: running.main_pid,
        running_control_group: running.control_group,
        terminal,
        after_stop,
        elapsed_milliseconds: started.elapsed().as_millis(),
    })
}

pub fn start_controller_loss_smoke() -> Result<ControllerLossStartReceipt, SystemdSmokeError> {
    let context_path = Path::new(CONTROLLER_LOSS_CONTEXT_PATH);
    match fs::symlink_metadata(context_path) {
        Ok(_) => return Err(SystemdSmokeError::State),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    if inspect_unit_optional(CONTROLLER_LOSS_UNIT_NAME)?.is_some() {
        return Err(SystemdSmokeError::State);
    }
    let controller = crate::process_identity::inspect_process(std::process::id())?;
    start_transient_unit(&controller_loss_spec())?;
    let fixture = match observe_controller_loss_running() {
        Ok(fixture) => fixture,
        Err(error) => {
            let _ = stop_unit_and_wait(CONTROLLER_LOSS_UNIT_NAME, 5);
            return Err(error);
        }
    };
    let context = ControllerLossContext {
        schema_version: 1,
        unit_name: CONTROLLER_LOSS_UNIT_NAME.to_owned(),
        fixture_command: format!("/usr/bin/sleep {CONTROLLER_LOSS_FIXTURE_SECONDS}"),
        product_candidate_started: false,
        controller_pid: controller.pid,
        controller_start_ticks: controller.start_ticks,
        controller_boot_id: controller.boot_id,
        fixture: fixture.clone(),
    };
    if let Err(error) = write_context(context_path, &context) {
        let _ = stop_unit_and_wait(CONTROLLER_LOSS_UNIT_NAME, 5);
        return Err(error);
    }
    Ok(ControllerLossStartReceipt {
        schema_version: "hydracache-w11-controller-loss-start-v1",
        unit_name: CONTROLLER_LOSS_UNIT_NAME,
        fixture_command: "/usr/bin/sleep 30",
        product_candidate_started: false,
        controller_pid: context.controller_pid,
        controller_start_ticks: context.controller_start_ticks,
        fixture,
    })
}

pub fn resume_controller_loss_smoke() -> Result<ControllerLossResumeReceipt, SystemdSmokeError> {
    let context_path = Path::new(CONTROLLER_LOSS_CONTEXT_PATH);
    let context = read_context(context_path)?;
    if context.schema_version != 1
        || context.unit_name != CONTROLLER_LOSS_UNIT_NAME
        || context.fixture_command != "/usr/bin/sleep 30"
        || context.product_candidate_started
    {
        return Err(SystemdSmokeError::State);
    }
    if original_controller_still_running(&context)? {
        return Err(SystemdSmokeError::State);
    }
    let snapshot = inspect_unit_optional(CONTROLLER_LOSS_UNIT_NAME)?
        .filter(|snapshot| {
            snapshot.active_state == "active"
                && snapshot.sub_state == "running"
                && snapshot.main_pid == context.fixture.pid
                && snapshot.control_group == context.fixture.cgroup_path
                && snapshot.result == "success"
        })
        .ok_or(SystemdSmokeError::State)?;
    crate::process_identity::verify_process_identity(&context.fixture)?;
    stop_unit_and_wait(CONTROLLER_LOSS_UNIT_NAME, 5)?;
    let after_stop = inspect_unit_optional(CONTROLLER_LOSS_UNIT_NAME)?;
    if after_stop.as_ref().is_some_and(|snapshot| {
        snapshot.active_state != "inactive"
            || snapshot.sub_state != "dead"
            || snapshot.main_pid != 0
    }) {
        return Err(SystemdSmokeError::State);
    }
    remove_context(context_path)?;
    Ok(ControllerLossResumeReceipt {
        schema_version: "hydracache-w11-controller-loss-resume-v1",
        unit_name: CONTROLLER_LOSS_UNIT_NAME,
        fixture_command: "/usr/bin/sleep 30",
        product_candidate_started: false,
        original_controller_exited: true,
        fixture_identity_unchanged: true,
        observed_before_stop: snapshot,
        after_stop,
    })
}

fn observe_controller_loss_running() -> Result<ProcessIdentity, SystemdSmokeError> {
    let deadline = Instant::now() + OBSERVATION_TIMEOUT;
    loop {
        if let Some(snapshot) = inspect_unit_optional(CONTROLLER_LOSS_UNIT_NAME)? {
            if snapshot.active_state == "active"
                && snapshot.sub_state == "running"
                && snapshot.main_pid > 0
                && snapshot.control_group == format!("/system.slice/{CONTROLLER_LOSS_UNIT_NAME}")
                && snapshot.result == "success"
            {
                let process = crate::process_identity::inspect_process(snapshot.main_pid)?;
                return Ok(crate::process_identity::identity_from_snapshot(
                    process,
                    CONTROLLER_LOSS_UNIT_NAME,
                )?);
            }
        }
        if Instant::now() >= deadline {
            return Err(SystemdSmokeError::State);
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn original_controller_still_running(
    context: &ControllerLossContext,
) -> Result<bool, SystemdSmokeError> {
    match crate::process_identity::inspect_process(context.controller_pid) {
        Ok(process) => Ok(process.start_ticks == context.controller_start_ticks
            && process.boot_id == context.controller_boot_id),
        Err(crate::process_identity::ProcessIdentityError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(false)
        }
        Err(error) => Err(error.into()),
    }
}

fn write_context(path: &Path, context: &ControllerLossContext) -> Result<(), SystemdSmokeError> {
    let bytes = serde_json::to_vec(context)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_CONTEXT_BYTES {
        return Err(SystemdSmokeError::State);
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    File::open(path.parent().ok_or(SystemdSmokeError::State)?)?.sync_all()?;
    Ok(())
}

fn read_context(path: &Path) -> Result<ControllerLossContext, SystemdSmokeError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != 0
        || metadata.gid() != 0
        || metadata.mode() & 0o7777 != 0o600
        || metadata.len() == 0
        || metadata.len() > MAX_CONTEXT_BYTES
    {
        return Err(SystemdSmokeError::State);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(MAX_CONTEXT_BYTES + 1)
        .read_to_end(&mut bytes)?;
    let context: ControllerLossContext = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&context)? != bytes {
        return Err(SystemdSmokeError::State);
    }
    Ok(context)
}

fn remove_context(path: &Path) -> Result<(), SystemdSmokeError> {
    let parent = path.parent().ok_or(SystemdSmokeError::State)?;
    fs::remove_file(path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn observe_running_and_terminal() -> Result<(UnitSnapshot, UnitSnapshot), SystemdSmokeError> {
    let deadline = Instant::now() + OBSERVATION_TIMEOUT;
    let mut running = None;
    loop {
        let snapshot = inspect_unit_optional(UNIT_NAME)?.ok_or(SystemdSmokeError::State)?;
        if snapshot.active_state == "active"
            && snapshot.sub_state == "running"
            && snapshot.main_pid > 0
            && snapshot.control_group == format!("/system.slice/{UNIT_NAME}")
        {
            running = Some(snapshot.clone());
        }
        if snapshot.active_state == "active"
            && snapshot.sub_state == "exited"
            && snapshot.main_pid == 0
            && snapshot.result == "success"
        {
            return Ok((running.ok_or(SystemdSmokeError::State)?, snapshot));
        }
        if Instant::now() >= deadline {
            return Err(SystemdSmokeError::State);
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn smoke_spec() -> TransientUnitSpec {
    TransientUnitSpec {
        unit_name: UNIT_NAME.to_owned(),
        properties: vec![
            (
                "Description",
                UnitProperty::Text("HydraCache 0.74 bounded systemd smoke".to_owned()),
            ),
            ("User", UnitProperty::Text("hydracache-perf".to_owned())),
            ("Group", UnitProperty::Text("hydracache-perf".to_owned())),
            ("Type", UnitProperty::Text("exec".to_owned())),
            ("RemainAfterExit", UnitProperty::Boolean(true)),
            ("Restart", UnitProperty::Text("no".to_owned())),
            ("KillMode", UnitProperty::Text("control-group".to_owned())),
            ("Slice", UnitProperty::Text("system.slice".to_owned())),
            ("RuntimeMaxUSec", UnitProperty::Unsigned(5_000_000)),
            ("TimeoutStopUSec", UnitProperty::Unsigned(2_000_000)),
            ("MemoryMax", UnitProperty::Unsigned(67_108_864)),
            ("LimitNOFILE", UnitProperty::Unsigned(1_024)),
            ("TasksMax", UnitProperty::Unsigned(16)),
            ("NoNewPrivileges", UnitProperty::Boolean(true)),
            ("PrivateTmp", UnitProperty::Boolean(true)),
            ("PrivateDevices", UnitProperty::Boolean(true)),
            ("ProtectSystem", UnitProperty::Text("strict".to_owned())),
            ("ProtectHome", UnitProperty::Text("yes".to_owned())),
            ("ProtectKernelTunables", UnitProperty::Boolean(true)),
            ("ProtectKernelModules", UnitProperty::Boolean(true)),
            ("ProtectControlGroups", UnitProperty::Boolean(true)),
            ("RestrictSUIDSGID", UnitProperty::Boolean(true)),
            ("LockPersonality", UnitProperty::Boolean(true)),
            (
                "Environment",
                UnitProperty::Strings(vec![
                    "LANG=C.UTF-8".to_owned(),
                    "LC_ALL=C.UTF-8".to_owned(),
                    "PATH=/usr/bin:/bin".to_owned(),
                    "TZ=UTC".to_owned(),
                ]),
            ),
            (
                "ExecStart",
                UnitProperty::Commands(vec![ExecCommand {
                    path: "/usr/bin/sleep".to_owned(),
                    argv: vec!["/usr/bin/sleep".to_owned(), "1".to_owned()],
                    ignore_failure: false,
                }]),
            ),
        ],
    }
}

fn controller_loss_spec() -> TransientUnitSpec {
    TransientUnitSpec {
        unit_name: CONTROLLER_LOSS_UNIT_NAME.to_owned(),
        properties: vec![
            (
                "Description",
                UnitProperty::Text("HydraCache 0.74 controller-loss smoke".to_owned()),
            ),
            ("User", UnitProperty::Text("hydracache-perf".to_owned())),
            ("Group", UnitProperty::Text("hydracache-perf".to_owned())),
            ("Type", UnitProperty::Text("exec".to_owned())),
            ("RemainAfterExit", UnitProperty::Boolean(false)),
            ("Restart", UnitProperty::Text("no".to_owned())),
            ("KillMode", UnitProperty::Text("control-group".to_owned())),
            ("Slice", UnitProperty::Text("system.slice".to_owned())),
            ("RuntimeMaxUSec", UnitProperty::Unsigned(35_000_000)),
            ("TimeoutStopUSec", UnitProperty::Unsigned(2_000_000)),
            ("MemoryMax", UnitProperty::Unsigned(67_108_864)),
            ("LimitNOFILE", UnitProperty::Unsigned(1_024)),
            ("TasksMax", UnitProperty::Unsigned(4)),
            ("NoNewPrivileges", UnitProperty::Boolean(true)),
            ("PrivateTmp", UnitProperty::Boolean(true)),
            ("PrivateDevices", UnitProperty::Boolean(true)),
            ("ProtectSystem", UnitProperty::Text("strict".to_owned())),
            ("ProtectHome", UnitProperty::Text("yes".to_owned())),
            ("ProtectKernelTunables", UnitProperty::Boolean(true)),
            ("ProtectKernelModules", UnitProperty::Boolean(true)),
            ("ProtectControlGroups", UnitProperty::Boolean(true)),
            ("RestrictSUIDSGID", UnitProperty::Boolean(true)),
            ("LockPersonality", UnitProperty::Boolean(true)),
            (
                "Environment",
                UnitProperty::Strings(vec![
                    "LANG=C.UTF-8".to_owned(),
                    "LC_ALL=C.UTF-8".to_owned(),
                    "PATH=/usr/bin:/bin".to_owned(),
                    "TZ=UTC".to_owned(),
                ]),
            ),
            (
                "ExecStart",
                UnitProperty::Commands(vec![ExecCommand {
                    path: "/usr/bin/sleep".to_owned(),
                    argv: vec![
                        "/usr/bin/sleep".to_owned(),
                        CONTROLLER_LOSS_FIXTURE_SECONDS.to_string(),
                    ],
                    ignore_failure: false,
                }]),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{controller_loss_spec, smoke_spec, CONTROLLER_LOSS_UNIT_NAME};
    use crate::systemd_unit::UnitProperty;

    #[test]
    fn smoke_spec_is_bounded_hardened_and_has_no_product_command() {
        let spec = smoke_spec();
        assert_eq!(
            spec.unit_name,
            "hydracache-performance-074-systemd-smoke.service"
        );
        let encoded = format!("{spec:?}");
        for required in [
            "hydracache-perf",
            "NoNewPrivileges",
            "ProtectSystem",
            "PrivateDevices",
            "/usr/bin/sleep",
        ] {
            assert!(encoded.contains(required));
        }
        assert!(!encoded.contains("/opt/hydracache-performance/0.74"));
        assert!(spec
            .properties
            .iter()
            .any(|(name, value)| { *name == "TasksMax" && *value == UnitProperty::Unsigned(16) }));
        assert!(spec.properties.iter().any(|(name, value)| {
            *name == "ProtectHome" && *value == UnitProperty::Text("yes".to_owned())
        }));
    }

    #[test]
    fn controller_loss_spec_is_long_enough_to_outlive_its_caller_and_stays_non_product() {
        let spec = controller_loss_spec();
        assert_eq!(spec.unit_name, CONTROLLER_LOSS_UNIT_NAME);
        let encoded = format!("{spec:?}");
        for required in [
            "hydracache-perf",
            "NoNewPrivileges",
            "ProtectSystem",
            "/usr/bin/sleep",
            "30",
        ] {
            assert!(encoded.contains(required));
        }
        assert!(!encoded.contains("/opt/hydracache-performance/0.74"));
        assert!(spec.properties.iter().any(|(name, value)| {
            *name == "RuntimeMaxUSec" && *value == UnitProperty::Unsigned(35_000_000)
        }));
        assert!(spec.properties.iter().any(|(name, value)| {
            *name == "RemainAfterExit" && *value == UnitProperty::Boolean(false)
        }));
    }
}
