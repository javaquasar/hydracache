use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
fn output(mut command: Command) -> std::process::Output {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            panic!("bootstrap refusal child did not exit");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let result = child.wait_with_output().unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("no start, send or executable handoff")
    );
    result
}
#[test]
fn bootstrap_child_no_argument_future_mode_and_unknown_environment_never_start() {
    for args in [
        vec![],
        vec!["--systemd-bootstrap-entry-v2"],
        vec!["--systemd-bootstrap-entry-v1", "extra"],
    ] {
        let mut c = Command::new(env!("CARGO_BIN_EXE_diagnostic_held_bootstrap"));
        c.env_clear().args(args);
        output(c);
    }
    let mut c = Command::new(env!("CARGO_BIN_EXE_diagnostic_held_bootstrap"));
    c.env_clear()
        .arg("--systemd-bootstrap-entry-v1")
        .env("PATH", "untrusted");
    output(c);
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "support/bootstrap_child_material.rs"]
mod material;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn bootstrap_child_forged_but_canonical_sealed_record_reaches_only_closed_authentication() {
    use std::os::{fd::AsRawFd, unix::process::CommandExt};
    const NAMES:&str="release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry";
    // A fresh shell supplies its own PID and execs only the bootstrap reader.
    // This harness shell is not a command/path fallback in the bootstrap.
    let attempt = |entry: Option<i32>, extra: bool| {
        let mut c = Command::new("/bin/sh");
        c.env_clear()
            .args([
                "-c",
                "unset PWD; LISTEN_PID=$$; export LISTEN_PID; exec \"$@\"",
                "bootstrap-reader-control",
            ])
            .arg(env!("CARGO_BIN_EXE_diagnostic_held_bootstrap"))
            .arg("--systemd-bootstrap-entry-v1")
            .env("LISTEN_FDS", "12")
            .env("LISTEN_FDNAMES", NAMES);
        if extra {
            c.env("UNEXPECTED", "");
        }
        // Only async-signal-safe FD syscalls after fork. Never change parent FDs.
        unsafe {
            c.pre_exec(move || {
                if let Some(fd) = entry {
                    if fd != 14 {
                        if libc::dup2(fd, 14) < 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                        libc::close(fd);
                    }
                    if libc::fcntl(14, libc::F_SETFD, 0) != 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                } else {
                    libc::close(14);
                }
                Ok(())
            });
        }
        output(c)
    };
    let missing = attempt(None, false);
    assert!(!String::from_utf8_lossy(&missing.stderr).contains("authentication is closed"));
    let f = material::sealed(&material::raw(), material::SEALS);
    let observed = attempt(Some(f.as_raw_fd()), false);
    assert!(String::from_utf8_lossy(&observed.stderr)
        .contains("original bootstrap authentication is closed"));
    let unknown = attempt(Some(f.as_raw_fd()), true);
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("bootstrap activation assertions"));
    let mut bad = material::raw();
    bad[12] = 1;
    let f = material::sealed(&bad, material::SEALS);
    let invalid = attempt(Some(f.as_raw_fd()), false);
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("bootstrap entry shape"));
}
