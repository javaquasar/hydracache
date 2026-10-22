//! Value-only census. Contributions retain files with their original owners.
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};

fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "held role census")
}
fn need(value: bool) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(refused())
    }
}
#[derive(PartialEq, Eq)]
struct Stamp {
    fd: i32,
    stat: [u64; 11],
    flags: [i32; 2],
    offset: Option<i64>,
}
fn stamp(file: &File) -> io::Result<Stamp> {
    let fd = file.as_raw_fd();
    let m = file.metadata()?;
    // SAFETY: query-only fcntls on the provider's still-owned file.
    let flags = unsafe {
        [
            libc::fcntl(fd, libc::F_GETFD),
            libc::fcntl(fd, libc::F_GETFL),
        ]
    };
    need(fd >= 0 && flags[0] == libc::FD_CLOEXEC && flags[1] >= 0 && flags[1] & libc::O_PATH == 0)?;
    let offset = if m.is_file() {
        let offset = unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) };
        need(offset >= 0)?;
        Some(offset)
    } else {
        None
    };
    Ok(Stamp {
        fd,
        stat: [
            m.dev(),
            m.ino(),
            u64::from(m.uid()),
            u64::from(m.gid()),
            u64::from(m.mode()),
            m.nlink(),
            m.len(),
            m.mtime() as u64,
            m.mtime_nsec() as u64,
            m.ctime() as u64,
            m.ctime_nsec() as u64,
        ],
        flags,
        offset,
    })
}
fn pidfd(file: &File, pid: i32, identity: [u64; 2]) -> io::Result<Stamp> {
    let before = stamp(file)?;
    need(before.stat[..2] == identity)?;
    let fd = file.as_raw_fd();
    let mut poll = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    need(unsafe { libc::poll(&mut poll, 1, 0) } == 0 && poll.revents == 0)?;
    // Fixed procfs path for THIS retained descriptor, never a PID reopen.
    let info = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(format!("/proc/thread-self/fdinfo/{fd}"))?;
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    need(
        unsafe { libc::fstatfs(info.as_raw_fd(), &mut fs) } == 0
            && fs.f_type == libc::PROC_SUPER_MAGIC as libc::c_long,
    )?;
    let mut raw = [0; 4097];
    let n = info.read_at(&mut raw, 0)?;
    need(n > 0 && n <= 4096)?;
    let text = std::str::from_utf8(&raw[..n]).map_err(|_| refused())?;
    let mut values = text.lines().filter_map(|line| line.strip_prefix("Pid:"));
    need(
        values.next().and_then(|v| v.trim().parse::<i32>().ok()) == Some(pid)
            && values.next().is_none(),
    )?;
    need(
        unsafe { libc::poll(&mut poll, 1, 0) } == 0 && poll.revents == 0 && stamp(file)? == before,
    )?;
    Ok(before)
}

// No Debug/Clone/serde/raw-FD/borrow getter: this cannot disclose or carry files.
#[derive(PartialEq, Eq)]
pub(crate) struct CompleteBindings {
    roles: [i32; 11],
    extra: [Stamp; 3],
}
pub(crate) struct Census {
    worker_identity: [u64; 3],
    worker_digest: String,
    guardians: [(i32, [u64; 2]); 2],
    worker: Option<Stamp>,
    guards: Option<[Stamp; 2]>,
    failed: bool,
}
impl Census {
    pub(crate) fn new(
        worker_identity: [u64; 3],
        worker_digest: &str,
        guardians: [(i32, [u64; 2]); 2],
        creator: i64,
    ) -> io::Result<Self> {
        need(
            worker_identity.iter().all(|v| *v > 0)
                && crate::is_hash(worker_digest)
                && creator > 1
                && guardians[0].0 > 1
                && guardians[1].0 > 1
                && guardians[0].0 != guardians[1].0
                // Older kernels share the anon_inode across distinct pidfds.
                // The retained FD number plus fresh fdinfo PID and liveness
                // identify each task; inode inequality is not a task identity.
                && guardians
                    .iter()
                    .all(|g| i64::from(g.0) != creator && g.1.iter().all(|v| *v > 0)),
        )?;
        Ok(Self {
            worker_identity,
            worker_digest: worker_digest.into(),
            guardians,
            worker: None,
            guards: None,
            failed: false,
        })
    }
    pub(crate) fn reject(&mut self) {
        self.failed = true;
    }
    pub(crate) fn worker(&mut self, file: &File, digest: &str) -> io::Result<()> {
        let result = (|| {
            need(
                !self.failed
                    && self.worker.is_none()
                    && self.guards.is_none()
                    && digest == self.worker_digest,
            )?;
            let before = stamp(file)?;
            need(
                [before.stat[0], before.stat[1], before.stat[6]] == self.worker_identity
                    && before.stat[4] as u32 & libc::S_IFMT == libc::S_IFREG
                    && before.stat[6] <= super::MAX_BINARY_BYTES
                    && before.flags[1] & libc::O_ACCMODE == libc::O_RDONLY,
            )?;
            let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
            let exact =
                libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
            need(
                seals == exact
                    || (before.stat[2] == 0
                        && before.stat[4] & 0o022 == 0
                        && before.stat[4] & 0o111 != 0),
            )?;
            need(stamp(file)? == before)?;
            self.worker = Some(before);
            Ok(())
        })();
        if result.is_err() {
            self.reject();
        }
        result
    }
    pub(crate) fn guardians(&mut self, files: [(i32, &File); 2]) -> io::Result<()> {
        let result = (|| {
            need(!self.failed && self.worker.is_some() && self.guards.is_none())?;
            need(files[0].0 == self.guardians[0].0 && files[1].0 == self.guardians[1].0)?;
            let a = pidfd(files[0].1, files[0].0, self.guardians[0].1)?;
            let b = pidfd(files[1].1, files[1].0, self.guardians[1].1)?;
            need(
                a.fd != b.fd
                    && self
                        .worker
                        .as_ref()
                        .is_some_and(|w| w.fd != a.fd && w.fd != b.fd),
            )?;
            self.guards = Some([a, b]);
            Ok(())
        })();
        if result.is_err() {
            self.reject();
        }
        result
    }
    // Claims/value join only. A detached caller cannot create the original owner.
    pub(crate) fn complete(
        self,
        partial: [(u8, i32); 8],
        original: [i32; 20],
    ) -> io::Result<CompleteBindings> {
        need(!self.failed)?;
        let worker = self.worker.ok_or_else(refused)?;
        let [a, b] = self.guards.ok_or_else(refused)?;
        let extras = [worker, a, b];
        let mut fds = [0; 23];
        fds[..20].copy_from_slice(&original);
        for (at, stamp) in extras.iter().enumerate() {
            fds[20 + at] = stamp.fd;
        }
        need(
            fds.iter()
                .enumerate()
                .all(|(i, fd)| *fd >= 0 && !fds[..i].contains(fd)),
        )?;
        let expected = [
            (0, 14),
            (1, 6),
            (2, 7),
            (3, 16),
            (5, 17),
            (6, 19),
            (9, 1),
            (10, 18),
        ];
        need(partial == expected.map(|(slot, at)| (slot, original[at])))?;
        let mut roles = [-1; 11];
        for (slot, fd) in partial {
            roles[slot as usize] = fd;
        }
        for (slot, s) in [4, 7, 8].into_iter().zip(&extras) {
            roles[slot] = s.fd;
        }
        Ok(CompleteBindings {
            roles,
            extra: extras,
        })
    }
}

#[cfg(test)]
#[path = "diagnostic_held_role_census_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
impl CompleteBindings {
    pub(crate) fn model_for_test() -> Self {
        Self {
            roles: [0; 11],
            extra: std::array::from_fn(|_| Stamp {
                fd: 0,
                stat: [0; 11],
                flags: [0; 2],
                offset: None,
            }),
        }
    }
    pub(crate) fn tamper_for_test(&mut self) {
        self.extra[0].stat[1] += 1;
    }
}
