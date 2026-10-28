//! Tests of an [`Event`] and its listeners.
//!
//! Most of these drive listeners by hand, polling them with a waker that goes nowhere or with one
//! that counts its wakes, so that each says exactly which listener a notification reached and
//! which task it woke. A few poll with a waker built on a vtable of their own, which counts the
//! clones of the waker as well, or notifies the event from inside a clone: a waker built on
//! [`Wake`] is cloned by the standard library, where no test can see it. That vtable is the one
//! piece of `unsafe` code here: the library's own is in the `lock` module, in the pin projection of
//! a `Timeout`'s poll and, on Windows, in the poller's calls into Winsock and the `net` module's
//! check of a connect under way. They come in the order of what they pin down: that a listener is
//! queued as it is taken, that nothing is kept for one taken later, the order listeners are
//! notified in, what each kind of notification counts, what a notified listener does when polled or
//! dropped, what outlives what, which wakers are kept, that a waker may come back into the event it
//! was woken or dropped by, and that one that panics as it is woken leaves the others woken all the
//! same.
//!
//! The last five drive one event from many threads at once — through listeners taken just as
//! another thread changes the condition they are for and notifies, through a listener dropped just
//! as another thread notifies, through listeners polled for the first time while one notification
//! for all of them is sent, and through listeners that give up the moment a notification reaches
//! them — and check that no notification is lost on the way, and that one that finds nobody to
//! reach comes after whatever left nobody there. They are shrunk under Miri, which runs them far
//! more slowly, and which also lets an atomic load see an older value than the hardware would, as
//! the memory model allows: under it, the first of them fails without the fence `listen` makes or
//! the one a notification makes, and the second without the ordering of the word a notification
//! looks at without the event's lock, which a run on the hardware alone can hide. Driving an event
//! from many threads through a lock built on it, and through wakers that come back into it, is
//! tested with the locks, in the `lock` tests.

use std::{
    future::Future,
    panic::{self, AssertUnwindSafe},
    pin::Pin,
    ptr,
    sync::{
        Arc, Barrier, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Wake, Waker},
    thread,
};

use futures::executor::block_on;
use ntest::timeout;

use crate::{Event, EventListener};

/// A notification sent once a listener is taken reaches it, although nothing has polled it yet:
/// what the listen-then-check pattern rests on.
#[test]
fn a_listener_is_queued_before_it_is_first_polled() {
    let event = Event::new();
    let mut listener = event.listen();

    assert_eq!(event.notify(1), 1);

    assert!(ready(&mut listener));
}

/// A notification reaches only the listeners there when it is sent: none is kept for a listener
/// taken after it.
#[test]
fn a_notification_is_not_kept_for_a_later_listener() {
    let event = Event::new();

    assert_eq!(event.notify(1), 0);
    assert_eq!(event.notify_additional(1), 0);
    let mut listener = event.listen();

    assert!(!ready(&mut listener));
    assert_eq!(event.notify(1), 1);
    assert!(ready(&mut listener));
}

#[test]
fn listeners_are_notified_oldest_first() {
    let event = Event::new();
    let mut listeners = [event.listen(), event.listen(), event.listen()];
    let counters = poll_all(&mut listeners);

    for notified in 1..=listeners.len() {
        assert_eq!(event.notify_additional(1), 1);
        let wakes: Vec<_> = counters.iter().map(|counter| counter.wakes()).collect();
        let expected: Vec<_> = (0..listeners.len())
            .map(|i| usize::from(i < notified))
            .collect();
        assert_eq!(wakes, expected);
    }
}

/// `notify(n)` makes sure `n` listeners are notified, counting those notified already that have
/// not yet completed, and hands back how many it notified itself.
#[test]
fn notify_counts_the_listeners_notified_already() {
    let event = Event::new();
    let [mut first, mut second, mut third] = [event.listen(), event.listen(), event.listen()];

    assert_eq!(event.notify(0), 0);
    assert_eq!(event.notify(1), 1);
    assert_eq!(event.notify(1), 0);
    assert_eq!(event.notify(2), 1);
    assert!(!ready(&mut third));
    // Completed, the first counts no more: of the two, only the second is notified still.
    assert!(ready(&mut first));
    assert_eq!(event.notify(2), 1);

    assert!(ready(&mut second));
    assert!(ready(&mut third));
}

#[test]
fn notify_all_notifies_every_listener_there_is() {
    let event = Event::new();
    let mut listeners: Vec<_> = (0..5).map(|_| event.listen()).collect();
    assert_eq!(event.notify(1), 1);

    // The one notified already counts towards `usize::MAX` as well.
    assert_eq!(event.notify(usize::MAX), 4);

    assert!(listeners.iter_mut().all(ready));
    assert_eq!(event.notify(usize::MAX), 0);
}

/// `notify_additional(n)` notifies `n` more listeners, however many are notified already.
#[test]
fn notify_additional_notifies_more_whatever_is_notified_already() {
    let event = Event::new();
    let [mut first, mut second, mut third] = [event.listen(), event.listen(), event.listen()];

    assert_eq!(event.notify(1), 1);
    assert_eq!(event.notify_additional(1), 1);
    assert!(!ready(&mut third));
    // Two are notified, so a counting notification for two has nothing to do...
    assert_eq!(event.notify(2), 0);
    // ...while an additional one goes to every listener left, and no further.
    assert_eq!(event.notify_additional(5), 1);

    assert!(ready(&mut first));
    assert!(ready(&mut second));
    assert!(ready(&mut third));
}

/// A notified listener completes on its next poll and leaves the queue, where it counts no more.
#[test]
fn a_notified_listener_completes_and_leaves_the_queue() {
    let event = Event::new();
    let [mut first, mut second] = [event.listen(), event.listen()];
    assert_eq!(event.notify(1), 1);

    assert!(ready(&mut first));

    assert_eq!(format!("{event:?}"), "Event { listeners: 1, notified: 0 }");
    assert_eq!(event.notify(1), 1);
    assert!(ready(&mut second));
}

#[test]
fn a_notification_wakes_the_task_that_polled_the_listener() {
    let event = Event::new();
    let mut listener = event.listen();
    let (counter, waker) = counting_waker();

    assert!(poll(&mut listener, &waker).is_pending());
    assert_eq!(counter.wakes(), 0);
    event.notify(1);

    assert_eq!(counter.wakes(), 1);
    assert!(poll(&mut listener, &waker).is_ready());
    assert_eq!(counter.wakes(), 1);
}

#[test]
fn a_listener_dropped_while_notified_passes_the_notification_on() {
    let event = Event::new();
    let [first, mut second] = [event.listen(), event.listen()];
    let (counter, waker) = counting_waker();
    assert!(poll(&mut second, &waker).is_pending());
    assert_eq!(event.notify(1), 1);

    drop(first);

    assert_eq!(counter.wakes(), 1);
    assert!(ready(&mut second));
}

/// A notification from `notify` is passed on as `notify(1)` would send it: to nobody while
/// another listener is notified still.
#[test]
fn a_counting_notification_is_passed_on_only_where_nobody_else_is_notified() {
    let event = Event::new();
    let [first, mut second, mut third] = [event.listen(), event.listen(), event.listen()];
    let (counter, waker) = counting_waker();
    assert!(poll(&mut third, &waker).is_pending());
    assert_eq!(event.notify(2), 2);

    drop(first);

    assert_eq!(counter.wakes(), 0);
    assert!(!ready(&mut third));
    assert!(ready(&mut second));
}

/// A notification from `notify_additional` is passed on as `notify_additional(1)` would send it:
/// to the next listener, whatever the others are.
#[test]
fn an_additional_notification_is_passed_on_whatever_else_is_notified() {
    let event = Event::new();
    let [mut first, second, mut third] = [event.listen(), event.listen(), event.listen()];
    let (counter, waker) = counting_waker();
    assert!(poll(&mut third, &waker).is_pending());
    assert_eq!(event.notify(1), 1);
    assert_eq!(event.notify_additional(1), 1);

    drop(second);

    assert_eq!(counter.wakes(), 1);
    assert!(ready(&mut first));
    assert!(ready(&mut third));
}

/// A notification passed on with nobody left to take it is gone, as one sent to nobody is.
#[test]
fn a_notification_passed_on_to_nobody_is_lost() {
    let event = Event::new();
    let first = event.listen();
    assert_eq!(event.notify(1), 1);

    drop(first);
    let mut second = event.listen();

    assert!(!ready(&mut second));
    assert_eq!(event.notify(1), 1);
    assert!(ready(&mut second));
}

/// A listener dropped before a notification reached it leaves the queue, and the listeners on
/// either side of it are notified as though it had never been there.
#[test]
fn a_listener_dropped_before_being_notified_changes_nothing_else() {
    let event = Event::new();
    let [mut first, second, mut third] = [event.listen(), event.listen(), event.listen()];
    let counters = poll_all([&mut first, &mut third]);

    drop(second);

    assert_eq!(counters.iter().map(|c| c.wakes()).sum::<usize>(), 0);
    assert_eq!(event.notify(1), 1);
    assert_eq!(counters[0].wakes(), 1);
    assert_eq!(event.notify_additional(1), 1);
    assert_eq!(counters[1].wakes(), 1);
    assert_eq!(event.notify_additional(1), 0);
    assert!(ready(&mut first));
    assert!(ready(&mut third));
}

/// The oldest listener not notified yet, where a notification starts, may go: the next one
/// notified is the one behind it.
#[test]
fn dropping_the_oldest_listener_waiting_moves_notifications_on_to_the_next() {
    let event = Event::new();
    let [mut first, second, mut third] = [event.listen(), event.listen(), event.listen()];
    assert_eq!(event.notify(1), 1);

    drop(second);

    assert_eq!(event.notify_additional(1), 1);
    assert!(ready(&mut first));
    assert!(ready(&mut third));
    // And one taken after all that is at the back of the queue, behind nobody.
    let mut fourth = event.listen();
    assert_eq!(event.notify(1), 1);
    assert!(ready(&mut fourth));
}

#[test]
fn a_listener_outlives_its_event() {
    let event = Event::new();
    let mut listener = event.listen();

    drop(event);

    assert!(!ready(&mut listener));
    assert_eq!(format!("{listener:?}"), "EventListener { notified: false }");
}

/// Dropping an event notifies nobody: a listener notified by then still completes, and one that
/// was not never does.
#[test]
fn dropping_the_event_notifies_nobody() {
    let event = Event::new();
    let mut listeners = [event.listen(), event.listen()];
    let counters = poll_all(&mut listeners);
    assert_eq!(event.notify(1), 1);

    drop(event);

    assert_eq!(counters[0].wakes(), 1);
    assert_eq!(counters[1].wakes(), 0);
    let [first, second] = &mut listeners;
    assert!(ready(first));
    assert!(!ready(second));
    assert_eq!(counters[1].wakes(), 0);
}

/// A listener notified before its event went, and dropped after, still passes its notification on:
/// what the listeners share outlives the event.
#[test]
fn a_listener_dropped_after_its_event_passes_its_notification_on() {
    let event = Event::new();
    let [first, mut second] = [event.listen(), event.listen()];
    let (counter, waker) = counting_waker();
    assert!(poll(&mut second, &waker).is_pending());
    assert_eq!(event.notify(1), 1);
    drop(event);

    drop(first);

    assert_eq!(counter.wakes(), 1);
    assert!(ready(&mut second));
}

/// A listener polled again once it completed completes again, at once, and does not go back
/// into the queue.
#[test]
fn a_listener_polled_after_completing_is_ready_again() {
    let event = Event::new();
    let mut listener = event.listen();
    event.notify(1);

    assert!(ready(&mut listener));
    assert!(ready(&mut listener));
    assert!(ready(&mut listener));

    assert_eq!(format!("{listener:?}"), "EventListener { notified: true }");
    assert_eq!(event.notify(usize::MAX), 0);
    let mut later = event.listen();
    assert_eq!(event.notify(1), 1);
    assert!(ready(&mut later));
}

/// A listener keeps the waker of its latest poll alone: the one before it is let go of, and is
/// not woken.
#[test]
fn only_the_waker_of_the_latest_poll_is_kept() {
    let event = Event::new();
    let mut listener = event.listen();
    let (first, first_waker) = counting_waker();
    let (second, second_waker) = counting_waker();

    assert!(poll(&mut listener, &first_waker).is_pending());
    // The test's own counter and waker, and the listener's clone of the waker.
    assert_eq!(Arc::strong_count(&first), 3);
    assert!(poll(&mut listener, &second_waker).is_pending());
    assert_eq!(Arc::strong_count(&first), 2);
    assert_eq!(Arc::strong_count(&second), 3);
    event.notify(1);

    assert_eq!(first.wakes(), 0);
    assert_eq!(second.wakes(), 1);
}

/// A poll clones its waker only to store it for a task other than the one of the waker stored:
/// polled again for the same task, with the same waker or another that wakes that task, it clones
/// nothing. The waker a clone replaces is dropped, and the one a notification takes is woken, which
/// lets go of it too.
#[test]
fn a_waker_is_cloned_only_to_be_stored_for_another_task() {
    static FIRST: RawTask = RawTask::new(None);
    static SECOND: RawTask = RawTask::new(None);
    let event = Event::new();
    let mut listener = event.listen();
    let first = raw_waker(&FIRST);
    let first_again = raw_waker(&FIRST);
    let second = raw_waker(&SECOND);
    assert!(!first.will_wake(&second));

    assert!(poll(&mut listener, &first).is_pending());
    assert_eq!(FIRST.counts(), [1, 0, 0]);
    assert!(poll(&mut listener, &first).is_pending());
    assert!(poll(&mut listener, &first_again).is_pending());
    assert_eq!(FIRST.counts(), [1, 0, 0]);
    assert!(poll(&mut listener, &second).is_pending());
    assert_eq!(FIRST.counts(), [1, 1, 0]);
    assert_eq!(SECOND.counts(), [1, 0, 0]);
    assert_eq!(event.notify(1), 1);
    assert_eq!(SECOND.counts(), [1, 0, 1]);
    assert!(poll(&mut listener, &second).is_ready());
    drop(listener);
    assert_eq!(SECOND.counts(), [1, 0, 1]);

    // Every waker made or cloned is let go of once: the three the test made, dropped here, and
    // the two clones, one dropped and one woken.
    drop([first, first_again, second]);
    assert_eq!(FIRST.counts(), [1, 3, 0]);
    assert_eq!(SECOND.counts(), [1, 1, 1]);
}

/// A notification that lands between the two looks a first poll takes at the list — while the
/// poll clones its waker, with the lock let go of — is one the second look finds: the poll
/// completes, and stores nothing.
#[test]
#[timeout(15000)]
fn a_notification_landing_while_a_first_poll_clones_its_waker_completes_it() {
    static EVENT: Event = Event::new();
    static NOTIFYING: RawTask = RawTask::new(Some(&EVENT));
    let mut listener = EVENT.listen();
    let waker = raw_waker(&NOTIFYING);

    assert!(poll(&mut listener, &waker).is_ready());

    // Cloned once, and that clone dropped unused; nothing was stored, so nothing was woken.
    assert_eq!(NOTIFYING.counts(), [1, 1, 0]);
    assert_eq!(format!("{EVENT:?}"), "Event { listeners: 0, notified: 0 }");
    assert!(poll(&mut listener, &waker).is_ready());
    assert_eq!(NOTIFYING.counts(), [1, 1, 0]);
}

/// A notification that lands while a later poll, for another task, clones its waker takes the
/// waker stored by the poll before and wakes that task, which is woken for nothing; the poll
/// under way finds the notification on its second look, and completes.
#[test]
#[timeout(15000)]
fn a_notification_landing_while_a_later_poll_clones_its_waker_completes_it() {
    static EVENT: Event = Event::new();
    static NOTIFYING: RawTask = RawTask::new(Some(&EVENT));
    let mut listener = EVENT.listen();
    let (counter, before) = counting_waker();
    assert!(poll(&mut listener, &before).is_pending());
    let waker = raw_waker(&NOTIFYING);

    assert!(poll(&mut listener, &waker).is_ready());

    assert_eq!(counter.wakes(), 1);
    assert_eq!(NOTIFYING.counts(), [1, 1, 0]);
    assert_eq!(format!("{EVENT:?}"), "Event { listeners: 0, notified: 0 }");
}

/// A waker woken by a notification may listen, notify and drop listeners on the same event from
/// inside its wake, which it could not do were the event's lock still held.
#[test]
#[timeout(15000)]
fn a_waker_may_come_back_into_the_event_from_its_wake() {
    let event = Arc::new(Event::new());
    let mut first = event.listen();
    let second = event.listen();
    let mut third = event.listen();
    let comes_back = Arc::new(ComesBack {
        event: event.clone(),
        listener: Mutex::new(Some(second)),
        woken: AtomicBool::new(false),
    });
    assert!(poll(&mut first, &Waker::from(comes_back.clone())).is_pending());

    assert_eq!(event.notify(1), 1);

    assert!(comes_back.woken.load(Ordering::SeqCst));
    assert!(ready(&mut first));
    // The wake dropped the second listener before it was notified, and notified one more: the
    // third.
    assert!(ready(&mut third));
    assert_eq!(format!("{event:?}"), "Event { listeners: 0, notified: 0 }");
}

/// A waker woken by a notification that a dropped listener passes on may come back into the same
/// event from inside its wake, which it could not do were the event's lock still held.
#[test]
#[timeout(15000)]
fn a_waker_woken_by_a_passed_on_notification_may_come_back_into_the_event() {
    let event = Arc::new(Event::new());
    let first = event.listen();
    let mut second = event.listen();
    let mut third = event.listen();
    let comes_back = Arc::new(ComesBack {
        event: event.clone(),
        listener: Mutex::new(None),
        woken: AtomicBool::new(false),
    });
    assert!(poll(&mut second, &Waker::from(comes_back.clone())).is_pending());
    assert_eq!(event.notify(1), 1);

    drop(first);

    assert!(comes_back.woken.load(Ordering::SeqCst));
    assert!(ready(&mut second));
    // The wake notified one more: the third.
    assert!(ready(&mut third));
}

/// A waker the event lets go of as a later poll replaces it may drop a listener of the same event
/// as it goes, which it could not do were the event's lock still held.
#[test]
#[timeout(15000)]
fn a_waker_replaced_by_a_poll_is_dropped_clear_of_the_lock() {
    let event = Event::new();
    let mut listener = event.listen();
    let (counter, holder) = holding(event.listen());
    assert!(poll(&mut listener, &holder).is_pending());
    // The listener's clone is the last one left, and goes as the next poll replaces it.
    drop(holder);

    assert!(poll(&mut listener, Waker::noop()).is_pending());

    assert_eq!(counter.wakes(), 0);
    assert_eq!(format!("{event:?}"), "Event { listeners: 1, notified: 0 }");
}

/// A waker the event lets go of along with the listener it was stored for may drop another
/// listener of the same event as it goes: here a notified one, which passes its notification on
/// from there.
#[test]
#[timeout(15000)]
fn a_waker_dropped_with_its_listener_is_dropped_clear_of_the_lock() {
    let event = Event::new();
    let notified = event.listen();
    let mut listener = event.listen();
    let mut last = event.listen();
    let (counter, waker) = counting_waker();
    assert!(poll(&mut last, &waker).is_pending());
    let (holder_counter, holder) = holding(notified);
    assert!(poll(&mut listener, &holder).is_pending());
    drop(holder);
    assert_eq!(event.notify(1), 1);

    drop(listener);

    assert_eq!(holder_counter.wakes(), 0);
    assert_eq!(counter.wakes(), 1);
    assert!(ready(&mut last));
}

/// A waker the event wakes, and so lets go of, may drop a listener of the same event as it goes.
#[test]
#[timeout(15000)]
fn a_waker_woken_by_a_notification_is_dropped_clear_of_the_lock() {
    let event = Event::new();
    let mut listener = event.listen();
    let (counter, holder) = holding(event.listen());
    assert!(poll(&mut listener, &holder).is_pending());
    drop(holder);

    assert_eq!(event.notify(1), 1);

    assert_eq!(counter.wakes(), 1);
    assert!(ready(&mut listener));
    assert_eq!(format!("{event:?}"), "Event { listeners: 0, notified: 0 }");
}

/// A waker that panics as it is woken does not keep the tasks after it from being woken, by
/// either kind of notification. Each of their listeners is marked notified before any waker is
/// woken, so one left unwoken would wait for good, for a notification that came and went. The
/// panic reaches whoever notified once every waker has been woken.
#[test]
#[timeout(15000)]
fn a_waker_that_panics_does_not_keep_the_others_from_being_woken() {
    for additional in [false, true] {
        let event = Event::new();
        let mut first = event.listen();
        let mut second = event.listen();
        let mut third = event.listen();
        assert!(poll(&mut first, &Waker::from(Arc::new(PanicsOnWake))).is_pending());
        let counters = poll_all([&mut second, &mut third]);

        let notified = panic::catch_unwind(AssertUnwindSafe(|| {
            if additional {
                event.notify_additional(3)
            } else {
                event.notify(3)
            }
        }));

        assert!(
            notified.is_err(),
            "the panic of the first waker reaches the caller"
        );
        for counter in &counters {
            assert_eq!(counter.wakes(), 1, "additional: {additional}");
        }
        assert!(ready(&mut first));
        assert!(ready(&mut second));
        assert!(ready(&mut third));
    }
}

/// A waker whose panic carries a payload that panics as it is dropped does not keep the tasks
/// after it from being woken either: the event forgets that payload rather than drop it, as it is
/// the first panic that reaches the caller.
// Long enough for Miri, which takes seconds over each of the panics this raises.
#[test]
#[timeout(60000)]
fn a_panic_whose_payload_panics_as_it_drops_does_not_keep_the_others_from_being_woken() {
    let event = Event::new();
    let mut first = event.listen();
    let mut second = event.listen();
    let mut third = event.listen();
    assert!(poll(&mut first, &Waker::from(Arc::new(PanicsOnWake))).is_pending());
    let payload_waker = Waker::from(Arc::new(PanicsWithPayloadOnWake));
    assert!(poll(&mut second, &payload_waker).is_pending());
    let counters = poll_all([&mut third]);

    let notified = panic::catch_unwind(AssertUnwindSafe(|| event.notify(3)));

    assert!(
        notified.is_err(),
        "the panic of the first waker reaches the caller"
    );
    assert_eq!(counters[0].wakes(), 1);
    assert!(ready(&mut third));
}

/// An event and its listeners may be shared with, and sent to, any thread, and a listener may be
/// polled through a plain `&mut` and kept for as long as its owner likes.
#[test]
fn an_event_and_its_listeners_cross_threads() {
    fn sent_and_shared<T>()
    where
        T: Send + Sync,
    {
    }

    fn sent_shared_and_kept<T>()
    where
        T: Future<Output = ()> + Send + Sync + Unpin + 'static,
    {
    }

    sent_and_shared::<Event>();
    sent_shared_and_kept::<EventListener>();
}

/// Making an event allocates nothing, so one can be a `static`; the first `listen` or `notify`
/// allocates what its listeners are kept in.
#[test]
fn an_event_allocates_nothing_until_first_used() {
    static EVENT: Event = Event::new();
    assert_eq!(EVENT.slots(), None);
    drop(EVENT.listen());
    assert_eq!(EVENT.slots(), Some(1));

    let event = Event::default();
    assert_eq!(format!("{event:?}"), "Event { listeners: 0, notified: 0 }");
    assert_eq!(event.slots(), None);
    event.notify(1);
    assert_eq!(event.slots(), Some(0));
}

/// Once the queue has room for as many listeners as are alive at once, a listener costs no
/// allocation of its own: it takes the slot of one gone, whether that one was dropped or
/// completed.
#[test]
fn a_listener_takes_the_slot_of_one_gone() {
    let event = Event::new();
    drop([event.listen(), event.listen(), event.listen()]);
    assert_eq!(event.slots(), Some(3));

    for _ in 0..10 {
        let mut listeners = [event.listen(), event.listen(), event.listen()];
        event.notify(2);
        assert!(listeners.iter_mut().take(2).all(ready));
        let _next = event.listen();
        drop(listeners);
    }

    assert_eq!(event.slots(), Some(3));
}

/// Listeners taken on two threads just as a third changes the condition they are for and notifies
/// are reached by the notification, unless the check that follows each listener's taking sees the
/// change: the listen-then-check pattern, raced round after round.
///
/// The condition is changed and checked with `Relaxed` accesses, which order nothing on their
/// own, so that only what the event itself does keeps the notification and a check from both
/// missing the other side. A notification that did would leave a listening thread waiting for
/// good, and the test timed out. With two listeners, a notification can also come in between the
/// taking of one and that of the other, and then has to reach the first while the second's check
/// sees the change.
#[test]
#[timeout(15000)]
fn a_notification_racing_listeners_reaches_them_or_their_checks_see_the_change() {
    const LISTENING: usize = 2;
    let rounds = if cfg!(miri) { 30 } else { 10_000 };
    let event = Arc::new(Event::new());
    let changed_in = Arc::new(AtomicUsize::new(0));
    let start = Arc::new(Barrier::new(LISTENING + 1));

    let listening: Vec<_> = (0..LISTENING)
        .map(|_| {
            let event = event.clone();
            let changed_in = changed_in.clone();
            let start = start.clone();
            thread::spawn(move || {
                for round in 1..=rounds {
                    start.wait();
                    let listener = event.listen();
                    if changed_in.load(Ordering::Relaxed) < round {
                        block_on(listener);
                    }
                }
            })
        })
        .collect();

    for round in 1..=rounds {
        start.wait();
        changed_in.store(round, Ordering::Relaxed);
        event.notify(usize::MAX);
    }
    for thread in listening {
        thread.join().unwrap();
    }
}

/// A notification that finds the last listener dropped, and so reaches nobody, comes after that
/// drop and whatever came before it, as it did when every notification took the event's lock.
#[test]
#[timeout(15000)]
fn a_notification_finding_the_last_listener_dropped_comes_after_the_drop() {
    let rounds = if cfg!(miri) { 30 } else { 1_000 };

    for _ in 0..rounds {
        let event = Arc::new(Event::new());
        let listening = Arc::new(AtomicBool::new(false));
        let before_drop = Arc::new(AtomicBool::new(false));
        let dropper = thread::spawn({
            let event = event.clone();
            let listening = listening.clone();
            let before_drop = before_drop.clone();
            move || {
                let listener = event.listen();
                listening.store(true, Ordering::Release);
                // Ordered before the notification below by nothing but the drop that follows.
                before_drop.store(true, Ordering::Relaxed);
                drop(listener);
            }
        });

        while !listening.load(Ordering::Acquire) {
            thread::yield_now();
        }
        if event.notify(1) == 0 {
            assert!(before_drop.load(Ordering::Relaxed));
        }
        dropper.join().unwrap();
    }
}

/// Listeners polled for the first time on many threads, while one notification for all of them
/// is sent, are each reached by it.
#[test]
#[timeout(15000)]
fn one_notify_all_reaches_every_listener_on_many_threads() {
    let (threads, rounds) = if cfg!(miri) { (3, 5) } else { (16, 100) };
    let event = Arc::new(Event::new());

    for _ in 0..rounds {
        let barrier = Arc::new(Barrier::new(threads + 1));
        let listeners: Vec<_> = (0..threads)
            .map(|_| {
                let event = event.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    let listener = event.listen();
                    barrier.wait();
                    block_on(listener);
                })
            })
            .collect();
        barrier.wait();

        assert_eq!(event.notify(usize::MAX), threads);

        for listener in listeners {
            listener.join().unwrap();
        }
    }
}

/// A notification from `notify` travels down a queue of listeners each dropped by its own thread
/// as soon as the notification reaches it, and so passed on by each in turn, to reach the one
/// listener behind them all that takes it.
#[test]
#[timeout(15000)]
fn a_notification_is_passed_down_listeners_dropped_once_notified() {
    let rounds = if cfg!(miri) { 2 } else { 100 };

    for _ in 0..rounds {
        pass_down_listeners_dropped_once_notified(1, |event| event.notify(1));
    }
}

/// Notifications from `notify_additional` travel down a queue of listeners each dropped by its own
/// thread as soon as a notification reaches it, and so passed on by each in turn, to reach as many
/// listeners behind them all, which take them.
#[test]
#[timeout(15000)]
fn additional_notifications_are_passed_down_listeners_dropped_once_notified() {
    let rounds = if cfg!(miri) { 2 } else { 100 };

    for _ in 0..rounds {
        pass_down_listeners_dropped_once_notified(3, |event| event.notify_additional(3));
    }
}

/// Polls `listener` once, with `waker`.
fn poll(listener: &mut EventListener, waker: &Waker) -> Poll<()> {
    Pin::new(listener).poll(&mut Context::from_waker(waker))
}

/// Polls `listener` once, with a waker that goes nowhere, and tells whether it completed.
fn ready(listener: &mut EventListener) -> bool {
    poll(listener, Waker::noop()).is_ready()
}

/// Polls each of `listeners` once, each with a counting waker of its own, and hands back the
/// counters in the same order.
fn poll_all<'a, I>(listeners: I) -> Vec<Arc<Counter>>
where
    I: IntoIterator<Item = &'a mut EventListener>,
{
    listeners
        .into_iter()
        .map(|listener| {
            let (counter, waker) = counting_waker();
            assert!(poll(listener, &waker).is_pending());

            counter
        })
        .collect()
}

/// Queues listeners that are each dropped by a thread of their own once a notification reaches
/// them, then `takers` listeners behind those, each awaited by a thread of its own; calls `notify`
/// on the event once every one of the former has stored its waker, and waits for every thread.
///
/// A notification that one of the dropped listeners took along, rather than passing it on, would
/// leave a taker, and the dropped listeners behind that one, waiting for good.
fn pass_down_listeners_dropped_once_notified<F>(takers: usize, notify: F)
where
    F: FnOnce(&Event) -> usize,
{
    let dropped = if cfg!(miri) { 3 } else { 6 };
    let event = Event::new();
    let registered = Arc::new(Barrier::new(dropped + 1));

    let droppers: Vec<_> = (0..dropped)
        .map(|_| {
            let dropper = DropOnceWoken {
                listener: Some(event.listen()),
                registered: registered.clone(),
                polled: false,
            };
            thread::spawn(move || block_on(dropper))
        })
        .collect();
    let takers: Vec<_> = (0..takers)
        .map(|_| {
            let listener = event.listen();
            thread::spawn(move || block_on(listener))
        })
        .collect();
    registered.wait();

    // Sent to as many listeners as there are takers, each of them one to be dropped.
    assert_eq!(notify(&event), takers.len());

    for thread in droppers.into_iter().chain(takers) {
        thread.join().unwrap();
    }
    assert_eq!(format!("{event:?}"), "Event { listeners: 0, notified: 0 }");
}

/// A waker that counts its wakes, and the counter it counts them on.
fn counting_waker() -> (Arc<Counter>, Waker) {
    let counter = Arc::new(Counter::default());
    let waker = Waker::from(counter.clone());

    (counter, waker)
}

/// A waker that holds `listener`, and drops it as the last clone of the waker goes, and the
/// counter it counts its wakes on.
fn holding(listener: EventListener) -> (Arc<Counter>, Waker) {
    let counter = Arc::new(Counter::default());
    let waker = Waker::from(Arc::new(Holder {
        listener: Some(listener),
        counter: counter.clone(),
    }));

    (counter, waker)
}

/// Counts the wakes of the wakers made from it.
#[derive(Default)]
struct Counter(AtomicUsize);

impl Counter {
    /// How many wakes have reached this counter.
    fn wakes(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

impl Wake for Counter {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// A waker that panics as it is woken.
struct PanicsOnWake;

impl Wake for PanicsOnWake {
    fn wake(self: Arc<Self>) {
        panic!("a waker that panics as it is woken");
    }
}

/// A waker that panics as it is woken, with a payload that panics as it is dropped.
struct PanicsWithPayloadOnWake;

impl Wake for PanicsWithPayloadOnWake {
    fn wake(self: Arc<Self>) {
        panic::panic_any(PanicsOnDrop);
    }
}

/// A panic payload that panics as it is dropped.
struct PanicsOnDrop;

impl Drop for PanicsOnDrop {
    fn drop(&mut self) {
        panic!("a panic payload that panics as it is dropped");
    }
}

/// A waker whose wake comes back into the event it is woken by: it drops the listener it holds,
/// listens and drops that listener at once, and notifies one more listener.
struct ComesBack {
    event: Arc<Event>,
    listener: Mutex<Option<EventListener>>,
    woken: AtomicBool,
}

impl Wake for ComesBack {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let held = self
            .listener
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        drop(held);
        drop(self.event.listen());
        self.event.notify_additional(1);
        self.woken.store(true, Ordering::SeqCst);
    }
}

/// A waker that holds a listener, and drops it as the last clone of the waker goes.
struct Holder {
    listener: Option<EventListener>,
    /// Where the wakes of this waker are counted.
    counter: Arc<Counter>,
}

impl Wake for Holder {
    fn wake(self: Arc<Self>) {
        self.counter.wake_by_ref();
    }
}

impl Drop for Holder {
    fn drop(&mut self) {
        // A listener of the event this waker is stored in, so dropping it takes that event's lock.
        drop(self.listener.take());
    }
}

/// The task a waker made by [`raw_waker`] stands for: how many times its wakers were cloned,
/// dropped and woken, and the event, if any, that each clone of them notifies once.
struct RawTask {
    clones: AtomicUsize,
    drops: AtomicUsize,
    wakes: AtomicUsize,
    /// Notified by every clone before it is made, so that a notification can be made to land
    /// while a poll clones its waker.
    notified_on_clone: Option<&'static Event>,
}

impl RawTask {
    /// A task whose wakers were never cloned, dropped or woken, and whose clones notify
    /// `notified_on_clone`, if given.
    const fn new(notified_on_clone: Option<&'static Event>) -> Self {
        Self {
            clones: AtomicUsize::new(0),
            drops: AtomicUsize::new(0),
            wakes: AtomicUsize::new(0),
            notified_on_clone,
        }
    }

    /// How many times this task's wakers were cloned, dropped and woken, in that order.
    fn counts(&self) -> [usize; 3] {
        [&self.clones, &self.drops, &self.wakes].map(|count| count.load(Ordering::SeqCst))
    }
}

/// A waker of `task`, built on [`RAW_TASK_VTABLE`]. Two wakers of the same task wake the same
/// task, as far as [`Waker::will_wake`] can tell; wakers of two tasks do not.
fn raw_waker(task: &'static RawTask) -> Waker {
    let data = ptr::from_ref(task).cast::<()>();
    // SAFETY: the data pointer is that of a `&'static RawTask`, so it is valid for as long as any
    // waker made from it lives, and what it points to is `Sync`, so it may be used from any
    // thread. The functions of the vtable keep to the contract of `RawWaker`: a clone is a waker
    // of the same task, and a wake by value or a drop leaves nothing behind to be let go of.
    unsafe { Waker::from_raw(RawWaker::new(data, &RAW_TASK_VTABLE)) }
}

/// What a waker made by [`raw_waker`] does: it counts what is done with it on its task.
static RAW_TASK_VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_raw_task, wake_raw_task, wake_raw_task, drop_raw_task);

/// Counts a clone of a waker of the task behind `data`, and notifies that task's event if it has
/// one; called by the standard library, with the data pointer of a waker made by [`raw_waker`].
unsafe fn clone_raw_task(data: *const ()) -> RawWaker {
    // SAFETY: only a waker made by `raw_waker` has this vtable, and its data pointer is that of a
    // `&'static RawTask`.
    let task = unsafe { &*data.cast::<RawTask>() };
    task.clones.fetch_add(1, Ordering::SeqCst);
    if let Some(event) = task.notified_on_clone {
        event.notify(1);
    }

    RawWaker::new(data, &RAW_TASK_VTABLE)
}

/// Counts a wake of a waker of the task behind `data`, by value or by reference; called by the
/// standard library, with the data pointer of a waker made by [`raw_waker`].
unsafe fn wake_raw_task(data: *const ()) {
    // SAFETY: only a waker made by `raw_waker` has this vtable, and its data pointer is that of a
    // `&'static RawTask`.
    let task = unsafe { &*data.cast::<RawTask>() };
    task.wakes.fetch_add(1, Ordering::SeqCst);
}

/// Counts a drop of a waker of the task behind `data`; called by the standard library, with the
/// data pointer of a waker made by [`raw_waker`].
unsafe fn drop_raw_task(data: *const ()) {
    // SAFETY: only a waker made by `raw_waker` has this vtable, and its data pointer is that of a
    // `&'static RawTask`.
    let task = unsafe { &*data.cast::<RawTask>() };
    task.drops.fetch_add(1, Ordering::SeqCst);
}

/// A future that polls its listener once, to store the waker, and drops it unpolled once that
/// waker is woken: a waiter that gives up the moment a notification reaches it.
struct DropOnceWoken {
    listener: Option<EventListener>,
    /// Waited on right after the first poll, so that whoever notifies can wait for that poll.
    registered: Arc<Barrier>,
    polled: bool,
}

impl Future for DropOnceWoken {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        let Some(listener) = &mut this.listener else {
            return Poll::Ready(());
        };
        if this.polled {
            // Woken, which only a notification reaching the listener does.
            this.listener = None;

            return Poll::Ready(());
        }

        this.polled = true;
        assert!(Pin::new(listener).poll(cx).is_pending());
        this.registered.wait();

        Poll::Pending
    }
}
