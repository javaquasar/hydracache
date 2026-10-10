//! Original-generation, point-in-time object binding; never start/stop authority.

use super::{child, Document, FileId, ProcessError, ProcessRead, DOCUMENT_BYTES};
use std::ffi::CString;
use std::fs::{File, Metadata};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;

const STREAM_BYTES: u64 = crate::diagnostic_receipts::STREAM_BYTES as u64;

#[derive(Clone, PartialEq, Eq)]
struct Stable {
    id: FileId,
    uid: u32,
    gid: u32,
    mode: u32,
    links: u64,
}
impl Stable {
    fn of(m: &Metadata) -> Self {
        Self {
            id: FileId::of(m),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
            links: m.nlink(),
        }
    }
}
#[derive(PartialEq, Eq)]
struct Executable {
    stable: Stable,
    len: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}
impl Executable {
    fn of(m: &Metadata) -> Self {
        Self {
            stable: Stable::of(m),
            len: m.len(),
            mtime: (m.mtime(), m.mtime_nsec()),
            ctime: (m.ctime(), m.ctime_nsec()),
        }
    }
}

/// Borrows the original process and owns read-only expected/O_PATH descriptors.
/// Outputs are caller assertions, not fixed pathname/production-root proof.
/// No serialization or durable refusal; absence of drift is point-in-time only.
pub struct ProcessIoRead<'a> {
    process: &'a ProcessRead,
    expected: [File; 3],
    targets: [File; 3],
    fd: File,
    fdinfo: File,
    documents: [Document; 2],
    exe: Executable,
    streams: [Stable; 2],
    high_water: [u64; 2],
    refused: bool,
}

impl ProcessRead {
    // The public production binding is the typed checked start-material hook.
    pub(crate) fn pin_io<'a>(
        &'a self,
        exe: &File,
        stdout: &File,
        stderr: &File,
    ) -> Result<ProcessIoRead<'a>, ProcessError> {
        self.revalidate()?;
        let expected = [
            readonly_clone(exe)?,
            readonly_clone(stdout)?,
            readonly_clone(stderr)?,
        ];
        let exe_meta = expected[0].metadata()?;
        if !exe_meta.is_file() || exe_meta.len() == 0 || exe_meta.nlink() != 1 {
            return Err(ProcessError::Invalid);
        }
        let stream_meta = [expected[1].metadata()?, expected[2].metadata()?];
        for meta in &stream_meta {
            check_stream(meta)?;
        }
        let exe = Executable::of(&exe_meta);
        let streams = [Stable::of(&stream_meta[0]), Stable::of(&stream_meta[1])];
        if streams[0].id == streams[1].id || streams.iter().any(|s| s.id == exe.stable.id) {
            return Err(ProcessError::Invalid);
        }
        let directory = &self.probe.files.directory;
        let fd = child(directory, "fd", true)?;
        let fdinfo = child(directory, "fdinfo", true)?;
        check_directory(directory, &fd, "fd")?;
        check_directory(directory, &fdinfo, "fdinfo")?;
        let documents = [Document::open(&fdinfo, "1")?, Document::open(&fdinfo, "2")?];
        let targets = [
            magic_target(directory, "exe")?,
            magic_target(&fd, "1")?,
            magic_target(&fd, "2")?,
        ];
        let mut guard = ProcessIoRead {
            process: self,
            expected,
            targets,
            fd,
            fdinfo,
            documents,
            exe,
            streams,
            high_water: [stream_meta[0].len(), stream_meta[1].len()],
            refused: false,
        };
        guard.revalidate()?;
        Ok(guard)
    }
}
impl ProcessIoRead<'_> {
    pub fn is_refused(&self) -> bool {
        self.refused
    }

    /// Never recaptures a pidfd. Every failed observation permanently refuses
    /// this runtime object, even if the caller later repairs the observed state.
    pub fn revalidate(&mut self) -> Result<(), ProcessError> {
        if self.refused {
            return Err(ProcessError::Drift);
        }
        let result = self.inspect();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    fn inspect(&mut self) -> Result<(), ProcessError> {
        self.process.revalidate()?;
        let directory = &self.process.probe.files.directory;
        check_directory(directory, &self.fd, "fd")?;
        check_directory(directory, &self.fdinfo, "fdinfo")?;
        for file in [
            &self.expected[0],
            &self.targets[0],
            &magic_target(directory, "exe")?,
        ] {
            if Executable::of(&file.metadata()?) != self.exe {
                return Err(ProcessError::Drift);
            }
        }
        for i in 0..2 {
            self.observe_stream(i, &self.expected[i + 1].metadata()?)?;
            self.observe_stream(i, &self.targets[i + 1].metadata()?)?;
            let target = magic_target(&self.fd, if i == 0 { "1" } else { "2" })?;
            self.observe_stream(i, &target.metadata()?)?;
            let (flags, ino) = parse_fdinfo(&self.documents[i].read(&self.fdinfo)?)?;
            if flags & libc::O_ACCMODE as u64 != libc::O_WRONLY as u64
                || flags & libc::O_APPEND as u64 == 0
                || flags & libc::O_PATH as u64 != 0
                || ino != self.streams[i].id.1
            {
                return Err(ProcessError::Drift);
            }
            // Fresh targets bracket fdinfo. This is not an atomic FD-table snapshot.
            let after = magic_target(&self.fd, if i == 0 { "1" } else { "2" })?;
            self.observe_stream(i, &after.metadata()?)?;
            self.observe_stream(i, &self.expected[i + 1].metadata()?)?;
        }
        if Executable::of(&magic_target(directory, "exe")?.metadata()?) != self.exe {
            return Err(ProcessError::Drift);
        }
        check_directory(directory, &self.fd, "fd")?;
        check_directory(directory, &self.fdinfo, "fdinfo")?;
        self.process.revalidate()
    }
    fn observe_stream(&mut self, i: usize, meta: &Metadata) -> Result<(), ProcessError> {
        check_stream(meta)?;
        if Stable::of(meta) != self.streams[i] || meta.len() < self.high_water[i] {
            return Err(ProcessError::Drift);
        }
        self.high_water[i] = meta.len();
        Ok(())
    }
}
fn readonly_clone(file: &File) -> Result<File, ProcessError> {
    // SAFETY: retained descriptor, read-only fcntl query.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if flags & libc::O_ACCMODE != libc::O_RDONLY || flags & libc::O_PATH != 0 {
        return Err(ProcessError::Invalid);
    }
    Ok(file.try_clone()?)
}
fn check_stream(m: &Metadata) -> Result<(), ProcessError> {
    if !m.is_file() || m.nlink() != 1 || m.mode() & 0o7777 != 0o600 {
        return Err(ProcessError::Invalid);
    }
    if m.len() > STREAM_BYTES {
        return Err(ProcessError::Budget);
    }
    Ok(())
}
fn check_directory(parent: &File, retained: &File, name: &str) -> Result<(), ProcessError> {
    let named = child(parent, name, true)?;
    let meta = retained.metadata()?;
    if !meta.is_dir()
        || meta.dev() != parent.metadata()?.dev()
        || FileId::of(&meta) != FileId::of(&named.metadata()?)
    {
        return Err(ProcessError::Drift);
    }
    Ok(())
}
fn magic_target(directory: &File, name: &str) -> Result<File, ProcessError> {
    // Fixed procfs magic links only. Ordinary path readers keep O_NOFOLLOW.
    if !matches!(name, "exe" | "1" | "2") {
        return Err(ProcessError::Invalid);
    }
    let name = CString::new(name).map_err(|_| ProcessError::Invalid)?;
    // SAFETY: owned original proc-directory FD and fixed NUL-terminated name.
    // Intentionally follow magic link with O_PATH; never open target for IO.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_PATH | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: openat returned a new unowned descriptor.
    let file = unsafe { File::from_raw_fd(fd) };
    if !file.metadata()?.is_file() {
        return Err(ProcessError::Invalid);
    }
    Ok(file)
}
fn parse_fdinfo(bytes: &[u8]) -> Result<(u64, u64), ProcessError> {
    if bytes.len() > DOCUMENT_BYTES {
        return Err(ProcessError::Budget);
    }
    if bytes.is_empty() || !bytes.ends_with(b"\n") || bytes.contains(&0) || bytes.contains(&b'\r') {
        return Err(ProcessError::Invalid);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ProcessError::Invalid)?;
    let mut flags = None;
    let mut ino = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("flags:\t") {
            if flags.is_some()
                || value.is_empty()
                || !value.bytes().all(|b| (b'0'..=b'7').contains(&b))
            {
                return Err(ProcessError::Invalid);
            }
            let parsed = u64::from_str_radix(value, 8).map_err(|_| ProcessError::Invalid)?;
            if parsed > u32::MAX as u64 {
                return Err(ProcessError::Invalid);
            }
            flags = Some(parsed);
        } else if let Some(value) = line.strip_prefix("ino:\t") {
            if ino.is_some() {
                return Err(ProcessError::Invalid);
            }
            let parsed = super::number(value.as_bytes())?;
            if parsed == 0 {
                return Err(ProcessError::Invalid);
            }
            ino = Some(parsed);
        } else if line.starts_with("flags:") || line.starts_with("ino:") {
            return Err(ProcessError::Invalid);
        }
    }
    Ok((
        flags.ok_or(ProcessError::Invalid)?,
        ino.ok_or(ProcessError::Invalid)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic_process::{pin, read_boot, ExpectedGeneration, ProcFiles, ProcessError};
    use std::fs::{File, OpenOptions};
    use std::io::{Seek, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    struct Helper {
        child: Child,
        dir: tempfile::TempDir,
        exe: File,
        out: File,
        err: File,
    }
    impl Drop for Helper {
        fn drop(&mut self) {
            self.child.stdin.take();
            self.child.wait().unwrap();
        }
    }
    impl Helper {
        fn start(worker: bool, append: bool) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let writer = |name: &str| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .append(append)
                    .mode(0o600)
                    .open(dir.path().join(name))
                    .unwrap()
            };
            let mut command = if worker {
                let mut c = Command::new(std::env::current_exe().unwrap());
                c.args([
                    "--exact",
                    "diagnostic_process::io::tests::worker_fixture",
                    "--nocapture",
                ])
                .env("HYDRACACHE_IO_FIXTURE", dir.path());
                c
            } else {
                Command::new("/bin/cat")
            };
            let child = command
                .stdin(Stdio::piped())
                .stdout(writer("stdout"))
                .stderr(writer("stderr"))
                .spawn()
                .unwrap();
            let exe = File::open(if worker {
                std::env::current_exe().unwrap()
            } else {
                "/bin/cat".into()
            })
            .unwrap();
            let mut helper = Self {
                child,
                exe,
                out: File::open(dir.path().join("stdout")).unwrap(),
                err: File::open(dir.path().join("stderr")).unwrap(),
                dir,
            };
            if worker {
                helper.wait_bytes("stderr", b"ready\n");
            } else {
                helper
                    .child
                    .stdin
                    .as_mut()
                    .unwrap()
                    .write_all(b"ready\n")
                    .unwrap();
                helper.wait_bytes("stdout", b"ready\n");
            }
            helper
        }
        fn wait_bytes(&mut self, name: &str, bytes: &[u8]) {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if std::fs::read(self.dir.path().join(name))
                    .unwrap()
                    .windows(bytes.len())
                    .any(|v| v == bytes)
                {
                    return;
                }
                assert!(
                    self.child.try_wait().unwrap().is_none(),
                    "helper exited before handshake"
                );
                assert!(
                    Instant::now() < deadline,
                    "helper {name} handshake {bytes:?} timed out"
                );
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        fn process(&self) -> crate::diagnostic_process::ProcessRead {
            let files = ProcFiles::open(self.child.id()).unwrap();
            let fields = files.inspect().unwrap();
            pin(ExpectedGeneration {
                boot_id: read_boot(&files.root).unwrap(),
                pid: self.child.id(),
                start_ticks: fields.stat.start_ticks,
                cgroup_path: fields.cgroup,
            })
            .unwrap()
        }
    }

    // Only an owned unittest child uses this control protocol. No CLI seam or
    // product binary, systemd unit, signal or campaign is involved.
    #[test]
    fn worker_fixture() {
        let Some(dir) = std::env::var_os("HYDRACACHE_IO_FIXTURE") else {
            return;
        };
        std::io::stderr().write_all(b"ready\n").unwrap();
        let mut byte = [0];
        // Avoid Rust's buffered stdin: unread bytes must survive exec.
        // SAFETY: owned child's stdin, initialized one-byte destination.
        while unsafe { libc::read(0, byte.as_mut_ptr().cast(), 1) } == 1 {
            match byte[0] {
                b'r' => {
                    let replacement = OpenOptions::new()
                        .create_new(true)
                        .append(true)
                        .mode(0o600)
                        .open(std::path::Path::new(&dir).join("replacement"))
                        .unwrap();
                    // SAFETY: owned helper's live descriptor, target is its own stdout.
                    assert_eq!(unsafe { libc::dup2(replacement.as_raw_fd(), 1) }, 1);
                    std::io::stderr().write_all(b"retargeted\n").unwrap();
                }
                b'e' => panic!("exec failed: {}", Command::new("/bin/cat").exec()),
                b'a' => {
                    // SAFETY: read/update only this owned helper's stdout flags.
                    let flags = unsafe { libc::fcntl(1, libc::F_GETFL) };
                    assert!(flags >= 0);
                    assert_eq!(
                        unsafe { libc::fcntl(1, libc::F_SETFL, flags & !libc::O_APPEND) },
                        0
                    );
                    std::io::stderr().write_all(b"append-cleared\n").unwrap();
                }
                b'p' => {
                    let mut pipe = [-1; 2];
                    // SAFETY: initialized two-FD destination; helper owns both
                    // ends, keeps the read end until exit and never writes data.
                    assert_eq!(
                        unsafe { libc::pipe2(pipe.as_mut_ptr(), libc::O_CLOEXEC) },
                        0
                    );
                    assert_eq!(unsafe { libc::dup2(pipe[1], 1) }, 1);
                    unsafe {
                        libc::close(pipe[1]);
                    }
                    std::io::stderr().write_all(b"pipe-retargeted\n").unwrap();
                }
                _ => panic!("unknown fixture command"),
            }
        }
    }

    #[test]
    fn original_exe_and_append_outputs_accept_growth_without_shared_seek() {
        let mut h = Helper::start(false, true);
        let p = h.process();
        h.out.seek(std::io::SeekFrom::Start(2)).unwrap();
        let mut guard = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
        guard.revalidate().unwrap();
        for target in &guard.targets {
            // SAFETY: retained live descriptor, fcntl query only.
            assert_ne!(
                unsafe { libc::fcntl(target.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
                0
            );
        }
        h.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"growth\n")
            .unwrap();
        h.wait_bytes("stdout", b"growth\n");
        guard.revalidate().unwrap();
        assert_eq!(h.out.stream_position().unwrap(), 2);
        assert!(!guard.is_refused());
        assert!(p.pin_io(&guard.targets[0], &h.out, &h.err).is_err());
    }

    #[test]
    fn wrong_exe_alias_swapped_outputs_and_writable_expectations_refuse() {
        let h = Helper::start(false, true);
        let p = h.process();
        assert!(p
            .pin_io(&File::open("/bin/true").unwrap(), &h.out, &h.err)
            .is_err());
        assert!(p.pin_io(&h.exe, &h.err, &h.out).is_err());
        assert!(p.pin_io(&h.exe, &h.out, &h.out).is_err());
        let writable = OpenOptions::new()
            .write(true)
            .open(h.dir.path().join("stdout"))
            .unwrap();
        assert!(p.pin_io(&h.exe, &writable, &h.err).is_err());
    }

    #[test]
    fn outputs_require_append_regular_single_link_mode_and_bounded_length() {
        let h = Helper::start(false, false);
        let p = h.process();
        assert!(p.pin_io(&h.exe, &h.out, &h.err).is_err());
        drop(h);
        let h = Helper::start(false, true);
        let p = h.process();
        std::fs::set_permissions(
            h.dir.path().join("stdout"),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(p.pin_io(&h.exe, &h.out, &h.err).is_err());
        std::fs::set_permissions(
            h.dir.path().join("stdout"),
            std::fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        std::fs::hard_link(h.dir.path().join("stdout"), h.dir.path().join("alias")).unwrap();
        assert!(p.pin_io(&h.exe, &h.out, &h.err).is_err());
        std::fs::remove_file(h.dir.path().join("alias")).unwrap();
        let writer = OpenOptions::new()
            .write(true)
            .open(h.dir.path().join("stdout"))
            .unwrap();
        writer.set_len(STREAM_BYTES + 1).unwrap();
        assert!(matches!(
            p.pin_io(&h.exe, &h.out, &h.err),
            Err(ProcessError::Budget)
        ));
    }

    #[test]
    fn observed_shrink_budget_and_exit_are_sticky() {
        let mut h = Helper::start(false, true);
        let p = h.process();
        let mut shrink = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
        let writer = OpenOptions::new()
            .write(true)
            .open(h.dir.path().join("stdout"))
            .unwrap();
        writer.set_len(0).unwrap();
        assert!(shrink.revalidate().is_err());
        writer.set_len(6).unwrap();
        assert!(shrink.revalidate().is_err());
        assert!(shrink.is_refused());
        let mut budget = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
        writer.set_len(STREAM_BYTES + 1).unwrap();
        assert!(matches!(budget.revalidate(), Err(ProcessError::Budget)));
        writer.set_len(6).unwrap();
        assert!(budget.revalidate().is_err());
        let mut exit = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
        h.child.stdin.take();
        h.child.wait().unwrap();
        assert!(matches!(exit.revalidate(), Err(ProcessError::NotLive)));
        assert!(exit.revalidate().is_err());
    }

    #[test]
    fn same_generation_fd_retarget_and_exec_refuse_original_objects() {
        let mut h = Helper::start(true, true);
        let p = h.process();
        let mut guard = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
        h.child.stdin.as_mut().unwrap().write_all(b"r").unwrap();
        h.wait_bytes("stderr", b"retargeted\n");
        p.revalidate().unwrap();
        assert!(guard.revalidate().is_err());
        assert!(guard.is_refused());
        drop(h);
        let mut h = Helper::start(true, true);
        let p = h.process();
        let mut guard = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
        h.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(b"eexec-complete\n")
            .unwrap();
        h.wait_bytes("stdout", b"exec-complete\n");
        p.revalidate().unwrap();
        assert!(guard.revalidate().is_err());
        assert!(guard.is_refused());
    }

    #[test]
    fn same_generation_append_flag_and_nonregular_fd_drift_refuse() {
        for (command, acknowledgement) in [
            (b'a', b"append-cleared\n".as_slice()),
            (b'p', b"pipe-retargeted\n".as_slice()),
        ] {
            let mut h = Helper::start(true, true);
            let p = h.process();
            let mut guard = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
            h.child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(&[command])
                .unwrap();
            h.wait_bytes("stderr", acknowledgement);
            p.revalidate().unwrap();
            assert!(guard.revalidate().is_err());
            assert!(guard.is_refused());
        }
    }

    #[test]
    fn object_binding_does_not_certify_output_pathnames() {
        let h = Helper::start(false, true);
        let p = h.process();
        let mut guard = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
        std::fs::rename(
            h.dir.path().join("stdout"),
            h.dir.path().join("original-output"),
        )
        .unwrap();
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(h.dir.path().join("stdout"))
            .unwrap();
        // Still the same single-linked original object. A future fixed named
        // output reader must separately refuse the replacement pathname.
        guard.revalidate().unwrap();
        assert!(!guard.is_refused());
    }

    #[test]
    fn parallel_original_io_readers_allow_concurrent_append_without_seeking() {
        let mut h = Helper::start(false, true);
        let p = h.process();
        let mut stdin = h.child.stdin.take().unwrap();
        std::thread::scope(|scope| {
            let writer = scope.spawn(move || {
                for _ in 0..256 {
                    stdin.write_all(b"append\n").unwrap();
                }
                stdin
            });
            let mut readers = Vec::new();
            for _ in 0..4 {
                readers.push(scope.spawn(|| {
                    let mut io = p.pin_io(&h.exe, &h.out, &h.err).unwrap();
                    for _ in 0..32 {
                        io.revalidate().unwrap();
                    }
                }));
            }
            for reader in readers {
                reader.join().unwrap();
            }
            h.child.stdin = Some(writer.join().unwrap());
        });
        assert_eq!(h.out.stream_position().unwrap(), 0);
        assert_eq!(h.err.stream_position().unwrap(), 0);
    }

    #[test]
    fn fdinfo_required_fields_are_unique_strict_and_bounded() {
        assert_eq!(
            parse_fdinfo(b"pos:\t0\nflags:\t0102001\nmnt_id:\t23\nino:\t42\n").unwrap(),
            (0o102001, 42)
        );
        for bytes in [
            &b"flags:\t02001\nino:\t42\nino:\t42\n"[..],
            &b"flags:\t02001\nflags:\t02001\nino:\t42\n"[..],
            &b"flags:\t02009\nino:\t42\n"[..],
            &b"flags:\t02001\nino:\t0\n"[..],
            &b"flags:\t-1\nino:\t42\n"[..],
            &b"flags:\t02001\nino:\t42"[..],
            &b"flags:\t02001\nino:\t18446744073709551616\n"[..],
            &b"flags:\t02001\nino:\t42\0\n"[..],
            &b"ino:\t42\n"[..],
            &b"flags:\t02001\r\nino:\t42\n"[..],
        ] {
            assert!(parse_fdinfo(bytes).is_err(), "accepted {bytes:?}");
        }
        assert!(matches!(
            parse_fdinfo(&vec![b'x'; 65_537]),
            Err(ProcessError::Budget)
        ));
        for ino in 1..=256 {
            let bytes = format!("pos:\t999\nflags:\t02001\nino:\t{ino}\n");
            assert_eq!(parse_fdinfo(bytes.as_bytes()).unwrap(), (0o2001, ino));
        }
    }
}
