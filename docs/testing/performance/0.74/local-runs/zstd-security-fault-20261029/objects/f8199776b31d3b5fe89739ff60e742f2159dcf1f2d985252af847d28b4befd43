//! Read-only fixed path identities, not manager ownership or exclusive population.
use super::*;
use std::os::unix::fs::OpenOptionsExt;

const COMPONENTS: [&CStr; 3] = [c"sys", c"fs", c"cgroup"];

fn directory_stamp(file: &File) -> io::Result<Stamp> {
    let m = file.metadata()?;
    if !m.is_dir() || m.uid() != 0 || m.gid() != 0 || m.mode() & 0o022 != 0 {
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
fn anchors() -> io::Result<[File; 4]> {
    let slash = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    directory_stamp(&slash)?;
    let sys = child(slash.as_fd(), COMPONENTS[0], true, false)?;
    directory_stamp(&sys)?;
    let fs = child(sys.as_fd(), COMPONENTS[1], true, false)?;
    directory_stamp(&fs)?;
    let cgroup = child(fs.as_fd(), COMPONENTS[2], true, false)?;
    directory_stamp(&cgroup)?;
    stamp(&cgroup)?;
    Ok([slash, sys, fs, cgroup])
}
fn identities(files: &[File; 4]) -> io::Result<[Stamp; 4]> {
    Ok([
        directory_stamp(&files[0])?,
        directory_stamp(&files[1])?,
        directory_stamp(&files[2])?,
        stamp(&files[3])?,
    ])
}
fn name(attempt: [u8; 32]) -> CString {
    let hex: String = attempt.iter().map(|b| format!("{b:02x}")).collect();
    CString::new(format!("system-hc074{hex}.slice")).expect("fixed ASCII digest")
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Facts {
    anchors: [Stamp; 4],
    system: Stamp,
    named_root: Stamp,
    retained_parent: Stamp,
    retained_root: Stamp,
    name_matches: bool,
}
fn binding(facts: &Facts, original: Stamp) -> io::Result<()> {
    if !facts.name_matches
        || facts.retained_parent != facts.system
        || facts.retained_root != original
        || facts.named_root != original
        || original.device != facts.system.device
        || original.device != facts.anchors[3].device
        || original.inode == facts.system.inode
        || facts.system.inode == facts.anchors[3].inode
    {
        return Err(refused());
    }
    Ok(())
}
fn unchanged(current: &Facts, retained: &Facts) -> io::Result<()> {
    if current != retained {
        return Err(refused());
    }
    Ok(())
}

pub(crate) struct FixedSlice {
    anchors: [File; 4],
    system: File,
    name: CString,
    facts: Facts,
}
impl FixedSlice {
    fn facts(&self, root: &PinnedRoot) -> io::Result<Facts> {
        let system = child(self.anchors[3].as_fd(), c"system.slice", true, false)?;
        let named = child(system.as_fd(), &self.name, true, false)?;
        let facts = Facts {
            anchors: identities(&self.anchors)?,
            system: stamp(&system)?,
            named_root: stamp(&named)?,
            retained_parent: stamp(&root.parent)?,
            retained_root: stamp(&root.directory)?,
            name_matches: root.name == self.name,
        };
        binding(&facts, root.identity)?;
        if stamp(&self.system)? != facts.system
            || root.parent_identity != facts.system
            || identities(&anchors()?)? != facts.anchors
        {
            return Err(refused());
        }
        Ok(facts)
    }
    pub(super) fn pin(root: &PinnedRoot, attempt: [u8; 32]) -> io::Result<Self> {
        let expected = name(attempt);
        // Reject an unrelated fixture before opening any other target.
        if root.name != expected || root.refused {
            return Err(refused());
        }
        let anchors = anchors()?;
        let system = child(anchors[3].as_fd(), c"system.slice", true, false)?;
        let named = child(system.as_fd(), &expected, true, false)?;
        let facts = Facts {
            anchors: identities(&anchors)?,
            system: stamp(&system)?,
            named_root: stamp(&named)?,
            retained_parent: stamp(&root.parent)?,
            retained_root: stamp(&root.directory)?,
            name_matches: true,
        };
        binding(&facts, root.identity)?;
        let result = Self {
            anchors,
            system,
            name: expected,
            facts,
        };
        result.revalidate(root, attempt)?;
        Ok(result)
    }
    pub(super) fn revalidate(&self, root: &PinnedRoot, attempt: [u8; 32]) -> io::Result<()> {
        if root.refused || self.name != name(attempt) {
            return Err(refused());
        }
        unchanged(&self.facts(root)?, &self.facts)
    }
}

#[cfg(test)]
#[path = "diagnostic_fixed_slice_tests.rs"]
mod tests;
