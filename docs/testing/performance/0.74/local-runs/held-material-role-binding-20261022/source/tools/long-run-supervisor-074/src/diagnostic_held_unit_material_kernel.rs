//! Private bounded kernel construction. No send, fork, exec or inherited mapping.
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};

const SEALS: i32 = libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
const BOOT_BYTES: usize = 256;

pub(super) struct Bindings {
    pub(super) owner: [i64; 6],
    pub(super) deadline: u64,
    pub(super) guardians: [(i32, [u64; 2]); 2],
    pub(super) worker: ([u64; 3], String),
    pub(super) config: ([u64; 3], String),
    pub(super) streams: [[u64; 2]; 2],
}
fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "held material binding")
}
fn need(value: bool) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(refused())
    }
}
fn owned(fd: RawFd) -> io::Result<File> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: each caller passes a newly-created, uniquely owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn current_owner() -> [i64; 6] {
    // SAFETY: read-only process/thread/credential queries.
    unsafe {
        [
            i64::from(libc::getpid()),
            libc::syscall(libc::SYS_gettid),
            i64::from(libc::getuid()),
            i64::from(libc::getgid()),
            i64::from(libc::geteuid()),
            i64::from(libc::getegid()),
        ]
    }
}
fn now() -> io::Result<u64> {
    let mut t: libc::timespec = unsafe { std::mem::zeroed() };
    if unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut t) } != 0 {
        return Err(io::Error::last_os_error());
    }
    need(t.tv_sec >= 0 && (0..1_000_000_000).contains(&t.tv_nsec))?;
    (t.tv_sec as u64)
        .checked_mul(1_000_000_000)
        .and_then(|v| v.checked_add(t.tv_nsec as u64))
        .ok_or_else(refused)
}
fn owner(b: &Bindings) -> io::Result<()> {
    need(
        current_owner() == b.owner
            && b.owner[0] > 1
            && b.deadline > now()?
            && b.deadline <= i64::MAX as u64
            && b.guardians[0].0 > 1
            && b.guardians[1].0 > 1
            && b.guardians[0].0 != b.guardians[1].0
            && b.guardians.iter().all(|g| i64::from(g.0) != b.owner[0]),
    )
}
fn id(file: &File) -> io::Result<[u64; 3]> {
    let m = file.metadata()?;
    Ok([m.dev(), m.ino(), m.len()])
}
fn pair_id(file: &File) -> io::Result<[u64; 2]> {
    let i = id(file)?;
    Ok([i[0], i[1]])
}
fn readonly(file: &File) -> io::Result<()> {
    let fd = file.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    need(
        flags >= 0
            && flags & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY
            && unsafe { libc::fcntl(fd, libc::F_GETFD) } == libc::FD_CLOEXEC
            && unsafe { libc::fcntl(fd, libc::F_GET_SEALS) } == SEALS
            && file.metadata()?.is_file(),
    )
}
fn read(file: &File, maximum: usize) -> io::Result<Vec<u8>> {
    let size = file.metadata()?.len();
    need(size > 0 && size <= maximum as u64)?;
    let mut raw = vec![0; size as usize];
    file.read_exact_at(&mut raw, 0)?;
    Ok(raw)
}
fn sealed(raw: &[u8]) -> io::Result<File> {
    need(!raw.is_empty() && raw.len() <= crate::diagnostic_artifacts::MAX_CONFIG_BYTES as usize)?;
    let mut writer = owned(unsafe {
        libc::memfd_create(
            c"hc074-held-material".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    })?;
    writer.write_all(raw)?;
    need(unsafe { libc::fchmod(writer.as_raw_fd(), 0o400) } == 0)?;
    need(unsafe { libc::fcntl(writer.as_raw_fd(), libc::F_ADD_SEALS, SEALS) } == 0)?;
    // Only this retained descriptor's fixed proc magic link is followed. No
    // caller path. All writable aliases close before the resource is published.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(format!("/proc/thread-self/fd/{}", writer.as_raw_fd()))?;
    need(id(&file)? == id(&writer)?)?;
    readonly(&file)?;
    need(read(&file, raw.len())? == raw)?;
    drop(writer);
    Ok(file)
}
fn stdin() -> io::Result<File> {
    let mut fds = [-1; 2];
    if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let reader = owned(fds[0])?;
    let writer = owned(fds[1])?;
    drop(writer);
    Ok(reader)
}
fn socket_option(fd: RawFd, option: i32) -> io::Result<i32> {
    let mut value = 0i32;
    let mut size = std::mem::size_of_val(&value) as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            option,
            (&mut value as *mut i32).cast(),
            &mut size,
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    need(size as usize == std::mem::size_of::<i32>())?;
    Ok(value)
}
fn release() -> io::Result<[File; 2]> {
    let mut fds = [-1; 2];
    if unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
            0,
            fds.as_mut_ptr(),
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    let files = [owned(fds[0])?, owned(fds[1])?];
    let yes = 1i32;
    for file in &files {
        need(
            unsafe {
                libc::setsockopt(
                    file.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_PASSCRED,
                    (&yes as *const i32).cast(),
                    std::mem::size_of_val(&yes) as _,
                )
            } == 0,
        )?;
    }
    Ok(files)
}
fn peer(file: &File, b: &Bindings) -> io::Result<()> {
    let fd = file.as_raw_fd();
    need(
        socket_option(fd, libc::SO_DOMAIN)? == libc::AF_UNIX
            && socket_option(fd, libc::SO_TYPE)? == libc::SOCK_SEQPACKET
            && socket_option(fd, libc::SO_PASSCRED)? == 1,
    )?;
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut size = std::mem::size_of_val(&cred) as libc::socklen_t;
    need(
        unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut cred as *mut libc::ucred).cast(),
                &mut size,
            )
        } == 0
            && size as usize == std::mem::size_of::<libc::ucred>()
            && i64::from(cred.pid) == b.owner[0]
            && i64::from(cred.uid) == b.owner[4]
            && i64::from(cred.gid) == b.owner[5],
    )?;
    let mut byte = 0u8;
    let n = unsafe {
        libc::recv(
            fd,
            (&mut byte as *mut u8).cast(),
            1,
            libc::MSG_PEEK | libc::MSG_TRUNC | libc::MSG_DONTWAIT,
        )
    };
    need(n < 0 && io::Error::last_os_error().raw_os_error() == Some(libc::EAGAIN))
}
fn creator(b: &Bindings) -> io::Result<File> {
    owner(b)?;
    owned(unsafe { libc::syscall(libc::SYS_pidfd_open, b.owner[0] as i32, 0u32) } as RawFd)
}
fn check_creator(file: &File, b: &Bindings) -> io::Result<()> {
    let fd = file.as_raw_fd();
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
    let mut pids = text.lines().filter_map(|line| line.strip_prefix("Pid:"));
    need(
        pids.next().and_then(|pid| pid.trim().parse::<i64>().ok()) == Some(b.owner[0])
            && pids.next().is_none(),
    )?;
    let mut p = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    need(unsafe { libc::poll(&mut p, 1, 0) } == 0 && p.revents == 0)
}
fn hash(raw: &str) -> io::Result<[u8; 32]> {
    need(crate::is_hash(raw))?;
    let mut value = [0; 32];
    for (i, b) in value.iter_mut().enumerate() {
        *b = u8::from_str_radix(&raw[2 * i..2 * i + 2], 16).map_err(|_| refused())?;
    }
    Ok(value)
}
fn bootstrap(
    b: &Bindings,
    token: &[u8; 32],
    stdin: [u64; 2],
    config: [u64; 3],
    channel: [u64; 2],
) -> io::Result<[u8; BOOT_BYTES]> {
    let mut raw = [0; BOOT_BYTES];
    raw[..8].copy_from_slice(b"HCHL0741");
    raw[8..16].copy_from_slice(&b.deadline.to_le_bytes());
    for (at, value) in [
        (16, b.owner[0]),
        (20, b.owner[4]),
        (24, b.owner[5]),
        (28, i64::from(b.guardians[0].0)),
        (32, i64::from(b.guardians[1].0)),
    ] {
        let value = u32::try_from(value).map_err(|_| refused())?;
        raw[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    need(token.iter().any(|b| *b != 0))?;
    raw[40..72].copy_from_slice(token);
    raw[72..104].copy_from_slice(&hash(&b.worker.1)?);
    raw[104..136].copy_from_slice(&hash(&b.config.1)?);
    for (at, value) in [
        (136, b.worker.0.as_slice()),
        (160, config.as_slice()),
        (184, stdin.as_slice()),
        (200, b.streams[0].as_slice()),
        (216, b.streams[1].as_slice()),
        (232, channel.as_slice()),
    ] {
        for (i, word) in value.iter().enumerate() {
            raw[at + 8 * i..at + 8 * i + 8].copy_from_slice(&word.to_le_bytes());
        }
    }
    Ok(raw)
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    fd: RawFd,
    stat: [u64; 11],
    descriptor: i32,
    status: i32,
    position: Option<i64>,
}

// Values only, private to materialization. No AsRawFd, extraction, borrow or
// remapping API. Worker and Armed guardian FDs are deliberately unresolved.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct RoleBindings(pub(super) [(u8, RawFd); 8]);
const MATERIAL_ROLES: [(u8, usize); 8] = [
    (0, 14),
    (1, 6),
    (2, 7),
    (3, 16),
    (5, 17),
    (6, 19),
    (9, 1),
    (10, 18),
];
pub(super) fn bind_roles(b: &Bindings, inventory: &[Stamp]) -> io::Result<RoleBindings> {
    need(inventory.len() == 20)?;
    for (i, s) in inventory.iter().enumerate() {
        need(
            s.fd >= 0
                && s.descriptor == libc::FD_CLOEXEC
                && !inventory[..i].iter().any(|old| old.fd == s.fd),
        )?;
    }
    for (at, expected) in [(6, b.streams[0]), (7, b.streams[1])] {
        let s = &inventory[at];
        need(
            s.stat[..2] == expected
                && s.stat[4] as u32 & libc::S_IFMT == libc::S_IFIFO
                && s.status & (libc::O_ACCMODE | libc::O_PATH) == libc::O_WRONLY,
        )?;
    }
    need(b.streams[0] != b.streams[1])?;
    for (at, kind, access) in [
        (14, libc::S_IFIFO, libc::O_RDONLY),
        (16, libc::S_IFSOCK, libc::O_RDWR),
        (17, libc::S_IFREG, libc::O_RDONLY),
        (18, libc::S_IFREG, libc::O_RDONLY),
    ] {
        let s = &inventory[at];
        need(
            s.stat[4] as u32 & libc::S_IFMT == kind
                && s.status & (libc::O_ACCMODE | libc::O_PATH) == access,
        )?;
    }
    need(inventory[17].stat[6] == b.config.0[2] && inventory[18].stat[6] == BOOT_BYTES as u64)?;
    Ok(RoleBindings(
        MATERIAL_ROLES.map(|(role, at)| (role, inventory[at].fd)),
    ))
}
fn stamp(fd: RawFd) -> io::Result<Stamp> {
    let mut m: libc::stat = unsafe { std::mem::zeroed() };
    need(unsafe { libc::fstat(fd, &mut m) } == 0)?;
    let descriptor = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    let status = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    need(descriptor >= 0 && status >= 0)?;
    let position = if m.st_mode & libc::S_IFMT == libc::S_IFREG {
        let p = unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) };
        need(p >= 0)?;
        Some(p)
    } else {
        None
    };
    Ok(Stamp {
        fd,
        stat: [
            m.st_dev,
            m.st_ino,
            u64::from(m.st_uid),
            u64::from(m.st_gid),
            u64::from(m.st_mode),
            m.st_nlink,
            m.st_size as u64,
            m.st_mtime as u64,
            m.st_mtime_nsec as u64,
            m.st_ctime as u64,
            m.st_ctime_nsec as u64,
        ],
        descriptor,
        status,
        position,
    })
}
pub(super) struct Resources {
    stdin: File,
    release: [File; 2],
    config: File,
    bootstrap: File,
    creator: File,
    token: [u8; 32],
    retained: Vec<Stamp>,
}
impl Resources {
    pub(super) fn create(b: &Bindings, config: &[u8]) -> io::Result<Self> {
        owner(b)?;
        need(
            !config.is_empty()
                && config.len() <= crate::diagnostic_artifacts::MAX_CONFIG_BYTES as usize
                && config.len() as u64 == b.config.0[2]
                && crate::sha256_hex(config) == b.config.1,
        )?;
        let stdin = stdin()?;
        let release = release()?;
        let config = sealed(config)?;
        let creator = creator(b)?;
        let mut token = [0; 32];
        need(
            unsafe { libc::getrandom(token.as_mut_ptr().cast(), token.len(), libc::GRND_NONBLOCK) }
                == token.len() as isize,
        )?;
        let raw = bootstrap(
            b,
            &token,
            pair_id(&stdin)?,
            id(&config)?,
            pair_id(&release[1])?,
        )?;
        let bootstrap = sealed(&raw)?;
        let mut resource = Self {
            stdin,
            release,
            config,
            bootstrap,
            creator,
            token,
            retained: vec![],
        };
        resource.retained = resource.stamps()?;
        resource.validate(b)?;
        Ok(resource)
    }
    fn fds(&self) -> [RawFd; 6] {
        [
            self.stdin.as_raw_fd(),
            self.release[0].as_raw_fd(),
            self.release[1].as_raw_fd(),
            self.config.as_raw_fd(),
            self.bootstrap.as_raw_fd(),
            self.creator.as_raw_fd(),
        ]
    }
    fn stamps(&self) -> io::Result<Vec<Stamp>> {
        self.fds().into_iter().map(stamp).collect()
    }
    fn validate(&self, b: &Bindings) -> io::Result<()> {
        owner(b)?;
        let current = self.stamps()?;
        need(current.iter().all(|s| s.descriptor == libc::FD_CLOEXEC) && current == self.retained)?;
        for file in [&self.config, &self.bootstrap] {
            readonly(file)?;
        }
        let config = read(
            &self.config,
            crate::diagnostic_artifacts::MAX_CONFIG_BYTES as usize,
        )?;
        need(config.len() as u64 == b.config.0[2] && crate::sha256_hex(&config) == b.config.1)?;
        let raw = bootstrap(
            b,
            &self.token,
            pair_id(&self.stdin)?,
            id(&self.config)?,
            pair_id(&self.release[1])?,
        )?;
        need(read(&self.bootstrap, BOOT_BYTES)? == raw)?;
        peer(&self.release[0], b)?;
        peer(&self.release[1], b)?;
        check_creator(&self.creator, b)?;
        let fd = self.stdin.as_raw_fd();
        let s = stamp(fd)?;
        need(
            s.stat[4] as u32 & libc::S_IFMT == libc::S_IFIFO
                && s.status & (libc::O_ACCMODE | libc::O_PATH) == libc::O_RDONLY,
        )?;
        let mut available = 0i32;
        need(unsafe { libc::ioctl(fd, libc::FIONREAD, &mut available) } == 0 && available == 0)?;
        let mut p = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        need(unsafe { libc::poll(&mut p, 1, 0) } == 1 && p.revents == libc::POLLHUP)?;
        let input = pair_id(&self.stdin)?;
        need(input != b.streams[0] && input != b.streams[1] && b.streams[0] != b.streams[1])?;
        for file in [&self.stdin, &self.release[0], &self.release[1]] {
            let s = stamp(file.as_raw_fd())?;
            need(s.descriptor == libc::FD_CLOEXEC && s.status & libc::O_NONBLOCK != 0)?;
        }
        owner(b)
    }
    pub(super) fn inspect(&self, b: &Bindings, original: [RawFd; 14]) -> io::Result<Vec<Stamp>> {
        self.validate(b)?;
        let fds: Vec<_> = original.into_iter().chain(self.fds()).collect();
        need(
            fds.len() == 20
                && fds
                    .iter()
                    .enumerate()
                    .all(|(i, fd)| *fd >= 0 && !fds[..i].contains(fd)),
        )?;
        let descriptors = fds.into_iter().map(stamp).collect::<io::Result<_>>()?;
        owner(b)?;
        Ok(descriptors)
    }
}

#[cfg(test)]
#[path = "diagnostic_held_unit_material_kernel_tests.rs"]
mod tests;
