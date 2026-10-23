//! Read-only managed-PID consistency. Only the consuming owner selects expectations.
use super::*;
use std::ffi::CStr;

pub(crate) struct Expected {
    pub(crate) boot: String,
    pub(crate) root: String,
    pub(crate) group: String,
    pub(crate) root_id: [u64; 2],
    pub(crate) image: [u64; 3],
    pub(crate) streams: [[u64; 2]; 2],
    pub(crate) credentials: AssertedWorkerCredentials,
}
fn need(ok: bool) -> Result<(), ProcessError> {
    if ok {
        Ok(())
    } else {
        Err(ProcessError::Drift)
    }
}
fn follow(dir: &File, name: &CStr, directory: bool) -> Result<File, ProcessError> {
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_CLOEXEC
                | if directory {
                    libc::O_RDONLY | libc::O_DIRECTORY
                } else {
                    libc::O_PATH
                },
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn ns(dir: &File, name: &CStr, kind: i32) -> Result<File, ProcessError> {
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    need(
        unsafe { libc::fstatfs(fd, &mut fs) } == 0
            && fs.f_type == libc::NSFS_MAGIC as libc::c_long
            && unsafe { libc::ioctl(fd, libc::NS_GET_NSTYPE) } == kind,
    )?;
    Ok(file)
}
const NS: [(&CStr, i32); 4] = [
    (c"user", libc::CLONE_NEWUSER),
    (c"pid", libc::CLONE_NEWPID),
    (c"time", libc::CLONE_NEWTIME),
    (c"mnt", libc::CLONE_NEWNS),
];
fn namespaces(process: &ProcessRead) -> Result<[File; 8], ProcessError> {
    let worker = child(&process.probe.files.directory, "ns", true)?;
    let reader = follow(&process.probe.files.root, c"thread-self", true)?;
    require_procfs(&reader)?;
    let reader = child(&reader, "ns", true)?;
    let mut result = Vec::with_capacity(8);
    for (name, kind) in NS {
        let a = ns(&worker, name, kind)?;
        let b = ns(&reader, name, kind)?;
        need(
            (FileId::of(&a.metadata()?) == FileId::of(&b.metadata()?))
                == (kind != libc::CLONE_NEWNS),
        )?;
        result.extend([a, b]);
    }
    result.try_into().map_err(|_| ProcessError::Invalid)
}
fn entries(raw: &[u8], names: &mut Vec<(u8, String)>) -> Result<(), ProcessError> {
    let mut at = 0;
    while at < raw.len() {
        need(raw.len() - at >= 20)?;
        let len = u16::from_ne_bytes([raw[at + 16], raw[at + 17]]) as usize;
        need(len >= 20 && len <= raw.len() - at)?;
        let bytes = &raw[at + 19..at + len];
        let n = bytes
            .iter()
            .position(|b| *b == 0)
            .ok_or(ProcessError::Invalid)?;
        let name = std::str::from_utf8(&bytes[..n]).map_err(|_| ProcessError::Invalid)?;
        need(!name.is_empty() && name.len() <= 128 && !name.contains('/'))?;
        if name != "." && name != ".." {
            need(names.len() < 128 && !names.iter().any(|(_, prior)| prior == name))?;
            names.push((raw[at + 18], name.into()));
        }
        at += len;
    }
    Ok(())
}
fn listing(dir: &File) -> Result<Vec<(u8, String)>, ProcessError> {
    let file = follow(dir, c".", true)?;
    let mut names = vec![];
    let mut raw = [0; 8192];
    for _ in 0..129 {
        let n = unsafe {
            libc::syscall(
                libc::SYS_getdents64,
                file.as_raw_fd(),
                raw.as_mut_ptr(),
                raw.len(),
            )
        };
        if n < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if n == 0 {
            return Ok(names);
        }
        need(n as usize <= raw.len())?;
        entries(&raw[..n as usize], &mut names)?;
    }
    Err(ProcessError::Budget)
}
fn fdinfo(raw: &[u8]) -> Result<(u64, u64), ProcessError> {
    let text = std::str::from_utf8(raw).map_err(|_| ProcessError::Invalid)?;
    need(text.ends_with('\n'))?;
    let field = |prefix: &str, radix| -> Result<u64, ProcessError> {
        let mut lines = text.lines().filter_map(|l| l.strip_prefix(prefix));
        let v = lines.next().ok_or(ProcessError::Invalid)?.trim();
        need(
            !v.is_empty()
                && v.len() <= 24
                && lines.next().is_none()
                && v.bytes().all(|b| b.is_ascii_digit()),
        )?;
        u64::from_str_radix(v, radix).map_err(|_| ProcessError::Invalid)
    };
    let flags = field("flags:", 8)?;
    need(
        flags & (libc::O_ACCMODE | libc::O_PATH | libc::O_APPEND | libc::O_ASYNC) as u64
            == libc::O_WRONLY as u64,
    )?;
    Ok((flags, field("ino:", 10)?))
}
fn object(file: &File) -> Result<[u64; 11], ProcessError> {
    let m = file.metadata()?;
    Ok([
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
    ])
}
fn cgroup(file: &File) -> Result<[u64; 2], ProcessError> {
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    let m = file.metadata()?;
    need(
        unsafe { libc::fstatfs(file.as_raw_fd(), &mut fs) } == 0
            && fs.f_type == libc::CGROUP2_SUPER_MAGIC as libc::c_long
            && m.is_dir()
            && m.uid() == 0
            && m.gid() == 0
            && m.mode() & 0o022 == 0,
    )?;
    Ok([m.dev(), m.ino()])
}
fn read(dir: &File, name: &str) -> Result<Vec<u8>, ProcessError> {
    bounded(&child(dir, name, false)?)
}
fn population(raw: &[u8], populated: bool) -> Result<(), ProcessError> {
    need(
        raw == if populated {
            b"populated 1\nfrozen 0\n".as_slice()
        } else {
            b"populated 0\nfrozen 0\n".as_slice()
        } || raw
            == if populated {
                b"frozen 0\npopulated 1\n".as_slice()
            } else {
                b"frozen 0\npopulated 0\n".as_slice()
            },
    )
}

pub(crate) struct SandboxRead {
    process: ProcessRead,
    expected: Expected,
    namespaces: [File; 8],
    targets: [File; 3],
    image: [u64; 11],
    stream_flags: [u64; 2],
    bounding: u64,
    root: File,
    node: File,
    node_id: [u64; 2],
    refused: bool,
}
impl SandboxRead {
    pub(crate) fn pin(pid: u32, expected: Expected) -> Result<Self, ProcessError> {
        let before = ProcFiles::open(pid)?;
        need(read_boot(&before.root)? == expected.boot)?;
        let first = before.inspect()?;
        need(first.cgroup == expected.group)?;
        let process = pin(ExpectedGeneration {
            boot_id: expected.boot.clone(),
            pid,
            start_ticks: first.stat.start_ticks,
            cgroup_path: expected.group.clone(),
        })?;
        need(before.inspect()? == first)?;
        let namespaces = namespaces(&process)?;
        let targets = Self::targets(&process)?;
        let image = object(&targets[0])?;
        need(
            [image[0], image[1], image[6]] == expected.image
                && image[2..4] == [0, 0]
                && image[5] == 1
                && matches!(image[4] & 0o7777, 0o555 | 0o755),
        )?;
        let fdinfo_dir = child(&process.probe.files.directory, "fdinfo", true)?;
        let stream_flags = [
            fdinfo(&read(&fdinfo_dir, "1")?)?.0,
            fdinfo(&read(&fdinfo_dir, "2")?)?.0,
        ];
        let bounding = credentials::asserted_task_status(
            &read(&process.probe.files.directory, "status")?,
            pid,
            pid,
            &expected.credentials,
        )
        .map_err(|_| ProcessError::Drift)?;
        need(
            expected.root.starts_with("/system.slice/system-hc074")
                && expected
                    .group
                    .strip_prefix(&format!("{}/", expected.root))
                    .is_some_and(safe_component),
        )?;
        let root = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(format!("/sys/fs/cgroup{}", expected.root))?;
        need(cgroup(&root)? == expected.root_id)?;
        let name = expected
            .group
            .strip_prefix(&format!("{}/", expected.root))
            .ok_or(ProcessError::Invalid)?;
        let node = child(&root, name, true)?;
        let node_id = cgroup(&node)?;
        let mut held = Self {
            process,
            expected,
            namespaces,
            targets,
            image,
            stream_flags,
            bounding,
            root,
            node,
            node_id,
            refused: false,
        };
        held.revalidate()?;
        Ok(held)
    }
    fn targets(process: &ProcessRead) -> Result<[File; 3], ProcessError> {
        let dir = &process.probe.files.directory;
        let fds = child(dir, "fd", true)?;
        Ok([
            follow(dir, c"exe", false)?,
            follow(&fds, c"1", false)?,
            follow(&fds, c"2", false)?,
        ])
    }
    fn inspect(&self) -> Result<(), ProcessError> {
        self.process.revalidate()?;
        let fresh = namespaces(&self.process)?;
        for (old, new) in self.namespaces.iter().zip(&fresh) {
            need(FileId::of(&old.metadata()?) == FileId::of(&new.metadata()?))?;
        }
        let dir = &self.process.probe.files.directory;
        let tasks = listing(&child(dir, "task", true)?)?;
        let pid = self.process.observation.expected.pid;
        need(tasks == [(libc::DT_DIR, pid.to_string())])?;
        need(
            credentials::asserted_task_status(
                &read(dir, "status")?,
                pid,
                pid,
                &self.expected.credentials,
            )
            .map_err(|_| ProcessError::Drift)?
                == self.bounding,
        )?;
        let fresh = Self::targets(&self.process)?;
        need(object(&self.targets[0])? == self.image && object(&fresh[0])? == self.image)?;
        let fdinfo_dir = child(dir, "fdinfo", true)?;
        for i in 0..2 {
            for file in [&self.targets[i + 1], &fresh[i + 1]] {
                let m = file.metadata()?;
                need(
                    m.mode() & libc::S_IFMT == libc::S_IFIFO
                        && [m.dev(), m.ino()] == self.expected.streams[i],
                )?;
            }
            need(
                fdinfo(&read(&fdinfo_dir, if i == 0 { "1" } else { "2" })?)?
                    == (self.stream_flags[i], self.expected.streams[i][1]),
            )?;
        }
        let named = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY)
            .open(format!("/sys/fs/cgroup{}", self.expected.root))?;
        need(
            cgroup(&named)? == self.expected.root_id
                && cgroup(&self.root)? == self.expected.root_id,
        )?;
        let name = self
            .expected
            .group
            .strip_prefix(&format!("{}/", self.expected.root))
            .ok_or(ProcessError::Invalid)?;
        need(
            cgroup(&child(&self.root, name, true)?)? == self.node_id
                && cgroup(&self.node)? == self.node_id,
        )?;
        for (kind, value) in listing(&self.root)? {
            need(kind == libc::DT_REG || (kind == libc::DT_DIR && value == name))?;
        }
        need(
            listing(&self.node)?
                .iter()
                .all(|(kind, _)| *kind == libc::DT_REG),
        )?;
        need(
            read(&self.root, "cgroup.procs")?.is_empty()
                && read(&self.root, "cgroup.type")? == b"domain\n"
                && read(&self.node, "cgroup.type")? == b"domain\n"
                && read(&self.node, "cgroup.procs")? == format!("{pid}\n").as_bytes(),
        )?;
        population(&read(&self.root, "cgroup.events")?, true)?;
        population(&read(&self.node, "cgroup.events")?, true)?;
        self.process.revalidate()
    }
    pub(crate) fn revalidate(&mut self) -> Result<(), ProcessError> {
        if self.refused {
            return Err(ProcessError::Drift);
        }
        let result = self.inspect();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bootstrap_sandbox_fdinfo_and_population_refuse_unknown_or_writable_modes() {
        assert_eq!(fdinfo(b"flags:\t01\nino:\t74\n").unwrap(), (1, 74));
        for raw in [
            b"flags:\t02\nino:\t74\n".as_slice(),
            b"flags:\t01\nino:\t74\nino:\t74\n",
            b"flags:\t01\nino:\t-1\n",
            b"flags:\t01\nino:\t74",
        ] {
            assert!(fdinfo(raw).is_err());
        }
        population(b"populated 1\nfrozen 0\n", true).unwrap();
        for raw in [
            b"populated 0\nfrozen 0\n".as_slice(),
            b"populated 1\nfrozen 1\n",
            b"populated 1\nfrozen 0\nx 1\n",
        ] {
            assert!(population(raw, true).is_err());
        }
    }
    #[test]
    fn bootstrap_sandbox_directory_parser_is_bounded_and_refuses_duplicates() {
        let row = |name: &[u8]| {
            let mut r = vec![0; 20 + name.len()];
            let n = r.len() as u16;
            r[16..18].copy_from_slice(&n.to_ne_bytes());
            r[18] = libc::DT_DIR;
            r[19..19 + name.len()].copy_from_slice(name);
            r
        };
        let raw = row(b"74");
        let mut names = vec![];
        entries(&raw, &mut names).unwrap();
        assert_eq!(names, [(libc::DT_DIR, "74".into())]);
        assert!(entries(&raw, &mut names).is_err());
        assert!(entries(&[0; 19], &mut vec![]).is_err());
        assert!(entries(&row(b"../x"), &mut vec![]).is_err());
        assert!(entries(&row(&[b'x'; 129]), &mut vec![]).is_err());
    }
    #[test]
    fn bootstrap_sandbox_owned_helper_cannot_substitute_a_foreign_cgroup() {
        struct Owned(std::process::Child);
        impl Drop for Owned {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let child = Owned(
            std::process::Command::new("/bin/sleep")
                .arg("5")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let pid = child.0.id();
        let files = ProcFiles::open(pid).unwrap();
        let facts = files.inspect().unwrap();
        let generation = pin(ExpectedGeneration {
            boot_id: read_boot(&files.root).unwrap(),
            pid,
            start_ticks: facts.stat.start_ticks,
            cgroup_path: facts.cgroup.clone(),
        })
        .unwrap();
        generation.revalidate().unwrap();
        let targets = SandboxRead::targets(&generation).unwrap();
        assert!(targets[0].metadata().unwrap().is_file());
        for target in &targets[1..] {
            assert_eq!(
                target.metadata().unwrap().mode() & libc::S_IFMT,
                libc::S_IFIFO
            );
        }
        assert!(
            namespaces(&generation).is_err(),
            "ordinary same-host helper is not a sandbox namespace"
        );
        let expected = Expected {
            boot: read_boot(&ProcFiles::open(pid).unwrap().root).unwrap(),
            root: "/system.slice/system-hc074a.slice".into(),
            group: "/foreign".into(),
            root_id: [1, 2],
            image: [1, 2, 3],
            streams: [[1, 2], [1, 3]],
            credentials: AssertedWorkerCredentials::new(1000, 1000, vec![]).unwrap(),
        };
        let refused = SandboxRead::pin(pid, expected).is_err();
        assert!(refused);
    }
}
