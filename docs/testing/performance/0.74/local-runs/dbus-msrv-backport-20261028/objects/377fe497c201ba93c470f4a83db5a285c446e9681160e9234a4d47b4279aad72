//! Unix-specific extensions to [`fs`](super).
//!
//! The extension traits add to the builders and the entries of the parent module what the traits of
//! the same names in [`std::os::unix::fs`] add to those of std. Import the trait, then call its
//! methods on the builder or the entry. [`Metadata`](super::Metadata),
//! [`Permissions`](super::Permissions) and [`FileType`](super::FileType) are the types of std, so
//! the traits of std that extend them are re-exported here.

use std::{io, path::Path};

use super::{inline, sealed::Sealed};

#[doc(no_inline)]
pub use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};

/// Makes `dst` a symbolic link to `src`.
///
/// Runs [`std::os::unix::fs::symlink`] on the thread that polls the future. The link stores the
/// path `src` as given. A relative `src` is read relative to the directory of the link, not the
/// working directory of the process. `src` need not exist.
///
/// # Example
///
/// ```
/// use futures::executor::block_on;
/// use zruntime::fs::{self, unix};
///
/// # let pid = std::process::id();
/// # let dir = std::env::temp_dir().join(format!("zruntime-fs-doc-symlink-{pid}"));
/// # std::fs::create_dir_all(&dir).unwrap();
/// block_on(async {
///     let target = dir.join("target.txt");
///     let link = dir.join("link.txt");
///     fs::write(&target, "pointed at").await?;
///
///     unix::symlink(&target, &link).await?;
///
///     assert_eq!(fs::read_link(&link).await?, target);
///     assert_eq!(fs::read_to_string(&link).await?, "pointed at");
///
///     std::io::Result::Ok(())
/// })
/// .unwrap();
/// # std::fs::remove_dir_all(&dir).unwrap();
/// ```
pub async fn symlink<P, Q>(src: P, dst: Q) -> io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    inline(|| std::os::unix::fs::symlink(src, dst)).await
}

/// Unix-specific extensions to [`DirBuilder`](super::DirBuilder).
///
/// Only that type implements it. The trait is sealed.
pub trait DirBuilderExt: Sealed {
    /// Sets the permission bits that the directories created are given.
    ///
    /// The OS clears the bits that the umask of the process has set, so a directory usually ends up
    /// with fewer bits than these. The default is `0o777`. The bits apply to the parents created by
    /// a [recursive](super::DirBuilder::recursive) builder as well.
    fn mode(&mut self, mode: u32) -> &mut Self;
}

/// Unix-specific extensions to [`DirEntry`](super::DirEntry).
///
/// Only that type implements it. The trait is sealed.
pub trait DirEntryExt: Sealed {
    /// The inode number of the entry, as the directory stores it.
    ///
    /// This is the `d_ino` field of the entry that the OS returned. It is already known, so reading
    /// it needs no further disk access.
    fn ino(&self) -> u64;
}

/// Unix-specific extensions to [`OpenOptions`](super::OpenOptions).
///
/// Only that type implements it. The trait is sealed.
pub trait OpenOptionsExt: Sealed {
    /// Sets the permission bits that a file is given if the open creates it.
    ///
    /// The OS clears the bits that the umask of the process has set, so a file usually ends up with
    /// fewer bits than these. The default is `0o666`. The bits have no effect on a file that
    /// already exists.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::executor::block_on;
    /// use zruntime::fs::{self, OpenOptions, unix::{OpenOptionsExt, PermissionsExt}};
    ///
    /// # let pid = std::process::id();
    /// # let dir = std::env::temp_dir().join(format!("zruntime-fs-doc-open-mode-{pid}"));
    /// # std::fs::create_dir_all(&dir).unwrap();
    /// block_on(async {
    ///     let path = dir.join("secret.txt");
    ///     OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path).await?;
    ///
    ///     let permissions = fs::metadata(&path).await?.permissions();
    ///     assert_eq!(permissions.mode() & 0o777, 0o600);
    ///
    ///     std::io::Result::Ok(())
    /// })
    /// .unwrap();
    /// # std::fs::remove_dir_all(&dir).unwrap();
    /// ```
    fn mode(&mut self, mode: u32) -> &mut Self;

    /// Passes `flags` to the call that opens the file, along with the flags that the other options
    /// produce.
    ///
    /// The access-mode bits are cleared from `flags`, so they cannot conflict with
    /// [`read`](super::OpenOptions::read), [`write`](super::OpenOptions::write) and
    /// [`append`](super::OpenOptions::append). The flags can set bits but cannot clear bits that
    /// the options set. Each call replaces the flags of an earlier call.
    fn custom_flags(&mut self, flags: i32) -> &mut Self;
}
