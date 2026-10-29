//! Private socketpair, real SCM_RIGHTS; no system bus or original-start authority.
use crate::bridge::{Connection, Proxy};
use std::{
    fs::{self, File, OpenOptions},
    future::poll_fn,
    io::{Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsFd, AsRawFd, OwnedFd},
        unix::{fs::MetadataExt, net::UnixStream},
    },
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use zbus::{export::futures_core::Stream, message::Message};

const LIMIT: Duration = Duration::from_millis(250);
const PATH: &str = "/org/hydracache/Fixture";
const IFACE: &str = "org.hydracache.Fixture";
static NEXT: AtomicUsize = AtomicUsize::new(0);

fn pair() -> (Connection, zbus::Connection, zbus::MessageStream) {
    let (a, b) = UnixStream::pair().unwrap();
    zbus::block_on(async {
        let runtime = tokio::runtime::Handle::current();
        let server = runtime.spawn(async move {
            tokio::time::timeout(
                LIMIT,
                zbus::connection::Builder::unix_stream(a)
                    .server("07400000000000000000000000000000")
                    .p2p()
                    .auth_mechanism(zbus::connection::AuthMechanism::External)
                    .method_timeout(LIMIT)
                    .build_message_stream(),
            )
            .await
            .unwrap()
            .unwrap()
        });
        let client = tokio::time::timeout(
            LIMIT,
            zbus::connection::Builder::unix_stream(b)
                .p2p()
                .auth_mechanism(zbus::connection::AuthMechanism::External)
                .method_timeout(LIMIT)
                .build(),
        )
        .await
        .unwrap()
        .unwrap();
        let stream = tokio::time::timeout(LIMIT, server).await.unwrap().unwrap();
        (
            Connection::from_async(client),
            zbus::Connection::from(&stream),
            stream,
        )
    })
}

async fn request(stream: &mut zbus::MessageStream, member: &str) -> Message {
    let msg = tokio::time::timeout(LIMIT, poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)))
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(msg.header().destination().unwrap().as_str(), ":1.1");
    assert_eq!(msg.header().path().unwrap().as_str(), PATH);
    assert_eq!(msg.header().interface().unwrap().as_str(), IFACE);
    assert_eq!(msg.header().member().unwrap().as_str(), member);
    msg
}

fn join(task: tokio::task::JoinHandle<()>) {
    zbus::block_on(async { tokio::time::timeout(LIMIT, task).await.unwrap().unwrap() });
}
fn close(client: Connection, server: zbus::Connection) {
    client.close().unwrap();
    zbus::block_on(server.close()).unwrap();
}
fn cloexec(fd: i32) {
    let info = fs::read_to_string(format!("/proc/self/fdinfo/{fd}")).unwrap();
    let flags = info
        .lines()
        .find_map(|v| v.strip_prefix("flags:\t"))
        .unwrap();
    assert_ne!(u64::from_str_radix(flags, 8).unwrap() & 0o2000000, 0);
}
struct Fixture {
    file: File,
    path: PathBuf,
    identity: (u64, u64),
}
impl Fixture {
    fn new() -> Self {
        let root =
            PathBuf::from(std::env::var_os("HC074_FD_ROOT").expect("owned fixture directory"));
        let path = root.join(format!(
            "fd-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let m = file.metadata().unwrap();
        Self {
            file,
            path,
            identity: (m.dev(), m.ino()),
        }
    }
    fn aliases(&self) -> usize {
        fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(|p| match fs::metadata(p.unwrap().path()) {
                Ok(m) => Some((m.dev(), m.ino())),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => panic!("FD census: {e}"),
            })
            .filter(|v| *v == self.identity)
            .count()
    }
    fn clean(&self) {
        assert_eq!(self.aliases(), 1, "received/outgoing descriptor leaked");
        let m = fs::metadata(&self.path).unwrap();
        assert_eq!((m.dev(), m.ino()), self.identity);
        fs::remove_file(&self.path).unwrap();
    }
}

#[test]
fn authenticated_unix_pair_reports_actual_peer() {
    let (client, server, stream) = pair();
    let uid = fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|v| v.strip_prefix("Uid:\t"))
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse::<u32>()
        .unwrap();
    zbus::block_on(async {
        for c in [client.inner(), &server] {
            let creds = c.peer_creds().await.unwrap();
            assert_eq!(creds.unix_user_id(), Some(uid));
            assert_eq!(creds.process_id(), Some(std::process::id()));
        }
        zbus::AsyncDrop::async_drop(stream).await;
    });
    close(client, server);
    println!("HC074_UNIX:credentials:kernel-socketpair");
}

fn roundtrip(payload: Vec<u8>) {
    let mut f = Fixture::new();
    f.file.write_all(b"abcdef").unwrap();
    f.file.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(f.aliases(), 1);
    let (client, server, mut stream) = pair();
    let other = server.clone();
    let identity = f.identity;
    let task = zbus::block_on(async { tokio::runtime::Handle::current() }).spawn(async move {
        let msg = request(&mut stream, "Roundtrip").await;
        assert_eq!(msg.data().fds().len(), 1);
        let (fd, bytes): (zbus::OwnedFd, Vec<u8>) = msg.body().deserialize().unwrap();
        let mut file = File::from(OwnedFd::from(fd));
        let m = file.metadata().unwrap();
        assert_eq!((m.dev(), m.ino()), identity);
        cloexec(file.as_raw_fd());
        let mut prefix = [0u8; 2];
        file.read_exact(&mut prefix).unwrap();
        assert_eq!(&prefix, b"ab");
        let reply = Message::method_return(&msg.header())
            .sender(":1.1")
            .build(&(zbus::Fd::from(file.as_fd()), bytes))
            .unwrap();
        other.send(&reply).await.unwrap();
        drop(reply);
        drop(file);
        drop(msg);
        zbus::AsyncDrop::async_drop(stream).await;
    });
    let (fd, actual): (zbus::OwnedFd, Vec<u8>) = {
        let proxy = Proxy::new(&client, ":1.1", PATH, IFACE).unwrap();
        proxy
            .call(
                "Roundtrip",
                &(zbus::Fd::from(f.file.as_fd()), payload.clone()),
            )
            .unwrap()
    };
    let mut received = File::from(OwnedFd::from(fd));
    cloexec(received.as_raw_fd());
    let m = received.metadata().unwrap();
    assert_eq!((m.dev(), m.ino()), f.identity);
    let mut next = [0u8; 2];
    received.read_exact(&mut next).unwrap();
    assert_eq!(&next, b"cd");
    assert_eq!(f.file.stream_position().unwrap(), 4);
    assert_eq!(actual, payload);
    drop(received);
    join(task);
    close(client, server);
    f.clean();
    println!("HC074_UNIX:roundtrip:bytes={}:aliases=1", actual.len());
}
#[test]
fn descriptor_roundtrip_is_original_and_cloexec() {
    roundtrip(vec![0, 74, 255]);
}
#[test]
fn large_body_and_descriptor_survive_roundtrip() {
    roundtrip((0..262144).map(|n| (n % 251) as u8).collect());
}

#[test]
fn wrong_reply_type_drops_received_descriptor() {
    let f = Fixture::new();
    let held = f.file.try_clone().unwrap();
    let (client, server, mut stream) = pair();
    let other = server.clone();
    let task = zbus::block_on(async { tokio::runtime::Handle::current() }).spawn(async move {
        let msg = request(&mut stream, "WrongType").await;
        let _: () = msg.body().deserialize().unwrap();
        let reply = Message::method_return(&msg.header())
            .sender(":1.1")
            .build(&zbus::Fd::from(held.as_fd()))
            .unwrap();
        other.send(&reply).await.unwrap();
        drop(reply);
        drop(held);
        drop(msg);
        zbus::AsyncDrop::async_drop(stream).await;
    });
    let result: zbus::Result<String> = {
        let p = Proxy::new(&client, ":1.1", PATH, IFACE).unwrap();
        p.call("WrongType", &())
    };
    assert!(result.is_err());
    join(task);
    close(client, server);
    f.clean();
    println!("HC074_UNIX:decode-refusal:aliases=1");
}

fn disconnect(effect: bool) {
    let mut f = Fixture::new();
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    let (client, server, mut stream) = pair();
    let other = server.clone();
    let task = zbus::block_on(async { tokio::runtime::Handle::current() }).spawn(async move {
        let msg = request(&mut stream, "Disconnect").await;
        let fd: zbus::OwnedFd = msg.body().deserialize().unwrap();
        let mut file = File::from(OwnedFd::from(fd));
        seen.fetch_add(1, Ordering::SeqCst);
        if effect {
            file.write_all(b"effect").unwrap();
        }
        drop(file);
        drop(msg);
        zbus::AsyncDrop::async_drop(stream).await;
        other.close().await.unwrap();
    });
    let result: zbus::Result<u32> = {
        let p = Proxy::new(&client, ":1.1", PATH, IFACE).unwrap();
        p.call("Disconnect", &zbus::Fd::from(f.file.as_fd()))
    };
    let error = result.unwrap_err();
    assert!(matches!(error, zbus::Error::InputOutput(_)), "{error:?}");
    join(task);
    close(client, server);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    f.file.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    f.file.read_to_end(&mut bytes).unwrap();
    assert_eq!(
        bytes,
        if effect {
            b"effect".as_slice()
        } else {
            b"".as_slice()
        }
    );
    f.clean();
    println!("HC074_UNIX:disconnect:effect={effect}:requests=1:aliases=1:error={error:?}");
}
#[test]
fn disconnect_after_request_refuses() {
    disconnect(false);
}
#[test]
fn lost_ack_keeps_single_effect_without_replay() {
    disconnect(true);
}
