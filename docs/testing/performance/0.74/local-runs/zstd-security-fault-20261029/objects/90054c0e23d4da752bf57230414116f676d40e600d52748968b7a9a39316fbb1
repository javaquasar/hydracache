use super::*;

fn busy(root: &Path) {
    assert!(matches!(
        lock_host_root(root),
        Err(HostExecutionError::Busy)
    ));
    assert!(matches!(
        HostExecutionClaim::acquire(root, &"d".repeat(64)),
        Err(HostExecutionError::Busy)
    ));
}

#[test]
fn ordinary_drop_still_unlocks_a_retained_alias() {
    let (temp, uid, gid) = fixture();
    let root = campaigns(temp.path());
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
    let read = bind_original_host_fence(&mut state_read).unwrap();
    let alias = read.fence.lock.try_clone().unwrap();
    busy(&root);
    drop(read);
    // This default is intentionally different from transferred execution custody.
    let short = lock_host_root(&root).unwrap();
    drop(alias);
    busy(&root);
    drop(short);
    assert!(lock_host_root(&root).is_ok());
    state_read.revalidate(&state()).unwrap();
}

#[test]
fn custody_survives_original_drop() {
    let (temp, uid, gid) = fixture();
    let root = campaigns(temp.path());
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
    let mut read = bind_original_host_fence(&mut state_read).unwrap();
    let parent = File::open(at(temp.path(), 3)).unwrap();
    let held = read.fence.retain_custody(&parent).unwrap();
    assert_eq!(
        unsafe { libc::fcntl(held._lock.as_raw_fd(), libc::F_GETFD) },
        libc::FD_CLOEXEC
    );
    read.revalidate().unwrap();
    drop(read);
    busy(&root);
    drop(held);
    assert!(lock_host_root(&root).is_ok());
    state_read.revalidate(&state()).unwrap();
}

#[test]
fn custody_first_drop_keeps_original_checks_and_refuses_a_second_transfer() {
    let (temp, uid, gid) = fixture();
    let root = campaigns(temp.path());
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
    let mut read = bind_original_host_fence(&mut state_read).unwrap();
    let parent = File::open(at(temp.path(), 3)).unwrap();
    let held = read.fence.retain_custody(&parent).unwrap();
    assert!(matches!(
        read.fence.retain_custody(&parent),
        Err(OriginalHostFenceError::Refused)
    ));
    drop(held);
    busy(&root);
    read.revalidate().unwrap();
    assert!(matches!(
        read.fence.retain_custody(&parent),
        Err(OriginalHostFenceError::Refused)
    ));
    drop(read);
    assert!(lock_host_root(&root).is_ok());
}

#[test]
fn custody_cannot_adopt_a_replaced_named_lock() {
    let (temp, uid, gid) = fixture();
    let root = campaigns(temp.path());
    let mut storage = walk(temp.path(), uid, gid).unwrap();
    let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
    let mut read = bind_original_host_fence(&mut state_read).unwrap();
    let alias = read.fence.lock.try_clone().unwrap();
    let parent = File::open(at(temp.path(), 3)).unwrap();
    let name = root.join(HOST_EXECUTION_LOCK_NAME);
    fs::rename(&name, temp.path().join("original-lock")).unwrap();
    fs::write(&name, []).unwrap();
    fs::set_permissions(&name, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(
        read.fence.retain_custody(&parent),
        Err(OriginalHostFenceError::Object)
    ));
    assert!(!read.fence.custody_retained);
    assert!(read.revalidate().is_err());
    drop(read);
    alias.try_lock_exclusive().unwrap();
    FileExt::unlock(&alias).unwrap();
    assert!(state_read.is_refused());
}

// Only the manual fixture consumes this private owner. This holder has no
// worker, timer, collector or completion authority and never executes a product.
struct Holder {
    pid: libc::pid_t,
    pidfd: Option<File>,
    control: File,
    ack: File,
}
fn socket_pair() -> [File; 2] {
    use std::os::fd::FromRawFd;
    let mut fds = [-1; 2];
    assert_eq!(
        unsafe {
            libc::socketpair(
                libc::AF_UNIX,
                libc::SOCK_SEQPACKET | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
                fds.as_mut_ptr(),
            )
        },
        0
    );
    unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] }
}
fn wait_readable(file: &File, milliseconds: i32) -> i32 {
    let mut poll = libc::pollfd {
        fd: file.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let n = unsafe { libc::poll(&mut poll, 1, milliseconds) };
    assert!(n >= 0);
    n
}
fn receive(file: &File) -> (isize, u8) {
    let mut byte = 0;
    let n = unsafe {
        libc::recv(
            file.as_raw_fd(),
            (&mut byte as *mut u8).cast(),
            1,
            libc::MSG_TRUNC,
        )
    };
    (n, byte)
}
fn send(file: &File, byte: u8) {
    assert_eq!(
        unsafe {
            libc::send(
                file.as_raw_fd(),
                (&byte as *const u8).cast(),
                1,
                libc::MSG_NOSIGNAL,
            )
        },
        1
    );
}
impl Holder {
    fn spawn(custody: FenceCustody, notify: bool, disconnected: bool) -> Self {
        use std::os::fd::FromRawFd;
        let [control, child_control] = socket_pair();
        let [ack, child_ack] = socket_pair();
        if disconnected {
            assert_eq!(unsafe { libc::shutdown(ack.as_raw_fd(), libc::SHUT_RD) }, 0);
        }
        let mut keep = [
            custody._lock.as_raw_fd(),
            child_control.as_raw_fd(),
            child_ack.as_raw_fd(),
        ];
        keep.sort_unstable();
        let full = u64::MAX;
        let mut previous = 0u64;
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_rt_sigprocmask,
                    libc::SIG_SETMASK,
                    &full,
                    &mut previous,
                    8usize,
                )
            },
            0
        );
        let pid = unsafe { libc::syscall(libc::SYS_fork) } as libc::pid_t;
        if pid == 0 {
            // No allocator, destructors, Rust IO/validation, panic, atfork hooks,
            // signal handler or library mutex is allowed in this raw-fork child.
            let mut start = 0u32;
            for fd in keep {
                if start < fd as u32
                    && unsafe { libc::syscall(libc::SYS_close_range, start, fd as u32 - 1, 0u32) }
                        != 0
                {
                    unsafe { libc::_exit(81) };
                }
                start = fd as u32 + 1;
            }
            if unsafe { libc::syscall(libc::SYS_close_range, start, u32::MAX, 0u32) } != 0 {
                unsafe { libc::_exit(82) };
            }
            let a = b'A';
            if notify {
                unsafe {
                    libc::send(
                        child_ack.as_raw_fd(),
                        (&a as *const u8).cast(),
                        1,
                        libc::MSG_NOSIGNAL,
                    )
                };
            }
            // A test-only structural barrier AFTER the optional ACK attempt.
            // It is never an original readiness or start acknowledgement.
            let b = b'B';
            if unsafe {
                libc::send(
                    child_control.as_raw_fd(),
                    (&b as *const u8).cast(),
                    1,
                    libc::MSG_NOSIGNAL,
                )
            } != 1
            {
                unsafe { libc::_exit(83) };
            }
            let mut p = libc::pollfd {
                fd: child_control.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            if unsafe { libc::poll(&mut p, 1, -1) } != 1 {
                unsafe { libc::_exit(84) };
            }
            let mut byte = 0u8;
            if unsafe {
                libc::recv(
                    child_control.as_raw_fd(),
                    (&mut byte as *mut u8).cast(),
                    1,
                    libc::MSG_TRUNC,
                )
            } != 1
                || byte != b'Q'
            {
                unsafe { libc::_exit(85) };
            }
            unsafe { libc::_exit(0) };
        }
        let restore = unsafe {
            libc::syscall(
                libc::SYS_rt_sigprocmask,
                libc::SIG_SETMASK,
                &previous,
                0usize,
                8usize,
            )
        };
        assert!(pid > 0, "raw fork refused");
        // Own the unreaped child before any subsequent assertion can fail.
        let mut held = Self {
            pid,
            pidfd: None,
            control,
            ack,
        };
        assert_eq!(restore, 0);
        let mut current = 0u64;
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_rt_sigprocmask,
                    libc::SIG_SETMASK,
                    0usize,
                    &mut current,
                    8usize,
                )
            },
            0
        );
        assert_eq!(current, previous);
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        assert!(fd >= 0);
        held.pidfd = Some(unsafe { File::from_raw_fd(fd as i32) });
        drop(child_control);
        drop(child_ack);
        drop(custody);
        assert_eq!(wait_readable(&held.control, 5000), 1);
        assert_eq!(receive(&held.control), (1, b'B'));
        if disconnected {
            assert_eq!(receive(&held.ack).0, 0);
        } else if notify {
            assert_eq!(receive(&held.ack), (1, b'A'));
        } else {
            assert_eq!(receive(&held.ack).0, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EAGAIN)
            );
        }
        assert_eq!(wait_readable(held.pidfd.as_ref().unwrap(), 0), 0);
        held
    }
    fn finish(&mut self, killed: bool) {
        let fd = self.pidfd.as_ref().unwrap();
        if killed {
            assert_eq!(
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        fd.as_raw_fd(),
                        libc::SIGKILL,
                        0usize,
                        0u32,
                    )
                },
                0
            );
        } else {
            send(&self.control, b'Q');
        }
        assert_eq!(wait_readable(fd, 5000), 1);
        let mut status = 0;
        assert_eq!(unsafe { libc::waitpid(self.pid, &mut status, 0) }, self.pid);
        self.pid = 0;
        if killed {
            assert!(libc::WIFSIGNALED(status));
            assert_eq!(libc::WTERMSIG(status), libc::SIGKILL);
        } else {
            assert!(libc::WIFEXITED(status));
            assert_eq!(libc::WEXITSTATUS(status), 0);
        }
    }
}
impl Drop for Holder {
    fn drop(&mut self) {
        if self.pid == 0 {
            return;
        }
        if let Some(fd) = &self.pidfd {
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                )
            };
        } else {
            // Exact direct unreaped child only; its PID cannot have been reused.
            unsafe { libc::kill(self.pid, libc::SIGKILL) };
        }
        unsafe { libc::waitpid(self.pid, std::ptr::null_mut(), 0) };
    }
}

#[test]
#[ignore = "manual nonroot original-fence custody process faults; no product execution"]
fn original_fence_custody_matrix() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    assert_ne!(unsafe { libc::getegid() }, 0);
    let seed = 0x0740_c517_u64;
    for repeat in 1..=3 {
        for case in [
            "close",
            "ack-omitted",
            "ack-disconnected",
            "holder-kill",
            "original-last",
            "repeat-refused",
        ] {
            let (temp, uid, gid) = fixture();
            let root = campaigns(temp.path());
            let before = fs::read(at(temp.path(), 4).join("active-diagnostic.json")).unwrap();
            let mut storage = walk(temp.path(), uid, gid).unwrap();
            let mut state_read = bind_fixed_reserved_state(&mut storage, &state()).unwrap();
            let mut read = bind_original_host_fence(&mut state_read).unwrap();
            let parent = File::open(at(temp.path(), 3)).unwrap();
            let custody = read.fence.retain_custody(&parent).unwrap();
            let mut holder =
                Holder::spawn(custody, case != "ack-omitted", case == "ack-disconnected");
            busy(&root);
            read.revalidate().unwrap();
            if case == "repeat-refused" {
                assert!(matches!(
                    read.fence.retain_custody(&parent),
                    Err(OriginalHostFenceError::Refused)
                ));
            }
            if case == "original-last" {
                holder.finish(false);
                busy(&root);
                read.revalidate().unwrap();
                drop(read);
            } else {
                drop(read);
                busy(&root);
                assert_eq!(wait_readable(holder.pidfd.as_ref().unwrap(), 0), 0);
                holder.finish(case == "holder-kill");
            }
            assert!(lock_host_root(&root).is_ok());
            state_read.revalidate(&state()).unwrap();
            assert_eq!(
                fs::read(at(temp.path(), 4).join("active-diagnostic.json")).unwrap(),
                before
            );
            println!("HC074-FENCE-CUSTODY seed=0x{seed:08x} repeat={repeat} case={case} original=true terminal=true retained_state=true");
        }
    }
    println!("HC074-FENCE-CUSTODY completed=true cases=18 product=false");
}
