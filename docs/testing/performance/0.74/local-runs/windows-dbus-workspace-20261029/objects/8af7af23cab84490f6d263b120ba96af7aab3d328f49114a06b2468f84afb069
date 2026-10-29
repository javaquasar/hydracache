#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "support/bootstrap_child_material.rs"]
mod bootstrap_child_material;
#[path = "support/bootstrap_installed_controls.rs"]
mod installed_controls;
#[path = "support/bootstrap_census_material.rs"]
mod material;
use sha2::{Digest, Sha256};
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
const BINARY: &str = env!("CARGO_BIN_EXE_diagnostic_held_bootstrap");
fn fixture(raw: &[u8]) -> material::Fixture {
    let mut f = material::Fixture::new();
    f.files[13] = material::sealed(raw);
    assert_eq!(unsafe { libc::fchmod(f.files[13].as_raw_fd(), 0o500) }, 0);
    f.entry.identities[2] = material::identity(&f.files[13]);
    f.entry.image_hashes[2] = Sha256::digest(raw).into();
    f.replace_entry();
    f
}
fn attempt(f: &material::Fixture, matching: bool, extra: bool) -> String {
    // Private synthetic child only. This is not original-owner FD transport.
    let scratch: Vec<File> = f
        .fds()
        .into_iter()
        .map(|fd| {
            let n = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 64) };
            assert!(n >= 64);
            unsafe { File::from_raw_fd(n) }
        })
        .collect();
    let numbers: Vec<_> = scratch.iter().map(AsRawFd::as_raw_fd).collect();
    let mut c = Command::new("/bin/sh");
    c.env_clear().args(["-c", "unset PWD; LISTEN_PID=$$; export LISTEN_PID; exec \"$@\"", "bootstrap-image-control"])
        .arg(if matching { "/proc/self/fd/13" } else { BINARY }).arg("--systemd-bootstrap-image-v1")
        .env("LISTEN_FDS", "12").env("LISTEN_FDNAMES", "release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry")
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
            panic!("current-image child timeout; no retry");
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
fn current_image_matching_sealed_inode_still_cannot_authenticate_forged_context() {
    if crate::owned_case::isolate(
        module_path!(),
        "current_image_matching_sealed_inode_still_cannot_authenticate_forged_context",
    ) {
        return;
    }
    let raw = std::fs::read(BINARY).unwrap();
    let f = fixture(&raw);
    let text = attempt(&f, true, false);
    assert!(
        text.contains("original bootstrap authentication is closed"),
        "{text}"
    );
}
#[test]
fn current_image_identical_bytes_at_different_inode_refuse_before_authority_boundary() {
    if crate::owned_case::isolate(
        module_path!(),
        "current_image_identical_bytes_at_different_inode_refuse_before_authority_boundary",
    ) {
        return;
    }
    let raw = std::fs::read(BINARY).unwrap();
    let f = fixture(&raw);
    let text = attempt(&f, false, false);
    assert!(
        text.contains("bootstrap current executable consistency"),
        "{text}"
    );
    assert!(!text.contains("authentication is closed"), "{text}");
}
#[test]
fn current_image_extra_descriptor_and_wrong_assertion_cannot_bypass_old_census() {
    if crate::owned_case::isolate(
        module_path!(),
        "current_image_extra_descriptor_and_wrong_assertion_cannot_bypass_old_census",
    ) {
        return;
    }
    let raw = std::fs::read(BINARY).unwrap();
    let f = fixture(&raw);
    let text = attempt(&f, true, true);
    assert!(!text.contains("authentication is closed"), "{text}");
    let mut f = fixture(&raw);
    f.entry.image_hashes[2][0] ^= 1;
    f.replace_entry();
    let text = attempt(&f, true, false);
    assert!(!text.contains("authentication is closed"), "{text}");
}

#[path = "../../owned-case-074/library.rs"]
pub(crate) mod owned_case;
