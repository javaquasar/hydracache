//! Windows-specific extensions to [`fs`](super).
//!
//! The extension trait adds to [`OpenOptions`](super::OpenOptions) what the trait of the same name
//! in [`std::os::windows::fs`] adds to the one of std. Import the trait, then call its methods on
//! the builder. [`Metadata`](super::Metadata) is the type of std, so the trait of std that extends
//! it is re-exported here.

use std::{io, path::Path};

use super::{inline, sealed::Sealed};

#[doc(no_inline)]
pub use std::os::windows::fs::MetadataExt;

/// Makes `dst` a symbolic link to the directory `src`.
///
/// Runs [`std::os::windows::fs::symlink_dir`] on the thread that polls the future. To link to a
/// file, use [`symlink_file`].
pub async fn symlink_dir<P, Q>(src: P, dst: Q) -> io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    inline(|| std::os::windows::fs::symlink_dir(src, dst)).await
}

/// Makes `dst` a symbolic link to the file `src`.
///
/// Runs [`std::os::windows::fs::symlink_file`] on the thread that polls the future. To link to a
/// directory, use [`symlink_dir`].
pub async fn symlink_file<P, Q>(src: P, dst: Q) -> io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    inline(|| std::os::windows::fs::symlink_file(src, dst)).await
}

/// Windows-specific extensions to [`OpenOptions`](super::OpenOptions).
///
/// Only that type implements it. The trait is sealed. Each method sets an argument of the
/// `CreateFile` call that opens the file.
pub trait OpenOptionsExt: Sealed {
    /// Sets the `dwDesiredAccess` argument to `access`, in place of the value that
    /// [`read`](super::OpenOptions::read), [`write`](super::OpenOptions::write) and
    /// [`append`](super::OpenOptions::append) work out between them.
    ///
    /// This gives finer control of the access than those options. For example, an access mode of
    /// `0` opens a file only to look at its metadata.
    fn access_mode(&mut self, access: u32) -> &mut Self;

    /// Sets the `dwShareMode` argument to `share`.
    ///
    /// The default is `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`, which lets other
    /// processes read, write, and delete or rename the file while it is open. Leaving out one of
    /// the bits refuses other processes that operation until the handle is closed.
    fn share_mode(&mut self, share: u32) -> &mut Self;

    /// Sets the flags of the `dwFlagsAndAttributes` argument that are neither attributes nor
    /// security quality-of-service flags, such as `FILE_FLAG_DELETE_ON_CLOSE`.
    ///
    /// The flags can set bits but cannot clear bits that the options set. Each call replaces the
    /// flags of an earlier call.
    fn custom_flags(&mut self, flags: u32) -> &mut Self;

    /// Sets the attributes in the `dwFlagsAndAttributes` argument, such as `FILE_ATTRIBUTE_HIDDEN`.
    ///
    /// They are the attributes of a file that the open creates. For an existing file, they have an
    /// effect only if the open truncates it, and are then added to its attributes.
    fn attributes(&mut self, attributes: u32) -> &mut Self;

    /// Sets the security quality-of-service flags in `dwFlagsAndAttributes`, such as
    /// `SECURITY_IDENTIFICATION`, along with `SECURITY_SQOS_PRESENT`.
    ///
    /// They control how far the server of a named pipe may act on behalf of the client that opens
    /// it. They are not set by default. Set them when the path to open comes from an untrusted
    /// source, as it may lead to a named pipe.
    fn security_qos_flags(&mut self, flags: u32) -> &mut Self;
}
