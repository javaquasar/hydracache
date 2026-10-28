//! Pull-driven Rust streams over bounded page and listener sources.
//!
//! Sources fetch only after downstream polls. Pending means no item is available yet;
//! `Ok(None)` alone terminates a stream. Every terminal, cancellation and drop path closes once.

use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_util::stream::{FusedStream, Stream};

pub type BatchPoll<T, E> = Poll<Result<Option<Vec<T>>, E>>;
pub type StreamPoll<T, E> = Poll<Option<Result<T, DemandStreamError<E>>>>;

pub trait BoundedStreamSource: Unpin {
    type Item;
    type Error;

    /// `Some` must contain 1..=max_items values. Register the waker before returning Pending.
    fn poll_batch(
        &mut self,
        context: &mut Context<'_>,
        max_items: usize,
    ) -> BatchPoll<Self::Item, Self::Error>;

    /// Page sources revalidate continuity even while their previous page is buffered.
    fn validate_buffered(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn close(&mut self);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DemandStreamError<E> {
    InvalidPrefetch,
    EmptyBatch,
    OversizedBatch { maximum: usize, actual: usize },
    Source(E),
}

/// A stream owns one page buffer and one source. No background prefetch task is created.
pub struct DemandStream<S: BoundedStreamSource> {
    source: S,
    prefetch: usize,
    buffered: VecDeque<S::Item>,
    terminated: bool,
    closed: bool,
}

// Moving this stream never moves a pinned future or source; the source itself is Unpin.
impl<S: BoundedStreamSource> Unpin for DemandStream<S> {}

impl<S: BoundedStreamSource> DemandStream<S> {
    pub fn new(mut source: S, prefetch: usize) -> Result<Self, DemandStreamError<S::Error>> {
        if prefetch == 0 || prefetch > 256 {
            source.close();
            return Err(DemandStreamError::InvalidPrefetch);
        }
        Ok(Self {
            source,
            prefetch,
            buffered: VecDeque::new(),
            terminated: false,
            closed: false,
        })
    }

    pub fn retained_items(&self) -> usize {
        self.buffered.len()
    }

    pub fn cancel(&mut self) {
        self.terminated = true;
        self.buffered.clear();
        self.close_once();
    }

    fn close_once(&mut self) {
        if !self.closed {
            self.closed = true;
            self.source.close();
        }
    }

    fn fail(&mut self, error: DemandStreamError<S::Error>) -> StreamPoll<S::Item, S::Error> {
        self.cancel();
        Poll::Ready(Some(Err(error)))
    }
}

impl<S: BoundedStreamSource> Stream for DemandStream<S> {
    type Item = Result<S::Item, DemandStreamError<S::Error>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.terminated {
            return Poll::Ready(None);
        }
        if let Err(error) = this.source.validate_buffered() {
            return this.fail(DemandStreamError::Source(error));
        }
        if let Some(item) = this.buffered.pop_front() {
            return Poll::Ready(Some(Ok(item)));
        }
        match this.source.poll_batch(context, this.prefetch) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(error)) => this.fail(DemandStreamError::Source(error)),
            Poll::Ready(Ok(None)) => {
                this.cancel();
                Poll::Ready(None)
            }
            Poll::Ready(Ok(Some(batch))) if batch.is_empty() => {
                this.fail(DemandStreamError::EmptyBatch)
            }
            Poll::Ready(Ok(Some(batch))) if batch.len() > this.prefetch => {
                this.fail(DemandStreamError::OversizedBatch {
                    maximum: this.prefetch,
                    actual: batch.len(),
                })
            }
            Poll::Ready(Ok(Some(batch))) => {
                this.buffered = batch.into();
                Poll::Ready(this.buffered.pop_front().map(Ok))
            }
        }
    }
}

impl<S: BoundedStreamSource> FusedStream for DemandStream<S> {
    fn is_terminated(&self) -> bool {
        self.terminated
    }
}

impl<S: BoundedStreamSource> Drop for DemandStream<S> {
    fn drop(&mut self) {
        self.close_once();
    }
}
