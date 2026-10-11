//! Read-only fixed ancestry prerequisite, not an intent backend or host fence.
use super::open_at;
use std::fs::{File, Metadata, OpenOptions};
use std::marker::PhantomData;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use thiserror::Error;

pub const FIXED_INTENT_STORAGE_PATH: &str =
    "/var/lib/hydracache-performance/diagnostic-intents-074";
const COMPONENTS: [&str; 4] = [
    "var",
    "lib",
    "hydracache-performance",
    "diagnostic-intents-074",
];

#[path = "diagnostic_storage_preflight.rs"]
pub mod preflight;

#[derive(Clone, Copy)]
enum StorageLeaf {
    Intent,
    Authority,
}
impl StorageLeaf {
    fn name(self) -> &'static str {
        match self {
            Self::Intent => "diagnostic-intents-074",
            Self::Authority => "worker-authority-074",
        }
    }
}

#[derive(Debug, Error)]
pub enum FixedIntentStorageError {
    #[error("fixed intent storage owner, mode or original object refused")]
    Invalid,
    #[error("original fixed intent storage guard permanently refused")]
    Refused,
    #[error("fixed intent storage read failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("fixed intent component open refused: {0}")]
    Open(#[from] super::PreparationError),
}

#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
    mode: u32,
}
impl Stamp {
    fn of(m: &Metadata) -> Self {
        Self {
            device: m.dev(),
            inode: m.ino(),
            uid: m.uid(),
            gid: m.gid(),
            mode: m.mode(),
        }
    }
    fn safe(&self, uid: u32, gid: u32, level: usize) -> bool {
        self.mode & libc::S_IFMT == libc::S_IFDIR
            && self.uid == uid
            // Provisioned performance parent is root:hydracache-perf, not root:root.
            && (level == 3 || self.gid == gid)
            && if level == 4 { self.mode & 0o7777 == 0o700 }
               else { self.mode & 0o7022 == 0 }
    }
}
struct Component {
    file: File,
    stamp: Stamp,
}
fn open_root(path: &Path) -> Result<File, FixedIntentStorageError> {
    Ok(OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)?)
}

/// Retains original directories only. No publication, fence, authority or start.
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::FixedIntentStorageRead;
/// fn copy(read: &FixedIntentStorageRead) { let _ = (*read).clone(); }
/// ```
///
/// ```compile_fail
/// use hydracache_long_run_supervisor_074::diagnostic_output_preparation::fixed_intent_storage::FixedIntentStorageRead;
/// fn start(read: &mut FixedIntentStorageRead) { read.start_once(); }
/// ```
pub struct FixedIntentStorageRead {
    root_name: PathBuf,
    root: Component,
    components: Vec<Component>,
    refused: bool,
    _original_thread: PhantomData<Rc<()>>,
    kind: StorageLeaf,
}
impl FixedIntentStorageRead {
    pub fn is_refused(&self) -> bool {
        self.refused
    }
    pub fn revalidate(&mut self) -> Result<(), FixedIntentStorageError> {
        if self.refused {
            return Err(FixedIntentStorageError::Refused);
        }
        let result = self.inspect();
        if result.is_err() {
            self.refused = true;
        }
        result
    }
    fn inspect(&self) -> Result<(), FixedIntentStorageError> {
        let named_root = open_root(&self.root_name)?;
        if Stamp::of(&named_root.metadata()?) != self.root.stamp
            || Stamp::of(&self.root.file.metadata()?) != self.root.stamp
        {
            return Err(FixedIntentStorageError::Invalid);
        }
        let mut parent = &self.root.file;
        for (index, original) in self.components.iter().enumerate() {
            let name = if index == 3 {
                self.kind.name()
            } else {
                COMPONENTS[index]
            };
            let named = open_at(parent, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
            if Stamp::of(&named.metadata()?) != original.stamp
                || Stamp::of(&original.file.metadata()?) != original.stamp
            {
                return Err(FixedIntentStorageError::Invalid);
            }
            parent = &original.file;
        }
        Ok(())
    }
}

// Caller-selected origin/ownership is private and used only by local modeled tests.
fn walk(
    root_name: &Path,
    uid: u32,
    gid: u32,
) -> Result<FixedIntentStorageRead, FixedIntentStorageError> {
    walk_kind(root_name, uid, gid, StorageLeaf::Intent)
}

fn walk_kind(
    root_name: &Path,
    uid: u32,
    gid: u32,
    kind: StorageLeaf,
) -> Result<FixedIntentStorageRead, FixedIntentStorageError> {
    let file = open_root(root_name)?;
    let stamp = Stamp::of(&file.metadata()?);
    if !stamp.safe(uid, gid, 0) {
        return Err(FixedIntentStorageError::Invalid);
    }
    let root = Component { file, stamp };
    let mut components: Vec<Component> = Vec::with_capacity(COMPONENTS.len());
    for (index, component) in COMPONENTS.iter().enumerate() {
        let name = if index == 3 { kind.name() } else { component };
        let parent = components.last().map_or(&root.file, |c| &c.file);
        let file = open_at(parent, name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        let stamp = Stamp::of(&file.metadata()?);
        if !stamp.safe(uid, gid, index + 1) {
            return Err(FixedIntentStorageError::Invalid);
        }
        components.push(Component { file, stamp });
    }
    let mut read = FixedIntentStorageRead {
        root_name: root_name.into(),
        root,
        components,
        refused: false,
        _original_thread: PhantomData,
        kind,
    };
    read.revalidate()?;
    Ok(read)
}

/// Read-only authority directory ancestry, not an initialized authority registry.
pub struct FixedAuthorityStorageRead {
    read: FixedIntentStorageRead,
}
impl FixedAuthorityStorageRead {
    pub fn revalidate(&mut self) -> Result<(), FixedIntentStorageError> {
        self.read.revalidate()
    }
    pub fn is_refused(&self) -> bool {
        self.read.is_refused()
    }
}
pub fn open_fixed_authority_storage() -> Result<FixedAuthorityStorageRead, FixedIntentStorageError>
{
    if unsafe { libc::geteuid() } != 0 || unsafe { libc::getegid() } != 0 {
        return Err(FixedIntentStorageError::Invalid);
    }
    Ok(FixedAuthorityStorageRead {
        read: walk_kind(Path::new("/"), 0, 0, StorageLeaf::Authority)?,
    })
}

/// Open existing fixed root-owned storage only. Numeric root is not host enrollment.
pub fn open_fixed_intent_storage() -> Result<FixedIntentStorageRead, FixedIntentStorageError> {
    if unsafe { libc::geteuid() } != 0 || unsafe { libc::getegid() } != 0 {
        return Err(FixedIntentStorageError::Invalid);
    }
    walk(Path::new("/"), 0, 0)
}

#[cfg(test)]
#[path = "diagnostic_intent_fixed_storage_tests.rs"]
mod tests;
