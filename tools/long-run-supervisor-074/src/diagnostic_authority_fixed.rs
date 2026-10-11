//! Fixed root-owned journal backend. Neither enrollment nor a signed start guard.
use super::*;

pub const FIXED_AUTHORITY_PATH: &str = "/var/lib/hydracache-performance/worker-authority-074";
const ANCESTORS: [&str; 3] = ["var", "lib", "hydracache-performance"];
const FIXED_LEAF: &str = "worker-authority-074";

struct Ancestor {
    name: String,
    file: File,
    stamp: Stamp,
}
pub(super) struct Ancestry {
    root_stamp: Stamp,
    ancestors: Vec<Ancestor>,
}
impl Ancestry {
    pub(super) fn revalidate(&self, root: &File) -> Result<(), AuthorityError> {
        if !self.root_stamp.directory_matches(&root.metadata()?) {
            return Err(AuthorityError::Invalid);
        }
        let mut parent = root;
        for ancestor in &self.ancestors {
            let named = open_at(parent, &ancestor.name, true)?;
            if !ancestor.stamp.directory_matches(&named.metadata()?)
                || !ancestor.stamp.directory_matches(&ancestor.file.metadata()?)
            {
                return Err(AuthorityError::Invalid);
            }
            parent = &ancestor.file;
        }
        Ok(())
    }
}

// Also exercised on synthetic stamps without root filesystem mutation.
fn safe_directory(stamp: &Stamp, uid: u32, gid: u32, leaf: bool) -> bool {
    stamp.mode & libc::S_IFMT == libc::S_IFDIR
        && stamp.uid == uid
        && stamp.gid == gid
        && if leaf {
            stamp.mode & 0o7777 == 0o700
        } else {
            stamp.mode & 0o7022 == 0
        }
}

// Private engine walker; only the fixed constructor can select a production origin.
fn walk(root: File, uid: u32, gid: u32) -> Result<Directory, AuthorityError> {
    let root_stamp = Stamp::of(&root.metadata()?);
    if !safe_directory(&root_stamp, uid, gid, false) {
        return Err(AuthorityError::Invalid);
    }
    let mut ancestors: Vec<Ancestor> = Vec::with_capacity(ANCESTORS.len());
    for name in ANCESTORS {
        let parent = ancestors.last().map_or(&root, |ancestor| &ancestor.file);
        let file = open_at(parent, name, true)?;
        let stamp = Stamp::of(&file.metadata()?);
        if !safe_directory(&stamp, uid, gid, false) {
            return Err(AuthorityError::Invalid);
        }
        ancestors.push(Ancestor {
            name: name.into(),
            file,
            stamp,
        });
    }
    let last = ancestors.last().ok_or(AuthorityError::Invalid)?;
    let temporary = last.file.try_clone()?;
    let temporary_stamp = last.stamp.clone();
    let leaf = open_at(&temporary, FIXED_LEAF, true)?;
    let leaf_stamp = Stamp::of(&leaf.metadata()?);
    if !safe_directory(&leaf_stamp, uid, gid, true) {
        return Err(AuthorityError::Invalid);
    }
    FileExt::try_lock_exclusive(&leaf)?;
    let directory = Directory {
        root,
        temporary,
        temporary_stamp,
        leaf,
        leaf_stamp,
        leaf_name: FIXED_LEAF.into(),
        uid,
        gid,
        fixed_ancestry: Some(Ancestry {
            root_stamp,
            ancestors,
        }),
    };
    directory.revalidate()?;
    Ok(directory)
}
fn fixed_directory() -> Result<Directory, AuthorityError> {
    // Scalar credentials are only a preliminary ownership gate, not host enrollment.
    if unsafe { libc::geteuid() } != 0 || unsafe { libc::getegid() } != 0 {
        return Err(AuthorityError::Invalid);
    }
    let root = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open("/")?;
    walk(root, 0, 0)
}

/// Non-cloneable original-thread journal writer, not a production start capability.
pub struct FixedAuthorityRegistry {
    journal: FixtureAuthorityRegistry,
}
/// Explicit initialization of an already provisioned empty fixed leaf only.
pub fn initialize_fixed_registry(
    policy: &CheckedWorkerPolicy,
) -> Result<FixedAuthorityRegistry, AuthorityError> {
    Ok(FixedAuthorityRegistry {
        journal: initialize_directory(fixed_directory()?, policy)?,
    })
}
/// Reopen the complete journal under independently supplied current pins.
pub fn open_fixed_registry(
    trust: &WorkerPolicyTrust,
    host: &AssertedWorkerHost,
) -> Result<FixedAuthorityRegistry, AuthorityError> {
    Ok(FixedAuthorityRegistry {
        journal: open_directory(fixed_directory()?, trust, host)?,
    })
}
impl FixedAuthorityRegistry {
    pub fn is_failed(&self) -> bool {
        self.journal.is_failed()
    }
    /// Cached data only; does not revalidate objects or authorize a launch.
    pub fn view(&self) -> Result<AuthorityView, AuthorityError> {
        self.journal.view()
    }
    pub fn inspect(
        &mut self,
        trust: &WorkerPolicyTrust,
        host: &AssertedWorkerHost,
    ) -> Result<AuthorityView, AuthorityError> {
        self.journal.inspect(trust, host)
    }
    pub fn refuse(&mut self, reason: RefusalReason) -> Result<(), AuthorityError> {
        self.journal.refuse(reason)
    }
    /// Explicit operator epoch transition; no refresh of an original signed worker guard.
    pub fn advance(&mut self, policy: &CheckedWorkerPolicy) -> Result<(), AuthorityError> {
        self.journal.advance(policy)
    }
}

#[cfg(test)]
#[path = "diagnostic_authority_fixed_tests.rs"]
mod tests;
