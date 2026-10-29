//! Isolated local-root pair mechanics, NOT original owner death or admission.
use super::super::super::super::RootBindingClaims;
use super::super::super::guardian::RootPair;
use super::*;

fn now() -> u64 {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    assert_eq!(
        unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) },
        0
    );
    t.tv_sec as u64 * 1_000_000_000 + t.tv_nsec as u64
}
fn alarm(deadline: u64) -> File {
    let fd = unsafe {
        libc::timerfd_create(libc::CLOCK_BOOTTIME, libc::TFD_NONBLOCK | libc::TFD_CLOEXEC)
    };
    assert!(fd >= 0);
    let file = unsafe { File::from_raw_fd(fd) };
    let value = libc::itimerspec {
        it_interval: libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        it_value: libc::timespec {
            tv_sec: (deadline / 1_000_000_000) as _,
            tv_nsec: (deadline % 1_000_000_000) as _,
        },
    };
    assert_eq!(
        unsafe { libc::timerfd_settime(fd, libc::TFD_TIMER_ABSTIME, &value, std::ptr::null_mut()) },
        0
    );
    file
}
struct LocalPair<'a> {
    pair: RootPair<'a>,
    cleanup_fence: File,
}
impl Drop for LocalPair<'_> {
    fn drop(&mut self) {
        // Fixture cleanup only, including assertion failure. No runtime receipt.
        self.pair.cleanup_for_fixture();
        assert_eq!(
            unsafe {
                libc::flock(
                    self.cleanup_fence.as_raw_fd(),
                    libc::LOCK_EX | libc::LOCK_NB,
                )
            },
            0
        );
    }
}
fn binding(root: &PinnedRoot, deadline: u64) -> RootBindingClaims {
    RootBindingClaims {
        attempt_sha256: [3; 32],
        original_context_sha256: [4; 32],
        root_device: root.identity.device,
        root_inode: root.identity.inode,
        deadline_boottime_ns: deadline,
    }
}
fn prepare<'a>(
    fixture: &mut Fixture,
    root: &'a mut PinnedRoot,
    deadline: u64,
    fault: u8,
) -> LocalPair<'a> {
    let timer = alarm(deadline);
    let expected = binding(root, deadline);
    let pair =
        RootPair::spawn(fixture.fence.as_fd(), timer.as_fd(), root, expected, fault).unwrap();
    let replacement = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.temporary.path().join("fence"))
        .unwrap();
    let cleanup_fence = replacement.try_clone().unwrap();
    let old = std::mem::replace(&mut fixture.fence, replacement);
    let result = LocalPair {
        pair,
        cleanup_fence,
    };
    // Neither ordinary parent copy owns the original locked OFD now. Only the
    // isolated guardians keep it; cleanup_fence is a different open description.
    drop(old);
    result
}
fn locked(fixture: &Fixture) {
    let other = OpenOptions::new()
        .read(true)
        .write(true)
        .open(fixture.temporary.path().join("fence"))
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(other.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        -1
    );
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::EWOULDBLOCK)
    );
}
fn elapsed(deadline: u64) {
    while now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn target(fixture: &mut Fixture, nested: bool, output: Option<File>) -> usize {
    let mut command = Command::new("/bin/cat");
    command
        .stdin(Stdio::piped())
        .stdout(output.map_or_else(Stdio::null, Stdio::from))
        .stderr(Stdio::null())
        .uid(65534)
        .gid(65534);
    let owned = OwnedChild::new(command);
    let directory = if nested {
        if !fixture.nested {
            std::fs::create_dir(fixture.path.join("nested")).unwrap();
            fixture.nested = true;
        }
        fixture.path.join("nested")
    } else {
        fixture.path.clone()
    };
    let procs = OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(directory.join("cgroup.procs"))
        .unwrap();
    let pid = owned.child.id().to_string();
    assert_eq!(procs.write_at(pid.as_bytes(), 0).unwrap(), pid.len());
    // A live revoker can kill immediately after successful migration. Do not
    // demand a later /proc membership snapshot from an already dying process.
    fixture.children.push(owned);
    fixture.children.len() - 1
}
#[test]
#[ignore = "manual isolated local-root guardian pair; no service or product"]
fn cgroup_root_guardian_fault_matrix() {
    assert_eq!(
        std::env::var("HC074_ROOT_GUARDIAN_FIXTURE").as_deref(),
        Ok("local-only-v1")
    );
    assert_eq!(unsafe { libc::geteuid() }, 0);
    eprintln!("HC074-ROOT-GUARDIAN seed=0x074c2021 local_only=true original_authority=false");
    for fault in 3..=6 {
        let fixture = Fixture::new();
        let mut root = fixture.pin();
        let deadline = now() + 30_000_000_000;
        let timer = alarm(deadline);
        let expected = binding(&root, deadline);
        let children = format!("/proc/self/task/{}/children", unsafe {
            libc::syscall(libc::SYS_gettid)
        });
        let before = std::fs::read_to_string(&children).unwrap();
        let mut mask_before = 0u64;
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_rt_sigprocmask,
                    libc::SIG_SETMASK,
                    0usize,
                    &mut mask_before,
                    8usize,
                )
            },
            0
        );
        assert!(RootPair::spawn(
            fixture.fence.as_fd(),
            timer.as_fd(),
            &mut root,
            expected,
            fault
        )
        .is_err());
        let mut mask_after = 0u64;
        assert_eq!(
            unsafe {
                libc::syscall(
                    libc::SYS_rt_sigprocmask,
                    libc::SIG_SETMASK,
                    0usize,
                    &mut mask_after,
                    8usize,
                )
            },
            0
        );
        assert_eq!(mask_after, mask_before);
        assert_eq!(std::fs::read_to_string(&children).unwrap(), before);
        eprintln!("HC074-ROOT-GUARDIAN case=construction-{fault} exact_children_reaped=true signal_mask_unchanged=true");
    }
    for case in 0..12 {
        let mut fixture = Fixture::new();
        let mut root = fixture.pin();
        // Open before clone: EOF also proves no unrelated inherited writer is
        // left in a guardian. An explicit outside alias remains a negative.
        let mut pipe = if case == 9 {
            let mut fds = [-1; 2];
            assert_eq!(
                unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) },
                0
            );
            Some(unsafe { [File::from_raw_fd(fds[0]), File::from_raw_fd(fds[1])] })
        } else {
            None
        };
        let duration = if matches!(case, 1 | 8 | 9) {
            30_000_000_000
        } else {
            2_000_000_000
        };
        let deadline = now() + duration; // fixture-only absolute bound
        let fault = match case {
            6 => 1,
            7 => 2,
            _ => 0,
        };
        let mut local = prepare(&mut fixture, &mut root, deadline, fault);
        let armed = local.pair.enroll_once();
        if matches!(case, 6 | 7) {
            assert!(armed.is_err());
            assert_eq!(local.pair.state_for_fixture(), (true, false, true));
            assert!(local.pair.enroll_once().is_err());
        } else {
            armed.unwrap();
            assert_eq!(local.pair.state_for_fixture(), (true, true, false));
            for pid in local.pair.pids_for_fixture() {
                let paths: Vec<_> = std::fs::read_dir(format!("/proc/{pid}/fd"))
                    .unwrap()
                    .map(|entry| std::fs::read_link(entry.unwrap().path()).unwrap())
                    .collect();
                assert_eq!(
                    paths.len(),
                    9,
                    "initial five, three rights, one independent watch"
                );
                assert_eq!(
                    paths
                        .iter()
                        .filter(|p| p.file_name() == Some(std::ffi::OsStr::new("cgroup.events")))
                        .count(),
                    2
                );
            }
        }
        let hidden = pipe.as_ref().map(|p| p[1].try_clone().unwrap());
        let output = pipe.as_mut().map(|p| p[1].try_clone().unwrap());
        let reader = pipe.take().map(|[r, w]| {
            drop(w);
            r
        });
        let direct = target(&mut fixture, false, output);
        let nested = target(&mut fixture, true, None);
        match case {
            1 | 9 => local.pair.disconnect_for_fixture(),
            2 | 3 => local.pair.signal_for_fixture(case - 2, libc::SIGKILL),
            4 | 5 => local.pair.signal_for_fixture(case - 4, libc::SIGSTOP),
            8 => {
                assert!(local.pair.enroll_once().is_err());
            }
            10 => {
                local.pair.signal_for_fixture(0, libc::SIGKILL);
                local.pair.signal_for_fixture(1, libc::SIGKILL);
            }
            11 => {
                local.pair.signal_for_fixture(0, libc::SIGSTOP);
                local.pair.signal_for_fixture(1, libc::SIGSTOP);
            }
            _ => {}
        }
        if matches!(case, 10 | 11) {
            elapsed(deadline);
            assert!(fixture.children[direct].live() && fixture.children[nested].live());
            if case == 10 {
                assert!(local.pair.terminal_for_fixture(0) && local.pair.terminal_for_fixture(1));
                assert_eq!(
                    unsafe {
                        libc::flock(fixture.fence.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)
                    },
                    0
                );
            }
            if case == 11 {
                locked(&fixture);
            }
            eprintln!("HC074-ROOT-GUARDIAN case={case} expected_negative=true all_writers=false");
        } else {
            fixture.children[direct].killed();
            fixture.children[nested].killed();
            locked(&fixture);
            if case == 1 {
                for wave in 0..8 {
                    let late = target(&mut fixture, wave % 2 == 0, None);
                    fixture.children[late].killed();
                    locked(&fixture);
                }
            }
            if let Some(reader) = reader {
                let mut byte = [0u8];
                assert_eq!(
                    unsafe { libc::read(reader.as_raw_fd(), byte.as_mut_ptr().cast(), 1) },
                    -1
                );
                assert_eq!(io::Error::last_os_error().kind(), io::ErrorKind::WouldBlock);
                drop(hidden);
                assert_eq!(
                    unsafe { libc::read(reader.as_raw_fd(), byte.as_mut_ptr().cast(), 1) },
                    0
                );
            }
            eprintln!("HC074-ROOT-GUARDIAN case={case} actual_target_sigkill=true fence_guardian_only=true completion=false");
        }
        drop(local); // exact controlled fixture cleanup, never production proof
    }
    eprintln!("HC074-ROOT-GUARDIAN completed=true cases=16 creator_sigkill_tested=false original_armed=false completion=false");
}

#[path = "diagnostic_root_guardian_original_kernel_fault_tests.rs"]
mod original_adapter;
