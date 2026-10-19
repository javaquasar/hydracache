//! Private retained cgroup2 mechanics, not original trust or an Armed permit.
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt};

fn refused() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "pinned cgroup root refused",
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
fn stamp(file: &File) -> io::Result<Stamp> {
    let mut fs: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(file.as_raw_fd(), &mut fs) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if fs.f_type != 0x63677270 {
        return Err(refused());
    }
    let m = file.metadata()?;
    if m.uid() != 0 || m.gid() != 0 || m.mode() & 0o022 != 0 {
        return Err(refused());
    }
    Ok(Stamp {
        device: m.dev(),
        inode: m.ino(),
        uid: m.uid(),
        gid: m.gid(),
        mode: m.mode(),
    })
}
fn child(parent: BorrowedFd<'_>, name: &CStr, directory: bool, write: bool) -> io::Result<File> {
    let raw = name.to_bytes();
    if raw.is_empty() || raw.len() > 128 || raw == b"." || raw == b".." || raw.contains(&b'/') {
        return Err(refused());
    }
    let flags = libc::O_CLOEXEC
        | libc::O_NOFOLLOW
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 }
        | if write {
            libc::O_WRONLY
        } else {
            libc::O_RDONLY
        };
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn bounded(file: &File) -> io::Result<Vec<u8>> {
    let mut bytes = [0u8; 4097];
    let mut used = 0;
    while used < bytes.len() {
        match file.read_at(&mut bytes[used..], used as u64) {
            Ok(0) => return Ok(bytes[..used].to_vec()),
            Ok(n) => used += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Err(refused())
}
#[derive(Debug, PartialEq, Eq)]
struct Observation {
    populated: bool,
    frozen: bool,
}
fn events(bytes: &[u8]) -> io::Result<Observation> {
    let mut populated = None;
    let mut frozen = None;
    if !bytes.ends_with(b"\n") {
        return Err(refused());
    }
    for line in bytes[..bytes.len() - 1].split(|b| *b == b'\n') {
        let (slot, value) = match line {
            b"populated 0" => (&mut populated, false),
            b"populated 1" => (&mut populated, true),
            b"frozen 0" => (&mut frozen, false),
            b"frozen 1" => (&mut frozen, true),
            _ => return Err(refused()),
        };
        if slot.replace(value).is_some() {
            return Err(refused());
        }
    }
    Ok(Observation {
        populated: populated.ok_or_else(refused)?,
        frozen: frozen.ok_or_else(refused)?,
    })
}
struct Document {
    file: File,
    identity: Stamp,
    name: &'static CStr,
    write: bool,
}
impl Document {
    fn open(root: &File, name: &'static CStr, write: bool) -> io::Result<Self> {
        let file = child(root.as_fd(), name, false, write)?;
        let identity = stamp(&file)?;
        if !file.metadata()?.is_file() || identity.device != root.metadata()?.dev() {
            return Err(refused());
        }
        Ok(Self {
            file,
            identity,
            name,
            write,
        })
    }
    fn revalidate(&self, root: &File) -> io::Result<()> {
        let named = child(root.as_fd(), self.name, false, self.write)?;
        if stamp(&named)? != self.identity || stamp(&self.file)? != self.identity {
            return Err(refused());
        }
        Ok(())
    }
}
/// No constructor from signed metadata; this is a private kernel handle only.
struct PinnedRoot {
    parent: File,
    parent_identity: Stamp,
    name: CString,
    directory: File,
    identity: Stamp,
    kind: Document,
    events: Document,
    procs: Document,
    kill: Document,
    submitted: bool,
    refused: bool,
}
impl PinnedRoot {
    fn pin_empty(parent: BorrowedFd<'_>, name: &CStr) -> io::Result<Self> {
        let parent = File::from(parent.try_clone_to_owned()?);
        let parent_identity = stamp(&parent)?;
        if !parent.metadata()?.is_dir() {
            return Err(refused());
        }
        let directory = child(parent.as_fd(), name, true, false)?;
        let identity = stamp(&directory)?;
        if identity.device != parent_identity.device || identity.inode == parent_identity.inode {
            return Err(refused());
        }
        let mut root = Self {
            parent,
            parent_identity,
            name: name.into(),
            kind: Document::open(&directory, c"cgroup.type", false)?,
            events: Document::open(&directory, c"cgroup.events", false)?,
            procs: Document::open(&directory, c"cgroup.procs", false)?,
            kill: Document::open(&directory, c"cgroup.kill", true)?,
            directory,
            identity,
            submitted: false,
            refused: false,
        };
        let observation = root.observe()?;
        if observation.populated || observation.frozen || !bounded(&root.procs.file)?.is_empty() {
            return Err(refused());
        }
        Ok(root)
    }
    fn inspect(&self) -> io::Result<Observation> {
        if self.refused {
            return Err(refused());
        }
        let named = child(self.parent.as_fd(), &self.name, true, false)?;
        if stamp(&self.parent)? != self.parent_identity
            || stamp(&named)? != self.identity
            || stamp(&self.directory)? != self.identity
        {
            return Err(refused());
        }
        for document in [&self.kind, &self.events, &self.procs, &self.kill] {
            document.revalidate(&self.directory)?;
        }
        if bounded(&self.kind.file)? != b"domain\n" {
            return Err(refused());
        }
        let observation = events(&bounded(&self.events.file)?)?;
        // Recheck original objects, never replace them with the just-opened names.
        let closing = child(self.parent.as_fd(), &self.name, true, false)?;
        if stamp(&self.parent)? != self.parent_identity
            || stamp(&closing)? != self.identity
            || stamp(&self.directory)? != self.identity
        {
            return Err(refused());
        }
        for document in [&self.kind, &self.events, &self.procs, &self.kill] {
            document.revalidate(&self.directory)?;
        }
        Ok(observation)
    }
    fn observe(&mut self) -> io::Result<Observation> {
        let result = self.inspect();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    /// Submission only. This cannot return a completion, EOF or unlock receipt.
    /// Repeating revocation is not replaying a launch/enrollment operation.
    fn submit_kill(&mut self) -> io::Result<()> {
        self.observe()?;
        self.submitted = true;
        #[cfg(test)]
        tests::checkpoint(1);
        let result = self.kill.file.write_at(b"1", 0);
        #[cfg(test)]
        tests::checkpoint(2);
        match result {
            Ok(1) => Ok(()),
            Ok(_) => {
                self.refused = true;
                Err(refused())
            }
            Err(error) => {
                self.refused = true;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
#[path = "diagnostic_cgroup_root_tests.rs"]
mod tests;

#[path = "diagnostic_cgroup_root_transport.rs"]
mod transport;
