use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

use futures_util::stream::{FusedStream, Stream};
use futures_util::StreamExt;
use hydracache_cluster_testkit::value_plane_stream_075::*;

#[derive(Default)]
struct State {
    batches: VecDeque<Result<Option<Vec<u8>>, &'static str>>,
    waker: Option<Waker>,
    polls: usize,
    closes: usize,
    stale: bool,
}

struct Source(Arc<Mutex<State>>);

impl BoundedStreamSource for Source {
    type Item = u8;
    type Error = &'static str;
    fn poll_batch(&mut self, context: &mut Context<'_>, _: usize) -> BatchPoll<u8, &'static str> {
        let mut state = self.0.lock().unwrap();
        state.polls += 1;
        match state.batches.pop_front() {
            Some(batch) => Poll::Ready(batch),
            None => {
                state.waker = Some(context.waker().clone());
                Poll::Pending
            }
        }
    }
    fn close(&mut self) {
        let mut state = self.0.lock().unwrap();
        state.closes += 1;
        state.waker = None;
    }
    fn validate_buffered(&mut self) -> Result<(), Self::Error> {
        if self.0.lock().unwrap().stale {
            Err("stale_cursor")
        } else {
            Ok(())
        }
    }
}

#[tokio::test]
async fn demand_stream_fetches_one_page_and_releases_every_terminal_owner() {
    let state = Arc::new(Mutex::new(State {
        batches: VecDeque::from([Ok(Some(vec![1, 2])), Ok(Some(vec![3])), Ok(None)]),
        ..State::default()
    }));
    let mut stream = DemandStream::new(Source(Arc::clone(&state)), 2).unwrap();
    assert_eq!(state.lock().unwrap().polls, 0);
    assert_eq!(stream.next().await, Some(Ok(1)));
    assert_eq!(stream.retained_items(), 1);
    assert_eq!(stream.next().await, Some(Ok(2)));
    assert_eq!(state.lock().unwrap().polls, 1);
    assert_eq!(stream.next().await, Some(Ok(3)));
    assert_eq!(stream.next().await, None);
    assert!(stream.is_terminated());
    assert_eq!(stream.next().await, None);
    drop(stream);
    assert_eq!(state.lock().unwrap().closes, 1);
}

#[tokio::test]
async fn stream_errors_cancel_buffers_and_close_exactly_once() {
    for (batch, expected) in [
        (Ok(Some(vec![])), DemandStreamError::EmptyBatch),
        (
            Ok(Some(vec![1, 2, 3])),
            DemandStreamError::OversizedBatch {
                maximum: 2,
                actual: 3,
            },
        ),
        (
            Err("stale_cursor"),
            DemandStreamError::Source("stale_cursor"),
        ),
    ] {
        let state = Arc::new(Mutex::new(State {
            batches: VecDeque::from([batch]),
            ..State::default()
        }));
        let mut stream = DemandStream::new(Source(Arc::clone(&state)), 2).unwrap();
        assert_eq!(stream.next().await, Some(Err(expected)));
        assert_eq!(stream.next().await, None);
        assert_eq!(stream.retained_items(), 0);
        stream.cancel();
        drop(stream);
        assert_eq!(state.lock().unwrap().closes, 1);
    }
    for limit in [0, 257] {
        let state = Arc::new(Mutex::new(State::default()));
        assert!(matches!(
            DemandStream::new(Source(Arc::clone(&state)), limit),
            Err(DemandStreamError::InvalidPrefetch)
        ));
        assert_eq!(state.lock().unwrap().closes, 1);
    }
    let state = Arc::new(Mutex::new(State {
        batches: VecDeque::from([Ok(Some(vec![1, 2]))]),
        ..State::default()
    }));
    let mut stream = DemandStream::new(Source(Arc::clone(&state)), 2).unwrap();
    assert_eq!(stream.next().await, Some(Ok(1)));
    state.lock().unwrap().stale = true;
    assert_eq!(
        stream.next().await,
        Some(Err(DemandStreamError::Source("stale_cursor")))
    );
    stream.cancel();
    assert_eq!(stream.retained_items(), 0);
    assert_eq!(stream.next().await, None);
    drop(stream);
    assert_eq!(state.lock().unwrap().closes, 1);
}

#[derive(Default)]
struct WakeCounter(std::sync::atomic::AtomicUsize);
impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[test]
fn temporarily_empty_listener_waits_for_signal_instead_of_completing() {
    let state = Arc::new(Mutex::new(State::default()));
    let mut stream = DemandStream::new(Source(Arc::clone(&state)), 1).unwrap();
    let wake = Arc::new(WakeCounter::default());
    let waker = Waker::from(Arc::clone(&wake));
    let mut context = Context::from_waker(&waker);
    assert_eq!(Pin::new(&mut stream).poll_next(&mut context), Poll::Pending);
    assert!(!stream.is_terminated());
    let saved = {
        let mut state = state.lock().unwrap();
        state.batches.push_back(Ok(Some(vec![7])));
        state.waker.take().unwrap()
    };
    saved.wake();
    assert_eq!(wake.0.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        Pin::new(&mut stream).poll_next(&mut context),
        Poll::Ready(Some(Ok(7)))
    );
    drop(stream);
    assert!(state.lock().unwrap().waker.is_none());
    assert_eq!(state.lock().unwrap().closes, 1);
}
