//! Private test bus only; never a fixed manager or original-start capability.
use super::*;
use std::future::{poll_fn, Future};
use std::io::{self, Write};
use std::os::fd::{BorrowedFd, OwnedFd};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use zbus::connection::socket::{BoxedSplit, ReadHalf, Split, WriteHalf};
use zbus::export::{async_trait::async_trait, futures_core::Stream};
use zbus::{Connection, MatchRule, MessageStream};

// These are harness fail-safes, never original admission deadlines or time origins.
async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .expect("private bus infrastructure timeout")
}
#[derive(Debug, Default)]
struct Wire {
    incoming: Vec<u8>,
    outgoing: Vec<u8>,
    writes: Vec<(usize, usize)>,
    fragment: bool,
}
fn prefix(target: &mut Vec<u8>, bytes: &[u8]) {
    let count = bytes.len().min(1024usize.saturating_sub(target.len()));
    target.extend_from_slice(&bytes[..count]);
}
#[derive(Debug)]
struct Reader(OwnedReadHalf, Arc<Mutex<Wire>>, bool);
#[async_trait]
impl ReadHalf for Reader {
    async fn recvmsg(&mut self, bytes: &mut [u8]) -> io::Result<(usize, Vec<OwnedFd>)> {
        let (count, fds) = self.0.recvmsg(bytes).await?;
        prefix(&mut self.1.lock().unwrap().incoming, &bytes[..count]);
        Ok((count, fds))
    }
    fn can_pass_unix_fd(&self) -> bool {
        self.2
    }
    async fn peer_credentials(&mut self) -> io::Result<zbus::fdo::ConnectionCredentials> {
        ReadHalf::peer_credentials(&mut self.0).await
    }
}
#[derive(Debug)]
struct Writer(OwnedWriteHalf, Arc<Mutex<Wire>>, bool);
#[async_trait]
impl WriteHalf for Writer {
    async fn sendmsg(&mut self, bytes: &[u8], fds: &[BorrowedFd<'_>]) -> io::Result<usize> {
        let max = if self.1.lock().unwrap().fragment {
            17
        } else {
            bytes.len()
        };
        let count = self.0.sendmsg(&bytes[..bytes.len().min(max)], fds).await?;
        let mut wire = self.1.lock().unwrap();
        prefix(&mut wire.outgoing, &bytes[..count]);
        wire.writes.push((count, fds.len()));
        Ok(count)
    }
    async fn close(&mut self) -> io::Result<()> {
        self.0.close().await
    }
    fn can_pass_unix_fd(&self) -> bool {
        self.2
    }
    async fn peer_credentials(&mut self) -> io::Result<zbus::fdo::ConnectionCredentials> {
        WriteHalf::peer_credentials(&mut self.0).await
    }
}
struct PrivateBus {
    child: Child,
    socket: std::path::PathBuf,
    _root: tempfile::TempDir,
}
impl Drop for PrivateBus {
    fn drop(&mut self) {
        // Only the exact unreaped child owned by this fixture, never a service/PID search.
        let _ = self.child.kill();
        let status = self.child.wait().expect("reap owned private bus");
        assert!(
            !status.success(),
            "private bus unexpectedly exited normally"
        );
    }
}
impl PrivateBus {
    fn spawn() -> Self {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        let root = tempfile::Builder::new()
            .prefix("hc074-private-bus-")
            .tempdir()
            .unwrap();
        assert_eq!(
            std::fs::metadata(root.path()).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let socket = root.path().join("bus");
        let config = root.path().join("bus.conf");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&config)
            .unwrap();
        write!(
            file,
            "<busconfig><type>session</type><listen>unix:path={}</listen>\
            <auth>EXTERNAL</auth><policy context=\"default\"><allow user=\"*\"/>\
            <allow send_destination=\"*\"/><allow receive_sender=\"*\"/>\
            </policy></busconfig>",
            socket.display()
        )
        .unwrap();
        drop(file);
        let child = Command::new("/usr/bin/dbus-daemon")
            .arg("--nofork")
            .arg("--nopidfile")
            .arg("--config-file")
            .arg(config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("explicit private dbus-daemon required");
        Self {
            child,
            socket,
            _root: root,
        }
    }
    async fn connect(&mut self, capable: bool) -> (Connection, Arc<Mutex<Wire>>) {
        // Only startup readiness is polled. A matrix failure is never retried.
        bounded(async {
            while !self.socket.exists() {
                assert!(
                    self.child.try_wait().unwrap().is_none(),
                    "private bus died during startup"
                );
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
        let socket = bounded(tokio::net::UnixStream::connect(&self.socket))
            .await
            .unwrap();
        let peer = socket.peer_cred().unwrap();
        assert_eq!(peer.pid(), Some(self.child.id() as i32));
        assert_eq!(peer.uid(), unsafe { libc::geteuid() });
        assert_eq!(peer.gid(), unsafe { libc::getegid() });
        let wire = Arc::new(Mutex::new(Wire::default()));
        let (read, write) = socket.into_split();
        let socket: BoxedSplit = Split::new(
            Box::new(Reader(read, wire.clone(), capable)),
            Box::new(Writer(write, wire.clone(), capable)),
        );
        let connection = bounded(
            zbus::connection::Builder::socket(socket)
                .auth_mechanism(zbus::AuthMechanism::External)
                .max_queued(16)
                .method_timeout(Duration::from_secs(2))
                .build(),
        )
        .await
        .unwrap();
        assert!(connection.is_bus());
        let wire_view = wire.lock().unwrap();
        let outgoing = String::from_utf8_lossy(&wire_view.outgoing);
        let incoming = String::from_utf8_lossy(&wire_view.incoming);
        assert!(outgoing.starts_with("\0AUTH EXTERNAL "));
        assert!(incoming.starts_with("OK "));
        assert_eq!(outgoing.contains("NEGOTIATE_UNIX_FD\r\n"), capable);
        assert_eq!(incoming.contains("AGREE_UNIX_FD\r\n"), capable);
        drop(wire_view);
        (connection, wire)
    }
}
async fn messages(connection: &Connection, kind: Type) -> MessageStream {
    bounded(MessageStream::for_match_rule(
        MatchRule::builder().msg_type(kind).build(),
        connection,
        Some(8),
    ))
    .await
    .unwrap()
}
async fn next(stream: &mut MessageStream) -> Message {
    bounded(poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)))
        .await
        .expect("private bus closed")
        .unwrap()
}
async fn absent(stream: &mut MessageStream) {
    assert!(tokio::time::timeout(
        Duration::from_millis(100),
        poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx))
    )
    .await
    .is_err());
}
fn request(writers: [&File; 2], manager: &str) -> Message {
    let legacy = request_kind(writers, "ok");
    type Body<'a> = (
        &'a str,
        &'a str,
        Vec<(&'a str, Value<'a>)>,
        Vec<(&'a str, Vec<(&'a str, Value<'a>)>)>,
    );
    let body = legacy.body();
    let body: Body<'_> = body.deserialize().unwrap();
    Message::method_call(PATH, METHOD)
        .unwrap()
        .destination(manager)
        .unwrap()
        .interface(INTERFACE)
        .unwrap()
        .serial(legacy.primary_header().serial_num())
        .build(&body)
        .unwrap()
}
fn held(reader: &mut File) {
    assert_eq!(
        reader.read(&mut [0; 1]).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}
fn write_original(received: &Message, readers: &mut [File; 2]) {
    for (index, (fd, reader)) in received.data().fds().iter().zip(readers).enumerate() {
        let byte = [b'A' + index as u8];
        assert_eq!(
            unsafe { libc::write(fd.as_raw_fd(), byte.as_ptr().cast(), 1) },
            1
        );
        let mut seen = [0];
        assert_eq!(reader.read(&mut seen).unwrap(), 1);
        assert_eq!(seen, byte);
    }
}
async fn scenario(case: &str) {
    let mut bus = PrivateBus::spawn();
    let (client, wire) = bus.connect(case != "no-capability").await;
    let (manager, _) = bus.connect(true).await;
    let manager_name = manager.unique_name().unwrap().to_string();
    let client_name = client.unique_name().unwrap().to_string();
    let mut incoming = messages(&manager, Type::MethodCall).await;
    let mut returns = messages(&client, Type::MethodReturn).await;
    let mut errors = messages(&client, Type::Error).await;
    let [out, out_w] = pipe();
    let [err, err_w] = pipe();
    let mut readers = [out, err];
    let request = request([&out_w, &err_w], &manager_name);
    let mut exchange = DispatchExchange::prepare(request, &manager_name, &client_name).unwrap();
    drop(out_w);
    drop(err_w);
    {
        let mut wire = wire.lock().unwrap();
        wire.writes.clear();
        wire.fragment = case == "fragmented";
    }
    exchange.arm().unwrap();
    assert_eq!(exchange.phase, Phase::Uncertain);
    let sent = bounded(client.send(&exchange.request)).await;
    if case == "no-capability" {
        assert!(matches!(sent, Err(zbus::Error::Unsupported)));
        exchange.lose_reply();
        assert!(wire.lock().unwrap().writes.is_empty());
        absent(&mut incoming).await;
        assert_eq!(exchange.arm(), Err(ExchangeError::AlreadyAttempted));
        for reader in &mut readers {
            held(reader);
        }
        drop(exchange);
        for reader in &mut readers {
            assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
        }
        bounded(client.close()).await.unwrap();
        bounded(manager.close()).await.unwrap();
        println!("HC074-NEGOTIATED case={case} result=ok sends=0 retry=false original=false");
        return;
    }
    sent.unwrap();
    let received = next(&mut incoming).await;
    assert_eq!(received.header().sender().unwrap().as_str(), client_name);
    assert_eq!(
        received.header().destination().unwrap().as_str(),
        manager_name
    );
    assert_eq!(received.primary_header().serial_num(), exchange.serial);
    assert_eq!(
        received.body().data().bytes(),
        exchange.request.body().data().bytes()
    );
    assert_eq!(received.header().unix_fds(), Some(2));
    assert_eq!(received.data().fds().len(), 2);
    for (fd, original) in received
        .data()
        .fds()
        .iter()
        .zip(exchange.request.data().fds())
    {
        assert_eq!(
            writer_shape(fd.as_raw_fd()).unwrap(),
            writer_shape(original.as_raw_fd()).unwrap()
        );
    }
    {
        let wire = wire.lock().unwrap();
        assert_eq!(
            wire.writes.iter().map(|(n, _)| n).sum::<usize>(),
            exchange.request.data().bytes().len()
        );
        assert_eq!(wire.writes.iter().filter(|(_, fds)| *fds != 0).count(), 1);
        assert_eq!(wire.writes[0].1, 2);
        assert!(wire.writes.iter().all(|(n, _)| *n > 0));
        if case == "fragmented" {
            assert!(wire.writes.len() > 1);
            assert!(wire.writes.iter().all(|(n, _)| *n <= 17));
        }
    }
    write_original(&received, &mut readers);
    if case == "disconnect" {
        bounded(manager.close()).await.unwrap();
        absent(&mut returns).await;
        exchange.lose_reply();
    } else {
        let mut builder = if case == "manager-error" {
            Message::error(&received.header(), "org.freedesktop.systemd1.UnitExists").unwrap()
        } else {
            Message::method_return(&received.header()).unwrap()
        };
        if case == "wrong-serial" {
            builder = builder.reply_serial(Some(
                NonZeroU32::new(exchange.serial.get().checked_add(1).unwrap()).unwrap(),
            ));
        }
        let response = if case == "manager-error" {
            builder.build(&"private-sensitive-detail").unwrap()
        } else {
            builder
                .build(&OwnedObjectPath::try_from("/org/freedesktop/systemd1/job/74").unwrap())
                .unwrap()
        };
        if case == "lost-ack" {
            absent(&mut returns).await;
            exchange.lose_reply();
        }
        let foreign = if case == "foreign-sender" {
            Some(bus.connect(true).await.0)
        } else {
            None
        };
        bounded(foreign.as_ref().unwrap_or(&manager).send(&response))
            .await
            .unwrap();
        let response = if case == "manager-error" {
            next(&mut errors).await
        } else {
            next(&mut returns).await
        };
        if let Some(foreign) = foreign {
            assert_eq!(
                response.header().sender().map(|v| v.as_str()),
                foreign.unique_name().map(|v| v.as_str())
            );
            assert_ne!(response.header().sender().unwrap().as_str(), manager_name);
            bounded(foreign.close()).await.unwrap();
        } else {
            assert_eq!(response.header().sender().unwrap().as_str(), manager_name);
        }
        let observation = exchange.observe(&response);
        match case {
            "healthy" | "fragmented" => {
                assert_eq!(
                    observation.unwrap().path.as_str(),
                    "/org/freedesktop/systemd1/job/74"
                );
                assert_eq!(exchange.phase, Phase::JobObserved);
                assert_eq!(
                    exchange.observe(&response).unwrap_err(),
                    ExchangeError::Reply
                );
            }
            "manager-error" => {
                let error = observation.unwrap_err();
                assert_eq!(error, ExchangeError::ManagerError);
                assert!(!format!("{error:?}").contains("sensitive"));
            }
            _ => assert_eq!(observation.unwrap_err(), ExchangeError::Reply),
        }
        bounded(manager.close()).await.unwrap();
    }
    assert_eq!(exchange.arm(), Err(ExchangeError::AlreadyAttempted));
    drop(exchange);
    bounded(client.close()).await.unwrap();
    drop(incoming);
    drop(returns);
    drop(errors);
    // A remote alias outlives its connection and the exchange. It still writes.
    for reader in &mut readers {
        held(reader);
    }
    write_original(&received, &mut readers);
    drop(received);
    for reader in &mut readers {
        assert_eq!(reader.read(&mut [0; 1]).unwrap(), 0);
    }
    println!("HC074-NEGOTIATED case={case} result=ok sends=1 retry=false retained_writer=true original=false");
}
fn run_private_bus_matrix() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(60), async {
            // No scenario retries. Names, credentials and each daemon are freshly observed.
            for case in [
                "healthy",
                "fragmented",
                "lost-ack",
                "foreign-sender",
                "wrong-serial",
                "disconnect",
                "manager-error",
                "no-capability",
            ] {
                scenario(case).await;
            }
        })
        .await
        .expect("private bus whole-matrix fail-safe");
    });
}
#[test]
#[ignore = "manual nonroot private dbus-daemon negotiation; no system bus or execution"]
fn dispatch_negotiated_private_bus_fault_matrix() {
    run_private_bus_matrix();
}
