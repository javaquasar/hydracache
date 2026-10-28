//! [`File`], an open file with async reads and writes.

use std::{
    fmt,
    io::{self, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write},
    path::Path,
    pin::Pin,
    task::{Context, Poll, ready},
};

#[cfg(any(unix, target_os = "wasi"))]
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, OwnedHandle, RawHandle};

use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};

use super::{Metadata, Permissions, inline};
use crate::coop;

/// An open file, whose reads, writes and seeks run on the thread that polls them.
///
/// Open a file with [`open`](File::open), [`create`](File::create) or
/// [`OpenOptions`](super::OpenOptions), or convert a [`std::fs::File`] with [`From`]. `File`
/// implements `futures-io`'s [`AsyncRead`], [`AsyncWrite`] and [`AsyncSeek`]. The other methods of
/// std's file are `async` methods here.
///
/// Like the rest of the module, it needs no runtime and works under any executor. It is `Send`,
/// `Sync` and `Unpin`.
///
/// The file is closed when it is dropped. An error from closing it is lost, so call
/// [`sync_all`](File::sync_all) before dropping the file to learn of one.
///
/// # Reading and writing
///
/// A read, a write or a seek is one call to the OS, made in the poll that runs it, and goes
/// straight to the file. A read returns what the OS gave for the buffer it was handed, and nothing
/// is read ahead. A write completes once the OS has the bytes, and fails with the error the OS
/// reports for it. The position of the file is the one the OS keeps, which reads, writes and seeks
/// all move, as they do with the file of std.
///
/// A file that is not a regular file, such as a pipe, a socket or a terminal, can have nothing to
/// read or no room for a write, and the call then blocks the thread until it has. See
/// [Blocking](super#blocking) for what that costs, and [Files](super#files) for what to use
/// instead.
///
/// # Raw handles
///
/// On unix and WASI, the file implements `AsFd` and `AsRawFd`. On Windows, it implements `AsHandle`
/// and `AsRawHandle`. They give the raw descriptor or handle, for calls that std or the OS has and
/// this type does not. The file keeps no state of its own, so what is done through them, such as a
/// seek, is what its next read or write finds.
///
/// # Example
///
/// This example writes to a file, then reads it through the same handle from a position it seeks
/// to. It drives the futures with `block_on` from the `futures` crate, but the `block_on` of any
/// executor works, as the file needs no runtime:
///
/// ```
/// use std::io::SeekFrom;
///
/// use futures::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, executor::block_on};
/// use zruntime::fs::OpenOptions;
///
/// # let pid = std::process::id();
/// # let dir = std::env::temp_dir().join(format!("zruntime-fs-doc-file-{pid}"));
/// # std::fs::create_dir_all(&dir).unwrap();
/// block_on(async {
///     let mut file = OpenOptions::new()
///         .read(true)
///         .write(true)
///         .create(true)
///         .open(dir.join("notes.txt"))
///         .await?;
///
///     file.write_all(b"hello, world").await?;
///     // The write went straight to the file, so its length counts the bytes written.
///     assert_eq!(file.metadata().await?.len(), 12);
///
///     file.seek(SeekFrom::Start(7)).await?;
///     let mut word = String::new();
///     file.read_to_string(&mut word).await?;
///     assert_eq!(word, "world");
///
///     std::io::Result::Ok(())
/// })
/// .unwrap();
/// # std::fs::remove_dir_all(&dir).unwrap();
/// ```
pub struct File(std::fs::File);

impl File {
    /// Opens the file at `path` for reading.
    ///
    /// Runs [`std::fs::File::open`] on the thread that polls the future. To open a file in other
    /// ways, use [`OpenOptions`](super::OpenOptions).
    ///
    /// # Errors
    ///
    /// Fails if there is no file at `path`, or if the process may not read it.
    pub async fn open<P>(path: P) -> io::Result<File>
    where
        P: AsRef<Path>,
    {
        let file = inline(|| std::fs::File::open(path)).await?;
        Ok(File::from(file))
    }

    /// Opens the file at `path` for writing, creating it if it does not exist and emptying it if it
    /// does.
    ///
    /// Runs [`std::fs::File::create`] on the thread that polls the future. The file cannot be read
    /// through the returned handle. To open one that can, use
    /// [`OpenOptions`](super::OpenOptions).
    ///
    /// # Errors
    ///
    /// Fails if the directory of the file does not exist, or if the process may not write to the
    /// file.
    pub async fn create<P>(path: P) -> io::Result<File>
    where
        P: AsRef<Path>,
    {
        let file = inline(|| std::fs::File::create(path)).await?;
        Ok(File::from(file))
    }

    /// Asks the OS to write the data and metadata of the file to disk.
    ///
    /// Runs [`std::fs::File::sync_all`] on the thread that polls the future. It tells whether the
    /// file reached the disk. It also reports an error that a write, or closing the file, would
    /// otherwise lose.
    ///
    /// # Example
    ///
    /// ```
    /// use futures::{AsyncWriteExt, executor::block_on};
    /// use zruntime::fs::File;
    ///
    /// # let pid = std::process::id();
    /// # let dir = std::env::temp_dir().join(format!("zruntime-fs-doc-sync-all-{pid}"));
    /// # std::fs::create_dir_all(&dir).unwrap();
    /// block_on(async {
    ///     let mut file = File::create(dir.join("journal.txt")).await?;
    ///     file.write_all(b"entry").await?;
    ///     file.sync_all().await?;
    ///
    ///     std::io::Result::Ok(())
    /// })
    /// .unwrap();
    /// # std::fs::remove_dir_all(&dir).unwrap();
    /// ```
    pub async fn sync_all(&self) -> io::Result<()> {
        inline(|| self.0.sync_all()).await
    }

    /// Asks the OS to write the data of the file to disk.
    ///
    /// Runs [`std::fs::File::sync_data`] on the thread that polls the future. The metadata is
    /// written only if it is needed to read the data back. This does less than
    /// [`sync_all`](File::sync_all) on a platform that can tell the two apart, and the same on one
    /// that cannot.
    pub async fn sync_data(&self) -> io::Result<()> {
        inline(|| self.0.sync_data()).await
    }

    /// Truncates or extends the file to `size` bytes.
    ///
    /// Runs [`std::fs::File::set_len`] on the thread that polls the future. An extended file is
    /// filled with zeros. The position of the file does not change, even if it is now past the
    /// end.
    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        inline(|| self.0.set_len(size)).await
    }

    /// Reads the metadata of the file.
    ///
    /// Runs [`std::fs::File::metadata`] on the thread that polls the future. The length it reports
    /// includes every byte written before the call.
    pub async fn metadata(&self) -> io::Result<Metadata> {
        inline(|| self.0.metadata()).await
    }

    /// Changes the permissions of the file to `perm`.
    ///
    /// Runs [`std::fs::File::set_permissions`] on the thread that polls the future. WASI has no
    /// permissions, so there it fails with [`Unsupported`](io::ErrorKind::Unsupported).
    pub async fn set_permissions(&self, perm: Permissions) -> io::Result<()> {
        inline(|| self.0.set_permissions(perm)).await
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl From<std::fs::File> for File {
    /// Converts a [`std::fs::File`] into an async `File`, keeping its current position.
    fn from(file: std::fs::File) -> Self {
        Self(file)
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl From<OwnedFd> for File {
    fn from(fd: OwnedFd) -> Self {
        Self::from(std::fs::File::from(fd))
    }
}

#[cfg(windows)]
impl From<OwnedHandle> for File {
    fn from(handle: OwnedHandle) -> Self {
        Self::from(std::fs::File::from(handle))
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl AsFd for File {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl AsRawFd for File {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsHandle for File {
    fn as_handle(&self) -> BorrowedHandle<'_> {
        self.0.as_handle()
    }
}

#[cfg(windows)]
impl AsRawHandle for File {
    fn as_raw_handle(&self) -> RawHandle {
        self.0.as_raw_handle()
    }
}

impl AsyncRead for File {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        poll_inline(cx, || (&self.0).read(buf))
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        poll_inline(cx, || (&self.0).read_vectored(bufs))
    }
}

impl AsyncWrite for File {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        poll_inline(cx, || (&self.0).write(buf))
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        poll_inline(cx, || (&self.0).write_vectored(bufs))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        poll_inline(cx, || (&self.0).flush())
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // The file is closed when it is dropped, so closing it is a flush.
        self.poll_flush(cx)
    }
}

impl AsyncSeek for File {
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        poll_inline(cx, || (&self.0).seek(pos))
    }
}

/// Runs `operation` within the poll that calls this, as [`inline`] does for a future, and retries
/// it at once if the OS cut it short with `Interrupted`.
///
/// A retry spends nothing, so a call that was interrupted counts against the budget once, however
/// many tries it took.
fn poll_inline<T, F>(cx: &mut Context<'_>, mut operation: F) -> Poll<io::Result<T>>
where
    F: FnMut() -> io::Result<T>,
{
    ready!(coop::poll_proceed(cx));
    let result = loop {
        match operation() {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => break result,
        }
    };
    coop::spend();

    Poll::Ready(result)
}
