//! Synthetic FD ownership controls, isolated from concurrent libtest FD churn.
use super::*;
use std::collections::BTreeSet;
use std::fs::File;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::process::{Child, Command, Stdio};
use std::task::{Context, Poll, Waker};
use std::time::Instant;

const CHILD_ENV: &str = "HC074_ALARM_SECURITY_CHILD";
const CASE_BOUND: Duration = Duration::from_secs(5);

struct OwnedChild(Option<Child>);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

fn isolated(name: &str, body: impl FnOnce()) {
    if std::env::var(CHILD_ENV).as_deref() == Ok(name) {
        body();
        return;
    }
    // A fresh one-test process makes exact FD closure/reuse assertions independent
    // of other libtests. It creates no descendant, manager connection or service.
    let path = module_path!().split_once("::").unwrap().1;
    let mut owned = OwnedChild(Some(
        Command::new(std::env::current_exe().unwrap())
            .args([
                &format!("{path}::{name}"),
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_ENV, name)
            .env("RUST_BACKTRACE", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    let start = Instant::now();
    loop {
        if owned.0.as_mut().unwrap().try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            start.elapsed() < CASE_BOUND,
            "isolated alarm case timed out: {name}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let output = owned.0.take().unwrap().wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{name}: {stdout}\n{stderr}");
    assert!(
        stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;"),
        "{name}: {stdout}"
    );
}

fn runtime() -> Runtime {
    Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
}

fn descriptors() -> BTreeSet<RawFd> {
    let scan = std::path::PathBuf::from(format!("/proc/{}/fd", std::process::id()));
    std::fs::read_dir("/proc/self/fd")
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().parse().unwrap(),
                std::fs::read_link(entry.path()).unwrap(),
            )
        })
        .filter_map(|(fd, link)| (link != scan).then_some(fd))
        .collect()
}

fn flags(fd: RawFd) -> i32 {
    unsafe { libc::fcntl(fd, libc::F_GETFD) }
}

#[test]
fn manager_alarm_owned_alias_lifetime() {
    isolated("manager_alarm_owned_alias_lifetime", || {
        let runtime = runtime();
        let _entered = runtime.enter();
        let op = Operation::new().unwrap();
        let alias = op.file.try_clone().unwrap();
        let alarm = register_operation_alarm(&op.file).unwrap();
        let fd = alarm.get_ref().as_raw_fd();
        assert_ne!(fd, op.file.as_raw_fd());
        assert_ne!(fd, alias.as_raw_fd());
        assert_eq!(flags(fd), libc::FD_CLOEXEC);
        assert_ne!(
            unsafe { libc::fcntl(fd, libc::F_GETFL) } & libc::O_NONBLOCK,
            0
        );
        drop(op);
        drop(alias);
        assert_eq!(flags(fd), libc::FD_CLOEXEC);
        drop(alarm);
        assert_eq!(flags(fd), -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBADF)
        );
    });
}

#[test]
fn manager_alarm_cancellation_and_fd_reuse() {
    isolated("manager_alarm_cancellation_and_fd_reuse", || {
        use std::io::{Read, Write};
        let seed = 0x0740_fd38_u64;
        let runtime = runtime();
        let _entered = runtime.enter();
        let op = Operation::new().unwrap();
        let origin = op.deadline;
        let (reader, mut writer) = std::os::unix::net::UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        let source = File::from(OwnedFd::from(reader));
        for iteration in 0..16 {
            let before = descriptors();
            let polled = std::cell::Cell::new(false);
            let mut work = Box::pin(op.bound(std::future::poll_fn(|_| {
                polled.set(true);
                Poll::<Result<()>>::Pending
            })));
            assert!(work
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending());
            assert!(polled.get());
            let during = descriptors();
            let added = during.difference(&before).copied().collect::<Vec<_>>();
            assert_eq!(added.len(), 1, "seed={seed:#x} iteration={iteration}");
            let retired = added[0];
            drop(work);
            assert_eq!(descriptors(), before);
            assert_eq!(flags(retired), -1);
            // The same numeric slot now denotes a distinct open file description.
            // The canceled alarm is already dropped; no live FD is closed/rebound.
            let next = register_operation_alarm(&source).unwrap();
            assert_eq!(next.get_ref().as_raw_fd(), retired);
            writer.write_all(&[74]).unwrap();
            drop(runtime.block_on(next.readable()).unwrap());
            let mut byte = [0];
            (&source).read_exact(&mut byte).unwrap();
            assert_eq!(byte, [74]);
            drop(next);
            assert_eq!(descriptors(), before);
            assert_eq!(op.deadline, origin);
        }
    });
}

#[test]
fn manager_alarm_registration_failure_keeps_original() {
    isolated("manager_alarm_registration_failure_keeps_original", || {
        let runtime = runtime();
        let _entered = runtime.enter();
        let original = tempfile::tempfile().unwrap();
        let before = descriptors();
        // epoll refuses a regular file. Both the old and new registration API
        // must drop only the failed private duplicate, never the source File.
        for _ in 0..16 {
            assert!(matches!(
                register_operation_alarm(&original),
                Err(ChannelError::Io)
            ));
            assert_eq!(flags(original.as_raw_fd()), libc::FD_CLOEXEC);
            assert_eq!(descriptors(), before);
        }
    });
}

#[test]
fn manager_alarm_unpolled_and_ready_paths_keep_original_origin() {
    isolated(
        "manager_alarm_unpolled_and_ready_paths_keep_original_origin",
        || {
            let runtime = runtime();
            let _entered = runtime.enter();
            let op = Operation::new().unwrap();
            let origin = op.deadline;
            let before = descriptors();
            let polled = std::cell::Cell::new(false);
            drop(Box::pin(op.bound(async {
                polled.set(true);
                Ok(())
            })));
            assert!(!polled.get());
            assert_eq!(descriptors(), before);
            assert_eq!(runtime.block_on(op.bound(async { Ok(74u32) })), Ok(74));
            assert_eq!(
                runtime.block_on(op.bound(async { Err::<(), _>(ChannelError::Changed) })),
                Err(ChannelError::Changed)
            );
            assert_eq!(descriptors(), before);
            assert_eq!(op.deadline, origin);
            assert_eq!(flags(op.file.as_raw_fd()), libc::FD_CLOEXEC);
        },
    );
}
