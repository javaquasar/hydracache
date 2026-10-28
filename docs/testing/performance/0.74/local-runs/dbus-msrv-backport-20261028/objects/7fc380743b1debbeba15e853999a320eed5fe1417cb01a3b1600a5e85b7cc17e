//! Async access to the filesystem, in the shape of [`std::fs`].
//!
//! Each operation runs on the thread that polls its future, within that poll. Waiting for
//! readiness would not help with a regular file on a native system: epoll refuses to watch one
//! (`EPERM`), `poll(2)` reports it as always ready, kqueue as ready while its position is short of
//! its end, and Winsock's `select` watches only sockets. None of them says whether a read or a
//! write would wait for the disk. So there is nothing to wait for, and the operation is simply
//! made. The module needs no runtime and works under any executor.
//!
//! The functions have the names of the ones in std, from [`read`] to [`write()`]. They take paths
//! as `AsRef<Path>` and fail with the errors of the std function they run. [`OpenOptions`],
//! [`DirBuilder`], [`DirEntry`] and [`File`] are built and used like their std counterparts.
//! Platform-specific options come from extension traits in the `unix` and `windows` modules, each
//! present only on its own platform. They match the extension traits of std. Code written for
//! `smol::fs` usually needs only a changed import path.
//!
//! # Blocking
//!
//! Most operations are served from the cache of the OS and take microseconds. But one that misses
//! the cache, one on a slow or network mount, and a single read or write of a large buffer block
//! the thread while they run, and every task on that thread waits meanwhile.
#![cfg_attr(
    all(feature = "unblock", not(target_os = "wasi")),
    doc = "Work that must not do that can go through [`unblock()`](crate::unblock()), which runs \
           it on a pool of threads kept for it."
)]
//!
//! Each operation spends the budget of the poll it runs in, if that poll has one.
#![cfg_attr(
    feature = "runtime",
    doc = "On a runtime of this crate, a poll of a task may complete 128 I/O operations before the \
           next one yields, so a task in a loop of `fs` operations lets the other tasks run after \
           128 of them. [`Registration::poll_io`](crate::Registration::poll_io) explains the \
           budget in full."
)]
#![cfg_attr(
    not(feature = "runtime"),
    doc = "Only a runtime of this crate gives a poll a budget, and this build has no runtime."
)]
//! A thread that is inside no poll by a runtime of this crate has no budget, so an operation polled
//! there, by another executor or by plain code, always runs.
//!
//! # WASI
//!
//! The module works on WASI as it does elsewhere, with these differences:
//!
//! * A path reaches only the directories that the host lends the program, its "preopened"
//!   directories. With wasmtime, `wasmtime run --dir` lends one.
//! * WASI has no permissions, so [`set_permissions`] and [`File::set_permissions`] fail with
//!   [`Unsupported`](io::ErrorKind::Unsupported).
//! * There is no `unix` or `windows` module of extensions.
//!
//! # Dropping a future
//!
//! A future does nothing until it is polled, and the poll that runs its operation runs all of it.
//! A future that is dropped before that poll has done nothing, and one that has been polled to its
//! end is done. No work is left running that dropping a future would give up on.
//!
//! If the operation is a single call, it is either done or not done. If it is many calls, as in
//! [`remove_dir_all`], a crash or a full disk can leave it half done, as in std.
//!
//! # Files
//!
//! A [`File`] reads, writes and seeks through the `AsyncRead`, `AsyncWrite` and `AsyncSeek` traits
//! of `futures-io`, straight to the file: it reads nothing ahead and holds no write back. A write
//! completes once the OS has the bytes, and fails with the error the OS gives for it. The [`File`]
//! docs explain more.
//!
//! A `File` made from something that is not a regular file, such as a pipe, a socket or a
//! terminal, can find nothing to read or no room to write, and the read or the write then blocks
//! the thread until there is.
#![cfg_attr(
    feature = "runtime",
    doc = "On unix, [`AsyncIo`](crate::AsyncIo) waits for such a source without blocking the \
           thread, and on Windows it does for a socket."
)]
#![cfg_attr(
    all(feature = "unblock", not(target_os = "wasi")),
    doc = "[`Unblock`](crate::Unblock) runs its calls on a pool of threads, out of the way of the \
           tasks."
)]
//!
//! # Example
//!
//! The example works in a directory created for it. It drives the futures with `block_on` from the
//! `futures` crate, but the `block_on` of any executor works, as the module needs no runtime:
//!
//! ```
//! use futures::{AsyncReadExt, AsyncWriteExt, executor::block_on};
//! use zruntime::fs::{self, File};
//!
//! # let pid = std::process::id();
//! # let dir = std::env::temp_dir().join(format!("zruntime-fs-doc-module-{pid}"));
//! # std::fs::create_dir_all(&dir).unwrap();
//! block_on(async {
//!     let path = dir.join("greeting.txt");
//!
//!     let mut file = File::create(&path).await?;
//!     file.write_all(b"hello, world").await?;
//!     file.flush().await?;
//!
//!     assert_eq!(fs::read_to_string(&path).await?, "hello, world");
//!     assert_eq!(fs::metadata(&path).await?.len(), 12);
//!
//!     fs::rename(&path, dir.join("farewell.txt")).await?;
//!     assert!(fs::metadata(&path).await.is_err());
//!
//!     let mut text = String::new();
//!     File::open(dir.join("farewell.txt")).await?.read_to_string(&mut text).await?;
//!     assert_eq!(text, "hello, world");
//!
//!     std::io::Result::Ok(())
//! })
//! .unwrap();
//! # std::fs::remove_dir_all(&dir).unwrap();
//! ```
//!
//! # Differences from `async-fs`
//!
//! This module is modelled on [`async-fs`], the crate behind `smol::fs`. It differs in these ways:
//!
//! * `async-fs` runs every operation as blocking work on the pool of threads of `blocking`, so that
//!   it does not hold the thread that polls the task. This module runs each on the thread that
//!   polls the future, within that poll, and [`ReadDir`] yields one entry per poll. See
//!   [Blocking](self#blocking) for what that costs.
//! * [`File`] has no read-ahead and no write-behind. A read returns what the OS gave for it, and a
//!   write completes once the OS has the bytes, so the length that [`File::metadata`] reports
//!   includes every earlier write, and an error shows at the write that caused it. The position of
//!   the file is the one the OS keeps, as in std.
//! * The futures of [`DirBuilder::create`] and [`OpenOptions::open`] do nothing until polled, like
//!   those of the functions. In `async-fs`, the first starts its work at once.
//!
//! [`async-fs`]: https://crates.io/crates/async-fs

mod dir;
mod file;
mod options;
#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

use std::{
    future::poll_fn,
    io,
    path::{Path, PathBuf},
};

pub use dir::{DirBuilder, DirEntry, ReadDir};
pub use file::File;
pub use options::OpenOptions;
#[doc(no_inline)]
pub use std::fs::{FileType, Metadata, Permissions};

use crate::coop;

/// Resolves `path` to its canonical form: absolute, with every `.` and `..` resolved and every
/// symbolic link followed.
///
/// Runs [`std::fs::canonicalize`] on the thread that polls the future.
///
/// # Errors
///
/// Fails if `path`, or any directory on the way to it, does not exist.
pub async fn canonicalize<P>(path: P) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::canonicalize(path)).await
}

/// Copies the contents and the permissions of the file `src` to `dst`, and returns the number of
/// bytes copied.
///
/// Runs [`std::fs::copy`] on the thread that polls the future. An existing `dst` is overwritten. If
/// `dst` is the same file as `src`, it is likely to be truncated. To copy between two open
/// [`File`]s, use an async copy that reads and writes through them, such as `futures::io::copy`.
pub async fn copy<P, Q>(src: P, dst: Q) -> io::Result<u64>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    inline(|| std::fs::copy(src, dst)).await
}

/// Creates a new, empty directory at `path`.
///
/// Runs [`std::fs::create_dir`] on the thread that polls the future.
///
/// # Errors
///
/// Fails if the parent of `path` does not exist, or if `path` already exists. To create missing
/// parents and accept an existing directory, use [`create_dir_all`].
pub async fn create_dir<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::create_dir(path)).await
}

/// Creates a directory at `path`, along with any parents that do not exist.
///
/// Runs [`std::fs::create_dir_all`] on the thread that polls the future. It is not an error if the
/// directory already exists, or if another thread or process creates it in the meantime.
pub async fn create_dir_all<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::create_dir_all(path)).await
}

/// Makes `dst` another name for the file `src`.
///
/// Runs [`std::fs::hard_link`] on the thread that polls the future. Both names refer to the same
/// file. Most operating systems allow this only for two names on the same filesystem.
pub async fn hard_link<P, Q>(src: P, dst: Q) -> io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    inline(|| std::fs::hard_link(src, dst)).await
}

/// Reads the metadata of the file or directory at `path`, following symbolic links.
///
/// Runs [`std::fs::metadata`] on the thread that polls the future. To read the metadata of a link
/// itself, use [`symlink_metadata`].
pub async fn metadata<P>(path: P) -> io::Result<Metadata>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::metadata(path)).await
}

/// Reads the whole file at `path` into a vector of bytes.
///
/// Runs [`std::fs::read`] on the thread that polls the future. It sizes the vector by the length of
/// the file where it can, so it reads a file in a call or two. That makes it cheaper than opening
/// a [`File`] and reading that, which takes a call per chunk read. To read text, use
/// [`read_to_string`].
pub async fn read<P>(path: P) -> io::Result<Vec<u8>>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::read(path)).await
}

/// Opens the directory at `path` and returns a stream of its entries.
///
/// Runs [`std::fs::read_dir`] on the thread that polls the future. The entries come out of
/// [`ReadDir`] in no particular order.
///
/// # Errors
///
/// Fails if `path` is not a directory that can be read. Reading the [`ReadDir`] stream can fail
/// too.
///
/// # Example
///
/// ```
/// use futures::{StreamExt, executor::block_on};
/// use zruntime::fs;
///
/// # let pid = std::process::id();
/// # let dir = std::env::temp_dir().join(format!("zruntime-fs-doc-read-dir-{pid}"));
/// # std::fs::create_dir_all(&dir).unwrap();
/// block_on(async {
///     fs::write(dir.join("a.txt"), "first").await?;
///     fs::create_dir(dir.join("b")).await?;
///
///     let mut entries = fs::read_dir(&dir).await?;
///     let mut names = Vec::new();
///     while let Some(entry) = entries.next().await {
///         let entry = entry?;
///         names.push((entry.file_name(), entry.file_type().await?.is_dir()));
///     }
///     names.sort();
///
///     assert_eq!(names, [("a.txt".into(), false), ("b".into(), true)]);
///
///     std::io::Result::Ok(())
/// })
/// .unwrap();
/// # std::fs::remove_dir_all(&dir).unwrap();
/// ```
pub async fn read_dir<P>(path: P) -> io::Result<ReadDir>
where
    P: AsRef<Path>,
{
    let dir = inline(|| std::fs::read_dir(path)).await?;
    Ok(ReadDir::new(dir))
}

/// Reads the target of the symbolic link at `path`.
///
/// Runs [`std::fs::read_link`] on the thread that polls the future. The path it returns is the one
/// stored in the link. It can be relative to the directory of the link, and can lead nowhere.
pub async fn read_link<P>(path: P) -> io::Result<PathBuf>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::read_link(path)).await
}

/// Reads the whole file at `path` into a string.
///
/// Runs [`std::fs::read_to_string`] on the thread that polls the future. To read bytes, use
/// [`read`].
///
/// # Errors
///
/// Fails with [`InvalidData`](io::ErrorKind::InvalidData) if the file is not valid UTF-8.
pub async fn read_to_string<P>(path: P) -> io::Result<String>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::read_to_string(path)).await
}

/// Removes the directory at `path`, which must be empty.
///
/// Runs [`std::fs::remove_dir`] on the thread that polls the future. To remove a directory together
/// with its contents, use [`remove_dir_all`].
pub async fn remove_dir<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::remove_dir(path)).await
}

/// Removes the directory at `path` and everything in it.
///
/// Runs [`std::fs::remove_dir_all`] on the thread that polls the future. A large directory takes a
/// while to remove, and the thread is held meanwhile. If the removal fails part of the way, what
/// was not yet removed stays where it is.
pub async fn remove_dir_all<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::remove_dir_all(path)).await
}

/// Removes the file at `path`.
///
/// Runs [`std::fs::remove_file`] on the thread that polls the future. It removes a name. For a
/// symbolic link, that is the link and not its target.
pub async fn remove_file<P>(path: P) -> io::Result<()>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::remove_file(path)).await
}

/// Renames the file or directory at `src` to `dst`, replacing what is at `dst` if the platform
/// allows it.
///
/// Runs [`std::fs::rename`] on the thread that polls the future.
///
/// # Errors
///
/// Fails, for example, if `src` and `dst` are on different filesystems.
pub async fn rename<P, Q>(src: P, dst: Q) -> io::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    inline(|| std::fs::rename(src, dst)).await
}

/// Changes the permissions of the file or directory at `path` to `perm`.
///
/// Runs [`std::fs::set_permissions`] on the thread that polls the future. To change only some of
/// the permissions, start from the ones that [`metadata`] reads. WASI has no permissions, so there
/// it fails with [`Unsupported`](io::ErrorKind::Unsupported).
pub async fn set_permissions<P>(path: P, perm: Permissions) -> io::Result<()>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::set_permissions(path, perm)).await
}

/// Reads the metadata of the file, directory or symbolic link at `path`, without following a link.
///
/// Runs [`std::fs::symlink_metadata`] on the thread that polls the future. To read the metadata of
/// what a link points at, use [`metadata`].
pub async fn symlink_metadata<P>(path: P) -> io::Result<Metadata>
where
    P: AsRef<Path>,
{
    inline(|| std::fs::symlink_metadata(path)).await
}

/// Writes `contents` to the file at `path`, creating the file if it does not exist and replacing
/// its contents if it does.
///
/// Runs [`std::fs::write`] on the thread that polls the future.
pub async fn write<P, C>(path: P, contents: C) -> io::Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    inline(|| std::fs::write(path, contents)).await
}

/// What the `unix` and `windows` modules seal their extension traits with, out of reach of every
/// crate but this one.
///
/// The trait is public in name only, so that the extension traits can name it as their
/// supertrait, and sits in a module nobody outside can reach, so that nobody outside can implement
/// it: the types of this module that it is implemented for are the only ones the extension traits
/// are meant for.
#[cfg(any(unix, windows))]
pub(crate) mod sealed {
    /// Marks a type of this module that an extension trait is implemented for.
    pub trait Sealed {}
}

/// Runs `operation` within the poll that is running this future, and hands back what it returns.
///
/// This is the future of every function of the module that makes a single call. It does nothing
/// until it is first polled. It then waits for its turn if the poll has spent its budget, and
/// counts against the budget once the operation has run.
async fn inline<T, F>(operation: F) -> T
where
    F: FnOnce() -> T,
{
    poll_fn(coop::poll_proceed).await;
    let output = operation();
    coop::spend();

    output
}
