//! Exact source-extracted syscall leaves plus the actual public framing defaults.
use super::*;
use std::{
    fs::{self, File, OpenOptions},
    os::unix::{fs::MetadataExt, net::UnixStream},
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tokio::io::unix::AsyncFd;
use zbus::{
    connection::socket::{ReadHalf, WriteHalf},
    export::async_trait::async_trait,
    message::Message,
};
const LIMIT: Duration = Duration::from_millis(250);
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    file: File,
    path: PathBuf,
    identity: (u64, u64),
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(std::env::var_os("HC074_FD_ROOT").unwrap());
        let path = root.join(format!(
            "fragment-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let m = file.metadata().unwrap();
        Self {
            file,
            path,
            identity: (m.dev(), m.ino()),
        }
    }
    fn aliases(&self) -> usize {
        fs::read_dir("/proc/self/fd")
            .unwrap()
            .filter_map(|entry| match fs::metadata(entry.unwrap().path()) {
                Ok(m) => Some((m.dev(), m.ino())),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => panic!("FD census: {e}"),
            })
            .filter(|v| *v == self.identity)
            .count()
    }
    fn clean(&self) {
        assert_eq!(self.aliases(), 1);
        let m = fs::metadata(&self.path).unwrap();
        assert_eq!((m.dev(), m.ino()), self.identity);
        fs::remove_file(&self.path).unwrap();
    }
    fn message(&self) -> Message {
        Message::method_call("/org/hydracache/Fragment", "Read")
            .destination(":1.1")
            .interface("org.hydracache.Fragment")
            .build(&(zbus::Fd::from(self.file.as_fd()), "bounded-payload-074"))
            .unwrap()
    }
}
#[derive(Debug)]
struct Reader {
    socket: AsyncFd<UnixStream>,
    cap: usize,
    calls: Vec<(usize, usize)>,
}
#[async_trait]
impl ReadHalf for Reader {
    async fn recvmsg(&mut self, buffer: &mut [u8]) -> io::Result<(usize, Vec<OwnedFd>)> {
        let len = self.cap.min(buffer.len());
        assert!(len > 0);
        loop {
            let mut ready = self.socket.readable().await?;
            match ready.try_io(|inner| fd_recvmsg(inner.get_ref().as_fd(), &mut buffer[..len])) {
                Ok(result) => {
                    if let Ok((n, fds)) = &result {
                        self.calls.push((*n, fds.len()));
                    }
                    return result;
                }
                Err(_) => continue,
            }
        }
    }
    fn can_pass_unix_fd(&self) -> bool {
        true
    }
}
#[derive(Debug)]
struct Writer {
    socket: AsyncFd<UnixStream>,
    cap: usize,
    calls: Vec<(usize, usize)>,
    flushes: usize,
    break_at: Option<usize>,
}
#[async_trait]
impl WriteHalf for Writer {
    async fn sendmsg(&mut self, buffer: &[u8], fds: &[BorrowedFd<'_>]) -> io::Result<usize> {
        let len = self.cap.min(buffer.len());
        assert!(len > 0);
        if self.break_at == Some(self.calls.len()) {
            self.socket.get_ref().shutdown(std::net::Shutdown::Write)?;
            self.break_at = None;
        }
        loop {
            let mut ready = self.socket.writable().await?;
            match ready.try_io(|inner| fd_sendmsg(inner.get_ref().as_fd(), &buffer[..len], fds)) {
                Ok(result) => {
                    self.calls
                        .push((result.as_ref().copied().unwrap_or(0), fds.len()));
                    return result;
                }
                Err(_) => continue,
            }
        }
    }
    async fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        Ok(())
    }
    async fn close(&mut self) -> io::Result<()> {
        self.socket.get_ref().shutdown(std::net::Shutdown::Both)
    }
    fn can_pass_unix_fd(&self) -> bool {
        true
    }
}
fn sockets() -> (UnixStream, UnixStream) {
    let (a, b) = UnixStream::pair().unwrap();
    a.set_nonblocking(true).unwrap();
    b.set_nonblocking(true).unwrap();
    (a, b)
}

#[test]
fn forced_small_chunks_preserve_bytes_and_single_fd() {
    for (wcap, rcap) in [(1, 1), (3, 5), (7, 2)] {
        let f = Fixture::new();
        let message = f.message();
        let expected = message.data().to_vec();
        let (a, b) = sockets();
        let (sent, received, writes, reads, flushes) = zbus::block_on(async {
            let mut writer = Writer {
                socket: registered(a),
                cap: wcap,
                calls: vec![],
                flushes: 0,
                break_at: None,
            };
            let mut reader = Reader {
                socket: registered(b),
                cap: rcap,
                calls: vec![],
            };
            let task = tokio::runtime::Handle::current().spawn(async move {
                let value = tokio::time::timeout(
                    LIMIT,
                    reader.receive_message(74, &mut vec![], &mut vec![]),
                )
                .await
                .unwrap();
                (value, reader.calls)
            });
            let sent = tokio::time::timeout(LIMIT, writer.send_message(&message))
                .await
                .unwrap();
            let (received, reads) = tokio::time::timeout(LIMIT, task).await.unwrap().unwrap();
            (sent, received, writer.calls, reads, writer.flushes)
        });
        sent.unwrap();
        let received = received.unwrap();
        assert_eq!(&received.data()[..], expected.as_slice());
        assert_eq!(received.data().fds().len(), 1);
        let (fd, text): (zbus::OwnedFd, String) = received.body().deserialize().unwrap();
        let file = File::from(OwnedFd::from(fd));
        let m = file.metadata().unwrap();
        assert_eq!((m.dev(), m.ino()), f.identity);
        assert_eq!(text, "bounded-payload-074");
        assert!(writes.len() > 1 && reads.len() > 1);
        assert!(writes.iter().all(|(n, _)| *n > 0 && *n <= wcap));
        assert!(reads.iter().all(|(n, _)| *n > 0 && *n <= rcap));
        assert_eq!(writes.iter().map(|(n, _)| n).sum::<usize>(), expected.len());
        assert_eq!(reads.iter().map(|(n, _)| n).sum::<usize>(), expected.len());
        assert_eq!(writes[0].1, 1);
        assert!(writes[1..].iter().all(|(_, fds)| *fds == 0));
        assert_eq!(reads.iter().map(|(_, n)| n).sum::<usize>(), 1);
        assert_eq!(flushes, 1);
        println!(
            "HC074_FRAGMENT:write_cap={wcap}:read_cap={rcap}:bytes={}:writes={}:reads={}:fd_sends=1:fd_received=1:flushes=1",
            expected.len(),
            writes.len(),
            reads.len()
        );
        drop(file);
        drop(received);
        drop(message);
        f.clean();
    }
}

#[test]
fn cut_frames_refuse_and_release_received_fds() {
    for cut_kind in 0..4 {
        let f = Fixture::new();
        let message = f.message();
        let bytes = &message.data()[..];
        let cut = [1, 15, 16, bytes.len() - 1][cut_kind];
        let (a, b) = sockets();
        assert_eq!(
            fd_sendmsg(a.as_fd(), &bytes[..cut], &[f.file.as_fd()]).unwrap(),
            cut
        );
        a.shutdown(std::net::Shutdown::Write).unwrap();
        let (result, reads) = zbus::block_on(async {
            let mut reader = Reader {
                socket: registered(b),
                cap: 3,
                calls: vec![],
            };
            let result =
                tokio::time::timeout(LIMIT, reader.receive_message(74, &mut vec![], &mut vec![]))
                    .await
                    .unwrap();
            (result, reader.calls)
        });
        assert!(result.is_err());
        assert_eq!(reads.iter().map(|(n, _)| n).sum::<usize>(), cut);
        assert_eq!(reads.iter().map(|(_, n)| n).sum::<usize>(), 1);
        println!("HC074_CUT:cut={cut}:fd_received=1:message=false");
        drop(a);
        drop(message);
        f.clean();
    }
}

#[test]
fn midstream_write_failure_never_replays_fds_or_flushes() {
    let f = Fixture::new();
    let message = f.message();
    let (a, b) = sockets();
    let (sent, received, writes, reads, flushes) = zbus::block_on(async {
        let mut writer = Writer {
            socket: registered(a),
            cap: 3,
            calls: vec![],
            flushes: 0,
            break_at: Some(1),
        };
        let mut reader = Reader {
            socket: registered(b),
            cap: 2,
            calls: vec![],
        };
        let task = tokio::runtime::Handle::current().spawn(async move {
            let result =
                tokio::time::timeout(LIMIT, reader.receive_message(74, &mut vec![], &mut vec![]))
                    .await
                    .unwrap();
            (result, reader.calls)
        });
        let sent = tokio::time::timeout(LIMIT, writer.send_message(&message))
            .await
            .unwrap();
        let (received, reads) = tokio::time::timeout(LIMIT, task).await.unwrap().unwrap();
        (sent, received, writer.calls, reads, writer.flushes)
    });
    assert!(sent.is_err() && received.is_err());
    assert_eq!(writes, vec![(3, 1), (0, 0)]);
    assert_eq!(reads.iter().map(|(n, _)| n).sum::<usize>(), 3);
    assert_eq!(reads.iter().map(|(_, n)| n).sum::<usize>(), 1);
    assert_eq!(flushes, 0);
    println!("HC074_WRITE_FAILURE:bytes=3:fd_sends=1:flushes=0:message=false");
    drop(message);
    f.clean();
}

struct Restore(rustix::process::Rlimit);
impl Drop for Restore {
    fn drop(&mut self) {
        rustix::process::setrlimit(rustix::process::Resource::Nofile, self.0)
            .expect("restore own child soft limit");
    }
}
fn ctrunc_child() {
    let f = Fixture::new();
    let (a, b) = UnixStream::pair().unwrap();
    let borrowed = [f.file.as_fd(); 8];
    assert_eq!(FDS_MAX, 1024);
    assert_eq!(fd_sendmsg(a.as_fd(), &[74], &borrowed).unwrap(), 1);
    let mut byte = [0u8];
    OBSERVED_TRUNC.with(|v| v.set(false));
    let (n, fds) = fd_recvmsg(b.as_fd(), &mut byte).unwrap();
    assert_eq!(n, 1);
    assert_eq!(fds.len(), 8);
    assert!(!OBSERVED_TRUNC.with(|v| v.get()));
    drop(fds);
    assert_eq!(f.aliases(), 1);
    assert_eq!(fd_sendmsg(a.as_fd(), &[74], &borrowed).unwrap(), 1);
    let highest = fs::read_dir("/proc/self/fd")
        .unwrap()
        .map(|v| {
            v.unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        })
        .max()
        .unwrap();
    let old = rustix::process::getrlimit(rustix::process::Resource::Nofile);
    let limited = rustix::process::Rlimit {
        current: Some(highest + 1),
        maximum: old.maximum,
    };
    assert!(old.current.is_none_or(|n| highest + 1 < n));
    rustix::process::setrlimit(rustix::process::Resource::Nofile, limited).unwrap();
    let restore = Restore(old);
    OBSERVED_TRUNC.with(|v| v.set(false));
    let result = fd_recvmsg(b.as_fd(), &mut byte);
    let truncated = OBSERVED_TRUNC.with(|v| v.get());
    drop(restore);
    assert_eq!(
        rustix::process::getrlimit(rustix::process::Resource::Nofile),
        old
    );
    let (rejected, received) = match result {
        Ok((_, fds)) => {
            let n = fds.len();
            drop(fds);
            (false, n)
        }
        Err(e) => {
            assert_eq!(e.kind(), io::ErrorKind::InvalidData);
            (true, 0)
        }
    };
    assert!(
        truncated,
        "kernel did not report MSG_CTRUNC; setup is not a product verdict"
    );
    assert_eq!(f.aliases(), 1);
    f.clean();
    println!(
        "HC074_CTRUNC:kernel_flag=true:limit_restored=true:sent=8:returned={received}:rejected={rejected}:aliases=1:capacity={FDS_MAX}"
    );
    assert!(rejected, "truncated ancillary data was accepted");
}
#[test]
fn ctrunc_rejected_and_descriptors_retired() {
    if let Some(flag) = std::env::var_os("HC074_FRAGMENT_CHILD") {
        assert_eq!(flag, "ctrunc");
        ctrunc_child();
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "io_leaf::tests::ctrunc_rejected_and_descriptors_retired",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HC074_FRAGMENT_CHILD", "ctrunc")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    println!(
        "HC074_CTRUNC_CHILD_STDOUT:{:?}",
        String::from_utf8(output.stdout).unwrap()
    );
    println!(
        "HC074_CTRUNC_CHILD_STDERR:{:?}",
        String::from_utf8(output.stderr).unwrap()
    );
    assert!(
        output.status.success(),
        "isolated child refused: {}",
        output.status
    );
}

fn registered(stream: UnixStream) -> AsyncFd<UnixStream> {
    // SAFETY: This fixture moves an owned, open, nonblocking UnixStream into
    // AsyncFd. Its FD is never replaced, closed by number, extracted or forgotten.
    // All syscalls borrow it through get_ref/as_fd; shutdown does not close it.
    unsafe { AsyncFd::register(stream) }.unwrap()
}
