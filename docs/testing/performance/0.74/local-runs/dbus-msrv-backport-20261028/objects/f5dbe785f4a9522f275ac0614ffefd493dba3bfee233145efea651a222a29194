//! The directory types of [`fs`](super): [`ReadDir`], [`DirEntry`] and [`DirBuilder`].

use std::{
    ffi::OsString,
    fmt,
    future::Future,
    io,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, ready},
};

use futures_core::Stream;

#[cfg(any(unix, windows))]
use super::sealed::Sealed;
use super::{FileType, Metadata, inline};
use crate::coop;

/// A stream of the entries of a directory, created by [`read_dir()`](super::read_dir).
///
/// It yields an [`io::Result`] of a [`DirEntry`] for each entry, and ends after the last one. The
/// directory is read as the stream is polled, one entry for each poll, so an error can occur in the
/// middle, and an entry created while the stream is read may or may not appear. The entries are in
/// no particular order. `.` and `..` are not among them. See [`read_dir()`](super::read_dir) for an
/// example.
pub struct ReadDir(std::fs::ReadDir);

impl ReadDir {
    /// A stream of the entries of `dir`.
    pub(super) fn new(dir: std::fs::ReadDir) -> Self {
        Self(dir)
    }
}

impl Stream for ReadDir {
    type Item = io::Result<DirEntry>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        ready!(coop::poll_proceed(cx));
        let entry = self.0.next();
        coop::spend();

        Poll::Ready(entry.map(|entry| entry.map(|entry| DirEntry(Arc::new(entry)))))
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadDir").finish_non_exhaustive()
    }
}

/// An entry of a directory, yielded by the [`ReadDir`] stream.
///
/// The name and the path of the entry are already known and are read without waiting. Its
/// [metadata](DirEntry::metadata) and [type](DirEntry::file_type) can need disk access, so they are
/// `async` methods, which run on the thread that polls the future like the other operations of the
/// module. On unix, the `DirEntryExt` trait of the `unix` module adds the inode number.
///
/// Cloning an entry is cheap. The clones share the entry of std.
#[derive(Clone)]
pub struct DirEntry(Arc<std::fs::DirEntry>);

impl DirEntry {
    /// The full path of the entry: the path given to [`read_dir()`](super::read_dir), joined with
    /// the name of the entry.
    pub fn path(&self) -> PathBuf {
        self.0.path()
    }

    /// The name of the entry, without the path of the directory it is in.
    pub fn file_name(&self) -> OsString {
        self.0.file_name()
    }

    /// Reads the metadata of the entry itself, without following a symbolic link.
    ///
    /// Runs [`std::fs::DirEntry::metadata`] on the thread that polls the future. For a symbolic
    /// link, it describes the link and not its target. To read the metadata of the target, call
    /// [`metadata`](super::metadata) with the [`path`](DirEntry::path) of the entry.
    ///
    /// # Errors
    ///
    /// Fails if the entry was removed after the directory was read.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        inline(|| self.0.metadata()).await
    }

    /// Reads the type of the entry (file, directory or symbolic link), without following a link.
    ///
    /// Runs [`std::fs::DirEntry::file_type`] on the thread that polls the future. Most platforms
    /// return the type together with the entry, which makes this a short call. Not all do.
    pub async fn file_type(&self) -> io::Result<FileType> {
        inline(|| self.0.file_type()).await
    }
}

impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

#[cfg(any(unix, windows))]
impl Sealed for DirEntry {}

#[cfg(unix)]
impl super::unix::DirEntryExt for DirEntry {
    fn ino(&self) -> u64 {
        std::os::unix::fs::DirEntryExt::ino(&*self.0)
    }
}

/// A builder of directories, with options for how they are created.
///
/// Set the options on the builder, then call [`create`](DirBuilder::create) to create a directory
/// with them. A builder can create any number of directories. On unix, the `DirBuilderExt` trait of
/// the `unix` module adds the permission bits of new directories.
///
/// # Example
///
/// ```
/// use futures::executor::block_on;
/// use zruntime::fs::DirBuilder;
///
/// # let pid = std::process::id();
/// # let dir = std::env::temp_dir().join(format!("zruntime-fs-doc-dir-builder-{pid}"));
/// block_on(async {
///     let path = dir.join("one").join("two");
///     DirBuilder::new().recursive(true).create(&path).await?;
///
///     assert!(path.is_dir());
///
///     std::io::Result::Ok(())
/// })
/// .unwrap();
/// # std::fs::remove_dir_all(&dir).unwrap();
/// ```
#[derive(Debug, Default)]
pub struct DirBuilder {
    /// Whether the parents that are missing are made as well, and a directory that is there
    /// already is no error.
    recursive: bool,
    /// The permission bits of the directories made, where they are not the default.
    #[cfg(unix)]
    mode: Option<u32>,
}

impl DirBuilder {
    /// Creates a builder with the defaults of std: `recursive` is off.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether missing parent directories are created as well.
    ///
    /// If set, it is also not an error if the directory already exists, as with
    /// [`create_dir_all`](super::create_dir_all). The parents are created with the same options as
    /// the directory itself. This is off by default.
    pub fn recursive(&mut self, recursive: bool) -> &mut Self {
        self.recursive = recursive;
        self
    }

    /// Creates the directory at `path` with the options of the builder.
    ///
    /// Runs [`std::fs::DirBuilder::create`] on the thread that polls the future. The returned
    /// future does not borrow the builder, so the builder can be changed or dropped right away. The
    /// future does nothing until it is polled.
    pub fn create<P>(&self, path: P) -> impl Future<Output = io::Result<()>> + use<P>
    where
        P: AsRef<Path>,
    {
        let builder = self.to_std();
        let path = path.as_ref().to_owned();
        inline(move || builder.create(path))
    }

    /// The builder of std with the options of this one.
    fn to_std(&self) -> std::fs::DirBuilder {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(self.recursive);
        #[cfg(unix)]
        if let Some(mode) = self.mode {
            std::os::unix::fs::DirBuilderExt::mode(&mut builder, mode);
        }
        builder
    }
}

#[cfg(any(unix, windows))]
impl Sealed for DirBuilder {}

#[cfg(unix)]
impl super::unix::DirBuilderExt for DirBuilder {
    fn mode(&mut self, mode: u32) -> &mut Self {
        self.mode = Some(mode);
        self
    }
}
