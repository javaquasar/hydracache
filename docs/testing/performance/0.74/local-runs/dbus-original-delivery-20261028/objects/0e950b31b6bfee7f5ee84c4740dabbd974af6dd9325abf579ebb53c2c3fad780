use super::*;
use std::os::fd::AsFd;

fn fixture(run: impl FnOnce(&File, &File)) {
    let fence = tempfile::tempfile().unwrap();
    assert_eq!(unsafe { libc::flock(fence.as_raw_fd(), libc::LOCK_EX) }, 0);
    let mut now: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut now) },
        0
    );
    let deadline = now.tv_sec as u64 * 1_000_000_000 + now.tv_nsec as u64 + 10_000_000_000;
    let timer = alarm(deadline).unwrap();
    run(&fence, &timer);
}
#[test]
fn inherited_pending_layout_refuses_every_duplicate_and_negative_role() {
    assert_eq!(keep_roles([9, 3, 11, 5]).unwrap(), [3, 5, 9, 11]);
    for i in 0..4 {
        let mut fds = [3, 5, 9, 11];
        fds[i] = -1;
        assert!(keep_roles(fds).is_err());
        for j in 0..4 {
            if i != j {
                let mut fds = [3, 5, 9, 11];
                fds[i] = fds[j];
                assert!(keep_roles(fds).is_err());
            }
        }
    }
}
#[test]
fn inherited_pending_uses_same_timer_ofd_and_excludes_unrelated_writers() {
    fixture(|fence, timer| {
        let extra = tempfile::tempfile().unwrap();
        let pair = InheritedPending::spawn(fence.as_fd(), timer.as_fd(), 0).unwrap();
        pair.observe().unwrap();
        let pins: Vec<File> = pair
            .guardians
            .iter()
            .map(|g| g.0.pinned.try_clone().unwrap())
            .collect();
        let flags = unsafe { libc::fcntl(timer.as_raw_fd(), libc::F_GETFL) };
        // Owned fixture only: prove shared OFD status without consuming/rearming.
        assert_eq!(
            unsafe { libc::fcntl(timer.as_raw_fd(), libc::F_SETFL, flags & !libc::O_NONBLOCK) },
            0
        );
        for guardian in &pair.guardians {
            assert_eq!(
                unsafe { libc::fcntl(guardian.0.alarm.as_raw_fd(), libc::F_GETFL) },
                flags & !libc::O_NONBLOCK
            );
            let root = format!("/proc/{}/fd", guardian.0.pid);
            let entries: Vec<_> = std::fs::read_dir(&root).unwrap().collect();
            assert_eq!(entries.len(), 4);
            assert!(std::fs::symlink_metadata(format!("{root}/{}", extra.as_raw_fd())).is_err());
            let metadata = std::fs::metadata(format!("{root}/{}", fence.as_raw_fd())).unwrap();
            assert_eq!(metadata.ino(), fence.metadata().unwrap().ino());
        }
        assert_eq!(
            unsafe { libc::fcntl(timer.as_raw_fd(), libc::F_SETFL, flags) },
            0
        );
        drop(pair);
        for pin in pins {
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            assert_eq!(
                unsafe {
                    libc::waitid(
                        libc::P_PIDFD,
                        pin.as_raw_fd() as _,
                        &mut info,
                        libc::WEXITED | libc::WNOHANG,
                    )
                },
                -1
            );
            assert_eq!(unsafe { *libc::__errno_location() }, libc::ECHILD);
        }
        assert_eq!(
            unsafe { libc::fcntl(fence.as_raw_fd(), libc::F_GETFD) },
            libc::FD_CLOEXEC
        );
    });
}
#[test]
fn inherited_pending_partial_clone_and_restore_failures_reap_only_owned_children() {
    fixture(|fence, timer| {
        let children = || std::fs::read_to_string("/proc/thread-self/children").unwrap();
        let mask = || {
            let mut mask = 0u64;
            assert_eq!(
                unsafe {
                    libc::syscall(
                        libc::SYS_rt_sigprocmask,
                        libc::SIG_SETMASK,
                        0usize,
                        &mut mask,
                        8usize,
                    )
                },
                0
            );
            mask
        };
        for fault in 1..=4 {
            let before = children();
            let before_mask = mask();
            assert!(InheritedPending::spawn(fence.as_fd(), timer.as_fd(), fault).is_err());
            assert_eq!(children(), before, "fault {fault}");
            assert_eq!(mask(), before_mask, "signal mask changed in fault {fault}");
        }
    });
}
#[test]
fn inherited_pending_refuses_a_different_current_thread() {
    fixture(|fence, timer| {
        let pending = InheritedPending::spawn(fence.as_fd(), timer.as_fd(), 0).unwrap();
        std::thread::spawn(move || assert!(pending.observe().is_err()))
            .join()
            .unwrap();
    });
}
