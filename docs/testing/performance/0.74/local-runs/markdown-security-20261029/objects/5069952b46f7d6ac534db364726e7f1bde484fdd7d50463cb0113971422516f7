//! Real owned shell/cat object observations under synthetic assertions only.
use super::*;
use crate::diagnostic_artifacts::ArtifactDigest;
use crate::diagnostic_signer_bundle::{tests::Fixture, FILES};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const PROGRAM: &str = "printf 'ready\\n'; while IFS= read -r line; do case \"$line\" in other) exec /bin/cat;; same) exec \"$SELF\" -c \"$PROGRAM\";; *) printf 'ack\\n';; esac; done";

struct Helper {
    fixture: Fixture,
    child: Child,
    output: std::path::PathBuf,
}
impl Drop for Helper {
    fn drop(&mut self) {
        self.child.stdin.take();
        self.child.wait().unwrap();
    }
}
impl Helper {
    fn start() -> Self {
        let mut fixture = Fixture::new();
        // A copied system shell is an owned helper, not a real controller build.
        let bytes = fs::read("/bin/sh").unwrap();
        fixture.edit(FILES[0], &bytes);
        fixture.statement.binary = ArtifactDigest::of(&bytes);
        fs::set_permissions(
            fixture.root.join(FILES[1]),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fixture.write_receipt();
        let output = fixture.root.parent().unwrap().join("helper-output");
        let writer = OpenOptions::new()
            .create_new(true)
            .append(true)
            .mode(0o600)
            .open(&output)
            .unwrap();
        let child = Command::new(fixture.root.join(FILES[0]))
            .args(["-c", PROGRAM])
            .env_clear()
            .env("SELF", fixture.root.join(FILES[0]))
            .env("PROGRAM", PROGRAM)
            .stdin(Stdio::piped())
            .stdout(writer)
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut helper = Self {
            fixture,
            child,
            output,
        };
        helper.wait_count(b"ready\n", 1);
        helper
    }
    fn process(&self) -> ProcessRead {
        crate::diagnostic_process::pin_owned_test_helper(self.child.id())
    }
    fn send(&mut self, value: &[u8]) {
        self.child.stdin.as_mut().unwrap().write_all(value).unwrap();
    }
    fn wait_count(&mut self, marker: &[u8], count: usize) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let bytes = fs::read(&self.output).unwrap();
            if bytes
                .windows(marker.len())
                .filter(|part| *part == marker)
                .count()
                >= count
            {
                return;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "owned helper exited before acknowledgement"
            );
            assert!(Instant::now() < deadline, "owned helper handshake deadline");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    fn different_exec(&mut self) {
        self.send(b"other\n");
        let expected = fs::metadata("/bin/cat").unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let observed = fs::metadata(format!("/proc/{}/exe", self.child.id())).unwrap();
            if (observed.dev(), observed.ino()) == (expected.dev(), expected.ino()) {
                return;
            }
            assert!(self.child.try_wait().unwrap().is_none());
            assert!(
                Instant::now() < deadline,
                "owned different-exec handshake deadline"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

#[test]
fn signer_executable_original_kernel_object_positive_and_healthy_drop() {
    let mut h = Helper::start();
    let process = h.process();
    let mut bundle = h.fixture.inspect().unwrap();
    let before = process.observation().clone();
    {
        let mut read = bundle.bind_asserted_process_executable(&process).unwrap();
        read.revalidate().unwrap();
        h.send(b"ordinary\n");
        h.wait_count(b"ack\n", 1);
        read.revalidate().unwrap();
        assert!(!read.is_refused());
    }
    bundle.check_original().unwrap();
    process.revalidate().unwrap();
    assert_eq!(&before, process.observation());
    bundle
        .bind_asserted_process_executable(&process)
        .unwrap()
        .revalidate()
        .unwrap();
}

#[test]
fn signer_executable_equal_byte_foreign_inode_and_wrong_process_refuse_bundle() {
    let h = Helper::start();
    let process = h.process();
    let bytes = fs::read(h.fixture.root.join(FILES[0])).unwrap();
    let foreign = Fixture::new();
    foreign.edit(FILES[0], &bytes);
    let mut statement = foreign.statement.clone();
    statement.binary = ArtifactDigest::of(&bytes);
    // Sign a separately valid equal-byte bundle under the same explicit test key.
    let mut foreign = foreign;
    foreign.statement = statement;
    fs::set_permissions(
        foreign.root.join(FILES[1]),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    foreign.write_receipt();
    let mut bundle = foreign.inspect().unwrap();
    assert!(bundle.bind_asserted_process_executable(&process).is_err());
    assert!(bundle.is_refused());
    let other = Helper::start();
    let other_process = other.process();
    let mut original = h.fixture.inspect().unwrap();
    assert!(original
        .bind_asserted_process_executable(&other_process)
        .is_err());
    assert!(original.is_refused());
}

#[test]
fn signer_executable_different_inode_exec_keeps_generation_but_cross_refuses() {
    let mut h = Helper::start();
    let process = h.process();
    let before = process.observation().clone();
    let mut bundle = h.fixture.inspect().unwrap();
    let mut read = bundle.bind_asserted_process_executable(&process).unwrap();
    h.different_exec();
    process.revalidate().unwrap();
    assert_eq!(&before, process.observation());
    assert!(matches!(
        read.revalidate(),
        Err(SignerExecutableError::Process(ProcessError::Drift))
    ));
    assert!(read.is_refused());
    assert!(read.revalidate().is_err());
    drop(read);
    assert!(bundle.is_refused());
    assert!(bundle.check_original().is_err());
}

#[test]
fn signer_executable_same_inode_reexec_is_not_original_invocation_proof() {
    let mut h = Helper::start();
    let process = h.process();
    let before = process.observation().clone();
    let mut bundle = h.fixture.inspect().unwrap();
    let mut read = bundle.bind_asserted_process_executable(&process).unwrap();
    h.send(b"same\n");
    h.wait_count(b"ready\n", 2);
    process.revalidate().unwrap();
    assert_eq!(&before, process.observation());
    read.revalidate().unwrap();
    assert!(!read.is_refused());
    eprintln!("actual_same_inode_reexec=observed object_and_generation_match=true original_invocation_or_runtime_proved=false");
}

#[test]
fn signer_executable_process_exit_and_failed_initial_binding_stay_refused() {
    let mut h = Helper::start();
    let process = h.process();
    let mut bundle = h.fixture.inspect().unwrap();
    let mut read = bundle.bind_asserted_process_executable(&process).unwrap();
    h.child.stdin.take();
    h.child.wait().unwrap();
    assert!(matches!(
        read.revalidate(),
        Err(SignerExecutableError::Process(ProcessError::NotLive))
    ));
    assert!(read.is_refused());
    drop(read);
    assert!(bundle.check_original().is_err());
    let mut fresh = h.fixture.inspect().unwrap();
    assert!(fresh.bind_asserted_process_executable(&process).is_err());
    assert!(fresh.is_refused());
}

#[test]
fn signer_executable_bundle_drift_and_prior_refusal_cannot_be_repaired() {
    let h = Helper::start();
    let process = h.process();
    let mut bundle = h.fixture.inspect().unwrap();
    let mut read = bundle.bind_asserted_process_executable(&process).unwrap();
    let original = fs::read(h.fixture.root.join(FILES[4])).unwrap();
    h.fixture.edit(FILES[4], b"changed runtime assertion");
    assert!(read.revalidate().is_err());
    h.fixture.edit(FILES[4], &original);
    assert!(read.revalidate().is_err());
    drop(read);
    assert!(bundle.bind_asserted_process_executable(&process).is_err());
    let h = Helper::start();
    let process = h.process();
    let mut bundle = h.fixture.inspect().unwrap();
    let bytes = fs::read(h.fixture.root.join(FILES[2])).unwrap();
    h.fixture.replace(FILES[2], &bytes);
    assert!(bundle.bind_asserted_process_executable(&process).is_err());
    assert!(bundle.is_refused());
}

#[test]
fn signer_executable_seeded_live_and_exec_cases_and_independent_originals() {
    let mut seed = 0x0740_3b24u64;
    eprintln!("seed=0x07403b24 fresh_process_observations=16 independent_originals=4 production_custody=false");
    for _ in 0..16 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut h = Helper::start();
        let process = h.process();
        let mut bundle = h.fixture.inspect().unwrap();
        let mut read = bundle.bind_asserted_process_executable(&process).unwrap();
        if seed & 1 == 0 {
            h.different_exec();
            assert!(read.revalidate().is_err());
        } else {
            h.send(b"same\n");
            h.wait_count(b"ready\n", 2);
            read.revalidate().unwrap();
        }
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    let h = Helper::start();
                    let process = h.process();
                    let mut bundle = h.fixture.inspect().unwrap();
                    bundle
                        .bind_asserted_process_executable(&process)
                        .unwrap()
                        .revalidate()
                        .unwrap();
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
}

#[test]
fn signer_executable_cloexec_and_additional_descriptor_release() {
    let h = Helper::start();
    let process = h.process();
    let mut bundle = h.fixture.inspect().unwrap();
    let file = &bundle.files[0].file;
    let executable = process.pin_executable(file).unwrap();
    executable.assert_owned_cloexec_and_drop();
    bundle.check_original().unwrap();
    process.revalidate().unwrap();
    let metadata = file_metadata(&bundle);
    assert!(metadata.is_file());
    // SAFETY: still-owned bundle descriptor, read-only descriptor flag query.
    let flags = unsafe { libc::fcntl(bundle.files[0].file.as_raw_fd(), libc::F_GETFD) };
    assert!(flags >= 0);
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
}
fn file_metadata(bundle: &AssertedSignerBundle) -> std::fs::Metadata {
    bundle.files[0].file.metadata().unwrap()
}
