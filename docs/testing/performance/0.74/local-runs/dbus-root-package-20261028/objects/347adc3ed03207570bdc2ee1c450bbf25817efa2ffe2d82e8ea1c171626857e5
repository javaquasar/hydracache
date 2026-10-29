//! Child-only selected root mechanics; no signer exec, credential or admission.
use super::diagnostic_signer_runtime_fixture::{read_bounded, validate_images};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const MAX_IMAGE: usize = 8 * 1024 * 1024;
const DIRS: [&str; 4] = ["", "lib", "lib/x86_64-linux-gnu", "lib64"];
const FILES: [&str; 4] = [
    "signer",
    "lib64/ld-linux-x86-64.so.2",
    "lib/x86_64-linux-gnu/libc.so.6",
    "lib/x86_64-linux-gnu/libgcc_s.so.1",
];
const STOP: i32 = (libc::SIGSTOP << 8) | 0x7f;
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "owned runtime root refused")
}
#[derive(Clone, PartialEq, Eq)]
struct Stamp([u64; 11]);
impl Stamp {
    fn of(m: Metadata) -> Self {
        Self([
            m.dev(),
            m.ino(),
            m.uid() as u64,
            m.gid() as u64,
            m.mode() as u64,
            m.nlink(),
            m.len(),
            m.mtime() as u64,
            m.mtime_nsec() as u64,
            m.ctime() as u64,
            m.ctime_nsec() as u64,
        ])
    }
    fn identity(&self) -> [u64; 2] {
        [self.0[0], self.0[1]]
    }
}
fn open(path: &Path, directory: bool) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(
            libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_NONBLOCK
                | if directory { libc::O_DIRECTORY } else { 0 },
        )
        .open(path)
}
fn readonly(file: &File, directory: bool, uid: u32, gid: u32) -> io::Result<Stamp> {
    let m = file.metadata()?;
    // SAFETY: fixed flag queries on a borrowed live descriptor, no ownership transfer.
    let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
    let fd_flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
    if flags < 0
        || fd_flags < 0
        || flags & libc::O_ACCMODE != libc::O_RDONLY
        || flags & libc::O_PATH != 0
        || fd_flags & libc::FD_CLOEXEC == 0
        || m.uid() != uid
        || m.gid() != gid
        || m.mode() & 0o7777 != 0o555
        || if directory {
            !m.is_dir()
        } else {
            !m.is_file() || m.nlink() != 1 || m.len() < 64 || m.len() > MAX_IMAGE as u64
        }
    {
        return Err(invalid());
    }
    Ok(Stamp::of(m))
}
struct Bound {
    file: File,
    stamp: Stamp,
    hash: Vec<u8>,
}
struct Root {
    _temp: tempfile::TempDir,
    path: PathBuf,
    dirs: Vec<Bound>,
    files: Vec<Bound>,
    uid: u32,
    gid: u32,
    refused: bool,
    attempted: bool,
}
impl Root {
    fn new() -> io::Result<Self> {
        // SAFETY: local nonroot identity queries, no account/provider selection.
        let (uid, gid) = unsafe { (libc::geteuid(), libc::getegid()) };
        if uid == 0 {
            return Err(invalid());
        }
        static IMAGES: OnceLock<Result<[Vec<u8>; 4], String>> = OnceLock::new();
        let images = IMAGES.get_or_init(|| {
            let prepare = || -> io::Result<[Vec<u8>; 4]> {
                Ok([
                    super::diagnostic_signer_joined_fixture::compiled_helper()?,
                    read_bounded(&File::open("/lib64/ld-linux-x86-64.so.2")?)?,
                    read_bounded(&File::open("/lib/x86_64-linux-gnu/libc.so.6")?)?,
                    read_bounded(&File::open("/lib/x86_64-linux-gnu/libgcc_s.so.1")?)?,
                ])
            };
            prepare().map_err(|e| e.to_string())
        });
        let images = images.as_ref().map_err(|e| io::Error::other(e.clone()))?;
        Self::materialize(images, uid, gid)
    }
    fn materialize(images: &[Vec<u8>; 4], uid: u32, gid: u32) -> io::Result<Self> {
        if uid == 0 {
            return Err(invalid());
        }
        validate_images(images)?;
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("root");
        let mut root = Self {
            _temp: temp,
            path,
            dirs: Vec::new(),
            files: Vec::new(),
            uid,
            gid,
            refused: false,
            attempted: false,
        };
        for dir in DIRS {
            fs::create_dir(root.path.join(dir))?;
        }
        for (name, bytes) in FILES.iter().zip(images) {
            // No writable executable/library description in the parent fork table.
            let mut copy = Command::new("/usr/bin/tee")
                .arg("--")
                .arg(root.path.join(name))
                .env_clear()
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            let sent = copy.stdin.take().ok_or_else(invalid)?.write_all(bytes);
            let status = copy.wait()?;
            sent?;
            if !status.success() {
                return Err(invalid());
            }
            fs::set_permissions(root.path.join(name), fs::Permissions::from_mode(0o555))?;
            let file = open(&root.path.join(name), false)?;
            let stamp = readonly(&file, false, uid, gid)?;
            if read_bounded(&file)? != *bytes {
                return Err(invalid());
            }
            root.files.push(Bound {
                file,
                stamp,
                hash: Sha256::digest(bytes).to_vec(),
            });
        }
        for dir in DIRS {
            let file = open(&root.path.join(dir), true)?;
            // SAFETY: owned original directory only, no substituted pathname followed.
            if unsafe { libc::fchmod(file.as_raw_fd(), 0o555) } < 0 {
                return Err(io::Error::last_os_error());
            }
            let stamp = readonly(&file, true, uid, gid)?;
            root.dirs.push(Bound {
                file,
                stamp,
                hash: Vec::new(),
            });
        }
        root.check()?;
        Ok(root)
    }
    fn check(&mut self) -> io::Result<()> {
        if self.refused {
            return Err(invalid());
        }
        let result = self.check_inner();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    fn check_inner(&self) -> io::Result<()> {
        if self.dirs.len() != 4 || self.files.len() != 4 {
            return Err(invalid());
        }
        for (index, dir) in DIRS.iter().enumerate() {
            let bound = &self.dirs[index];
            let named = open(&self.path.join(dir), true)?;
            if readonly(&bound.file, true, self.uid, self.gid)? != bound.stamp
                || readonly(&named, true, self.uid, self.gid)? != bound.stamp
            {
                return Err(invalid());
            }
            let expected: BTreeSet<_> = match index {
                0 => vec!["signer", "lib", "lib64"],
                1 => vec!["x86_64-linux-gnu"],
                2 => vec!["libc.so.6", "libgcc_s.so.1"],
                _ => vec!["ld-linux-x86-64.so.2"],
            }
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect();
            let entries = fs::read_dir(format!("/proc/self/fd/{}", bound.file.as_raw_fd()))?
                .take(5)
                .map(|e| e.map(|e| e.file_name()))
                .collect::<io::Result<BTreeSet<_>>>()?;
            if entries != expected
                || readonly(&bound.file, true, self.uid, self.gid)? != bound.stamp
            {
                return Err(invalid());
            }
        }
        let mut images = Vec::with_capacity(4);
        for (index, name) in FILES.iter().enumerate() {
            let bound = &self.files[index];
            let named = open(&self.path.join(name), false)?;
            if readonly(&named, false, self.uid, self.gid)? != bound.stamp
                || readonly(&bound.file, false, self.uid, self.gid)? != bound.stamp
            {
                return Err(invalid());
            }
            let bytes = read_bounded(&bound.file)?;
            if Sha256::digest(&bytes).as_slice() != bound.hash
                || readonly(&named, false, self.uid, self.gid)? != bound.stamp
                || readonly(&bound.file, false, self.uid, self.gid)? != bound.stamp
            {
                return Err(invalid());
            }
            images.push(bytes);
        }
        validate_images(&images.try_into().map_err(|_| invalid())?)?;
        for bound in &self.dirs {
            if readonly(&bound.file, true, self.uid, self.gid)? != bound.stamp {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        // Only retained original owned directories. Never traverse replacement ancestors.
        for dir in self.dirs.iter().rev() {
            // SAFETY: original owned directory FD; no path lookup or exported descriptor.
            if unsafe { libc::fchmod(dir.file.as_raw_fd(), 0o755) } < 0 {
                eprintln!(
                    "owned root cleanup preparation failed: {}",
                    io::Error::last_os_error()
                );
            }
        }
    }
}
fn text(path: impl AsRef<Path>) -> io::Result<String> {
    let mut raw = String::new();
    File::open(path)?.take(65537).read_to_string(&mut raw)?;
    if raw.len() > 65536 {
        return Err(invalid());
    }
    Ok(raw)
}
fn mapping(raw: &str, id: u32) -> io::Result<()> {
    if raw.len() > 4096 {
        return Err(invalid());
    }
    let values = raw
        .split_whitespace()
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid())?;
    if values != [id, id, 1] {
        return Err(invalid());
    }
    Ok(())
}
fn credentials(raw: &str, uid: u32, gid: u32) -> io::Result<()> {
    if raw.len() > 65536 || uid == 0 {
        return Err(invalid());
    }
    let mut found = BTreeSet::new();
    for line in raw.lines() {
        if let Some((key, value)) = line.split_once(':') {
            match key {
                "Uid" | "Gid" => {
                    if !found.insert(key) {
                        return Err(invalid());
                    }
                    let ids = value
                        .split_whitespace()
                        .map(str::parse::<u32>)
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| invalid())?;
                    if ids != [if key == "Uid" { uid } else { gid }; 4] {
                        return Err(invalid());
                    }
                }
                "CapInh" | "CapPrm" | "CapEff" | "CapBnd" | "CapAmb" => {
                    if !found.insert(key) || value.trim() != "0000000000000000" {
                        return Err(invalid());
                    }
                }
                "NoNewPrivs" => {
                    if !found.insert(key) || value.trim() != "1" {
                        return Err(invalid());
                    }
                }
                _ => {}
            }
        }
    }
    if found.len() != 8 {
        return Err(invalid());
    }
    Ok(())
}
fn magic(path: impl AsRef<Path>) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
        .open(path)
}
fn identity(file: &File) -> io::Result<[u64; 2]> {
    let m = file.metadata()?;
    Ok([m.dev(), m.ino()])
}
#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Initial,
    Rooted,
    Terminal,
    Refused,
}
struct Owner<'a> {
    root: &'a mut Root,
    pid: libc::pid_t,
    pidfd: Option<OwnedFd>,
    report: File,
    writer: i32,
    namespace: Option<File>,
    parent_namespace: File,
    reaped: bool,
    phase: Phase,
    _thread: PhantomData<Rc<()>>,
}
impl<'a> Owner<'a> {
    fn new(root: &'a mut Root) -> io::Result<Self> {
        root.check()?;
        if root.attempted {
            root.refused = true;
            return Err(invalid());
        }
        root.attempted = true;
        // SAFETY: parent queries only, no credential mutation.
        if unsafe { libc::getuid() } != root.uid || unsafe { libc::getgid() } != root.gid {
            return Err(invalid());
        }
        let last = text("/proc/sys/kernel/cap_last_cap")?
            .trim()
            .parse::<u32>()
            .map_err(|_| invalid())?;
        if last > 63 {
            return Err(invalid());
        }
        let parent_namespace = magic("/proc/self/ns/user")?;
        let null = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open("/dev/null")?;
        let mut pipe = [-1; 2];
        // SAFETY: initialized two-FD output, flags fixed; each successful FD owned once.
        if unsafe { libc::pipe2(pipe.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: distinct fresh pipe descriptors transferred exactly once.
        let (report, writer) =
            unsafe { (File::from_raw_fd(pipe[0]), OwnedFd::from_raw_fd(pipe[1])) };
        let fds = [
            null.as_raw_fd(),
            root.dirs[0].file.as_raw_fd(),
            writer.as_raw_fd(),
        ];
        if fds.iter().any(|fd| *fd < 3) {
            return Err(invalid());
        }
        // SAFETY: prepared scalars; child uses only libc/_exit, no Rust locks/allocation.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid == 0 {
            child(fds, last);
        }
        drop(writer);
        drop(null);
        let mut owner = Self {
            root,
            pid,
            pidfd: None,
            report,
            writer: pipe[1],
            namespace: None,
            parent_namespace,
            reaped: false,
            phase: Phase::Initial,
            _thread: PhantomData,
        };
        // SAFETY: own unreaped direct child, never caller PID adoption.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: fresh pidfd transferred once.
        owner.pidfd = Some(unsafe { OwnedFd::from_raw_fd(fd as i32) });
        if owner.wait()? != STOP {
            return Err(invalid());
        }
        owner.trace(
            libc::PTRACE_SETOPTIONS,
            (libc::PTRACE_O_EXITKILL | libc::PTRACE_O_TRACEEXEC) as usize,
        )?;
        Ok(owner)
    }
    fn trace(&self, op: u32, data: usize) -> io::Result<()> {
        // SAFETY: fixed ptrace scalar operation on owned stopped child only.
        if unsafe { libc::ptrace(op, self.pid, 0usize, data) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    fn wait(&mut self) -> io::Result<i32> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if self.reaped {
                return Err(invalid());
            }
            let mut status = 0;
            // SAFETY: exact original child only; no wait-any.
            let found =
                unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) };
            if found == self.pid {
                self.reaped = libc::WIFEXITED(status) || libc::WIFSIGNALED(status);
                return Ok(status);
            }
            if found < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
                return Err(io::Error::last_os_error());
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "owned root child stalled",
                ));
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    fn enter(&mut self) -> io::Result<()> {
        let result = self.enter_inner();
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn enter_inner(&mut self) -> io::Result<()> {
        if self.phase != Phase::Initial {
            return Err(invalid());
        }
        self.root.check()?;
        self.trace(libc::PTRACE_CONT, 0)?;
        if self.wait()? != STOP {
            return Err(invalid());
        }
        let ns = magic(format!("/proc/{}/ns/user", self.pid))?;
        if identity(&ns)? == identity(&self.parent_namespace)? {
            return Err(invalid());
        }
        for field in ["uid_map", "gid_map"] {
            if !text(format!("/proc/{}/{field}", self.pid))?.is_empty() {
                return Err(invalid());
            }
        }
        for (field, value) in [
            ("setgroups", "deny\n".to_owned()),
            (
                "uid_map",
                format!("{} {} 1\n", self.root.uid, self.root.uid),
            ),
            (
                "gid_map",
                format!("{} {} 1\n", self.root.gid, self.root.gid),
            ),
        ] {
            OpenOptions::new()
                .write(true)
                .open(format!("/proc/{}/{field}", self.pid))?
                .write_all(value.as_bytes())?;
        }
        self.namespace = Some(ns);
        self.trace(libc::PTRACE_CONT, 0)?;
        if self.wait()? != STOP {
            return Err(invalid());
        }
        self.rooted()?;
        self.phase = Phase::Rooted;
        Ok(())
    }
    fn rooted(&mut self) -> io::Result<()> {
        self.root.check()?;
        let ns = magic(format!("/proc/{}/ns/user", self.pid))?;
        if identity(&ns)? != identity(self.namespace.as_ref().ok_or_else(invalid)?)? {
            return Err(invalid());
        }
        mapping(&text(format!("/proc/{}/uid_map", self.pid))?, self.root.uid)?;
        mapping(&text(format!("/proc/{}/gid_map", self.pid))?, self.root.gid)?;
        if text(format!("/proc/{}/setgroups", self.pid))? != "deny\n" {
            return Err(invalid());
        }
        for name in ["root", "cwd"] {
            if Stamp::of(magic(format!("/proc/{}/{name}", self.pid))?.metadata()?)
                != self.root.dirs[0].stamp
            {
                return Err(invalid());
            }
        }
        credentials(
            &text(format!("/proc/{}/status", self.pid))?,
            self.root.uid,
            self.root.gid,
        )?;
        let tasks = fs::read_dir(format!("/proc/{}/task", self.pid))?
            .take(2)
            .map(|e| e.map(|e| e.file_name()))
            .collect::<io::Result<Vec<_>>>()?;
        if tasks != [std::ffi::OsString::from(self.pid.to_string())] {
            return Err(invalid());
        }
        let names = fs::read_dir(format!("/proc/{}/fd", self.pid))?
            .take(5)
            .map(|e| e.map(|e| e.file_name()))
            .collect::<io::Result<BTreeSet<_>>>()?;
        if names
            != [
                "0".to_owned(),
                "1".to_owned(),
                "2".to_owned(),
                self.writer.to_string(),
            ]
            .map(std::ffi::OsString::from)
            .into_iter()
            .collect()
        {
            return Err(invalid());
        }
        Ok(())
    }
    fn probe(&mut self) -> io::Result<()> {
        let result = self.probe_inner();
        if result.is_err() {
            self.refuse()?;
        }
        result
    }
    fn probe_inner(&mut self) -> io::Result<()> {
        if self.phase != Phase::Rooted {
            return Err(invalid());
        }
        self.rooted()?;
        self.trace(libc::PTRACE_CONT, 0)?;
        if self.wait()? != 0 {
            return Err(invalid());
        }
        let mut raw = [0u8; 96];
        self.report.read_exact(&mut raw)?;
        let values: Vec<_> = raw
            .chunks_exact(8)
            .map(|p| u64::from_ne_bytes(p.try_into().unwrap()))
            .collect();
        if values[..2] != self.root.dirs[0].stamp.identity() || values[10..] != [31, 0] {
            return Err(invalid());
        }
        for (i, bound) in self.root.files.iter().enumerate() {
            if values[2 + i * 2..4 + i * 2] != bound.stamp.identity() {
                return Err(invalid());
            }
        }
        let mut extra = [0u8; 1];
        if self.report.read(&mut extra)? != 0 {
            return Err(invalid());
        }
        self.root.check()?;
        self.phase = Phase::Terminal;
        Ok(())
    }
    fn refuse(&mut self) -> io::Result<()> {
        self.phase = Phase::Refused;
        self.root.refused = true;
        self.terminate()
    }
    fn terminate(&mut self) -> io::Result<()> {
        if self.reaped {
            return Ok(());
        }
        // SAFETY: exact owned pidfd; PID used only for failed acquisition of own child.
        let result = unsafe {
            match &self.pidfd {
                Some(fd) => libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    0usize,
                    0u32,
                ),
                None => libc::kill(self.pid, libc::SIGKILL) as libc::c_long,
            }
        };
        if result < 0 && io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH) {
            return Err(io::Error::last_os_error());
        }
        while !self.reaped {
            self.wait()?;
        }
        Ok(())
    }
}
impl Drop for Owner<'_> {
    fn drop(&mut self) {
        if let Err(e) = self.terminate() {
            eprintln!("owned root child cleanup failed: {e}");
        }
    }
}
#[repr(C)]
struct CapHeader {
    version: u32,
    pid: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CapData {
    effective: u32,
    permitted: u32,
    inheritable: u32,
}
fn child(fds: [i32; 3], last: u32) -> ! {
    // SAFETY: all inputs prepared; only scalar libc and stack storage after raw fork.
    unsafe {
        if libc::dup2(fds[0], 0) < 0 || libc::dup2(fds[0], 1) < 0 || libc::dup2(fds[0], 2) < 0 {
            libc::_exit(70);
        }
        let lo = fds[1].min(fds[2]) as u32;
        let hi = fds[1].max(fds[2]) as u32;
        let limit = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        if (lo > 3 && libc::syscall(libc::SYS_close_range, 3u32, lo - 1, 0u32) < 0)
            || (hi > lo + 1 && libc::syscall(libc::SYS_close_range, lo + 1, hi - 1, 0u32) < 0)
            || libc::syscall(libc::SYS_close_range, hi + 1, u32::MAX, 0u32) < 0
            || libc::setrlimit(libc::RLIMIT_CORE, &limit) < 0
            || libc::ptrace(libc::PTRACE_TRACEME, 0, 0usize, 0usize) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
            libc::_exit(71);
        }
        if libc::unshare(libc::CLONE_NEWUSER) < 0 || libc::kill(libc::getpid(), libc::SIGSTOP) < 0 {
            libc::_exit(72);
        }
        if libc::fchdir(fds[1]) < 0
            || libc::chroot(c".".as_ptr()) < 0
            || libc::chdir(c"/".as_ptr()) < 0
            || libc::close(fds[1]) < 0
        {
            libc::_exit(73);
        }
        for cap in 0..=last {
            if libc::prctl(libc::PR_CAPBSET_DROP, cap as usize, 0usize, 0usize, 0usize) < 0 {
                libc::_exit(74);
            }
        }
        let header = CapHeader {
            version: 0x2008_0522,
            pid: 0,
        };
        let data = [CapData {
            effective: 0,
            permitted: 0,
            inheritable: 0,
        }; 2];
        if libc::prctl(
            libc::PR_CAP_AMBIENT,
            libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0usize,
            0usize,
            0usize,
        ) < 0
            || libc::syscall(libc::SYS_capset, &header, data.as_ptr()) < 0
            || libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1usize, 0usize, 0usize, 0usize) < 0
            || libc::kill(libc::getpid(), libc::SIGSTOP) < 0
        {
            libc::_exit(75);
        }
        let mut values = [0u64; 12];
        for (i, path) in [
            c"/",
            c"/signer",
            c"/lib64/ld-linux-x86-64.so.2",
            c"/lib/x86_64-linux-gnu/libc.so.6",
            c"/lib/x86_64-linux-gnu/libgcc_s.so.1",
        ]
        .iter()
        .enumerate()
        {
            let fd = libc::open(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            );
            let mut stat: libc::stat = std::mem::zeroed();
            if fd < 0 || libc::fstat(fd, &mut stat) < 0 || libc::close(fd) < 0 {
                libc::_exit(76);
            }
            values[i * 2] = stat.st_dev;
            values[i * 2 + 1] = stat.st_ino;
        }
        for (i, path) in [
            c"/etc/passwd",
            c"/proc/self/root",
            c"/etc/ld.so.cache",
            c"/etc/ld.so.preload",
        ]
        .iter()
        .enumerate()
        {
            if libc::open(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            ) >= 0
                || *libc::__errno_location() != libc::ENOENT
            {
                libc::_exit(77);
            }
            values[10] |= 1 << i;
        }
        let fd = libc::open(
            c"/..".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        );
        let mut stat: libc::stat = std::mem::zeroed();
        if fd < 0
            || libc::fstat(fd, &mut stat) < 0
            || libc::close(fd) < 0
            || [stat.st_dev, stat.st_ino] != values[..2]
        {
            libc::_exit(78);
        }
        values[10] |= 16;
        if libc::write(
            fds[2],
            values.as_ptr().cast(),
            std::mem::size_of_val(&values),
        ) != 96
        {
            libc::_exit(79);
        }
        libc::_exit(0);
    }
}
#[path = "diagnostic_signer_rooted_exec_fixture.rs"]
mod rooted_exec;
#[path = "diagnostic_signer_root_fixture_tests.rs"]
mod tests;
