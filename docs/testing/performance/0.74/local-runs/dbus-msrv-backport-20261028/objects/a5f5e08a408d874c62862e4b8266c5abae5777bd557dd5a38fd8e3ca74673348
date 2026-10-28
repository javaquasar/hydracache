//! Tests of [`UnixListener`], [`UnixStream`], [`UnixDatagram`] and [`Incoming`], over socket files
//! in directories made for the tests.
//!
//! Each test makes the socket files it needs in a directory of its own, which a guard removes when
//! the test is done, so that tests running side by side never meet. The ones that run in both
//! flavours go through [`in_both_modes`]; the ones that spawn tasks, or send a stream to another
//! thread, only mean something on a shared runtime and are written for that.
//!
//! The first tests are of making a connection: that a connection carries bytes both ways, that one
//! to a path with no socket behind it reports why, that one to an address, of a path or of a name
//! in the abstract namespace, does the same and refuses an address with no name, and that one the
//! listener has no room for yet is waited for. Then come the tests of the mode the sockets are in:
//! that the stream a listener accepts, the streams of a pair, and the sockets made from std's,
//! never block the thread, and those of a stream handed back to std, which is the same connection,
//! in the same mode, and watched by nobody. Closing and shutting down follow, with a write to a
//! peer that has gone, then the ways of sharing a stream and what a call that has to wait does: a
//! write waits for room and a read for bytes, without spinning, and giving up an `accept` loses no
//! connection. After the tests of `incoming` come those of the datagram sockets, a `send_to` that
//! finds the receiver's queue full and one that finds the budget of its poll spent among them, of
//! a shared runtime's sockets crossing threads, serving tasks and having several of them wait at
//! once, and last, of the sockets' `Debug`, descriptors and auto traits.

#[cfg(target_os = "android")]
use std::os::android::net::SocketAddrExt;
#[cfg(target_os = "linux")]
use std::os::linux::net::SocketAddrExt;
use std::{
    cell::Cell,
    future::{Future, poll_fn},
    io::{self, Read, Write},
    net::Shutdown,
    os::fd::AsRawFd,
    path::PathBuf,
    pin::{Pin, pin},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    thread,
};

use futures::{
    AsyncReadExt, AsyncWriteExt, StreamExt,
    executor::block_on,
    future::{join, poll_immediate},
    io::{AsyncWrite, copy},
};
use ntest::timeout;
use socket2::SockRef;
#[cfg(any(target_os = "linux", target_os = "android"))]
use socket2::{Domain, SockAddr, Socket, Type};

use super::{DELAY, after, counting_polls, in_both_modes, pattern};
use crate::{
    Local, Mode, Runtime, Shared, SharedRuntime,
    coop::{self, budgeted, spend},
    net::unix::{Incoming, UnixDatagram, UnixListener, UnixStream},
    tests::core::counting_waker,
};

in_both_modes! {
    /// A connection carries bytes both ways, and the addresses tell the ends apart: the listener's
    /// path is the one the client connects to and the one the accepted stream is on, while the
    /// client is bound to no path, so that it, and the address the connection is accepted from,
    /// are unnamed.
    fn a_connection_echoes_both_ways<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("echo");
        let path = directory.socket("listener");
        let listener = UnixListener::bind(&runtime, &path).unwrap();

        runtime.block_on(async {
            let mut client = UnixStream::connect(&runtime, &path).await.unwrap();
            let (mut server, peer) = listener.accept().await.unwrap();

            client.write_all(b"ping").await.unwrap();
            let mut received = [0; 4];
            server.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"ping");

            server.write_all(b"pong").await.unwrap();
            client.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"pong");

            assert_eq!(listener.local_addr().unwrap().as_pathname(), Some(path.as_path()));
            assert_eq!(client.peer_addr().unwrap().as_pathname(), Some(path.as_path()));
            assert_eq!(server.local_addr().unwrap().as_pathname(), Some(path.as_path()));
            assert!(client.local_addr().unwrap().is_unnamed());
            assert!(server.peer_addr().unwrap().is_unnamed());
            assert!(peer.is_unnamed());
        });
    }
}

in_both_modes! {
    /// A connection to a path where there is no file fails with the error the kernel reports for
    /// it, which the connect returns at once.
    fn a_connection_to_a_missing_file_fails_with_not_found<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("missing");

        let error = runtime
            .block_on(UnixStream::connect(&runtime, directory.socket("nobody")))
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}

in_both_modes! {
    /// A connection to a socket file whose listener is gone is refused, which the connect reports
    /// as it is: dropping a listener leaves the file it made where it was, and the kernel answers
    /// a connection to such a file with a refusal.
    fn a_connection_to_a_socket_nobody_listens_at_fails_with_connection_refused<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("refused");
        let path = directory.socket("gone");
        drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
        assert!(path.exists(), "the listener took its file with it");

        let error = runtime
            .block_on(UnixStream::connect(&runtime, &path))
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::ConnectionRefused);
    }
}

in_both_modes! {
    /// A path too long to name a socket is reported as invalid input, by each constructor that is
    /// given one, rather than cut short to the path of some other file.
    ///
    /// The name is far longer than any platform's limit, which is a little over a hundred bytes at
    /// most.
    fn a_path_too_long_for_a_socket_is_invalid_input<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("long");
        let path = directory.socket(&"x".repeat(300));

        let error = UnixListener::bind(&runtime, &path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

        let error = UnixDatagram::bind(&runtime, &path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

        let error = runtime
            .block_on(UnixStream::connect(&runtime, &path))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}

in_both_modes! {
    /// A path with a zero byte in it is refused, as std refuses it to a listener, rather than cut
    /// short at that byte: the connect does not reach the socket at the path before it, though one
    /// listens there.
    fn a_path_with_a_zero_byte_in_it_is_invalid_input<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("zero-byte");
        let path = directory.socket("listening");
        let _listener = UnixListener::bind(&runtime, &path).unwrap();
        let mut cut_short = path.clone().into_os_string();
        cut_short.push("\0and more");

        let error = runtime
            .block_on(UnixStream::connect(&runtime, &cut_short))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}

in_both_modes! {
    /// A connection to the address of a listener, as the listener tells it, is a connection to the
    /// path the listener is bound to: it carries bytes both ways, and the stream is connected to
    /// that path.
    fn connect_addr_connects_to_the_address_of_a_listener<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("addr");
        let path = directory.socket("listener");
        let listener = UnixListener::bind(&runtime, &path).unwrap();
        let address = listener.local_addr().unwrap();

        runtime.block_on(async {
            let mut client = UnixStream::connect_addr(&runtime, &address).await.unwrap();
            let (mut server, _) = listener.accept().await.unwrap();

            client.write_all(b"ping").await.unwrap();
            let mut received = [0; 4];
            server.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"ping");

            server.write_all(b"pong").await.unwrap();
            client.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"pong");

            assert_eq!(client.peer_addr().unwrap().as_pathname(), Some(path.as_path()));
            assert!(client.local_addr().unwrap().is_unnamed());
        });
    }
}

in_both_modes! {
    /// A connection to the address of a path fails with the errors a connection to the path does:
    /// with `ConnectionRefused` where the socket at the path is gone, which leaves its file
    /// behind, and with `NotFound` where the file is gone too.
    fn connect_addr_to_a_path_with_no_socket_behind_it_reports_why<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("addr-gone");
        let path = directory.socket("gone");
        let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        assert!(path.exists(), "the listener took its file with it");

        let error = runtime
            .block_on(UnixStream::connect_addr(&runtime, &address))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::ConnectionRefused);

        std::fs::remove_file(&path).unwrap();
        let error = runtime
            .block_on(UnixStream::connect_addr(&runtime, &address))
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}

in_both_modes! {
    /// A connection to a name in the abstract namespace, which has no file behind it, carries
    /// bytes both ways, and the stream is connected to that name.
    ///
    /// The listener is a std one, in blocking mode: the connection is queued by the time the
    /// connect completes, so its `accept` returns at once.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn connect_addr_connects_to_an_abstract_name<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let name = abstract_name("echo");
        let address = std::os::unix::net::SocketAddr::from_abstract_name(&name).unwrap();
        let listener = std::os::unix::net::UnixListener::bind_addr(&address).unwrap();

        runtime.block_on(async {
            let mut client = UnixStream::connect_addr(&runtime, &address).await.unwrap();
            let (accepted, _) = listener.accept().unwrap();
            let mut server = UnixStream::from_std(&runtime, accepted).unwrap();

            client.write_all(b"ping").await.unwrap();
            let mut received = [0; 4];
            server.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"ping");

            server.write_all(b"pong").await.unwrap();
            client.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"pong");

            assert_eq!(client.peer_addr().unwrap().as_abstract_name(), Some(&name[..]));
            assert!(client.peer_addr().unwrap().as_pathname().is_none());
        });
    }
}

in_both_modes! {
    /// A zero byte inside an abstract name is part of it, unlike one inside a path: the connection
    /// reaches the socket with the whole name, not the one whose name is the bytes before the
    /// zero.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn connect_addr_takes_a_zero_byte_inside_an_abstract_name_as_part_of_it<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut name = abstract_name("zero-byte");
        let cut_short = std::os::unix::net::SocketAddr::from_abstract_name(&name).unwrap();
        let cut_short_listener = std::os::unix::net::UnixListener::bind_addr(&cut_short).unwrap();
        cut_short_listener.set_nonblocking(true).unwrap();
        name.extend_from_slice(b"\0and more");
        let address = std::os::unix::net::SocketAddr::from_abstract_name(&name).unwrap();
        let listener = std::os::unix::net::UnixListener::bind_addr(&address).unwrap();

        runtime.block_on(async {
            let client = UnixStream::connect_addr(&runtime, &address).await.unwrap();

            assert_eq!(client.peer_addr().unwrap().as_abstract_name(), Some(&name[..]));
            // The connection is queued at the listener with the whole name, and not at the other.
            listener.accept().unwrap();
            let error = cut_short_listener.accept().unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        });
    }
}

in_both_modes! {
    /// A connection to a name in the abstract namespace that nobody listens at is refused, which
    /// the connect reports as it is: there is no file for it to be missing.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn a_connection_to_an_abstract_name_nobody_listens_at_fails_with_connection_refused<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let address = std::os::unix::net::SocketAddr::from_abstract_name(abstract_name("gone"))
            .unwrap();

        let error = runtime
            .block_on(UnixStream::connect_addr(&runtime, &address))
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::ConnectionRefused);
    }
}

in_both_modes! {
    /// An unnamed address names no socket to connect to, so a connection to it is reported as
    /// invalid input rather than tried. The address of either end of a pair is one.
    fn connect_addr_with_an_unnamed_address_is_invalid_input<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (stream, _peer) = std::os::unix::net::UnixStream::pair().unwrap();
        let unnamed = stream.local_addr().unwrap();
        assert!(unnamed.is_unnamed());

        let error = runtime
            .block_on(UnixStream::connect_addr(&runtime, &unnamed))
            .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}

in_both_modes! {
    /// A connection the listener has no room for yet is waited for, not reported: the connect is
    /// still pending after a poll, and completes once the listener takes the connection in its
    /// queue out of it, which makes the room.
    ///
    /// A blocking connect waits in the kernel for room in the listener's queue. A non-blocking one
    /// is turned away at once instead, so the connect asks again every so often, which makes this
    /// take a moment. The listener is a plain socket with a backlog of zero, which queues one
    /// connection and no more, and a thread of its own takes the queued connection and the one
    /// that waits, by blocking in `accept`, after a delay that lets the connect have been turned
    /// away at least once. How long the connect has to wait changes how many times it asks, never
    /// whether it gets in, which only the test's own timeout would catch.
    ///
    /// Linux and Android queue a connection where the backlog says none, and turn away the one
    /// after it. Other platforms refuse a connection that finds the queue full, or admit more
    /// connections than the backlog asks for, so one there would not be left pending, and the test
    /// is written for these two alone.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn a_full_backlog_is_waited_out<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("backlog");
        let path = directory.socket("listener");
        let listener = Socket::new(Domain::UNIX, Type::STREAM, None).unwrap();
        listener.bind(&SockAddr::unix(&path).unwrap()).unwrap();
        listener.listen(0).unwrap();

        runtime.block_on(async {
            // The queue holds this one, so the kernel has nowhere to put the next connection
            // until the listener takes this one out.
            let _queued = UnixStream::connect(&runtime, &path).await.unwrap();

            let mut pending = pin!(UnixStream::connect(&runtime, &path));
            assert!(
                poll_immediate(pending.as_mut()).await.is_none(),
                "the connection was reported before the listener had room for it",
            );

            let accepting = thread::spawn(move || {
                thread::sleep(DELAY);
                listener.accept().unwrap();
                listener.accept().unwrap();
            });
            let stream = pending.await.unwrap();
            accepting.join().unwrap();

            assert_eq!(stream.peer_addr().unwrap().as_pathname(), Some(path.as_path()));
        });
    }
}

in_both_modes! {
    /// A stream the listener accepted is in non-blocking mode, though std's `accept` leaves the
    /// socket it makes blocking on Linux: a read that has nothing to read yet waits for the
    /// runtime, which is what lets the write that is due after a delay, from this very thread,
    /// happen. A blocking socket would hold the thread in the read for good, and the test's
    /// timeout would fail it.
    fn an_accepted_stream_is_non_blocking<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("accepted");
        let path = directory.socket("listener");
        let listener = UnixListener::bind(&runtime, &path).unwrap();

        runtime.block_on(async {
            let mut client = UnixStream::connect(&runtime, &path).await.unwrap();
            let (mut server, _) = listener.accept().await.unwrap();

            let mut received = [0; 4];
            let (written, read) = join(
                after(&runtime, client.write_all(b"late")),
                server.read_exact(&mut received),
            )
            .await;

            written.unwrap();
            read.unwrap();
            assert_eq!(&received, b"late");
        });
    }
}

in_both_modes! {
    /// A pair of streams carries bytes both ways, and both are in non-blocking mode: a read with
    /// nothing to read yet waits for the runtime, which is what lets the write that is due after a
    /// delay, from this very thread, happen, whichever of the two reads. A blocking stream would
    /// hold the thread in the read for good, and the test's timeout would fail it.
    fn a_pair_of_streams_echoes_both_ways_and_is_non_blocking<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (mut left, mut right) = UnixStream::pair(&runtime).unwrap();

        runtime.block_on(async {
            left.write_all(b"ping").await.unwrap();
            let mut received = [0; 4];
            right.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"ping");

            right.write_all(b"pong").await.unwrap();
            left.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"pong");

            let (written, read) = join(
                after(&runtime, left.write_all(b"late")),
                right.read_exact(&mut received),
            )
            .await;
            written.unwrap();
            read.unwrap();
            assert_eq!(&received, b"late");

            let (written, read) = join(
                after(&runtime, right.write_all(b"back")),
                left.read_exact(&mut received),
            )
            .await;
            written.unwrap();
            read.unwrap();
            assert_eq!(&received, b"back");
        });
    }
}

in_both_modes! {
    /// A listener made from a std one in blocking mode is switched to non-blocking mode: an
    /// `accept` with no connection to take waits for the runtime, which is what lets the
    /// connection that is made after a delay, from this very thread, come in. A blocking listener
    /// would hold the thread in `accept` for good, and the test's timeout would fail it.
    fn from_std_makes_a_blocking_listener_non_blocking<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("std-listener");
        let path = directory.socket("listener");
        let std_listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
        let listener = UnixListener::from_std(&runtime, std_listener).unwrap();

        runtime.block_on(async {
            let (accepted, client) = join(
                listener.accept(),
                after(&runtime, UnixStream::connect(&runtime, &path)),
            )
            .await;

            let (_server, peer) = accepted.unwrap();
            assert!(peer.is_unnamed());
            let client = client.unwrap();
            assert_eq!(client.peer_addr().unwrap().as_pathname(), Some(path.as_path()));
        });
    }
}

in_both_modes! {
    /// A stream made from a std one in blocking mode is switched to non-blocking mode: a read
    /// with nothing to read yet waits for the runtime, which is what lets the write that is due
    /// after a delay, from this very thread, happen. A blocking stream would hold the thread in
    /// the read for good, and the test's timeout would fail it.
    ///
    /// Both ends are std streams to begin with, in blocking mode as std makes them, and the one
    /// that is read from is the one that is made into a stream of this type.
    fn from_std_makes_a_blocking_stream_non_blocking<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (std_stream, mut peer) = std::os::unix::net::UnixStream::pair().unwrap();
        let mut stream = UnixStream::from_std(&runtime, std_stream).unwrap();

        runtime.block_on(async {
            let mut received = [0; 4];
            let (written, read) = join(
                after(&runtime, async { peer.write_all(b"late") }),
                stream.read_exact(&mut received),
            )
            .await;

            written.unwrap();
            read.unwrap();
            assert_eq!(&received, b"late");
        });
    }
}

in_both_modes! {
    /// A datagram socket made from a std one in blocking mode is switched to non-blocking mode: a
    /// receive with nothing to receive yet waits for the runtime, which is what lets the datagram
    /// that is sent after a delay, from this very thread, arrive. A blocking socket would hold the
    /// thread in the receive for good, and the test's timeout would fail it.
    fn from_std_makes_a_blocking_datagram_socket_non_blocking<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (std_socket, peer) = std::os::unix::net::UnixDatagram::pair().unwrap();
        let socket = UnixDatagram::from_std(&runtime, std_socket).unwrap();

        runtime.block_on(async {
            let mut received = [0; 4];
            let (sent, len) = join(
                after(&runtime, async { peer.send(b"late") }),
                socket.recv(&mut received),
            )
            .await;

            assert_eq!(sent.unwrap(), 4);
            let len = len.unwrap();
            assert_eq!(&received[..len], b"late");
        });
    }
}

in_both_modes! {
    /// A stream handed back by `into_std` is the same connection, still in non-blocking mode: the
    /// std stream names the same two ends, a read with nothing to read yet fails with `WouldBlock`
    /// instead of waiting, what the peer sends is read from it, and what it writes is read by the
    /// peer.
    ///
    /// The mode is looked at before anything else is done to the std stream, by that first read: a
    /// blocking stream would hold the thread in it for good, and the test's timeout would fail it.
    /// The std stream is switched to blocking mode only after that, to read the bytes the peer has
    /// sent by then.
    fn into_std_hands_back_the_same_connection_in_non_blocking_mode<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("into-std");

        runtime.block_on(async {
            let (client, mut server) = connected(&runtime, &directory).await;

            let mut std_client = client.into_std();
            assert!(std_client.local_addr().unwrap().is_unnamed());
            assert_eq!(
                std_client.peer_addr().unwrap().as_pathname(),
                Some(directory.socket("connected").as_path()),
            );

            let error = std_client.read(&mut [0; 1]).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::WouldBlock);

            server.write_all(b"ping").await.unwrap();
            std_client.set_nonblocking(false).unwrap();
            let mut received = [0; 4];
            std_client.read_exact(&mut received).unwrap();
            assert_eq!(&received, b"ping");

            std_client.write_all(b"pong").unwrap();
            server.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"pong");
        });
    }
}

in_both_modes! {
    /// What the peer sent before the stream was handed back is left in the socket for the std
    /// stream to read: handing the socket back takes nothing off it.
    ///
    /// The peer's bytes have arrived by the time the stream goes, since a unix socket is written
    /// straight into the queue of the one it is connected to, and the std stream finds them at
    /// once, in the non-blocking mode it is handed back in.
    fn into_std_leaves_the_bytes_that_arrived_for_the_std_stream<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (client, mut server) = UnixStream::pair(&runtime).unwrap();

        runtime.block_on(async {
            server.write_all(b"early").await.unwrap();

            let mut std_client = client.into_std();
            let mut received = [0; 5];
            std_client.read_exact(&mut received).unwrap();
            assert_eq!(&received, b"early");
        });
    }
}

in_both_modes! {
    /// A socket handed back by `into_std` is watched by nobody: the runtime takes it up again
    /// with `from_std`, which it turns away with `AlreadyExists` for a socket it watches still,
    /// and the new stream is served by the reactor like any other, a read that has to wait
    /// included.
    fn a_socket_handed_back_by_into_std_can_be_taken_up_again<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("into-std-again");

        runtime.block_on(async {
            let (client, mut server) = connected(&runtime, &directory).await;

            let std_client = client.into_std();
            let mut client = UnixStream::from_std(&runtime, std_client).unwrap();

            let mut received = [0; 4];
            let (written, read) = join(
                after(&runtime, server.write_all(b"late")),
                client.read_exact(&mut received),
            )
            .await;

            written.unwrap();
            read.unwrap();
            assert_eq!(&received, b"late");

            client.write_all(b"back").await.unwrap();
            server.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"back");
        });
    }
}

in_both_modes! {
    /// Closing a stream makes the peer read the end of the stream once it has read what was sent,
    /// leaves the other direction open, and closing it again is fine.
    fn close_makes_the_peer_read_the_end_of_the_stream<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("close");

        runtime.block_on(async {
            let (mut client, mut server) = connected(&runtime, &directory).await;

            client.write_all(b"bye").await.unwrap();
            client.close().await.unwrap();

            let mut received = Vec::new();
            server.read_to_end(&mut received).await.unwrap();
            assert_eq!(received, b"bye");

            // Only the write half is shut down: the answer still gets to the client.
            server.write_all(b"ack").await.unwrap();
            let mut answer = [0; 3];
            client.read_exact(&mut answer).await.unwrap();
            assert_eq!(&answer, b"ack");

            client.close().await.unwrap();
        });
    }
}

in_both_modes! {
    /// Closing through a shared reference shuts the socket down, so it ends the stream for every
    /// handle on it: a write through the stream itself fails afterwards, and the peer reads the
    /// end.
    fn closing_through_a_reference_closes_the_stream_for_every_handle<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("reference");

        runtime.block_on(async {
            let (mut client, mut server) = connected(&runtime, &directory).await;

            (&client).close().await.unwrap();

            let error = client.write_all(b"too late").await.unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
            let mut received = Vec::new();
            server.read_to_end(&mut received).await.unwrap();
            assert!(received.is_empty());
        });
    }
}

in_both_modes! {
    /// Shutting the write half down makes the peer read the end of the stream, after the bytes
    /// that were sent before it.
    fn shutdown_of_the_write_half_makes_the_peer_read_the_end<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("shutdown");

        runtime.block_on(async {
            let (mut client, mut server) = connected(&runtime, &directory).await;

            client.write_all(b"last words").await.unwrap();
            client.shutdown(Shutdown::Write).unwrap();

            let mut received = Vec::new();
            server.read_to_end(&mut received).await.unwrap();
            assert_eq!(received, b"last words");
        });
    }
}

in_both_modes! {
    /// A write to a peer that has gone fails with `BrokenPipe`. The stream sends with
    /// `MSG_NOSIGNAL`, on the platforms that have the flag, so the kernel reports the error
    /// instead of raising `SIGPIPE`, which would end the process.
    ///
    /// The writes are made until one fails: the first does on Linux, whose kernel knows at once
    /// that the peer is gone.
    ///
    /// What this pins down is the error, not the lack of the signal. The Rust runtime that starts
    /// the test binary ignores `SIGPIPE`, which `/proc/self/status` shows on Linux with the bit of
    /// the signal set in its `SigIgn` mask, so a send without the flag fails with the same error
    /// here, and the test passes with the flag or without it. Only a process that leaves the
    /// signal at its default can tell the two apart, and a test cannot make itself one without
    /// `libc`. A nightly toolchain makes the whole binary one, with
    /// `RUSTFLAGS=-Zon-broken-pipe=kill`, and then a send without the flag ends the test process
    /// with the signal.
    fn a_write_to_a_peer_that_has_gone_fails_with_broken_pipe<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("gone");

        runtime.block_on(async {
            let (mut client, server) = connected(&runtime, &directory).await;
            drop(server);

            let error = loop {
                if let Err(error) = client.write_all(b"anybody there?").await {
                    break error;
                }
            };

            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
        });
    }
}

in_both_modes! {
    /// A reader and a writer share a stream through two references to it, in one task: 1 MiB
    /// written through one comes back through the other, echoed by the peer, which does so through
    /// two references to its own stream, and arrives intact.
    ///
    /// The echo ends when the client closes its end, as the copy of the peer's reads ends at the
    /// end of the stream, and the peer closes its own end then, which is what ends the client's
    /// reads in turn.
    fn a_reader_and_a_writer_share_a_stream<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("share");

        runtime.block_on(async {
            let (client, server) = connected(&runtime, &directory).await;
            let data = pattern(1 << 20);

            let writer = async {
                (&client).write_all(&data).await.unwrap();
                (&client).close().await.unwrap();
            };
            let reader = async {
                let mut echoed = Vec::new();
                (&client).read_to_end(&mut echoed).await.unwrap();

                echoed
            };
            let echo = async {
                copy(&server, &mut &server).await.unwrap();
                (&server).close().await.unwrap();
            };

            let (((), echoed), ()) = join(join(writer, reader), echo).await;

            assert_eq!(echoed.len(), data.len());
            // Not `assert_eq!`, which would print a megabyte on a failure.
            assert!(echoed == data, "the echo differs from what was sent");
        });
    }
}

in_both_modes! {
    /// A write that finds no room in the socket waits until the peer makes some by reading, and
    /// then completes: its wait is for the socket to become writable, which a socket whose
    /// buffers are full is not.
    ///
    /// The socket is filled first, with writes that are polled once each, until one of them has
    /// to wait. The peer is a std stream that another thread reads from, after a delay, everything
    /// that was written and the waiting write too. Nothing but the socket becoming writable can
    /// wake the write then: a peer read in the same task would wake it by the reads' own
    /// readiness, whatever the write waited for.
    fn a_write_that_waits_resumes_once_the_peer_reads<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (std_stream, mut peer) = std::os::unix::net::UnixStream::pair().unwrap();
        let chunk = pattern(64 * 1024);

        runtime.block_on(async {
            let stream = UnixStream::from_std(&runtime, std_stream).unwrap();

            let mut writer = &stream;
            let mut filled = 0;
            poll_fn(|cx| loop {
                match Pin::new(&mut writer).poll_write(cx, &chunk) {
                    Poll::Ready(written) => filled += written.unwrap(),
                    Poll::Pending => break Poll::Ready(()),
                }
            })
            .await;

            let total = filled + chunk.len();
            let reading = thread::spawn(move || {
                thread::sleep(DELAY);
                peer.read_exact(&mut vec![0; total]).unwrap();
            });

            writer.write_all(&chunk).await.unwrap();
            reading.join().unwrap();
        });
    }
}

in_both_modes! {
    /// A read, an accept or a receive that has to wait is polled when something changes, not over
    /// and over: it waits for the socket to become readable, which a connected socket with
    /// nothing to read, and a listener with nothing to accept, are not, whereas a socket with room
    /// to send is writable all the while, so a wait for that instead would spin the thread.
    ///
    /// A call that waits is polled when it begins to, and again each time its task is woken
    /// while it waits: by the timer that ends the delay, since the task polls both of the calls
    /// that are joined, and by what arrives. A thread that spun would poll it hundreds of times.
    fn a_read_an_accept_or_a_receive_that_waits_does_not_spin<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("spin");
        let path = directory.socket("listener");
        let listener = UnixListener::bind(&runtime, &path).unwrap();
        let (mut client, mut server) = UnixStream::pair(&runtime).unwrap();
        let (sender, receiver) = UnixDatagram::pair(&runtime).unwrap();

        runtime.block_on(async {
            let mut buffer = [0; 1];

            let read_polls = Cell::new(0);
            let (read, written) = join(
                counting_polls(&read_polls, server.read_exact(&mut buffer)),
                after(&runtime, client.write_all(b"1")),
            )
            .await;
            read.unwrap();
            written.unwrap();
            assert!(read_polls.get() <= 10, "the read was polled {} times", read_polls.get());

            let accept_polls = Cell::new(0);
            let (accepted, connection) = join(
                counting_polls(&accept_polls, listener.accept()),
                after(&runtime, UnixStream::connect(&runtime, &path)),
            )
            .await;
            accepted.unwrap();
            connection.unwrap();
            assert!(
                accept_polls.get() <= 10,
                "the accept was polled {} times",
                accept_polls.get(),
            );

            let receive_polls = Cell::new(0);
            let (received, sent) = join(
                counting_polls(&receive_polls, receiver.recv(&mut buffer)),
                after(&runtime, sender.send(b"2")),
            )
            .await;
            received.unwrap();
            sent.unwrap();
            assert!(
                receive_polls.get() <= 10,
                "the `recv` was polled {} times",
                receive_polls.get(),
            );

            let receive_from_polls = Cell::new(0);
            let (received, sent) = join(
                counting_polls(&receive_from_polls, receiver.recv_from(&mut buffer)),
                after(&runtime, sender.send(b"3")),
            )
            .await;
            received.unwrap();
            sent.unwrap();
            assert!(
                receive_from_polls.get() <= 10,
                "the `recv_from` was polled {} times",
                receive_from_polls.get(),
            );
        });
    }
}

in_both_modes! {
    /// Giving up an `accept` that is waiting loses no connection that came in while it waited,
    /// though the runtime woke it for that connection: the next `accept` takes it.
    ///
    /// The connection is made while the `accept` waits, and the task sleeps long enough for the
    /// runtime to see the listener readable, which wakes the task the `accept` waited for, before
    /// the future is dropped without being polled again.
    ///
    /// The streams of a unix socket's connections carry no address that tells one from another, so
    /// the client tells its own by what it writes: the stream the next `accept` returns reads it.
    fn a_dropped_accept_loses_no_connection<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("dropped");
        let path = directory.socket("listener");
        let listener = UnixListener::bind(&runtime, &path).unwrap();

        runtime.block_on(async {
            // The `accept` is dropped at the end of this block, without being polled again.
            let mut client = {
                let mut accept = pin!(listener.accept());
                assert!(poll_immediate(accept.as_mut()).await.is_none());

                let client = UnixStream::connect(&runtime, &path).await.unwrap();
                runtime.sleep(DELAY).await;

                client
            };

            let (mut server, _) = listener.accept().await.unwrap();

            client.write_all(b"here").await.unwrap();
            let mut received = [0; 4];
            server.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"here");
        });
    }
}

in_both_modes! {
    /// `incoming` waits while no connection does, and yields the stream of each connection that
    /// is made, in the order they were made.
    ///
    /// The streams of a unix socket's connections carry no address that tells one client from
    /// another, so each client tells its own by what it writes: the first accepted stream reads
    /// what the first client wrote, and so on.
    fn incoming_yields_one_stream_per_connection<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("incoming");
        let path = directory.socket("listener");
        let listener = UnixListener::bind(&runtime, &path).unwrap();

        runtime.block_on(async {
            let mut incoming = listener.incoming();
            assert!(poll_immediate(incoming.next()).await.is_none());

            // The first connection is made after a delay, with the stream already waiting for it.
            let (first_accepted, first) = join(
                incoming.next(),
                after(&runtime, UnixStream::connect(&runtime, &path)),
            )
            .await;
            let mut first_accepted = first_accepted.expect("the stream never ends").unwrap();
            let mut first = first.unwrap();

            let mut second = UnixStream::connect(&runtime, &path).await.unwrap();
            let mut third = UnixStream::connect(&runtime, &path).await.unwrap();
            let mut second_accepted =
                incoming.next().await.expect("the stream never ends").unwrap();
            let mut third_accepted =
                incoming.next().await.expect("the stream never ends").unwrap();

            first.write_all(b"1").await.unwrap();
            second.write_all(b"2").await.unwrap();
            third.write_all(b"3").await.unwrap();

            let mut tag = [0; 1];
            first_accepted.read_exact(&mut tag).await.unwrap();
            assert_eq!(&tag, b"1");
            second_accepted.read_exact(&mut tag).await.unwrap();
            assert_eq!(&tag, b"2");
            third_accepted.read_exact(&mut tag).await.unwrap();
            assert_eq!(&tag, b"3");
        });
    }
}

in_both_modes! {
    /// Datagrams travel between two sockets bound to paths, and each receive names its sender by
    /// the path that sender is bound to, which is the address to answer to.
    fn datagrams_travel_between_bound_sockets<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("bound");
        let first_path = directory.socket("first");
        let second_path = directory.socket("second");
        let first = UnixDatagram::bind(&runtime, &first_path).unwrap();
        let second = UnixDatagram::bind(&runtime, &second_path).unwrap();

        runtime.block_on(async {
            assert_eq!(first.send_to(b"ping", &second_path).await.unwrap(), 4);
            let mut received = [0; 16];
            let (len, sender) = second.recv_from(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"ping");
            assert_eq!(sender.as_pathname(), Some(first_path.as_path()));

            // The address the datagram came from is one to answer to.
            let sender = sender.as_pathname().unwrap();
            assert_eq!(second.send_to(b"pong", sender).await.unwrap(), 4);
            let (len, sender) = first.recv_from(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"pong");
            assert_eq!(sender.as_pathname(), Some(second_path.as_path()));

            assert_eq!(first.local_addr().unwrap().as_pathname(), Some(first_path.as_path()));
            assert_eq!(second.local_addr().unwrap().as_pathname(), Some(second_path.as_path()));
            // Neither is connected to anything, so neither has a peer to name or to send to.
            assert_eq!(first.peer_addr().unwrap_err().kind(), io::ErrorKind::NotConnected);
            assert!(second.send(b"nowhere").await.is_err());
        });
    }
}

in_both_modes! {
    /// A socket connected to a path sends to it with `send` and receives with `recv`, and the two
    /// ends of the connection name each other as their peers.
    fn a_connected_datagram_socket_sends_and_receives<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("connected");
        let first_path = directory.socket("first");
        let second_path = directory.socket("second");
        let first = UnixDatagram::bind(&runtime, &first_path).unwrap();
        let second = UnixDatagram::bind(&runtime, &second_path).unwrap();
        first.connect(&second_path).unwrap();
        second.connect(&first_path).unwrap();

        runtime.block_on(async {
            assert_eq!(first.send(b"ping").await.unwrap(), 4);
            let mut received = [0; 16];
            let len = second.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"ping");

            assert_eq!(second.send(b"pong").await.unwrap(), 4);
            let len = first.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"pong");

            assert_eq!(first.peer_addr().unwrap().as_pathname(), Some(second_path.as_path()));
            assert_eq!(second.peer_addr().unwrap().as_pathname(), Some(first_path.as_path()));
        });
    }
}

in_both_modes! {
    /// A socket bound to no path sends to a bound one, whose receive sees the sender as unnamed:
    /// there is no path to answer to.
    ///
    /// The unbound socket is connected to nothing either, which tells it from an end of a pair:
    /// that one has no path of its own as well, but has a peer.
    fn an_unbound_socket_sends_to_a_bound_one<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("unbound");
        let path = directory.socket("receiver");
        let receiver = UnixDatagram::bind(&runtime, &path).unwrap();
        let sender = UnixDatagram::unbound(&runtime).unwrap();
        assert!(sender.local_addr().unwrap().is_unnamed());
        assert_eq!(sender.peer_addr().unwrap_err().kind(), io::ErrorKind::NotConnected);

        runtime.block_on(async {
            assert_eq!(sender.send_to(b"hello", &path).await.unwrap(), 5);

            let mut received = [0; 16];
            let (len, from) = receiver.recv_from(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"hello");
            assert!(from.is_unnamed());
        });
    }
}

in_both_modes! {
    /// A datagram sent, or a socket connected, to a path where there is no file fails with
    /// `NotFound`, and to the file of a socket that is gone with `ConnectionRefused`: the kernel
    /// reports both at once, so a send does not wait for a receiver that is not there.
    fn a_datagram_to_a_path_with_no_socket_fails<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("nobody");
        let missing = directory.socket("missing");
        let gone = directory.socket("gone");
        drop(std::os::unix::net::UnixDatagram::bind(&gone).unwrap());
        let socket = UnixDatagram::unbound(&runtime).unwrap();

        runtime.block_on(async {
            let error = socket.send_to(b"anybody?", &missing).await.unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::NotFound);
            let error = socket.send_to(b"anybody?", &gone).await.unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::ConnectionRefused);
        });

        assert_eq!(socket.connect(&missing).unwrap_err().kind(), io::ErrorKind::NotFound);
        assert_eq!(socket.connect(&gone).unwrap_err().kind(), io::ErrorKind::ConnectionRefused);
    }
}

in_both_modes! {
    /// A pair of datagram sockets exchanges datagrams both ways, and each datagram arrives apart
    /// from the others: one receive takes one datagram, of the length it was sent with, and what
    /// does not fit into a buffer that is too short for it is discarded, not left for the next
    /// receive.
    fn a_datagram_pair_keeps_the_datagrams_apart<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (left, right) = UnixDatagram::pair(&runtime).unwrap();

        runtime.block_on(async {
            assert_eq!(left.send(b"one").await.unwrap(), 3);
            assert_eq!(left.send(b"second").await.unwrap(), 6);
            let mut received = [0; 16];
            let len = right.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"one");
            let len = right.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"second");

            assert_eq!(right.send(b"back").await.unwrap(), 4);
            let len = left.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"back");

            assert_eq!(left.send(b"truncated").await.unwrap(), 9);
            assert_eq!(left.send(b"next").await.unwrap(), 4);
            let mut short = [0; 4];
            let len = right.recv(&mut short).await.unwrap();
            assert_eq!(&short[..len], b"trun");
            let len = right.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"next");
        });
    }
}

in_both_modes! {
    /// A receive that finds no datagram waits for the runtime, which is what lets the datagram that
    /// is sent after a delay, from this very thread, arrive: for `recv` and `recv_from` alike.
    fn a_receive_waits_for_a_delayed_send<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (left, right) = UnixDatagram::pair(&runtime).unwrap();

        runtime.block_on(async {
            let mut received = [0; 16];
            let (sent, len) = join(
                after(&runtime, left.send(b"late")),
                right.recv(&mut received),
            )
            .await;
            assert_eq!(sent.unwrap(), 4);
            let len = len.unwrap();
            assert_eq!(&received[..len], b"late");

            let (sent, from) = join(
                after(&runtime, left.send(b"later")),
                right.recv_from(&mut received),
            )
            .await;
            assert_eq!(sent.unwrap(), 5);
            let (len, from) = from.unwrap();
            assert_eq!(&received[..len], b"later");
            assert!(from.is_unnamed());
        });
    }
}

in_both_modes! {
    /// A `send_to` that finds the receiver's queue full waits for room without spinning, and
    /// completes once the receiver has read a datagram: it is polled again each time its retry
    /// interval runs out, which is a handful of times over twice the delay, and not over and over.
    ///
    /// Linux reports a socket that sends to an address of its choosing as writable whether or not
    /// the receiver has room, so a wait for writability would end at once, every time, and the
    /// send would spin the thread for as long as the queue stays full. The queue is filled first,
    /// with datagrams from sockets of std's, until one of them is turned away with `WouldBlock`.
    /// The receiver reads one datagram after twice the delay, which is what makes room, and the
    /// send must not complete before that.
    ///
    /// The spin is Linux's, and so is the way a full queue is reported, with `WouldBlock`, which
    /// is how the test knows that the queue is full: it is written for Linux and Android alone.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn a_send_to_a_full_receiver_waits_without_spinning<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("full");
        let path = directory.socket("receiver");
        let receiver = UnixDatagram::bind(&runtime, &path).unwrap();
        fill_the_queue(&path);
        let sender = UnixDatagram::unbound(&runtime).unwrap();

        runtime.block_on(async {
            let polls = Cell::new(0);
            let made_room = Cell::new(false);
            let send = async {
                let sent = sender.send_to(b"last", &path).await;
                assert!(made_room.get(), "the datagram was sent before the receiver made room");

                sent
            };
            let receive = async {
                runtime.sleep(2 * DELAY).await;
                let mut received = [0; 16];
                receiver.recv(&mut received).await.unwrap();
                made_room.set(true);
            };

            let (sent, ()) = join(counting_polls(&polls, send), receive).await;

            assert_eq!(sent.unwrap(), 4);
            assert!(polls.get() <= 20, "the `send_to` was polled {} times", polls.get());
        });
    }
}

in_both_modes! {
    /// A `send_to` spends the budget of the poll it is made in, and once that is gone it wakes its
    /// task and sends nothing, as an operation that goes through `AsyncIo` does.
    ///
    /// It does not go through `AsyncIo`, as it waits for room on a timer and not for readiness,
    /// so it counts for itself. The poll here is the test's own, which has a budget the way a
    /// task's poll has, spent down to one operation: the send that takes it completes, and the
    /// one after it yields. The datagram of that one is sent, in a poll with a budget of its own,
    /// only after the receiver has been shown to hold the first alone.
    fn a_send_to_spends_the_budget_and_yields_once_it_is_gone<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("budget");
        let path = directory.socket("receiver");
        let receiver = UnixDatagram::bind(&runtime, &path).unwrap();
        let sender = UnixDatagram::unbound(&runtime).unwrap();
        let (woken, waker) = counting_waker();
        let mut cx = Context::from_waker(&waker);

        budgeted(|| {
            for _ in 1..coop::BUDGET {
                spend();
            }

            let mut first = pin!(sender.send_to(b"first", &path));
            assert!(matches!(first.as_mut().poll(&mut cx), Poll::Ready(Ok(5))));
            assert_eq!(woken.count(), 0);

            let mut second = pin!(sender.send_to(b"second", &path));
            assert!(second.as_mut().poll(&mut cx).is_pending());
            assert_eq!(woken.count(), 1);
        });

        runtime.block_on(async {
            let mut received = [0; 16];
            let len = receiver.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"first");
            assert!(poll_immediate(receiver.recv(&mut received)).await.is_none());

            assert_eq!(sender.send_to(b"second", &path).await.unwrap(), 6);
            let len = receiver.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"second");
        });
    }
}

in_both_modes! {
    /// Shutting the write half of a datagram socket down ends its sending, and leaves its
    /// receiving alone: a send fails with `BrokenPipe` afterwards, and the datagram the peer sends
    /// still arrives.
    fn shutdown_of_the_write_half_ends_the_sending_of_datagrams<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (left, right) = UnixDatagram::pair(&runtime).unwrap();

        left.shutdown(Shutdown::Write).unwrap();

        runtime.block_on(async {
            let error = left.send(b"too late").await.unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);

            right.send(b"still open").await.unwrap();
            let mut received = [0; 16];
            let len = left.recv(&mut received).await.unwrap();
            assert_eq!(&received[..len], b"still open");
        });
    }
}

/// A stream built on a shared runtime moves to another thread, which uses it there: that thread
/// drives the runtime with a `block_on` of its own, after the thread that made the stream has left
/// the one it made it in, since a runtime made by `new` is driven by one thread at a time.
#[test]
#[timeout(15000)]
fn a_shared_stream_crosses_threads() {
    let runtime = SharedRuntime::new().unwrap();
    let directory = Directory::new("crossing");
    let (client, mut server) = runtime.block_on(connected(&runtime, &directory));

    let user = thread::spawn({
        let runtime = runtime.clone();
        move || {
            runtime.block_on(async move {
                let mut client = client;
                let mut request = [0; 5];
                // Waits for the bytes, with the reactor of the runtime driven by this thread.
                client.read_exact(&mut request).await.unwrap();
                client
                    .write_all(&request.to_ascii_uppercase())
                    .await
                    .unwrap();
            });
        }
    });

    // This thread is inside no `block_on`, so it leaves the runtime to the other one: a write
    // that has room goes to the kernel without the runtime, and any executor can poll it.
    thread::sleep(DELAY);
    block_on(server.write_all(b"hello")).unwrap();
    user.join().unwrap();

    let mut response = [0; 5];
    runtime.block_on(server.read_exact(&mut response)).unwrap();
    assert_eq!(&response, b"HELLO");
}

/// The tasks of a shared runtime use sockets: their futures are `Send`, which spawning one
/// checks, and the runtime runs them to the end.
#[test]
#[timeout(15000)]
fn shared_tasks_use_sockets() {
    let runtime = SharedRuntime::new().unwrap();
    let directory = Directory::new("tasks");
    let path = directory.socket("listener");
    let listener = UnixListener::bind(&runtime, &path).unwrap();

    runtime.block_on(async {
        let server = runtime.spawn("an echo server", async move {
            let (stream, _) = listener.accept().await.unwrap();
            copy(&stream, &mut &stream).await.unwrap();
            (&stream).close().await.unwrap();
        });
        let client = runtime.spawn("a client", {
            let runtime = runtime.clone();
            async move {
                let mut stream = UnixStream::connect(&runtime, path).await.unwrap();
                stream.write_all(b"hello").await.unwrap();
                stream.close().await.unwrap();

                let mut echoed = Vec::new();
                stream.read_to_end(&mut echoed).await.unwrap();

                echoed
            }
        });

        assert_eq!(client.await.unwrap(), b"hello");
        server.await.unwrap();
    });
}

/// Several tasks wait in `accept` on one listener at once, and each gets a connection of its own:
/// none is left waiting for a wake-up that another's wait took.
///
/// Each client sends a byte of its own once connected, which what each accepted stream reads tells
/// the connections apart by. The accepts run in tasks because a task is polled when it is woken,
/// and only then. The future that `block_on` polls is polled after every wait, woken or not, which
/// would hide a connection that woke the wrong task.
#[test]
#[timeout(15000)]
fn several_tasks_accept_from_one_listener() {
    let runtime = SharedRuntime::new().unwrap();
    let directory = Directory::new("accepts");
    let path = directory.socket("listener");
    let listener = Arc::new(UnixListener::bind(&runtime, &path).unwrap());

    runtime.block_on(async {
        let acceptors: Vec<_> = (0..3)
            .map(|_| {
                let listener = listener.clone();
                runtime.spawn("an acceptor", async move {
                    let (stream, _) = listener.accept().await.unwrap();

                    stream
                })
            })
            .collect();
        // The tasks have begun to wait by the time the sleep is over.
        runtime.sleep(DELAY).await;

        let mut clients = Vec::new();
        for id in 0..3u8 {
            let mut client = UnixStream::connect(&runtime, &path).await.unwrap();
            client.write_all(&[id]).await.unwrap();
            clients.push(client);
        }

        let mut ids = Vec::new();
        for acceptor in acceptors {
            let mut stream = acceptor.await.unwrap();
            let mut id = [0];
            stream.read_exact(&mut id).await.unwrap();
            ids.push(id[0]);
        }
        ids.sort();
        assert_eq!(ids, [0, 1, 2]);
    });
}

/// A task waiting in `accept` and another waiting for the next item of `incoming`, on one listener
/// at once, each get a connection: neither wait takes the place of the other, so neither is left
/// waiting for a wake-up that the other's wait took.
///
/// The waits run in tasks for the reason given at `several_tasks_accept_from_one_listener`: a task
/// is polled when it is woken, and only then.
#[test]
#[timeout(15000)]
fn accept_and_incoming_wait_together() {
    let runtime = SharedRuntime::new().unwrap();
    let directory = Directory::new("together");
    let path = directory.socket("listener");
    let listener = Arc::new(UnixListener::bind(&runtime, &path).unwrap());

    runtime.block_on(async {
        let acceptor = runtime.spawn("an acceptor", {
            let listener = listener.clone();
            async move {
                let (stream, _) = listener.accept().await.unwrap();

                stream
            }
        });
        let streamer = runtime.spawn("an incoming stream", {
            let listener = listener.clone();
            async move {
                listener
                    .incoming()
                    .next()
                    .await
                    .expect("the stream never ends")
                    .unwrap()
            }
        });
        // The tasks have begun to wait by the time the sleep is over.
        runtime.sleep(DELAY).await;

        let mut clients = Vec::new();
        for id in 0..2u8 {
            let mut client = UnixStream::connect(&runtime, &path).await.unwrap();
            client.write_all(&[id]).await.unwrap();
            clients.push(client);
        }

        let mut ids = Vec::new();
        for mut stream in [acceptor.await.unwrap(), streamer.await.unwrap()] {
            let mut id = [0];
            stream.read_exact(&mut id).await.unwrap();
            ids.push(id[0]);
        }
        ids.sort();
        assert_eq!(ids, [0, 1]);
    });
}

/// Two tasks wait in `recv_from` on one datagram socket at once, and each gets one of the two
/// datagrams that are sent: none is left waiting for a wake-up that another's wait took.
///
/// The receives run in tasks for the reason given at `several_tasks_accept_from_one_listener`: a
/// task is polled when it is woken, and only then.
#[test]
#[timeout(15000)]
fn several_tasks_receive_on_one_datagram_socket() {
    let runtime = SharedRuntime::new().unwrap();
    let (client, server) = UnixDatagram::pair(&runtime).unwrap();
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

        client.send(b"first").await.unwrap();
        client.send(b"second").await.unwrap();

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
    /// socket's: the one `AsRawFd` names, and the one a borrow of it reaches.
    fn sockets_print_and_expose_the_descriptors_of_the_std_sockets_they_wrap<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let directory = Directory::new("debug");
        let listener_path = directory.socket("listener");
        let datagram_path = directory.socket("datagram");
        let std_listener = std::os::unix::net::UnixListener::bind(&listener_path).unwrap();
        let std_stream = std::os::unix::net::UnixStream::connect(&listener_path).unwrap();
        let std_datagram = std::os::unix::net::UnixDatagram::bind(&datagram_path).unwrap();
        let printed_listener = format!("{std_listener:?}");
        let printed_stream = format!("{std_stream:?}");
        let printed_datagram = format!("{std_datagram:?}");
        let raw_listener = std_listener.as_raw_fd();
        let raw_stream = std_stream.as_raw_fd();
        let raw_datagram = std_datagram.as_raw_fd();

        let listener = UnixListener::from_std(&runtime, std_listener).unwrap();
        let stream = UnixStream::from_std(&runtime, std_stream).unwrap();
        let datagram = UnixDatagram::from_std(&runtime, std_datagram).unwrap();

        assert_eq!(format!("{listener:?}"), printed_listener);
        assert_eq!(format!("{stream:?}"), printed_stream);
        assert_eq!(format!("{datagram:?}"), printed_datagram);
        assert_eq!(
            format!("{:?}", listener.incoming()),
            format!("Incoming {{ listener: {printed_listener} }}"),
        );

        assert_eq!(listener.as_raw_fd(), raw_listener);
        assert_eq!(stream.as_raw_fd(), raw_stream);
        assert_eq!(datagram.as_raw_fd(), raw_datagram);
        // `SockRef` asks the system about the descriptor it is given, which is how the borrow is
        // seen to be of the socket itself: it finds the addresses that socket has.
        let listener_address = SockRef::from(&listener).local_addr().unwrap();
        assert_eq!(listener_address.as_pathname(), Some(listener_path.as_path()));
        let stream_peer = SockRef::from(&stream).peer_addr().unwrap();
        assert_eq!(stream_peer.as_pathname(), Some(listener_path.as_path()));
        let datagram_address = SockRef::from(&datagram).local_addr().unwrap();
        assert_eq!(datagram_address.as_pathname(), Some(datagram_path.as_path()));
    }
}

/// A shared runtime's sockets are `Send` and `Sync`, and `Unpin`, which the extension traits of
/// futures ask of what they read from, write to or iterate over.
///
/// Only the bounds are checked here, at compile time. That a stream does cross threads, and that
/// the tasks of a shared runtime use sockets, is shown by `a_shared_stream_crosses_threads` and
/// `shared_tasks_use_sockets`.
#[test]
#[timeout(15000)]
fn shared_sockets_are_send_sync_and_unpin() {
    fn sent_and_shared<T>()
    where
        T: Send + Sync + Unpin,
    {
    }

    sent_and_shared::<UnixListener<Shared>>();
    sent_and_shared::<UnixStream<Shared>>();
    sent_and_shared::<UnixDatagram<Shared>>();
    sent_and_shared::<Incoming<'static, Shared>>();
}

/// A local runtime's sockets are `Unpin` too. That they are neither `Send` nor `Sync` is shown by
/// the compile-fail doc tests, since only code that does not compile can show it.
#[test]
#[timeout(15000)]
fn local_sockets_are_unpin() {
    fn unpin<T>()
    where
        T: Unpin,
    {
    }

    unpin::<UnixListener<Local>>();
    unpin::<UnixStream<Local>>();
    unpin::<UnixDatagram<Local>>();
    unpin::<Incoming<'static, Local>>();
}

/// A connected pair: the end that connected, and the end the listener accepted.
///
/// The listener is dropped on the way out, as the pair needs no more than the two ends.
async fn connected<M>(runtime: &Runtime<M>, directory: &Directory) -> (UnixStream<M>, UnixStream<M>)
where
    M: Mode,
{
    let path = directory.socket("connected");
    let listener = UnixListener::bind(runtime, &path).unwrap();
    let client = UnixStream::connect(runtime, &path).await.unwrap();
    let (server, _) = listener.accept().await.unwrap();

    (client, server)
}

/// Sends datagrams to the socket bound to `path` until its queue is full, so that the next one is
/// turned away with `WouldBlock`.
///
/// Each datagram is sent from a socket of its own: the send buffer of one socket may fill before
/// the receiver's queue does, which would turn that socket away while the receiver still had room.
/// A socket that has sent nothing yet is turned away only where the receiver is full.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn fill_the_queue(path: &std::path::Path) {
    // The queue holds `net.unix.max_dgram_qlen` datagrams, which is ten unless the system sets it
    // higher, so a loop this long that has not filled it is a queue that never will be.
    for _ in 0..100_000 {
        let sender = std::os::unix::net::UnixDatagram::unbound().unwrap();
        sender.set_nonblocking(true).unwrap();

        match sender.send_to(b"filler", path) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return,
            Err(e) => panic!("a datagram to the receiver failed: {e}"),
        }
    }

    panic!("the receiver's queue never filled up");
}

/// A name in the abstract namespace for the test `test`, which no other test, in this run of the
/// suite or in another, has.
///
/// Abstract names have no file to put in a directory of the test's own, and are shared by every
/// socket of the network namespace, so the name holds the process id, the test's name and a count
/// of the names the process has made, which tells the two flavours of a test apart.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn abstract_name(test: &str) -> Vec<u8> {
    static MADE: AtomicUsize = AtomicUsize::new(0);

    format!(
        "zruntime-unix-{}-{test}-{}",
        std::process::id(),
        MADE.fetch_add(1, Ordering::Relaxed),
    )
    .into_bytes()
}

/// A directory of a test's own, for its socket files, which is removed, with whatever is in it,
/// when this is dropped.
///
/// Its name holds the process id, the test's name and a count of the directories the process has
/// made, so that no two tests, in this run of the suite or in another, share one.
struct Directory {
    path: PathBuf,
}

impl Directory {
    /// A new directory for the test `test`.
    ///
    /// A socket's path is limited to about a hundred bytes, which the directory's name counts
    /// toward, so `test` is a few words, not the test's whole name.
    fn new(test: &str) -> Self {
        static MADE: AtomicUsize = AtomicUsize::new(0);

        let path = std::env::temp_dir().join(format!(
            "zruntime-unix-{}-{test}-{}",
            std::process::id(),
            MADE.fetch_add(1, Ordering::Relaxed),
        ));
        // The name is this process's own, so a directory by it is one that an earlier process of
        // the same id left behind: a test that timed out never gets to remove its directory.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir(&path).expect("a directory of its own can be made");

        Self { path }
    }

    /// The path of a socket file called `name` in the directory.
    fn socket(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        // A directory that cannot be removed is a stray in the system's temporary directory, and
        // nothing a test that got this far has to answer for.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
