#[cfg(target_os = "linux")]
mod linux {
    use super::super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, SeekFrom, Write};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::fs::OpenOptionsExt;

    const SEALS: i32 =
        libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;

    fn memfd(raw: &[u8], seals: i32, readonly: bool) -> File {
        // SAFETY: fixed NUL-terminated name; new descriptor is uniquely owned.
        let fd = unsafe {
            libc::memfd_create(
                c"held-role-fixture".as_ptr(),
                libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
            )
        };
        assert!(fd >= 0);
        let mut file = unsafe { File::from_raw_fd(fd) };
        file.write_all(raw).unwrap();
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, seals) }, 0);
        if readonly {
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_CLOEXEC)
                .open(format!("/proc/self/fd/{fd}"))
                .unwrap()
        } else {
            file
        }
    }
    fn build(f: &Fixture) -> held::VerifiedBuild {
        held::verify_receipt(&held::document_bytes(&f.receipt()).unwrap(), &f.checked()).unwrap()
    }
    fn expected(f: &Fixture) -> LaunchArtifacts {
        LaunchArtifacts {
            launcher_sha256: hash(&f.launcher),
            watchdog_sha256: hash(&f.host),
            collector_sha256: hash(&f.host),
        }
    }
    #[test]
    fn sealed_roles_retain_receipt_and_offsets_without_execution() {
        let f = Fixture::new();
        let writer = memfd(&f.host, SEALS, false);
        let mut host = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(format!("/proc/self/fd/{}", writer.as_raw_fd()))
            .unwrap();
        let mut alias = host.try_clone().unwrap();
        host.seek(SeekFrom::Start(17)).unwrap();
        let mut roles = build(&f)
            .pin_sealed_roles(
                host,
                memfd(&f.launcher, SEALS, true),
                &f.lock,
                &f.log,
                &expected(&f),
            )
            .unwrap();
        for _ in 0..3 {
            roles.revalidate(&expected(&f)).unwrap();
        }
        assert_eq!(alias.stream_position().unwrap(), 17);
        assert_eq!(
            unsafe { libc::pwrite(writer.as_raw_fd(), b"x".as_ptr().cast(), 1, 0) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EPERM)
        );
    }

    fn pin(
        f: &Fixture,
        host: File,
        launcher: File,
    ) -> Result<
        held::role_files::PinnedRoles,
        hydracache_long_run_supervisor_074::diagnostic_artifacts::ArtifactError,
    > {
        build(f).pin_sealed_roles(host, launcher, &f.lock, &f.log, &expected(f))
    }
    #[test]
    fn swapped_aliased_and_changed_role_objects_refuse() {
        let f = Fixture::new();
        assert!(pin(
            &f,
            memfd(&f.launcher, SEALS, true),
            memfd(&f.host, SEALS, true)
        )
        .is_err());
        let host = memfd(&f.host, SEALS, true);
        let alias = host.try_clone().unwrap();
        assert!(pin(&f, host, alias).is_err());
        assert!(pin(
            &f,
            memfd(&elf(3), SEALS, true),
            memfd(&f.launcher, SEALS, true)
        )
        .is_err());
        assert!(pin(&f, memfd(&f.host, SEALS, true), memfd(&elf(4), SEALS, true)).is_err());
    }
    #[test]
    fn each_missing_or_extra_seal_refuses() {
        let f = Fixture::new();
        for seals in [
            0,
            SEALS & !libc::F_SEAL_SEAL,
            SEALS & !libc::F_SEAL_GROW,
            SEALS & !libc::F_SEAL_SHRINK,
            SEALS & !libc::F_SEAL_WRITE,
            SEALS | libc::F_SEAL_FUTURE_WRITE,
        ] {
            assert!(pin(
                &f,
                memfd(&f.host, seals, true),
                memfd(&f.launcher, SEALS, true)
            )
            .is_err());
            assert!(pin(
                &f,
                memfd(&f.host, SEALS, true),
                memfd(&f.launcher, seals, true)
            )
            .is_err());
        }
    }
    #[test]
    fn writable_path_only_and_non_cloexec_descriptors_refuse() {
        let f = Fixture::new();
        assert!(pin(
            &f,
            memfd(&f.host, SEALS, false),
            memfd(&f.launcher, SEALS, true)
        )
        .is_err());
        let host = memfd(&f.host, SEALS, true);
        let path_only = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
            .open(format!("/proc/self/fd/{}", host.as_raw_fd()))
            .unwrap();
        assert!(pin(&f, path_only, memfd(&f.launcher, SEALS, true)).is_err());
        assert_eq!(
            unsafe { libc::fcntl(host.as_raw_fd(), libc::F_SETFD, 0) },
            0
        );
        assert!(pin(&f, host, memfd(&f.launcher, SEALS, true)).is_err());
    }
    #[test]
    fn ordinary_file_directory_and_pipe_refuse_without_blocking() {
        use std::os::unix::net::UnixStream;
        let f = Fixture::new();
        let ordinary = tempfile::tempfile().unwrap();
        let directory = File::open("/tmp").unwrap();
        let (socket, _) = UnixStream::pair().unwrap();
        let mut pipe = [0; 2];
        assert_eq!(
            unsafe { libc::pipe2(pipe.as_mut_ptr(), libc::O_CLOEXEC) },
            0
        );
        let reader = unsafe { File::from_raw_fd(pipe[0]) };
        let _writer = unsafe { File::from_raw_fd(pipe[1]) };
        use std::os::fd::OwnedFd;
        let socket_fd: OwnedFd = socket.into();
        for file in [ordinary, directory, reader, File::from(socket_fd)] {
            assert!(pin(&f, file, memfd(&f.launcher, SEALS, true)).is_err());
        }
    }
    #[test]
    fn lock_and_log_must_match_consumed_receipt() {
        let f = Fixture::new();
        for (lock, log) in [
            (b"wrong".as_slice(), f.log.as_slice()),
            (f.lock.as_slice(), b"wrong".as_slice()),
            (b"".as_slice(), f.log.as_slice()),
            (f.lock.as_slice(), b"".as_slice()),
        ] {
            assert!(build(&f)
                .pin_sealed_roles(
                    memfd(&f.host, SEALS, true),
                    memfd(&f.launcher, SEALS, true),
                    lock,
                    log,
                    &expected(&f)
                )
                .is_err());
        }
    }
    #[test]
    fn role_expectation_refusal_is_sticky_for_each_role() {
        let f = Fixture::new();
        for index in 0..3 {
            let mut roles = pin(
                &f,
                memfd(&f.host, SEALS, true),
                memfd(&f.launcher, SEALS, true),
            )
            .unwrap();
            let mut bad = expected(&f);
            match index {
                0 => bad.launcher_sha256 = "c".repeat(64),
                1 => bad.watchdog_sha256 = "c".repeat(64),
                _ => bad.collector_sha256 = "c".repeat(64),
            }
            assert!(roles.revalidate(&bad).is_err());
            assert!(roles.revalidate(&expected(&f)).is_err());
        }
    }
    #[test]
    fn metadata_change_is_refused_even_when_sealed_content_matches() {
        let f = Fixture::new();
        let host = memfd(&f.host, SEALS, true);
        let alias = host.try_clone().unwrap();
        let mut roles = pin(&f, host, memfd(&f.launcher, SEALS, true)).unwrap();
        assert_eq!(unsafe { libc::fchmod(alias.as_raw_fd(), 0o400) }, 0);
        assert!(roles.revalidate(&expected(&f)).is_err());
        assert_eq!(unsafe { libc::fchmod(alias.as_raw_fd(), 0o777) }, 0);
        assert!(roles.revalidate(&expected(&f)).is_err());
    }
    #[test]
    fn seeded_sealed_corruption_never_reuses_receipt() {
        let f = Fixture::new();
        let mut seed = 0x0074_f11eu64;
        eprintln!("held_role_files seed=0x{seed:08x} corruptions=32");
        for index in 0..32 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let mut raw = if index % 2 == 0 {
                f.host.clone()
            } else {
                f.launcher.clone()
            };
            let at = (seed as usize) % raw.len();
            raw[at] ^= 1 << ((seed >> 32) % 8);
            let (host, launcher) = if index % 2 == 0 {
                (memfd(&raw, SEALS, true), memfd(&f.launcher, SEALS, true))
            } else {
                (memfd(&f.host, SEALS, true), memfd(&raw, SEALS, true))
            };
            assert!(pin(&f, host, launcher).is_err(), "mutation {index}");
        }
    }
    #[test]
    fn concurrent_shared_offset_changes_do_not_substitute_role_content() {
        let f = Fixture::new();
        let host = memfd(&f.host, SEALS, true);
        let mut alias = host.try_clone().unwrap();
        let mut roles = pin(&f, host, memfd(&f.launcher, SEALS, true)).unwrap();
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                for round in 0..16 {
                    barrier.wait();
                    for offset in 0..64 {
                        alias.seek(SeekFrom::Start(4096 + offset)).unwrap();
                    }
                    alias.seek(SeekFrom::Start(8192 + round)).unwrap();
                    barrier.wait();
                }
            });
            for _ in 0..16 {
                barrier.wait();
                roles.revalidate(&expected(&f)).unwrap();
                barrier.wait();
            }
        });
        assert_eq!(alias.stream_position().unwrap(), 8207);
    }
}
