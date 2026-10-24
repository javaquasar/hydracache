#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

fn base() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_diagnostic_signer_isolation_probe"));
    command
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // SAFETY: only the fresh child's descriptor flags change, before exec and
    // before any fixture can clear CLOEXEC on its one explicit negative-control FD.
    // Keep Rust's exec-error pipe open until exec; never close or mutate parent FDs.
    unsafe {
        command.pre_exec(|| {
            if libc::syscall(
                libc::SYS_close_range,
                3u32,
                u32::MAX,
                libc::CLOSE_RANGE_CLOEXEC,
            ) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command
}
fn command(case: &str) -> Command {
    let mut command = base();
    command.arg(case);
    command
}
// Own exactly this Child; no PID adoption, global kill or detached process cleanup.
fn bounded(mut child: Child) -> Output {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!("owned fixed probe timed out");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn refused(output: Output) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}
fn passed() {
    let output = bounded(command("native").spawn().unwrap());
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        b"fixed child isolation probe passed; admission=false\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn ordinary_child_native_denials_and_positive_unfiltered_control() {
    // SAFETY: the owned fork child only performs async-signal-safe chdir/_exit;
    // it never allocates, touches Rust locks, execs or runs a workload.
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0);
    if pid == 0 {
        unsafe {
            libc::_exit(if libc::chdir(c"/".as_ptr()) == 0 {
                0
            } else {
                1
            })
        }
    }
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let mut status = 0;
        // SAFETY: wait only for the exact owned child, without blocking the test harness.
        let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if waited == pid {
            assert!(libc::WIFEXITED(status));
            assert_eq!(libc::WEXITSTATUS(status), 0);
            break;
        }
        assert_eq!(waited, 0);
        if Instant::now() >= until {
            // SAFETY: this unreaped owned child cannot have had its PID reused.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
                libc::waitpid(pid, &mut status, 0);
            }
            panic!("owned control timed out");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    passed();
    println!("unfiltered_chdir=0 fixed_native_probes=20 exact_EPERM=true admission=false");
}

#[test]
fn ordinary_child_x32_and_i386_die_sigsys_without_output_or_core() {
    for case in ["x32", "i386"] {
        let output = bounded(command(case).spawn().unwrap());
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGSYS),
            "{case}: {output:?}"
        );
        assert!(!output.status.core_dumped());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        println!("{case}=SIGSYS core=false stdout=empty admission=false");
    }
}

#[test]
fn ordinary_child_refuses_environment_and_unknown_or_extra_arguments() {
    refused(bounded(
        command("native")
            .env("HYDRACACHE_FIXED_PROBE_UNTRUSTED", "1")
            .spawn()
            .unwrap(),
    ));
    refused(bounded(command("--exec").spawn().unwrap()));
    refused(bounded(command("native").arg("x32").spawn().unwrap()));
    let mut missing = base();
    refused(bounded(missing.spawn().unwrap()));
}

#[test]
fn ordinary_child_refuses_extra_inherited_descriptor_without_parent_flag_mutation() {
    let file = File::open("/dev/null").unwrap();
    let fd = file.as_raw_fd();
    // SAFETY: read only this parent-owned descriptor's flags.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    assert!(flags >= 0);
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
    let mut cmd = command("native");
    // SAFETY: the post-fork closure only calls fcntl on this child's descriptor copy.
    unsafe {
        cmd.pre_exec(move || {
            if libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    refused(bounded(cmd.spawn().unwrap()));
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, flags);
}

fn parent_state() -> (String, (libc::rlim_t, libc::rlim_t), std::path::PathBuf) {
    let raw = fs::read_to_string("/proc/thread-self/status").unwrap();
    let status = raw
        .lines()
        .filter(|line| {
            ["NoNewPrivs:", "Seccomp:", "Seccomp_filters:"]
                .iter()
                .any(|name| line.starts_with(name))
        })
        .collect::<Vec<_>>()
        .join("\n");
    // SAFETY: read only this test parent's core limits.
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut limit) }, 0);
    (
        status,
        (limit.rlim_cur, limit.rlim_max),
        std::env::current_dir().unwrap(),
    )
}

#[test]
fn ordinary_child_four_independent_owners_leave_parent_unchanged() {
    let before = parent_state();
    let owners: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                passed();
                passed();
            })
        })
        .collect();
    for owner in owners {
        owner.join().unwrap();
    }
    assert_eq!(before, parent_state());
    println!("owned_native_children=8 parent_restrictions_and_cwd_unchanged=true");
}
