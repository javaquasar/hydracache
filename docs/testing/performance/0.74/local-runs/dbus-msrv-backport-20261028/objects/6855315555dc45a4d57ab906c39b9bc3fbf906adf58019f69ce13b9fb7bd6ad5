//! Tests of `zruntime::Unblock`, the async adapter for a blocking I/O handle.
//!
//! The handles are in memory, cursors, vectors, iterators and small types of the tests' own, so
//! that the tests run under Miri. Most drive the adapter with the `block_on` of `futures`; the
//! few that need an operation to be pending poll it by hand, with a handle that waits for the test
//! to let it go on. They come in the order of what they pin down: that reads hand over every byte,
//! whatever the buffers, the short reads, the interruptions and the errors; that a read polled
//! again may take another buffer, and that what a read given up on read, or what was read ahead,
//! outlives the writes in between; that seeks land where the reads got to, keep what was read ahead
//! when they fail, and are tried again when interrupted; that writes and flushes reach the handle,
//! report its errors and outlive the adapter, and that a flush is tried again when interrupted;
//! that the handle can be reached directly; that an iterator streams its items a batch at a time;
//! that a panic reaches the caller and takes the handle with it; that two tasks waiting on one
//! operation are both woken, even where the waker of one panics; and that the adapter prints
//! without its handle.

use std::{
    future::poll_fn,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    num::NonZeroUsize,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::{Context, Wake, Waker},
};

use futures::{
    AsyncReadExt, AsyncSeekExt, AsyncWriteExt, StreamExt,
    executor::block_on,
    io::{AsyncRead, AsyncWrite},
};
use ntest::timeout;

use crate::Unblock;

/// Read through a buffer smaller than what one operation reads ahead, a handle hands over every
/// byte it holds, in order, and then its end.
#[test]
#[timeout(15000)]
fn small_buffers_read_every_byte_of_the_handle() {
    let data = bytes(300);
    let mut reader = Unblock::with_capacity(cap(64), Cursor::new(data.clone()));

    let mut read = Vec::new();
    let mut buf = [0; 7];
    loop {
        let len = block_on(reader.read(&mut buf)).unwrap();
        if len == 0 {
            break;
        }
        read.extend_from_slice(&buf[..len]);
    }

    assert_eq!(read, data);
}

/// Read through a buffer larger than the capacity, a handle hands over no more than the capacity
/// at a time, and every byte it holds all the same.
#[test]
#[timeout(15000)]
fn large_buffers_read_every_byte_of_the_handle() {
    let data = bytes(300);
    let mut reader = Unblock::with_capacity(cap(64), Cursor::new(data.clone()));

    let mut first = [0; 1024];
    let len = block_on(reader.read(&mut first)).unwrap();
    assert_eq!(len, 64);
    let mut rest = Vec::new();
    block_on(reader.read_to_end(&mut rest)).unwrap();

    assert_eq!([&first[..len], &rest].concat(), data);
}

/// A handle that reads one byte at a time, and is interrupted before each, hands over every byte
/// all the same: an interruption is tried again, and never reaches the caller.
#[test]
#[timeout(15000)]
fn short_and_interrupted_reads_hand_over_every_byte() {
    let data = bytes(40);
    let mut reader = Unblock::new(Trickle {
        inner: Cursor::new(data.clone()),
        interrupt: true,
    });

    let mut read = Vec::new();
    let mut buf = [0; 8];
    loop {
        let len = block_on(reader.read(&mut buf)).expect("interruptions are tried again");
        if len == 0 {
            break;
        }
        read.extend_from_slice(&buf[..len]);
    }

    assert_eq!(read, data);
}

/// A read error reaches the caller as the handle reported it, and the reads after it go on.
#[test]
#[timeout(15000)]
fn a_read_error_reaches_the_caller_and_the_reads_after_it_go_on() {
    let mut reader = Unblock::new(FailsOnce {
        failed: false,
        inner: Cursor::new(b"after".to_vec()),
    });
    let mut buf = [0; 8];

    let error = block_on(reader.read(&mut buf)).expect_err("the first read fails");
    assert_eq!(error.kind(), io::ErrorKind::ConnectionReset);

    let len = block_on(reader.read(&mut buf)).unwrap();
    assert_eq!(&buf[..len], b"after");
}

/// A read that was pending may be polled again with another buffer than the one it was first
/// given, as `futures-io` allows: the bytes land in the buffer of the poll that hands them out,
/// and the first one is left untouched.
#[test]
#[timeout(15000)]
fn a_read_polled_again_with_another_buffer_fills_that_one() {
    let (open, gate) = mpsc::channel();
    let mut reader = Unblock::new(Gated {
        gate,
        inner: Cursor::new(b"hello".to_vec()),
    });

    let mut first = [0; 8];
    let mut cx = Context::from_waker(Waker::noop());
    assert!(
        Pin::new(&mut reader)
            .poll_read(&mut cx, &mut first)
            .is_pending()
    );
    open.send(()).expect("the read waits for the test");

    let mut second = [0; 8];
    let len = block_on(poll_fn(|cx| {
        Pin::new(&mut reader).poll_read(cx, &mut second)
    }))
    .unwrap();

    assert_eq!(&second[..len], b"hello");
    assert_eq!(first, [0; 8]);
}

/// A read that was pending and given up on, its handle written to after it, still hands what it
/// read to the next read: a handle with two separate streams loses none of the bytes it read.
#[test]
#[timeout(15000)]
fn a_read_given_up_on_keeps_what_it_read_for_the_next_read() {
    let (open, gate) = mpsc::channel();
    let mut duplex = Unblock::new(Gated {
        gate,
        inner: Duplex::new(b"in", Vec::new()),
    });

    let mut cx = Context::from_waker(Waker::noop());
    assert!(
        Pin::new(&mut duplex)
            .poll_read(&mut cx, &mut [0; 8])
            .is_pending()
    );
    // One read and no more: a second one would fail.
    open.send(()).expect("the read waits for the test");
    drop(open);
    block_on(duplex.write_all(b"out")).unwrap();

    let mut buf = [0; 8];
    let len = block_on(duplex.read(&mut buf)).unwrap();
    assert_eq!(&buf[..len], b"in");
    assert_eq!(block_on(duplex.into_inner()).inner.writer, b"out");
}

/// The end of a handle that a read given up on found reaches the next read, which does not read
/// the handle again for it: a read past the end of the standard input of a terminal, say, would
/// wait for more.
#[test]
#[timeout(15000)]
fn the_end_found_by_a_read_given_up_on_reaches_the_next_read() {
    let (open, gate) = mpsc::channel();
    let mut duplex = Unblock::new(Gated {
        gate,
        inner: Duplex::new(b"", Vec::new()),
    });

    let mut cx = Context::from_waker(Waker::noop());
    assert!(
        Pin::new(&mut duplex)
            .poll_read(&mut cx, &mut [0; 8])
            .is_pending()
    );
    // One read and no more: a second one would fail.
    open.send(()).expect("the read waits for the test");
    drop(open);
    block_on(duplex.write_all(b"out")).unwrap();

    assert_eq!(block_on(duplex.read(&mut [0; 8])).unwrap(), 0);
    assert_eq!(block_on(duplex.into_inner()).inner.writer, b"out");
}

/// The bytes read ahead outlive a write to the handle: a handle with two separate streams, as a
/// socket has, hands them to the reads after the write.
#[test]
#[timeout(15000)]
fn the_bytes_read_ahead_outlive_a_write() {
    let mut duplex = Unblock::new(Duplex::new(b"abcdef", Vec::new()));

    let mut buf = [0; 2];
    block_on(duplex.read_exact(&mut buf)).unwrap();
    assert_eq!(&buf, b"ab");
    block_on(async {
        duplex.write_all(b"x").await?;
        duplex.flush().await
    })
    .unwrap();

    let mut rest = Vec::new();
    block_on(duplex.read_to_end(&mut rest)).unwrap();
    assert_eq!(rest, b"cdef");
    assert_eq!(block_on(duplex.into_inner()).writer, b"x");
}

/// A seek from the current position, after a read that read ahead, lands where the reads got to
/// plus the offset, not where the handle's own position is.
#[test]
#[timeout(15000)]
fn a_seek_from_the_current_position_counts_from_where_the_reads_got_to() {
    let mut file = Unblock::with_capacity(cap(64), Cursor::new(bytes(256)));

    block_on(async {
        let mut buf = [0; 10];
        file.read_exact(&mut buf).await?;

        assert_eq!(file.seek(SeekFrom::Current(5)).await?, 15);
        file.read_exact(&mut buf[..1]).await?;
        assert_eq!(buf[0], 15);

        assert_eq!(file.seek(SeekFrom::Current(-6)).await?, 10);
        file.read_exact(&mut buf[..1]).await?;
        assert_eq!(buf[0], 10);

        io::Result::Ok(())
    })
    .unwrap();
}

/// A seek to the current position, after a read that read ahead, brings the handle's own position
/// back to where the reads got to, as a write after a read on a file needs.
#[test]
#[timeout(15000)]
fn a_seek_to_the_current_position_brings_the_handle_back_to_the_reads() {
    let mut file = Unblock::with_capacity(cap(64), Cursor::new(bytes(256)));

    block_on(file.read_exact(&mut [0; 10])).unwrap();
    assert_eq!(block_on(file.seek(SeekFrom::Current(0))).unwrap(), 10);

    assert_eq!(block_on(file.into_inner()).position(), 10);
}

/// A seek from the current position whose offset from the handle's own position is out of the
/// range of an `i64` fails without seeking, and the reads go on from where they got to.
#[test]
#[timeout(15000)]
fn a_seek_out_of_range_fails_and_keeps_the_bytes_read_ahead() {
    let mut file = Unblock::with_capacity(cap(64), Cursor::new(bytes(256)));

    let mut buf = [0; 10];
    block_on(file.read_exact(&mut buf)).unwrap();
    let error = block_on(file.seek(SeekFrom::Current(i64::MIN))).expect_err("out of range");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);

    block_on(file.read_exact(&mut buf[..1])).unwrap();
    assert_eq!(buf[0], 10);
}

/// A seek that the handle fails leaves its position where it was, and the bytes read ahead with
/// it: the reads go on from where they got to.
#[test]
#[timeout(15000)]
fn a_seek_the_handle_fails_keeps_the_bytes_read_ahead() {
    let mut file = Unblock::with_capacity(cap(64), Cursor::new(bytes(256)));

    let mut buf = [0; 10];
    block_on(file.read_exact(&mut buf)).unwrap();
    block_on(file.seek(SeekFrom::Current(-1000))).expect_err("a seek to before the start fails");

    block_on(file.read_exact(&mut buf[..1])).unwrap();
    assert_eq!(buf[0], 10);
}

/// A seek that the handle is interrupted in is tried again, since `futures-io` lets no seek report
/// an interruption: it lands where it was asked to, the handle having been sought twice.
#[test]
#[timeout(15000)]
fn an_interrupted_seek_is_tried_again() {
    let mut handle = Unblock::new(Hiccups {
        inner: Cursor::new(bytes(10)),
        ..Hiccups::default()
    });

    assert_eq!(block_on(handle.seek(SeekFrom::Start(4))).unwrap(), 4);

    let handle = block_on(handle.into_inner());
    assert_eq!(handle.seeks, 2);
    assert_eq!(handle.inner.position(), 4);
}

/// Bytes written, then flushed, reach the handle, which is flushed in turn.
#[test]
#[timeout(15000)]
fn a_write_and_a_flush_reach_the_handle() {
    let log = Arc::new(Mutex::new(Log::default()));
    let mut writer = Unblock::new(Recorder(log.clone()));

    block_on(async {
        writer.write_all(b"hello").await?;
        writer.flush().await
    })
    .unwrap();

    let log = log.lock().unwrap();
    assert_eq!(log.written, b"hello");
    assert_eq!(log.flushes, 1);
}

/// A flush that the handle is interrupted in is tried again, since `futures-io` lets no flush
/// report an interruption: it completes, the handle having been flushed twice.
#[test]
#[timeout(15000)]
fn an_interrupted_flush_is_tried_again() {
    let mut writer = Unblock::new(Hiccups::default());

    block_on(writer.flush()).unwrap();

    assert_eq!(block_on(writer.into_inner()).flushes, 2);
}

/// A write completes once it has taken its bytes, so an error the handle runs into writing them is
/// reported by the next write, which then takes nothing; the write after that goes on.
#[test]
#[timeout(15000)]
fn a_write_error_reaches_the_next_write() {
    let mut writer = Unblock::new(Broken);

    assert_eq!(block_on(writer.write(b"lost")).unwrap(), 4);
    let error = block_on(writer.write(b"again")).expect_err("the write before failed");
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);

    assert_eq!(block_on(writer.write(b"again")).unwrap(), 5);
    // Waited for, so that no write is left in flight past the test.
    block_on(writer.flush()).expect_err("the write before failed");
}

/// An error the handle runs into writing the bytes a write took is reported by the flush after it,
/// and the flush after that goes on.
#[test]
#[timeout(15000)]
fn a_write_error_reaches_the_next_flush() {
    let mut writer = Unblock::new(Broken);

    assert_eq!(block_on(writer.write(b"lost")).unwrap(), 4);
    let error = block_on(writer.flush()).expect_err("the write before failed");
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);

    block_on(writer.flush()).unwrap();
}

/// An error the handle runs into writing, found by a read that waited for the write, is kept for
/// the next write rather than reported by the read, which it is none of.
#[test]
#[timeout(15000)]
fn a_write_error_found_by_a_read_is_kept_for_the_next_write() {
    let mut duplex = Unblock::new(Duplex::new(b"in", Broken));

    assert_eq!(block_on(duplex.write(b"lost")).unwrap(), 4);
    let mut buf = [0; 8];
    let len = block_on(duplex.read(&mut buf)).expect("the read is not the write's to fail");
    assert_eq!(&buf[..len], b"in");

    let error = block_on(duplex.write(b"again")).expect_err("the write before failed");
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}

/// The handle out of the adapter holds every byte the writes took, flushed or not.
#[test]
#[timeout(15000)]
fn into_inner_hands_back_a_handle_holding_every_byte_written() {
    let data = bytes(100);
    let mut writer = Unblock::with_capacity(cap(4), Vec::new());

    block_on(writer.write_all(&data)).unwrap();

    assert_eq!(block_on(writer.into_inner()), data);
}

/// A write that took its bytes goes on after the adapter is dropped: no byte it took is left
/// behind.
#[test]
#[timeout(15000)]
fn a_dropped_adapter_still_writes_what_it_took() {
    let (report, reported) = mpsc::channel();
    let mut writer = Unblock::new(Reporting(report));

    block_on(writer.write_all(b"taken")).unwrap();
    drop(writer);

    assert_eq!(reported.recv().expect("the write goes on"), b"taken");
}

/// `with_mut` runs its operation on the handle, and hands back what the operation returned.
#[test]
#[timeout(15000)]
fn with_mut_runs_on_the_handle_and_hands_back_what_it_returned() {
    let mut reader = Unblock::new(Cursor::new(bytes(10)));

    let len = block_on(reader.with_mut(|cursor| {
        cursor.set_position(3);
        cursor.get_ref().len()
    }));
    assert_eq!(len, 10);

    let mut buf = [0; 1];
    block_on(reader.read_exact(&mut buf)).unwrap();
    assert_eq!(buf[0], 3);
}

/// `get_mut` reaches the handle, past the bytes read ahead, which it drops: the reads after it go
/// on from where the handle is.
#[test]
#[timeout(15000)]
fn get_mut_reaches_the_handle_and_drops_what_was_read_ahead() {
    let mut reader = Unblock::with_capacity(cap(8), Cursor::new(bytes(20)));

    let mut buf = [0; 2];
    block_on(reader.read_exact(&mut buf)).unwrap();
    assert_eq!(block_on(reader.get_mut()).position(), 8);

    block_on(reader.read_exact(&mut buf[..1])).unwrap();
    assert_eq!(buf[0], 8);
}

/// The stream of an iterator yields every item of it, in order, and ends where it does.
#[test]
#[timeout(15000)]
fn a_stream_of_an_iterator_yields_every_item_in_order_and_ends() {
    let mut stream = Unblock::new(0..100);

    let items: Vec<_> = block_on((&mut stream).collect());

    assert_eq!(items, (0..100).collect::<Vec<_>>());
    assert_eq!(block_on(stream.next()), None);
}

/// A stream pulls items from its iterator a batch at a time, of 16 items, or of the capacity where
/// that is smaller.
#[test]
#[timeout(15000)]
fn a_stream_pulls_a_batch_of_items_at_a_time() {
    for (capacity, batch) in [(DEFAULT, 16), (cap(1), 1), (cap(5), 5)] {
        let pulled = Arc::new(AtomicUsize::new(0));
        let counted = pulled.clone();
        let iter = (0..100).inspect(move |_| {
            counted.fetch_add(1, Ordering::Relaxed);
        });
        let mut stream = Unblock::with_capacity(capacity, iter);

        assert_eq!(block_on(stream.next()), Some(0));
        assert_eq!(pulled.load(Ordering::Relaxed), batch);
    }
}

/// A panic in an operation on the handle is raised again where the adapter is polled, with the
/// payload it had. The handle goes with it, and any use of the adapter after that panics as well.
#[test]
#[timeout(15000)]
fn a_panic_in_an_operation_reaches_the_caller_and_takes_the_handle() {
    let mut reader = Unblock::new(Panicking);

    let panicked = catch_unwind(AssertUnwindSafe(|| block_on(reader.read(&mut [0; 8]))))
        .expect_err("the read panicked");
    assert_eq!(
        panicked.downcast_ref::<&str>(),
        Some(&"the handle panicked"),
    );

    for later in [
        catch_unwind(AssertUnwindSafe(|| {
            block_on(reader.read(&mut [0; 8])).ok();
        })),
        catch_unwind(AssertUnwindSafe(|| {
            block_on(reader.get_mut());
        })),
    ] {
        let lost = later.expect_err("the adapter has no handle left");
        assert_eq!(lost.downcast_ref::<&str>(), Some(&LOST));
    }
}

/// A panic in the operation that `with_mut` runs is raised again where its future is polled, and
/// takes the handle with it as well.
#[test]
#[timeout(15000)]
fn a_panic_in_with_mut_reaches_the_caller_and_takes_the_handle() {
    let mut reader = Unblock::new(Cursor::new(bytes(10)));

    let panicked = catch_unwind(AssertUnwindSafe(|| {
        block_on(reader.with_mut(|_| panic!("the operation panicked")))
    }))
    .expect_err("the operation panicked");
    assert_eq!(
        panicked.downcast_ref::<&str>(),
        Some(&"the operation panicked"),
    );

    let lost = catch_unwind(AssertUnwindSafe(|| block_on(reader.into_inner())))
        .expect_err("the adapter has no handle left");
    assert_eq!(lost.downcast_ref::<&str>(), Some(&LOST));
}

/// Two tasks may wait for one operation at once, as the halves of a split adapter do: one reads,
/// and the other writes while the read is in flight. Both are woken once the read is over, not
/// only the one that polled last, and both then go on.
#[test]
#[timeout(15000)]
fn two_tasks_waiting_for_one_operation_are_both_woken() {
    let (open, gate) = mpsc::channel();
    let mut duplex = Unblock::new(Gated {
        gate,
        inner: Duplex::new(b"in", Vec::new()),
    });
    let (report, reported) = mpsc::channel();
    let reader = Waker::from(Arc::new(Report(report.clone(), "reader")));
    let writer = Waker::from(Arc::new(Report(report, "writer")));

    let mut buf = [0; 8];
    assert!(
        Pin::new(&mut duplex)
            .poll_read(&mut Context::from_waker(&reader), &mut buf)
            .is_pending()
    );
    assert!(
        Pin::new(&mut duplex)
            .poll_write(&mut Context::from_waker(&writer), b"out")
            .is_pending()
    );
    open.send(()).expect("the read waits for the test");

    let mut woken = [(); 2].map(|()| reported.recv().expect("both tasks are woken"));
    woken.sort();
    assert_eq!(woken, ["reader", "writer"]);

    let written = block_on(poll_fn(|cx| Pin::new(&mut duplex).poll_write(cx, b"out"))).unwrap();
    assert_eq!(written, 3);
    let len = block_on(poll_fn(|cx| Pin::new(&mut duplex).poll_read(cx, &mut buf))).unwrap();
    assert_eq!(&buf[..len], b"in");
    assert_eq!(block_on(duplex.into_inner()).inner.writer, b"out");
}

/// A waker that panics as an operation ends keeps no other task waiting for that operation from
/// being woken: here the reader's waker panics, and the writer, which waits behind it, is woken all
/// the same, and goes on.
#[test]
#[timeout(15000)]
fn a_waker_that_panics_leaves_the_other_tasks_woken() {
    let (open, gate) = mpsc::channel();
    let mut duplex = Unblock::new(Gated {
        gate,
        inner: Duplex::new(b"in", Vec::new()),
    });
    let (report, reported) = mpsc::channel();
    let reader = Waker::from(Arc::new(Panic));
    let writer = Waker::from(Arc::new(Report(report, "writer")));

    let mut buf = [0; 8];
    assert!(
        Pin::new(&mut duplex)
            .poll_read(&mut Context::from_waker(&reader), &mut buf)
            .is_pending()
    );
    assert!(
        Pin::new(&mut duplex)
            .poll_write(&mut Context::from_waker(&writer), b"out")
            .is_pending()
    );
    open.send(()).expect("the read waits for the test");

    assert_eq!(reported.recv(), Ok("writer"));
    let written = block_on(poll_fn(|cx| Pin::new(&mut duplex).poll_write(cx, b"out"))).unwrap();
    assert_eq!(written, 3);
    let len = block_on(poll_fn(|cx| Pin::new(&mut duplex).poll_read(cx, &mut buf))).unwrap();
    assert_eq!(&buf[..len], b"in");
    assert_eq!(block_on(duplex.into_inner()).inner.writer, b"out");
}

/// An adapter prints without its handle, so the handle need not be `Debug`.
#[test]
#[timeout(15000)]
fn an_adapter_prints_without_its_handle() {
    struct NotDebug;

    assert_eq!(format!("{:?}", Unblock::new(NotDebug)), "Unblock { .. }");
}

/// What a use of an adapter whose handle went with a panic panics with.
const LOST: &str = "the handle of an `Unblock` was lost to a panic in an operation on it";

/// The default capacity of an adapter.
const DEFAULT: NonZeroUsize = NonZeroUsize::new(8 * 1024).unwrap();

/// `len` bytes, each the low byte of its index.
fn bytes(len: usize) -> Vec<u8> {
    (0..len).map(|i| i as u8).collect()
}

/// A capacity of `cap`, which is not zero.
fn cap(cap: usize) -> NonZeroUsize {
    NonZeroUsize::new(cap).expect("a capacity of no bytes")
}

/// A handle that reads from one stream and writes to another, as a socket does.
struct Duplex<W> {
    reader: Cursor<Vec<u8>>,
    writer: W,
}

impl<W> Duplex<W> {
    fn new(incoming: &[u8], writer: W) -> Self {
        Self {
            reader: Cursor::new(incoming.to_vec()),
            writer,
        }
    }
}

impl<W> Read for Duplex<W> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

impl<W> Write for Duplex<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// A handle whose every read waits for the test to let it go on, and fails once the test will not
/// let another go on.
struct Gated<T> {
    gate: mpsc::Receiver<()>,
    inner: T,
}

impl<T> Read for Gated<T>
where
    T: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.gate
            .recv()
            .map_err(|_| io::Error::other("a read the test did not let go on"))?;
        self.inner.read(buf)
    }
}

impl<T> Write for Gated<T>
where
    T: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A reader that reads one byte at a time, and is interrupted before each.
struct Trickle {
    inner: Cursor<Vec<u8>>,
    interrupt: bool,
}

impl Read for Trickle {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.interrupt = !self.interrupt;
        if !self.interrupt {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let len = buf.len().min(1);
        self.inner.read(&mut buf[..len])
    }
}

/// A reader whose first read fails, and whose reads after it read from `inner`.
struct FailsOnce {
    failed: bool,
    inner: Cursor<Vec<u8>>,
}

impl Read for FailsOnce {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.failed {
            self.failed = true;
            return Err(io::ErrorKind::ConnectionReset.into());
        }
        self.inner.read(buf)
    }
}

/// A reader whose reads panic.
struct Panicking;

impl Read for Panicking {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("the handle panicked");
    }
}

/// What a [`Recorder`] was asked to do.
#[derive(Default)]
struct Log {
    written: Vec<u8>,
    flushes: usize,
}

/// A writer that logs what it writes, and how often it is flushed, where the test can see it.
struct Recorder(Arc<Mutex<Log>>);

impl Write for Recorder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().flushes += 1;
        Ok(())
    }
}

/// A writer that sends what it writes to the test.
struct Reporting(mpsc::Sender<Vec<u8>>);

impl Write for Reporting {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf.to_vec()).ok();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A writer whose writes fail, and whose flushes do not.
struct Broken;

impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A cursor whose first flush and first seek are each interrupted, and which counts its flushes
/// and its seeks.
#[derive(Default)]
struct Hiccups {
    inner: Cursor<Vec<u8>>,
    flushes: usize,
    seeks: usize,
}

impl Write for Hiccups {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if self.flushes == 1 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        Ok(())
    }
}

impl Seek for Hiccups {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seeks += 1;
        if self.seeks == 1 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.inner.seek(pos)
    }
}

/// A waker that tells the test which task it wakes.
struct Report(mpsc::Sender<&'static str>, &'static str);

impl Wake for Report {
    fn wake(self: Arc<Self>) {
        self.0.send(self.1).ok();
    }
}

/// A waker whose `wake` panics.
struct Panic;

impl Wake for Panic {
    fn wake(self: Arc<Self>) {
        panic!("the waker panicked");
    }
}
