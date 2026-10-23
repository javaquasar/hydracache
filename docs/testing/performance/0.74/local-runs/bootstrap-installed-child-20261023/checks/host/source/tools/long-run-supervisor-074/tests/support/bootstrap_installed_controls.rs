//! Owned synthetic filesystem executable, never installation or original transport.
use super::*;
use hydracache_long_run_supervisor_074::diagnostic_builder::bootstrap_build::installed_entry_format::AssertedInstalledEntry;
use std::os::unix::fs::PermissionsExt;
const ARG: &str = "--systemd-bootstrap-installed-v1";
struct Installed {
    old: material::Fixture,
    directory: tempfile::TempDir,
    image: File,
    record: File,
}
impl Installed {
    fn new() -> Self {
        let raw = std::fs::read(BINARY).unwrap();
        let old = fixture(&raw);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("owned-bootstrap");
        std::fs::write(&path, &raw).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let image = File::open(&path).unwrap();
        let assertion = AssertedInstalledEntry {
            original: old.entry.clone(),
            installed_identity: material::identity(&image),
            installed_sha256: old.entry.image_hashes[2],
            parent_inventory_sha256: [7; 32],
        };
        let record = material::sealed(&assertion.encode().unwrap());
        Self {
            old,
            directory,
            image,
            record,
        }
    }
    fn path(&self) -> std::path::PathBuf {
        self.directory.path().join("owned-bootstrap")
    }
}
fn attempt(f: &Installed, matching: bool, fault: &str) -> String {
    let sources: Vec<_> = f
        .old
        .fds()
        .into_iter()
        .chain([f.image.as_raw_fd(), f.record.as_raw_fd()])
        .collect();
    let scratch: Vec<File> = sources
        .into_iter()
        .map(|fd| {
            let n = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 64) };
            assert!(n >= 64);
            unsafe { File::from_raw_fd(n) }
        })
        .collect();
    let numbers: Vec<_> = scratch.iter().map(AsRawFd::as_raw_fd).collect();
    let extra = fault == "extra";
    let missing = fault == "missing";
    let offset = fault == "offset";
    let mut c = Command::new("/bin/sh");
    c.env_clear().args(["-c","unset PWD; LISTEN_PID=$$; export LISTEN_PID; exec \"$@\"","installed-bootstrap-control"])
        .arg(if matching {f.path()} else {std::path::PathBuf::from(BINARY)}).arg(ARG)
        .env("LISTEN_FDS","14").env("LISTEN_FDNAMES","release:worker:config:creator:root-a:root-b:alarm:bootstrap:host:launcher:bootstrap-image:entry:installed-image:installed-entry")
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
                17u32,
                u32::MAX,
                libc::CLOSE_RANGE_CLOEXEC,
            ) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            if extra && libc::fcntl(20, libc::F_SETFD, 0) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if missing && libc::close(16) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if offset && libc::lseek(16, 1, libc::SEEK_SET) != 1 {
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
            panic!("installed child timeout; no retry");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(child.wait().unwrap().code(), Some(2));
    let mut raw = Vec::new();
    let mut file = &f.old.retained[2];
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
fn installed_child_filesystem_inode_matches_but_forged_context_still_refuses_authentication() {
    let f = Installed::new();
    let text = attempt(&f, true, "");
    assert!(
        text.contains("original bootstrap authentication is closed"),
        "{text}"
    );
}
#[test]
fn installed_child_same_bytes_other_executable_inode_cannot_authenticate() {
    let f = Installed::new();
    let text = attempt(&f, false, "");
    assert!(!text.contains("authentication is closed"), "{text}");
}
#[test]
fn installed_child_extra_missing_offset_hash_and_embedded_old_record_refuse() {
    for fault in ["extra", "missing", "offset"] {
        let f = Installed::new();
        let text = attempt(&f, true, fault);
        assert!(
            !text.contains("authentication is closed"),
            "{fault}: {text}"
        );
        if fault == "offset" {
            assert_eq!(
                unsafe { libc::lseek(f.record.as_raw_fd(), 0, libc::SEEK_CUR) },
                1
            );
        }
    }
    let mut f = Installed::new();
    let mut assertion = AssertedInstalledEntry::decode(
        &std::fs::read(format!("/proc/thread-self/fd/{}", f.record.as_raw_fd())).unwrap(),
    )
    .unwrap();
    assertion.original.original_context[0] ^= 1;
    f.record = material::sealed(&assertion.encode().unwrap());
    assert!(!attempt(&f, true, "").contains("authentication is closed"));
    let f = Installed::new();
    let mut raw = std::fs::read(f.path()).unwrap();
    *raw.last_mut().unwrap() ^= 1;
    std::fs::write(f.path(), raw).unwrap();
    assert!(!attempt(&f, false, "").contains("authentication is closed"));
}
