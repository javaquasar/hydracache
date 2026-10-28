//! The UDP socket, [`UdpSocket`].

#[cfg(any(unix, target_os = "wasi"))]
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, AsSocket, BorrowedSocket, RawSocket};
use std::{
    fmt, io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr},
};

use crate::{AsyncIo, Local, Mode, Runtime};

/// A UDP socket, to send datagrams from and receive datagrams on.
///
/// Create a socket with [`UdpSocket::bind`]. It receives the datagrams sent to its address.
/// [`send_to`](UdpSocket::send_to) sends a datagram to any address, and
/// [`recv_from`](UdpSocket::recv_from) receives one together with the address it came from.
///
/// A socket can be [connected](UdpSocket::connect) to a peer. Then [`send`](UdpSocket::send) sends
/// to that peer with no address to pass, and the socket receives only the datagrams that peer
/// sends. [`recv`](UdpSocket::recv) returns a datagram without the address it came from.
///
/// A datagram is one message, sent and received whole. Addresses are socket addresses, never host
/// names; see [`UdpSocket::bind`]. The socket is the async counterpart of [`std::net::UdpSocket`]:
/// an operation that has to wait lets other tasks run instead of blocking the thread.
///
/// The socket runs on the runtime passed to its constructor. Its type carries that runtime's
/// flavour, as the [module documentation](super) explains: a `UdpSocket<Local>` (the default) stays
/// on the thread that made it, and a `UdpSocket<Shared>` can be sent to and used from any thread.
///
/// Any number of tasks can wait to receive (with `recv`, `recv_from`, `peek` or `peek_from`) or to
/// send (with `send` or `send_to`) at once, each through a reference to the socket. Each receive
/// takes a datagram of its own, so tasks that receive together get one each. A peek leaves the
/// datagram on the socket for the next receive.
///
/// # Example
///
/// Two sockets: one sends a greeting to the other, which receives it together with the address it
/// came from.
///
/// ```
/// use std::net::Ipv4Addr;
///
/// use zruntime::{LocalRuntime, net::UdpSocket};
///
/// let runtime = LocalRuntime::new()?;
/// // Port `0` lets the system pick a free port.
/// let sender = UdpSocket::bind(&runtime, (Ipv4Addr::LOCALHOST, 0))?;
/// let receiver = UdpSocket::bind(&runtime, (Ipv4Addr::LOCALHOST, 0))?;
///
/// runtime.block_on(async {
///     sender.send_to(b"hello", receiver.local_addr()?).await?;
///
///     let mut greeting = [0; 16];
///     let (len, from) = receiver.recv_from(&mut greeting).await?;
///
///     assert_eq!(&greeting[..len], b"hello");
///     assert_eq!(from, sender.local_addr()?);
///     # Ok::<_, std::io::Error>(())
/// })?;
/// # Ok::<_, std::io::Error>(())
/// ```
pub struct UdpSocket<M = Local>
where
    M: Mode,
{
    io: AsyncIo<std::net::UdpSocket, M>,
}

impl<M> UdpSocket<M>
where
    M: Mode,
{
    /// Creates a socket bound to `addr`, on `runtime`.
    ///
    /// `addr` is a socket address: a [`SocketAddr`], or anything that converts into one, such as a
    /// pair of an [`Ipv4Addr`] and a port. It is never a host name; the
    /// [module documentation](super) explains what to do instead. Binding to port `0` lets the
    /// system pick a free port, which [`local_addr`](UdpSocket::local_addr) returns.
    ///
    /// The socket is bound when this returns, and receives the datagrams sent to its address from
    /// then on.
    ///
    /// # Errors
    ///
    /// Fails if the socket cannot be bound to `addr`, or for the reasons
    /// [`from_std`](UdpSocket::from_std) fails.
    ///
    /// On `wasm32-wasip1` it always fails with [`Unsupported`](io::ErrorKind::Unsupported): that
    /// target has no UDP (see the [module documentation](super)).
    pub fn bind<A>(runtime: &Runtime<M>, addr: A) -> io::Result<Self>
    where
        A: Into<SocketAddr>,
    {
        Self::from_std(runtime, std::net::UdpSocket::bind(addr.into())?)
    }

    /// Creates a socket on `runtime` from a std socket.
    ///
    /// `socket` is switched to non-blocking mode, as every socket of this type is. On unix, the
    /// mode belongs to the open socket, so a duplicate made with
    /// [`try_clone`](std::net::UdpSocket::try_clone) is switched too. A receive or send on that
    /// duplicate then fails with [`WouldBlock`](io::ErrorKind::WouldBlock) instead of waiting.
    ///
    /// # Errors
    ///
    /// Fails if switching to non-blocking mode fails, or if the runtime cannot start watching the
    /// socket. On Windows a runtime watches a limited number of sockets; see the
    /// [module documentation](super).
    ///
    /// On `wasm32-wasip1` it always fails with [`Unsupported`](io::ErrorKind::Unsupported), even
    /// on a socket that the host handed in: that target has no UDP.
    pub fn from_std(runtime: &Runtime<M>, socket: std::net::UdpSocket) -> io::Result<Self> {
        socket.set_nonblocking(true)?;

        Ok(Self {
            io: AsyncIo::from_nonblocking(runtime, socket)?,
        })
    }

    /// The socket address this socket is bound to.
    ///
    /// After binding to port `0`, this returns the port the system picked.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().local_addr()
    }

    /// The socket address of the peer this socket is connected to.
    ///
    /// # Errors
    ///
    /// Fails with [`NotConnected`](io::ErrorKind::NotConnected) if the socket is not connected; see
    /// [`connect`](UdpSocket::connect).
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.io.get_ref().peer_addr()
    }

    /// Connects this socket to `addr`.
    ///
    /// `addr` becomes the address that [`send`](UdpSocket::send) sends to. From then on the socket
    /// receives only the datagrams that address sends, and drops the datagrams that any other
    /// address sends. A datagram from another address that arrived before the connect is not
    /// dropped: it stays queued, and a receive takes it.
    ///
    /// `addr` is a socket address, never a host name; see [`bind`](UdpSocket::bind). Nothing goes
    /// over the network and nothing waits, so this is not an `async` function. The socket can be
    /// connected again, to another address.
    pub fn connect<A>(&self, addr: A) -> io::Result<()>
    where
        A: Into<SocketAddr>,
    {
        self.io.get_ref().connect(addr.into())
    }

    /// Sends `buf` as one datagram to `addr`.
    ///
    /// Returns the number of bytes sent. This is the length of `buf`: a datagram is sent whole or
    /// not at all. The send waits only if the system has no room for the datagram, until there is
    /// room.
    ///
    /// `addr` is a socket address, never a host name; see [`bind`](UdpSocket::bind).
    pub async fn send_to<A>(&self, buf: &[u8], addr: A) -> io::Result<usize>
    where
        A: Into<SocketAddr>,
    {
        let target = addr.into();

        self.io
            .write_with(|socket| socket.send_to(buf, target))
            .await
    }

    /// Waits for a datagram, and copies it into `buf`.
    ///
    /// Returns the number of bytes copied and the address the datagram came from. If the datagram
    /// is longer than `buf`, the bytes that do not fit may be discarded. On Windows the receive
    /// fails instead, with the error Winsock reports for this (`WSAEMSGSIZE`), and the whole
    /// datagram is lost.
    ///
    /// # Cancel safety
    ///
    /// Dropping the future before it completes gives up the wait. No datagram is lost: a datagram
    /// that arrives meanwhile stays queued for the next receive.
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.io.read_with(|socket| socket.recv_from(buf)).await
    }

    /// Waits for a datagram, and copies it into `buf` without removing it from the socket.
    ///
    /// Returns the number of bytes copied and the address the datagram came from. The next receive
    /// or `peek_from` returns the same datagram again. If the datagram is longer than `buf`, only
    /// as much of it as fits is copied. On Windows the peek fails instead, with the error Winsock
    /// reports for this (`WSAEMSGSIZE`), though the datagram stays queued.
    pub async fn peek_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.io.read_with(|socket| socket.peek_from(buf)).await
    }

    /// Sends `buf` as one datagram to the peer the socket is connected to.
    ///
    /// Returns the number of bytes sent. This is the length of `buf`: a datagram is sent whole or
    /// not at all. The send waits only if the system has no room for the datagram, until there is
    /// room.
    ///
    /// # Errors
    ///
    /// Fails if the socket is not connected; see [`connect`](UdpSocket::connect).
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        self.io.write_with(|socket| socket.send(buf)).await
    }

    /// Waits for a datagram, and copies it into `buf`, without the address it came from.
    ///
    /// Returns the number of bytes copied. This is [`recv_from`](UdpSocket::recv_from) without the
    /// sender's address. It suits a socket [connected](UdpSocket::connect) to a peer, because a
    /// datagram that arrives after the connect can only come from there. A datagram that arrived
    /// before the connect may have come from any address. It is returned all the same, without its
    /// address. A datagram longer than `buf` is handled as for [`recv_from`](UdpSocket::recv_from).
    ///
    /// # Cancel safety
    ///
    /// As for [`recv_from`](UdpSocket::recv_from): dropping the future gives up the wait and loses
    /// no datagram.
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.read_with(|socket| socket.recv(buf)).await
    }

    /// Waits for a datagram, and copies it into `buf` without removing it from the socket.
    ///
    /// Returns the number of bytes copied. This is [`peek_from`](UdpSocket::peek_from) without the
    /// sender's address. The next receive or peek returns the same datagram again. A datagram
    /// longer than `buf` is handled as for [`peek_from`](UdpSocket::peek_from).
    pub async fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.io.read_with(|socket| socket.peek(buf)).await
    }

    /// Whether this socket may send datagrams to a broadcast address.
    ///
    /// This is the `SO_BROADCAST` option. See [`set_broadcast`](UdpSocket::set_broadcast).
    pub fn broadcast(&self) -> io::Result<bool> {
        self.io.get_ref().broadcast()
    }

    /// Sets the `SO_BROADCAST` option of this socket: whether it may send datagrams to a broadcast
    /// address, the address that reaches every host on the local network.
    pub fn set_broadcast(&self, on: bool) -> io::Result<()> {
        self.io.get_ref().set_broadcast(on)
    }

    /// Whether the multicast datagrams this socket sends are looped back to the local host.
    ///
    /// This is the `IP_MULTICAST_LOOP` option. On Windows the option belongs to the receiving
    /// socket, not the sending socket. See
    /// [`set_multicast_loop_v4`](UdpSocket::set_multicast_loop_v4).
    pub fn multicast_loop_v4(&self) -> io::Result<bool> {
        self.io.get_ref().multicast_loop_v4()
    }

    /// Sets the `IP_MULTICAST_LOOP` option of this socket: whether the multicast datagrams it sends
    /// are looped back to the local host, where the sockets that joined the group, this one
    /// included, receive them.
    ///
    /// On Windows the option applies to the receive path instead: it decides whether this socket
    /// receives the multicast datagrams that applications on the local host send. POSIX systems
    /// apply it to the sending socket.
    ///
    /// This is for IPv4 sockets. For IPv6 sockets, see
    /// [`set_multicast_loop_v6`](UdpSocket::set_multicast_loop_v6).
    pub fn set_multicast_loop_v4(&self, on: bool) -> io::Result<()> {
        self.io.get_ref().set_multicast_loop_v4(on)
    }

    /// The `IP_MULTICAST_TTL` option of this socket.
    ///
    /// This is the time-to-live field of the multicast IP packets sent from the socket. See
    /// [`set_multicast_ttl_v4`](UdpSocket::set_multicast_ttl_v4).
    pub fn multicast_ttl_v4(&self) -> io::Result<u32> {
        self.io.get_ref().multicast_ttl_v4()
    }

    /// Sets the `IP_MULTICAST_TTL` option of this socket.
    ///
    /// This is the time-to-live field of the multicast IP packets sent from the socket. It says how
    /// far they may travel. The default, `1`, keeps them on the local network.
    ///
    /// This is for IPv4 sockets.
    pub fn set_multicast_ttl_v4(&self, ttl: u32) -> io::Result<()> {
        self.io.get_ref().set_multicast_ttl_v4(ttl)
    }

    /// Whether the multicast datagrams this socket sends are looped back to the local host.
    ///
    /// This is the `IPV6_MULTICAST_LOOP` option. On Windows the option belongs to the receiving
    /// socket, not the sending socket. See
    /// [`set_multicast_loop_v6`](UdpSocket::set_multicast_loop_v6).
    pub fn multicast_loop_v6(&self) -> io::Result<bool> {
        self.io.get_ref().multicast_loop_v6()
    }

    /// Sets the `IPV6_MULTICAST_LOOP` option of this socket: whether the multicast datagrams it
    /// sends are looped back to the local host, where the sockets that joined the group, this one
    /// included, receive them.
    ///
    /// On Windows the option applies to the receive path instead, as for
    /// [`set_multicast_loop_v4`](UdpSocket::set_multicast_loop_v4), which is the IPv4 version of
    /// this.
    pub fn set_multicast_loop_v6(&self, on: bool) -> io::Result<()> {
        self.io.get_ref().set_multicast_loop_v6(on)
    }

    /// Joins the IPv4 multicast group `multiaddr`.
    ///
    /// The socket then receives the datagrams sent to that group, as the `IP_ADD_MEMBERSHIP` option
    /// does. `multiaddr` must be a multicast address. `interface` is the address of the local
    /// interface to join the group on, or [`Ipv4Addr::UNSPECIFIED`] to let the system choose one.
    /// Leave the group with [`leave_multicast_v4`](UdpSocket::leave_multicast_v4), passing the same
    /// arguments.
    pub fn join_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> io::Result<()> {
        self.io.get_ref().join_multicast_v4(&multiaddr, &interface)
    }

    /// Leaves the IPv4 multicast group `multiaddr`, as the `IP_DROP_MEMBERSHIP` option does.
    ///
    /// Pass the arguments that were passed to [`join_multicast_v4`](UdpSocket::join_multicast_v4).
    pub fn leave_multicast_v4(&self, multiaddr: Ipv4Addr, interface: Ipv4Addr) -> io::Result<()> {
        self.io.get_ref().leave_multicast_v4(&multiaddr, &interface)
    }

    /// Joins the IPv6 multicast group `multiaddr`.
    ///
    /// The socket then receives the datagrams sent to that group, as the `IPV6_ADD_MEMBERSHIP`
    /// option does. `multiaddr` must be a multicast address. `interface` is the index of the local
    /// interface to join the group on, or `0` to let the system choose one. Leave the group with
    /// [`leave_multicast_v6`](UdpSocket::leave_multicast_v6), passing the same arguments.
    pub fn join_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> io::Result<()> {
        self.io.get_ref().join_multicast_v6(multiaddr, interface)
    }

    /// Leaves the IPv6 multicast group `multiaddr`, as the `IPV6_DROP_MEMBERSHIP` option does.
    ///
    /// Pass the arguments that were passed to [`join_multicast_v6`](UdpSocket::join_multicast_v6).
    pub fn leave_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> io::Result<()> {
        self.io.get_ref().leave_multicast_v6(multiaddr, interface)
    }

    /// The `IP_TTL` option of this socket.
    ///
    /// This is the time-to-live field of the IP packets sent from the socket. See
    /// [`set_ttl`](UdpSocket::set_ttl).
    pub fn ttl(&self) -> io::Result<u32> {
        self.io.get_ref().ttl()
    }

    /// Sets the `IP_TTL` option of this socket.
    ///
    /// This is the time-to-live field of the IP packets sent from the socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.io.get_ref().set_ttl(ttl)
    }
}

impl<M> fmt::Debug for UdpSocket<M>
where
    M: Mode,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.io.get_ref(), f)
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl<M> AsFd for UdpSocket<M>
where
    M: Mode,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.io.get_ref().as_fd()
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl<M> AsRawFd for UdpSocket<M>
where
    M: Mode,
{
    fn as_raw_fd(&self) -> RawFd {
        self.io.get_ref().as_raw_fd()
    }
}

#[cfg(windows)]
impl<M> AsSocket for UdpSocket<M>
where
    M: Mode,
{
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.io.get_ref().as_socket()
    }
}

#[cfg(windows)]
impl<M> AsRawSocket for UdpSocket<M>
where
    M: Mode,
{
    fn as_raw_socket(&self) -> RawSocket {
        self.io.get_ref().as_raw_socket()
    }
}
