//! Tests for the socket every connection's I/O goes through.
//!
//! Anything here that needs a runtime is run under every one this build can make, since the
//! wrapper's whole purpose is to behave the same on all of them.

use std::{
    io::{Read, Write},
    mem::MaybeUninit,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, ToSocketAddrs},
    sync::Arc,
};
#[cfg(unix)]
use std::{
    os::{fd::OwnedFd, unix::net::UnixStream},
    sync::Mutex,
};

use ntest::timeout;

use socket2::{Domain, SockRef, Socket, Type};

#[cfg(unix)]
use super::UnixOps;
#[cfg(all(unix, any(feature = "default-rt", feature = "tokio")))]
use super::unix_socket_address;
use super::{RegisteredIo, TcpOps};
#[cfg(unix)]
use crate::runtime::{Runtime, test_runtime::TestRuntime};
use crate::{
    connection::socket::{ReadHalf, WriteHalf},
    runtime::{IoSource, test_runtime::under_every_runtime},
};

#[cfg(unix)]
#[test]
#[timeout(15000)]
fn a_partial_write_is_reported_as_success() {
    under_every_runtime(|runtime| async move {
        let (left, right) = UnixStream::pair().unwrap();
        // A send buffer far smaller than the write below is what makes the kernel take only part
        // of it.
        rustix::net::sockopt::set_socket_send_buffer_size(&left, 1024).unwrap();
        let mut socket = half(&runtime, left, UnixOps);

        let written = socket.sendmsg(&[b'z'; 64 * 1024], &[]).await.unwrap();

        assert!(written > 0, "the write reported no progress at all");
        assert!(written < 64 * 1024, "the whole write fitted after all");
        drop(right);
    });
}

#[cfg(unix)]
#[test]
#[timeout(15000)]
fn file_descriptors_cross_a_unix_socket() {
    under_every_runtime(|runtime| async move {
        let (left, right) = UnixStream::pair().unwrap();
        let mut sender = half(&runtime, left, UnixOps);
        let mut receiver = half(&runtime, right, UnixOps);

        let (read_end, mut write_end) = std::io::pipe().unwrap();
        write_end.write_all(b"through the pipe").unwrap();
        drop(write_end);
        let read_end = OwnedFd::from(read_end);

        let written = sender
            .sendmsg(b"!", &[std::os::fd::AsFd::as_fd(&read_end)])
            .await
            .unwrap();
        assert_eq!(written, 1);

        let mut byte = [0; 1];
        let (read, mut fds) = receiver.recvmsg(&mut byte).await.unwrap();
        assert_eq!(read, 1);
        assert_eq!(&byte, b"!");
        assert_eq!(fds.len(), 1, "the descriptor did not come with the byte");

        let mut through = String::new();
        std::fs::File::from(fds.remove(0))
            .read_to_string(&mut through)
            .unwrap();
        assert_eq!(through, "through the pipe");
    });
}

#[cfg(unix)]
#[test]
#[timeout(15000)]
fn readiness_written_before_registration_is_seen() {
    under_every_runtime(|runtime| async move {
        let (left, mut right) = UnixStream::pair().unwrap();
        // The bytes are there before the runtime is ever told about the socket, so a
        // registration has to find them whether it tries the read first or waits for a readiness
        // that has already happened.
        right.write_all(b"early").unwrap();
        let mut socket = half(&runtime, left, UnixOps);

        let mut buffer = [0; 5];
        let (read, _fds) = socket.recvmsg(&mut buffer).await.unwrap();

        assert_eq!(&buffer[..read], b"early");
    });
}

#[cfg(unix)]
#[test]
#[timeout(15000)]
fn a_registration_is_dropped_before_its_source() {
    let (left, peer) = UnixStream::pair().unwrap();
    peer.set_nonblocking(true).unwrap();
    let runtime = Watching {
        inner: TestRuntime::new(),
        peer: Arc::new(peer),
        log: Arc::default(),
    };
    let log = runtime.log.clone();
    let socket = half(&Runtime::from_external(runtime), left, UnixOps);

    drop(socket);

    assert_eq!(
        log.lock().unwrap().as_slice(),
        ["the source is still open"],
        "the source was closed before the registration let go of it",
    );
}

/// A TCP connect reaches the listener, and hands back a socket that no longer blocks.
#[test]
#[timeout(15000)]
fn a_tcp_connect_reaches_the_listener_in_non_blocking_mode() {
    under_every_runtime(|runtime| async move {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();

        let source = runtime
            .connect_tcp(listener.local_addr().unwrap())
            .await
            .unwrap();

        // The address the listener sees the connection come from is the socket's own.
        let (_accepted, peer) = listener.accept().unwrap();
        assert_eq!(
            SockRef::from(&source).local_addr().unwrap().as_socket(),
            Some(peer),
            "the connection reached some other socket",
        );
        assert_non_blocking(&source);
    });
}

/// A TCP connect that is turned away reports why.
#[test]
#[timeout(15000)]
fn a_refused_tcp_connect_reports_the_socket_error() {
    under_every_runtime(|runtime| async move {
        let refused = RefusedPort::on(Ipv4Addr::LOCALHOST.into());

        let error = runtime.connect_tcp(refused.address()).await.unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
    });
}

/// The socket a connect hands back is one the connection can register for its traffic.
///
/// A runtime that watched the socket to wait for the connect has to have let go of it by then, or
/// it turns the connection's registration away.
#[test]
#[timeout(15000)]
fn a_connected_socket_carries_traffic_once_registered() {
    under_every_runtime(|runtime| async move {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let source = runtime
            .connect_tcp(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (mut accepted, _) = listener.accept().unwrap();
        let mut socket = Arc::new(RegisteredIo::new(&runtime, source, TcpOps).unwrap());

        accepted.write_all(b"ping").unwrap();
        let mut buffer = [0; 4];
        let received = socket.recvmsg(&mut buffer).await.unwrap();
        // Only unix sockets bring file descriptors along.
        #[cfg(unix)]
        let (received, _fds) = received;
        assert_eq!(&buffer[..received], b"ping");

        #[cfg(unix)]
        socket.sendmsg(b"pong", &[]).await.unwrap();
        #[cfg(not(unix))]
        socket.sendmsg(b"pong").await.unwrap();
        accepted.read_exact(&mut buffer).unwrap();
        assert_eq!(&buffer, b"pong");
    });
}

/// A unix connect reaches the listener, and hands back a socket that no longer blocks.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn a_unix_connect_reaches_the_listener_in_non_blocking_mode() {
    under_every_runtime(|runtime| async move {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("socket");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();

        let source = runtime.connect_unix(&path).await.unwrap();

        // A byte written at the other end that comes out of the socket is what says the two ends
        // are joined, since neither end of a connection to a path names the other.
        let (mut accepted, _) = listener.accept().unwrap();
        accepted.write_all(b"!").unwrap();
        let mut byte = [MaybeUninit::uninit(); 1];
        assert_eq!(SockRef::from(&source).recv(&mut byte).unwrap(), 1);
        assert_non_blocking(&source);
    });
}

/// A path too long to be a socket address is the connect's error, not a panic or a hang.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn a_unix_path_too_long_for_an_address_is_the_connects_error() {
    under_every_runtime(|runtime| async move {
        let path = "a".repeat(4096);

        let error = runtime
            .connect_unix(std::path::Path::new(&path))
            .await
            .unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    });
}

/// A path that starts with a zero byte names a socket in the abstract namespace by the rest of it.
///
/// The name may hold zero bytes of its own, and has no file behind it.
#[cfg(all(target_os = "linux", any(feature = "default-rt", feature = "tokio")))]
#[test]
fn a_leading_zero_byte_makes_the_address_an_abstract_name() {
    use std::{
        ffi::OsStr,
        os::{linux::net::SocketAddrExt, unix::ffi::OsStrExt},
        path::Path,
    };

    let path = Path::new(OsStr::from_bytes(b"\0zbus\0name"));

    let address = unix_socket_address(path).unwrap();

    assert_eq!(address.as_abstract_name(), Some(&b"zbus\0name"[..]));
    assert_eq!(address.as_pathname(), None);
}

/// Any other path is the path of the file the socket is bound to.
#[cfg(all(unix, any(feature = "default-rt", feature = "tokio")))]
#[test]
fn a_path_makes_the_address_a_pathname() {
    use std::path::Path;

    let address = unix_socket_address(Path::new("/run/user/1000/bus")).unwrap();

    assert_eq!(address.as_pathname(), Some(Path::new("/run/user/1000/bus")));
}

/// A zero byte anywhere but the start of a path, and a path too long for an address, are invalid
/// input.
#[cfg(all(unix, any(feature = "default-rt", feature = "tokio")))]
#[test]
fn a_path_that_cannot_be_an_address_is_invalid_input() {
    use std::path::Path;

    for path in [Path::new("/run/\0bus"), Path::new(&"a".repeat(4096))] {
        let error = unix_socket_address(path).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }
}

/// A name in the abstract namespace is reached by a path that starts with a zero byte.
#[cfg(target_os = "linux")]
#[test]
#[timeout(15000)]
fn a_unix_connect_reaches_an_abstract_socket() {
    use std::{
        ffi::OsString,
        os::{
            linux::net::SocketAddrExt,
            unix::{
                ffi::OsStringExt,
                net::{SocketAddr, UnixListener},
            },
        },
        path::PathBuf,
    };

    under_every_runtime(|runtime| async move {
        // A name of this process's own: the listener is gone before the next runtime's turn, but
        // another test binary may well be running beside this one.
        let name = format!("zbus-test-unix-connect-{}", std::process::id());
        let listener =
            UnixListener::bind_addr(&SocketAddr::from_abstract_name(&name).unwrap()).unwrap();
        let mut path = vec![0];
        path.extend_from_slice(name.as_bytes());

        let source = runtime
            .connect_unix(&PathBuf::from(OsString::from_vec(path)))
            .await
            .unwrap();

        listener.accept().unwrap();
        let peer = SockRef::from(&source).peer_addr().unwrap();
        assert_eq!(peer.as_abstract_namespace(), Some(name.as_bytes()));
    });
}

/// A unix connect to a listener with no room left for it is made once the listener has some.
///
/// A blocking connect waits in the kernel for that on Linux and Android, which is what makes the
/// connect here pending in the meantime. Elsewhere the kernel turns the connection away instead.
#[cfg(any(target_os = "android", target_os = "linux"))]
#[test]
#[timeout(15000)]
fn a_unix_connect_to_a_full_backlog_is_made_once_the_listener_accepts() {
    use socket2::SockAddr;

    under_every_runtime(|runtime| async move {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("socket");
        let listener = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
        listener.bind(&SockAddr::unix(&path).unwrap()).unwrap();
        listener.listen(0).unwrap();

        // The listener's one place is taken from here on, so a further connection has to wait
        // until it accepts this one.
        let _queued = runtime.connect_unix(&path).await.unwrap();
        let mut pending = std::pin::pin!(runtime.connect_unix(&path));
        assert!(
            futures_lite::future::poll_once(pending.as_mut())
                .await
                .is_none(),
            "the connection was made before the listener had room for it",
        );

        let _accepted = listener.accept().unwrap();
        let source = pending.await.unwrap();

        assert!(
            SockRef::from(&source).peer_addr().is_ok(),
            "the connection was reported without a peer at the other end",
        );
    });
}

/// Asserts that `source` is in non-blocking mode.
///
/// A read on a socket with nothing to read is how it shows: a socket that blocks would wait for
/// something that never comes, and the test's timeout is what ends it.
fn assert_non_blocking(source: &IoSource) {
    let error = SockRef::from(source)
        .recv(&mut [MaybeUninit::uninit(); 1])
        .unwrap_err();

    assert_eq!(
        error.kind(),
        std::io::ErrorKind::WouldBlock,
        "the connected socket is not in non-blocking mode",
    );
}

/// A loopback TCP port that refuses every connection made to it.
///
/// Only a port nothing listens on can refuse a connection, and a test needs one that no other
/// test can be handed while it runs. A socket that is bound and never listened on is both: the
/// kernel answers a connection to it with a reset, and the port is this value's until it is
/// dropped.
pub(crate) struct RefusedPort {
    // Holding the bound sockets is what holds the port; nothing else is ever done with them.
    _bound: Vec<Socket>,
    addresses: Vec<SocketAddr>,
}

impl RefusedPort {
    /// A refused port on the loopback address of one family.
    pub(crate) fn on(loopback: IpAddr) -> Self {
        Self::reserve(&[loopback])
    }

    /// A refused port on every address `host` resolves to.
    ///
    /// A name can stand for an address of either family, and a connection to it tries every
    /// address the resolver hands back, so all of them have to refuse it.
    pub(crate) fn for_host(host: &str) -> Self {
        let resolved = (host, 0)
            .to_socket_addrs()
            .unwrap_or_else(|e| panic!("`{host}` resolves to nothing: {e}"));
        let mut addresses = Vec::new();
        for address in resolved {
            if !addresses.contains(&address.ip()) {
                addresses.push(address.ip());
            }
        }

        Self::reserve(&addresses)
    }

    /// The address a connection is refused at, in the family reserved first.
    pub(crate) fn address(&self) -> SocketAddr {
        self.addresses[0]
    }

    /// The port, which is the same number in every family it was reserved in.
    pub(crate) fn port(&self) -> u16 {
        self.address().port()
    }

    /// One port, held in the family of every address in `addresses`.
    fn reserve(addresses: &[IpAddr]) -> Self {
        for _ in 0..PORT_ATTEMPTS {
            if let Some(reserved) = Self::attempt(addresses) {
                return reserved;
            }
        }

        panic!("no ephemeral port was free in every family of {addresses:?}");
    }

    /// One try at holding the same port in the family of every address in `addresses`.
    ///
    /// The kernel picks the number for the first address and the rest ask for that same one, so a
    /// number already taken in one of their families gives up on the try rather than on the whole
    /// reservation: another number may well be free in all of them.
    fn attempt(addresses: &[IpAddr]) -> Option<Self> {
        let (first, rest) = addresses
            .split_first()
            .expect("a port is reserved on at least one address");
        let first = SocketAddr::new(*first, 0);
        let first = bound(first)
            .unwrap_or_else(|e| panic!("cannot bind an ephemeral port on {first}: {e}"));
        let port = first
            .local_addr()
            .expect("a bound socket reports the address it was given")
            .as_socket()
            .expect("a bound TCP socket has an IP address")
            .port();

        let mut sockets = vec![first];
        for address in rest {
            let address = SocketAddr::new(*address, port);
            match bound(address) {
                Ok(socket) => sockets.push(socket),
                // This family has the number taken, and another number may be free in all of
                // them; anything else is not about the number at all.
                Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => return None,
                Err(e) => panic!("`{address}` cannot be reserved: {e}"),
            }
        }

        Some(Self {
            addresses: addresses
                .iter()
                .map(|address| SocketAddr::new(*address, port))
                .collect(),
            _bound: sockets,
        })
    }
}

/// How many ports a reservation tries before it gives up.
///
/// Every try needs one number that is free in the family of each address involved, which a port
/// taken in one of them can deny; a second number is already unlikely to be needed.
const PORT_ATTEMPTS: usize = 8;

/// A TCP socket bound to `address` and listened on by nobody.
fn bound(address: SocketAddr) -> std::io::Result<Socket> {
    let socket = Socket::new(Domain::for_address(address), Type::STREAM, None)?;

    socket.bind(&address.into()).map(|()| socket)
}

/// `stream`, registered on `runtime` as one half of a socket with `ops`.
#[cfg(unix)]
fn half<S, O>(runtime: &Runtime, stream: S, ops: O) -> Arc<RegisteredIo<O>>
where
    S: Into<OwnedFd>,
    O: super::SocketOps,
{
    Arc::new(super::registered(runtime, stream, ops).unwrap())
}

/// A runtime whose registrations keep no source of their own.
///
/// That is what lets a test see when the source a [`RegisteredIo`] holds is released: the peer of
/// the socket pair only reads end-of-file once the last owner of the descriptor has let go.
#[cfg(unix)]
#[derive(Clone, Debug)]
struct Watching {
    inner: TestRuntime,
    peer: Arc<UnixStream>,
    log: Arc<Mutex<Vec<&'static str>>>,
}

#[cfg(unix)]
impl crate::runtime::traits::Runtime for Watching {
    type RegisteredIoSource = WatchingRegistration;
    type Sleep = <TestRuntime as crate::runtime::traits::Runtime>::Sleep;
    type Task<T>
        = <TestRuntime as crate::runtime::traits::Runtime>::Task<T>
    where
        T: Send + 'static;

    fn register_io_source(&self, source: IoSource) -> std::io::Result<WatchingRegistration> {
        drop(source);

        Ok(WatchingRegistration {
            peer: self.peer.clone(),
            log: self.log.clone(),
        })
    }

    fn sleep(&self, duration: std::time::Duration) -> Self::Sleep {
        crate::runtime::traits::Runtime::sleep(&self.inner, duration)
    }

    fn spawn<T>(
        &self,
        name: &str,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Self::Task<T>
    where
        T: Send + 'static,
    {
        crate::runtime::traits::Runtime::spawn(&self.inner, name, future)
    }

    fn connect_tcp(
        &self,
        address: SocketAddr,
    ) -> impl Future<Output = std::io::Result<IoSource>> + Send {
        crate::runtime::traits::Runtime::connect_tcp(&self.inner, address)
    }

    fn connect_unix(
        &self,
        path: &std::path::Path,
    ) -> impl Future<Output = std::io::Result<IoSource>> + Send {
        crate::runtime::traits::Runtime::connect_unix(&self.inner, path)
    }

    fn spawn_process(
        &self,
        command: std::process::Command,
        stdin: std::process::Stdio,
        stdout: std::process::Stdio,
        stderr: std::process::Stdio,
    ) -> std::io::Result<
        crate::runtime::erased::BoxFuture<'static, std::io::Result<std::process::ExitStatus>>,
    > {
        crate::runtime::traits::Runtime::spawn_process(&self.inner, command, stdin, stdout, stderr)
    }
}

/// A registration that records, as it goes, whether the source it was given is still open.
#[cfg(unix)]
#[derive(Debug)]
struct WatchingRegistration {
    peer: Arc<UnixStream>,
    log: Arc<Mutex<Vec<&'static str>>>,
}

#[cfg(unix)]
impl crate::runtime::traits::PollIo for WatchingRegistration {
    fn poll_io<T>(
        &self,
        _cx: &mut std::task::Context<'_>,
        _interest: crate::runtime::Interest,
        mut operation: impl FnMut() -> std::io::Result<T>,
    ) -> std::task::Poll<std::io::Result<T>> {
        std::task::Poll::Ready(operation())
    }
}

#[cfg(unix)]
impl Drop for WatchingRegistration {
    fn drop(&mut self) {
        // A closed source makes the peer of the socket pair read end-of-file; while it is open
        // the non-blocking read finds nothing to return instead.
        let closed = matches!((&*self.peer).read(&mut [0; 1]), Ok(0));

        self.log.lock().unwrap().push(if closed {
            "the source is closed"
        } else {
            "the source is still open"
        });
    }
}

/// A peer's supplementary groups are looked up through the runtime, and only where zbus makes
/// that lookup.
///
/// The peer of a socket pair is this process itself, so what the lookup finds is of no interest
/// here; what is, is whether the runtime was handed any blocking work to find it.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn a_group_lookup_reaches_the_runtime_only_where_there_is_one() {
    let counting = TestRuntime::new();
    let (left, _peer) = UnixStream::pair().unwrap();
    let mut socket = half(&Runtime::from_external(counting.clone()), left, UnixOps);

    futures_lite::future::block_on(ReadHalf::peer_credentials(&mut socket)).unwrap();

    // zbus looks supplementary groups up on Linux and Android only, and one call is what that
    // takes.
    let expected = usize::from(cfg!(any(target_os = "android", target_os = "linux")));
    assert_eq!(counting.blocking_calls(), expected);
}
