//! [`OpenOptions`], a builder for opening files.

use std::{future::Future, io, path::Path};

#[cfg(any(unix, windows))]
use super::sealed::Sealed;
use super::{File, inline};

/// A builder for opening a file: for reading, writing or both, and for what to do if the file does
/// or does not exist.
///
/// Set the options on the builder, then call [`open`](OpenOptions::open) to open a file with them.
/// A builder can open any number of files. It wraps [`std::fs::OpenOptions`], with the same
/// defaults and the same combinations that make an open fail.
///
/// On unix, the `OpenOptionsExt` trait of the `unix` module adds the permission bits of a new file
/// and custom open flags. On Windows, the trait of that name in the `windows` module adds options
/// such as the access mode, the share mode and the attributes.
///
/// # Example
///
/// A file opened for appending gets every write at its end, whatever its position is:
///
/// ```
/// use futures::{AsyncWriteExt, executor::block_on};
/// use zruntime::fs::{self, OpenOptions};
///
/// # let pid = std::process::id();
/// # let dir = std::env::temp_dir().join(format!("zruntime-fs-doc-open-options-{pid}"));
/// # std::fs::create_dir_all(&dir).unwrap();
/// block_on(async {
///     let path = dir.join("log.txt");
///     fs::write(&path, "first\n").await?;
///
///     let mut log = OpenOptions::new().append(true).open(&path).await?;
///     log.write_all(b"second\n").await?;
///     log.flush().await?;
///
///     assert_eq!(fs::read_to_string(&path).await?, "first\nsecond\n");
///
///     std::io::Result::Ok(())
/// })
/// .unwrap();
/// # std::fs::remove_dir_all(&dir).unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct OpenOptions(std::fs::OpenOptions);

impl OpenOptions {
    /// Creates a builder with every option off.
    ///
    /// [`open`](OpenOptions::open) fails until at least one of [`read`](OpenOptions::read),
    /// [`write`](OpenOptions::write) and [`append`](OpenOptions::append) is on.
    pub fn new() -> Self {
        Self(std::fs::OpenOptions::new())
    }

    /// Sets whether the file is opened for reading.
    pub fn read(&mut self, read: bool) -> &mut Self {
        self.0.read(read);
        self
    }

    /// Sets whether the file is opened for writing.
    ///
    /// Writing to an existing file overwrites its contents from the start of the file and does not
    /// shorten it. To empty it on opening, use [`truncate`](OpenOptions::truncate).
    pub fn write(&mut self, write: bool) -> &mut Self {
        self.0.write(write);
        self
    }

    /// Sets whether the file is opened for appending, which also opens it for writing.
    ///
    /// Every write goes to the end of the file, wherever its position is.
    pub fn append(&mut self, append: bool) -> &mut Self {
        self.0.append(append);
        self
    }

    /// Sets whether an existing file is emptied when it is opened.
    ///
    /// Opening fails unless the file is also opened for [writing](OpenOptions::write).
    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.0.truncate(truncate);
        self
    }

    /// Sets whether the file is created if it does not exist.
    ///
    /// Opening fails unless the file is also opened for [writing](OpenOptions::write) or
    /// [appending](OpenOptions::append). An existing file is opened as it is.
    pub fn create(&mut self, create: bool) -> &mut Self {
        self.0.create(create);
        self
    }

    /// Sets whether opening must create the file, and fail with
    /// [`AlreadyExists`](io::ErrorKind::AlreadyExists) if the file exists.
    ///
    /// The check and the creation are one step. If several callers try to create the same file at
    /// once, exactly one succeeds. A symbolic link at the path to open is not followed, and its
    /// presence also fails the open. While this is on, [`create`](OpenOptions::create) and
    /// [`truncate`](OpenOptions::truncate) have no effect. Like them, it needs the file to be
    /// opened for [writing](OpenOptions::write) or [appending](OpenOptions::append).
    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.0.create_new(create_new);
        self
    }

    /// Opens the file at `path` with the options of the builder.
    ///
    /// Runs [`std::fs::OpenOptions::open`] on the thread that polls the future. The returned future
    /// does not borrow the builder, so the builder can be changed or dropped right away. The
    /// future does nothing until it is polled.
    ///
    /// # Errors
    ///
    /// Fails for the reasons the options give, for example a missing file with `create` off, and
    /// for the reasons the OS gives, for example a lack of permission.
    pub fn open<P>(&self, path: P) -> impl Future<Output = io::Result<File>> + use<P>
    where
        P: AsRef<Path>,
    {
        let options = self.0.clone();
        let path = path.as_ref().to_owned();
        async move {
            let file = inline(|| options.open(path)).await?;
            Ok(File::from(file))
        }
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(unix, windows))]
impl Sealed for OpenOptions {}

#[cfg(unix)]
impl super::unix::OpenOptionsExt for OpenOptions {
    fn mode(&mut self, mode: u32) -> &mut Self {
        std::os::unix::fs::OpenOptionsExt::mode(&mut self.0, mode);
        self
    }

    fn custom_flags(&mut self, flags: i32) -> &mut Self {
        std::os::unix::fs::OpenOptionsExt::custom_flags(&mut self.0, flags);
        self
    }
}

#[cfg(windows)]
impl super::windows::OpenOptionsExt for OpenOptions {
    fn access_mode(&mut self, access: u32) -> &mut Self {
        std::os::windows::fs::OpenOptionsExt::access_mode(&mut self.0, access);
        self
    }

    fn share_mode(&mut self, share: u32) -> &mut Self {
        std::os::windows::fs::OpenOptionsExt::share_mode(&mut self.0, share);
        self
    }

    fn custom_flags(&mut self, flags: u32) -> &mut Self {
        std::os::windows::fs::OpenOptionsExt::custom_flags(&mut self.0, flags);
        self
    }

    fn attributes(&mut self, attributes: u32) -> &mut Self {
        std::os::windows::fs::OpenOptionsExt::attributes(&mut self.0, attributes);
        self
    }

    fn security_qos_flags(&mut self, flags: u32) -> &mut Self {
        std::os::windows::fs::OpenOptionsExt::security_qos_flags(&mut self.0, flags);
        self
    }
}
