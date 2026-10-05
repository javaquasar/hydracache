use crate::systemd_unit::{
    inspect_unit_optional, start_transient_unit, stop_unit_and_wait, ExecCommand,
    TransientUnitSpec, UnitError, UnitProperty, UnitSnapshot,
};
use serde::Serialize;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const UNIT_NAME: &str = "hydracache-performance-074-systemd-smoke.service";
const OBSERVATION_TIMEOUT: Duration = Duration::from_secs(5);

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

#[derive(Debug, Error)]
pub enum SystemdSmokeError {
    #[error("systemd smoke unit operation failed: {0}")]
    Unit(#[from] UnitError),
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

#[cfg(test)]
mod tests {
    use super::smoke_spec;
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
}
