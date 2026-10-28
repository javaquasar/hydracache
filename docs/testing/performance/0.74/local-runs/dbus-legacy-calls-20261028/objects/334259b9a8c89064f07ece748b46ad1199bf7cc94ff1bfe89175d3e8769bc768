//! Synthetic Channel only; no OS credentials, real bus or manager authority.
use super::*;
use std::{
    future::poll_fn,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use zbus::{export::futures_core::Stream, message::Message};

const LIMIT: Duration = Duration::from_millis(250);

#[derive(Clone)]
enum Reply {
    Text(&'static str),
    Number(u32),
    VariantText(&'static str),
    VariantNumber(u32),
    Error(&'static str),
    Silent,
}

#[derive(Clone)]
struct Step {
    destination: &'static str,
    interface: &'static str,
    path: &'static str,
    member: &'static str,
    args: Vec<&'static str>,
    sender: &'static str,
    reply: Reply,
}

fn owner_steps(owner: &'static str, uid: u32, pid: u32) -> Vec<Step> {
    [
        ("GetNameOwner", DESTINATION, Reply::Text(owner)),
        ("GetConnectionUnixUser", owner, Reply::Number(uid)),
        ("GetConnectionUnixProcessID", owner, Reply::Number(pid)),
    ]
    .into_iter()
    .map(|(member, arg, reply)| Step {
        destination: "org.freedesktop.DBus",
        interface: "org.freedesktop.DBus",
        path: "/org/freedesktop/DBus",
        member,
        args: vec![arg],
        sender: "org.freedesktop.DBus",
        reply,
    })
    .collect()
}

fn property(name: &'static str, reply: Reply) -> Step {
    Step {
        destination: ":1.1",
        interface: "org.freedesktop.DBus.Properties",
        path: "/fixture",
        member: "Get",
        args: vec!["org.example.Unit", name],
        sender: ":1.1",
        reply,
    }
}

struct Peer {
    client: Connection,
    server: zbus::Connection,
    task: tokio::task::JoinHandle<()>,
    release: tokio::sync::oneshot::Sender<()>,
    trace: Arc<Mutex<Vec<String>>>,
    expected: Vec<String>,
}

impl Peer {
    fn new(steps: Vec<Step>) -> Self {
        let (client, server) = zbus::block_on(async {
            let (a, b) = zbus::connection::socket::Channel::pair();
            let guid = zbus::Guid::try_from("07400000000000000000000000000000").unwrap();
            let client = zbus::connection::Builder::authenticated_socket(a, guid.clone())
                .p2p()
                .method_timeout(LIMIT)
                .build()
                .await
                .unwrap();
            let server = zbus::connection::Builder::authenticated_socket(b, guid)
                .p2p()
                .method_timeout(LIMIT)
                .build()
                .await
                .unwrap();
            (Connection::from_async(client), server)
        });
        let expected = steps
            .iter()
            .map(|s| format!("{}:{:?}", s.member, s.args))
            .collect();
        let trace = Arc::new(Mutex::new(Vec::new()));
        let record = trace.clone();
        let connection = server.clone();
        let (release, wait) = tokio::sync::oneshot::channel();
        let (runtime, mut stream) = zbus::block_on(async {
            (
                tokio::runtime::Handle::current(),
                zbus::MessageStream::from(&connection),
            )
        });
        let task = runtime.spawn(async move {
            let mut wait = Some(wait);
            for step in steps {
                let request =
                    tokio::time::timeout(LIMIT, poll_fn(|cx| Pin::new(&mut stream).poll_next(cx)))
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap();
                assert_eq!(
                    request.header().destination().unwrap().as_str(),
                    step.destination
                );
                assert_eq!(
                    request.header().interface().unwrap().as_str(),
                    step.interface
                );
                assert_eq!(request.header().path().unwrap().as_str(), step.path);
                assert_eq!(request.header().member().unwrap().as_str(), step.member);
                let args = if step.args.len() == 1 {
                    vec![request.body().deserialize::<String>().unwrap()]
                } else {
                    let (a, b): (String, String) = request.body().deserialize().unwrap();
                    vec![a, b]
                };
                assert_eq!(args, step.args);
                record
                    .lock()
                    .unwrap()
                    .push(format!("{}:{:?}", step.member, args));
                let header = request.header();
                let response = match step.reply {
                    Reply::Text(value) => Message::method_return(&header)
                        .sender(step.sender)
                        .build(&value),
                    Reply::Number(value) => Message::method_return(&header)
                        .sender(step.sender)
                        .build(&value),
                    Reply::VariantText(value) => Message::method_return(&header)
                        .sender(step.sender)
                        .build(&zbus::Variant::from(value)),
                    Reply::VariantNumber(value) => Message::method_return(&header)
                        .sender(step.sender)
                        .build(&zbus::Variant::from(value)),
                    Reply::Error(name) => Message::error(&header, name)
                        .sender(step.sender)
                        .build(&"synthetic refusal"),
                    Reply::Silent => {
                        wait.take().unwrap().await.unwrap();
                        continue;
                    }
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
            release,
            trace,
            expected,
        }
    }

    fn bus(&self) -> Proxy<'_> {
        proxy(
            &self.client,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .unwrap()
    }

    fn properties(&self) -> Proxy<'_> {
        proxy(&self.client, ":1.1", "/fixture", "org.example.Unit").unwrap()
    }

    fn finish(self) {
        let _ = self.release.send(());
        zbus::block_on(async {
            tokio::time::timeout(LIMIT, self.task)
                .await
                .unwrap()
                .unwrap();
        });
        assert_eq!(*self.trace.lock().unwrap(), self.expected);
        println!("HC074_READ_TRACE:{:?}", self.expected);
        self.client.close().unwrap();
        zbus::block_on(self.server.close()).unwrap();
    }
}

#[test]
fn manager_owner_valid_reads_in_original_order() {
    let peer = Peer::new(owner_steps(":1.1", 0, 1));
    let bus = peer.bus();
    assert_eq!(manager_owner(&bus), Ok(":1.1".into()));
    drop(bus);
    peer.finish();
}

#[test]
fn manager_identity_refuses_after_all_three_reads() {
    for (owner, uid, pid) in [("not-unique", 0, 1), (":1.1", 1, 1), (":1.1", 0, 2)] {
        println!("HC074_OWNER_IDENTITY:{owner}:{uid}:{pid}");
        let peer = Peer::new(owner_steps(owner, uid, pid));
        let bus = peer.bus();
        assert_eq!(manager_owner(&bus), Err("foreign manager owner".into()));
        drop(bus);
        peer.finish();
    }
}

#[test]
fn manager_wrong_sender_stops_at_original_read() {
    for (phase, reason) in [
        "manager owner unavailable",
        "manager user unavailable",
        "manager pid unavailable",
    ]
    .into_iter()
    .enumerate()
    {
        println!("HC074_OWNER_SENDER:{phase}");
        let mut steps = owner_steps(":1.1", 0, 1);
        steps.truncate(phase + 1);
        steps[phase].sender = ":1.99";
        let peer = Peer::new(steps);
        let bus = peer.bus();
        assert_eq!(manager_owner(&bus), Err(reason.into()));
        drop(bus);
        peer.finish();
    }
}

#[test]
fn manager_signature_and_error_refuse_without_later_reads() {
    for (phase, reply, reason) in [
        (
            0,
            Reply::Error("org.freedesktop.DBus.Error.NameHasNoOwner"),
            "manager owner unavailable",
        ),
        (1, Reply::Text("not-a-uid"), "manager user unavailable"),
        (2, Reply::Text("not-a-pid"), "manager pid unavailable"),
    ] {
        println!("HC074_OWNER_SIGNATURE:{phase}");
        let mut steps = owner_steps(":1.1", 0, 1);
        steps.truncate(phase + 1);
        steps[phase].reply = reply;
        let peer = Peer::new(steps);
        let bus = peer.bus();
        assert_eq!(manager_owner(&bus), Err(reason.into()));
        drop(bus);
        peer.finish();
    }
}

#[test]
fn property_string_u32_and_no_cache_roundtrip() {
    let peer = Peer::new(vec![
        property("ActiveState", Reply::VariantText("active")),
        property("ActiveState", Reply::VariantText("failed")),
        property("MainPID", Reply::VariantNumber(74)),
    ]);
    let proxy = peer.properties();
    assert_eq!(
        proxy.get_property::<String>("ActiveState").unwrap(),
        "active"
    );
    assert_eq!(
        proxy.get_property::<String>("ActiveState").unwrap(),
        "failed"
    );
    assert_eq!(proxy.get_property::<u32>("MainPID").unwrap(), 74);
    drop(proxy);
    peer.finish();
}

#[test]
fn property_wrong_type_body_and_error_refuse() {
    for (case, reply) in [
        Reply::VariantNumber(74),
        Reply::Text("missing-variant"),
        Reply::Error("org.freedesktop.DBus.Error.UnknownProperty"),
    ]
    .into_iter()
    .enumerate()
    {
        println!("HC074_PROPERTY_REFUSAL:{case}");
        let peer = Peer::new(vec![property("ActiveState", reply)]);
        let proxy = peer.properties();
        assert!(proxy.get_property::<String>("ActiveState").is_err());
        drop(proxy);
        peer.finish();
    }
}

#[test]
fn property_received_but_unanswered_preserves_timeout() {
    let peer = Peer::new(vec![property("ActiveState", Reply::Silent)]);
    let proxy = peer.properties();
    assert!(proxy.get_property::<String>("ActiveState").is_err());
    assert_eq!(peer.trace.lock().unwrap().len(), 1);
    drop(proxy);
    peer.finish();
}
