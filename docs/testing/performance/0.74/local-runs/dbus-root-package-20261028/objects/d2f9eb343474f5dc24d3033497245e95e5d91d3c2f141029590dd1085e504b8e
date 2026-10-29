use super::*;
use std::os::unix::fs::symlink;

fn root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    root
}
fn prepare(root: &Path, limit: u64) -> (Collector, [File; 2]) {
    let (collector, writers) = prepare_fixture(root, limit, None).unwrap();
    (collector, writers.0)
}
fn eof(collector: &mut Collector) {
    for _ in 0..100 {
        if collector.pump().unwrap().both_eof() {
            return;
        }
    }
    panic!("closed bounded fixture did not drain");
}

#[test]
fn collector_empty_requires_actual_eof_on_both_streams() {
    let r = root();
    let (mut c, [stdout, stderr]) = prepare(r.path(), 256);
    assert!(!c.pump().unwrap().both_eof());
    drop(stdout);
    assert!(!c.pump().unwrap().both_eof());
    assert!(c.streams[0].eof);
    assert!(!c.streams[1].eof);
    drop(stderr);
    eof(&mut c);
    let d = c.finish().unwrap();
    assert_eq!(d.bytes, [0, 0]);
    assert_eq!(d.hashes, [crate::sha256_hex(b""), crate::sha256_hex(b"")]);
}

#[test]
fn collector_fragmentation_seed_0750_preserves_exact_bytes_and_hashes() {
    let mut seed = 0x0750u64;
    for _ in 0..32 {
        let r = root();
        let (mut c, mut writers) = prepare(r.path(), 4096);
        let mut expected = [Vec::new(), Vec::new()];
        for _ in 0..32 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            for index in 0..2 {
                let chunk: Vec<_> = (0..(seed as usize % 63 + 1))
                    .map(|n| (seed >> (n % 8)) as u8)
                    .collect();
                writers[index].write_all(&chunk).unwrap();
                expected[index].extend_from_slice(&chunk);
            }
            c.pump().unwrap();
        }
        drop(writers);
        eof(&mut c);
        let d = c.finish().unwrap();
        for index in 0..2 {
            assert_eq!(d.bytes[index], expected[index].len() as u64);
            assert_eq!(d.hashes[index], crate::sha256_hex(&expected[index]));
            assert_eq!(
                fs::read(r.path().join(ATTEMPT).join(NAMES[index])).unwrap(),
                expected[index]
            );
        }
    }
}

#[test]
fn collector_exact_ceiling_needs_eof_and_one_extra_byte_refuses() {
    assert_eq!(STREAM_BYTES, 8_388_608);
    assert_eq!(
        STREAM_BYTES as u64 * 2,
        crate::diagnostic_lease::MAX_RECEIPT_BYTES
    );
    for overflow in [false, true] {
        let r = root();
        let (mut c, [mut stdout, mut stderr]) = prepare(r.path(), 256);
        stdout.write_all(&[0; 256]).unwrap();
        stderr.write_all(&[255; 256]).unwrap();
        assert_eq!(c.pump().unwrap().bytes(), [256, 256]);
        assert!(!c.progress().both_eof());
        if overflow {
            stdout.write_all(b"x").unwrap();
            assert!(matches!(c.pump(), Err(CollectorError::Overflow)));
            assert!(matches!(c.pump(), Err(CollectorError::Refused)));
            drop(stdout);
            drop(stderr);
            assert!(matches!(c.finish(), Err(CollectorError::Refused)));
        } else {
            drop(stdout);
            drop(stderr);
            eof(&mut c);
            assert_eq!(c.finish().unwrap().bytes, [256, 256]);
        }
        assert_eq!(
            fs::metadata(r.path().join(ATTEMPT).join(NAMES[0]))
                .unwrap()
                .len(),
            256
        );
    }
}

#[test]
fn collector_leaked_pipe_writer_blocks_eof_and_can_append_late_bytes() {
    let r = root();
    let (mut c, [mut stdout, stderr]) = prepare(r.path(), 256);
    let mut outside_writer = stdout.try_clone().unwrap();
    stdout.write_all(b"original").unwrap();
    drop(stdout);
    drop(stderr);
    let progress = c.pump().unwrap();
    assert_eq!(progress.bytes(), [8, 0]);
    assert!(!progress.both_eof());
    for _ in 0..8 {
        assert!(!c.pump().unwrap().both_eof());
    }
    outside_writer.write_all(b"-after-original-close").unwrap();
    drop(outside_writer);
    eof(&mut c);
    assert_eq!(c.finish().unwrap().bytes, [29, 0]);
    assert_eq!(
        fs::read(r.path().join(ATTEMPT).join(NAMES[0])).unwrap(),
        b"original-after-original-close"
    );
}

#[test]
fn collector_early_finish_consumes_owner_retains_files_without_retry() {
    let r = root();
    let (c, writers) = prepare(r.path(), 256);
    assert!(matches!(c.finish(), Err(CollectorError::NotDrained)));
    drop(writers);
    assert!(matches!(
        prepare_fixture(r.path(), 256, None),
        Err(CollectorError::Conflict)
    ));
    assert_eq!(fs::read_dir(r.path().join(ATTEMPT)).unwrap().count(), 2);
}

#[test]
fn collector_pipes_are_cloexec_with_blocking_writers_and_nonblocking_readers() {
    let r = root();
    let (c, writers) = prepare(r.path(), 256);
    for (index, writer) in writers.iter().enumerate() {
        for f in [&c.streams[index].reader, writer] {
            assert_eq!(
                unsafe { libc::fcntl(f.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                libc::FD_CLOEXEC
            );
            assert_eq!(f.metadata().unwrap().mode() & libc::S_IFMT, libc::S_IFIFO);
        }
        assert_ne!(
            unsafe { libc::fcntl(c.streams[index].reader.as_raw_fd(), libc::F_GETFL) }
                & libc::O_NONBLOCK,
            0
        );
        assert_eq!(
            unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_GETFL) } & libc::O_NONBLOCK,
            0
        );
        assert_ne!(
            c.streams[index].sink.metadata().unwrap().ino(),
            writer.metadata().unwrap().ino()
        );
    }
}

#[test]
fn collector_poll_all_65536_masks_refuse_unknown_events() {
    let mut accepted = 0;
    for events in i16::MIN..=i16::MAX {
        if classify_poll(1, events).is_ok() {
            accepted += 1;
        }
    }
    assert_eq!(accepted, 3);
    assert!(!classify_poll(0, 0).unwrap());
    for count in [-1, 0, 2, i32::MAX] {
        assert!(classify_poll(count, libc::POLLIN).is_err());
    }
}

#[test]
fn collector_metadata_and_named_replacements_refuse_sticky() {
    for case in 0..7 {
        let r = root();
        let (mut c, writers) = prepare(r.path(), 256);
        let path = r.path().join(ATTEMPT).join(NAMES[0]);
        match case {
            0 => {
                fs::rename(&path, r.path().join("old")).unwrap();
                fs::write(&path, b"").unwrap();
            }
            1 => {
                fs::remove_file(&path).unwrap();
                symlink(r.path().join("outside"), &path).unwrap();
            }
            2 => fs::hard_link(&path, r.path().join("alias")).unwrap(),
            3 => fs::set_permissions(&path, fs::Permissions::from_mode(0o660)).unwrap(),
            4 => fs::write(&path, b"unexpected").unwrap(),
            5 => fs::set_permissions(r.path().join(ATTEMPT), fs::Permissions::from_mode(0o750))
                .unwrap(),
            _ => {
                fs::rename(r.path().join(ATTEMPT), r.path().join("old")).unwrap();
                fs::create_dir(r.path().join(ATTEMPT)).unwrap();
            }
        }
        assert!(c.pump().is_err(), "case {case}");
        assert!(matches!(c.pump(), Err(CollectorError::Refused)));
        drop(writers);
        assert!(matches!(c.finish(), Err(CollectorError::Refused)));
        assert!(prepare_fixture(r.path(), 256, None).is_err());
    }
}

#[test]
fn collector_same_length_content_mutation_refuses() {
    let r = root();
    let (mut c, [mut stdout, stderr]) = prepare(r.path(), 256);
    stdout.write_all(b"original").unwrap();
    c.pump().unwrap();
    fs::write(r.path().join(ATTEMPT).join(NAMES[0]), b"replaced").unwrap();
    drop(stdout);
    drop(stderr);
    // Coarse timestamps can miss the change at pump; retained byte hashing at
    // finish must refuse it regardless of that metadata observation.
    let pumped = c.pump();
    if pumped.is_ok() {
        eof(&mut c);
    }
    assert!(c.finish().is_err());
}

#[test]
fn collector_both_streams_make_one_bounded_quantum_of_progress() {
    let r = root();
    let (mut c, mut writers) = prepare(r.path(), STREAM_BYTES as u64);
    for writer in &mut writers {
        let capacity = unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_GETPIPE_SZ) };
        assert!(capacity > 0 && capacity <= 1_048_576);
        writer.write_all(&vec![1; capacity as usize]).unwrap();
    }
    let p = c.pump().unwrap();
    assert!(p.bytes().iter().all(|v| *v > 0 && *v <= QUANTUM as u64));
    drop(writers);
    eof(&mut c);
    c.finish().unwrap();
}

#[test]
fn collector_real_write_error_latches_refusal_and_retains_outputs() {
    let r = root();
    let (mut c, [mut stdout, stderr]) = prepare(r.path(), 256);
    c.streams[0].sink = open_at(&c.directory, NAMES[0], libc::O_RDONLY, 0).unwrap();
    stdout.write_all(b"error").unwrap();
    assert!(matches!(c.pump(), Err(CollectorError::Io(_))));
    assert!(matches!(c.pump(), Err(CollectorError::Refused)));
    drop(stdout);
    drop(stderr);
    assert!(c.finish().is_err());
    assert_eq!(fs::read_dir(r.path().join(ATTEMPT)).unwrap().count(), 2);
}

#[test]
fn collector_owned_sigkill_and_external_pipe_copy_require_late_eof() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Child, Command, Stdio};
    struct OwnedChild(Child);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            if self.0.try_wait().ok().flatten().is_none() {
                let _ = self.0.kill();
            }
            let _ = self.0.wait();
        }
    }
    let r = root();
    let (mut c, [stdout, stderr]) = prepare(r.path(), 256);
    let mut outside = stdout.try_clone().unwrap();
    let mut child = OwnedChild(
        Command::new("/bin/cat")
            .stdin(Stdio::piped())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .unwrap(),
    );
    let mut input = child.0.stdin.take().unwrap();
    input.write_all(b"before-kill").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while c.pump().unwrap().bytes()[0] != 11 {
        assert!(
            std::time::Instant::now() < deadline,
            "owned fixture stalled"
        );
        std::thread::yield_now();
    }
    child.0.kill().unwrap();
    assert_eq!(child.0.wait().unwrap().signal(), Some(libc::SIGKILL));
    drop(input);
    assert!(!c.pump().unwrap().both_eof());
    outside.write_all(b"-after-kill").unwrap();
    drop(outside);
    eof(&mut c);
    let d = c.finish().unwrap();
    assert_eq!(d.bytes, [22, 0]);
    assert_eq!(d.hashes[0], crate::sha256_hex(b"before-kill-after-kill"));
    eprintln!("seed=0x0750 actual_owned_sigkill=1 outside_pipe_writer_blocks_eof=true late_bytes_reconciled=true production_cgroup_proof=false");
}

#[test]
fn collector_paths_and_unsafe_roots_refuse_before_creation() {
    let r = root();
    let alias = r.path().join("alias");
    symlink(r.path(), &alias).unwrap();
    for path in [
        Path::new("relative"),
        Path::new("/var/lib/hydracache-performance/diagnostics"),
        Path::new("/opt/hydracache-performance"),
        &alias,
    ] {
        assert!(prepare_fixture(path, 256, None).is_err());
    }
    fs::set_permissions(r.path(), fs::Permissions::from_mode(0o750)).unwrap();
    assert!(prepare_fixture(r.path(), 256, None).is_err());
    assert!(!r.path().join(ATTEMPT).exists());
}

#[test]
fn collector_concurrent_creation_has_exactly_one_original_owner() {
    let r = root();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let path = r.path().to_path_buf();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                match prepare_fixture(&path, 256, None) {
                    Ok(_) => true,
                    Err(CollectorError::Conflict) => false,
                    Err(e) => panic!("unexpected: {e}"),
                }
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .filter(|v| *v)
            .count(),
        1
    );
    assert_eq!(fs::read_dir(r.path().join(ATTEMPT)).unwrap().count(), 2);
}

#[test]
fn collector_creation_faults_retain_partial_attempt_and_deny_replay() {
    for (stage, count) in [
        (Stage::Directory, 0),
        (Stage::StdoutCreated, 1),
        (Stage::StderrCreated, 2),
    ] {
        let r = root();
        assert!(matches!(
            prepare_fixture(r.path(), 256, Some(stage)),
            Err(CollectorError::Injected(_))
        ));
        assert_eq!(fs::read_dir(r.path().join(ATTEMPT)).unwrap().count(), count);
        assert!(matches!(
            prepare_fixture(r.path(), 256, None),
            Err(CollectorError::Conflict)
        ));
    }
}

#[test]
fn collector_write_and_finish_faults_never_return_receipt() {
    for stage in [
        Stage::Write,
        Stage::StdoutSync,
        Stage::StderrSync,
        Stage::Readonly,
        Stage::DirectorySync,
        Stage::Close,
        Stage::Acknowledge,
    ] {
        let r = root();
        let (mut c, [mut stdout, stderr]) = prepare(r.path(), 256);
        c.fault = Some(stage);
        stdout.write_all(b"retained").unwrap();
        drop(stdout);
        drop(stderr);
        if stage == Stage::Write {
            assert!(matches!(c.pump(), Err(CollectorError::Injected(_))));
            assert!(matches!(c.finish(), Err(CollectorError::Refused)));
        } else {
            eof(&mut c);
            assert!(matches!(c.finish(), Err(CollectorError::Injected(_))));
        }
        assert_eq!(
            fs::read(r.path().join(ATTEMPT).join(NAMES[0])).unwrap(),
            b"retained"
        );
        assert!(matches!(
            prepare_fixture(r.path(), 256, None),
            Err(CollectorError::Conflict)
        ));
    }
}

#[test]
fn collector_invalid_limits_never_create_attempt() {
    for limit in [0, STREAM_BYTES as u64 + 1, u64::MAX] {
        let r = root();
        assert!(prepare_fixture(r.path(), limit, None).is_err());
        assert!(!r.path().join(ATTEMPT).exists());
    }
}

#[test]
fn collector_real_frozen_limits_bound_both_files_and_refuse_overflow() {
    for overflow in [false, true] {
        let r = root();
        let (mut c, mut writers) = prepare(r.path(), STREAM_BYTES as u64);
        let chunk = [42; 4096];
        for n in 1..=STREAM_BYTES / chunk.len() {
            for writer in &mut writers {
                writer.write_all(&chunk).unwrap();
            }
            assert_eq!(c.pump().unwrap().bytes(), [(n * chunk.len()) as u64; 2]);
        }
        if overflow {
            writers[1].write_all(b"x").unwrap();
            assert!(matches!(c.pump(), Err(CollectorError::Overflow)));
            drop(writers);
            assert!(matches!(c.finish(), Err(CollectorError::Refused)));
        } else {
            drop(writers);
            eof(&mut c);
            assert_eq!(c.finish().unwrap().bytes, [STREAM_BYTES as u64; 2]);
        }
        for name in NAMES {
            assert_eq!(
                fs::metadata(r.path().join(ATTEMPT).join(name))
                    .unwrap()
                    .len(),
                STREAM_BYTES as u64
            );
        }
    }
}

#[test]
fn collector_extra_entries_and_parent_replacement_refuse() {
    for replace_parent in [false, true] {
        let outer = root();
        let path = outer.path().join("private");
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let (mut c, writers) = prepare(&path, 256);
        if replace_parent {
            fs::rename(&path, outer.path().join("original")).unwrap();
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        } else {
            fs::write(path.join(ATTEMPT).join("unexpected"), b"retained").unwrap();
        }
        assert!(c.pump().is_err());
        drop(writers);
        assert!(c.finish().is_err());
    }
}
