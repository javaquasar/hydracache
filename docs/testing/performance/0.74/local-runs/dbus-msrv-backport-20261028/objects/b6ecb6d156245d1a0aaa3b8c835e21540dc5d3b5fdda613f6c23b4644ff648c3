//! Tests of the MPMC channel, `zruntime::mpmc`.
//!
//! All of them are new, written for this channel. The first ones drive a channel from one thread
//! and check what its calls report: that messages come out in the order they went in, what a full
//! or a closed channel hands back, what the ends say of the channel, and what is counted and
//! dropped as senders and receivers come and go. A few run its futures under `block_on`.
//!
//! The wakeup tests poll the futures by hand, with wakers that only set a flag, so that each says
//! exactly who a send, a receive, a close or a drop wakes and who is left waiting, with no timing
//! in it. They pin down the channel's notification protocol: that a second send reaches a receiver
//! beside one that is notified already, that a future that was woken and gives up passes the wakeup
//! on, that one that was woken and runs does not, and that a receiver polled as a stream, which
//! keeps its listener from one poll to the next, never holds a wakeup that a `Recv` needs.
//!
//! The tests after those run many threads on one channel, to check that every message is received
//! exactly once, however the receivers receive, and that a close racing with the operations
//! waiting for it strands none of them. They are shrunk under Miri, which runs them far more
//! slowly. The last ones check that the futures and the receiver are `Unpin` whatever the message,
//! that a waker that panics, or a message whose drop does, leaves the channel as it should be, and
//! that the ends, the futures and the errors can be printed.
//!
//! What the ends of a channel may be sent to another thread with, and shared with, is not for these
//! tests to show: it takes code that does not compile, which the doc tests in `tests/mod.rs` are.

use std::{
    future::Future,
    marker::PhantomPinned,
    num::NonZeroUsize,
    panic::{self, AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    thread,
};

use futures::{
    executor::block_on,
    future::join,
    stream::{FusedStream, StreamExt},
};
use futures_core::Stream;
use ntest::timeout;

use super::{or, yield_now};
use crate::mpmc::{
    self, Receiver, RecvError, SendError, Sender, TryRecvError, TrySendError, bounded, unbounded,
};

#[test]
fn try_send_and_try_recv_move_messages_through() {
    let (s, r) = unbounded();
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty));

    s.try_send(1).unwrap();
    s.try_send(2).unwrap();
    assert_eq!(r.try_recv(), Ok(1));
    assert_eq!(r.try_recv(), Ok(2));
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty));

    // A bounded channel does the same for as long as it has room.
    let (s, r) = bounded(cap(1));
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty));
    s.try_send(3).unwrap();
    assert_eq!(r.try_recv(), Ok(3));
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty));
}

/// The channel is kept nearly full while the messages go through it, from one of two senders to
/// one of two receivers, so that they wrap around the buffer of its queue again and again. They
/// come out in the order they went in, whichever end is used.
#[test]
fn messages_come_out_in_the_order_they_went_in() {
    let (s1, r1) = bounded(cap(3));
    let (s2, r2) = (s1.clone(), r1.clone());

    let mut next_in = 0;
    let mut next_out = 0;
    for _ in 0..3 {
        s1.try_send(next_in).unwrap();
        next_in += 1;
    }
    for round in 0..20 {
        let (s, r) = if round % 2 == 0 {
            (&s1, &r2)
        } else {
            (&s2, &r1)
        };

        assert_eq!(r.try_recv(), Ok(next_out));
        next_out += 1;
        s.try_send(next_in).unwrap();
        next_in += 1;
    }
    for _ in 0..3 {
        assert_eq!(r1.try_recv(), Ok(next_out));
        next_out += 1;
    }
    assert_eq!(r1.try_recv(), Err(TryRecvError::Empty));
}

/// A `try_send` into a full channel fails and hands the message back, which the channel has not
/// kept: a message that is not `Copy` shows it.
#[test]
fn try_send_on_a_full_channel_hands_the_message_back() {
    let (s, r) = bounded(cap(2));
    s.try_send("one".to_string()).unwrap();
    s.try_send("two".to_string()).unwrap();

    let err = s.try_send("three".to_string()).unwrap_err();
    assert!(err.is_full());
    assert!(!err.is_closed());
    assert_eq!(err, TrySendError::Full("three".to_string()));
    assert_eq!(err.into_inner(), "three");
    assert_eq!(s.len(), 2);

    // Once a receive has made room, the message the failure handed back goes in.
    assert_eq!(r.try_recv().unwrap(), "one");
    s.try_send("three".to_string()).unwrap();
    assert_eq!(r.try_recv().unwrap(), "two");
    assert_eq!(r.try_recv().unwrap(), "three");
}

#[test]
fn a_bounded_channel_reports_its_length_and_capacity() {
    let (s, r) = bounded(cap(2));
    assert_eq!((s.capacity(), r.capacity()), (Some(cap(2)), Some(cap(2))));
    assert_eq!(state(&s, &r), (0, true, false));

    s.try_send(1).unwrap();
    assert_eq!(state(&s, &r), (1, false, false));
    s.try_send(2).unwrap();
    assert_eq!(state(&s, &r), (2, false, true));
    // A failed send changes nothing.
    assert!(s.try_send(3).is_err());
    assert_eq!(state(&s, &r), (2, false, true));

    r.try_recv().unwrap();
    assert_eq!(state(&s, &r), (1, false, false));
    r.try_recv().unwrap();
    assert_eq!(state(&s, &r), (0, true, false));
    assert_eq!((s.capacity(), r.capacity()), (Some(cap(2)), Some(cap(2))));
}

#[test]
fn an_unbounded_channel_is_never_full() {
    let (s, r) = unbounded();
    assert_eq!((s.capacity(), r.capacity()), (None, None));
    assert_eq!(state(&s, &r), (0, true, false));

    for i in 0..1000 {
        s.try_send(i).unwrap();
    }
    assert_eq!(state(&s, &r), (1000, false, false));

    for i in 0..1000 {
        assert_eq!(r.try_recv(), Ok(i));
    }
    assert_eq!(state(&s, &r), (0, true, false));
    assert_eq!((s.capacity(), r.capacity()), (None, None));
}

#[test]
fn the_ends_are_counted_through_clones_and_drops() {
    let (s1, r1) = unbounded::<()>();
    // Both ends count the same.
    let counts = |expected: (usize, usize)| {
        assert_eq!((s1.sender_count(), s1.receiver_count()), expected);
        assert_eq!((r1.sender_count(), r1.receiver_count()), expected);
    };
    counts((1, 1));

    let s2 = s1.clone();
    counts((2, 1));
    let s3 = s2.clone();
    let r2 = r1.clone();
    counts((3, 2));
    let r3 = r2.clone();
    counts((3, 3));

    drop(s2);
    drop(r2);
    counts((2, 2));
    drop(s3);
    drop(r3);
    counts((1, 1));

    drop(s1);
    assert_eq!((r1.sender_count(), r1.receiver_count()), (0, 1));
    assert!(r1.is_closed());
}

/// A channel of the largest capacity there is takes no room up front, as one that made its queue
/// with that capacity would have to: that would not even be possible.
#[test]
fn a_bounded_channel_of_the_largest_capacity_allocates_nothing_up_front() {
    let (s, r) = bounded::<u8>(NonZeroUsize::MAX);
    assert_eq!(
        (s.capacity(), r.capacity()),
        (Some(NonZeroUsize::MAX), Some(NonZeroUsize::MAX))
    );
    assert_eq!(state(&s, &r), (0, true, false));

    s.try_send(1).unwrap();
    s.try_send(2).unwrap();
    assert_eq!(state(&s, &r), (2, false, false));
    assert_eq!(r.try_recv(), Ok(1));
    assert_eq!(r.try_recv(), Ok(2));
}

#[test]
fn closing_returns_true_once_from_either_side() {
    for closer in Closer::BOTH {
        let (s, r) = unbounded::<u8>();
        assert!(!s.is_closed());
        assert!(!r.is_closed());

        assert!(closer.close(&s, &r), "{closer:?} closes the channel");
        assert!(s.is_closed());
        assert!(r.is_closed());

        // It is closed already, whichever end asks again.
        assert!(!s.close());
        assert!(!r.close());
        assert!(!closer.close(&s, &r), "{closer:?} closes it again");
        // Closing is not the end of either side.
        assert_eq!((s.sender_count(), s.receiver_count()), (1, 1));
    }
}

/// A closed channel refuses a message whether it has room or not, and hands it back as `Closed`,
/// never as `Full`.
#[test]
#[timeout(10000)]
fn a_closed_channel_refuses_sends_and_hands_the_message_back() {
    for closer in Closer::BOTH {
        // A channel with room, one that is full, and one that never is.
        let (with_room_s, with_room_r) = bounded::<u8>(cap(1));
        let (full_s, full_r) = bounded::<u8>(cap(1));
        full_s.try_send(0).unwrap();

        for (s, r) in [(with_room_s, with_room_r), (full_s, full_r), unbounded()] {
            assert!(closer.close(&s, &r));
            let len = s.len();

            let err = s.try_send(7).unwrap_err();
            assert!(err.is_closed());
            assert!(!err.is_full());
            assert_eq!(err, TrySendError::Closed(7));
            assert_eq!(block_on(s.send(8)), Err(SendError(8)));
            assert_eq!(SendError(9).into_inner(), 9);
            // Neither of them put anything in.
            assert_eq!(s.len(), len);
        }
    }
}

/// A `send` that waits for room and finds the channel closed by the time there is some fails, and
/// does not put its message in the room.
#[test]
#[timeout(10000)]
fn a_send_woken_for_room_finds_the_channel_closed_and_fails() {
    for closer in Closer::BOTH {
        let (s, r) = bounded(cap(1));
        s.try_send(1).unwrap();

        let (flag, waker) = Flag::new();
        let mut send = s.send(2);
        assert_eq!(poll(&mut send, &waker), Poll::Pending);

        assert_eq!(r.try_recv(), Ok(1));
        assert!(flag.woken());
        assert!(closer.close(&s, &r));
        assert_eq!(poll(&mut send, &waker), Poll::Ready(Err(SendError(2))));
        assert!(s.is_empty());
    }
}

/// A closed channel is closed to its senders only: the messages in it are still received, and the
/// receivers are told it is closed once none is left, rather than that it is empty.
#[test]
#[timeout(10000)]
fn a_closed_channel_hands_out_the_messages_left_and_then_closed_errors() {
    for closer in Closer::BOTH {
        let (s, r) = unbounded();
        s.try_send(1).unwrap();
        s.try_send(2).unwrap();
        s.try_send(3).unwrap();
        assert!(closer.close(&s, &r));

        assert_eq!(r.try_recv(), Ok(1));
        assert_eq!(block_on(r.recv()), Ok(2));
        assert_eq!(r.try_recv(), Ok(3));
        assert!(r.is_empty());

        let err = r.try_recv().unwrap_err();
        assert!(err.is_closed());
        assert!(!err.is_empty());
        assert_eq!(err, TryRecvError::Closed);
        assert_eq!(block_on(r.recv()), Err(RecvError));
    }
}

#[test]
#[timeout(10000)]
fn a_closed_channel_ends_the_stream_after_the_messages_left() {
    for closer in Closer::BOTH {
        let (s, mut r) = unbounded();
        s.try_send(1).unwrap();
        s.try_send(2).unwrap();
        assert!(closer.close(&s, &r));

        assert_eq!(block_on(r.next()), Some(1));
        assert_eq!(block_on(r.next()), Some(2));
        assert_eq!(block_on(r.next()), None);
        // The stream stays ended.
        assert_eq!(block_on(r.next()), None);
    }
}

/// A receiver that is a stream is terminated only once the channel is closed and nothing is left
/// for it to yield: not while the channel is open, empty or not, and not while it is closed but
/// still holds messages.
#[test]
#[timeout(10000)]
fn a_stream_is_terminated_once_the_channel_is_closed_and_drained() {
    for closer in Closer::BOTH {
        let (s, mut r) = unbounded();
        assert!(!r.is_terminated());
        s.try_send(1).unwrap();
        assert!(!r.is_terminated());

        assert!(closer.close(&s, &r));
        assert!(!r.is_terminated());

        assert_eq!(block_on(r.next()), Some(1));
        assert!(r.is_terminated());
        assert_eq!(block_on(r.next()), None);
        assert!(r.is_terminated());
    }
}

#[test]
fn dropping_the_last_sender_closes_the_channel() {
    let (s1, r) = unbounded();
    let s2 = s1.clone();
    s1.try_send(1).unwrap();

    // Another sender is left, so the channel stays open.
    drop(s1);
    assert!(!r.is_closed());
    assert_eq!(r.sender_count(), 1);
    s2.try_send(2).unwrap();

    drop(s2);
    assert!(r.is_closed());
    assert_eq!(r.sender_count(), 0);
    // The messages left are still received, and then the channel says it is closed.
    assert_eq!(r.try_recv(), Ok(1));
    assert_eq!(r.try_recv(), Ok(2));
    assert_eq!(r.try_recv(), Err(TryRecvError::Closed));
    assert!(!r.close());
}

#[test]
fn dropping_the_last_receiver_closes_the_channel() {
    let (s, r1) = unbounded();
    let r2 = r1.clone();

    // Another receiver is left, so the channel stays open.
    drop(r1);
    assert!(!s.is_closed());
    assert_eq!(s.receiver_count(), 1);
    s.try_send(1).unwrap();

    drop(r2);
    assert!(s.is_closed());
    assert_eq!(s.receiver_count(), 0);
    assert_eq!(s.try_send(2), Err(TrySendError::Closed(2)));
    assert!(!s.close());
}

/// The last receiver drops the messages left, once each and there and then, with the senders still
/// there and whether or not the channel was closed already. A receiver that is not the last drops
/// none of them.
#[test]
fn dropping_the_last_receiver_drops_the_messages_left_once_each() {
    for close_first in [false, true] {
        let drops = Drops::default();
        let (s, r1) = bounded(cap(4));
        let r2 = r1.clone();
        for _ in 0..4 {
            s.try_send(drops.message()).unwrap();
        }

        // One message leaves the channel, and is dropped by whoever took it.
        drop(r1.try_recv().unwrap());
        assert_eq!(drops.count(), 1);
        if close_first {
            assert!(s.close());
        }

        drop(r1);
        assert_eq!(drops.count(), 1);
        assert_eq!(s.len(), 3);

        drop(r2);
        assert_eq!(drops.count(), 4);
        assert!(s.is_empty());
        assert!(s.is_closed());

        // Nothing is left to drop a second time.
        drop(s);
        assert_eq!(drops.count(), 4);
    }
}

/// The channel's lock is not held while the messages left are dropped. Had it been, the drop of the
/// sender this message holds would wait for it for good: the timeout is what would tell.
#[test]
#[timeout(10000)]
fn a_message_holding_a_sender_of_its_own_channel_is_freed_with_the_last_receiver() {
    let token = Arc::new(());
    let (s, r) = unbounded();
    s.try_send(Looped {
        _sender: s.clone(),
        _token: token.clone(),
    })
    .unwrap();
    assert_eq!(s.sender_count(), 2);
    assert_eq!(Arc::strong_count(&token), 2);

    drop(r);
    assert_eq!(Arc::strong_count(&token), 1);
    assert_eq!(s.sender_count(), 1);
    assert!(s.is_closed());
}

/// A message dropped with the last receiver finds the channel closed and without a receiver, and
/// can use it as it is dropped: the channel's lock is not held then, or this would never end.
#[test]
#[timeout(10000)]
fn a_message_dropped_with_the_last_receiver_finds_the_channel_closed() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (s, r) = unbounded();
    for _ in 0..2 {
        s.try_send(Observer {
            sender: s.clone(),
            seen: seen.clone(),
        })
        .unwrap();
    }

    drop(r);
    assert_eq!(*seen.lock().unwrap(), [(true, 0), (true, 0)]);
    assert_eq!(s.sender_count(), 1);
}

/// A `Send` dropped while it waits for room has sent nothing, and its message is dropped with it.
#[test]
#[timeout(10000)]
fn a_dropped_send_drops_its_message_and_sends_nothing() {
    let drops = Drops::default();
    let (s, r) = bounded(cap(1));
    s.try_send(drops.message()).unwrap();

    let mut send = s.send(drops.message());
    assert!(poll(&mut send, Waker::noop()).is_pending());
    assert_eq!(drops.count(), 0);

    drop(send);
    assert_eq!(drops.count(), 1);
    assert_eq!(s.len(), 1);

    drop(r.try_recv().unwrap());
    assert_eq!(drops.count(), 2);
    assert_eq!(r.try_recv().err(), Some(TryRecvError::Empty));
}

#[test]
#[timeout(10000)]
fn async_send_and_recv_move_messages_through() {
    block_on(async {
        let (s, r) = bounded(cap(2));
        s.send(1).await.unwrap();
        s.send(2).await.unwrap();
        assert_eq!(r.recv().await, Ok(1));
        assert_eq!(r.recv().await, Ok(2));

        let (s, r) = unbounded();
        for i in 0..100 {
            s.send(i).await.unwrap();
        }
        for i in 0..100 {
            assert_eq!(r.recv().await, Ok(i));
        }

        // Once the channel is closed, and the messages in it are received, a receive fails.
        drop(s);
        assert_eq!(r.recv().await, Err(RecvError));
    });
}

/// A channel of capacity one handing messages from a thread that sends to one that receives, each
/// waiting in `block_on` for the other in turn.
#[test]
#[timeout(30000)]
fn a_thread_waiting_in_block_on_is_woken_by_the_other_end() {
    let messages = if cfg!(miri) { 10 } else { 500 };
    let (s, r) = bounded(cap(1));

    thread::scope(|scope| {
        scope.spawn(move || {
            for i in 0..messages {
                block_on(s.send(i)).unwrap();
            }
        });

        for i in 0..messages {
            assert_eq!(block_on(r.recv()), Ok(i));
        }
        assert_eq!(block_on(r.recv()), Err(RecvError));
    });
}

#[test]
#[timeout(10000)]
fn a_receiver_is_a_stream_of_its_messages() {
    let messages = if cfg!(miri) { 10 } else { 200 };
    let (s, r) = bounded(cap(2));

    thread::scope(|scope| {
        scope.spawn(move || {
            for i in 0..messages {
                block_on(s.send(i)).unwrap();
            }
        });

        // The stream ends once the sender thread is done with the last sender.
        assert_eq!(
            block_on(r.collect::<Vec<_>>()),
            (0..messages).collect::<Vec<_>>()
        );
    });
}

/// `recv` takes `&self`, so one receiver serves as many `Recv`s as there are at once, and each
/// message goes to one of them, the oldest to the one that is polled first.
#[test]
#[timeout(10000)]
fn one_receiver_serves_several_recvs_at_once() {
    let (s, r) = unbounded();
    s.try_send(1).unwrap();
    s.try_send(2).unwrap();

    assert_eq!(block_on(join(r.recv(), r.recv())), (Ok(1), Ok(2)));
}

#[test]
#[timeout(10000)]
fn a_send_into_a_full_channel_waits_until_a_receive_makes_room() {
    let (s, r) = bounded(cap(1));
    s.try_send(1).unwrap();

    let (flag, waker) = Flag::new();
    let mut send = s.send(2);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    // Polled again with no room made, it goes on waiting.
    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    assert!(!flag.woken());
    assert_eq!(s.len(), 1);

    assert_eq!(r.try_recv(), Ok(1));
    assert!(flag.woken());
    assert_eq!(poll(&mut send, &waker), Poll::Ready(Ok(())));
    assert_eq!(r.try_recv(), Ok(2));
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty));
}

#[test]
#[timeout(10000)]
fn a_send_into_an_unbounded_channel_never_waits() {
    let (s, _r) = unbounded();

    for i in 0..100 {
        assert_eq!(poll(&mut s.send(i), Waker::noop()), Poll::Ready(Ok(())));
    }
    assert_eq!(s.len(), 100);
}

#[test]
#[timeout(10000)]
fn a_send_wakes_a_waiting_recv() {
    let (s, r) = unbounded();

    let (flag, waker) = Flag::new();
    let mut recv = r.recv();
    assert_eq!(poll(&mut recv, &waker), Poll::Pending);
    assert!(!flag.woken());

    s.try_send(1).unwrap();
    assert!(flag.woken());
    assert_eq!(poll(&mut recv, &waker), Poll::Ready(Ok(1)));
}

/// The second send wakes the second `Recv` though the first is notified already and has not run:
/// it notifies one more waiter, rather than counting the one that is notified. Counted, it would
/// reach nobody, and the second `Recv` would wait with a message in the channel.
#[test]
#[timeout(10000)]
fn two_sends_wake_two_waiting_recvs() {
    let (s, r) = unbounded();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut recv_a = r.recv();
    let mut recv_b = r.recv();
    assert_eq!(poll(&mut recv_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut recv_b, &waker_b), Poll::Pending);

    s.try_send(1).unwrap();
    assert!(flag_a.woken());
    assert!(!flag_b.woken());
    s.try_send(2).unwrap();
    assert!(flag_b.woken());

    assert_eq!(poll(&mut recv_a, &waker_a), Poll::Ready(Ok(1)));
    assert_eq!(poll(&mut recv_b, &waker_b), Poll::Ready(Ok(2)));
}

/// One message is one wakeup, for the `Recv` that has waited longest. That `Recv` takes its
/// notification as it completes, so the other is not woken for nothing: a `Recv` that tried before
/// it polled its listener would drop the listener notified, which passes the notification on to the
/// next waiter, and wake it with no message to take.
#[test]
#[timeout(10000)]
fn one_send_wakes_only_the_first_of_two_waiting_recvs() {
    let (s, r) = unbounded();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut recv_a = r.recv();
    let mut recv_b = r.recv();
    assert_eq!(poll(&mut recv_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut recv_b, &waker_b), Poll::Pending);

    s.try_send(1).unwrap();
    assert!(flag_a.woken());
    assert!(!flag_b.woken());

    assert_eq!(poll(&mut recv_a, &waker_a), Poll::Ready(Ok(1)));
    assert!(!flag_b.woken());
    // The first `Recv` is kept, and holds no notification: letting go of it passes none on.
    drop(recv_a);
    assert!(!flag_b.woken());
    assert_eq!(poll(&mut recv_b, &waker_b), Poll::Pending);
    assert!(!flag_b.woken());

    s.try_send(2).unwrap();
    assert!(flag_b.woken());
    assert_eq!(poll(&mut recv_b, &waker_b), Poll::Ready(Ok(2)));
}

/// A future that is polled again is woken through the waker of its latest poll, as a task that has
/// moved to another waker needs, and not through the one of its first poll.
#[test]
#[timeout(10000)]
fn a_recv_polled_again_is_woken_through_the_waker_of_its_latest_poll() {
    let (s, r) = unbounded();

    let (old_flag, old_waker) = Flag::new();
    let (new_flag, new_waker) = Flag::new();
    let mut recv = r.recv();
    assert_eq!(poll(&mut recv, &old_waker), Poll::Pending);
    assert_eq!(poll(&mut recv, &new_waker), Poll::Pending);

    s.try_send(1).unwrap();
    assert!(new_flag.woken());
    assert!(!old_flag.woken());
    assert_eq!(poll(&mut recv, &new_waker), Poll::Ready(Ok(1)));
}

/// As for a `Recv`, a `Send` polled again is woken through the waker of its latest poll.
#[test]
#[timeout(10000)]
fn a_send_polled_again_is_woken_through_the_waker_of_its_latest_poll() {
    let (s, r) = bounded(cap(1));
    s.try_send(1).unwrap();

    let (old_flag, old_waker) = Flag::new();
    let (new_flag, new_waker) = Flag::new();
    let mut send = s.send(2);
    assert_eq!(poll(&mut send, &old_waker), Poll::Pending);
    assert_eq!(poll(&mut send, &new_waker), Poll::Pending);

    assert_eq!(r.try_recv(), Ok(1));
    assert!(new_flag.woken());
    assert!(!old_flag.woken());
    assert_eq!(poll(&mut send, &new_waker), Poll::Ready(Ok(())));
}

/// As for a `Recv`, a receiver polled as a stream again is woken through the waker of its latest
/// poll.
#[test]
#[timeout(10000)]
fn a_stream_polled_again_is_woken_through_the_waker_of_its_latest_poll() {
    let (s, mut r) = unbounded();

    let (old_flag, old_waker) = Flag::new();
    let (new_flag, new_waker) = Flag::new();
    assert_eq!(poll_next(&mut r, &old_waker), Poll::Pending);
    assert_eq!(poll_next(&mut r, &new_waker), Poll::Pending);

    s.try_send(1).unwrap();
    assert!(new_flag.woken());
    assert!(!old_flag.woken());
    assert_eq!(poll_next(&mut r, &new_waker), Poll::Ready(Some(1)));
}

/// A `Recv` that is dropped after it was notified and before it ran gives its notification to the
/// next waiter, which then gets the message.
#[test]
#[timeout(10000)]
fn a_notified_recv_dropped_unpolled_passes_its_notification_on() {
    let (s, r) = unbounded();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut recv_a = r.recv();
    let mut recv_b = r.recv();
    assert_eq!(poll(&mut recv_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut recv_b, &waker_b), Poll::Pending);

    s.try_send(1).unwrap();
    assert!(flag_a.woken());
    assert!(!flag_b.woken());

    drop(recv_a);
    assert!(flag_b.woken());
    assert_eq!(poll(&mut recv_b, &waker_b), Poll::Ready(Ok(1)));
}

/// A `Recv` that is woken for a message that somebody else takes first has not been wronged: it
/// waits again, with a listener of its own and the same waker, and the next message wakes it.
#[test]
#[timeout(10000)]
fn a_notified_recv_that_finds_the_message_taken_goes_back_to_waiting() {
    let (s, r) = unbounded();

    let (flag, waker) = Flag::new();
    let mut recv = r.recv();
    assert_eq!(poll(&mut recv, &waker), Poll::Pending);

    s.try_send(1).unwrap();
    assert!(flag.woken());
    assert_eq!(r.try_recv(), Ok(1));

    assert_eq!(poll(&mut recv, &waker), Poll::Pending);
    assert!(!flag.woken());
    s.try_send(2).unwrap();
    assert!(flag.woken());
    assert_eq!(poll(&mut recv, &waker), Poll::Ready(Ok(2)));
}

#[test]
#[timeout(10000)]
fn a_receive_wakes_a_waiting_send() {
    let (s, r) = bounded(cap(1));
    s.try_send(1).unwrap();

    let (flag, waker) = Flag::new();
    let mut send = s.send(2);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    assert!(!flag.woken());

    assert_eq!(r.try_recv(), Ok(1));
    assert!(flag.woken());
    assert_eq!(poll(&mut send, &waker), Poll::Ready(Ok(())));
    assert_eq!(r.try_recv(), Ok(2));
}

/// The second receive wakes the second `Send` though the first is notified already and has not
/// run: it makes room for one more, which counts for one more waiter. The channel holds two
/// messages, so that there is a second to receive.
#[test]
#[timeout(10000)]
fn two_receives_wake_two_waiting_sends() {
    let (s, r) = bounded(cap(2));
    s.try_send(1).unwrap();
    s.try_send(2).unwrap();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut send_a = s.send(3);
    let mut send_b = s.send(4);
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);

    assert_eq!(r.try_recv(), Ok(1));
    assert!(flag_a.woken());
    assert!(!flag_b.woken());
    assert_eq!(r.try_recv(), Ok(2));
    assert!(flag_b.woken());

    assert_eq!(poll(&mut send_a, &waker_a), Poll::Ready(Ok(())));
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Ok(())));
    assert_eq!(r.try_recv(), Ok(3));
    assert_eq!(r.try_recv(), Ok(4));
}

/// One slot of room is one wakeup, for the `Send` that has waited longest, as one message is for a
/// `Recv`: that `Send` takes its notification as it completes, and the other is not woken for room
/// that is gone again.
#[test]
#[timeout(10000)]
fn one_receive_wakes_only_the_first_of_two_waiting_sends() {
    let (s, r) = bounded(cap(1));
    s.try_send(1).unwrap();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut send_a = s.send(2);
    let mut send_b = s.send(3);
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);

    assert_eq!(r.try_recv(), Ok(1));
    assert!(flag_a.woken());
    assert!(!flag_b.woken());

    assert_eq!(poll(&mut send_a, &waker_a), Poll::Ready(Ok(())));
    assert!(!flag_b.woken());
    // The first `Send` is kept, and holds no notification: letting go of it passes none on.
    drop(send_a);
    assert!(!flag_b.woken());
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);
    assert!(!flag_b.woken());

    assert_eq!(r.try_recv(), Ok(2));
    assert!(flag_b.woken());
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Ok(())));
    assert_eq!(r.try_recv(), Ok(3));
}

/// A `Send` that is dropped after it was notified and before it ran gives its notification to the
/// next waiter, which then has the room. The message the dropped one held goes with it.
#[test]
#[timeout(10000)]
fn a_notified_send_dropped_unpolled_passes_its_notification_on() {
    let (s, r) = bounded(cap(1));
    s.try_send(1).unwrap();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut send_a = s.send(2);
    let mut send_b = s.send(3);
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);

    assert_eq!(r.try_recv(), Ok(1));
    assert!(flag_a.woken());
    assert!(!flag_b.woken());

    drop(send_a);
    assert!(flag_b.woken());
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Ok(())));
    assert_eq!(r.try_recv(), Ok(3));
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty));
}

/// A `Send` that is woken for room that somebody else takes first waits again with its message,
/// which the failed attempt has not lost, and the next receive wakes it.
#[test]
#[timeout(10000)]
fn a_notified_send_that_finds_the_room_taken_goes_back_to_waiting() {
    let (s, r) = bounded(cap(1));
    s.try_send(1).unwrap();

    let (flag, waker) = Flag::new();
    let mut send = s.send(2);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);

    assert_eq!(r.try_recv(), Ok(1));
    assert!(flag.woken());
    s.try_send(3).unwrap();

    assert_eq!(poll(&mut send, &waker), Poll::Pending);
    assert!(!flag.woken());
    assert_eq!(r.try_recv(), Ok(3));
    assert!(flag.woken());
    assert_eq!(poll(&mut send, &waker), Poll::Ready(Ok(())));
    assert_eq!(r.try_recv(), Ok(2));
}

/// Senders wait on a full channel, so this is the full channel's side of closing; the receivers'
/// side follows. The messages in the channel are still there for the receivers afterwards.
#[test]
#[timeout(10000)]
fn closing_wakes_every_waiting_send() {
    for closer in Closer::BOTH {
        let (s, r) = bounded(cap(1));
        s.try_send(1).unwrap();

        let (flag_a, waker_a) = Flag::new();
        let (flag_b, waker_b) = Flag::new();
        let mut send_a = s.send(2);
        let mut send_b = s.send(3);
        assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
        assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);
        assert!(!flag_a.woken());
        assert!(!flag_b.woken());

        assert!(closer.close(&s, &r));
        assert!(flag_a.woken());
        assert!(flag_b.woken());
        // Each gets its own message back.
        assert_eq!(poll(&mut send_a, &waker_a), Poll::Ready(Err(SendError(2))));
        assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Err(SendError(3))));
        assert_eq!(r.try_recv(), Ok(1));
    }
}

/// Receivers wait on an empty channel: two `Recv`s of one receiver, and two receivers polled as
/// streams, each with a waker of its own.
#[test]
#[timeout(10000)]
fn closing_wakes_every_waiting_recv_and_stream() {
    for closer in Closer::BOTH {
        let (s, r1) = unbounded::<u8>();
        let (mut r2, mut r3) = (r1.clone(), r1.clone());

        let (flag_a, waker_a) = Flag::new();
        let (flag_b, waker_b) = Flag::new();
        let (flag_c, waker_c) = Flag::new();
        let (flag_d, waker_d) = Flag::new();
        let mut recv_a = r1.recv();
        let mut recv_b = r1.recv();
        assert_eq!(poll(&mut recv_a, &waker_a), Poll::Pending);
        assert_eq!(poll(&mut recv_b, &waker_b), Poll::Pending);
        assert_eq!(poll_next(&mut r2, &waker_c), Poll::Pending);
        assert_eq!(poll_next(&mut r3, &waker_d), Poll::Pending);
        assert!(!flag_a.woken());
        assert!(!flag_b.woken());
        assert!(!flag_c.woken());
        assert!(!flag_d.woken());

        assert!(closer.close(&s, &r1));
        assert!(flag_a.woken());
        assert!(flag_b.woken());
        assert!(flag_c.woken());
        assert!(flag_d.woken());
        assert_eq!(poll(&mut recv_a, &waker_a), Poll::Ready(Err(RecvError)));
        assert_eq!(poll(&mut recv_b, &waker_b), Poll::Ready(Err(RecvError)));
        assert_eq!(poll_next(&mut r2, &waker_c), Poll::Ready(None));
        assert_eq!(poll_next(&mut r3, &waker_d), Poll::Ready(None));
    }
}

/// The last sender is the one that closes the channel, so the receivers waiting are not woken by
/// a sender that is not the last.
#[test]
#[timeout(10000)]
fn dropping_the_last_sender_wakes_waiting_recvs_and_streams() {
    let (s1, r1) = unbounded::<u8>();
    let s2 = s1.clone();
    let mut r2 = r1.clone();

    let (recv_flag, recv_waker) = Flag::new();
    let (stream_flag, stream_waker) = Flag::new();
    let mut recv = r1.recv();
    assert_eq!(poll(&mut recv, &recv_waker), Poll::Pending);
    assert_eq!(poll_next(&mut r2, &stream_waker), Poll::Pending);

    drop(s1);
    assert!(!recv_flag.woken());
    assert!(!stream_flag.woken());

    drop(s2);
    assert!(recv_flag.woken());
    assert!(stream_flag.woken());
    assert_eq!(poll(&mut recv, &recv_waker), Poll::Ready(Err(RecvError)));
    assert_eq!(poll_next(&mut r2, &stream_waker), Poll::Ready(None));
}

/// The last receiver is the one that closes the channel, so the senders waiting are not woken by
/// a receiver that is not the last.
#[test]
#[timeout(10000)]
fn dropping_the_last_receiver_wakes_waiting_sends() {
    let (s, r1) = bounded(cap(1));
    let r2 = r1.clone();
    s.try_send(1).unwrap();

    let (flag_a, waker_a) = Flag::new();
    let (flag_b, waker_b) = Flag::new();
    let mut send_a = s.send(2);
    let mut send_b = s.send(3);
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Pending);
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Pending);

    drop(r1);
    assert!(!flag_a.woken());
    assert!(!flag_b.woken());

    drop(r2);
    assert!(flag_a.woken());
    assert!(flag_b.woken());
    assert_eq!(poll(&mut send_a, &waker_a), Poll::Ready(Err(SendError(2))));
    assert_eq!(poll(&mut send_b, &waker_b), Poll::Ready(Err(SendError(3))));
}

/// Every stream waiting is woken by every message, and the one that is polled first gets it: the
/// others find the channel empty again and go back to waiting for the next.
#[test]
#[timeout(10000)]
fn one_send_wakes_every_waiting_stream() {
    let (s, mut r1) = unbounded();
    let mut r2 = r1.clone();

    let (flag_1, waker_1) = Flag::new();
    let (flag_2, waker_2) = Flag::new();
    assert_eq!(poll_next(&mut r1, &waker_1), Poll::Pending);
    assert_eq!(poll_next(&mut r2, &waker_2), Poll::Pending);
    assert!(!flag_1.woken());
    assert!(!flag_2.woken());

    s.try_send(1).unwrap();
    assert!(flag_1.woken());
    assert!(flag_2.woken());

    assert_eq!(poll_next(&mut r2, &waker_2), Poll::Ready(Some(1)));
    assert_eq!(poll_next(&mut r1, &waker_1), Poll::Pending);
    assert!(!flag_1.woken());

    s.try_send(2).unwrap();
    assert!(flag_1.woken());
    assert_eq!(poll_next(&mut r1, &waker_1), Poll::Ready(Some(2)));
}

/// A stream keeps its listener from one poll to the next, so one that is woken and then not polled
/// again, because its `next()` lost a `select` or its task is busy, holds on to its wakeup. It is
/// not the one a `Recv` of another receiver needs, since streams wait on an event of their own and
/// are all woken by every message: one send wakes both, and the `Recv` gets the message. The order
/// they started to wait in makes no difference.
#[test]
#[timeout(10000)]
fn a_stream_woken_and_never_polled_again_does_not_keep_a_recv_from_being_woken() {
    for stream_first in [true, false] {
        let (s, r1) = unbounded();
        let mut r2 = r1.clone();

        let (recv_flag, recv_waker) = Flag::new();
        let (stream_flag, stream_waker) = Flag::new();
        let mut recv = r1.recv();
        if stream_first {
            assert_eq!(poll_next(&mut r2, &stream_waker), Poll::Pending);
            assert_eq!(poll(&mut recv, &recv_waker), Poll::Pending);
        } else {
            assert_eq!(poll(&mut recv, &recv_waker), Poll::Pending);
            assert_eq!(poll_next(&mut r2, &stream_waker), Poll::Pending);
        }
        assert!(!recv_flag.woken());
        assert!(!stream_flag.woken());

        s.try_send(7).unwrap();
        assert!(recv_flag.woken(), "stream first: {stream_first}");
        assert!(stream_flag.woken(), "stream first: {stream_first}");
        // The stream is not polled, as if its task never ran again, and the `Recv` is.
        assert_eq!(poll(&mut recv, &recv_waker), Poll::Ready(Ok(7)));

        // When the stream does run, it finds the message gone, waits for the next, and gets it.
        assert_eq!(poll_next(&mut r2, &stream_waker), Poll::Pending);
        assert!(!stream_flag.woken());
        s.try_send(8).unwrap();
        assert!(stream_flag.woken());
        assert_eq!(poll_next(&mut r2, &stream_waker), Poll::Ready(Some(8)));
    }
}

/// Senders and receivers on a channel of capacity one, which they all keep full or empty, the
/// receivers by every means there is.
#[test]
#[timeout(60000)]
fn every_message_is_received_once_on_a_channel_of_capacity_one() {
    assert_every_message_is_received_once(bounded(cap(1)), &Style::ALL);
}

#[test]
#[timeout(60000)]
fn every_message_is_received_once_on_a_channel_of_capacity_two() {
    assert_every_message_is_received_once(bounded(cap(2)), &Style::ALL);
}

#[test]
#[timeout(60000)]
fn every_message_is_received_once_on_an_unbounded_channel() {
    assert_every_message_is_received_once(unbounded(), &Style::ALL);
}

/// Receivers that only ever wait, with nobody to take a message that none of them was woken for:
/// a wakeup lost anywhere would leave a receiver, or a sender waiting for the room it makes, to
/// the timeout.
#[test]
#[timeout(60000)]
fn every_message_is_received_once_by_receivers_that_only_wait() {
    assert_every_message_is_received_once(bounded(cap(1)), &Style::WAITING);
    assert_every_message_is_received_once(unbounded(), &Style::WAITING);
}

/// A `Recv` takes its listener before its last look at the channel, so a message that comes as it
/// goes to sleep is one it finds or one it is woken for, never one it sleeps through. A thread that
/// waits for a request in `block_on` and answers it, and a thread that sends the next request the
/// moment it sees the answer, so that the request lands as the other goes to wait, take turns for
/// as many rounds as it takes for the two to meet: a message missed would leave the one waiting for
/// it and the other waiting for its answer, for the timeout to catch.
#[test]
#[timeout(60000)]
fn a_recv_is_not_left_asleep_by_a_message_that_comes_as_it_goes_to_wait() {
    ping_pong(|requests, responses| {
        while let Ok(request) = block_on(requests.recv()) {
            responses.try_send(request).unwrap();
        }
    });
}

/// As for a `Recv`, with a receiver polled as a stream, which listens on an event of its own.
#[test]
#[timeout(60000)]
fn a_stream_is_not_left_asleep_by_a_message_that_comes_as_it_goes_to_wait() {
    ping_pong(|mut requests, responses| {
        while let Some(request) = block_on(requests.next()) {
            responses.try_send(request).unwrap();
        }
    });
}

/// A `Recv` that completes lets go of its listener, which may be one it took only just now, as the
/// message came: kept after it completed, as a task that holds on to its futures keeps them, it
/// would stay in the queue of the event, and take the notification of the next message in place of
/// the `Recv` that waits for it. Here each `Recv` of the waiting side is kept until the end, and
/// the next one waits behind it.
#[test]
#[timeout(60000)]
fn a_recv_that_completed_and_is_kept_holds_no_notification_a_later_one_needs() {
    ping_pong(|requests, responses| {
        let mut completed = Vec::new();
        loop {
            let mut recv = requests.recv();
            match block_on(&mut recv) {
                Ok(request) => responses.try_send(request).unwrap(),
                Err(RecvError) => break,
            }
            completed.push(recv);
        }
    });
}

/// As for a `Recv`, with a `Send` that waits for room in a channel that holds one message, which
/// the thread that takes it takes as soon as it is there: room that comes as the `Send` goes to
/// sleep is room it finds or is woken for.
#[test]
#[timeout(60000)]
fn a_send_is_not_left_asleep_by_room_that_comes_as_it_goes_to_wait() {
    spin_receive(|sender, rounds| {
        block_on(async {
            for n in 0..rounds {
                sender.send(n).await.unwrap();
            }
        });
    });
}

/// As for a `Recv`, a `Send` that completed and is kept holds no notification that a later one
/// needs.
#[test]
#[timeout(60000)]
fn a_send_that_completed_and_is_kept_holds_no_notification_a_later_one_needs() {
    spin_receive(|sender, rounds| {
        block_on(async {
            let mut completed = Vec::new();
            for n in 0..rounds {
                let mut send = sender.send(n);
                (&mut send).await.unwrap();
                completed.push(send);
            }
        });
    });
}

/// A channel is never both empty and full, so each round has two: receivers wait on an empty one
/// and senders on a full one, and the main thread ends both at once while they start. A round
/// closes them from the senders' end, from the receivers' end, or by dropping the last of the end
/// that does not wait, and starts the closing after a head start of a different length each time,
/// so that it comes before, among and after the operations that it ends. None may be left waiting,
/// each receiver gets `RecvError` and each sender its message back.
#[test]
#[timeout(60000)]
fn a_close_racing_with_waiting_operations_strands_none_of_them() {
    let (waiters, rounds) = if cfg!(miri) { (2, 6) } else { (4, 240) };

    for round in 0..rounds {
        let (empty_s, empty_r) = unbounded::<usize>();
        let (full_s, full_r) = bounded::<usize>(cap(1));
        full_s.try_send(usize::MAX).unwrap();

        thread::scope(|scope| {
            for id in 0..waiters {
                let receiver = empty_r.clone();
                scope.spawn(move || assert_eq!(block_on(receiver.recv()), Err(RecvError)));
                let sender = full_s.clone();
                scope.spawn(move || assert_eq!(block_on(sender.send(id)), Err(SendError(id))));
            }

            for _ in 0..round % 8 {
                thread::yield_now();
            }
            match round % 3 {
                0 => {
                    assert!(empty_s.close());
                    assert!(full_s.close());
                }
                1 => {
                    assert!(empty_r.close());
                    assert!(full_r.close());
                }
                _ => {
                    drop(empty_s);
                    drop(full_r);
                }
            }
        });

        assert!(empty_r.is_closed());
        assert!(full_s.is_closed());
    }
}

/// The message is never pinned, so the futures and the receiver are `Unpin` for a message that is
/// not, and can be polled by hand without pinning them first.
#[test]
fn the_futures_and_the_receiver_are_unpin_whatever_the_message() {
    fn assert_unpin<T>()
    where
        T: Unpin,
    {
    }

    assert_unpin::<mpmc::Send<'static, u32>>();
    assert_unpin::<mpmc::Recv<'static, u32>>();
    assert_unpin::<Receiver<u32>>();
    assert_unpin::<Sender<u32>>();
    assert_unpin::<mpmc::Send<'static, PhantomPinned>>();
    assert_unpin::<mpmc::Recv<'static, PhantomPinned>>();
    assert_unpin::<Receiver<PhantomPinned>>();
    assert_unpin::<Sender<PhantomPinned>>();
}

/// The message was put in the channel before anybody was notified, so a waker that panics does not
/// keep it out, nor leave the counts wrong, nor stop the channel from being used.
#[test]
#[timeout(10000)]
fn a_panicking_waker_in_try_send_leaves_the_message_in_the_channel() {
    let (s, r) = unbounded();

    let waker = Waker::from(Arc::new(PanicOnWake));
    let mut recv = r.recv();
    assert_eq!(poll(&mut recv, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(|| s.try_send(1))).is_err());
    assert_eq!(s.len(), 1);
    assert_eq!((s.sender_count(), s.receiver_count()), (1, 1));
    assert!(!s.is_closed());

    assert_eq!(r.try_recv(), Ok(1));
    s.try_send(2).unwrap();
    assert_eq!(r.try_recv(), Ok(2));
    drop(recv);
}

/// The channel was closed before anybody was notified, so a waker that panics in the notification
/// of the last sender's drop leaves it closed, with that sender counted out.
#[test]
#[timeout(10000)]
fn a_panicking_waker_in_the_last_senders_drop_still_closes_the_channel() {
    let (s, r) = unbounded::<u8>();

    let waker = Waker::from(Arc::new(PanicOnWake));
    let mut recv = r.recv();
    assert_eq!(poll(&mut recv, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(move || drop(s))).is_err());
    assert!(r.is_closed());
    assert_eq!((r.sender_count(), r.receiver_count()), (0, 1));
    assert_eq!(r.try_recv(), Err(TryRecvError::Closed));
    drop(recv);
}

/// The message was taken out before the sender was notified, so a waker that panics in that
/// notification leaves the room that was made: the channel is not left looking full.
#[test]
#[timeout(10000)]
fn a_panicking_waker_in_try_recv_still_makes_room() {
    let (s, r) = bounded(cap(1));
    s.try_send(1).unwrap();

    let waker = Waker::from(Arc::new(PanicOnWake));
    let mut send = s.send(2);
    assert_eq!(poll(&mut send, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(|| r.try_recv())).is_err());
    assert!(r.is_empty());
    assert!(!s.is_full());

    // The `Send` was notified before the waker panicked, and goes in the room.
    assert_eq!(poll(&mut send, Waker::noop()), Poll::Ready(Ok(())));
    assert_eq!(r.try_recv(), Ok(2));
}

/// The messages left are dropped as the panic unwinds out of the last receiver's drop, once each,
/// and the channel is closed and has no receiver counted.
#[test]
#[timeout(10000)]
fn a_panicking_waker_in_the_last_receivers_drop_still_drops_the_messages_left() {
    let drops = Drops::default();
    let (s, r) = bounded(cap(1));
    s.try_send(drops.message()).unwrap();

    let waker = Waker::from(Arc::new(PanicOnWake));
    let mut send = s.send(drops.message());
    assert!(poll(&mut send, &waker).is_pending());

    assert!(catch_unwind(AssertUnwindSafe(move || drop(r))).is_err());
    assert_eq!(drops.count(), 1);
    assert!(s.is_closed());
    assert_eq!(s.receiver_count(), 0);
    assert!(s.is_empty());

    drop(send);
    assert_eq!(drops.count(), 2);
}

/// The last receiver tells the senders waiting before it drops the messages left, so a message
/// whose drop panics cannot keep one of them from being woken.
#[test]
#[timeout(10000)]
fn a_panicking_message_drop_does_not_keep_a_waiting_send_from_being_woken() {
    let (s, r) = bounded(cap(1));
    s.try_send(PanicOnDrop(true)).unwrap();

    let (flag, waker) = Flag::new();
    let mut send = s.send(PanicOnDrop(false));
    assert!(poll(&mut send, &waker).is_pending());

    assert!(catch_unwind(AssertUnwindSafe(move || drop(r))).is_err());
    assert!(flag.woken());
    assert!(s.is_closed());
    assert!(matches!(
        poll(&mut send, &waker),
        Poll::Ready(Err(SendError(_)))
    ));
}

/// The last receiver is counted out, and the channel closed, before the waker that its stream's
/// listener holds is dropped. A waker whose `Drop` panics then leaves a closed channel with no
/// receiver counted, which a sender finds closed, rather than one it can fill for nobody. The
/// listener goes before the channel is closed, too, so the close does not wake the task that
/// polled the stream, for a receiver that is gone.
#[test]
#[timeout(10000)]
fn a_waker_panicking_as_the_last_receiver_drops_it_leaves_the_channel_closed() {
    let (s, mut r) = bounded::<u8>(cap(1));
    let woken = Arc::new(AtomicBool::new(false));
    let waker = Waker::from(Arc::new(PanicOnLastDrop(woken.clone())));
    assert_eq!(poll_next(&mut r, &waker), Poll::Pending);
    // The stream's listener holds the only clone of the waker left.
    drop(waker);

    assert!(catch_unwind(AssertUnwindSafe(move || drop(r))).is_err());

    assert_eq!(s.receiver_count(), 0);
    assert!(s.is_closed());
    assert_eq!(s.try_send(1), Err(TrySendError::Closed(1)));
    assert!(!woken.load(Ordering::SeqCst));
}

/// A close notifies every event, even where a waker on one of them panics: a stream waiting beside
/// a `Recv` whose waker panics is woken all the same, and finds the channel closed.
#[test]
#[timeout(10000)]
fn a_waker_panicking_on_a_close_does_not_keep_a_stream_from_being_woken() {
    let (s, r) = unbounded::<u8>();
    let mut stream = r.clone();
    let mut recv = r.recv();
    assert!(poll(&mut recv, &Waker::from(Arc::new(PanicOnWake))).is_pending());
    let (flag, waker) = Flag::new();
    assert_eq!(poll_next(&mut stream, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(|| s.close())).is_err());

    assert!(flag.woken());
    assert_eq!(poll_next(&mut stream, &waker), Poll::Ready(None));
    assert!(s.is_closed());
}

/// A send notifies both the `Recv`s and the streams waiting, even where a waker of the former
/// panics: the stream is woken all the same, and gets the message.
#[test]
#[timeout(10000)]
fn a_waker_panicking_on_a_send_does_not_keep_a_stream_from_being_woken() {
    let (s, r) = unbounded();
    let mut stream = r.clone();
    let mut recv = r.recv();
    assert!(poll(&mut recv, &Waker::from(Arc::new(PanicOnWake))).is_pending());
    let (flag, waker) = Flag::new();
    assert_eq!(poll_next(&mut stream, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(|| s.try_send(1))).is_err());

    assert!(flag.woken());
    assert_eq!(poll_next(&mut stream, &waker), Poll::Ready(Some(1)));
}

/// Wakers that panic on every wake, on two events: the second panics as the first unwinds, and is
/// dropped there rather than aborting the process. The first panic reaches the caller, and the
/// stream behind the two panicking wakers is woken all the same.
#[test]
#[timeout(10000)]
fn wakers_panicking_on_two_events_do_not_abort_a_close() {
    let (s, r) = unbounded::<u8>();
    let mut panicking_stream = r.clone();
    let mut stream = r.clone();
    let mut recv = r.recv();
    let always = Waker::from(Arc::new(AlwaysPanicOnWake));
    assert!(poll(&mut recv, &always).is_pending());
    assert_eq!(poll_next(&mut panicking_stream, &always), Poll::Pending);
    let (flag, waker) = Flag::new();
    assert_eq!(poll_next(&mut stream, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(|| s.close())).is_err());

    assert!(flag.woken());
    assert_eq!(poll_next(&mut stream, &waker), Poll::Ready(None));
}

/// A waker that panics, on one event, as a close's earlier panic unwinds, with a payload whose own
/// `Drop` panics: the payload is forgotten rather than dropped there, which would abort the
/// process, and the stream behind it is woken all the same.
// Long enough for Miri, which takes seconds over each of the panics this raises.
#[test]
#[timeout(60000)]
fn a_panic_payload_that_panics_as_it_drops_does_not_abort_a_close() {
    let (s, r) = unbounded::<u8>();
    let mut payload_stream = r.clone();
    let mut stream = r.clone();
    let mut recv = r.recv();
    assert!(poll(&mut recv, &Waker::from(Arc::new(PanicOnWake))).is_pending());
    let payload_waker = Waker::from(Arc::new(PanicWithPayloadOnWake));
    assert_eq!(
        poll_next(&mut payload_stream, &payload_waker),
        Poll::Pending
    );
    let (flag, waker) = Flag::new();
    assert_eq!(poll_next(&mut stream, &waker), Poll::Pending);

    assert!(catch_unwind(AssertUnwindSafe(|| s.close())).is_err());

    assert!(flag.woken());
    assert_eq!(poll_next(&mut stream, &waker), Poll::Ready(None));
}

/// A close wakes every `Recv` waiting, even the ones after one whose waker panics.
#[test]
#[timeout(10000)]
fn a_waker_panicking_on_a_close_does_not_keep_the_recvs_after_it_from_being_woken() {
    let (s, r) = unbounded::<u8>();
    let mut first = r.recv();
    let mut second = r.recv();
    assert!(poll(&mut first, &Waker::from(Arc::new(PanicOnWake))).is_pending());
    let (flag, waker) = Flag::new();
    assert!(poll(&mut second, &waker).is_pending());

    assert!(catch_unwind(AssertUnwindSafe(|| s.close())).is_err());

    assert!(flag.woken());
    assert_eq!(poll(&mut second, &waker), Poll::Ready(Err(RecvError)));
}

/// The message of a `Send` is gone once it completed, so polling it again is not something it can
/// do: it panics, as its documentation says.
#[test]
#[timeout(10000)]
fn polling_a_send_again_after_it_completed_panics() {
    let (s, _r) = unbounded();

    let mut send = s.send(1);
    assert_eq!(poll(&mut send, Waker::noop()), Poll::Ready(Ok(())));
    assert!(catch_unwind(AssertUnwindSafe(|| poll(&mut send, Waker::noop()))).is_err());
}

/// None of the messages here is `Debug`, which is as it may be: the ends and the futures print
/// without their messages, and name what they are.
#[test]
#[timeout(10000)]
fn the_ends_and_the_futures_format_without_their_messages_being_debug() {
    let (s, r) = bounded::<NoDebug>(cap(1));
    assert!(format!("{s:?}").starts_with("Sender {"));
    assert!(format!("{r:?}").starts_with("Receiver {"));
    assert!(format!("{s:#?}").starts_with("Sender {"));
    assert!(format!("{r:#?}").starts_with("Receiver {"));

    let mut send = s.send(NoDebug);
    assert!(format!("{send:?}").starts_with("Send {"));
    assert!(format!("{send:?}").contains("waiting: false"));
    assert!(poll(&mut send, Waker::noop()).is_ready());

    let mut recv = r.recv();
    assert!(format!("{recv:?}").starts_with("Recv {"));
    assert!(format!("{recv:?}").contains("waiting: false"));
    assert!(poll(&mut recv, Waker::noop()).is_ready());

    // Waiting, they say so.
    let mut recv = r.recv();
    assert!(poll(&mut recv, Waker::noop()).is_pending());
    assert!(format!("{recv:?}").contains("waiting: true"));
    let mut send = s.send(NoDebug);
    assert!(poll(&mut send, Waker::noop()).is_ready());
    assert!(poll(&mut recv, Waker::noop()).is_ready());

    s.try_send(NoDebug).unwrap();
    let mut send = s.send(NoDebug);
    assert!(poll(&mut send, Waker::noop()).is_pending());
    assert!(format!("{send:?}").contains("waiting: true"));
}

#[test]
fn the_ends_show_the_state_of_the_channel_when_formatted() {
    let (s, r) = bounded(cap(3));
    let r2 = r.clone();
    s.try_send(1).unwrap();

    let sender = "Sender { len: 1, capacity: Some(3), senders: 1, receivers: 2, closed: false }";
    let receiver =
        "Receiver { len: 1, capacity: Some(3), senders: 1, receivers: 2, closed: false }";
    assert_eq!(format!("{s:?}"), sender);
    assert_eq!(format!("{r:?}"), receiver);
    assert_eq!(format!("{r2:?}"), receiver);

    drop(r2);
    s.close();
    assert_eq!(
        format!("{s:?}"),
        "Sender { len: 1, capacity: Some(3), senders: 1, receivers: 1, closed: true }"
    );

    let (s, _r) = unbounded::<u8>();
    assert_eq!(
        format!("{s:?}"),
        "Sender { len: 0, capacity: None, senders: 1, receivers: 1, closed: false }"
    );
}

#[test]
fn the_errors_display_what_went_wrong() {
    assert_eq!(SendError(1).to_string(), "the channel is closed");
    assert_eq!(TrySendError::Full(1).to_string(), "the channel is full");
    assert_eq!(TrySendError::Closed(1).to_string(), "the channel is closed");
    assert_eq!(
        RecvError.to_string(),
        "the channel is closed and has no message left"
    );
    assert_eq!(
        TryRecvError::Empty.to_string(),
        "the channel has no message"
    );
    assert_eq!(
        TryRecvError::Closed.to_string(),
        "the channel is closed and has no message left"
    );

    // The messages are left out of what is printed of the errors that hold one, which can then do
    // without `Debug` and `Display` of their own.
    assert_eq!(format!("{:?}", SendError(NoDebug)), "SendError(..)");
    assert_eq!(format!("{:?}", TrySendError::Full(NoDebug)), "Full(..)");
    assert_eq!(format!("{:?}", TrySendError::Closed(NoDebug)), "Closed(..)");
    assert_eq!(SendError(NoDebug).to_string(), "the channel is closed");
    assert_eq!(
        TrySendError::Full(NoDebug).to_string(),
        "the channel is full"
    );
    assert_eq!(format!("{RecvError:?}"), "RecvError");
    assert_eq!(format!("{:?}", TryRecvError::Empty), "Empty");
    assert_eq!(format!("{:?}", TryRecvError::Closed), "Closed");

    fn assert_error<E>()
    where
        E: std::error::Error,
    {
    }

    assert_error::<SendError<NoDebug>>();
    assert_error::<TrySendError<NoDebug>>();
    assert_error::<RecvError>();
    assert_error::<TryRecvError>();
}

#[test]
fn the_errors_hand_the_message_back_and_say_what_they_are() {
    assert_eq!(SendError(7).into_inner(), 7);

    let full = TrySendError::Full(8);
    assert!(full.is_full());
    assert!(!full.is_closed());
    assert_eq!(full.into_inner(), 8);

    let closed = TrySendError::Closed(9);
    assert!(closed.is_closed());
    assert!(!closed.is_full());
    assert_eq!(closed.into_inner(), 9);

    assert!(TryRecvError::Empty.is_empty());
    assert!(!TryRecvError::Empty.is_closed());
    assert!(TryRecvError::Closed.is_closed());
    assert!(!TryRecvError::Closed.is_empty());

    // They are `Copy`, and equal where they say the same of the same message.
    let err = TrySendError::Full(1);
    let copy = err;
    assert_eq!(err, copy);
    assert_ne!(TrySendError::Full(1), TrySendError::Closed(1));
    assert_ne!(TrySendError::Full(1), TrySendError::Full(2));
    assert_ne!(SendError(1), SendError(2));
    assert_ne!(TryRecvError::Empty, TryRecvError::Closed);
}

/// A message that holds a sender of the channel it is in, so that the two hold each other.
struct Looped {
    _sender: Sender<Looped>,
    _token: Arc<()>,
}

/// A message that says what it sees of its channel as it is dropped: whether it is closed, and how
/// many receivers it has.
struct Observer {
    sender: Sender<Observer>,
    seen: Arc<Mutex<Vec<(bool, usize)>>>,
}

impl Drop for Observer {
    fn drop(&mut self) {
        let seen = (self.sender.is_closed(), self.sender.receiver_count());
        self.seen.lock().unwrap().push(seen);
    }
}

/// A message that adds one to a count as it is dropped.
struct Counted(Arc<AtomicUsize>);

impl Drop for Counted {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// The count of the drops of the messages it makes.
#[derive(Default)]
struct Drops(Arc<AtomicUsize>);

impl Drops {
    /// A message that is counted as it is dropped.
    fn message(&self) -> Counted {
        Counted(self.0.clone())
    }

    /// How many of the messages made have been dropped.
    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

/// A message whose drop panics, if it is told to, and unless the thread is panicking already: a
/// second panic would abort the whole test run, where the first only fails the test.
struct PanicOnDrop(bool);

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        if self.0 && !thread::panicking() {
            panic!("a message whose drop panics");
        }
    }
}

/// A message with nothing to it, not even `Debug`.
struct NoDebug;

/// The ends of a channel that can close it.
#[derive(Clone, Copy, Debug)]
enum Closer {
    Sender,
    Receiver,
}

impl Closer {
    /// Each of them, for a test to try each.
    const BOTH: [Closer; 2] = [Closer::Sender, Closer::Receiver];

    /// Closes the channel from this end, and tells whether that call closed it.
    fn close<T>(self, sender: &Sender<T>, receiver: &Receiver<T>) -> bool {
        match self {
            Closer::Sender => sender.close(),
            Closer::Receiver => receiver.close(),
        }
    }
}

/// How a thread receives.
#[derive(Clone, Copy)]
enum Style {
    /// With `recv`, waiting in `block_on`.
    Recv,
    /// With `try_recv`, trying again after a `yield_now` while there is nothing to receive.
    TryRecv,
    /// As a stream, waiting in `block_on`.
    Stream,
    /// With `recv`, but giving the wait up over and over and starting it again, as a timeout or a
    /// `select` that goes the other way does: each wait that is given up is a `Recv` dropped, which
    /// may be one that was notified and has not run.
    Cancelling,
}

impl Style {
    /// Every style.
    const ALL: [Style; 4] = [
        Style::Recv,
        Style::TryRecv,
        Style::Stream,
        Style::Cancelling,
    ];

    /// The styles that wait where there is nothing to receive, and never try again on their own.
    const WAITING: [Style; 2] = [Style::Recv, Style::Stream];
}

/// Runs sender threads and receiver threads on a channel, and checks that every message came out
/// of it exactly once.
///
/// Each sender thread sends its own run of numbers, in order, by waiting for room in `block_on` and
/// by trying again after a `yield_now`, in turn. The receiver threads share one receiver by
/// reference, and receive in each of the `styles`, twice over. The channel is closed by its last
/// sender being dropped, which ends every receiver once the messages are gone. What the receivers
/// got, sorted, must be what was sent, and as seen by any one receiver, the numbers of a sender
/// must come in the order that sender sent them.
fn assert_every_message_is_received_once(
    channel: (Sender<usize>, Receiver<usize>),
    styles: &[Style],
) {
    let (senders, copies, messages) = if cfg!(miri) { (2, 1, 20) } else { (4, 2, 1000) };
    let (sender, receiver) = channel;

    let received: Vec<Vec<usize>> = thread::scope(|scope| {
        for id in 0..senders {
            let sender = sender.clone();
            scope.spawn(move || send_run(&sender, id * messages..(id + 1) * messages));
        }
        // The channel is open for as long as one of the clones is, and no longer.
        drop(sender);

        let receiver = &receiver;
        let receivers: Vec<_> = (0..copies)
            .flat_map(|_| styles)
            .map(|&style| scope.spawn(move || receive_all(receiver, style)))
            .collect();

        receivers
            .into_iter()
            .map(|receiver| receiver.join().unwrap())
            .collect()
    });

    for got in &received {
        let mut last = vec![None; senders];
        for &msg in got {
            let sender = msg / messages;
            assert!(
                last[sender].is_none_or(|last| last < msg),
                "{msg} came out of order"
            );
            last[sender] = Some(msg);
        }
    }

    let mut all: Vec<usize> = received.into_iter().flatten().collect();
    all.sort_unstable();
    assert_eq!(all, (0..senders * messages).collect::<Vec<_>>());
}

/// Sends the `run` of numbers in order, one by waiting for room and the next by trying again until
/// there is some.
fn send_run(sender: &Sender<usize>, run: std::ops::Range<usize>) {
    for msg in run {
        if msg % 2 == 0 {
            block_on(sender.send(msg)).unwrap();
            continue;
        }

        while let Err(err) = sender.try_send(msg) {
            assert!(err.is_full(), "the channel closed under a sender");
            thread::yield_now();
        }
    }
}

/// Receives in the `style` until the channel is closed and has nothing left, and returns what it
/// got, in the order it got it.
fn receive_all(receiver: &Receiver<usize>, style: Style) -> Vec<usize> {
    let mut got = Vec::new();

    match style {
        Style::Recv => {
            while let Ok(msg) = block_on(receiver.recv()) {
                got.push(msg);
            }
        }
        Style::TryRecv => loop {
            match receiver.try_recv() {
                Ok(msg) => got.push(msg),
                Err(TryRecvError::Empty) => thread::yield_now(),
                Err(TryRecvError::Closed) => break,
            }
        },
        Style::Stream => {
            let mut stream = receiver.clone();

            block_on(async {
                while let Some(msg) = stream.next().await {
                    got.push(msg);
                }
            });
        }
        Style::Cancelling => loop {
            // `or` prefers its first future, so the `recv` wins whenever it is ready, and a wait of
            // a single turn of the executor is all it gets before it is dropped.
            let received = block_on(or(async { Some(receiver.recv().await) }, async {
                yield_now().await;
                None
            }));

            match received {
                Some(Ok(msg)) => got.push(msg),
                Some(Err(RecvError)) => break,
                None => {}
            }
        },
    }

    got
}

/// Runs `waiting_side` on a thread of its own, with the receiver of requests and the sender of
/// responses, and takes the other side itself: it sends a request, spins until the response comes,
/// and sends the next request as soon as it does. It ends by dropping its sender of requests, which
/// ends the waiting side.
fn ping_pong<F>(waiting_side: F)
where
    F: FnOnce(Receiver<usize>, Sender<usize>) + std::marker::Send,
{
    let rounds = if cfg!(miri) { 20 } else { 5000 };
    let (request_s, request_r) = unbounded();
    let (response_s, response_r) = unbounded();

    thread::scope(|scope| {
        scope.spawn(move || waiting_side(request_r, response_s));

        for n in 0..rounds {
            request_s.try_send(n).unwrap();
            assert_eq!(spin_recv(&response_r), n);
        }
        drop(request_s);
    });
}

/// Runs `sending_side` on a thread of its own, with the sender of a channel that holds one message
/// and the number of messages to send, which are the numbers from zero on, and takes them itself:
/// it spins until each is there and takes it at once, so that room is made as the sender goes to
/// wait for it.
fn spin_receive<F>(sending_side: F)
where
    F: FnOnce(Sender<usize>, usize) + std::marker::Send,
{
    let rounds = if cfg!(miri) { 20 } else { 5000 };
    let (sender, receiver) = bounded(cap(1));

    thread::scope(|scope| {
        scope.spawn(move || sending_side(sender, rounds));

        for n in 0..rounds {
            assert_eq!(spin_recv(&receiver), n);
        }
    });
}

/// Receives a message by trying again, after a couple of `yield_now`s, until there is one. The
/// pause is long enough for the other side to be in the middle of what it does when the message
/// comes, which is what the tests that spin are for.
fn spin_recv(receiver: &Receiver<usize>) -> usize {
    loop {
        match receiver.try_recv() {
            Ok(msg) => return msg,
            Err(err) => {
                assert!(err.is_empty(), "the channel closed under a receiver");
                thread::yield_now();
                thread::yield_now();
            }
        }
    }
}

/// What the two ends of a channel say of it: how many messages it holds, whether it is empty and
/// whether it is full. The ends agree, and this checks that they do.
fn state<T>(sender: &Sender<T>, receiver: &Receiver<T>) -> (usize, bool, bool) {
    let state = (sender.len(), sender.is_empty(), sender.is_full());
    assert_eq!(
        state,
        (receiver.len(), receiver.is_empty(), receiver.is_full())
    );

    state
}

fn cap(cap: usize) -> NonZeroUsize {
    NonZeroUsize::new(cap).unwrap()
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

/// A waker whose `wake` panics, for a wake-up that goes wrong, unless the thread is panicking
/// already: a second panic would abort the whole test run, where the first only fails the test.
struct PanicOnWake;

impl Wake for PanicOnWake {
    fn wake(self: Arc<Self>) {
        if !thread::panicking() {
            panic!("a waker that panics");
        }
    }
}

/// A waker that panics on every wake, a panic already unwinding or not.
struct AlwaysPanicOnWake;

impl Wake for AlwaysPanicOnWake {
    fn wake(self: Arc<Self>) {
        panic!("a waker that always panics");
    }
}

/// A waker that panics on every wake with a payload that panics as it is dropped.
struct PanicWithPayloadOnWake;

impl Wake for PanicWithPayloadOnWake {
    fn wake(self: Arc<Self>) {
        panic::panic_any(PayloadPanickingOnDrop);
    }
}

/// A panic payload that panics as it is dropped, a panic already unwinding or not.
struct PayloadPanickingOnDrop;

impl Drop for PayloadPanickingOnDrop {
    fn drop(&mut self) {
        panic!("a panic payload that panics as it is dropped");
    }
}

/// A waker that panics as its last clone is dropped, and sets its flag if it is woken first.
struct PanicOnLastDrop(Arc<AtomicBool>);

impl Wake for PanicOnLastDrop {
    fn wake(self: Arc<Self>) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl Drop for PanicOnLastDrop {
    fn drop(&mut self) {
        if !thread::panicking() {
            panic!("a waker that panics as it goes");
        }
    }
}

/// Polls a future once with `waker`.
fn poll<F>(future: &mut F, waker: &Waker) -> Poll<F::Output>
where
    F: Future + Unpin,
{
    Pin::new(future).poll(&mut Context::from_waker(waker))
}

/// Polls a stream once with `waker`.
fn poll_next<S>(stream: &mut S, waker: &Waker) -> Poll<Option<S::Item>>
where
    S: Stream + Unpin,
{
    Pin::new(stream).poll_next(&mut Context::from_waker(waker))
}
