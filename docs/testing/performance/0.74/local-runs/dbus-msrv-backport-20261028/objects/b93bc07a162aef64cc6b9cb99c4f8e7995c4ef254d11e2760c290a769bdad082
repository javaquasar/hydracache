//! Tests of the broadcast channel, `zruntime::broadcast`.
//!
//! Most of them come from async-broadcast's own tests, with the threads they were driven by taken
//! from [`thread::scope`] and the blocking calls they made replaced by [`block_on`]. The last few
//! are new and pin down what the move changed: that the futures the channel hands out and its
//! receivers are `Unpin`, and that the wakeups the channel relies on `Event` for reach the tasks
//! they are meant for, which they check with wakers that only set a flag.

use std::{
    future::{self, Future},
    marker::PhantomPinned,
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    task::{Context, Poll, Wake, Waker},
    thread::{self, sleep},
    time::Duration,
};

use futures::{
    executor::block_on,
    future::join,
    stream::{FusedStream, StreamExt},
};
use ntest::timeout;

use crate::broadcast::{
    InactiveReceiver, Receiver, Recv, RecvError, Send, SendError, Sender, TryRecvError,
    TrySendError, channel,
};

fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

fn cap(cap: usize) -> NonZeroUsize {
    NonZeroUsize::new(cap).unwrap()
}

#[test]
fn basic_sync() {
    let (s, mut r1) = channel(cap(10));
    let mut r2 = r1.clone();

    s.try_broadcast(7).unwrap();
    assert_eq!(r1.try_recv().unwrap(), 7);
    assert_eq!(r2.try_recv().unwrap(), 7);

    let mut r3 = r1.clone();
    s.try_broadcast(8).unwrap();
    assert_eq!(r1.try_recv().unwrap(), 8);
    assert_eq!(r2.try_recv().unwrap(), 8);
    assert_eq!(r3.try_recv().unwrap(), 8);
}

#[test]
#[timeout(10000)]
fn basic_async() {
    block_on(async {
        let (s, mut r1) = channel(cap(10));
        let mut r2 = r1.clone();

        s.broadcast(7).await.unwrap();
        assert_eq!(r1.recv().await.unwrap(), 7);
        assert_eq!(r2.recv().await.unwrap(), 7);

        // Now let's try the Stream impl.
        let mut r3 = r1.clone();
        s.broadcast(8).await.unwrap();
        assert_eq!(r1.next().await.unwrap(), 8);
        assert_eq!(r2.next().await.unwrap(), 8);
        assert_eq!(r3.next().await.unwrap(), 8);
    });
}

#[test]
#[timeout(10000)]
fn basic_block_on() {
    let (s, mut r) = channel(cap(1));

    block_on(s.broadcast(7)).unwrap();
    assert_eq!(r.try_recv(), Ok(7));

    block_on(s.broadcast(8)).unwrap();
    assert_eq!(block_on(r.recv()), Ok(8));

    block_on(s.broadcast(9)).unwrap();
    assert_eq!(block_on(r.recv()), Ok(9));

    assert_eq!(r.try_recv(), Err(TryRecvError::Empty));
}

#[test]
#[timeout(10000)]
fn parallel() {
    let (s1, mut r1) = channel(cap(2));
    let s2 = s1.clone();
    let mut r2 = r1.clone();

    let (sender_sync_send, sender_sync_recv) = mpsc::channel();
    let (receiver_sync_send, receiver_sync_recv) = mpsc::channel();

    thread::scope(|scope| {
        scope.spawn(move || {
            sender_sync_recv.recv().unwrap();

            s1.try_broadcast(7).unwrap();
            s2.try_broadcast(8).unwrap();
            assert!(s2.try_broadcast(9).unwrap_err().is_full());
            assert!(s1.try_broadcast(10).unwrap_err().is_full());
            receiver_sync_send.send(()).unwrap();

            drop(s1);
            drop(s2);
            receiver_sync_send.send(()).unwrap();
        });
        scope.spawn(move || {
            assert_eq!(r1.try_recv(), Err(TryRecvError::Empty));
            assert_eq!(r2.try_recv(), Err(TryRecvError::Empty));
            sender_sync_send.send(()).unwrap();

            receiver_sync_recv.recv().unwrap();
            assert_eq!(r1.try_recv().unwrap(), 7);
            assert_eq!(r1.try_recv().unwrap(), 8);
            assert_eq!(r2.try_recv().unwrap(), 7);
            assert_eq!(r2.try_recv().unwrap(), 8);

            receiver_sync_recv.recv().unwrap();
            assert_eq!(r1.try_recv(), Err(TryRecvError::Closed));
            assert_eq!(r2.try_recv(), Err(TryRecvError::Closed));
        });
    });
}

#[test]
#[timeout(10000)]
fn parallel_async() {
    let (s1, mut r1) = channel(cap(2));
    let s2 = s1.clone();
    let mut r2 = r1.clone();

    let (sender_sync_send, sender_sync_recv) = mpsc::channel();
    let (receiver_sync_send, receiver_sync_recv) = mpsc::channel();

    thread::scope(|scope| {
        scope.spawn(move || {
            block_on(async move {
                sender_sync_recv.recv().unwrap();
                sleep(ms(5));

                s1.broadcast(7).await.unwrap();
                s2.broadcast(8).await.unwrap();
                assert!(s2.try_broadcast(9).unwrap_err().is_full());
                assert!(s1.try_broadcast(10).unwrap_err().is_full());
                receiver_sync_send.send(()).unwrap();

                s1.broadcast(9).await.unwrap();
                s2.broadcast(10).await.unwrap();

                drop(s1);
                drop(s2);
                receiver_sync_send.send(()).unwrap();
            })
        });
        scope.spawn(move || {
            block_on(async move {
                assert_eq!(r1.try_recv(), Err(TryRecvError::Empty));
                assert_eq!(r2.try_recv(), Err(TryRecvError::Empty));
                sender_sync_send.send(()).unwrap();

                receiver_sync_recv.recv().unwrap();
                assert_eq!(r1.next().await.unwrap(), 7);
                assert_eq!(r2.next().await.unwrap(), 7);
                assert_eq!(r1.recv().await.unwrap(), 8);
                assert_eq!(r2.recv().await.unwrap(), 8);

                receiver_sync_recv.recv().unwrap();
                sleep(ms(5));
                assert_eq!(r1.next().await.unwrap(), 9);
                assert_eq!(r2.next().await.unwrap(), 9);

                assert_eq!(r1.recv().await.unwrap(), 10);
                assert_eq!(r2.recv().await.unwrap(), 10);

                assert_eq!(r1.recv().await, Err(RecvError::Closed));
                assert_eq!(r2.recv().await, Err(RecvError::Closed));
            })
        });
    });
}

#[test]
fn channel_shrink() {
    let (s1, mut r1) = channel(cap(4));
    let mut r2 = r1.clone();
    let mut r3 = r1.clone();
    let mut r4 = r1.clone();

    s1.try_broadcast(1).unwrap();
    s1.try_broadcast(2).unwrap();
    s1.try_broadcast(3).unwrap();
    s1.try_broadcast(4).unwrap();

    assert_eq!(r2.try_recv().unwrap(), 1);
    assert_eq!(r2.try_recv().unwrap(), 2);

    assert_eq!(r3.try_recv().unwrap(), 1);
    assert_eq!(r3.try_recv().unwrap(), 2);
    assert_eq!(r3.try_recv().unwrap(), 3);

    assert_eq!(r4.try_recv().unwrap(), 1);
    assert_eq!(r4.try_recv().unwrap(), 2);
    assert_eq!(r4.try_recv().unwrap(), 3);
    assert_eq!(r4.try_recv().unwrap(), 4);

    r1.set_capacity(cap(2));

    assert_eq!(r1.try_recv(), Err(TryRecvError::Overflowed(2)));
    assert_eq!(r1.try_recv().unwrap(), 3);
    assert_eq!(r1.try_recv().unwrap(), 4);
    assert_eq!(r1.try_recv(), Err(TryRecvError::Empty));

    assert_eq!(r2.try_recv().unwrap(), 3);
    assert_eq!(r2.try_recv().unwrap(), 4);
    assert_eq!(r2.try_recv(), Err(TryRecvError::Empty));

    assert_eq!(r3.try_recv().unwrap(), 4);
    assert_eq!(r3.try_recv(), Err(TryRecvError::Empty));

    assert_eq!(r4.try_recv(), Err(TryRecvError::Empty));
}

#[test]
#[timeout(10000)]
fn overflow() {
    let (s1, mut r1) = channel(cap(2));
    r1.set_overflow(true);
    // We'll keep r1 as the lagging receiver.
    let mut r2 = r1.clone();
    let mut r3 = r1.clone();

    // Each of the two receivers says so once it has taken both messages, and the sender waits for
    // both before it sends the message that pushes the oldest out of the channel.
    let (taken_send, taken_recv) = mpsc::channel();
    let taken_send2 = taken_send.clone();

    thread::scope(|scope| {
        scope.spawn(move || {
            block_on(async move {
                s1.broadcast(7).await.unwrap();
                s1.broadcast(8).await.unwrap();
                taken_recv.recv().unwrap();
                taken_recv.recv().unwrap();

                s1.broadcast(9).await.unwrap();
            })
        });
        scope.spawn(move || {
            block_on(async move {
                assert_eq!(r2.next().await.unwrap(), 7);
                assert_eq!(r2.recv().await.unwrap(), 8);

                taken_send.send(()).unwrap();
                assert_eq!(r2.next().await.unwrap(), 9);
            })
        });
        scope.spawn(move || {
            block_on(async move {
                assert_eq!(r3.next().await.unwrap(), 7);
                assert_eq!(r3.recv().await.unwrap(), 8);

                taken_send2.send(()).unwrap();
                assert_eq!(r3.next().await.unwrap(), 9);
            })
        });
    });

    assert_eq!(r1.try_recv(), Err(TryRecvError::Overflowed(1)));
    assert_eq!(r1.try_recv().unwrap(), 8);
    assert_eq!(r1.try_recv().unwrap(), 9);
}

#[test]
#[timeout(10000)]
fn open_channel() {
    let (s1, r) = channel(cap(2));
    let inactive = r.deactivate();
    let s2 = s1.clone();

    let (receiver_sync_send, receiver_sync_recv) = mpsc::channel();
    let (sender_sync_send, sender_sync_recv) = mpsc::channel();
    let (activated_send, activated_recv) = mpsc::channel();

    thread::scope(|scope| {
        scope.spawn(move || {
            block_on(async move {
                // Both sends are left waiting for an active receiver, and polled again only once
                // the other thread has activated one. A send tries to send before it looks at its
                // listener, so a receiver activated in between `join`'s polls of the two would
                // have the second send its message straight away, ahead of the first's.
                let mut send1 = s1.broadcast(7);
                let mut send2 = s2.broadcast(8);
                future::poll_fn(|cx| {
                    assert!(Pin::new(&mut send1).poll(cx).is_pending());
                    assert!(Pin::new(&mut send2).poll(cx).is_pending());

                    Poll::Ready(())
                })
                .await;
                receiver_sync_send.send(()).unwrap();
                activated_recv.recv().unwrap();

                let (result1, result2) = join(send1, send2).await;
                result1.unwrap();
                result2.unwrap();

                sender_sync_recv.recv().unwrap();
                assert_eq!(s1.try_broadcast(9), Err(TrySendError::Inactive(9)));
                assert_eq!(s2.try_broadcast(10), Err(TrySendError::Inactive(10)));
                receiver_sync_send.send(()).unwrap();
                sleep(ms(5));

                s1.broadcast(9).await.unwrap();
                s2.broadcast(10).await.unwrap();
            })
        });
        scope.spawn(move || {
            block_on(async move {
                receiver_sync_recv.recv().unwrap();

                let mut r = inactive.activate_cloned();
                activated_send.send(()).unwrap();
                assert_eq!(r.next().await.unwrap(), 7);
                assert_eq!(r.recv().await.unwrap(), 8);
                drop(r);

                sender_sync_send.send(()).unwrap();
                receiver_sync_recv.recv().unwrap();

                let mut r = inactive.activate();
                assert_eq!(r.recv().await.unwrap(), 9);
                assert_eq!(r.recv().await.unwrap(), 10);
            })
        });
    });
}

#[test]
fn inactive_drop() {
    let (s, active_receiver) = channel::<()>(cap(1));
    let inactive = active_receiver.deactivate();
    let inactive2 = inactive.clone();
    drop(inactive);
    drop(inactive2);

    assert!(s.is_closed())
}

#[test]
#[timeout(10000)]
fn poll_recv() {
    let (s, mut r) = channel::<i32>(cap(2));
    r.set_overflow(true);

    // A quick custom stream impl to demonstrate/test `poll_recv`.
    struct MyStream(Receiver<i32>);
    impl futures_core::Stream for MyStream {
        type Item = Result<i32, RecvError>;
        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            self.0.poll_recv(cx)
        }
    }

    block_on(async move {
        let mut stream = MyStream(r);

        s.broadcast(1).await.unwrap();
        s.broadcast(2).await.unwrap();
        s.broadcast(3).await.unwrap();
        s.broadcast(4).await.unwrap();

        assert_eq!(stream.next().await.unwrap(), Err(RecvError::Overflowed(2)));
        assert_eq!(stream.next().await.unwrap(), Ok(3));
        assert_eq!(stream.next().await.unwrap(), Ok(4));

        drop(s);

        assert_eq!(stream.next().await, None);
    })
}

#[test]
fn futures_and_receiver_are_unpin() {
    fn assert_unpin<T: Unpin>() {}

    assert_unpin::<Send<'static, u32>>();
    assert_unpin::<Recv<'static, u32>>();
    assert_unpin::<Receiver<u32>>();
    assert_unpin::<Sender<u32>>();
    assert_unpin::<InactiveReceiver<u32>>();
    // The message is never pinned, so a message that is not `Unpin` does not make the future so.
    assert_unpin::<Send<'static, PhantomPinned>>();
    assert_unpin::<Recv<'static, PhantomPinned>>();
    assert_unpin::<Receiver<PhantomPinned>>();
}

/// The futures are `Unpin`, so they can be polled by hand without pinning them first.
#[test]
fn futures_are_polled_without_pinning() {
    let (s, mut r) = channel(cap(1));
    let mut cx = Context::from_waker(std::task::Waker::noop());

    let mut recv = r.recv();
    assert_eq!(Pin::new(&mut recv).poll(&mut cx), Poll::Pending);
    drop(recv);

    let mut send = s.broadcast(1);
    assert_eq!(Pin::new(&mut send).poll(&mut cx), Poll::Ready(Ok(None)));

    // The channel is full now, so the next send waits until a receiver makes room.
    let mut send = s.broadcast(2);
    assert_eq!(Pin::new(&mut send).poll(&mut cx), Poll::Pending);
    assert_eq!(r.try_recv(), Ok(1));
    assert_eq!(Pin::new(&mut send).poll(&mut cx), Poll::Ready(Ok(None)));
    assert_eq!(r.try_recv(), Ok(2));
}

#[test]
fn set_capacity_shrinks_and_grows() {
    let (mut s, mut r) = channel::<i32>(cap(3));
    assert_eq!(s.capacity(), cap(3));
    assert_eq!(r.capacity(), cap(3));
    s.try_broadcast(1).unwrap();
    s.try_broadcast(2).unwrap();
    s.try_broadcast(3).unwrap();

    // Shrinking drops the oldest messages.
    s.set_capacity(cap(1));
    assert_eq!(s.capacity(), cap(1));
    assert_eq!(r.capacity(), cap(1));
    assert_eq!(r.try_recv(), Err(TryRecvError::Overflowed(2)));
    assert_eq!(r.try_recv(), Ok(3));
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty));
    s.try_broadcast(4).unwrap();
    assert_eq!(s.try_broadcast(5), Err(TrySendError::Full(5)));

    // Growing makes room.
    r.set_capacity(cap(2));
    assert_eq!(s.capacity(), cap(2));
    s.try_broadcast(5).unwrap();
    assert_eq!(s.try_broadcast(6), Err(TrySendError::Full(6)));

    let mut inactive = r.deactivate();
    assert_eq!(inactive.capacity(), cap(2));
    inactive.set_capacity(cap(4));
    assert_eq!(s.capacity(), cap(4));
}

/// A waker that sets a flag, to see whether a task has been woken.
struct Flag(AtomicBool);

impl Wake for Flag {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl Flag {
    /// A flag and the waker that sets it.
    fn new() -> (Arc<Self>, Waker) {
        let flag = Arc::new(Self(AtomicBool::new(false)));
        let waker = Waker::from(flag.clone());

        (flag, waker)
    }

    /// Whether the waker has been woken since the last call, clearing the flag.
    fn woken(&self) -> bool {
        self.0.swap(false, Ordering::SeqCst)
    }
}

/// Polls a future once with `waker`.
fn poll<F: Future + Unpin>(future: &mut F, waker: &Waker) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(waker))
}

#[test]
fn a_waiting_sender_is_woken_by_a_receiver_making_room() {
    let (s, mut r) = channel(cap(1));
    s.try_broadcast(1).unwrap();

    let (flag, waker) = Flag::new();
    let mut send = s.broadcast(2);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    assert!(!flag.woken());

    // Receiving the message in the channel makes room, which wakes the sender.
    assert_eq!(r.try_recv(), Ok(1));
    assert!(flag.woken());
    assert_eq!(poll(&mut send, &waker), Poll::Ready(Ok(None)));
    assert_eq!(r.try_recv(), Ok(2));
}

/// Two senders wait on a full channel. `Event::notify(1)` counts a listener that is notified but
/// has not run yet, so two receives wake only the first sender. The first one sends on its retry,
/// before it polls its listener again, and lets go of the listener, still notified, which passes
/// the notification on to the second.
#[test]
fn a_sender_that_sends_passes_its_notification_on() {
    let (s, mut r) = channel(cap(2));
    s.try_broadcast(1).unwrap();
    s.try_broadcast(2).unwrap();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut send_a = s.broadcast(3);
    let mut send_b = s.broadcast(4);
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);

    assert_eq!(r.try_recv(), Ok(1));
    assert_eq!(r.try_recv(), Ok(2));
    assert!(flag_a.woken());
    assert!(!flag_b.woken());

    // The channel is empty, so there is room for both: the first sender takes some and passes the
    // notification it did not need on.
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Ready(Ok(None)));
    assert!(flag_b.woken());
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Ok(None)));

    assert_eq!(r.try_recv(), Ok(3));
    assert_eq!(r.try_recv(), Ok(4));
}

/// A sender that is woken and dropped before it runs passes the wake-up on, so that the sender
/// after it does not wait for room that is there.
#[test]
fn a_dropped_sender_passes_its_notification_on() {
    let (s, mut r) = channel(cap(2));
    s.try_broadcast(1).unwrap();
    s.try_broadcast(2).unwrap();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut send_a = s.broadcast(3);
    let mut send_b = s.broadcast(4);
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);

    assert_eq!(r.try_recv(), Ok(1));
    assert!(flag_a.woken());
    assert!(!flag_b.woken());

    drop(send_a);
    assert!(flag_b.woken());
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Ok(None)));
    assert_eq!(r.try_recv(), Ok(2));
    assert_eq!(r.try_recv(), Ok(4));
}

#[test]
fn closing_wakes_waiting_senders_and_receivers() {
    let (s, mut r) = channel(cap(1));
    s.try_broadcast(1).unwrap();
    // A receiver that starts with no message waits on a channel that is full.
    let mut r2 = s.new_receiver();

    let (send_flag, send_waker) = Flag::new();
    let (recv_flag, recv_waker) = Flag::new();
    let mut send = s.broadcast(2);
    let mut recv = r2.recv();
    assert_eq!(poll(&mut send, &send_waker), Poll::Pending);
    assert_eq!(poll(&mut recv, &recv_waker), Poll::Pending);
    assert!(!send_flag.woken());
    assert!(!recv_flag.woken());

    assert!(s.close());
    assert!(send_flag.woken());
    assert!(recv_flag.woken());
    assert_eq!(poll(&mut send, &send_waker), Poll::Ready(Err(SendError(2))));
    assert_eq!(
        poll(&mut recv, &recv_waker),
        Poll::Ready(Err(RecvError::Closed))
    );
    assert_eq!(r.try_recv(), Ok(1));
}

#[test]
#[timeout(10000)]
fn inactive_receivers_keep_the_channel_open_when_senders_do_not_await_active() {
    let (mut s, r) = channel::<i32>(cap(1));
    let _inactive = r.deactivate();
    s.set_await_active(false);

    assert_eq!(block_on(s.broadcast(1)), Err(SendError(1)));
    assert!(!s.is_closed());
    assert_eq!(s.try_broadcast(2), Err(TrySendError::Inactive(2)));
}

/// A waker whose `wake` panics, for a wake-up that goes wrong.
struct PanicOnWake;

impl Wake for PanicOnWake {
    fn wake(self: Arc<Self>) {
        panic!("a waker that panics");
    }
}

/// The channel's state is complete before it notifies anyone, so a waker that panics in the
/// notification leaves it as consistent as one that does not. Here the notification is the one for
/// the first active receiver: the receiver it made has to be dropped, and counted out, as the panic
/// unwinds.
#[test]
fn a_panicking_waker_in_activate_cloned_leaves_the_receiver_count_right() {
    let (s, r) = channel(cap(1));
    let inactive = r.deactivate();

    // A sender waits for an active receiver, which its waker will be woken for.
    let waker = Waker::from(Arc::new(PanicOnWake));
    let mut send = s.broadcast(1);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(|| inactive.activate_cloned())).is_err());
    assert_eq!(s.receiver_count(), 0);

    // No receiver is left uncounted, so the channel closes with its last inactive receiver.
    drop(inactive);
    drop(send);
    assert!(s.is_closed());
}

/// A receiver dropped as its drop wakes a waker that panics is counted out all the same, so the
/// channel closes with it.
#[test]
fn a_panicking_waker_in_a_receivers_drop_leaves_the_receiver_count_right() {
    let (s, r) = channel(cap(1));
    s.try_broadcast(1).unwrap();

    // A sender waits for room, which the drop of the receiver makes, and then the channel closes.
    let waker = Waker::from(Arc::new(PanicOnWake));
    let mut send = s.broadcast(2);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(move || drop(r))).is_err());
    assert_eq!(s.receiver_count(), 0);
    assert!(s.is_closed());

    drop(send);
}

#[test]
fn a_new_receiver_wakes_a_sender_waiting_for_an_active_one() {
    let (s, r) = channel(cap(1));
    let _inactive = r.deactivate();

    let (flag, waker) = Flag::new();
    let mut send = s.broadcast(1);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    assert!(!flag.woken());

    let mut r2 = s.new_receiver();
    assert!(flag.woken());
    assert_eq!(poll(&mut send, &waker), Poll::Ready(Ok(None)));
    assert_eq!(r2.try_recv(), Ok(1));
}

#[test]
fn growing_the_capacity_wakes_a_sender_waiting_for_room() {
    let (s, mut r) = channel(cap(1));
    s.try_broadcast(1).unwrap();

    let (flag, waker) = Flag::new();
    let mut send = s.broadcast(2);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    assert!(!flag.woken());

    // `send` borrows `s`, so grow the capacity through the receiver.
    r.set_capacity(cap(2));
    assert!(flag.woken());
    assert_eq!(poll(&mut send, &waker), Poll::Ready(Ok(None)));
    assert_eq!(r.try_recv(), Ok(1));
    assert_eq!(r.try_recv(), Ok(2));
}

#[test]
fn turning_overflow_on_wakes_all_the_senders_waiting_for_room() {
    let (s, mut r) = channel(cap(1));
    s.try_broadcast(1).unwrap();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut send_a = s.broadcast(2);
    let mut send_b = s.broadcast(3);
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);

    r.set_overflow(true);
    assert!(flag_a.woken());
    assert!(flag_b.woken());
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Ready(Ok(Some(1))));
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Ok(Some(2))));
    assert_eq!(r.try_recv(), Err(TryRecvError::Overflowed(2)));
    assert_eq!(r.try_recv(), Ok(3));
}

#[test]
fn no_longer_awaiting_active_wakes_a_sender_waiting_for_an_active_receiver() {
    let (s, r) = channel(cap(1));
    let mut inactive = r.deactivate();

    let (flag, waker) = Flag::new();
    let mut send = s.broadcast(1);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    assert!(!flag.woken());

    // `send` borrows `s`, so change the setting through the inactive receiver.
    inactive.set_await_active(false);
    assert!(flag.woken());
    assert_eq!(poll(&mut send, &waker), Poll::Ready(Err(SendError(1))));
}

/// A sender that fails after it was woken lets go of its listener as it fails, as one that
/// sends does: a completed future that is kept around must not hold on to a notification that a
/// later sender needs.
#[test]
fn a_sender_that_fails_passes_its_notification_on() {
    let (s, r) = channel(cap(1));
    let mut inactive = r.deactivate();

    let (flag_a, waker_a) = Flag::new();
    let mut send_a = s.broadcast(1);
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
    inactive.set_await_active(false);
    assert!(flag_a.woken());
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Ready(Err(SendError(1))));

    // `send_a` stays alive from here on.
    inactive.set_await_active(true);
    let mut r = inactive.activate_cloned();
    s.try_broadcast(2).unwrap();

    let (flag_b, waker_b) = Flag::new();
    let mut send_b = s.broadcast(3);
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);

    assert_eq!(r.try_recv(), Ok(2));
    assert!(flag_b.woken());
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Ok(None)));
    assert_eq!(r.try_recv(), Ok(3));
    drop(send_a);
}

#[test]
fn dropping_the_last_active_receiver_wakes_a_sender_waiting_for_room() {
    let (mut s, r) = channel(cap(1));
    let _inactive = r.clone().deactivate();
    s.set_await_active(false);
    s.try_broadcast(1).unwrap();

    let (flag, waker) = Flag::new();
    let mut send = s.broadcast(2);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    assert!(!flag.woken());

    drop(r);
    assert!(flag.woken());
    assert_eq!(poll(&mut send, &waker), Poll::Ready(Err(SendError(2))));
    assert!(!s.is_closed());
}

#[test]
#[timeout(10000)]
fn a_receiver_is_terminated_when_nothing_is_left_for_it() {
    let (s, mut r1) = channel(cap(1));
    let mut r2 = r1.clone();
    s.try_broadcast(1).unwrap();
    s.close();

    assert_eq!(block_on(r1.next()), Some(1));
    assert_eq!(block_on(r1.next()), None);
    assert!(r1.is_terminated());
    assert!(!r2.is_terminated());

    assert_eq!(block_on(r2.next()), Some(1));
    assert_eq!(block_on(r2.next()), None);
    assert!(r2.is_terminated());
}
