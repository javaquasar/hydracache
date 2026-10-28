//! Tests of [`UdpSocket`], over loopback.
//!
//! Each test makes the sockets it needs, bound to ports the system picks, so that tests running
//! side by side never meet. The ones that run in both flavours go through [`in_both_modes`]; the
//! ones that spawn tasks, or send a socket to another thread, only mean something on a shared
//! runtime and are written for that.
//!
//! The first tests are of sending and receiving: that a datagram goes from one socket to another
//! and names its sender, that a connected socket does the same without addresses, and that a peek
//! leaves the datagram for the next receive. Then come the tests of what a receive that has to
//! wait does: it waits for the runtime without blocking the thread or spinning, and giving it up
//! loses no datagram. After the forms an address may take, and the mode a socket made from std's
//! is put in, come the tests of the socket options, of a shared runtime's sockets crossing
//! threads, serving tasks, outliving a task that gave up its receive and having several tasks
//! receive at once, and last, of the sockets' `Debug`, descriptors and auto traits.

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawSocket;
use std::{
    cell::Cell,
    future::Future,
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4},
    pin::pin,
    sync::Arc,
    thread,
};

use futures::{
    executor::block_on,
    future::{join, poll_immediate},
};
use ntest::timeout;
use socket2::SockRef;

use super::{DELAY, after, counting_polls, in_both_modes, loopback};
use crate::{Local, Mode, Runtime, Shared, SharedRuntime, net::UdpSocket};

in_both_modes! {
    /// A datagram goes from one socket to another, which names the sender's address as the one it
    /// came from, and an answer sent to that address finds its way back.
    fn a_datagram_goes_from_one_socket_to_another<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (client, server) = pair(&runtime);
        let client_address = client.local_addr().unwrap();
        let server_address = server.local_addr().unwrap();

        runtime.block_on(async {
            assert_eq!(client.send_to(b"ping", server_address).await.unwrap(), 4);
            let mut received = [0; 16];
            let (len, from) = server.recv_from(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"ping");
            assert_eq!(from, client_address);

            assert_eq!(server.send_to(b"pong", from).await.unwrap(), 4);
            let (len, from) = client.recv_from(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"pong");
            assert_eq!(from, server_address);
        });
    }
}

in_both_modes! {
    /// A connected socket sends and receives through `send` and `recv`, with no address to give or
    /// to read, and names the peer it is connected to. Before it is connected it has no peer, and
    /// a `send` has nowhere to go.
    fn a_connected_socket_sends_and_receives_without_addresses<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (first, second) = pair(&runtime);
        let first_address = first.local_addr().unwrap();
        let second_address = second.local_addr().unwrap();

        assert_eq!(first.peer_addr().unwrap_err().kind(), io::ErrorKind::NotConnected);
        runtime.block_on(async {
            assert!(first.send(b"nowhere").await.is_err());
        });

        first.connect(second_address).unwrap();
        second.connect(first_address).unwrap();
        assert_eq!(first.peer_addr().unwrap(), second_address);
        assert_eq!(second.peer_addr().unwrap(), first_address);

        runtime.block_on(async {
            assert_eq!(first.send(b"ping").await.unwrap(), 4);
            let mut received = [0; 16];
            let len = second.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"ping");

            assert_eq!(second.send(b"pong").await.unwrap(), 4);
            let len = first.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"pong");
        });
    }
}

in_both_modes! {
    /// `peek_from` waits for a datagram and copies it out without taking it off the socket, so the
    /// next receive finds the same datagram again, from the same sender.
    fn peek_from_leaves_the_datagram_for_the_next_receive<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (client, server) = pair(&runtime);
        let client_address = client.local_addr().unwrap();
        let server_address = server.local_addr().unwrap();

        runtime.block_on(async {
            // Sent after a delay, with the `peek_from` already waiting for it.
            let mut peeked = [0; 16];
            let (peek, sent) = join(
                server.peek_from(&mut peeked),
                after(&runtime, client.send_to(b"peeked", server_address)),
            )
            .await;
            sent.unwrap();
            let (peeked_len, peeked_from) = peek.unwrap();
            assert_eq!(&peeked[..peeked_len], b"peeked");
            assert_eq!(peeked_from, client_address);

            // The datagram is on the socket still, so a receive resolves in its first poll. One
            // that had to wait would mean that the peek took the datagram away.
            let mut received = [0; 16];
            let (len, from) = poll_immediate(server.recv_from(&mut received))
                .await
                .expect("the peeked datagram is still on the socket")
                .unwrap();
            assert_eq!(&received[..len], b"peeked");
            assert_eq!(from, client_address);
        });
    }
}

in_both_modes! {
    /// `peek` waits for a datagram and copies it out without taking it off the socket, so the
    /// next receive finds the same datagram again.
    fn peek_leaves_the_datagram_for_the_next_receive<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (client, server) = connected_pair(&runtime);

        runtime.block_on(async {
            // Sent after a delay, with the `peek` already waiting for it.
            let mut peeked = [0; 16];
            let (peek, sent) = join(
                server.peek(&mut peeked),
                after(&runtime, client.send(b"peeked")),
            )
            .await;
            sent.unwrap();
            let peeked_len = peek.unwrap();
            assert_eq!(&peeked[..peeked_len], b"peeked");

            // The datagram is on the socket still, so a receive resolves in its first poll. One
            // that had to wait would mean that the peek took the datagram away.
            let mut received = [0; 16];
            let len = poll_immediate(server.recv(&mut received))
                .await
                .expect("the peeked datagram is still on the socket")
                .unwrap();
            assert_eq!(&received[..len], b"peeked");
        });
    }
}

in_both_modes! {
    /// A receive with no datagram to take waits for the runtime, which is what lets the send that
    /// is due after a delay, from this very thread, happen. A receive that blocked the thread would
    /// hold it for good, and the test's timeout would fail it.
    fn a_receive_waits_for_a_datagram_without_blocking_the_thread<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (client, server) = pair(&runtime);
        let client_address = client.local_addr().unwrap();
        let server_address = server.local_addr().unwrap();

        runtime.block_on(async {
            let mut received = [0; 16];
            let (received_from, sent) = join(
                server.recv_from(&mut received),
                after(&runtime, client.send_to(b"late", server_address)),
            )
            .await;

            let (len, from) = received_from.unwrap();
            sent.unwrap();
            assert_eq!(&received[..len], b"late");
            assert_eq!(from, client_address);
        });
    }
}

in_both_modes! {
    /// A receive or a peek that has to wait is polled when something changes, not over and over:
    /// it waits for the socket to become readable, which a socket with nothing to receive is not,
    /// whereas it is writable all the while, so a wait for that instead would spin the thread. The
    /// tests above would pass all the same, since the datagram does arrive in the end.
    ///
    /// A call that waits is polled when it begins to, and again each time its task is woken while
    /// it waits: by the timer that ends the delay, since the task polls both of the calls that are
    /// joined, and by the datagram arriving. A thread that spun would poll it hundreds of times.
    /// Each of the four calls waits once, and a peek is followed by the receive that takes the
    /// datagram it left.
    fn a_receive_that_waits_does_not_spin<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (client, server) = connected_pair(&runtime);
        let mut buffer = [0; 16];

        runtime.block_on(async {
            let (received, polls) =
                polls_while_waiting(&runtime, &client, server.recv_from(&mut buffer)).await;
            received.unwrap();
            assert!(polls <= 10, "`recv_from` was polled {polls} times");

            let (received, polls) =
                polls_while_waiting(&runtime, &client, server.recv(&mut buffer)).await;
            received.unwrap();
            assert!(polls <= 10, "`recv` was polled {polls} times");

            let (peeked, polls) =
                polls_while_waiting(&runtime, &client, server.peek_from(&mut buffer)).await;
            peeked.unwrap();
            assert!(polls <= 10, "`peek_from` was polled {polls} times");
            server.recv(&mut buffer).await.unwrap();

            let (peeked, polls) =
                polls_while_waiting(&runtime, &client, server.peek(&mut buffer)).await;
            peeked.unwrap();
            assert!(polls <= 10, "`peek` was polled {polls} times");
            server.recv(&mut buffer).await.unwrap();
        });
    }
}

in_both_modes! {
    /// Giving up a receive that is waiting loses no datagram: the next receive, which is already
    /// waiting when a datagram is sent, takes it. That a datagram wakes the task of the receive
    /// that waits for it, and not one that gave up, is for the test of tasks below.
    fn a_dropped_receive_loses_no_datagram<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (client, server) = pair(&runtime);
        let server_address = server.local_addr().unwrap();

        runtime.block_on(async {
            let mut buffer = [0; 16];
            {
                let mut abandoned = pin!(server.recv_from(&mut buffer));
                assert!(poll_immediate(abandoned.as_mut()).await.is_none());
            }

            let (received, sent) = join(
                server.recv_from(&mut buffer),
                after(&runtime, client.send_to(b"kept", server_address)),
            )
            .await;
            sent.unwrap();
            let (len, _) = received.unwrap();

            assert_eq!(&buffer[..len], b"kept");
        });
    }
}

in_both_modes! {
    /// An address may be given as a pair of an IP address and a port, or as a `SocketAddrV4`, as
    /// well as a `SocketAddr`: to `bind`, to `connect` and to `send_to` alike, which ask for
    /// anything that converts into a socket address.
    fn addresses_are_taken_as_pairs_and_as_socket_addresses<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        // One socket is bound with a pair, the other with a `SocketAddrV4`.
        let by_pair = UdpSocket::bind(&runtime, (Ipv4Addr::LOCALHOST, 0)).unwrap();
        let by_address =
            UdpSocket::bind(&runtime, SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)).unwrap();
        let pair_port = by_pair.local_addr().unwrap().port();
        let address_port = by_address.local_addr().unwrap().port();

        // Each is sent to with the other form of address.
        runtime.block_on(async {
            by_pair
                .send_to(b"to an address", SocketAddrV4::new(Ipv4Addr::LOCALHOST, address_port))
                .await
                .unwrap();
            by_address
                .send_to(b"to a pair", (Ipv4Addr::LOCALHOST, pair_port))
                .await
                .unwrap();

            let mut received = [0; 16];
            let (len, from) = by_address.recv_from(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"to an address");
            assert_eq!(from.port(), pair_port);
            let (len, from) = by_pair.recv_from(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"to a pair");
            assert_eq!(from.port(), address_port);
        });

        // And each is connected with the other form of address, too.
        by_pair
            .connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, address_port))
            .unwrap();
        by_address
            .connect((Ipv4Addr::LOCALHOST, pair_port))
            .unwrap();
        assert_eq!(by_pair.peer_addr().unwrap().port(), address_port);
        assert_eq!(by_address.peer_addr().unwrap().port(), pair_port);
    }
}

in_both_modes! {
    /// A socket made from a std one in blocking mode is switched to non-blocking mode: a receive
    /// with nothing to receive yet waits for the runtime, which is what lets the send that is due
    /// after a delay, from this very thread, happen. A blocking socket would hold the thread in
    /// the receive for good, and the test's timeout would fail it.
    ///
    /// Both sockets are std ones to begin with, in blocking mode as std makes them, and the one
    /// that sends stays a std socket.
    fn from_std_makes_a_blocking_socket_non_blocking<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let std_socket = std::net::UdpSocket::bind(loopback()).unwrap();
        let address = std_socket.local_addr().unwrap();
        let peer = std::net::UdpSocket::bind(loopback()).unwrap();
        let peer_address = peer.local_addr().unwrap();
        let socket = UdpSocket::from_std(&runtime, std_socket).unwrap();

        runtime.block_on(async {
            let mut received = [0; 16];
            let (received_from, sent) = join(
                socket.recv_from(&mut received),
                after(&runtime, async { peer.send_to(b"late", address) }),
            )
            .await;

            let (len, from) = received_from.unwrap();
            sent.unwrap();
            assert_eq!(&received[..len], b"late");
            assert_eq!(from, peer_address);
        });
    }
}

in_both_modes! {
    /// The time-to-live and the `SO_BROADCAST` option read back what was set.
    fn ttl_and_broadcast_read_back_what_was_set<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let socket = UdpSocket::bind(&runtime, loopback()).unwrap();

        socket.set_ttl(42).unwrap();
        assert_eq!(socket.ttl().unwrap(), 42);

        socket.set_broadcast(true).unwrap();
        assert!(socket.broadcast().unwrap());
        socket.set_broadcast(false).unwrap();
        assert!(!socket.broadcast().unwrap());
    }
}

in_both_modes! {
    /// The IPv4 multicast options read back what was set: the loop-back of the multicast
    /// datagrams a socket sends, and their time-to-live. Each is first set to a value other than
    /// the system's default, which is what tells a setter that did nothing.
    fn the_ipv4_multicast_options_read_back_what_was_set<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let socket = UdpSocket::bind(&runtime, loopback()).unwrap();

        socket.set_multicast_loop_v4(false).unwrap();
        assert!(!socket.multicast_loop_v4().unwrap());
        socket.set_multicast_loop_v4(true).unwrap();
        assert!(socket.multicast_loop_v4().unwrap());

        socket.set_multicast_ttl_v4(7).unwrap();
        assert_eq!(socket.multicast_ttl_v4().unwrap(), 7);
    }
}

in_both_modes! {
    /// A socket joins an IPv4 multicast group on the loopback interface and leaves it again.
    /// Leaving a group works only where it was joined, so a join that did nothing fails the first
    /// leave, and a leave that did nothing lets the second one through.
    ///
    /// A system's loopback interface is not always one that takes part in multicast. Linux's does,
    /// with no set-up, and the test is written for it alone.
    #[cfg(target_os = "linux")]
    fn a_socket_joins_and_leaves_an_ipv4_multicast_group<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let socket = UdpSocket::bind(&runtime, loopback()).unwrap();
        let group = Ipv4Addr::new(239, 255, 0, 1);

        socket.join_multicast_v4(group, Ipv4Addr::LOCALHOST).unwrap();
        socket.leave_multicast_v4(group, Ipv4Addr::LOCALHOST).unwrap();
        assert!(socket.leave_multicast_v4(group, Ipv4Addr::LOCALHOST).is_err());
    }
}

in_both_modes! {
    /// The IPv6 multicast options read back what was set, on a socket bound to the IPv6 wildcard
    /// address, and a group the socket joins it leaves again.
    ///
    /// Nothing here assumes the system has IPv6, which a container may lack: the test first binds
    /// a socket of std's to the IPv6 loopback address, and where that fails there is nothing to
    /// test, and it ends. Past that probe every call of this crate's own has to succeed, so that
    /// a bind that fails because of this crate fails the test, and is not mistaken for a system
    /// without IPv6. Nor does the test assume an interface to join a group on, which a system with
    /// IPv6 may lack as well: a join that fails is the system's answer, and only one that
    /// succeeds is followed by a leave.
    fn the_ipv6_multicast_options_read_back_what_was_set<M>() {
        // The probe is std's, so that it is the system that is asked, and not this crate.
        if std::net::UdpSocket::bind((Ipv6Addr::LOCALHOST, 0)).is_err() {
            return;
        }

        let runtime = Runtime::<M>::new().unwrap();
        let socket = UdpSocket::bind(&runtime, (Ipv6Addr::UNSPECIFIED, 0)).unwrap();

        socket.set_multicast_loop_v6(false).unwrap();
        assert!(!socket.multicast_loop_v6().unwrap());
        socket.set_multicast_loop_v6(true).unwrap();
        assert!(socket.multicast_loop_v6().unwrap());

        let group = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0x1234);
        if socket.join_multicast_v6(&group, 0).is_ok() {
            socket.leave_multicast_v6(&group, 0).unwrap();
        }
    }
}

/// A socket built on a shared runtime moves to another thread, which uses it there: that thread
/// drives the runtime with a `block_on` of its own, after the thread that made the socket has left
/// the one it made it in, since a runtime made by `new` is driven by one thread at a time.
#[test]
#[timeout(15000)]
fn a_shared_socket_crosses_threads() {
    let runtime = SharedRuntime::new().unwrap();
    // Made inside a `block_on`, as the sockets of a task are, and left behind by it.
    let (client, server) = runtime.block_on(async { pair(&runtime) });
    let server_address = server.local_addr().unwrap();

    let user = thread::spawn({
        let runtime = runtime.clone();
        move || {
            runtime.block_on(async move {
                let mut request = [0; 16];
                // Waits for the datagram, with the reactor of the runtime driven by this thread.
                let (len, from) = server.recv_from(&mut request).await.unwrap();
                server
                    .send_to(&request[..len].to_ascii_uppercase(), from)
                    .await
                    .unwrap();
            });
        }
    });

    // This thread is inside no `block_on`, so it leaves the runtime to the other one: a send that
    // has room goes to the kernel without the runtime, and any executor can poll it.
    thread::sleep(DELAY);
    block_on(client.send_to(b"hello", server_address)).unwrap();
    user.join().unwrap();

    let mut response = [0; 16];
    let (len, from) = runtime.block_on(client.recv_from(&mut response)).unwrap();
    assert_eq!(&response[..len], b"HELLO");
    assert_eq!(from, server_address);
}

/// The tasks of a shared runtime use sockets: their futures are `Send`, which spawning one
/// checks, and the runtime runs them to the end.
#[test]
#[timeout(15000)]
fn shared_tasks_use_sockets() {
    let runtime = SharedRuntime::new().unwrap();
    let (client, server) = pair(&runtime);
    let server_address = server.local_addr().unwrap();

    runtime.block_on(async {
        let echo = runtime.spawn("an echo server", async move {
            let mut buffer = [0; 16];
            let (len, from) = server.recv_from(&mut buffer).await.unwrap();
            server.send_to(&buffer[..len], from).await.unwrap();
        });
        let asker = runtime.spawn("a client", async move {
            client.send_to(b"hello", server_address).await.unwrap();

            let mut answer = [0; 16];
            let (len, _) = client.recv_from(&mut answer).await.unwrap();

            answer[..len].to_vec()
        });

        assert_eq!(asker.await.unwrap(), b"hello");
        echo.await.unwrap();
    });
}

/// A receive that was given up does not keep the next one from being woken: the task of the first
/// receive is cancelled while it waits, which gives up its wait, and the datagram that comes in
/// must reach the task of the second receive instead.
///
/// The receives run in tasks because a task is polled when it is woken, and only then. The future
/// that `block_on` polls is polled after every wait, woken or not, which would hide a datagram
/// that woke the wrong task.
#[test]
#[timeout(15000)]
fn a_receive_is_woken_after_another_task_gave_up_its_own() {
    let runtime = SharedRuntime::new().unwrap();
    let (client, server) = pair(&runtime);
    let server_address = server.local_addr().unwrap();
    let server = Arc::new(server);

    runtime.block_on(async {
        let first = runtime.spawn("a receiver that gives up", {
            let server = server.clone();
            async move { server.recv_from(&mut [0; 16]).await }
        });
        // The task has begun to wait by the time the sleep is over, and is cancelled there.
        runtime.sleep(DELAY).await;
        drop(first);

        let second = runtime.spawn("a receiver", {
            let server = server.clone();
            async move {
                let mut buffer = [0; 16];
                let (len, _) = server.recv_from(&mut buffer).await.unwrap();

                buffer[..len].to_vec()
            }
        });
        runtime.sleep(DELAY).await;

        client.send_to(b"kept", server_address).await.unwrap();
        assert_eq!(second.await.unwrap(), b"kept");
    });
}

/// Two tasks wait in `recv_from` on one socket at once, and each gets one of the two datagrams that
/// are sent: none is left waiting for a wake-up that another's wait took.
///
/// The receives run in tasks because a task is polled when it is woken, and only then. The future
/// that `block_on` polls is polled after every wait, woken or not, which would hide a datagram
/// that woke the wrong task.
#[test]
#[timeout(15000)]
fn several_tasks_receive_on_one_socket() {
    let runtime = SharedRuntime::new().unwrap();
    let (client, server) = pair(&runtime);
    let server_address = server.local_addr().unwrap();
    let server = Arc::new(server);

    runtime.block_on(async {
        let receivers: Vec<_> = (0..2)
            .map(|_| {
                let server = server.clone();
                runtime.spawn("a receiver", async move {
                    let mut buffer = [0; 16];
                    let (len, _) = server.recv_from(&mut buffer).await.unwrap();

                    buffer[..len].to_vec()
                })
            })
            .collect();
        // The tasks have begun to wait by the time the sleep is over.
        runtime.sleep(DELAY).await;

        client.send_to(b"first", server_address).await.unwrap();
        client.send_to(b"second", server_address).await.unwrap();

        let mut received = Vec::new();
        for receiver in receivers {
            received.push(receiver.await.unwrap());
        }
        received.sort();
        assert_eq!(received, [b"first".to_vec(), b"second".to_vec()]);
    });
}

in_both_modes! {
    /// A socket prints as the std socket it was made from does, and its descriptor is that
    /// socket's: the one `AsRawFd` or `AsRawSocket` names, and the one a borrow of it reaches.
    fn a_socket_prints_and_exposes_the_descriptor_of_the_std_socket_it_wraps<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let std_socket = std::net::UdpSocket::bind(loopback()).unwrap();
        let address = std_socket.local_addr().unwrap();
        let printed = format!("{std_socket:?}");
        #[cfg(unix)]
        let raw = std_socket.as_raw_fd();
        #[cfg(windows)]
        let raw = std_socket.as_raw_socket();

        let socket = UdpSocket::from_std(&runtime, std_socket).unwrap();

        assert_eq!(format!("{socket:?}"), printed);
        #[cfg(unix)]
        assert_eq!(socket.as_raw_fd(), raw);
        #[cfg(windows)]
        assert_eq!(socket.as_raw_socket(), raw);
        // `SockRef` asks the system about the descriptor it is given, which is how the borrow is
        // seen to be of the socket itself: it finds the address that socket has.
        let found = SockRef::from(&socket).local_addr().unwrap();
        assert_eq!(found.as_socket(), Some(address));
    }
}

/// A shared runtime's socket is `Send` and `Sync`, so it crosses threads and is used from several,
/// and a socket of either flavour is `Unpin`. That a local runtime's is neither `Send` nor `Sync`
/// is shown by the compile-fail doc tests, since only code that does not compile can show it.
#[test]
#[timeout(15000)]
fn sockets_are_unpin_and_shared_ones_are_send_and_sync() {
    fn sent_and_shared<T>()
    where
        T: Send + Sync + Unpin,
    {
    }

    fn unpin<T>()
    where
        T: Unpin,
    {
    }

    sent_and_shared::<UdpSocket<Shared>>();
    unpin::<UdpSocket<Local>>();
}

/// Two sockets bound to loopback ports the system picks.
fn pair<M>(runtime: &Runtime<M>) -> (UdpSocket<M>, UdpSocket<M>)
where
    M: Mode,
{
    let first = UdpSocket::bind(runtime, loopback()).unwrap();
    let second = UdpSocket::bind(runtime, loopback()).unwrap();

    (first, second)
}

/// [`pair`], each socket connected to the other.
fn connected_pair<M>(runtime: &Runtime<M>) -> (UdpSocket<M>, UdpSocket<M>)
where
    M: Mode,
{
    let (first, second) = pair(runtime);
    first.connect(second.local_addr().unwrap()).unwrap();
    second.connect(first.local_addr().unwrap()).unwrap();

    (first, second)
}

/// What `receive` resolves to, and how often it was polled meanwhile, as it waits for the datagram
/// that `sender`, connected to the socket `receive` is on, sends once [`DELAY`] has passed.
async fn polls_while_waiting<M, F>(
    runtime: &Runtime<M>,
    sender: &UdpSocket<M>,
    receive: F,
) -> (F::Output, usize)
where
    M: Mode,
    F: Future,
{
    let polls = Cell::new(0);
    let (output, sent) = join(
        counting_polls(&polls, receive),
        after(runtime, sender.send(b"1")),
    )
    .await;
    sent.unwrap();

    (output, polls.get())
}
