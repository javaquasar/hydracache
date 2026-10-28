//! Tests of the sockets of [`crate::net`], one module per family of them, and what the families'
//! tests have in common.
//!
//! Most of the tests run in both flavours of runtime, through [`in_both_modes`], since a socket
//! means the same on either; the ones that only mean something in one — a stream that moves to
//! another thread, which only a shared one may do — are written for that flavour alone. The
//! helpers here build the situations several families' tests need: a delay that puts a call to
//! sleep before the thing it waits for happens, a count of how often a call is polled while it
//! waits, a loopback address to bind to, a port that refuses connections, and a listener that has
//! room for one connection only.

#[cfg(any(feature = "tcp", feature = "udp"))]
use std::net::{Ipv4Addr, SocketAddr};
use std::{
    cell::Cell,
    future::{Future, poll_fn},
    time::Duration,
};

#[cfg(feature = "tcp")]
use socket2::{Domain, SockAddr, Socket, Type};

use crate::{Mode, Runtime};

#[cfg(feature = "tcp")]
mod tcp;
#[cfg(feature = "udp")]
mod udp;
#[cfg(all(feature = "unix", unix))]
mod unix;

/// Writes the test that follows once per flavour: a module named after it, holding a `local` and
/// a `shared` test that run its body with the mode parameter set to [`Local`](crate::Local) and to
/// [`Shared`](crate::Shared).
///
/// The body builds its runtime with `Runtime::<M>::new()` and drives it with `block_on`, and
/// reaches everything else through the socket types, which both flavours serve. What it cannot do
/// is spawn: the two flavours' `spawn` take different bounds, so a test that spawns is written for
/// one flavour.
macro_rules! in_both_modes {
    ($(#[$attr:meta])* fn $name:ident<$mode:ident>() $body:block) => {
        $(#[$attr])*
        mod $name {
            use super::*;

            #[test]
            #[ntest::timeout(15000)]
            fn local() {
                run::<$crate::Local>();
            }

            #[test]
            #[ntest::timeout(15000)]
            fn shared() {
                run::<$crate::Shared>();
            }

            fn run<$mode>()
            where
                $mode: $crate::Mode,
            $body
        }
    };
}

use in_both_modes;

/// `future`, started once [`DELAY`] has passed on the timer of `runtime`.
async fn after<M, F>(runtime: &Runtime<M>, future: F) -> F::Output
where
    M: Mode,
    F: Future,
{
    runtime.sleep(DELAY).await;

    future.await
}

/// `future`, polled as it is, and each poll of it counted in `polls`.
///
/// A call that waits for the right readiness is polled when it begins to wait and once for each
/// wake after that; one that waits for the wrong readiness is woken at once, over and over, for as
/// long as the other is not ready, which only a count of its polls tells from one that waits.
fn counting_polls<'a, F>(polls: &'a Cell<usize>, future: F) -> impl Future<Output = F::Output> + 'a
where
    F: Future + 'a,
{
    let mut future = Box::pin(future);

    poll_fn(move |cx| {
        polls.set(polls.get() + 1);

        future.as_mut().poll(cx)
    })
}

/// How long a test lets a task wait before the other one does what the first waits for: long
/// enough for the waiting to have begun, and short enough to cost the suite next to nothing.
///
/// The tests that use it are right whichever of the two happens first. What the delay changes is
/// whether the call that waits has to wait at all, which is what they are there to see.
const DELAY: Duration = Duration::from_millis(50);

/// `len` bytes that tell a change of order, or a loss, from the original.
///
/// They cycle through a prime number of values, so that the pattern does not line up with the size
/// of any buffer on the way.
#[cfg(any(feature = "tcp", all(feature = "unix", unix)))]
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// A loopback TCP port that refuses every connection made to it.
///
/// Only a port nothing listens on can refuse a connection, and a test needs one that no other test
/// can be handed while it runs. A socket bound to the port and connected to a listener elsewhere is
/// both: the port is this value's until it is dropped, and a connection attempt at it matches no
/// socket, which the kernel answers with a reset. A socket that is only bound would hold the port
/// as well, but Apple's kernel drops an attempt at a socket that is bound and neither listens nor
/// connects without an answer, so a connect there times out instead of being refused.
#[cfg(feature = "tcp")]
struct RefusedPort {
    socket: Socket,
    // What `socket` is connected to, kept open for as long as `socket` is: closing a listener
    // resets the connections waiting in it.
    _listener: Socket,
}

#[cfg(feature = "tcp")]
impl RefusedPort {
    fn new() -> Self {
        let listener = bound(loopback());
        listener.listen(1).expect("a bound socket can listen");
        let address = listener
            .local_addr()
            .expect("a bound socket has an address");
        let socket = bound(loopback());
        socket
            .connect(&address)
            .expect("a loopback listener with room takes a connection");

        Self {
            socket,
            _listener: listener,
        }
    }

    /// The address a connection is refused at.
    fn address(&self) -> SocketAddr {
        self.socket
            .local_addr()
            .expect("a bound socket has an address")
            .as_socket()
            .expect("a TCP socket has an IP address")
    }
}

/// A TCP socket listening on a loopback port, with room for one waiting connection and no more,
/// and the address it listens at.
///
/// A connection beyond that one is not refused: the kernel leaves it unanswered, so it stays
/// pending until the listener has taken the one in the queue out of it. That is how Linux and
/// Android queue connections, where the queue holds one connection more than the backlog says and
/// a backlog of zero is a queue of one. Other platforms' queues hold more than they are asked to,
/// so a connection there would not be left pending, and the tests that need it are written for
/// these two alone.
#[cfg(all(feature = "tcp", any(target_os = "linux", target_os = "android")))]
fn listener_with_room_for_one() -> (Socket, SocketAddr) {
    let socket = bound(loopback());
    socket.listen(0).expect("a bound socket can listen");
    let address = socket
        .local_addr()
        .expect("a bound socket has an address")
        .as_socket()
        .expect("a TCP socket has an IP address");

    (socket, address)
}

/// The loopback address a socket binds to, with port `0` for the system to pick a free port.
#[cfg(any(feature = "tcp", feature = "udp"))]
fn loopback() -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, 0))
}

/// A TCP socket bound to `address`, and not yet listening.
#[cfg(feature = "tcp")]
fn bound(address: SocketAddr) -> Socket {
    let socket = Socket::new(Domain::for_address(address), Type::STREAM, None)
        .expect("a TCP socket can be made");
    socket
        .bind(&SockAddr::from(address))
        .expect("a loopback port can be bound");

    socket
}
