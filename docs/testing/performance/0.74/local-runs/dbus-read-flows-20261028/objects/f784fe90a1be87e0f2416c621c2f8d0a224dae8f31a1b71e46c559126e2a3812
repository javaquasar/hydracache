//! Whole selected protocol/decoder bodies, synthetic peer only; no host admission.
use super::*;
use std::{
    future::poll_fn,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use zbus::{export::futures_core::Stream, message::Message};
const LIMIT: Duration = Duration::from_millis(250);
const NAME: &str = "hydracache-diagnostic-074-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1.service";
const LEGACY: &str = "hydracache-performance-074-read.service";
const OBJECT: &str = "/org/freedesktop/systemd1/unit/read";
type Values = HashMap<String, OwnedValue>;

fn val<T: Into<zbus::Variant<'static>>>(v: T) -> OwnedValue {
    OwnedValue::try_from(v.into()).unwrap()
}
fn unit(name: &str) -> Values {
    HashMap::from([
        ("Id".into(), val(name.to_owned())),
        ("InvocationID".into(), val(vec![0xabu8; 16])),
        ("ActiveState".into(), val("active")),
        ("SubState".into(), val("running")),
        ("Transient".into(), val(true)),
        ("Description".into(), val("unit description")),
    ])
}
fn service(name: &str, pid: u32, loaded: bool) -> Values {
    let mut v = Values::new();
    if loaded {
        for key in TEXT {
            v.insert((*key).into(), val(""));
        }
        for key in BOOL {
            v.insert((*key).into(), val(false));
        }
        for key in UINT {
            v.insert((*key).into(), val(1u64));
        }
        for key in STRINGS {
            v.insert((*key).into(), val(Vec::<String>::new()));
        }
        for key in COMMANDS {
            v.insert((*key).into(), val(Vec::<ExtendedCommand>::new()));
        }
        let command: ExtendedCommand = (
            "/fixture".into(),
            vec!["fixture".into()],
            vec![],
            0,
            0,
            0,
            0,
            74,
            0,
            0,
        );
        v.insert("ExecStartEx".into(), val(vec![command]));
        v.insert("CPUAffinity".into(), val(vec![1u8]));
        v.insert("EnvironmentFiles".into(), val(Vec::<(String, bool)>::new()));
        v.insert("Description".into(), val("foreign service description"));
    }
    v.insert("MainPID".into(), val(pid));
    v.insert("ControlGroup".into(), val(format!("/system.slice/{name}")));
    v.insert("Result".into(), val("success"));
    v
}
enum Response {
    Text(&'static str),
    Number(u32),
    Path,
    Map(Values),
    Error(&'static str),
}
struct Step {
    dest: &'static str,
    path: &'static str,
    iface: &'static str,
    member: &'static str,
    arg: String,
    sender: &'static str,
    response: Response,
}
fn owners(owner: &'static str) -> Vec<Step> {
    [
        ("GetNameOwner", DESTINATION, Response::Text(owner)),
        ("GetConnectionUnixUser", owner, Response::Number(0)),
        ("GetConnectionUnixProcessID", owner, Response::Number(1)),
    ]
    .into_iter()
    .map(|(member, arg, response)| Step {
        dest: "org.freedesktop.DBus",
        path: "/org/freedesktop/DBus",
        iface: "org.freedesktop.DBus",
        member,
        arg: arg.into(),
        sender: "org.freedesktop.DBus",
        response,
    })
    .collect()
}
fn lookup(name: &str, dest: &'static str, response: Response) -> Step {
    Step {
        dest,
        path: "/org/freedesktop/systemd1",
        iface: "org.freedesktop.systemd1.Manager",
        member: "GetUnit",
        arg: name.into(),
        sender: ":1.1",
        response,
    }
}
fn getall(dest: &'static str, iface: &'static str, response: Response) -> Step {
    Step {
        dest,
        path: OBJECT,
        iface: "org.freedesktop.DBus.Properties",
        member: "GetAll",
        arg: iface.into(),
        sender: ":1.1",
        response,
    }
}
fn pair_reads(loaded: bool, second_pid: u32) -> Vec<Step> {
    let mut v = owners(":1.1");
    v.push(lookup(NAME, ":1.1", Response::Path));
    for pid in [74, second_pid] {
        v.push(getall(":1.1", UNIT_INTERFACE, Response::Map(unit(NAME))));
        v.push(getall(
            ":1.1",
            SERVICE_INTERFACE,
            Response::Map(service(NAME, pid, loaded)),
        ));
    }
    v
}
struct Peer {
    client: Connection,
    server: zbus::Connection,
    task: tokio::task::JoinHandle<()>,
    trace: Arc<Mutex<Vec<String>>>,
    expected: Vec<String>,
}
impl Peer {
    fn new(steps: Vec<Step>) -> Self {
        let (client, server) = zbus::block_on(async {
            let (a, b) = zbus::connection::socket::Channel::pair();
            let guid = zbus::Guid::try_from("07400000000000000000000000000000").unwrap();
            let a = zbus::connection::Builder::authenticated_socket(a, guid.clone())
                .p2p()
                .method_timeout(LIMIT)
                .build()
                .await
                .unwrap();
            let b = zbus::connection::Builder::authenticated_socket(b, guid)
                .p2p()
                .method_timeout(LIMIT)
                .build()
                .await
                .unwrap();
            (Connection::from_async(a), b)
        });
        let expected = steps
            .iter()
            .map(|s| format!("{}:{}:{}:{}:{}", s.dest, s.path, s.iface, s.member, s.arg))
            .collect();
        let trace = Arc::new(Mutex::new(Vec::new()));
        let record = trace.clone();
        let connection = server.clone();
        let (runtime, mut stream) = zbus::block_on(async {
            (
                tokio::runtime::Handle::current(),
                zbus::MessageStream::from(&connection),
            )
        });
        let task = runtime.spawn(async move {
            for step in steps {
                let request =
                    tokio::time::timeout(LIMIT, poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)))
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap();
                assert_eq!(request.header().destination().unwrap().as_str(), step.dest);
                assert_eq!(request.header().path().unwrap().as_str(), step.path);
                assert_eq!(request.header().interface().unwrap().as_str(), step.iface);
                assert_eq!(request.header().member().unwrap().as_str(), step.member);
                let arg: String = request.body().deserialize().unwrap();
                assert_eq!(arg, step.arg);
                record.lock().unwrap().push(format!(
                    "{}:{}:{}:{}:{}",
                    step.dest, step.path, step.iface, step.member, arg
                ));
                let h = request.header();
                let response = match step.response {
                    Response::Text(v) => Message::method_return(&h).sender(step.sender).build(&v),
                    Response::Number(v) => Message::method_return(&h).sender(step.sender).build(&v),
                    Response::Path => Message::method_return(&h)
                        .sender(step.sender)
                        .build(&OwnedObjectPath::try_from(OBJECT).unwrap()),
                    Response::Map(v) => Message::method_return(&h).sender(step.sender).build(&v),
                    Response::Error(e) => Message::error(&h, e)
                        .sender(step.sender)
                        .build(&"synthetic error"),
                }
                .unwrap();
                connection.send(&response).await.unwrap();
            }
            zbus::AsyncDrop::async_drop(stream).await;
        });
        Self {
            client,
            server,
            task,
            trace,
            expected,
        }
    }
    fn finish(self) {
        zbus::block_on(async {
            tokio::time::timeout(LIMIT, self.task)
                .await
                .unwrap()
                .unwrap();
        });
        assert_eq!(*self.trace.lock().unwrap(), self.expected);
        println!("HC074_FLOW_TRACE:{:?}", self.expected);
        self.client.close().unwrap();
        zbus::block_on(self.server.close()).unwrap();
    }
}
#[test]
fn protocol_present_reads_twice_then_owner() {
    let mut script = pair_reads(false, 74);
    script.extend(owners(":1.1"));
    let peer = Peer::new(script);
    let (owner, unit, settings) = inspect_protocol(peer.client.clone(), NAME, false).unwrap();
    assert_eq!(owner, ":1.1");
    assert_eq!(unit.unwrap().main_pid, 74);
    assert!(settings.is_none());
    peer.finish();
}
#[test]
fn protocol_owner_change_refuses_after_reads() {
    let mut script = pair_reads(false, 74);
    script.extend(owners(":1.2"));
    let peer = Peer::new(script);
    assert_eq!(
        inspect_protocol(peer.client.clone(), NAME, false).unwrap_err(),
        "manager owner changed"
    );
    peer.finish();
}
#[test]
fn protocol_unit_change_stops_before_owner_recheck() {
    let peer = Peer::new(pair_reads(false, 75));
    assert_eq!(
        inspect_protocol(peer.client.clone(), NAME, false).unwrap_err(),
        "unit observation changed"
    );
    peer.finish();
}
#[test]
fn protocol_absence_and_foreign_error_keep_scope() {
    for foreign in [false, true] {
        let mut script = owners(":1.1");
        let mut request = lookup(
            NAME,
            ":1.1",
            Response::Error("org.freedesktop.systemd1.NoSuchUnit"),
        );
        if foreign {
            request.sender = ":1.2";
        }
        script.push(request);
        if !foreign {
            script.extend(owners(":1.1"));
        }
        let peer = Peer::new(script);
        let result = inspect_protocol(peer.client.clone(), NAME, false);
        if foreign {
            assert_eq!(result.unwrap_err(), "unit inspection failed");
        } else {
            let (_, unit, settings) = result.unwrap();
            assert!(unit.is_none() && settings.is_none());
        }
        peer.finish();
    }
}
#[test]
fn loaded_settings_decode_and_use_unit_description() {
    let mut script = pair_reads(true, 74);
    script.extend(owners(":1.1"));
    let peer = Peer::new(script);
    let (_, unit, settings) = inspect_protocol(peer.client.clone(), NAME, true).unwrap();
    assert_eq!(unit.unwrap().main_pid, 74);
    let settings = settings.unwrap();
    assert_eq!(
        settings.0.len(),
        TEXT.len() + BOOL.len() + UINT.len() + STRINGS.len() + COMMANDS.len() + 2
    );
    assert_eq!(
        settings.0.get("Description"),
        Some(&Setting::Text("unit description".into()))
    );
    assert!(
        matches!(settings.0.get("ExecStartEx"),Some(Setting::Commands(v)) if v.len()==1&&v[0].path=="/fixture")
    );
    peer.finish();
}
#[test]
fn loaded_missing_or_invalid_values_stop_first_pair() {
    for case in 0..6 {
        let mut u = unit(NAME);
        let mut s = service(NAME, 74, true);
        let expected = match case {
            0 => {
                s.remove("MainPID");
                "missing service identity"
            }
            1 => {
                u.remove("Description");
                "missing unit description"
            }
            2 => {
                s.remove("User");
                "missing unit property"
            }
            3 => {
                u.insert("InvocationID".into(), val(vec![0u8; 15]));
                "unit invocation identity"
            }
            4 => {
                s.insert("NoNewPrivileges".into(), val("wrong type"));
                "unit property type"
            }
            _ => {
                s.insert("User".into(), val("x".repeat(1025)));
                "loaded settings field or type"
            }
        };
        println!("HC074_LOADED_REFUSAL:{case}");
        let mut script = owners(":1.1");
        script.push(lookup(NAME, ":1.1", Response::Path));
        script.push(getall(":1.1", UNIT_INTERFACE, Response::Map(u)));
        script.push(getall(":1.1", SERVICE_INTERFACE, Response::Map(s)));
        let peer = Peer::new(script);
        assert_eq!(
            inspect_protocol(peer.client.clone(), NAME, true).unwrap_err(),
            expected
        );
        peer.finish();
    }
}
#[test]
fn legacy_reader_uses_lazy_getall_snapshots() {
    let peer = Peer::new(vec![
        lookup(LEGACY, DESTINATION, Response::Path),
        getall(DESTINATION, UNIT_INTERFACE, Response::Map(unit(LEGACY))),
        getall(
            DESTINATION,
            SERVICE_INTERFACE,
            Response::Map(service(LEGACY, 74, false)),
        ),
    ]);
    let actual = inspect_loaded_unit(peer.client.clone(), LEGACY).unwrap();
    assert_eq!(
        actual,
        UnitSnapshot {
            unit_name: LEGACY.into(),
            active_state: "active".into(),
            sub_state: "running".into(),
            main_pid: 74,
            control_group: format!("/system.slice/{LEGACY}"),
            result: "success".into()
        }
    );
    peer.finish();
}
#[test]
fn legacy_optional_absence_and_namespace_refusal() {
    let peer = Peer::new(vec![lookup(
        LEGACY,
        DESTINATION,
        Response::Error("org.freedesktop.systemd1.NoSuchUnit"),
    )]);
    assert!(
        inspect_unit_optional(peer.client.clone(), LEGACY)
            .unwrap()
            .is_none()
    );
    peer.finish();
    let peer = Peer::new(vec![]);
    assert!(matches!(
        inspect_unit_optional(peer.client.clone(), "foreign.service"),
        Err(UnitError::UnitName)
    ));
    peer.finish();
}
#[test]
fn legacy_lazy_getall_failure_is_not_a_snapshot() {
    let peer = Peer::new(vec![
        lookup(LEGACY, DESTINATION, Response::Path),
        getall(
            DESTINATION,
            UNIT_INTERFACE,
            Response::Error("org.freedesktop.DBus.Error.UnknownInterface"),
        ),
    ]);
    assert!(matches!(
        inspect_loaded_unit(peer.client.clone(), LEGACY),
        Err(UnitError::Dbus(_))
    ));
    peer.finish();
}
