//! Read-only settings projection, never launch or cleanup authority.
use crate::canonical_json;
use crate::diagnostic_lease::DiagnosticState;
use crate::diagnostic_manager::{ManagerScope, ManagerSnapshot};
use crate::diagnostic_unit::{build_diagnostic_unit_spec, diagnostic_start_intent};
use crate::systemd_unit::{TransientUnitSpec, UnitProperty};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use zbus::zvariant::OwnedValue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredCommand {
    path: String,
    argv: Vec<String>,
    flags: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
enum Setting {
    Text(String),
    Boolean(bool),
    Unsigned(u64),
    Bytes(Vec<u8>),
    Strings(Vec<String>),
    Commands(Vec<ConfiguredCommand>),
    EnvironmentFiles(Vec<(String, bool)>),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoadedSettings(BTreeMap<String, Setting>);

/// New, separate wire document. No public deserializer or policy-proof token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoadedSnapshot {
    schema_version: u32,
    manager: ManagerSnapshot,
    settings: Option<LoadedSettings>,
}
impl LoadedSnapshot {
    pub(crate) fn new(manager: ManagerSnapshot, settings: Option<LoadedSettings>) -> Self {
        Self {
            schema_version: 1,
            manager,
            settings,
        }
    }
    pub fn manager(&self) -> &ManagerSnapshot {
        &self.manager
    }
    pub(crate) fn decode(bytes: &[u8], scope: &ManagerScope) -> Result<Self, String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            schema_version: u32,
            manager: ManagerSnapshot,
            settings: Option<LoadedSettings>,
        }
        if bytes.is_empty() || bytes.len() > 65536 {
            return Err("loaded snapshot byte budget".into());
        }
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| "invalid loaded snapshot")?;
        let snapshot = Self {
            schema_version: wire.schema_version,
            manager: wire.manager,
            settings: wire.settings,
        };
        snapshot.manager.validate(scope)?;
        if snapshot.schema_version != 1
            || snapshot.manager.unit().is_some() != snapshot.settings.is_some()
            || canonical_json(&snapshot).map_err(|_| "loaded snapshot encoding")? != bytes
        {
            return Err("loaded snapshot binding".into());
        }
        if let Some(settings) = &snapshot.settings {
            settings.validate()?;
        }
        Ok(snapshot)
    }
    /// Checks only the exposed settings projection. Not an authenticated start
    /// intent, output destination, effective environment, executable or tree proof.
    pub fn pin_for(&self, state: &DiagnosticState) -> Result<InvocationGuard, String> {
        let intent = diagnostic_start_intent(state)?;
        let scope = ManagerScope::new(&intent.lease_id, &intent.boot_id, &intent.surface)?;
        self.manager.validate(&scope)?;
        let expected = LoadedSettings(expected_settings(&build_diagnostic_unit_spec(state)?)?);
        let guard = InvocationGuard {
            original: self.manager.clone(),
            expected,
            refused: false,
        };
        guard.check(self)?;
        Ok(guard)
    }
}

/// Runtime-only sticky refusal. Never serializable, never cleanup authority.
pub struct InvocationGuard {
    original: ManagerSnapshot,
    expected: LoadedSettings,
    refused: bool,
}
impl InvocationGuard {
    pub(crate) fn scope(&self) -> &ManagerScope {
        self.original.scope()
    }
    pub(crate) fn refuse(&mut self) {
        self.refused = true;
    }
    fn check(&self, snapshot: &LoadedSnapshot) -> Result<(), String> {
        snapshot.manager.validate(self.original.scope())?;
        let unit = snapshot.manager.unit().ok_or("loaded unit absent")?;
        let original = self.original.unit().ok_or("original unit absent")?;
        if snapshot.schema_version != 1
            || !self.original.same_manager(&snapshot.manager)
            || !unit.transient
            || unit.invocation_id == "00".repeat(16)
            || unit.control_group.is_empty()
            || unit.unit_name != original.unit_name
            || unit.object_path != original.object_path
            || unit.invocation_id != original.invocation_id
            || unit.control_group != original.control_group
            || snapshot.settings.as_ref() != Some(&self.expected)
        {
            return Err("original invocation or loaded settings drift".into());
        }
        Ok(())
    }
    pub fn revalidate(&mut self, snapshot: &LoadedSnapshot) -> Result<(), String> {
        if self.refused {
            return Err("loaded invocation previously refused".into());
        }
        if let Err(error) = self.check(snapshot) {
            self.refused = true;
            return Err(error);
        }
        Ok(())
    }
    pub fn is_refused(&self) -> bool {
        self.refused
    }
}

const TEXT: &[&str] = &[
    "Description",
    "User",
    "Group",
    "Type",
    "Restart",
    "KillMode",
    "Slice",
    "ProtectSystem",
    "ProtectHome",
    "WorkingDirectory",
    "StandardInput",
    "StandardOutput",
    "StandardError",
    "StandardOutputFileDescriptorName",
    "StandardErrorFileDescriptorName",
    "RootDirectory",
    "RootImage",
];
const BOOL: &[&str] = &[
    "RemainAfterExit",
    "SendSIGKILL",
    "Delegate",
    "NoNewPrivileges",
    "PrivateTmp",
    "PrivateDevices",
    "ProtectControlGroups",
    "ProtectKernelTunables",
    "ProtectKernelModules",
    "ProtectKernelLogs",
    "RestrictSUIDSGID",
    "RestrictRealtime",
    "LockPersonality",
    "IOAccounting",
    "DynamicUser",
];
const UINT: &[&str] = &[
    "RuntimeMaxUSec",
    "TimeoutStopUSec",
    "MemoryMax",
    "TasksMax",
    "LimitNOFILE",
    "LimitFSIZE",
    "LimitNOFILESoft",
    "LimitFSIZESoft",
];
const STRINGS: &[&str] = &[
    "Environment",
    "ReadWritePaths",
    "PassEnvironment",
    "UnsetEnvironment",
];
const COMMANDS: &[&str] = &[
    "ExecStartEx",
    "ExecConditionEx",
    "ExecStartPreEx",
    "ExecStartPostEx",
    "ExecReloadEx",
    "ExecStopEx",
    "ExecStopPostEx",
];
// Exact systemd 255 Service GetAll extended-command signature a(sasasttttuii).
type ExtendedCommand = (
    String,
    Vec<String>,
    Vec<String>,
    u64,
    u64,
    u64,
    u64,
    u32,
    i32,
    i32,
);

fn expected_settings(spec: &TransientUnitSpec) -> Result<BTreeMap<String, Setting>, String> {
    let mut settings = BTreeMap::new();
    for (name, value) in &spec.properties {
        let (name, setting) = match (*name, value) {
            ("StandardOutputFileToAppend", UnitProperty::Text(_)) => {
                ("StandardOutput", Setting::Text("append".into()))
            }
            ("StandardErrorFileToAppend", UnitProperty::Text(_)) => {
                ("StandardError", Setting::Text("append".into()))
            }
            ("ExecStart", UnitProperty::Commands(v)) => (
                "ExecStartEx",
                Setting::Commands(
                    v.iter()
                        .map(|c| ConfiguredCommand {
                            path: c.path.clone(),
                            argv: c.argv.clone(),
                            flags: if c.ignore_failure {
                                vec!["ignore-failure".into()]
                            } else {
                                vec![]
                            },
                        })
                        .collect(),
                ),
            ),
            (name, UnitProperty::Text(v)) => (name, Setting::Text(v.clone())),
            (name, UnitProperty::Boolean(v)) => (name, Setting::Boolean(*v)),
            (name, UnitProperty::Unsigned(v)) => (name, Setting::Unsigned(*v)),
            (name, UnitProperty::Bytes(v)) => (name, Setting::Bytes(v.clone())),
            (name, UnitProperty::Strings(v)) => (name, Setting::Strings(v.clone())),
            _ => return Err("unsupported diagnostic spec property".into()),
        };
        if settings.insert(name.into(), setting).is_some() {
            return Err("duplicate diagnostic spec property".into());
        }
    }
    for name in [
        "StandardOutputFileDescriptorName",
        "StandardErrorFileDescriptorName",
        "RootDirectory",
        "RootImage",
    ] {
        settings.insert(name.into(), Setting::Text(String::new()));
    }
    for name in ["PassEnvironment", "UnsetEnvironment"] {
        settings.insert(name.into(), Setting::Strings(vec![]));
    }
    for name in &COMMANDS[1..] {
        settings.insert((*name).into(), Setting::Commands(vec![]));
    }
    settings.insert("DynamicUser".into(), Setting::Boolean(false));
    settings.insert("EnvironmentFiles".into(), Setting::EnvironmentFiles(vec![]));
    for name in ["LimitNOFILE", "LimitFSIZE"] {
        settings.insert(
            format!("{name}Soft"),
            settings.get(name).ok_or("missing hard limit")?.clone(),
        );
    }
    LoadedSettings(settings.clone()).validate()?;
    Ok(settings)
}
impl LoadedSettings {
    fn validate(&self) -> Result<(), String> {
        let expected_len =
            TEXT.len() + BOOL.len() + UINT.len() + STRINGS.len() + COMMANDS.len() + 2;
        if self.0.len() != expected_len {
            return Err("loaded settings inventory".into());
        }
        for (name, value) in &self.0 {
            let valid = match value {
                Setting::Text(v) => TEXT.contains(&name.as_str()) && bounded(v),
                Setting::Boolean(_) => BOOL.contains(&name.as_str()),
                Setting::Unsigned(_) => UINT.contains(&name.as_str()),
                Setting::Bytes(v) => name == "CPUAffinity" && v.len() <= 128,
                Setting::Strings(v) => STRINGS.contains(&name.as_str()) && strings(v),
                Setting::Commands(v) => {
                    COMMANDS.contains(&name.as_str())
                        && v.len() <= 8
                        && v.iter()
                            .all(|c| bounded(&c.path) && strings(&c.argv) && strings(&c.flags))
                }
                Setting::EnvironmentFiles(v) => {
                    name == "EnvironmentFiles" && v.len() <= 8 && v.iter().all(|(p, _)| bounded(p))
                }
            };
            if !valid {
                return Err("loaded settings field or type".into());
            }
        }
        Ok(())
    }
}
fn bounded(v: &str) -> bool {
    v.len() <= 1024 && !v.bytes().any(|b| b == 0 || b == b'\n' || b == b'\r')
}
fn strings(v: &[String]) -> bool {
    v.len() <= 16 && v.iter().all(|s| bounded(s))
}

pub(crate) fn decode_settings(
    mut properties: HashMap<String, OwnedValue>,
) -> Result<LoadedSettings, String> {
    use crate::diagnostic_manager::take;
    let mut settings = BTreeMap::new();
    for name in TEXT {
        settings.insert((*name).into(), Setting::Text(take(&mut properties, name)?));
    }
    for name in BOOL {
        settings.insert(
            (*name).into(),
            Setting::Boolean(take(&mut properties, name)?),
        );
    }
    for name in UINT {
        settings.insert(
            (*name).into(),
            Setting::Unsigned(take(&mut properties, name)?),
        );
    }
    for name in STRINGS {
        settings.insert(
            (*name).into(),
            Setting::Strings(take(&mut properties, name)?),
        );
    }
    settings.insert(
        "CPUAffinity".into(),
        Setting::Bytes(take(&mut properties, "CPUAffinity")?),
    );
    settings.insert(
        "EnvironmentFiles".into(),
        Setting::EnvironmentFiles(take(&mut properties, "EnvironmentFiles")?),
    );
    for name in COMMANDS {
        let commands: Vec<ExtendedCommand> = take(&mut properties, name)?;
        settings.insert(
            (*name).into(),
            Setting::Commands(
                commands
                    .into_iter()
                    .map(|c| ConfiguredCommand {
                        path: c.0,
                        argv: c.1,
                        flags: c.2,
                    })
                    .collect(),
            ),
        );
    }
    let settings = LoadedSettings(settings);
    settings.validate()?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_lease::{DiagnosticIdentity, DiagnosticStage};
    use serde_json::json;

    fn state(index: usize) -> DiagnosticState {
        DiagnosticState {
            identity: DiagnosticIdentity {
                lease_id: "a".repeat(64),
                boot_id: "12345678-1234-1234-1234-123456789abc".into(),
                binary_sha256: "b".repeat(64),
                build_provenance_sha256: "c".repeat(64),
            },
            revision: 1,
            stage: DiagnosticStage::Reserved,
            completed_cells: index,
            reserved_monotonic_ns: 1,
            last_observed_monotonic_ns: 1,
            controller_monotonic_ns: 1,
            cell_started_monotonic_ns: None,
            cgroup_inode: None,
            reason: None,
            cleanup_confirmed: true,
            promotable: false,
            admission_allowed: false,
        }
    }
    fn fixture(index: usize) -> (DiagnosticState, LoadedSnapshot) {
        let state = state(index);
        let intent = diagnostic_start_intent(&state).unwrap();
        let scope = ManagerScope::new(&intent.lease_id, &intent.boot_id, &intent.surface).unwrap();
        let manager = serde_json::from_value(json!({
            "schema_version":1, "scope": scope, "manager_owner":":1.5",
            "manager_uid":0, "manager_pid":1,
            "unit": {"unit_name":intent.unit_name,
                "object_path":"/org/freedesktop/systemd1/unit/example",
                "invocation_id":"ab".repeat(16), "active_state":"active",
                "sub_state":"running", "main_pid":9,
                "control_group":intent.cgroup_path, "result":"success", "transient":true}
        }))
        .unwrap();
        let settings = expected_settings(&build_diagnostic_unit_spec(&state).unwrap()).unwrap();
        (
            state,
            LoadedSnapshot::new(manager, Some(LoadedSettings(settings))),
        )
    }
    fn change(snapshot: &LoadedSnapshot, key: &str, value: serde_json::Value) -> LoadedSnapshot {
        let mut wire = serde_json::to_value(snapshot).unwrap();
        wire["manager"][key] = value;
        if wire["manager"]["unit"].is_null() {
            wire["settings"] = serde_json::Value::Null;
        }
        LoadedSnapshot::decode(&canonical_json(&wire).unwrap(), snapshot.manager.scope()).unwrap()
    }
    #[test]
    fn loaded_settings_bind_four_cells_and_nonzero_original_invocation() {
        for index in 0..4 {
            let (state, snapshot) = fixture(index);
            let mut guard = snapshot.pin_for(&state).unwrap();
            guard.revalidate(&snapshot).unwrap();
            assert!(!guard.is_refused());
            let (foreign, _) = fixture((index + 1) % 4);
            assert!(snapshot.pin_for(&foreign).is_err());
            let mut zero = snapshot.clone();
            zero.manager = change(
                &snapshot,
                "unit",
                json!({
                    "unit_name": snapshot.manager.unit().unwrap().unit_name,
                    "object_path":"/org/freedesktop/systemd1/unit/example",
                    "invocation_id":"00".repeat(16), "active_state":"active",
                    "sub_state":"running", "main_pid":9,
                    "control_group":snapshot.manager.unit().unwrap().control_group,
                    "result":"success", "transient":true
                }),
            )
            .manager;
            assert!(zero.pin_for(&state).is_err());
        }
    }
    #[test]
    fn loaded_settings_each_projected_property_drift_is_sticky() {
        let (state, good) = fixture(2);
        for key in good.settings.as_ref().unwrap().0.keys() {
            let mut bad = good.clone();
            bad.settings.as_mut().unwrap().0.remove(key);
            assert!(bad.pin_for(&state).is_err(), "missing {key}");
            let mut guard = good.pin_for(&state).unwrap();
            assert!(guard.revalidate(&bad).is_err(), "drift {key}");
            assert!(guard.is_refused());
            assert!(guard.revalidate(&good).is_err());
            let mut changed = good.clone();
            let value = changed.settings.as_mut().unwrap().0.get_mut(key).unwrap();
            *value = match value {
                Setting::Text(v) => Setting::Text(format!("{v}-changed")),
                Setting::Boolean(v) => Setting::Boolean(!*v),
                Setting::Unsigned(v) => Setting::Unsigned(v.saturating_add(1)),
                Setting::Bytes(_) => Setting::Bytes(vec![0]),
                Setting::Strings(_) => Setting::Strings(vec!["changed".into()]),
                Setting::Commands(_) => Setting::Commands(vec![]),
                Setting::EnvironmentFiles(_) => {
                    Setting::EnvironmentFiles(vec![("/foreign".into(), false)])
                }
            };
            if changed.settings == good.settings {
                changed
                    .settings
                    .as_mut()
                    .unwrap()
                    .0
                    .insert(key.clone(), Setting::Text("wrong-type".into()));
            }
            assert!(changed.pin_for(&state).is_err(), "changed {key}");
        }
        let mut bad = good.clone();
        bad.settings
            .as_mut()
            .unwrap()
            .0
            .insert("ExecStopEx".into(), Setting::Text("foreign".into()));
        assert!(bad.pin_for(&state).is_err());
    }
    #[test]
    fn loaded_helper_failure_latches_guard_without_adopting_later_success() {
        let (state, good) = fixture(0);
        let mut guard = good.pin_for(&state).unwrap();
        // /proc/self/exe here is the test binary, not the supervisor worker.
        // It refuses the unknown mode before any DBus access or unit action.
        let mut client = crate::diagnostic_manager::ManagerClient::default();
        let error = client.inspect_original(&mut guard).err().unwrap();
        assert!(error.cleanup_confirmed);
        assert!(guard.is_refused());
        assert!(guard.revalidate(&good).is_err());
        let error = client.inspect_original(&mut guard).err().unwrap();
        assert_eq!(
            error.kind,
            crate::diagnostic_manager::WorkerFailureKind::Invalid
        );
        assert!(error.stdout.is_empty() && error.stderr.is_empty());
    }
    #[test]
    fn loaded_invocation_guard_refuses_replacement_owner_object_or_absence() {
        let (state, good) = fixture(0);
        for (key, value) in [
            ("manager_owner", json!(":1.6")),
            ("unit", serde_json::Value::Null),
        ] {
            let mut bad = change(&good, key, value);
            if bad.manager.unit().is_none() {
                bad.settings = None;
            }
            let mut guard = good.pin_for(&state).unwrap();
            assert!(guard.revalidate(&bad).is_err());
            assert!(guard.revalidate(&good).is_err());
        }
        for (key, value) in [
            ("invocation_id", json!("cd".repeat(16))),
            (
                "object_path",
                json!("/org/freedesktop/systemd1/unit/replaced"),
            ),
            ("transient", json!(false)),
        ] {
            let mut wire = serde_json::to_value(&good).unwrap();
            wire["manager"]["unit"][key] = value;
            let bad = LoadedSnapshot::decode(&canonical_json(&wire).unwrap(), good.manager.scope())
                .unwrap();
            let mut guard = good.pin_for(&state).unwrap();
            assert!(guard.revalidate(&bad).is_err());
            assert!(guard.revalidate(&good).is_err());
        }
    }
    #[test]
    fn loaded_wire_refuses_schema_unknown_duplicate_noncanonical_and_budget() {
        let (_, good) = fixture(1);
        let bytes = canonical_json(&good).unwrap();
        LoadedSnapshot::decode(&bytes, good.manager.scope()).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        for bad in [
            text.replacen("\"schema_version\":1", "\"schema_version\":2", 1),
            text.replacen('{', "{\"extra\":false,", 1),
            text.replacen('{', "{\"schema_version\":1,", 1),
            format!("{text}\n"),
        ] {
            assert!(LoadedSnapshot::decode(bad.as_bytes(), good.manager.scope()).is_err());
        }
        assert!(LoadedSnapshot::decode(&vec![b'x'; 65537], good.manager.scope()).is_err());
        let mut absent = good.clone();
        absent.manager = change(&good, "unit", serde_json::Value::Null).manager;
        assert!(
            LoadedSnapshot::decode(&canonical_json(&absent).unwrap(), good.manager.scope())
                .is_err()
        );
    }
    #[test]
    fn loaded_projection_distinguishes_setter_names_and_extended_exec_flags() {
        let (state, good) = fixture(2);
        let settings = &good.settings.as_ref().unwrap().0;
        assert!(!settings.contains_key("StandardOutputFileToAppend"));
        assert_eq!(settings["StandardOutput"], Setting::Text("append".into()));
        assert_eq!(settings["StandardError"], Setting::Text("append".into()));
        assert_eq!(settings["LimitNOFILESoft"], Setting::Unsigned(4096));
        assert_eq!(settings["LimitFSIZESoft"], Setting::Unsigned(8388608));
        let mut bad = good.clone();
        if let Setting::Commands(commands) = bad
            .settings
            .as_mut()
            .unwrap()
            .0
            .get_mut("ExecStartEx")
            .unwrap()
        {
            commands[0].flags.push("full-privileges".into());
        } else {
            panic!("missing extended command");
        }
        assert!(bad.pin_for(&state).is_err());
    }
    #[test]
    fn loaded_dbus_projection_requires_exact_types_and_ignores_only_command_status() {
        use zbus::zvariant::Value;
        let (_, good) = fixture(0);
        let expected = &good.settings.as_ref().unwrap().0;
        let mut service: HashMap<String, OwnedValue> = expected
            .iter()
            .map(|(key, value)| {
                let value = match value {
                    Setting::Text(v) => OwnedValue::try_from(Value::from(v.as_str())).unwrap(),
                    Setting::Boolean(v) => OwnedValue::from(*v),
                    Setting::Unsigned(v) => OwnedValue::from(*v),
                    Setting::Bytes(v) => OwnedValue::try_from(Value::from(v.clone())).unwrap(),
                    Setting::Strings(v) => OwnedValue::try_from(Value::from(v.clone())).unwrap(),
                    Setting::Commands(v) => {
                        let tuples: Vec<ExtendedCommand> = v
                            .iter()
                            .map(|c| {
                                (
                                    c.path.clone(),
                                    c.argv.clone(),
                                    c.flags.clone(),
                                    1,
                                    2,
                                    3,
                                    4,
                                    9,
                                    0,
                                    0,
                                )
                            })
                            .collect();
                        OwnedValue::try_from(Value::from(tuples)).unwrap()
                    }
                    Setting::EnvironmentFiles(v) => {
                        OwnedValue::try_from(Value::from(v.clone())).unwrap()
                    }
                };
                (key.clone(), value)
            })
            .collect();
        let copied = service
            .iter()
            .map(|(k, v)| (k.clone(), v.try_clone().unwrap()))
            .collect();
        assert_eq!(decode_settings(copied).unwrap().0, *expected);
        service.insert("RuntimeMaxUSec".into(), OwnedValue::from(60u32));
        assert!(decode_settings(service).is_err());
        let missing: HashMap<String, OwnedValue> = HashMap::new();
        assert!(decode_settings(missing).is_err());
    }
}
