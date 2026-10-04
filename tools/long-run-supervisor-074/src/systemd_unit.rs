use crate::manifest::CampaignManifest;
use crate::spawn::SpawnIntent;
use crate::{canonical_json, sha256_hex, ProcessIdentity, Role};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path};
use thiserror::Error;
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, Value};

const SYSTEMD_DESTINATION: &str = "org.freedesktop.systemd1";
const SYSTEMD_MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const SYSTEMD_MANAGER_INTERFACE: &str = "org.freedesktop.systemd1.Manager";
const SYSTEMD_UNIT_INTERFACE: &str = "org.freedesktop.systemd1.Unit";
const SYSTEMD_SERVICE_INTERFACE: &str = "org.freedesktop.systemd1.Service";
const UNIT_PREFIX: &str = "hydracache-performance-074-";
const SERVICE_USER: &str = "hydracache-perf";
const SERVICE_GROUP: &str = "hydracache-perf";
const SERVICE_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
pub const UNIT_MEMORY_MAX_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub const UNIT_NOFILE_LIMIT: u64 = 65_536;
pub const UNIT_TASKS_MAX: u64 = 4_096;
pub const MAXIMUM_ROLE_RUNTIME_SECONDS: u64 = 108_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecCommand {
    pub path: String,
    pub argv: Vec<String>,
    pub ignore_failure: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitProperty {
    Text(String),
    Boolean(bool),
    Unsigned(u64),
    Bytes(Vec<u8>),
    Strings(Vec<String>),
    Commands(Vec<ExecCommand>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientUnitSpec {
    pub unit_name: String,
    pub properties: Vec<(&'static str, UnitProperty)>,
}

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
    #[error("transient systemd unit policy or campaign path is invalid")]
    Policy,
    #[error("transient systemd unit policy serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("systemd unit identity differs: {0:?}")]
    Mismatch(Vec<UnitMismatch>),
}

pub fn build_transient_unit_spec(
    manifest: &CampaignManifest,
    campaign_directory: &Path,
    intent: &SpawnIntent,
) -> Result<TransientUnitSpec, UnitError> {
    validate_unit_name(&intent.unit_name)?;
    if manifest.campaign_id != intent.campaign_id
        || manifest.nonce_sha256 != intent.nonce_sha256
        || manifest.role_order != ["i74", "c74"]
        || campaign_directory
            .file_name()
            .and_then(|name| name.to_str())
            != Some(manifest.campaign_id.as_str())
        || !safe_absolute_path(campaign_directory)
    {
        return Err(UnitError::Policy);
    }
    let role_name = match intent.role {
        Role::I74 => "i74",
        Role::C74 => "c74",
    };
    let argv = match intent.role {
        Role::I74 => &manifest.argv_templates.i74,
        Role::C74 => &manifest.argv_templates.c74,
    };
    let binary = manifest
        .installed_binaries
        .iter()
        .find(|binary| binary.role == role_name)
        .ok_or(UnitError::Policy)?;
    if argv.first() != Some(&binary.path) || !safe_absolute_text_path(&binary.path) {
        return Err(UnitError::Policy);
    }
    let runtime_seconds = role_runtime_seconds(manifest)?;
    let role_directory = campaign_directory.join("roles").join(role_name);
    let environment = role_environment(&manifest.campaign_id, role_name, &role_directory)?;
    if expected_command_environment_sha256(manifest, campaign_directory)?
        != manifest.command_environment_sha256
    {
        return Err(UnitError::Policy);
    }
    let properties = vec![
        (
            "Description",
            UnitProperty::Text(format!(
                "HydraCache 0.74 {} role {}",
                role_name, manifest.campaign_id
            )),
        ),
        ("User", UnitProperty::Text(SERVICE_USER.to_owned())),
        ("Group", UnitProperty::Text(SERVICE_GROUP.to_owned())),
        ("Type", UnitProperty::Text("exec".to_owned())),
        ("Restart", UnitProperty::Text("no".to_owned())),
        ("KillMode", UnitProperty::Text("control-group".to_owned())),
        ("Delegate", UnitProperty::Boolean(false)),
        ("Slice", UnitProperty::Text("system.slice".to_owned())),
        (
            "CPUAffinity",
            UnitProperty::Bytes(cpuset_mask(&manifest.isolated_cpuset)?),
        ),
        (
            "WorkingDirectory",
            UnitProperty::Text(path_text(&role_directory)?),
        ),
        (
            "RuntimeMaxUSec",
            UnitProperty::Unsigned(
                runtime_seconds
                    .checked_mul(1_000_000)
                    .ok_or(UnitError::Policy)?,
            ),
        ),
        (
            "TimeoutStopUSec",
            UnitProperty::Unsigned(
                manifest
                    .diagnostic_grace_seconds
                    .checked_mul(1_000_000)
                    .ok_or(UnitError::Policy)?,
            ),
        ),
        ("MemoryMax", UnitProperty::Unsigned(UNIT_MEMORY_MAX_BYTES)),
        ("LimitNOFILE", UnitProperty::Unsigned(UNIT_NOFILE_LIMIT)),
        ("TasksMax", UnitProperty::Unsigned(UNIT_TASKS_MAX)),
        ("NoNewPrivileges", UnitProperty::Boolean(true)),
        ("PrivateTmp", UnitProperty::Boolean(true)),
        ("ProtectHome", UnitProperty::Boolean(true)),
        ("ProtectKernelTunables", UnitProperty::Boolean(true)),
        ("ProtectKernelModules", UnitProperty::Boolean(true)),
        ("ProtectControlGroups", UnitProperty::Boolean(true)),
        ("RestrictSUIDSGID", UnitProperty::Boolean(true)),
        ("LockPersonality", UnitProperty::Boolean(true)),
        ("Environment", UnitProperty::Strings(environment)),
        (
            "StandardOutput",
            UnitProperty::Text(format!("append:{}/stdout.log", path_text(&role_directory)?)),
        ),
        (
            "StandardError",
            UnitProperty::Text(format!("append:{}/stderr.log", path_text(&role_directory)?)),
        ),
        (
            "ExecStart",
            UnitProperty::Commands(vec![ExecCommand {
                path: binary.path.clone(),
                argv: argv.clone(),
                ignore_failure: false,
            }]),
        ),
    ];
    Ok(TransientUnitSpec {
        unit_name: intent.unit_name.clone(),
        properties,
    })
}

pub fn expected_command_environment_sha256(
    manifest: &CampaignManifest,
    campaign_directory: &Path,
) -> Result<String, UnitError> {
    let mut roles = BTreeMap::new();
    for role in ["i74", "c74"] {
        roles.insert(
            role,
            role_environment(
                &manifest.campaign_id,
                role,
                &campaign_directory.join("roles").join(role),
            )?,
        );
    }
    Ok(sha256_hex(&canonical_json(&roles)?))
}

pub fn start_transient_unit(spec: &TransientUnitSpec) -> Result<OwnedObjectPath, UnitError> {
    validate_unit_name(&spec.unit_name)?;
    let connection = Connection::system()?;
    let manager = Proxy::new(
        &connection,
        SYSTEMD_DESTINATION,
        SYSTEMD_MANAGER_PATH,
        SYSTEMD_MANAGER_INTERFACE,
    )?;
    let properties = dbus_properties(spec);
    let auxiliary: Vec<(&str, Vec<(&str, Value<'_>)>)> = Vec::new();
    Ok(manager.call(
        "StartTransientUnit",
        &(spec.unit_name.as_str(), "fail", properties, auxiliary),
    )?)
}

pub fn inspect_unit(unit_name: &str) -> Result<UnitSnapshot, UnitError> {
    validate_unit_name(unit_name)?;
    inspect_loaded_unit(unit_name)
}

pub fn inspect_unit_optional(unit_name: &str) -> Result<Option<UnitSnapshot>, UnitError> {
    validate_unit_name(unit_name)?;
    match inspect_loaded_unit(unit_name) {
        Ok(snapshot) => Ok(Some(snapshot)),
        Err(UnitError::Dbus(zbus::Error::MethodError(name, _, _)))
            if name.as_str() == "org.freedesktop.systemd1.NoSuchUnit" =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
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

fn dbus_properties(spec: &TransientUnitSpec) -> Vec<(&str, Value<'_>)> {
    spec.properties
        .iter()
        .map(|(name, property)| {
            let value = match property {
                UnitProperty::Text(value) => Value::from(value),
                UnitProperty::Boolean(value) => Value::from(*value),
                UnitProperty::Unsigned(value) => Value::from(*value),
                UnitProperty::Bytes(value) => Value::from(value),
                UnitProperty::Strings(value) => Value::from(value),
                UnitProperty::Commands(commands) => Value::from(
                    commands
                        .iter()
                        .map(|command| {
                            (
                                command.path.as_str(),
                                command.argv.iter().map(String::as_str).collect::<Vec<_>>(),
                                command.ignore_failure,
                            )
                        })
                        .collect::<Vec<_>>(),
                ),
            };
            (*name, value)
        })
        .collect()
}

fn role_runtime_seconds(manifest: &CampaignManifest) -> Result<u64, UnitError> {
    let phases = &manifest.phase_durations_seconds;
    let duration = [
        phases.warmup,
        phases.measured,
        phases.drain,
        phases.durable_companion,
        phases.post_work_idle,
        phases.reconciliation,
        manifest.diagnostic_grace_seconds,
    ]
    .into_iter()
    .try_fold(0_u64, |total, value| total.checked_add(value))
    .ok_or(UnitError::Policy)?;
    if duration == 0 || duration > MAXIMUM_ROLE_RUNTIME_SECONDS {
        return Err(UnitError::Policy);
    }
    Ok(duration)
}

fn role_environment(
    campaign_id: &str,
    role: &str,
    role_directory: &Path,
) -> Result<Vec<String>, UnitError> {
    if !matches!(role, "i74" | "c74") {
        return Err(UnitError::Policy);
    }
    Ok(vec![
        format!("HYDRACACHE_CAMPAIGN_ID={campaign_id}"),
        format!("HYDRACACHE_ROLE={role}"),
        format!(
            "HYDRACACHE_EVIDENCE_DIRECTORY={}",
            path_text(role_directory)?
        ),
        "LANG=C.UTF-8".to_owned(),
        "LC_ALL=C.UTF-8".to_owned(),
        format!("PATH={SERVICE_PATH}"),
        "RUST_BACKTRACE=0".to_owned(),
        "TZ=UTC".to_owned(),
    ])
}

fn cpuset_mask(value: &str) -> Result<Vec<u8>, UnitError> {
    if value.is_empty() || value.len() > 256 || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(UnitError::Policy);
    }
    let mut cpus = Vec::new();
    for item in value.split(',') {
        let mut bounds = item.split('-');
        let first = parse_cpu(bounds.next().ok_or(UnitError::Policy)?)?;
        let last = match bounds.next() {
            Some(last) => parse_cpu(last)?,
            None => first,
        };
        if bounds.next().is_some() || first > last {
            return Err(UnitError::Policy);
        }
        cpus.extend(first..=last);
    }
    cpus.sort_unstable();
    let original_len = cpus.len();
    cpus.dedup();
    if cpus.is_empty() || cpus.len() != original_len {
        return Err(UnitError::Policy);
    }
    let maximum = *cpus.last().ok_or(UnitError::Policy)?;
    let mut mask = vec![0_u8; maximum / 8 + 1];
    for cpu in cpus {
        mask[cpu / 8] |= 1 << (cpu % 8);
    }
    Ok(mask)
}

fn parse_cpu(value: &str) -> Result<usize, UnitError> {
    let cpu = value.parse::<usize>().map_err(|_| UnitError::Policy)?;
    if value != cpu.to_string() || cpu >= 4096 {
        return Err(UnitError::Policy);
    }
    Ok(cpu)
}

fn safe_absolute_text_path(value: &str) -> bool {
    safe_absolute_path(Path::new(value))
        && !value
            .chars()
            .any(|value| matches!(value, '\0' | '\n' | '\r'))
        && !value
            .split('/')
            .any(|component| matches!(component, "." | ".."))
}

fn safe_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::CurDir | Component::Prefix(_)
            )
        })
}

fn path_text(path: &Path) -> Result<String, UnitError> {
    path.to_str()
        .filter(|value| safe_absolute_text_path(value))
        .map(str::to_owned)
        .ok_or(UnitError::Policy)
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
