#![cfg(target_os = "linux")]

use hydracache_long_run_supervisor_074::diagnostic_collector::fixture;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

#[test]
fn collector_requires_both_kernel_eofs_before_byte_receipt() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let (mut collector, writers) = fixture::prepare(root.path()).unwrap();
    let (stdout, stderr) = writers.into_stdio();
    let child = std::process::Command::new("/bin/cat")
        .stdin(std::process::Stdio::piped())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .unwrap();
    // Keep cleanup owned even if an assertion or I/O operation fails.
    struct OwnedChild(std::process::Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
            }
            let _ = self.0.wait();
        }
    }
    let mut child = OwnedChild(child);
    let mut input = child.0.stdin.take().unwrap();
    input.write_all(b"\0binary\xff\n").unwrap();
    drop(input);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !collector.pump().unwrap().both_eof() {
        assert!(std::time::Instant::now() < deadline, "owned fixture stuck");
        std::thread::yield_now();
    }
    assert!(child.0.wait().unwrap().success());
    let receipt = collector.finish().unwrap();
    assert_eq!(receipt.stdout_bytes(), 9);
    assert_eq!(receipt.stderr_bytes(), 0);
    assert_eq!(
        fs::read(
            root.path()
                .join("collector-attempt-v1/collector-stdout.bin")
        )
        .unwrap(),
        b"\0binary\xff\n"
    );
    assert_eq!(
        fs::metadata(
            root.path()
                .join("collector-attempt-v1/collector-stdout.bin")
        )
        .unwrap()
        .permissions()
        .mode()
            & 0o7777,
        0o400
    );
    assert!(fixture::prepare(root.path()).is_err());
}
