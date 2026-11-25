use super::*;
use dbus::channel::{MatchingReceiver, Sender};
use dbus::message::MatchRule;
use dbus::Message;
use std::ffi::CString;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

type OwnedProperties = Vec<(String, Variant<Box<dyn RefArg>>)>;
type Auxiliary = Vec<(String, OwnedProperties)>;
const UNIT: &str = "hydracache-performance-074-ipc.service";
const UNIT_PATH: &str = "/org/freedesktop/systemd1/unit/ipc";

enum Call {
    Start(String, String, OwnedProperties, Auxiliary),
    Lookup(String),
    Get(String, String),
}

struct PrivateBus {
    daemon: Child,
    _directory: tempfile::TempDir,
}

impl Drop for PrivateBus {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

struct FakeSystemd {
    _bus: PrivateBus,
    client: Connection,
    calls: Arc<Mutex<Vec<Call>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl FakeSystemd {
    fn new(fault: Option<&'static str>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let address = format!("unix:path={}/bus", directory.path().display());
        let daemon = Command::new("dbus-daemon")
            .args([
                "--session",
                "--nofork",
                "--nopidfile",
                "--address",
                &address,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .expect("Linux supervisor tests require dbus-daemon");
        let bus = PrivateBus {
            daemon,
            _directory: directory,
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        let server = loop {
            match Connection::new_address(&address) {
                Ok(connection) => break connection,
                Err(error) => {
                    assert!(
                        Instant::now() < deadline,
                        "private bus did not start: {error}"
                    );
                    thread::sleep(Duration::from_millis(5));
                }
            }
        };
        server
            .request_name(SYSTEMD_DESTINATION, false, false, true)
            .unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&calls);
        server.start_receive(
            MatchRule::new_method_call(),
            Box::new(move |message, connection| {
                let member = message.member().unwrap().to_string();
                let reply = match member.as_str() {
                    "StartTransientUnit" => {
                        assert_eq!(message.path().as_deref(), Some(SYSTEMD_MANAGER_PATH));
                        assert_eq!(
                            message.interface().as_deref(),
                            Some(SYSTEMD_MANAGER_INTERFACE)
                        );
                        let (name, mode, properties, auxiliary) = message
                            .read4::<String, String, OwnedProperties, Auxiliary>()
                            .unwrap();
                        captured
                            .lock()
                            .unwrap()
                            .push(Call::Start(name, mode, properties, auxiliary));
                        if fault == Some("StartTransientUnit") {
                            error_reply(&message, "org.freedesktop.DBus.Error.AccessDenied")
                        } else {
                            message.method_return().append1(
                                ObjectPath::new("/org/freedesktop/systemd1/job/7").unwrap(),
                            )
                        }
                    }
                    "GetUnit" => {
                        assert_eq!(message.path().as_deref(), Some(SYSTEMD_MANAGER_PATH));
                        assert_eq!(
                            message.interface().as_deref(),
                            Some(SYSTEMD_MANAGER_INTERFACE)
                        );
                        let name = message.read1::<String>().unwrap();
                        captured.lock().unwrap().push(Call::Lookup(name.clone()));
                        if name.ends_with("missing.service") {
                            error_reply(&message, "org.freedesktop.systemd1.NoSuchUnit")
                        } else if name.ends_with("denied.service") {
                            error_reply(&message, "org.freedesktop.DBus.Error.AccessDenied")
                        } else if name.ends_with("wrong-path.service") {
                            message.method_return().append1("not-an-object-path")
                        } else {
                            message
                                .method_return()
                                .append1(ObjectPath::new(UNIT_PATH).unwrap())
                        }
                    }
                    "Get" => {
                        assert_eq!(message.path().as_deref(), Some(UNIT_PATH));
                        assert_eq!(
                            message.interface().as_deref(),
                            Some("org.freedesktop.DBus.Properties")
                        );
                        let (interface, property) = message.read2::<String, String>().unwrap();
                        captured
                            .lock()
                            .unwrap()
                            .push(Call::Get(interface.clone(), property.clone()));
                        if fault == Some(property.as_str()) {
                            message.method_return().append1(Variant(7_u8))
                        } else {
                            match (interface.as_str(), property.as_str()) {
                                (SYSTEMD_UNIT_INTERFACE, "ActiveState") => {
                                    message.method_return().append1(Variant("active"))
                                }
                                (SYSTEMD_UNIT_INTERFACE, "SubState") => {
                                    message.method_return().append1(Variant("running"))
                                }
                                (SYSTEMD_SERVICE_INTERFACE, "MainPID") => {
                                    message.method_return().append1(Variant(100_u32))
                                }
                                (SYSTEMD_SERVICE_INTERFACE, "ControlGroup") => message
                                    .method_return()
                                    .append1(Variant("/system.slice/ipc.service")),
                                (SYSTEMD_SERVICE_INTERFACE, "Result") => {
                                    message.method_return().append1(Variant("success"))
                                }
                                _ => panic!("unexpected property: {interface} {property}"),
                            }
                        }
                    }
                    _ => panic!("unexpected method: {member}"),
                };
                connection.send(reply).unwrap();
                true
            }),
        );
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            while !stopped.load(Ordering::Acquire) {
                server.process(Duration::from_millis(10)).unwrap();
            }
        });
        Self {
            _bus: bus,
            client: Connection::new_address(&address).unwrap(),
            calls,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for FakeSystemd {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let result = self.worker.take().unwrap().join();
        if !thread::panicking() {
            result.unwrap();
        }
    }
}

fn error_reply(message: &Message, name: &str) -> Message {
    message.error(
        &name.into(),
        &CString::new("scripted systemd error").unwrap(),
    )
}

fn spec() -> TransientUnitSpec {
    TransientUnitSpec {
        unit_name: UNIT.to_owned(),
        properties: vec![
            ("Description", UnitProperty::Text("role café; $HOME".into())),
            ("NoNewPrivileges", UnitProperty::Boolean(true)),
            ("MemoryMax", UnitProperty::Unsigned(UNIT_MEMORY_MAX_BYTES)),
            ("CPUAffinity", UnitProperty::Bytes(vec![0, 255, 3])),
            (
                "Environment",
                UnitProperty::Strings(vec!["A=x y".into(), "B=é".into()]),
            ),
            (
                "ExecStart",
                UnitProperty::Commands(vec![ExecCommand {
                    path: "/opt/role".into(),
                    argv: vec!["/opt/role".into(), "space ; argument".into()],
                    ignore_failure: false,
                }]),
            ),
        ],
    }
}

#[test]
fn start_transmits_exact_systemd_types_arguments_and_empty_auxiliary() {
    let bus = FakeSystemd::new(None);
    assert_eq!(
        start_transient_unit_on(&bus.client, &spec())
            .unwrap()
            .to_string(),
        "/org/freedesktop/systemd1/job/7"
    );
    let Call::Start(name, mode, properties, auxiliary) = bus.calls.lock().unwrap().remove(0) else {
        panic!("expected start")
    };
    assert_eq!(name, UNIT);
    assert_eq!(mode, "fail");
    assert!(auxiliary.is_empty());
    let values = properties
        .into_iter()
        .map(|(name, value)| (name, value.0))
        .collect::<BTreeMap<_, _>>();
    for (name, signature) in [
        ("Description", "s"),
        ("NoNewPrivileges", "b"),
        ("MemoryMax", "t"),
        ("CPUAffinity", "ay"),
        ("Environment", "as"),
        ("ExecStart", "a(sasb)"),
    ] {
        assert_eq!(values[name].signature().to_string(), signature, "{name}");
    }
    assert_eq!(values["Description"].as_str(), Some("role café; $HOME"));
    assert_eq!(values["NoNewPrivileges"].as_u64(), Some(1));
    assert_eq!(values["MemoryMax"].as_u64(), Some(UNIT_MEMORY_MAX_BYTES));
    assert_eq!(
        values["CPUAffinity"]
            .as_iter()
            .unwrap()
            .map(|v| v.as_u64().unwrap())
            .collect::<Vec<_>>(),
        [0, 255, 3]
    );
    assert_eq!(
        values["Environment"]
            .as_iter()
            .unwrap()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        ["A=x y", "B=é"]
    );
    let mut commands = values["ExecStart"].as_iter().unwrap();
    let command = commands.next().unwrap();
    assert!(commands.next().is_none());
    let fields = command.as_iter().unwrap().collect::<Vec<_>>();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].as_str(), Some("/opt/role"));
    assert_eq!(
        fields[1]
            .as_iter()
            .unwrap()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        ["/opt/role", "space ; argument"]
    );
    assert_eq!(fields[2].as_u64(), Some(0));
}

#[test]
fn inspection_reads_each_property_from_its_correct_interface() {
    let bus = FakeSystemd::new(None);
    let snapshot = optional_snapshot(inspect_loaded_unit_on(&bus.client, UNIT))
        .unwrap()
        .unwrap();
    assert_eq!(
        snapshot,
        UnitSnapshot {
            unit_name: UNIT.into(),
            active_state: "active".into(),
            sub_state: "running".into(),
            main_pid: 100,
            control_group: "/system.slice/ipc.service".into(),
            result: "success".into()
        }
    );
    let calls = bus.calls.lock().unwrap();
    assert_eq!(calls.len(), 6);
    assert!(matches!(&calls[0], Call::Lookup(name) if name == UNIT));
    for (call, (interface, property)) in calls[1..].iter().zip([
        (SYSTEMD_UNIT_INTERFACE, "ActiveState"),
        (SYSTEMD_UNIT_INTERFACE, "SubState"),
        (SYSTEMD_SERVICE_INTERFACE, "MainPID"),
        (SYSTEMD_SERVICE_INTERFACE, "ControlGroup"),
        (SYSTEMD_SERVICE_INTERFACE, "Result"),
    ]) {
        assert!(matches!(call, Call::Get(i, p) if i == interface && p == property));
    }
}

#[test]
fn only_no_such_unit_is_optional_and_other_errors_propagate() {
    let bus = FakeSystemd::new(None);
    assert_eq!(
        optional_snapshot(inspect_loaded_unit_on(
            &bus.client,
            "hydracache-performance-074-missing.service"
        ))
        .unwrap(),
        None
    );
    assert!(
        matches!(optional_snapshot(inspect_loaded_unit_on(&bus.client, "hydracache-performance-074-denied.service")), Err(UnitError::Dbus(e)) if e.name() == Some("org.freedesktop.DBus.Error.AccessDenied"))
    );
    assert!(matches!(
        optional_snapshot(Err(UnitError::Policy)),
        Err(UnitError::Policy)
    ));
    assert!(matches!(
        inspect_loaded_unit_on(&bus.client, "hydracache-performance-074-wrong-path.service"),
        Err(UnitError::Dbus(_))
    ));
}

#[test]
fn every_malformed_property_and_start_error_fail_loudly() {
    for property in [
        "ActiveState",
        "SubState",
        "MainPID",
        "ControlGroup",
        "Result",
    ] {
        let bus = FakeSystemd::new(Some(property));
        assert!(
            matches!(
                inspect_loaded_unit_on(&bus.client, UNIT),
                Err(UnitError::Dbus(_))
            ),
            "{property}"
        );
    }
    let bus = FakeSystemd::new(Some("StartTransientUnit"));
    assert!(
        matches!(start_transient_unit_on(&bus.client, &spec()), Err(UnitError::Dbus(e)) if e.name() == Some("org.freedesktop.DBus.Error.AccessDenied"))
    );
    let mut invalid = spec();
    invalid.unit_name = "other.service".into();
    assert!(matches!(
        start_transient_unit(&invalid),
        Err(UnitError::UnitName)
    ));
    assert!(matches!(
        start_transient_unit_on(&bus.client, &invalid),
        Err(UnitError::UnitName)
    ));
}
