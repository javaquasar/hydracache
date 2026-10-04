use crate::ProcessIdentity;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

const SYSTEMD_DESTINATION: &str = "org.freedesktop.systemd1";
const SYSTEMD_MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const SYSTEMD_MANAGER_INTERFACE: &str = "org.freedesktop.systemd1.Manager";
const SYSTEMD_UNIT_INTERFACE: &str = "org.freedesktop.systemd1.Unit";
const SYSTEMD_SERVICE_INTERFACE: &str = "org.freedesktop.systemd1.Service";
const UNIT_PREFIX: &str = "hydracache-performance-074-";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitSnapshot {
    pub unit_name: String,
    pub active_state: String,
    pub sub_state: String,
    pub main_pid: u32,
    pub control_group: String,
    pub result: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitMismatch {
    UnitName,
    ActiveState,
    SubState,
    MainPid,
    ControlGroup,
    Result,
}

#[derive(Debug, Error)]
pub enum UnitError {
    #[error("systemd unit name is outside the fixed HydraCache 0.74 namespace")]
    UnitName,
    #[error("systemd D-Bus inspection failed: {0}")]
    Dbus(#[from] zbus::Error),
    #[error("systemd unit identity differs: {0:?}")]
    Mismatch(Vec<UnitMismatch>),
}

pub fn inspect_unit(unit_name: &str) -> Result<UnitSnapshot, UnitError> {
    validate_unit_name(unit_name)?;
    inspect_loaded_unit(unit_name)
}

fn inspect_loaded_unit(unit_name: &str) -> Result<UnitSnapshot, UnitError> {
    let connection = Connection::system()?;
    let manager = Proxy::new(
        &connection,
        SYSTEMD_DESTINATION,
        SYSTEMD_MANAGER_PATH,
        SYSTEMD_MANAGER_INTERFACE,
    )?;
    let path: OwnedObjectPath = manager.call("GetUnit", &(unit_name,))?;
    let unit = Proxy::new(
        &connection,
        SYSTEMD_DESTINATION,
        path.as_str(),
        SYSTEMD_UNIT_INTERFACE,
    )?;
    let service = Proxy::new(
        &connection,
        SYSTEMD_DESTINATION,
        path.as_str(),
        SYSTEMD_SERVICE_INTERFACE,
    )?;
    Ok(UnitSnapshot {
        unit_name: unit_name.to_owned(),
        active_state: unit.get_property("ActiveState")?,
        sub_state: unit.get_property("SubState")?,
        main_pid: service.get_property("MainPID")?,
        control_group: service.get_property("ControlGroup")?,
        result: service.get_property("Result")?,
    })
}

#[cfg(test)]
mod tests {
    use super::inspect_loaded_unit;

    #[test]
    #[ignore = "requires a local systemd system bus"]
    fn reads_a_real_loaded_service_over_dbus() {
        let snapshot = inspect_loaded_unit("dbus.service").unwrap();
        assert_eq!(snapshot.unit_name, "dbus.service");
        assert_eq!(snapshot.active_state, "active");
        assert_eq!(snapshot.sub_state, "running");
        assert!(snapshot.main_pid > 0);
        assert_eq!(snapshot.control_group, "/system.slice/dbus.service");
        assert_eq!(snapshot.result, "success");
    }
}

pub fn verify_unit_identity(
    harness: &ProcessIdentity,
    daemon: &ProcessIdentity,
    snapshot: &UnitSnapshot,
) -> Result<(), UnitError> {
    let mut mismatches = Vec::new();
    if validate_unit_name(&snapshot.unit_name).is_err()
        || harness.unit_name != snapshot.unit_name
        || daemon.unit_name != snapshot.unit_name
    {
        mismatches.push(UnitMismatch::UnitName);
    }
    if snapshot.active_state != "active" {
        mismatches.push(UnitMismatch::ActiveState);
    }
    if snapshot.sub_state != "running" {
        mismatches.push(UnitMismatch::SubState);
    }
    if snapshot.main_pid != harness.pid {
        mismatches.push(UnitMismatch::MainPid);
    }
    if snapshot.control_group != harness.cgroup_path || snapshot.control_group != daemon.cgroup_path
    {
        mismatches.push(UnitMismatch::ControlGroup);
    }
    if snapshot.result != "success" {
        mismatches.push(UnitMismatch::Result);
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(UnitError::Mismatch(mismatches))
    }
}

fn validate_unit_name(value: &str) -> Result<(), UnitError> {
    if value.starts_with(UNIT_PREFIX)
        && value.ends_with(".service")
        && value.len() > UNIT_PREFIX.len() + ".service".len()
        && value.len() <= 255
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'@'))
    {
        Ok(())
    } else {
        Err(UnitError::UnitName)
    }
}
