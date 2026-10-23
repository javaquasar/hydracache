#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "support/bootstrap_child_material.rs"]
mod bootstrap_child_material;
#[path = "support/bootstrap_census_material.rs"]
mod material;
use std::{
    fs::File,
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::process::CommandExt,
    },
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn attempt(f: &material::Fixture, extra: bool) -> String {
    // Remap ONLY this synthetic child; retain parent resources unchanged.
    let scratch: Vec<File> = f
        .fds()
        .into_iter()
        .map(|source| {
            let fd = unsafe { libc::fcntl(source, libc::F_DUPFD_CLOEXEC, 64) };
            assert!(fd >= 64);
            unsafe { File::from_raw_fd(fd) }
        })
        .collect();
    let numbers: Vec<_> = scratch.iter().map(AsRawFd::as_raw_fd).collect();
    let mut c = Command::new("/bin/sh");
    c.env_clear().args(["-c","unset PWD; LISTEN_PID=$$; export LISTEN_PID; exec \"$@\"","bootstrap-census-control"])
        .arg(env!("CARGO_BIN_EXE_diagnostic_held_bootstrap")).arg("--systemd-bootstrap-census-v1")
        .env("LISTEN_FDS","12").env("LISTEN_FDNAMES","release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry")
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    unsafe {
        c.pre_exec(move || {
            for (i, fd) in numbers.iter().enumerate() {
                if libc::dup2(*fd, i as i32) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            if extra && libc::dup2(numbers[4], 20) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::syscall(
                libc::SYS_close_range,
                15u32,
                u32::MAX,
                libc::CLOSE_RANGE_CLOEXEC,
            ) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            if extra && libc::fcntl(20, libc::F_SETFD, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = c.spawn().unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!("census child timeout; no retry");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(child.wait().unwrap().code(), Some(2));
    let mut raw = Vec::new();
    let mut file = &f.retained[2];
    let mut buffer = [0; 1024];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&buffer[..n]);
                assert!(raw.len() <= 8192);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => panic!("{e}"),
        }
    }
    let text = String::from_utf8(raw).unwrap();
    assert!(
        text.contains("no start, send or executable handoff"),
        "{text}"
    );
    text
}
#[test]
fn bootstrap_census_complete_forged_roles_reach_only_closed_authentication() {
    let f = material::Fixture::new();
    let text = attempt(&f, false);
    assert!(
        text.contains("original bootstrap authentication is closed"),
        "{text}"
    );
}
#[test]
fn bootstrap_census_extra_descriptor_and_wrong_image_refuse_before_authentication_boundary() {
    let f = material::Fixture::new();
    let text = attempt(&f, true);
    assert!(!text.contains("authentication is closed"), "{text}");
    let mut f = material::Fixture::new();
    f.entry.image_hashes[0][0] ^= 1;
    f.replace_entry();
    let text = attempt(&f, false);
    assert!(!text.contains("authentication is closed"), "{text}");
}
#[test]
fn bootstrap_census_dead_guardian_refuses_without_replacing_attempt() {
    let mut f = material::Fixture::new();
    f.children[0].kill().unwrap();
    f.children[0].wait().unwrap();
    let text = attempt(&f, false);
    assert!(!text.contains("authentication is closed"), "{text}");
}
