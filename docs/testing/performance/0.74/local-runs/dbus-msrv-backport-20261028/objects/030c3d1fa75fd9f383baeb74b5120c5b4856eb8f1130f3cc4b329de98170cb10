//! Tests of [`AsyncIo`], the async handle of any source a runtime watches: over loopback sockets,
//! which every platform has, and, on unix, over pipes, unix-domain sockets and the standard I/O of
//! a child process.
//!
//! Most of the tests run in both flavours of runtime, through [`in_both_modes`], since a handle
//! means the same on either; the ones that only mean something in one — a source that is neither
//! `Send` nor `Sync`, which only a local runtime takes, and a thread that drives a runtime while
//! another gives up its source, which only a shared runtime has — are written for that flavour
//! alone.
//!
//! The first tests are of making a handle: that `new` puts its source in non-blocking mode and
//! `new_nonblocking` leaves the mode as it was. Then come the tests of `read_with` and
//! `write_with`: that they carry bytes between two handles, wait for room as well as for bytes,
//! make an interrupted call again at once and hand any other error back. After them come the
//! tests of what a wait does: that any number of tasks read from one handle or wait for its
//! readiness at once, that `readable` and `writable` wait for the right direction, and that a wait
//! given up leaves the others alone. The tests of the `AsyncRead` and `AsyncWrite` traits follow,
//! through the handle and through a reference to it. Then come the tests of how a handle shares the
//! thread with the other tasks: that a task whose I/O is always ready yields to the next once it
//! has spent the budget of operations of its poll, through the traits, `read_with` or `write_with`,
//! however many futures it polls side by side, and so does the future of `block_on`; that a future
//! polled by something other than a runtime has no budget, while a blocking executor that a task
//! runs shares the task's, and gets its future done inside `unconstrained`; and that the budget
//! ends with the poll it was given for. After them come those of `into_inner`, of the kinds of
//! source a handle takes, and last, of the handle's `Debug`, descriptors and auto traits.

#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, AsSocket, BorrowedSocket, OwnedSocket};
use std::{
    cell::Cell,
    future::{Future, poll_fn},
    io::{self, IoSlice, IoSliceMut, Read, Write},
    net::{Ipv4Addr, TcpListener, TcpStream, UdpSocket},
    panic::{AssertUnwindSafe, catch_unwind},
    pin::pin,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};
#[cfg(unix)]
use std::{
    io::{PipeReader, PipeWriter},
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
    process::{Command, Stdio},
};

use futures::{
    AsyncReadExt, AsyncWriteExt,
    future::{join, select_all},
};
use ntest::timeout;
use socket2::SockRef;

use super::{or, yield_now};
use crate::{
    AsyncIo, Local, LocalRuntime, Mode, Readiness, Runtime, Shared, SharedRuntime, coop,
    unconstrained,
};

/// Writes the test that follows once per flavour: a module named after it, holding a `local` and
/// a `shared` test that run its body with the type its parameter names standing for
/// [`Local`] and for [`Shared`].
///
/// The body is written out once for each flavour rather than once over a mode parameter. A
/// parameter cannot promise what `AsyncIo::new` asks of a source, which depends on the flavour, nor
/// what `spawn` asks of a future, so a body that is written out for each meets the bounds of
/// whichever flavour it runs on. What it uses has to meet both, though: a source that is `Send`
/// and `Sync`, and, for a task, a future that is `Send`.
///
/// A test whose tasks share one handle names the pointer they share it by as well, a second
/// parameter that stands for `Rc` in the local test and for `Arc` in the shared one: a handle of a
/// local runtime cannot go in an `Arc`, and one of a shared runtime cannot go in an `Rc`.
macro_rules! in_both_modes {
    ($(#[$attr:meta])* fn $name:ident<$mode:ident>() $body:block) => {
        $(#[$attr])*
        mod $name {
            use super::*;

            #[test]
            #[timeout(15000)]
            fn local() {
                type $mode = Local;

                $body
            }

            #[test]
            #[timeout(15000)]
            fn shared() {
                type $mode = Shared;

                $body
            }
        }
    };
    ($(#[$attr:meta])* fn $name:ident<$mode:ident, $pointer:ident>() $body:block) => {
        $(#[$attr])*
        mod $name {
            use super::*;

            #[test]
            #[timeout(15000)]
            fn local() {
                type $mode = Local;
                use std::rc::Rc as $pointer;

                $body
            }

            #[test]
            #[timeout(15000)]
            fn shared() {
                type $mode = Shared;
                use std::sync::Arc as $pointer;

                $body
            }
        }
    };
}

in_both_modes! {
    /// `new` puts a source that blocks into non-blocking mode: on unix the mode is read off the
    /// descriptor, and on every platform a read of a source with nothing to read fails at once
    /// where it would otherwise wait for a byte that nobody sends.
    fn new_puts_a_blocking_source_in_non_blocking_mode<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair();
        #[cfg(unix)]
        assert!(!SockRef::from(&near).nonblocking().unwrap());

        let near = AsyncIo::new(&runtime, near).unwrap();

        #[cfg(unix)]
        assert!(SockRef::from(&near).nonblocking().unwrap());
        let mut stream = near.get_ref();
        let error = stream.read(&mut [0]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    }
}

in_both_modes! {
    /// `new_nonblocking` leaves the mode as it was: a source that blocks still does, and one
    /// that does not still does not.
    ///
    /// The mode can only be read off a descriptor, which is what unix has.
    #[cfg(unix)]
    fn new_nonblocking_leaves_the_mode_as_it_was<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (blocking, _blocking_far) = tcp_pair();
        let (non_blocking, _non_blocking_far) = tcp_pair();
        non_blocking.set_nonblocking(true).unwrap();

        let blocking = AsyncIo::new_nonblocking(&runtime, blocking).unwrap();
        let non_blocking = AsyncIo::new_nonblocking(&runtime, non_blocking).unwrap();

        assert!(!SockRef::from(&blocking).nonblocking().unwrap());
        assert!(SockRef::from(&non_blocking).nonblocking().unwrap());
    }
}

in_both_modes! {
    /// A source put in non-blocking mode by its owner is taken as it is by `new_nonblocking`,
    /// and then does its I/O through the handle as one that `new` made does.
    fn new_nonblocking_takes_a_source_that_is_in_non_blocking_mode_already<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, mut far) = tcp_pair();
        near.set_nonblocking(true).unwrap();
        let near = AsyncIo::new_nonblocking(&runtime, near).unwrap();

        runtime.block_on(async {
            // Written once the read below has had to wait for it.
            let (received, ()) = join(read_exactly(&near, 5), async {
                runtime.sleep(DELAY).await;
                far.write_all(b"hello").unwrap();
            })
            .await;

            assert_eq!(received, b"hello");
        });
    }
}

in_both_modes! {
    /// `read_with` and `write_with` carry bytes between two handles, in both directions: a read
    /// that starts before the bytes are there waits for them, and one that starts after finds them
    /// at once.
    fn read_with_and_write_with_carry_bytes_between_two_asyncs<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, far) = tcp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();
        let far = AsyncIo::new(&runtime, far).unwrap();

        runtime.block_on(async {
            let (received, ()) = join(read_exactly(&far, 4), async {
                runtime.sleep(DELAY).await;
                write_everything(&near, b"ping").await;
            })
            .await;
            assert_eq!(received, b"ping");

            write_everything(&far, b"pong").await;
            assert_eq!(read_exactly(&near, 4).await, b"pong");
        });
    }
}

in_both_modes! {
    /// A write of more bytes than the socket has room for waits for the peer to make room, and
    /// every byte arrives, in order, for all the waiting in between.
    fn a_write_larger_than_the_socket_buffers_waits_for_room<M>() {
        // More than the buffers of both ends hold between them, which is a few megabytes at the
        // most, so that the writer runs out of room: one thread runs both ends, so the reader has
        // no turn until the writer has to wait.
        const LEN: usize = 16 << 20;
        // Written a piece at a time: Windows may take a single write whole, however large, into
        // a buffer that has room for some of it, and a write that never has to wait shows
        // nothing. A run of them fills the buffers there as everywhere else.
        const PIECE: usize = 64 << 10;

        let runtime = Runtime::<M>::new().unwrap();
        let (near, far) = tcp_pair();
        let writer = AsyncIo::new(&runtime, near).unwrap();
        let reader = AsyncIo::new(&runtime, far).unwrap();
        let bytes = pattern(LEN);
        let blocked = Cell::new(0);

        let (received, ()) = runtime.block_on(join(read_exactly(&reader, LEN), async {
            let mut sent = 0;
            while sent < LEN {
                sent += writer
                    .write_with(|mut stream| {
                        let result = stream.write(&bytes[sent..LEN.min(sent + PIECE)]);
                        if would_block(&result) {
                            blocked.set(blocked.get() + 1);
                        }

                        result
                    })
                    .await
                    .unwrap();
            }
        }));

        assert!(
            received == bytes,
            "the bytes arrived changed or out of order"
        );
        assert!(blocked.get() > 0, "the writer never ran out of room");
    }
}

in_both_modes! {
    /// An operation the kernel interrupted is made again at once, not after a wait for readiness,
    /// whichever of the four calls runs it: the source here is never readable, so a read that
    /// waited for it would never end.
    fn an_interrupted_operation_is_made_again_without_waiting<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();
        // Interrupted twice, and then a success that is the number of calls it took so far.
        let calls = Cell::new(0);
        let interrupted_twice = |_: &TcpStream| {
            calls.set(calls.get() + 1);

            if calls.get() % 3 == 0 {
                Ok(calls.get())
            } else {
                Err(io::ErrorKind::Interrupted.into())
            }
        };

        let results = runtime.block_on(async {
            let read = near.read_with(&interrupted_twice).await.unwrap();
            let write = near.write_with(&interrupted_twice).await.unwrap();
            let polled_read = poll_fn(|cx| near.poll_read_with(cx, &interrupted_twice));
            let polled_write = poll_fn(|cx| near.poll_write_with(cx, &interrupted_twice));

            [
                read,
                write,
                polled_read.await.unwrap(),
                polled_write.await.unwrap(),
            ]
        });

        assert_eq!(results, [3, 6, 9, 12]);
    }
}

in_both_modes! {
    /// An error other than `WouldBlock` is what the call resolves to, at once and unchanged.
    fn an_operation_that_fails_resolves_to_its_error<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();

        runtime.block_on(async {
            let error = near
                .read_with(|_| Err::<(), _>(io::ErrorKind::BrokenPipe.into()))
                .await
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);

            let error = near
                .write_with(|_| Err::<(), _>(io::Error::new(io::ErrorKind::TimedOut, "too slow")))
                .await
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::TimedOut);
            assert_eq!(error.to_string(), "too slow");
        });
    }
}

in_both_modes! {
    /// Several tasks wait in `read_with` on one handle at once, and each datagram sent to it
    /// completes exactly one of them: the others see that the one they were woken for is gone and
    /// wait again.
    ///
    /// A datagram socket makes each read take exactly one message, so that the count is the same
    /// on every run.
    fn several_tasks_read_with_on_one_async_at_once<M, P>() {
        const TASKS: usize = 4;

        let runtime = Runtime::<M>::new().unwrap();
        let (near, far) = udp_pair();
        let receiver = P::new(AsyncIo::new(&runtime, near).unwrap());
        let waiting = Arc::new(AtomicUsize::new(0));
        let received = Arc::new(AtomicUsize::new(0));
        let tasks: Vec<_> = (0..TASKS)
            .map(|_| {
                let (receiver, waiting, received) =
                    (receiver.clone(), waiting.clone(), received.clone());

                runtime.spawn("a reader", async move {
                    // Each task goes on from here to its wait without yielding, so that a task
                    // counted is a task that waits.
                    waiting.fetch_add(1, Ordering::SeqCst);
                    let mut datagram = [0];
                    let len = receiver
                        .read_with(|socket| socket.recv(&mut datagram))
                        .await
                        .unwrap();
                    received.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(len, 1);

                    datagram[0]
                })
            })
            .collect();

        let mut datagrams = runtime.block_on(async {
            while waiting.load(Ordering::SeqCst) < TASKS {
                yield_now().await;
            }
            for datagram in 0..TASKS {
                far.send(&[datagram as u8]).unwrap();
                // One of the tasks takes it, and the others wait for the next.
                while received.load(Ordering::SeqCst) <= datagram {
                    yield_now().await;
                }
            }

            let mut datagrams = Vec::new();
            for task in tasks {
                datagrams.push(task.await.unwrap());
            }

            datagrams
        });

        datagrams.sort();
        assert_eq!(datagrams, [0, 1, 2, 3]);
    }
}

in_both_modes! {
    /// Several tasks wait for one handle to become readable at once, and one write completes
    /// them all: readiness is for every waiter, where a byte is for one reader.
    ///
    /// None of them completes before there is something to read, and the wait of a task that
    /// starts after the bytes arrived completes as well, as the bytes are still there.
    fn several_tasks_wait_for_readable_at_once<M, P>() {
        const TASKS: usize = 4;

        let runtime = Runtime::<M>::new().unwrap();
        let (near, mut far) = tcp_pair();
        let reader = P::new(AsyncIo::new(&runtime, near).unwrap());
        let waiting = Arc::new(AtomicUsize::new(0));
        let tasks: Vec<_> = (0..TASKS)
            .map(|_| {
                let (reader, waiting) = (reader.clone(), waiting.clone());

                runtime.spawn("a waiter", async move {
                    // Each task goes on from here to its wait without yielding, so that a task
                    // counted is a task that waits.
                    waiting.fetch_add(1, Ordering::SeqCst);
                    reader.readable().await.unwrap();
                })
            })
            .collect();

        runtime.block_on(async {
            while waiting.load(Ordering::SeqCst) < TASKS {
                yield_now().await;
            }
            for _ in 0..5 {
                yield_now().await;
            }
            assert!(tasks.iter().all(|task| !task.is_finished()));

            far.write_all(b"!").unwrap();
            for task in tasks {
                task.await.unwrap();
            }
            // Nothing took the byte, so the source is still ready.
            reader.readable().await.unwrap();
        });
    }
}

in_both_modes! {
    /// `writable` completes on a socket that has room to write in, as every fresh one has.
    fn writable_completes_on_a_fresh_socket<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();

        runtime.block_on(near.writable()).unwrap();
    }
}

in_both_modes! {
    /// `writable` waits for room, not for bytes: while the socket's buffers are full it stays
    /// pending, though the socket has bytes to read in it, and it completes once the peer reads
    /// enough of what was written to make room.
    ///
    /// Other kernels report a socket writable on any room at all, and the acknowledgements of the
    /// bytes in flight when a write found none can bring some a moment later, which leaves the
    /// wait that stays pending something they do not promise. The test is written for Linux and
    /// Android, whose sockets are writable once a good part of what they hold is free.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn writable_waits_while_the_socket_has_no_room<M>() {
        use std::pin::pin;

        use futures::future::poll_immediate;

        let runtime = Runtime::<M>::new().unwrap();
        let (near, mut far) = tcp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();
        // Something to read in the socket that the wait for room is not to be taken for.
        far.write_all(b"!").unwrap();
        let filled = fill(&near);

        runtime.block_on(async {
            let mut writable = pin!(near.writable());
            for _ in 0..5 {
                assert!(poll_immediate(writable.as_mut()).await.is_none());
                yield_now().await;
            }

            // Read from a plain socket with the bytes on their way, which does not wait for the
            // runtime.
            far.read_exact(&mut vec![0; filled]).unwrap();
            writable.await.unwrap();
        });
    }
}

in_both_modes! {
    /// A `read_with` that lost a race to a timer, and was dropped for it, leaves nothing behind
    /// that keeps a later one from completing.
    fn a_dropped_read_with_does_not_keep_a_later_one_from_completing<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, mut far) = tcp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();

        runtime.block_on(async {
            let outcome = or(async { Some(read_exactly(&near, 1).await) }, async {
                runtime.sleep(DELAY).await;

                None
            })
            .await;
            assert!(outcome.is_none());

            far.write_all(b"!").unwrap();
            assert_eq!(read_exactly(&near, 1).await, b"!");
        });
    }
}

in_both_modes! {
    /// Several tasks wait in `write_with` on one handle with no room at once, and every one of
    /// them completes once the peer makes room.
    ///
    /// Only Linux and Android keep a socket with no room that way until the peer reads, which the
    /// check that every task is still waiting before then relies on.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn several_tasks_write_with_on_one_async_at_once<M, P>() {
        const TASKS: usize = 3;

        let runtime = Runtime::<M>::new().unwrap();
        let (near, mut far) = tcp_pair();
        let writer = P::new(AsyncIo::new(&runtime, near).unwrap());
        let filled = fill(&writer);
        let waiting = Arc::new(AtomicUsize::new(0));
        let tasks: Vec<_> = (0..TASKS)
            .map(|_| {
                let (writer, waiting) = (writer.clone(), waiting.clone());

                runtime.spawn("a writer", async move {
                    // Each task goes on from here to its wait without yielding, so that a task
                    // counted is a task that waits.
                    waiting.fetch_add(1, Ordering::SeqCst);

                    writer
                        .write_with(|mut stream| stream.write(&[1]))
                        .await
                        .unwrap()
                })
            })
            .collect();

        runtime.block_on(async {
            while waiting.load(Ordering::SeqCst) < TASKS {
                yield_now().await;
            }
            // A few rounds of the runtime, in which a task that did not wait would finish.
            for _ in 0..5 {
                yield_now().await;
            }
            assert!(tasks.iter().all(|task| !task.is_finished()));
            far.read_exact(&mut vec![0; filled]).unwrap();

            for task in tasks {
                assert_eq!(task.await.unwrap(), 1);
            }
        });
    }
}

in_both_modes! {
    /// Giving up one of two waits on a handle leaves the other waiting: the byte that arrives
    /// afterwards completes it.
    ///
    /// The two waits are in tasks of their own, and the one given up is cancelled, which is over
    /// once its future is gone, before the byte is sent. What completes the other is then the
    /// wake of the runtime and nothing else, as no poll of it comes from anywhere but there.
    fn giving_up_one_wait_leaves_the_other_waiting<M, P>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, mut far) = tcp_pair();
        let reader = P::new(AsyncIo::new(&runtime, near).unwrap());
        let waiting = Arc::new(AtomicUsize::new(0));
        let mut waits = (0..2).map(|_| {
            let (reader, waiting) = (reader.clone(), waiting.clone());

            runtime.spawn("a reader", async move {
                // Each task goes on from here to its wait without yielding, so that a task
                // counted is a task that waits.
                waiting.fetch_add(1, Ordering::SeqCst);

                read_exactly(&reader, 1).await
            })
        });
        // The wait stored first is the one given up, so that it is not simply the last one in.
        let (given_up, kept) = (waits.next().unwrap(), waits.next().unwrap());

        runtime.block_on(async {
            while waiting.load(Ordering::SeqCst) < 2 {
                yield_now().await;
            }

            assert_eq!(given_up.cancel().await, None);
            far.write_all(b"!").unwrap();

            assert_eq!(kept.await.unwrap(), b"!");
        });
    }
}

in_both_modes! {
    /// A handle is written to and read from through the traits of `futures-io`: a read that
    /// starts before the bytes are there waits for them, and the bytes come out as they went in.
    fn the_io_traits_carry_bytes_through_an_async<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, far) = tcp_pair();
        let mut near = AsyncIo::new(&runtime, near).unwrap();
        let mut far = AsyncIo::new(&runtime, far).unwrap();

        runtime.block_on(async {
            let mut received = [0; 5];
            let (read, written) = join(far.read_exact(&mut received), async {
                runtime.sleep(DELAY).await;
                near.write_all(b"hello").await
            })
            .await;

            read.unwrap();
            written.unwrap();
            assert_eq!(&received, b"hello");
        });
    }
}

in_both_modes! {
    /// A read goes on to the end of the stream, which is where the peer went away: the bytes
    /// written before that come out first, and a read after them finds nothing more.
    fn reading_through_the_traits_goes_on_to_the_end_of_the_stream<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, far) = tcp_pair();
        let mut near = AsyncIo::new(&runtime, near).unwrap();
        let mut far = AsyncIo::new(&runtime, far).unwrap();

        runtime.block_on(async {
            near.write_all(b"the end").await.unwrap();
            // The socket closes as the handle is dropped, which is the end of what is read.
            drop(near);

            let mut received = Vec::new();
            far.read_to_end(&mut received).await.unwrap();
            assert_eq!(received, b"the end");
            assert_eq!(far.read(&mut [0; 8]).await.unwrap(), 0);
        });
    }
}

in_both_modes! {
    /// A reader and a writer share one handle through references to it, each end doing both at
    /// once: the two directions of a handle wait independently of one another.
    fn a_reader_and_a_writer_share_one_async_through_references<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, far) = tcp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();
        let far = AsyncIo::new(&runtime, far).unwrap();
        let (mut near_reader, mut near_writer) = (&near, &near);
        let (mut far_reader, mut far_writer) = (&far, &far);
        let (mut at_near, mut at_far) = ([0; 4], [0; 4]);

        let ((near_read, near_written), (far_read, far_written)) = runtime.block_on(join(
            join(near_reader.read_exact(&mut at_near), async {
                runtime.sleep(DELAY).await;
                near_writer.write_all(b"ping").await
            }),
            join(far_reader.read_exact(&mut at_far), async {
                runtime.sleep(DELAY).await;
                far_writer.write_all(b"pong").await
            }),
        ));

        near_read.unwrap();
        near_written.unwrap();
        far_read.unwrap();
        far_written.unwrap();
        assert_eq!(&at_near, b"pong");
        assert_eq!(&at_far, b"ping");
    }
}

in_both_modes! {
    /// A vectored write takes the bytes of every buffer, in order, and a vectored read puts them
    /// in the buffers the same way, the first filled before the second is started on.
    fn vectored_reads_and_writes_carry_every_buffer<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, far) = tcp_pair();
        let mut near = AsyncIo::new(&runtime, near).unwrap();
        let mut far = AsyncIo::new(&runtime, far).unwrap();

        runtime.block_on(async {
            let written = near
                .write_vectored(&[IoSlice::new(b"ab"), IoSlice::new(b"cde")])
                .await
                .unwrap();
            assert_eq!(written, 5);

            let mut received = Vec::new();
            while received.len() < 5 {
                let (mut first, mut second) = ([0; 2], [0; 3]);
                let read = far
                    .read_vectored(&mut [IoSliceMut::new(&mut first), IoSliceMut::new(&mut second)])
                    .await
                    .unwrap();
                assert_ne!(read, 0, "the stream ended early");

                let in_first = read.min(first.len());
                received.extend_from_slice(&first[..in_first]);
                received.extend_from_slice(&second[..read - in_first]);
            }

            assert_eq!(received, b"abcde");
        });
    }
}

in_both_modes! {
    /// Flushing completes, and closing flushes and leaves the source open: a write through
    /// the source after it still goes out, and the peer sees no end of the stream.
    ///
    /// Closing through a reference to the handle does the same as closing the handle.
    fn closing_flushes_and_leaves_the_source_open<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, mut far) = tcp_pair();
        let mut near = AsyncIo::new(&runtime, near).unwrap();

        runtime.block_on(async {
            near.write_all(b"one ").await.unwrap();
            near.flush().await.unwrap();
            near.close().await.unwrap();
            let mut source = near.get_ref();
            source.write_all(b"two ").unwrap();

            let mut reference = &near;
            reference.write_all(b"three ").await.unwrap();
            reference.close().await.unwrap();
            source.write_all(b"four").unwrap();
        });

        let mut received = [0; 18];
        far.read_exact(&mut received).unwrap();
        assert_eq!(&received, b"one two three four");
    }
}

in_both_modes! {
    /// A task that reads a byte at a time through the traits, from a source that always has one
    /// ready, gives the next task its turn once its budget is spent, not once the last byte is
    /// read.
    ///
    /// Without the budget the reader would never return `Pending`, and the task spawned right after
    /// it would first run when the reader was done. The look is exact: the reader is stopped by the
    /// budget and by nothing else, as it has more bytes to read than that.
    fn a_task_reading_through_the_traits_from_a_ready_source_yields_to_the_next<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair_holding(READS);
        let mut near = AsyncIo::new(&runtime, near).unwrap();
        let (reads, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let reader = {
            let reads = reads.clone();
            runtime.spawn("a reader", async move {
                for _ in 0..READS {
                    near.read_exact(&mut [0]).await.unwrap();
                    reads.fetch_add(1, Ordering::SeqCst);
                }
            })
        };
        // Spawned right after the reader, so that it is next in the queue.
        let looker = runtime.spawn("a looker", note_progress(reads.clone(), seen.clone()));

        runtime.block_on(async {
            reader.await.unwrap();
            looker.await.unwrap();
        });

        assert_eq!(seen.load(Ordering::SeqCst), BUDGET);
        assert_eq!(reads.load(Ordering::SeqCst), READS);
    }
}

in_both_modes! {
    /// A task that reads a byte at a time through `read_with`, from a source that always has one
    /// ready, gives the next task its turn once its budget is spent.
    ///
    /// `read_with` does not go through `poll_io`, as the traits do, so it spends the budget itself.
    fn a_task_in_read_with_on_a_ready_source_yields_to_the_next<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair_holding(READS);
        let near = AsyncIo::new(&runtime, near).unwrap();
        let (reads, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let reader = {
            let reads = reads.clone();
            runtime.spawn("a reader", async move {
                for _ in 0..READS {
                    near.read_with(|mut stream| stream.read(&mut [0])).await.unwrap();
                    reads.fetch_add(1, Ordering::SeqCst);
                }
            })
        };
        let looker = runtime.spawn("a looker", note_progress(reads.clone(), seen.clone()));

        runtime.block_on(async {
            reader.await.unwrap();
            looker.await.unwrap();
        });

        assert_eq!(seen.load(Ordering::SeqCst), BUDGET);
        assert_eq!(reads.load(Ordering::SeqCst), READS);
    }
}

in_both_modes! {
    /// A task that sends a datagram at a time through `write_with`, to a socket that always has
    /// room for one, gives the next task its turn once its budget is spent.
    ///
    /// A datagram socket takes each send at once, and drops what its peer has no room for rather
    /// than make the sender wait, so that no send finds the socket not ready.
    fn a_task_in_write_with_on_a_ready_source_yields_to_the_next<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = udp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();
        let (writes, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let writer = {
            let writes = writes.clone();
            runtime.spawn("a writer", async move {
                for _ in 0..READS {
                    near.write_with(|socket| socket.send(&[0])).await.unwrap();
                    writes.fetch_add(1, Ordering::SeqCst);
                }
            })
        };
        let looker = runtime.spawn("a looker", note_progress(writes.clone(), seen.clone()));

        runtime.block_on(async {
            writer.await.unwrap();
            looker.await.unwrap();
        });

        assert_eq!(seen.load(Ordering::SeqCst), BUDGET);
        assert_eq!(writes.load(Ordering::SeqCst), READS);
    }
}

in_both_modes! {
    /// A task that reads a byte at a time through more futures than its budget has operations,
    /// which `select_all` polls side by side, from a source that always has one ready, gives the
    /// next task its turn once the budget is spent, as a task with a single reader does.
    ///
    /// `select_all` polls each of its readers in turn, within the one poll of the task and with the
    /// task's waker. The first reader spends the budget, and every reader after it finds the budget
    /// spent and yields, the last one as well as the second, so that a poll ends with no more reads
    /// than its budget, however many futures it polls. The first reader is done after as many polls
    /// as its reads take, and the others never read.
    fn a_task_that_selects_over_more_readers_than_its_budget_yields_to_the_next<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair_holding(2 * READS);
        let near = AsyncIo::new(&runtime, near).unwrap();
        let (reads, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let reader = {
            let reads = reads.clone();
            runtime.spawn("a reader", async move {
                let (near, reads) = (&near, &reads);
                let readers = (0..BUDGET + 1).map(|_| {
                    Box::pin(async move {
                        for _ in 0..READS {
                            near.read_with(|mut stream| stream.read(&mut [0]))
                                .await
                                .unwrap();
                            reads.fetch_add(1, Ordering::SeqCst);
                        }
                    })
                });
                select_all(readers).await;
            })
        };
        let looker = runtime.spawn("a looker", note_progress(reads.clone(), seen.clone()));

        runtime.block_on(async {
            reader.await.unwrap();
            looker.await.unwrap();
        });

        assert_eq!(seen.load(Ordering::SeqCst), BUDGET);
        assert_eq!(reads.load(Ordering::SeqCst), READS);
    }
}

in_both_modes! {
    /// The future passed to `block_on` has a budget as a task has: a task spawned from it runs
    /// after its budget's worth of reads, though the future itself would read on to the end.
    ///
    /// The tasks run only between the polls of that future, so one that does not return would hold
    /// them all up.
    fn the_future_of_block_on_yields_to_the_tasks_after_the_budget<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair_holding(READS);
        let near = AsyncIo::new(&runtime, near).unwrap();
        let (reads, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));

        runtime.block_on(async {
            let looker = runtime.spawn("a looker", note_progress(reads.clone(), seen.clone()));
            for _ in 0..READS {
                near.read_with(|mut stream| stream.read(&mut [0])).await.unwrap();
                reads.fetch_add(1, Ordering::SeqCst);
            }
            looker.await.unwrap();
        });

        assert_eq!(seen.load(Ordering::SeqCst), BUDGET);
        assert_eq!(reads.load(Ordering::SeqCst), READS);
    }
}

in_both_modes! {
    /// A future that something other than a runtime polls has no budget: it does all its reads in
    /// the one poll, whether they go through `read_with` or through the traits.
    ///
    /// The poll here is made by the test, with a waker that does nothing, so that a future that
    /// yielded would be left without anything to wake it.
    fn a_future_polled_outside_a_runtime_is_never_held_to_the_budget<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair_holding(2 * READS);
        let mut near = AsyncIo::new(&runtime, near).unwrap();

        assert!(reads_in_one_poll(&near, READS));

        let mut reading = pin!(async {
            for _ in 0..READS {
                near.read_exact(&mut [0]).await.unwrap();
            }
        });
        let polled = reading.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        assert!(polled.is_ready());
    }
}

in_both_modes! {
    /// A blocking executor of another crate that a task runs gets its future done when that
    /// future runs inside `unconstrained`, though the executor has no budget of its own and
    /// shares what is left of the task's.
    ///
    /// The task reads ten bytes and then has `futures`'s `block_on` read the rest, many times
    /// more than the budget. That executor polls again at once whenever a read that yields wakes
    /// it, inside the same poll of the task, so a read that found the budget spent would be
    /// turned away for good. Inside `unconstrained` none is, and the task holds the thread until
    /// the last byte is read.
    fn a_blocking_executor_that_a_task_runs_gets_its_future_done<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair_holding(READS);
        let near = AsyncIo::new(&runtime, near).unwrap();
        let task = runtime.spawn("a task", async move {
            read_ready(&near, 10).await;
            futures::executor::block_on(unconstrained(read_ready(&near, READS - 10)));

            near
        });

        let near = runtime.block_on(task).unwrap();

        // Every byte was read: the next read finds none.
        let read = near.get_ref().peek(&mut [0]);
        assert_eq!(read.unwrap_err().kind(), io::ErrorKind::WouldBlock);
    }
}

in_both_modes! {
    /// The budget goes with the poll it was given for: a thread that has been through a task, the
    /// future of `block_on`, a task that panicked or a future of `block_on` that did runs a future
    /// of someone else's to its end in one poll again.
    ///
    /// Each of them spends a part of the budget first, so that one left behind would show as a
    /// future that yields before the end of its reads.
    fn the_budget_is_gone_once_the_poll_it_was_given_for_is_over<M, P>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair_holding(4 * READS);
        let near = P::new(AsyncIo::new(&runtime, near).unwrap());

        runtime.block_on(async {
            let reader = {
                let near = near.clone();
                runtime.spawn("a reader", async move { read_ready(&near, 10).await })
            };
            reader.await.unwrap();
            read_ready(&near, 10).await;
        });
        assert!(reads_in_one_poll(&near, READS));

        let panicking = {
            let near = near.clone();
            runtime.spawn("a task that panics", async move {
                read_ready(&near, 10).await;
                panic!("the task panicked on purpose");
            })
        };
        assert!(runtime.block_on(panicking).is_err());
        assert!(reads_in_one_poll(&near, READS));

        let panicked = catch_unwind(AssertUnwindSafe(|| {
            runtime.block_on(async {
                read_ready(&near, 10).await;
                panic!("the future panicked on purpose");
            })
        }));
        assert!(panicked.is_err());
        assert!(reads_in_one_poll(&near, READS));
    }
}

in_both_modes! {
    /// Only an operation that completes spends the budget: a call that finds the source not ready
    /// costs nothing however many times over it is made, while one that fails for another reason
    /// costs as much as one that succeeds.
    ///
    /// The calls are made through `poll_read_with` in a poll of the test's own, which has a budget
    /// the way a task has.
    fn only_an_operation_that_completes_spends_budget<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair();
        let near = AsyncIo::new(&runtime, near).unwrap();
        let mut cx = Context::from_waker(Waker::noop());

        coop::budgeted(|| {
            for _ in 0..2 * BUDGET {
                let polled = near
                    .poll_read_with(&mut cx, |_| Err::<(), _>(io::ErrorKind::WouldBlock.into()));
                assert!(polled.is_pending());
            }

            for _ in 0..BUDGET {
                let polled = near
                    .poll_read_with(&mut cx, |_| Err::<(), _>(io::ErrorKind::BrokenPipe.into()));
                assert!(matches!(polled, Poll::Ready(Err(_))));
            }
            // The budget is spent: this yields, and the operation does not run.
            let polled = near.poll_read_with(&mut cx, |_| -> io::Result<()> {
                unreachable!("an operation does not run once the budget is spent")
            });
            assert!(polled.is_pending());
        });
    }
}

in_both_modes! {
    /// `into_inner` hands the source back: the same descriptor it was made from, still in
    /// non-blocking mode, and as good as it was, though a read of the handle was left waiting
    /// for the runtime to wake it.
    fn into_inner_hands_the_source_back<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, mut far) = tcp_pair();
        let raw = raw_id(&near);
        let near = AsyncIo::new(&runtime, near).unwrap();
        // A read that has to wait, so that a wait of the runtime watches the source before the
        // source is asked for.
        runtime.block_on(async {
            let (read, ()) = join(read_exactly(&near, 1), async {
                runtime.sleep(DELAY).await;
                (&far).write_all(b"!").unwrap();
            })
            .await;
            assert_eq!(read, b"!");
        });
        // A read that found nothing to read, whose waker the runtime holds on to.
        let mut cx = Context::from_waker(Waker::noop());
        let waiting = near.poll_read_with(&mut cx, |mut stream| stream.read(&mut [0]));
        assert!(waiting.is_pending());

        let near = near.into_inner();

        assert_eq!(raw_id(&near), raw);
        let mut stream = &near;
        let error = stream.read(&mut [0]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        stream.write_all(b"back").unwrap();
        let mut received = [0; 4];
        far.read_exact(&mut received).unwrap();
        assert_eq!(&received, b"back");
    }
}

/// `into_inner` on a shared runtime returns while another thread is inside `block_on` on that
/// runtime, in a wait that watches the source: the end of the watch breaks that wait, and the
/// source comes back whole. Where the platform's poller copied the source's descriptor in as the
/// wait started, the runtime keeps the source until the wait returns, which this waits for.
///
/// A read of the handle that found nothing to read leaves its waker with the runtime, which has
/// every wait that thread makes watch the source. The thread is made to wait for a byte on a
/// source of its own, and is let go of once the source is back.
///
/// That the thread is inside its wait by the time the source is asked for is what the pause
/// before it is for, and the test does not depend on it: a source whose watch ends outside a
/// wait is kept by nothing, and comes back in either case.
#[test]
#[timeout(15000)]
fn into_inner_returns_while_another_thread_waits_on_the_source() {
    let runtime = SharedRuntime::new().unwrap();
    let (near, mut far) = tcp_pair();
    let (trigger_near, mut trigger_far) = tcp_pair();
    let watched = AsyncIo::new(&runtime, near).unwrap();
    let trigger = AsyncIo::new(&runtime, trigger_near).unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    let waiting = watched.poll_read_with(&mut cx, |mut stream| stream.read(&mut [0]));
    assert!(waiting.is_pending());
    let (inside, entered) = mpsc::channel();
    let driver = {
        let runtime = runtime.clone();

        thread::spawn(move || {
            runtime.block_on(async {
                inside.send(()).unwrap();
                // The wait this ends up in watches the source above as well as this one.
                let mut byte = [0];
                trigger
                    .read_with(|mut stream| stream.read(&mut byte))
                    .await
                    .unwrap();
            });
        })
    };
    entered.recv().unwrap();
    thread::sleep(DELAY);

    let near = watched.into_inner();

    let mut stream = &near;
    stream.write_all(b"back").unwrap();
    let mut received = [0; 4];
    far.read_exact(&mut received).unwrap();
    assert_eq!(&received, b"back");
    trigger_far.write_all(b"!").unwrap();
    driver.join().unwrap();
}

/// A source whose destructor takes other handles' sources back, dropped while the thread driving
/// a shared runtime is in a wait that watches them all, has its destructor return. Where the
/// platform's poller copied the descriptors in as the wait started, the runtime keeps the dropped
/// source until the wait returns, and its destructor runs on that thread then, clear of every
/// lock and with no other source kept there; elsewhere it runs on the thread that drops it.
///
/// The handles each leave a waker with the runtime, so that the wait the thread makes watches
/// them all, and many of them, so that a source kept for a wait would be likely to come before
/// one the destructor takes back in whatever order the runtime let go of them in.
///
/// That the thread is inside its wait when the handle is dropped is what the pause before it is
/// for, and the test does not depend on it: a destructor run anywhere else returns as well.
#[test]
#[timeout(15000)]
fn a_destructor_run_by_a_wait_takes_other_sources_back() {
    const INNER: usize = 32;

    let runtime = SharedRuntime::new().unwrap();
    let (returned, destructed) = mpsc::channel();
    let mut peers = Vec::new();
    let mut inner = Vec::new();
    for _ in 0..INNER {
        let (near, far) = tcp_pair();
        inner.push(AsyncIo::new(&runtime, near).unwrap());
        peers.push(far);
    }
    let (near, _far) = tcp_pair();
    let outer = AsyncIo::new(
        &runtime,
        TakesBack {
            stream: near,
            inner,
            returned,
        },
    )
    .unwrap();
    let mut cx = Context::from_waker(Waker::noop());
    for handle in &outer.get_ref().inner {
        let waiting = handle.poll_read_with(&mut cx, |mut stream| stream.read(&mut [0]));
        assert!(waiting.is_pending());
    }
    let waiting = outer.poll_read_with(&mut cx, |source| (&source.stream).read(&mut [0]));
    assert!(waiting.is_pending());
    let (trigger_near, mut trigger_far) = tcp_pair();
    let trigger = AsyncIo::new(&runtime, trigger_near).unwrap();
    let (inside, entered) = mpsc::channel();
    let driver = {
        let runtime = runtime.clone();

        thread::spawn(move || {
            runtime.block_on(async {
                inside.send(()).unwrap();
                let mut byte = [0];
                trigger
                    .read_with(|mut stream| stream.read(&mut byte))
                    .await
                    .unwrap();
            });
        })
    };
    entered.recv().unwrap();
    thread::sleep(DELAY);

    drop(outer);

    let taken_back = destructed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(taken_back, INNER);
    trigger_far.write_all(b"!").unwrap();
    driver.join().unwrap();
}

in_both_modes! {
    /// A runtime runs no code of a source's while it waits: it reads the descriptor once, as it
    /// takes the source under its watch, and watches that one from then on.
    ///
    /// The source here counts each time it is asked for its descriptor once armed, and takes
    /// another handle's source back then, as a source whose `as_fd` runs code of its own might.
    /// A read of each handle that found nothing to read has every wait watch both.
    fn a_wait_runs_no_code_of_a_source<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (inner, _inner_far) = tcp_pair();
        let (near, _far) = tcp_pair();
        let inner = AsyncIo::new(&runtime, inner).unwrap();
        let outer = AsyncIo::new(
            &runtime,
            AsksToTakeBack {
                stream: near,
                inner: Mutex::new(Some(inner)),
                armed: AtomicBool::new(false),
                asked: AtomicUsize::new(0),
            },
        )
        .unwrap();
        outer.get_ref().armed.store(true, Ordering::SeqCst);
        let mut cx = Context::from_waker(Waker::noop());
        let waiting = outer.poll_read_with(&mut cx, |source| (&source.stream).read(&mut [0]));
        assert!(waiting.is_pending());
        {
            let inner = outer.get_ref().inner.lock().unwrap();
            let inner = inner.as_ref().expect("the inner handle is still there");
            let waiting = inner.poll_read_with(&mut cx, |mut stream| stream.read(&mut [0]));
            assert!(waiting.is_pending());
        }

        // A few waits of the runtime, each of them watching both sources.
        runtime.block_on(async {
            for _ in 0..3 {
                runtime.sleep(Duration::from_millis(1)).await;
            }
        });

        assert_eq!(outer.get_ref().asked.load(Ordering::SeqCst), 0);
        assert!(outer.get_ref().inner.lock().unwrap().is_some());
    }
}

in_both_modes! {
    /// Linux and Android cannot watch a regular file, and a wait on one fails, where the caller
    /// hears of it, rather than waits for readiness that nothing would report.
    #[cfg(all(any(target_os = "linux", target_os = "android"), not(zruntime_poll)))]
    fn a_wait_on_a_regular_file_fails<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let file = std::fs::File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
        let file = AsyncIo::new(&runtime, file).unwrap();

        let refused = runtime.block_on(file.readable()).unwrap_err();

        assert_eq!(refused.kind(), io::ErrorKind::PermissionDenied);
    }
}

/// A local runtime watches a source that is neither `Send` nor `Sync`, and the handle does its
/// I/O on it: its type says that it stays on the thread, which the compile-fail doc tests show.
#[test]
#[timeout(15000)]
fn a_local_async_takes_a_source_that_is_neither_send_nor_sync() {
    let runtime = LocalRuntime::new().unwrap();
    let (near, mut far) = tcp_pair();
    let near = AsyncIo::new(&runtime, Unshareable::new(near)).unwrap();

    runtime.block_on(async {
        far.write_all(b"in").unwrap();
        let mut received = [0; 2];
        let read = near
            .read_with(|source| source.read(&mut received))
            .await
            .unwrap();
        assert_eq!(&received[..read], b"in");

        let written = near
            .write_with(|source| source.write(b"out"))
            .await
            .unwrap();
        assert_eq!(written, 3);
    });

    let mut received = [0; 3];
    far.read_exact(&mut received).unwrap();
    assert_eq!(&received, b"out");
}

in_both_modes! {
    /// The two ends of a pipe are each a handle, and a read of the one goes on to the end of what
    /// the other wrote, which is where the writing end is dropped.
    #[cfg(unix)]
    fn the_ends_of_a_pipe_carry_bytes_to_the_end<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (reader, writer) = std::io::pipe().unwrap();
        let mut reader = AsyncIo::new(&runtime, reader).unwrap();
        let mut writer = AsyncIo::new(&runtime, writer).unwrap();
        let mut received = Vec::new();

        let (read, written) = runtime.block_on(join(reader.read_to_end(&mut received), async {
            runtime.sleep(DELAY).await;
            let written = writer.write_all(b"hello").await;
            // The pipe closes with its writing end, which is what ends the read.
            drop(writer);

            written
        }));

        assert_eq!(read.unwrap(), 5);
        written.unwrap();
        assert_eq!(received, b"hello");
    }
}

in_both_modes! {
    /// A wait for room in a pipe whose reader closed completes, and so does the next one, made at
    /// once after it. FreeBSD lets go of the pipe's write filter as it reports the first, and then
    /// refuses to watch the pipe (`EPIPE`), which is what the second ends with there; anywhere
    /// else the pipe is reported ready again.
    #[cfg(unix)]
    fn a_pipe_whose_reader_closed_is_waited_on_twice<M>() {
        use std::pin::pin;

        use futures::future::poll_immediate;

        let runtime = Runtime::<M>::new().unwrap();
        let (reader, writer) = std::io::pipe().unwrap();
        let writer = AsyncIo::new(&runtime, writer).unwrap();

        runtime.block_on(async {
            let mut writable = pin!(writer.writable());
            // The first poll has the runtime watch the pipe for room while the reader is open.
            assert!(poll_immediate(writable.as_mut()).await.is_none());
            drop(reader);
            writable.await.unwrap();

            if let Err(refused) = writer.writable().await {
                assert_eq!(refused.kind(), io::ErrorKind::BrokenPipe);
            }
        });
    }
}

in_both_modes! {
    /// A unix-domain socket pair is a pair of handles, each end reading and writing through the
    /// traits.
    #[cfg(unix)]
    fn a_unix_stream_pair_carries_bytes_both_ways<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, far) = std::os::unix::net::UnixStream::pair().unwrap();
        let mut near = AsyncIo::new(&runtime, near).unwrap();
        let mut far = AsyncIo::new(&runtime, far).unwrap();

        runtime.block_on(async {
            let mut received = [0; 4];
            let (read, written) = join(far.read_exact(&mut received), async {
                runtime.sleep(DELAY).await;
                near.write_all(b"ping").await
            })
            .await;
            read.unwrap();
            written.unwrap();
            assert_eq!(&received, b"ping");

            far.write_all(b"pong").await.unwrap();
            near.read_exact(&mut received).await.unwrap();
            assert_eq!(&received, b"pong");
        });
    }
}

in_both_modes! {
    /// The output of a child process is read to its end: `ChildStdout` reads only through a
    /// `&mut` of itself, so it goes through `OwnedFd` into a `PipeReader` first.
    #[cfg(unix)]
    fn a_child_process_output_is_read_to_the_end<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = Command::new("sh")
            .args(["-c", "printf hello"])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut stdout = AsyncIo::new(&runtime, PipeReader::from(OwnedFd::from(stdout))).unwrap();

        let mut output = Vec::new();
        runtime.block_on(stdout.read_to_end(&mut output)).unwrap();

        assert_eq!(output, b"hello");
        assert!(child.wait().unwrap().success());
    }
}

in_both_modes! {
    /// A child process is fed through its standard input and read through its standard output, as
    /// two handles: the pipes go through `OwnedFd` into a `PipeWriter` and a `PipeReader`.
    #[cfg(unix)]
    fn a_child_process_is_fed_and_read_through_its_pipes<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let mut child = Command::new("cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut stdin = AsyncIo::new(&runtime, PipeWriter::from(OwnedFd::from(stdin))).unwrap();
        let mut stdout = AsyncIo::new(&runtime, PipeReader::from(OwnedFd::from(stdout))).unwrap();
        let mut output = Vec::new();

        let (read, written) = runtime.block_on(join(stdout.read_to_end(&mut output), async {
            let written = stdin.write_all(b"hello").await;
            // The child ends with its input, which is what ends the read.
            drop(stdin);

            written
        }));

        assert_eq!(read.unwrap(), 5);
        written.unwrap();
        assert_eq!(output, b"hello");
        assert!(child.wait().unwrap().success());
    }
}

in_both_modes! {
    /// A handle prints as the source it wraps does, inside an `AsyncIo { io: .. }` of its own, and
    /// hands that source out by reference.
    fn an_async_prints_as_its_source_and_hands_it_out<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair();
        let printed = format!("{near:?}");
        let address = near.local_addr().unwrap();

        let near = AsyncIo::new(&runtime, near).unwrap();

        assert_eq!(
            format!("{near:?}"),
            format!("AsyncIo {{ io: {printed}, .. }}")
        );
        assert_eq!(near.get_ref().local_addr().unwrap(), address);
    }
}

in_both_modes! {
    /// A handle's descriptor is its source's: the one `AsRawFd` or `AsRawSocket` names, and the
    /// one a borrow of it reaches.
    fn an_async_names_the_descriptor_of_its_source<M>() {
        let runtime = Runtime::<M>::new().unwrap();
        let (near, _far) = tcp_pair();
        let address = near.local_addr().unwrap();
        let raw = raw_id(&near);

        let near = AsyncIo::new(&runtime, near).unwrap();

        #[cfg(unix)]
        {
            assert_eq!(near.as_raw_fd(), raw);
            assert_eq!(near.as_fd().as_raw_fd(), raw);
        }
        #[cfg(windows)]
        {
            assert_eq!(near.as_raw_socket(), raw);
            assert_eq!(near.as_socket().as_raw_socket(), raw);
        }
        // `SockRef` asks the system about the descriptor it is given, which is how the borrow is
        // seen to be of the socket itself: it finds the address that socket has.
        let found = SockRef::from(&near).local_addr().unwrap();
        assert_eq!(found.as_socket(), Some(address));
    }
}

/// A shared runtime's handles are `Send` and `Sync`, and `Unpin`, which the extension traits of
/// futures ask of what they read from and write to.
///
/// Only the bounds are checked here, at compile time. That a local runtime's handles are neither
/// `Send` nor `Sync` is shown by the compile-fail doc tests, since only code that does not
/// compile can show it.
#[test]
#[timeout(15000)]
fn shared_asyncs_are_send_sync_and_unpin() {
    fn sent_and_shared<T>()
    where
        T: Send + Sync + Unpin,
    {
    }

    sent_and_shared::<AsyncIo<TcpStream, Shared>>();
    sent_and_shared::<Readiness<'static, Shared>>();
}

/// A local runtime's handles are `Unpin` too.
#[test]
#[timeout(15000)]
fn local_asyncs_are_unpin() {
    fn unpin<T>()
    where
        T: Unpin,
    {
    }

    unpin::<AsyncIo<TcpStream, Local>>();
    unpin::<Readiness<'static, Local>>();
}

/// A source that, once armed, takes the source of the handle it holds back each time it is asked
/// for its own descriptor, and counts how often it was asked.
struct AsksToTakeBack<M>
where
    M: Mode,
{
    stream: TcpStream,
    inner: Mutex<Option<AsyncIo<TcpStream, M>>>,
    armed: AtomicBool,
    asked: AtomicUsize,
}

impl<M> AsksToTakeBack<M>
where
    M: Mode,
{
    /// What asking for the descriptor does once the source is armed.
    fn asked(&self) {
        if !self.armed.load(Ordering::SeqCst) {
            return;
        }
        self.asked.fetch_add(1, Ordering::SeqCst);
        let inner = self.inner.lock().unwrap().take();
        if let Some(inner) = inner {
            drop(inner.into_inner());
        }
    }
}

#[cfg(unix)]
impl<M> AsFd for AsksToTakeBack<M>
where
    M: Mode,
{
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.asked();

        self.stream.as_fd()
    }
}

#[cfg(windows)]
impl<M> AsSocket for AsksToTakeBack<M>
where
    M: Mode,
{
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.asked();

        self.stream.as_socket()
    }
}

/// A source that holds handles of other sources, and takes each of their sources back as it
/// goes, saying how many it took back.
struct TakesBack {
    stream: TcpStream,
    inner: Vec<AsyncIo<TcpStream, Shared>>,
    returned: mpsc::Sender<usize>,
}

impl Drop for TakesBack {
    fn drop(&mut self) {
        let taken_back = self.inner.drain(..).map(AsyncIo::into_inner).count();
        // The test may have stopped listening, having failed already.
        let _ = self.returned.send(taken_back);
    }
}

#[cfg(unix)]
impl AsFd for TakesBack {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.stream.as_fd()
    }
}

#[cfg(windows)]
impl AsSocket for TakesBack {
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.stream.as_socket()
    }
}

/// How many operations the tests of the budget make: a good many times what one poll may complete,
/// and few enough for any socket to hold the bytes without making the writer wait.
pub(super) const READS: usize = 1024;

/// How many operations a poll may complete, as a count.
pub(super) const BUDGET: usize = coop::BUDGET as usize;

const _: () = assert!(READS > 2 * BUDGET);

/// Notes in `seen` how far `count` has got by the time this runs, which is the first time its task
/// is polled.
pub(super) async fn note_progress(count: Arc<AtomicUsize>, seen: Arc<AtomicUsize>) {
    seen.store(count.load(Ordering::SeqCst), Ordering::SeqCst);
}

/// A connected pair of TCP streams over loopback, as [`tcp_pair`] makes them, whose first end
/// already holds `len` bytes that the second end wrote.
///
/// This returns only once the first end has all of them. A write on loopback does not promise that
/// its peer can read the bytes at once on every platform, and a read that finds none yet would
/// spoil the run of reads that never wait, which the tests of the budget are about.
pub(super) fn tcp_pair_holding(len: usize) -> (TcpStream, TcpStream) {
    let (near, mut far) = tcp_pair();
    far.write_all(&pattern(len)).unwrap();
    // Both are blocking, so a peek waits for at least a byte and returns what has come so far.
    let mut peeked = vec![0; len];
    while near.peek(&mut peeked).unwrap() < len {
        thread::yield_now();
    }

    (near, far)
}

/// How long a test lets a task wait before the other one does what the first waits for: long
/// enough for the waiting to have begun, and short enough to cost the suite next to nothing.
///
/// The tests that use it are right whichever of the two happens first. What the delay changes is
/// whether the call that waits has to wait at all, which is what they are there to see.
const DELAY: Duration = Duration::from_millis(50);

/// A source that is neither `Send` nor `Sync`: an owned descriptor of a socket beside an `Rc`,
/// which no other thread may touch.
struct Unshareable {
    socket: OwnedSource,
    _local: Rc<()>,
}

#[cfg(unix)]
type OwnedSource = OwnedFd;
#[cfg(windows)]
type OwnedSource = OwnedSocket;

impl Unshareable {
    fn new(stream: TcpStream) -> Self {
        Self {
            socket: stream.into(),
            _local: Rc::new(()),
        }
    }

    /// Reads from the socket, which the source has no `Read` of its own for.
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        (&*SockRef::from(&self.socket)).read(buf)
    }

    /// Writes to the socket, likewise.
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        (&*SockRef::from(&self.socket)).write(buf)
    }
}

#[cfg(unix)]
impl AsFd for Unshareable {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.socket.as_fd()
    }
}

#[cfg(windows)]
impl AsSocket for Unshareable {
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.socket.as_socket()
    }
}

/// Reads `len` bytes from `io` through `read_with`, however many reads that takes.
async fn read_exactly<M>(io: &AsyncIo<TcpStream, M>, len: usize) -> Vec<u8>
where
    M: Mode,
{
    let mut bytes = vec![0; len];
    let mut filled = 0;
    while filled < len {
        let read = io
            .read_with(|mut stream| stream.read(&mut bytes[filled..]))
            .await
            .unwrap();
        assert_ne!(read, 0, "the stream ended early");
        filled += read;
    }

    bytes
}

/// Writes all of `bytes` to `io` through `write_with`, however many writes that takes.
async fn write_everything<M>(io: &AsyncIo<TcpStream, M>, bytes: &[u8])
where
    M: Mode,
{
    let mut sent = 0;
    while sent < bytes.len() {
        sent += io
            .write_with(|mut stream| stream.write(&bytes[sent..]))
            .await
            .unwrap();
    }
}

/// Whether `reads` reads from `io`, which holds the bytes for them, complete in one poll made by
/// the caller, which no runtime is part of.
fn reads_in_one_poll<M>(io: &AsyncIo<TcpStream, M>, reads: usize) -> bool
where
    M: Mode,
{
    let mut reading = pin!(read_ready(io, reads));

    reading
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_ready()
}

/// Reads `reads` bytes from `io` one at a time through `read_with`.
///
/// A source that holds that many bytes never makes a read wait.
async fn read_ready<M>(io: &AsyncIo<TcpStream, M>, reads: usize)
where
    M: Mode,
{
    for _ in 0..reads {
        io.read_with(|mut stream| stream.read(&mut [0]))
            .await
            .unwrap();
    }
}

/// Writes to `io` until its socket has no room for more, and says how many bytes that took.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn fill(io: &AsyncIo<TcpStream, impl Mode>) -> usize {
    let mut stream = io.get_ref();
    let mut filled = 0;
    loop {
        match stream.write(&[0; 4096]) {
            Ok(written) => filled += written,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return filled,
            Err(e) => panic!("a write to a socket with room failed: {e}"),
        }
    }
}

/// Whether `result` is the failure of a call that found the socket not ready.
fn would_block<T>(result: &io::Result<T>) -> bool {
    matches!(result, Err(e) if e.kind() == io::ErrorKind::WouldBlock)
}

/// `len` bytes that tell a change of order, or a loss, from the original.
///
/// They cycle through a prime number of values, so that the pattern does not line up with the size
/// of any buffer on the way.
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

/// A connected pair of TCP streams over loopback, with Nagle's algorithm off at both ends so that
/// a short write goes out at once rather than wait for the acknowledgement of the one before it.
///
/// Both are in blocking mode, as std makes them.
fn tcp_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let far = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let near = loop {
        let (accepted, _) = listener.accept().unwrap();
        // A loopback listener is reachable by anything else on the machine, so a connection that
        // is not the one made just above is turned away rather than taken for it.
        if accepted.peer_addr().unwrap() == far.local_addr().unwrap() {
            break accepted;
        }
    };
    near.set_nodelay(true).unwrap();
    far.set_nodelay(true).unwrap();

    (near, far)
}

/// Two UDP sockets over loopback, each connected to the other, so that each takes the datagrams of
/// the other and no others.
fn udp_pair() -> (UdpSocket, UdpSocket) {
    let near = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let far = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    near.connect(far.local_addr().unwrap()).unwrap();
    far.connect(near.local_addr().unwrap()).unwrap();

    (near, far)
}

/// What names a socket to the system: a descriptor on unix, a socket on Windows.
#[cfg(unix)]
type RawId = std::os::fd::RawFd;
#[cfg(windows)]
type RawId = std::os::windows::io::RawSocket;

/// The raw descriptor of `socket`.
#[cfg(unix)]
fn raw_id(socket: &TcpStream) -> RawId {
    socket.as_raw_fd()
}

/// The raw descriptor of `socket`.
#[cfg(windows)]
fn raw_id(socket: &TcpStream) -> RawId {
    socket.as_raw_socket()
}
