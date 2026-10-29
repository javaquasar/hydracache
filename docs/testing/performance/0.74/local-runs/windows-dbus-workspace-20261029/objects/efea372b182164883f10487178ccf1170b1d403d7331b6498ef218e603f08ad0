use super::*;
use std::fs::OpenOptions;
use std::os::unix::fs::{symlink, OpenOptionsExt};

const ATTEMPT: &str = "attempt-00000000000000000000000000000074";
struct Fixture {
    temp: tempfile::TempDir,
    source: File,
    parent: File,
    expected: ArtifactDigest,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(temp.path().join("source"), b"unsigned fixture contents").unwrap();
        let source = File::open(temp.path().join("source")).unwrap();
        let parent = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY)
            .open(temp.path())
            .unwrap();
        Self {
            temp,
            source,
            parent,
            expected: ArtifactDigest::of(b"unsigned fixture contents"),
        }
    }
    fn output(&self) -> std::path::PathBuf {
        self.temp.path().join(ATTEMPT).join(NAME)
    }
    fn run(
        &self,
        hook: &mut impl FnMut(Cut) -> io::Result<()>,
    ) -> io::Result<AssertedPrebuiltCopy> {
        prepare(
            &self.source,
            &self.parent,
            ATTEMPT,
            self.expected.clone(),
            hook,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Only an exact owned child, no following substituted symlinks.
        let path = self.temp.path().join(ATTEMPT);
        if fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir()) {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
    }
}
fn writers(path: &std::path::Path) -> usize {
    let Ok(target) = fs::metadata(path) else {
        return 0;
    };
    fs::read_dir("/proc/self/fd")
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            let Ok(m) = fs::metadata(e.path()) else {
                return false;
            };
            if (m.dev(), m.ino()) != (target.dev(), target.ino()) {
                return false;
            }
            let fd: i32 = e.file_name().to_string_lossy().parse().unwrap();
            // SAFETY: query only; descriptor may concurrently close, which cannot count.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            flags >= 0 && flags & libc::O_ACCMODE != libc::O_RDONLY
        })
        .count()
}
#[test]
fn prebuilt_copy_owned_writer_closes_before_readonly_return() {
    let f = Fixture::new();
    let mut cuts = Vec::new();
    let mut copy = f
        .run(&mut |cut| {
            cuts.push(cut);
            if cut != Cut::Directory {
                assert_eq!(
                    writers(&f.output()),
                    usize::from(![Cut::Closed, Cut::Sealed].contains(&cut))
                );
            }
            Ok(())
        })
        .unwrap();
    assert!(
        cuts == [
            Cut::Directory,
            Cut::Created,
            Cut::Copied,
            Cut::Synced,
            Cut::Mode,
            Cut::Closed,
            Cut::Sealed
        ]
    );
    for file in [&copy.parent, &copy.directory, &copy.file] {
        readable(file).unwrap();
    }
    assert_eq!(copy.check_original().unwrap(), &f.expected);
    assert_eq!(writers(&f.output()), 0);
    drop(copy);
    assert_eq!(writers(&f.output()), 0);
}
#[test]
fn prebuilt_copy_each_injected_failure_retains_attempt_without_retry_or_writer() {
    for target in [
        Cut::Directory,
        Cut::Created,
        Cut::Copied,
        Cut::Synced,
        Cut::Mode,
        Cut::Closed,
        Cut::Sealed,
    ] {
        let f = Fixture::new();
        assert!(f
            .run(&mut |cut| if cut == target {
                Err(io::Error::other("injected cut"))
            } else {
                Ok(())
            })
            .is_err());
        assert!(f.output().parent().unwrap().exists());
        assert_eq!(writers(&f.output()), 0);
        assert!(f.run(&mut |_| Ok(())).is_err());
    }
}
#[test]
fn prebuilt_copy_chmod_does_not_revoke_a_foreign_retained_writer() {
    let f = Fixture::new();
    let mut retained = None;
    let mut copy = f
        .run(&mut |cut| {
            if cut == Cut::Created {
                retained = Some(OpenOptions::new().write(true).open(f.output())?);
            }
            Ok(())
        })
        .unwrap();
    // Point-in-time inventory can pass even with this foreign writer. It must
    // never be promoted to a whole-writer or authenticated custody capability.
    assert_eq!(writers(&f.output()), 1);
    assert_eq!(copy.check_original().unwrap(), &f.expected);
    retained.as_ref().unwrap().write_all_at(b"X", 0).unwrap();
    assert!(copy.check_original().is_err());
    assert!(copy.is_refused());
    retained.as_ref().unwrap().write_all_at(b"u", 0).unwrap();
    assert!(copy.check_original().is_err());
    drop(retained);
    assert_eq!(writers(&f.output()), 0);
    println!("retained_foreign_writer_survives_0555=true inventory_is_not_revocation=true");
}
#[test]
fn prebuilt_copy_source_changes_during_copy_refuse_without_source_repair() {
    let f = Fixture::new();
    let before = Stamp::of(f.source.metadata().unwrap());
    let mut metadata_equal = false;
    assert!(f
        .run(&mut |cut| {
            if cut == Cut::Copied {
                OpenOptions::new()
                    .write(true)
                    .open(f.temp.path().join("source"))?
                    .write_all_at(b"X", 0)?;
                metadata_equal = Stamp::of(f.source.metadata()?) == before;
            }
            Ok(())
        })
        .is_err());
    assert_eq!(fs::read(f.temp.path().join("source")).unwrap()[0], b'X');
    assert_eq!(writers(&f.output()), 0);
    println!("source_write_metadata_equal={metadata_equal} source_content_rehash_refuses=true");
}
#[test]
fn prebuilt_copy_source_offset_is_unchanged_and_wrong_digest_retains_failure() {
    use std::io::{Seek, SeekFrom};
    let f = Fixture::new();
    let mut source = &f.source;
    source.seek(SeekFrom::Start(7)).unwrap();
    let mut copy = f.run(&mut |_| Ok(())).unwrap();
    assert_eq!(source.stream_position().unwrap(), 7);
    copy.check_original().unwrap();
    let f = Fixture::new();
    let mut bad = f.expected.clone();
    bad.sha256 = "0".repeat(64);
    assert!(prepare_asserted_copy(&f.source, &f.parent, ATTEMPT, bad).is_err());
    assert!(f.output().exists());
    assert_eq!(writers(&f.output()), 0);
}
#[test]
fn prebuilt_copy_extra_child_or_equal_bytes_substitution_is_sticky() {
    for replace in [false, true] {
        let f = Fixture::new();
        let mut copy = f.run(&mut |_| Ok(())).unwrap();
        let root = f.output().parent().unwrap().to_owned();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        if replace {
            fs::rename(f.output(), root.join("saved")).unwrap();
            fs::write(f.output(), b"unsigned fixture contents").unwrap();
            fs::set_permissions(f.output(), fs::Permissions::from_mode(0o555)).unwrap();
            fs::remove_file(root.join("saved")).unwrap();
        } else {
            fs::write(root.join("extra"), b"extra").unwrap();
        }
        fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(copy.check_original().is_err());
        assert!(copy.is_refused());
        assert!(copy.check_original().is_err());
    }
}
#[test]
fn prebuilt_copy_invalid_names_bounds_parent_and_input_flags_create_nothing() {
    let f = Fixture::new();
    for attempt in [
        "",
        "../escape",
        "attempt-",
        "attempt-0000000000000000000000000000000G",
        "attempt-00000000000000000000000000000000/child",
    ] {
        assert!(prepare_asserted_copy(&f.source, &f.parent, attempt, f.expected.clone()).is_err());
    }
    for bytes in [0, MAX_BINARY_BYTES + 1, f.expected.bytes + 1] {
        let mut bad = f.expected.clone();
        bad.bytes = bytes;
        assert!(prepare_asserted_copy(&f.source, &f.parent, ATTEMPT, bad).is_err());
    }
    let writable = OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.temp.path().join("source"))
        .unwrap();
    assert!(prepare_asserted_copy(&writable, &f.parent, ATTEMPT, f.expected.clone()).is_err());
    assert!(prepare_asserted_copy(&f.parent, &f.parent, ATTEMPT, f.expected.clone()).is_err());
    for mode in [0o755, 0o770, 0o2770] {
        fs::set_permissions(f.temp.path(), fs::Permissions::from_mode(mode)).unwrap();
        assert!(f.run(&mut |_| Ok(())).is_err());
    }
    fs::set_permissions(f.temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = CString::new(f.temp.path().join("source").as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: isolated owned descriptor, wrapped exactly once; deliberately unsupported O_PATH.
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) };
    assert!(fd >= 0);
    let path_only = unsafe { File::from_raw_fd(fd) };
    assert!(prepare_asserted_copy(&path_only, &f.parent, ATTEMPT, f.expected.clone()).is_err());
    let inherited = f.source.try_clone().unwrap();
    // SAFETY: only this isolated owned duplicate loses CLOEXEC for the negative test.
    assert_eq!(
        unsafe { libc::fcntl(inherited.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    assert!(prepare_asserted_copy(&inherited, &f.parent, ATTEMPT, f.expected.clone()).is_err());
    assert!(!f.output().parent().unwrap().exists());
}
#[test]
fn prebuilt_copy_existing_symlink_attempt_never_touches_target() {
    let f = Fixture::new();
    let target = f.temp.path().join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join(NAME), b"untouched").unwrap();
    symlink(&target, f.output().parent().unwrap()).unwrap();
    assert!(f.run(&mut |_| Ok(())).is_err());
    assert_eq!(fs::read(target.join(NAME)).unwrap(), b"untouched");
}
#[test]
fn prebuilt_copy_bounded_stream_handles_short_io_and_faults() {
    struct Short {
        data: Vec<u8>,
        fail: bool,
    }
    impl Write for Short {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fail && self.data.len() >= 9 {
                return Err(io::Error::from_raw_os_error(libc::ENOSPC));
            }
            let n = bytes.len().min(3);
            self.data.extend_from_slice(&bytes[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    struct ShortRead<'a>(&'a [u8]);
    impl Read for ShortRead<'_> {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let n = out.len().min(5).min(self.0.len());
            out[..n].copy_from_slice(&self.0[..n]);
            self.0 = &self.0[n..];
            Ok(n)
        }
    }
    let bytes = b"unsigned fixture contents";
    let expected = ArtifactDigest::of(bytes);
    let mut short = Short {
        data: Vec::new(),
        fail: false,
    };
    copy_exact(&mut ShortRead(bytes), &mut short, &expected).unwrap();
    assert_eq!(short.data, bytes);
    let mut short = Short {
        data: Vec::new(),
        fail: true,
    };
    assert_eq!(
        copy_exact(&mut ShortRead(bytes), &mut short, &expected)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::ENOSPC)
    );
    assert!(copy_exact(&mut &bytes[..bytes.len() - 1], &mut io::sink(), &expected).is_err());
    let mut extra = bytes.to_vec();
    extra.push(0);
    assert!(copy_exact(&mut extra.as_slice(), &mut io::sink(), &expected).is_err());
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(libc::EIO))
        }
    }
    assert_eq!(
        copy_exact(&mut Broken, &mut io::sink(), &expected)
            .unwrap_err()
            .raw_os_error(),
        Some(libc::EIO)
    );
}
#[test]
fn prebuilt_copy_concurrent_same_attempt_has_exactly_one_owner() {
    let f = Fixture::new();
    std::thread::scope(|scope| {
        let joins: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| f.run(&mut |_| Ok(()))))
            .collect();
        let mut successes = 0;
        for join in joins {
            if let Ok(mut copy) = join.join().unwrap() {
                copy.check_original().unwrap();
                successes += 1;
            }
        }
        assert_eq!(successes, 1);
    });
}
#[test]
fn prebuilt_copy_independent_attempts_share_no_file_or_writer() {
    let f = Fixture::new();
    std::thread::scope(|scope| {
        let joins: Vec<_> = (0..4)
            .map(|n| {
                let f = &f;
                scope.spawn(move || {
                    let attempt = format!("attempt-{n:032x}");
                    let mut copy =
                        prepare_asserted_copy(&f.source, &f.parent, &attempt, f.expected.clone())
                            .unwrap();
                    copy.check_original().unwrap();
                    copy.file_stamp.identity
                })
            })
            .collect();
        let ids: std::collections::BTreeSet<_> =
            joins.into_iter().map(|j| j.join().unwrap()).collect();
        assert_eq!(ids.len(), 4);
    });
    // Explicit cleanup preparation, only exact owned attempt names.
    for n in 0..4 {
        fs::set_permissions(
            f.temp.path().join(format!("attempt-{n:032x}")),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    }
}

#[test]
fn prebuilt_copy_close_error_is_reported_once_without_retry() {
    let f = Fixture::new();
    let mut calls = 0;
    let error = close_owned(f.source.try_clone().unwrap(), |fd| {
        calls += 1;
        linux_close(fd)?;
        // Deterministic late close failure after releasing the owned descriptor.
        Err(io::Error::from_raw_os_error(libc::EIO))
    })
    .unwrap_err();
    assert_eq!(error.raw_os_error(), Some(libc::EIO));
    assert_eq!(calls, 1);
}

#[test]
fn prebuilt_copy_seeded_lengths_and_corruptions_keep_the_exact_budget() {
    let mut seed = 0x0740_c074u64;
    for len in [1, 65535, 65536, 65537, 131073] {
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            bytes.push(seed as u8);
        }
        let expected = ArtifactDigest::of(&bytes);
        copy_exact(&mut bytes.as_slice(), &mut io::sink(), &expected).unwrap();
        for n in 0..8 {
            let index = (seed as usize).wrapping_add(n) % len;
            bytes[index] ^= 1;
            assert!(copy_exact(&mut bytes.as_slice(), &mut io::sink(), &expected).is_err());
            bytes[index] ^= 1;
        }
    }
    println!("seed=0x0740c074 boundary_lengths=5 corruptions=40 copy_budget_exact=true");
}

#[test]
fn prebuilt_copy_parent_mode_link_and_named_directory_drift_refuse() {
    for mode in 0..3 {
        let f = Fixture::new();
        let mut copy = f.run(&mut |_| Ok(())).unwrap();
        if mode == 0 {
            fs::set_permissions(f.temp.path(), fs::Permissions::from_mode(0o755)).unwrap();
        }
        if mode == 1 {
            fs::hard_link(f.output(), f.temp.path().join("foreign-alias")).unwrap();
        }
        if mode == 2 {
            fs::rename(
                f.output().parent().unwrap(),
                f.temp.path().join("saved-directory"),
            )
            .unwrap();
            fs::create_dir(f.output().parent().unwrap()).unwrap();
        }
        assert!(copy.check_original().is_err());
        assert!(copy.is_refused());
        fs::set_permissions(f.temp.path(), fs::Permissions::from_mode(0o700)).unwrap();
        if mode == 2 {
            fs::set_permissions(
                f.temp.path().join("saved-directory"),
                fs::Permissions::from_mode(0o700),
            )
            .unwrap();
        }
        assert!(copy.check_original().is_err());
    }
}

#[test]
fn prebuilt_copy_sigkill_child() {
    let Some(root) = std::env::var_os("HYDRACACHE_COPY_TEST_ROOT") else {
        return;
    };
    let target: usize = std::env::var("HYDRACACHE_COPY_TEST_CUT")
        .unwrap()
        .parse()
        .unwrap();
    let root = std::path::PathBuf::from(root);
    let source = File::open(root.join("source")).unwrap();
    let parent = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY)
        .open(&root)
        .unwrap();
    let mut index = 0;
    let _ = prepare(
        &source,
        &parent,
        ATTEMPT,
        ArtifactDigest::of(b"unsigned fixture contents"),
        &mut |_| {
            if index == target {
                let mut gate = std::os::unix::net::UnixStream::connect(root.join("cut.socket"))?;
                gate.write_all(&[index as u8])?;
                loop {
                    // SAFETY: owned fixture child waits without advancing the copy;
                    // only the parent kills it. No prepared executable is invoked.
                    unsafe {
                        libc::pause();
                    }
                }
            }
            index += 1;
            Ok(())
        },
    );
    panic!("requested copy checkpoint was not reached");
}
#[test]
fn prebuilt_copy_actual_sigkill_retains_all_seven_attempts_without_resume() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Command, Stdio};
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn ready(file: &impl AsRawFd) {
        let mut poll = libc::pollfd {
            fd: file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one initialized pollfd. Timeout is an outer stall guard only.
        assert_eq!(
            unsafe { libc::poll(&mut poll, 1, 5000) },
            1,
            "copy child stalled"
        );
        assert_eq!(poll.revents, libc::POLLIN);
    }
    for target in 0..7 {
        let f = Fixture::new();
        let listener =
            std::os::unix::net::UnixListener::bind(f.temp.path().join("cut.socket")).unwrap();
        let mut child = Child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "diagnostic_prebuilt_copy::tests::prebuilt_copy_sigkill_child",
                    "--nocapture",
                ])
                .env_clear()
                .env("HYDRACACHE_COPY_TEST_ROOT", f.temp.path())
                .env("HYDRACACHE_COPY_TEST_CUT", target.to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        ready(&listener);
        let (mut gate, _) = listener.accept().unwrap();
        ready(&gate);
        let mut observed = [255];
        gate.read_exact(&mut observed).unwrap();
        assert_eq!(observed[0], target);
        child.0.kill().unwrap();
        let status = child.0.wait().unwrap();
        assert_eq!(status.signal(), Some(libc::SIGKILL));
        assert!(f.output().parent().unwrap().exists());
        assert_eq!(f.output().exists(), target != 0);
        assert_eq!(writers(&f.output()), 0);
        assert!(f.run(&mut |_| Ok(())).is_err());
        if target == 6 {
            assert_eq!(fs::metadata(f.output()).unwrap().mode() & 0o7777, 0o555);
            assert_eq!(fs::read(f.output()).unwrap(), b"unsigned fixture contents");
        }
    }
    println!("actual_owned_child_SIGKILL=7 retained_attempts=7 retries_refused=7 prepared_binary_executed=false power_loss_proved=false");
}
