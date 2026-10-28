//! Tests of `zruntime::fs`, async access to the filesystem on the thread that polls it.
//!
//! Each test works in a directory of its own under the temporary directory of the OS, which goes
//! with everything in it when the test ends, and drives the futures with the `block_on` of
//! `futures`, but for those of the budget, which drive theirs with the `block_on` of a runtime or
//! poll them by hand. They come in the order of what they pin down: that each function does what
//! the function of std it runs does, and takes paths and bytes of any type that converts; that the
//! stream of a directory lists every entry, and that an entry knows what it is; that the builders
//! of directories and of files set what they are told to, that their futures do not borrow them and
//! do nothing before they are polled; that a file hands back what it was given, wherever it is
//! sought to, and that a write after a read lands where the reads got to; that what looks at a file
//! as a whole sees every byte written before it, and that a file dropped right after a write loses
//! none; that a file gives out its raw handle and prints; and, where the platform has them, what
//! its extension traits add. The last tests are those of the budget of operations that a poll may
//! complete: that a task in a loop of them yields to the next one once the budget is spent, that a
//! blocking executor that a task runs gets its operations done inside `unconstrained`, and that
//! nothing yields outside a runtime.

use std::{
    env,
    io::{self, SeekFrom},
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

use futures::{
    AsyncReadExt, AsyncSeekExt, AsyncWriteExt, StreamExt, executor::block_on,
    future::poll_immediate,
};
use ntest::timeout;

use crate::fs::{self, DirBuilder, DirEntry, File, OpenOptions, ReadDir};

#[cfg(feature = "runtime")]
use super::async_io::{BUDGET, READS, note_progress};
#[cfg(unix)]
use crate::fs::unix::{self, DirBuilderExt, DirEntryExt, OpenOptionsExt, PermissionsExt};
#[cfg(feature = "runtime")]
use crate::{Local, Runtime, Shared, unconstrained};
#[cfg(windows)]
use std::os::windows::io::{AsHandle, AsRawHandle};
#[cfg(unix)]
use std::os::{
    fd::{AsFd, AsRawFd, OwnedFd},
    unix::{fs::MetadataExt, net::UnixStream},
};
#[cfg(feature = "runtime")]
use std::{
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Waker},
};

/// Writes the test that follows once per flavour: a module named after it, holding a `local` and
/// a `shared` test that run its body with the type its parameter names standing for
/// [`Local`] and for [`Shared`].
///
/// The body is written out once for each flavour rather than once over a mode parameter, as the
/// `spawn` of each takes bounds of its own. What it uses has to meet both: futures that are `Send`.
#[cfg(feature = "runtime")]
macro_rules! in_both_modes {
    ($(#[$attr:meta])* fn $name:ident<$mode:ident>() $body:block) => {
        $(#[$attr])*
        mod $name {
            use super::*;

            #[test]
            #[timeout(15000)]
            fn local() {
                type $mode = Local;

                $body
            }

            #[test]
            #[timeout(15000)]
            fn shared() {
                type $mode = Shared;

                $body
            }
        }
    };
}

/// A path comes out of `canonicalize` with its dots resolved, as std resolves them.
#[test]
#[timeout(15000)]
fn canonicalize_resolves_dots() {
    let dir = TempDir::new();
    std::fs::create_dir(dir.join("sub")).unwrap();

    let canonical = block_on(fs::canonicalize(dir.join("sub").join("..").join("sub"))).unwrap();

    assert_eq!(canonical, std::fs::canonicalize(dir.join("sub")).unwrap());
}

/// A path that is not there cannot be canonicalized.
#[test]
#[timeout(15000)]
fn canonicalize_fails_for_a_missing_path() {
    let dir = TempDir::new();

    let error = block_on(fs::canonicalize(dir.join("missing"))).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

/// `copy` makes the destination hold what the source does, and says how many bytes that is.
#[test]
#[timeout(15000)]
fn copy_copies_the_contents_and_counts_the_bytes() {
    let dir = TempDir::new();
    std::fs::write(dir.join("from"), "some text").unwrap();

    let copied = block_on(fs::copy(dir.join("from"), dir.join("to"))).unwrap();

    assert_eq!(copied, 9);
    assert_eq!(
        std::fs::read_to_string(dir.join("to")).unwrap(),
        "some text"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("from")).unwrap(),
        "some text"
    );
}

/// `copy` of a file that is not there fails.
#[test]
#[timeout(15000)]
fn copy_fails_for_a_missing_source() {
    let dir = TempDir::new();

    let error = block_on(fs::copy(dir.join("missing"), dir.join("to"))).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(!dir.join("to").exists());
}

/// `create_dir` makes a directory in one that is there, and refuses one that is there itself.
#[test]
#[timeout(15000)]
fn create_dir_makes_one_directory() {
    let dir = TempDir::new();

    block_on(fs::create_dir(dir.join("new"))).unwrap();
    let error = block_on(fs::create_dir(dir.join("new"))).unwrap_err();

    assert!(dir.join("new").is_dir());
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
}

/// `create_dir` does not make the parents that are missing.
#[test]
#[timeout(15000)]
fn create_dir_fails_without_a_parent() {
    let dir = TempDir::new();

    let error = block_on(fs::create_dir(dir.join("missing").join("new"))).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

/// `create_dir_all` makes every parent that is missing, and takes a directory that is there as
/// done.
#[test]
#[timeout(15000)]
fn create_dir_all_makes_every_missing_parent() {
    let dir = TempDir::new();
    let deep = dir.join("one").join("two").join("three");

    block_on(fs::create_dir_all(&deep)).unwrap();
    block_on(fs::create_dir_all(&deep)).unwrap();

    assert!(deep.is_dir());
}

/// The second name that `hard_link` makes is the same file as the first.
#[test]
#[timeout(15000)]
fn hard_link_makes_a_second_name_for_the_same_file() {
    let dir = TempDir::new();
    std::fs::write(dir.join("first"), "before").unwrap();

    block_on(fs::hard_link(dir.join("first"), dir.join("second"))).unwrap();
    std::fs::write(dir.join("second"), "after").unwrap();

    assert_eq!(std::fs::read_to_string(dir.join("first")).unwrap(), "after");
}

/// `metadata` tells a file from a directory, and gives the length of a file, as `symlink_metadata`
/// does for what is not a link.
#[test]
#[timeout(15000)]
fn metadata_tells_the_length_and_the_kind() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "four").unwrap();

    let file: fs::Metadata = block_on(fs::metadata(dir.join("file"))).unwrap();
    let directory = block_on(fs::metadata(dir.path())).unwrap();
    let not_a_link = block_on(fs::symlink_metadata(dir.join("file"))).unwrap();

    assert!(file.is_file());
    assert_eq!(file.len(), 4);
    assert!(directory.is_dir());
    assert!(not_a_link.is_file());
    assert_eq!(not_a_link.len(), 4);
}

/// `metadata` of a path that is not there fails.
#[test]
#[timeout(15000)]
fn metadata_fails_for_a_missing_path() {
    let dir = TempDir::new();

    let error = block_on(fs::metadata(dir.join("missing"))).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

/// `read` hands back every byte of a file, those that are not text included.
#[test]
#[timeout(15000)]
fn read_returns_the_bytes_of_a_file() {
    let dir = TempDir::new();
    let data = bytes(100_000);
    std::fs::write(dir.join("file"), &data).unwrap();

    assert_eq!(block_on(fs::read(dir.join("file"))).unwrap(), data);
}

/// `read_to_string` hands back the text of a file.
#[test]
#[timeout(15000)]
fn read_to_string_returns_the_text_of_a_file() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "héllo").unwrap();

    assert_eq!(
        block_on(fs::read_to_string(dir.join("file"))).unwrap(),
        "héllo"
    );
}

/// `read_to_string` refuses bytes that are not UTF-8.
#[test]
#[timeout(15000)]
fn read_to_string_fails_for_bytes_that_are_not_utf8() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), [0xff, 0xfe]).unwrap();

    let error = block_on(fs::read_to_string(dir.join("file"))).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

/// `remove_dir` removes an empty directory, and not one with something in it.
#[test]
#[timeout(15000)]
fn remove_dir_removes_an_empty_directory_only() {
    let dir = TempDir::new();
    std::fs::create_dir(dir.join("empty")).unwrap();
    std::fs::create_dir(dir.join("full")).unwrap();
    std::fs::write(dir.join("full").join("file"), "").unwrap();

    block_on(fs::remove_dir(dir.join("empty"))).unwrap();
    let full = block_on(fs::remove_dir(dir.join("full")));

    assert!(!dir.join("empty").exists());
    assert!(full.is_err());
    assert!(dir.join("full").join("file").exists());
}

/// `remove_dir_all` removes a directory with everything in it, down the tree.
#[test]
#[timeout(15000)]
fn remove_dir_all_removes_a_tree() {
    let dir = TempDir::new();
    let tree = dir.join("tree");
    std::fs::create_dir_all(tree.join("a").join("b")).unwrap();
    std::fs::write(tree.join("a").join("b").join("file"), "deep").unwrap();
    std::fs::write(tree.join("file"), "shallow").unwrap();

    block_on(fs::remove_dir_all(&tree)).unwrap();

    assert!(!tree.exists());
}

/// `remove_file` removes a file, and fails for one that is not there.
#[test]
#[timeout(15000)]
fn remove_file_removes_a_file() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();

    block_on(fs::remove_file(dir.join("file"))).unwrap();
    let error = block_on(fs::remove_file(dir.join("file"))).unwrap_err();

    assert!(!dir.join("file").exists());
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

/// `rename` moves a file to a new name, in place of the one that was at that name.
#[test]
#[timeout(15000)]
fn rename_moves_a_file_over_another() {
    let dir = TempDir::new();
    std::fs::write(dir.join("old"), "moved").unwrap();
    std::fs::write(dir.join("new"), "replaced").unwrap();

    block_on(fs::rename(dir.join("old"), dir.join("new"))).unwrap();

    assert!(!dir.join("old").exists());
    assert_eq!(std::fs::read_to_string(dir.join("new")).unwrap(), "moved");
}

/// `set_permissions` makes a file read-only, and writable again.
#[test]
#[timeout(15000)]
fn set_permissions_changes_the_permissions_of_a_file() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();
    let writable: fs::Permissions = std::fs::metadata(dir.join("file")).unwrap().permissions();
    let mut read_only = writable.clone();
    read_only.set_readonly(true);

    block_on(fs::set_permissions(dir.join("file"), read_only)).unwrap();
    let after = std::fs::metadata(dir.join("file"))
        .unwrap()
        .permissions()
        .readonly();
    // Windows does not remove a file that is read-only, so the directory could not go otherwise.
    block_on(fs::set_permissions(dir.join("file"), writable)).unwrap();

    assert!(after);
    assert!(
        !std::fs::metadata(dir.join("file"))
            .unwrap()
            .permissions()
            .readonly()
    );
}

/// `write` makes a file with the bytes it is given, and replaces what a file held.
#[test]
#[timeout(15000)]
fn write_creates_a_file_and_replaces_its_contents() {
    let dir = TempDir::new();

    block_on(fs::write(dir.join("file"), "a longer text")).unwrap();
    block_on(fs::write(dir.join("file"), "short")).unwrap();

    assert_eq!(std::fs::read_to_string(dir.join("file")).unwrap(), "short");
}

/// The paths and the bytes may be of any type that converts to a path or to bytes, owned or
/// borrowed.
#[test]
#[timeout(15000)]
fn paths_and_bytes_may_be_of_any_type_that_converts() {
    let dir = TempDir::new();
    let path = dir.join("file");
    let as_string = path.to_str().unwrap().to_owned();

    block_on(async {
        fs::write(&as_string, b"array").await.unwrap();
        fs::write(as_string.as_str(), vec![b'v']).await.unwrap();
        fs::write(path.clone(), String::from("string"))
            .await
            .unwrap();
        fs::write(path.as_path(), &b"slice"[..]).await.unwrap();
        fs::write(path.as_os_str(), "os str").await.unwrap();
        assert_eq!(fs::read_to_string(as_string).await.unwrap(), "os str");
        assert_eq!(fs::read(path).await.unwrap(), b"os str");
    });
}

/// The futures of the functions are `Send`, and `'static` where their arguments are, as a task
/// that awaits them needs. They are not polled, so nothing is done.
#[test]
#[timeout(15000)]
fn the_futures_are_send() {
    fn send<T>(_: T)
    where
        T: Send,
    {
    }

    fn send_and_static<T>(_: T)
    where
        T: Send + 'static,
    {
    }

    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();
    let file = block_on(File::open(dir.join("file"))).unwrap();
    let entry = block_on(block_on(fs::read_dir(dir.path())).unwrap().next())
        .unwrap()
        .unwrap();

    send_and_static(fs::read(String::new()));
    send_and_static(fs::write(PathBuf::new(), Vec::<u8>::new()));
    send_and_static(fs::read_dir(PathBuf::new()));
    send_and_static(fs::copy(String::new(), String::new()));
    send_and_static(File::open(String::new()));
    send_and_static(DirBuilder::new().create(String::new()));
    send_and_static(OpenOptions::new().open(String::new()));
    send(entry.metadata());
    send(entry.file_type());
    send(file.sync_all());
    send(file.sync_data());
    send(file.set_len(0));
    send(file.metadata());
    send(file.set_permissions(std::fs::metadata(dir.join("file")).unwrap().permissions()));
}

/// The types are `Send` and `Sync`, and the stream and the file are `Unpin`, so that a task that
/// holds one can move between threads and a combinator can poll it.
#[test]
#[timeout(15000)]
fn the_types_are_send_and_sync() {
    fn send_and_sync<T>()
    where
        T: Send + Sync,
    {
    }

    fn unpinned<T>()
    where
        T: Unpin,
    {
    }

    send_and_sync::<File>();
    send_and_sync::<ReadDir>();
    send_and_sync::<DirEntry>();
    send_and_sync::<DirBuilder>();
    send_and_sync::<OpenOptions>();
    unpinned::<File>();
    unpinned::<ReadDir>();
}

/// `read_link` fails for a path that is not a symbolic link, and for one that is not there.
#[test]
#[timeout(15000)]
fn read_link_fails_for_a_path_that_is_no_link() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();

    assert!(block_on(fs::read_link(dir.join("file"))).is_err());
    let error = block_on(fs::read_link(dir.join("missing"))).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

/// A symbolic link is read for where it points to, and `symlink_metadata` looks at the link and
/// `metadata` at what it points to.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn symlinks_are_made_read_and_not_followed_by_symlink_metadata() {
    let dir = TempDir::new();
    std::fs::write(dir.join("target"), "pointed at").unwrap();

    block_on(unix::symlink(dir.join("target"), dir.join("link"))).unwrap();

    assert_eq!(
        block_on(fs::read_link(dir.join("link"))).unwrap(),
        dir.join("target")
    );
    let link = block_on(fs::symlink_metadata(dir.join("link"))).unwrap();
    assert!(link.file_type().is_symlink());
    let followed = block_on(fs::metadata(dir.join("link"))).unwrap();
    assert!(followed.file_type().is_file());
    assert_eq!(
        block_on(fs::read_to_string(dir.join("link"))).unwrap(),
        "pointed at"
    );
}

/// The metadata of an entry that is a symbolic link describes the link, not what it points at,
/// which the metadata of the entry's path does.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn the_metadata_of_an_entry_does_not_follow_a_link() {
    let dir = TempDir::new();
    std::fs::write(dir.join("target"), "pointed at").unwrap();
    std::os::unix::fs::symlink(dir.join("target"), dir.join("link")).unwrap();

    block_on(async {
        let mut entries = fs::read_dir(dir.path()).await.unwrap();
        while let Some(entry) = entries.next().await {
            let entry = entry.unwrap();
            if entry.file_name() != "link" {
                continue;
            }

            assert!(entry.metadata().await.unwrap().file_type().is_symlink());
            let followed = fs::metadata(entry.path()).await.unwrap();
            assert!(followed.file_type().is_file());
            return;
        }
        panic!("the directory lists the link");
    });
}

/// A stream of a directory lists every entry of it, files and directories alike, and then ends.
#[test]
#[timeout(15000)]
fn read_dir_lists_every_entry() {
    let dir = TempDir::new();
    let mut expected = Vec::new();
    for index in 0..40 {
        std::fs::write(dir.join(format!("file-{index:02}")), "").unwrap();
        expected.push(format!("file-{index:02}"));
    }
    for name in ["dir-a", "dir-b", "dir-c"] {
        std::fs::create_dir(dir.join(name)).unwrap();
        expected.push(name.to_owned());
    }
    expected.sort();

    let mut entries = block_on(fs::read_dir(dir.path())).unwrap();
    let mut listed = Vec::new();
    while let Some(entry) = block_on(entries.next()) {
        listed.push(entry.unwrap().file_name().into_string().unwrap());
    }
    listed.sort();

    assert_eq!(listed, expected);
    assert!(block_on(entries.next()).is_none());
}

/// The stream of an empty directory ends at once.
#[test]
#[timeout(15000)]
fn read_dir_of_an_empty_directory_yields_nothing() {
    let dir = TempDir::new();

    let mut entries = block_on(fs::read_dir(dir.path())).unwrap();

    assert!(block_on(entries.next()).is_none());
}

/// A directory that is not there, and a path that is not a directory, cannot be read.
#[test]
#[timeout(15000)]
fn read_dir_fails_for_a_path_that_is_no_directory() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();

    let missing = block_on(fs::read_dir(dir.join("missing"))).unwrap_err();
    let file = block_on(fs::read_dir(dir.join("file")));

    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    assert!(file.is_err());
}

/// An entry gives its name and its path, and its type and its metadata. A clone of it is the same
/// entry.
#[test]
#[timeout(15000)]
fn entries_know_their_path_name_type_and_metadata() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "12345").unwrap();
    std::fs::create_dir(dir.join("sub")).unwrap();

    let mut entries = block_on(fs::read_dir(dir.path())).unwrap();
    let mut seen = Vec::new();
    while let Some(entry) = block_on(entries.next()) {
        let entry = entry.unwrap();
        let clone = entry.clone();
        assert_eq!(clone.path(), entry.path());
        assert!(format!("{entry:?}").contains(entry.file_name().to_str().unwrap()));
        let file_type = block_on(entry.file_type()).unwrap();
        let metadata = block_on(clone.metadata()).unwrap();
        seen.push((entry.path(), file_type.is_dir(), metadata.len()));
    }
    seen.sort();

    assert_eq!(seen[0].0, dir.join("file"));
    assert!(!seen[0].1);
    assert_eq!(seen[0].2, 5);
    assert_eq!(seen[1].0, dir.join("sub"));
    assert!(seen[1].1);
    assert_eq!(seen.len(), 2);
}

/// The stream of a directory prints as a struct, without anything of the directory.
#[test]
#[timeout(15000)]
fn a_stream_of_a_directory_prints_as_a_struct() {
    let dir = TempDir::new();

    let entries = block_on(fs::read_dir(dir.path())).unwrap();

    assert_eq!(format!("{entries:?}"), "ReadDir { .. }");
}

/// An entry gives the inode number the directory holds for it, which is the one of the file.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn entries_know_their_inode_number() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();

    let mut entries = block_on(fs::read_dir(dir.path())).unwrap();
    let entry = block_on(entries.next()).unwrap().unwrap();

    let metadata = std::fs::metadata(dir.join("file")).unwrap();
    assert_eq!(entry.ino(), metadata.ino());
}

/// A builder makes the directory it is given the path of, in one that is there, and neither makes
/// the parents that are missing nor takes a directory that is there as done.
#[test]
#[timeout(15000)]
fn a_dir_builder_makes_one_directory_by_default() {
    let dir = TempDir::new();
    let builder = DirBuilder::new();

    block_on(builder.create(dir.join("new"))).unwrap();
    let again = block_on(builder.create(dir.join("new"))).unwrap_err();
    let orphan = block_on(builder.create(dir.join("missing").join("new"))).unwrap_err();

    assert!(dir.join("new").is_dir());
    assert_eq!(again.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(orphan.kind(), io::ErrorKind::NotFound);
}

/// A recursive builder makes the parents that are missing, and takes a directory that is there as
/// done, and a builder that is turned back to not recursive stops doing both.
#[test]
#[timeout(15000)]
fn a_recursive_dir_builder_makes_every_missing_parent() {
    let dir = TempDir::new();
    let deep = dir.join("one").join("two");
    let mut builder = DirBuilder::default();
    builder.recursive(true);

    block_on(builder.create(&deep)).unwrap();
    block_on(builder.create(&deep)).unwrap();
    builder.recursive(false);
    let again = block_on(builder.create(&deep)).unwrap_err();

    assert!(deep.is_dir());
    assert_eq!(again.kind(), io::ErrorKind::AlreadyExists);
}

/// The future of a builder does not borrow it: the builder is gone at the end of the statement
/// that made the future, and the future is polled after that.
#[test]
#[timeout(15000)]
fn the_futures_of_the_builders_do_not_borrow_them() {
    let dir = TempDir::new();
    let path = dir.join("file");
    let new_dir = dir.join("new");

    let make_dir = DirBuilder::new().recursive(true).create(&new_dir);
    let open_file = OpenOptions::new().write(true).create(true).open(&path);

    assert!(block_on(make_dir).is_ok());
    assert!(block_on(open_file).is_ok());
    assert!(new_dir.is_dir());
    assert!(path.is_file());
}

/// The future of a builder does nothing before it is polled, as the future of a function does not.
#[test]
#[timeout(15000)]
fn the_futures_of_the_builders_do_nothing_before_they_are_polled() {
    let dir = TempDir::new();
    let make_dir = DirBuilder::new().create(dir.join("new"));
    let open_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(dir.join("file"));
    let write = fs::write(dir.join("written"), "text");

    thread::sleep(Duration::from_millis(50));

    assert!(!dir.join("new").exists());
    assert!(!dir.join("file").exists());
    assert!(!dir.join("written").exists());
    block_on(make_dir).unwrap();
    block_on(open_file).unwrap();
    block_on(write).unwrap();
    assert!(dir.join("new").exists());
    assert!(dir.join("file").exists());
    assert!(dir.join("written").exists());
}

/// A directory is made with the mode a unix builder is given, and so are its missing parents.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn a_dir_builder_sets_the_mode_of_new_directories() {
    let dir = TempDir::new();

    block_on(DirBuilder::new().mode(0o700).create(dir.join("one"))).unwrap();
    block_on(
        DirBuilder::new()
            .recursive(true)
            .mode(0o750)
            .create(dir.join("a").join("b")),
    )
    .unwrap();

    assert_eq!(mode_of(dir.join("one")), 0o700);
    assert_eq!(mode_of(dir.join("a")), 0o750);
    assert_eq!(mode_of(dir.join("a").join("b")), 0o750);
}

/// A file is not opened if it is not there, unless the options say that it is to be created.
#[test]
#[timeout(15000)]
fn open_fails_for_a_missing_file_unless_it_creates_it() {
    let dir = TempDir::new();

    let missing = block_on(OpenOptions::new().write(true).open(dir.join("file"))).unwrap_err();
    let created = block_on(
        OpenOptions::new()
            .write(true)
            .create(true)
            .open(dir.join("file")),
    );

    assert_eq!(missing.kind(), io::ErrorKind::NotFound);
    assert!(created.is_ok());
    assert!(dir.join("file").is_file());
}

/// `create_new` opens a file only if it is not there already.
#[test]
#[timeout(15000)]
fn create_new_fails_for_a_file_that_is_there() {
    let dir = TempDir::new();
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    block_on(options.open(dir.join("file"))).unwrap();
    let error = block_on(options.open(dir.join("file"))).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
}

/// A file opened to append to has every write go to its end, wherever it was sought to.
#[test]
#[timeout(15000)]
fn append_adds_to_the_end_of_a_file() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "abc").unwrap();

    block_on(async {
        let mut file = OpenOptions::new()
            .append(true)
            .open(dir.join("file"))
            .await
            .unwrap();
        file.seek(SeekFrom::Start(0)).await.unwrap();
        file.write_all(b"def").await.unwrap();
        file.flush().await.unwrap();
    });

    assert_eq!(std::fs::read_to_string(dir.join("file")).unwrap(), "abcdef");
}

/// A file opened to truncate is empty, and what is written to it is all it holds.
#[test]
#[timeout(15000)]
fn truncate_empties_a_file_that_is_there() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "a long text").unwrap();

    block_on(async {
        let opened = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(dir.join("file"))
            .await;
        let mut file = opened.unwrap();
        assert_eq!(file.metadata().await.unwrap().len(), 0);
        file.write_all(b"new").await.unwrap();
        file.flush().await.unwrap();
    });

    assert_eq!(std::fs::read_to_string(dir.join("file")).unwrap(), "new");
}

/// A file opened to write without truncating is overwritten from its start, and not shortened.
#[test]
#[timeout(15000)]
fn write_overwrites_a_file_without_shortening_it() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "abcdef").unwrap();

    block_on(async {
        let mut file = OpenOptions::new()
            .write(true)
            .open(dir.join("file"))
            .await
            .unwrap();
        file.write_all(b"XY").await.unwrap();
        file.flush().await.unwrap();
    });

    assert_eq!(std::fs::read_to_string(dir.join("file")).unwrap(), "XYcdef");
}

/// A write to a file opened for reading fails at the write, which is where the OS refuses it, and
/// leaves the file as it was.
#[test]
#[timeout(15000)]
fn a_write_to_a_file_opened_for_reading_fails() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "text").unwrap();

    block_on(async {
        let mut file = File::open(dir.join("file")).await.unwrap();
        assert!(file.write_all(b"x").await.is_err());
    });

    assert_eq!(std::fs::read_to_string(dir.join("file")).unwrap(), "text");
}

/// Options that open nothing, or that need to write and do not, fail to open a file.
#[test]
#[timeout(15000)]
fn open_fails_for_options_that_make_no_sense() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();

    let nothing = block_on(OpenOptions::new().open(dir.join("file"))).unwrap_err();
    let truncating = block_on(
        OpenOptions::new()
            .read(true)
            .truncate(true)
            .open(dir.join("file")),
    );

    assert_eq!(nothing.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(truncating.unwrap_err().kind(), io::ErrorKind::InvalidInput);
}

/// The options of a default builder are those of a new one, and a clone of a builder opens as the
/// builder does.
#[test]
#[timeout(15000)]
fn default_options_are_new_options_and_clones_open_alike() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "text").unwrap();
    let mut options = OpenOptions::default();
    options.read(true);

    let clone = options.clone();

    assert_eq!(
        format!("{:?}", OpenOptions::default()),
        format!("{:?}", OpenOptions::new())
    );
    assert_eq!(format!("{options:?}"), format!("{clone:?}"));
    assert!(block_on(clone.open(dir.join("file"))).is_ok());
}

/// A file is created with the mode a unix builder is given, and the mode has no effect on one that
/// is there.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn open_options_set_the_mode_of_new_files() {
    let dir = TempDir::new();
    std::fs::write(dir.join("old"), "").unwrap();
    let old_mode = mode_of(dir.join("old"));
    let mut options = OpenOptions::new();
    options.write(true).create(true).mode(0o600);

    block_on(options.open(dir.join("new"))).unwrap();
    block_on(options.open(dir.join("old"))).unwrap();

    assert_eq!(mode_of(dir.join("new")), 0o600);
    assert_eq!(mode_of(dir.join("old")), old_mode);
}

/// The flags a unix builder is given reach the call that opens the file: one that refuses to
/// follow a symbolic link fails to open one.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn open_options_pass_custom_flags() {
    let Some(no_follow) = O_NOFOLLOW else {
        return;
    };
    let dir = TempDir::new();
    std::fs::write(dir.join("target"), "").unwrap();
    std::os::unix::fs::symlink(dir.join("target"), dir.join("link")).unwrap();

    let followed = block_on(OpenOptions::new().read(true).open(dir.join("link")));
    let refused = block_on(
        OpenOptions::new()
            .read(true)
            .custom_flags(no_follow)
            .open(dir.join("link")),
    );

    assert!(followed.is_ok());
    assert!(refused.is_err());
}

/// What a file is written, it is read back as, however it is flushed to be there.
#[test]
#[timeout(15000)]
fn a_file_written_is_read_back() {
    let dir = TempDir::new();

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(b"hello, world").await.unwrap();
        file.flush().await.unwrap();

        let mut text = String::new();
        File::open(dir.join("file"))
            .await
            .unwrap()
            .read_to_string(&mut text)
            .await
            .unwrap();
        assert_eq!(text, "hello, world");
    });
}

/// A large file round-trips: written by one large buffer and by small ones, and read through large
/// buffers and small ones.
#[test]
#[timeout(15000)]
fn a_large_file_round_trips() {
    let dir = TempDir::new();
    let data = bytes(200_000);

    block_on(async {
        let mut file = File::create(dir.join("large")).await.unwrap();
        file.write_all(&data).await.unwrap();
        file.flush().await.unwrap();
        let mut file = File::create(dir.join("small")).await.unwrap();
        for chunk in data.chunks(777) {
            file.write_all(chunk).await.unwrap();
        }
        file.flush().await.unwrap();

        let mut large = Vec::new();
        File::open(dir.join("large"))
            .await
            .unwrap()
            .read_to_end(&mut large)
            .await
            .unwrap();
        assert_eq!(large, data);

        let mut file = File::open(dir.join("small")).await.unwrap();
        let mut small = Vec::new();
        let mut buf = [0; 100];
        loop {
            let len = file.read(&mut buf).await.unwrap();
            if len == 0 {
                break;
            }
            small.extend_from_slice(&buf[..len]);
        }
        assert_eq!(small, data);
    });
}

/// A file with nothing in it reads as the end at once.
#[test]
#[timeout(15000)]
fn an_empty_file_reads_as_the_end() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();

    block_on(async {
        let mut file = File::open(dir.join("file")).await.unwrap();
        assert_eq!(file.read(&mut [0; 8]).await.unwrap(), 0);
    });
}

/// A seek lands where it is asked to, from the start, the end and the current position, and the
/// next read reads from there.
#[test]
#[timeout(15000)]
fn seeks_land_where_asked() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "0123456789").unwrap();

    block_on(async {
        let mut file = File::open(dir.join("file")).await.unwrap();
        let mut two = [0; 2];

        assert_eq!(file.seek(SeekFrom::Start(3)).await.unwrap(), 3);
        file.read_exact(&mut two).await.unwrap();
        assert_eq!(&two, b"34");

        assert_eq!(file.seek(SeekFrom::Current(2)).await.unwrap(), 7);
        file.read_exact(&mut two).await.unwrap();
        assert_eq!(&two, b"78");

        assert_eq!(file.seek(SeekFrom::End(-4)).await.unwrap(), 6);
        file.read_exact(&mut two).await.unwrap();
        assert_eq!(&two, b"67");

        assert_eq!(file.seek(SeekFrom::Current(-3)).await.unwrap(), 5);
        file.read_exact(&mut two).await.unwrap();
        assert_eq!(&two, b"56");
    });
}

/// The current position is where the reads got to: a seek from it counts the bytes read.
#[test]
#[timeout(15000)]
fn the_current_position_is_where_the_reads_got_to() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "0123456789").unwrap();

    block_on(async {
        let mut file = File::open(dir.join("file")).await.unwrap();
        assert_eq!(position(&mut file).await, 0);

        file.read_exact(&mut [0; 3]).await.unwrap();

        assert_eq!(position(&mut file).await, 3);
        let mut one = [0; 1];
        file.read_exact(&mut one).await.unwrap();
        assert_eq!(&one, b"3");
    });
}

/// After a write, the position is at the end of the bytes written.
#[test]
#[timeout(15000)]
fn the_position_after_a_write_is_the_end_of_the_bytes_written() {
    let dir = TempDir::new();

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(b"hello").await.unwrap();

        assert_eq!(position(&mut file).await, 5);
    });
}

/// A write that follows a read goes where the reads got to: three bytes read, two bytes written,
/// and the two are at offsets 3 and 4.
#[test]
#[timeout(15000)]
fn a_write_after_a_read_lands_where_the_reads_got_to() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "hello world").unwrap();

    block_on(async {
        let mut file = read_write(dir.join("file")).await;
        let mut three = [0; 3];
        file.read_exact(&mut three).await.unwrap();
        assert_eq!(&three, b"hel");

        file.write_all(b"XY").await.unwrap();
        file.flush().await.unwrap();
    });

    assert_eq!(std::fs::read(dir.join("file")).unwrap(), b"helXY world");
}

/// A read after that write goes on from the end of the bytes written.
#[test]
#[timeout(15000)]
fn a_read_after_that_write_goes_on_from_the_bytes_written() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "abcdefgh").unwrap();

    block_on(async {
        let mut file = read_write(dir.join("file")).await;
        file.read_exact(&mut [0; 3]).await.unwrap();
        file.write_all(b"XY").await.unwrap();

        let mut rest = String::new();
        file.read_to_string(&mut rest).await.unwrap();

        assert_eq!(rest, "fgh");
    });
}

/// Reads and writes may alternate any number of times: each read gets the byte after the last
/// write, and each write goes after the last read.
#[test]
#[timeout(15000)]
fn reads_and_writes_alternate_without_losing_the_position() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "0123456789").unwrap();

    block_on(async {
        let mut file = read_write(dir.join("file")).await;
        let mut read = Vec::new();
        for _ in 0..5 {
            let mut one = [0; 1];
            file.read_exact(&mut one).await.unwrap();
            read.push(one[0]);
            file.write_all(b"a").await.unwrap();
        }
        file.flush().await.unwrap();

        assert_eq!(read, b"02468");
    });

    assert_eq!(std::fs::read(dir.join("file")).unwrap(), b"0a2a4a6a8a");
}

/// A read is over in the poll that begins it: a regular file is never waited on, so a read polled
/// once is done, and the write that follows goes where it got to.
#[test]
#[timeout(15000)]
fn a_read_is_over_in_the_poll_that_begins_it() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "abcdefgh").unwrap();

    block_on(async {
        let mut file = read_write(dir.join("file")).await;
        let mut buf = [0; 3];
        let read = poll_immediate(file.read(&mut buf)).await;
        assert_eq!(read.unwrap().unwrap(), 3);
        assert_eq!(&buf, b"abc");

        file.write_all(b"XY").await.unwrap();
    });

    assert_eq!(std::fs::read(dir.join("file")).unwrap(), b"abcXYfgh");
}

/// A file on a socket, which cannot be sought, is read and written in turn: each read gets the
/// bytes that the peer sent next, and each write reaches the peer.
///
/// The peer sends before each read and the file replies before the peer reads, so that no read
/// finds the socket empty and no write finds it full, which would block the thread for good.
#[cfg(unix)]
#[test]
#[timeout(15000)]
fn a_file_on_a_socket_is_read_and_written_in_turn() {
    let (mut peer, ours) = UnixStream::pair().unwrap();
    let mut file = File::from(OwnedFd::from(ours));

    block_on(async {
        for round in 0..3u8 {
            let first = [b'a' + round; 4];
            let second = [b'm' + round; 4];
            // Sent together, so that the second half waits in the socket while the file replies.
            io::Write::write_all(&mut peer, &[first, second].concat()).unwrap();
            let mut buf = [0; 4];
            file.read_exact(&mut buf).await.unwrap();
            assert_eq!(buf, first);

            file.write_all(&[b'A' + round; 4]).await.unwrap();
            file.flush().await.unwrap();
            let mut reply = [0; 4];
            io::Read::read_exact(&mut peer, &mut reply).unwrap();
            assert_eq!(reply, [b'A' + round; 4]);

            file.read_exact(&mut buf).await.unwrap();
            assert_eq!(buf, second);
        }
    });
}

/// `sync_all` after a large write leaves the file holding every byte of it.
#[test]
#[timeout(15000)]
fn sync_all_sees_every_byte_written() {
    let dir = TempDir::new();
    let data = bytes(1 << 20);

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(&data).await.unwrap();
        file.sync_all().await.unwrap();

        assert_eq!(
            std::fs::metadata(dir.join("file")).unwrap().len(),
            data.len() as u64
        );
    });
}

/// `sync_data` after a large write leaves the file holding every byte of it, as `sync_all` does.
#[test]
#[timeout(15000)]
fn sync_data_sees_every_byte_written() {
    let dir = TempDir::new();
    let data = bytes(1 << 20);

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(&data).await.unwrap();
        file.sync_data().await.unwrap();

        assert_eq!(
            std::fs::metadata(dir.join("file")).unwrap().len(),
            data.len() as u64
        );
    });
}

/// `metadata` after a large write reports a length that includes every byte of it.
#[test]
#[timeout(15000)]
fn metadata_sees_every_byte_written() {
    let dir = TempDir::new();
    let data = bytes(1 << 20);

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(&data).await.unwrap();

        assert_eq!(file.metadata().await.unwrap().len(), data.len() as u64);
    });
}

/// `set_len` after a large write cuts the file to the length it is given: what was written past
/// that is gone.
#[test]
#[timeout(15000)]
fn set_len_comes_after_every_byte_written() {
    let dir = TempDir::new();
    let data = bytes(1 << 20);

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(&data).await.unwrap();
        file.set_len(10).await.unwrap();
        file.flush().await.unwrap();

        assert_eq!(std::fs::read(dir.join("file")).unwrap(), &data[..10]);
    });
}

/// `set_len` extends a file with zeros, and leaves the position where it was: the next write goes
/// after the ones before it, and not to the end of the file.
#[test]
#[timeout(15000)]
fn set_len_extends_a_file_and_keeps_the_position() {
    let dir = TempDir::new();

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(b"ab").await.unwrap();
        file.set_len(6).await.unwrap();
        file.write_all(b"cd").await.unwrap();
        file.flush().await.unwrap();
    });

    assert_eq!(std::fs::read(dir.join("file")).unwrap(), b"abcd\0\0");
}

/// The reads after a `set_len` see the file as it is now, from the position the reads got to, so
/// that they find nothing past a cut, and the zeros of a file cut and then extended again rather
/// than the bytes it held before.
#[test]
#[timeout(15000)]
fn reads_after_set_len_see_the_file_as_it_is() {
    let dir = TempDir::new();
    std::fs::write(dir.join("cut"), "abcdef").unwrap();
    std::fs::write(dir.join("regrown"), "abcdef").unwrap();

    block_on(async {
        let mut cut = read_write(dir.join("cut")).await;
        cut.read_exact(&mut [0; 1]).await.unwrap();
        cut.set_len(1).await.unwrap();
        let mut rest = Vec::new();
        cut.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, b"");
        assert_eq!(cut.seek(SeekFrom::Current(0)).await.unwrap(), 1);

        let mut regrown = read_write(dir.join("regrown")).await;
        regrown.read_exact(&mut [0; 1]).await.unwrap();
        regrown.set_len(1).await.unwrap();
        regrown.set_len(6).await.unwrap();
        let mut rest = Vec::new();
        regrown.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, [0; 5]);
    });
}

/// What `sync_all`, `sync_data` and `metadata` do between reads leaves the reads where they were.
#[test]
#[timeout(15000)]
fn looking_at_a_file_does_not_disturb_the_reads() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "0123456789").unwrap();

    block_on(async {
        let mut file = read_write(dir.join("file")).await;
        file.read_exact(&mut [0; 3]).await.unwrap();

        file.sync_all().await.unwrap();
        assert_eq!(file.metadata().await.unwrap().len(), 10);
        file.sync_data().await.unwrap();

        let mut rest = String::new();
        file.read_to_string(&mut rest).await.unwrap();
        assert_eq!(rest, "3456789");
    });
}

/// A file dropped right after a write, with no flush, still has every byte: the write went to the
/// file when it was made.
#[test]
#[timeout(15000)]
fn a_file_dropped_after_a_write_still_has_every_byte() {
    let dir = TempDir::new();
    let data = bytes(1 << 20);

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(&data).await.unwrap();
        drop(file);
    });

    assert_eq!(std::fs::read(dir.join("file")).unwrap(), data);
}

/// Closing a file flushes it, so the file has the bytes written by the time the close is over.
#[test]
#[timeout(15000)]
fn closing_a_file_flushes_it() {
    let dir = TempDir::new();
    let data = bytes(1 << 20);

    block_on(async {
        let mut file = File::create(dir.join("file")).await.unwrap();
        file.write_all(&data).await.unwrap();
        file.close().await.unwrap();
    });

    assert_eq!(std::fs::read(dir.join("file")).unwrap(), data);
}

/// A file changes its own permissions.
#[test]
#[timeout(15000)]
fn a_file_changes_its_own_permissions() {
    let dir = TempDir::new();

    block_on(async {
        let file = File::create(dir.join("file")).await.unwrap();
        let writable = file.metadata().await.unwrap().permissions();
        let mut read_only = writable.clone();
        read_only.set_readonly(true);

        file.set_permissions(read_only).await.unwrap();
        let after = file.metadata().await.unwrap().permissions().readonly();
        // Windows does not remove a file that is read-only, so the directory could not go
        // otherwise.
        file.set_permissions(writable).await.unwrap();

        assert!(after);
    });
}

/// A file made from one of std's reads from where that was, and so does one made from its
/// descriptor or its handle.
#[test]
#[timeout(15000)]
fn a_file_made_from_one_of_stds_reads_from_where_that_was() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "0123456789").unwrap();
    let mut from_std = std::fs::File::open(dir.join("file")).unwrap();
    io::Seek::seek(&mut from_std, SeekFrom::Start(4)).unwrap();
    let from_owned = std::fs::File::open(dir.join("file")).unwrap();

    block_on(async {
        let mut text = String::new();
        File::from(from_std)
            .read_to_string(&mut text)
            .await
            .unwrap();
        assert_eq!(text, "456789");

        #[cfg(unix)]
        let mut owned = File::from(OwnedFd::from(from_owned));
        #[cfg(windows)]
        let mut owned = File::from(std::os::windows::io::OwnedHandle::from(from_owned));
        let mut text = String::new();
        owned.read_to_string(&mut text).await.unwrap();
        assert_eq!(text, "0123456789");
    });
}

/// A file gives out the descriptor, or the handle, of the file of std behind it, which is the same
/// whichever way it is asked for.
#[test]
#[timeout(15000)]
fn a_file_gives_out_its_raw_handle() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();

    let file = block_on(File::open(dir.join("file"))).unwrap();

    #[cfg(unix)]
    {
        assert_eq!(file.as_raw_fd(), file.as_fd().as_raw_fd());
        assert!(file.as_raw_fd() >= 0);
    }
    #[cfg(windows)]
    {
        assert_eq!(file.as_raw_handle(), file.as_handle().as_raw_handle());
        assert!(!file.as_raw_handle().is_null());
    }
}

/// A file prints as the file of std behind it does, and a file that is not there is not opened.
#[test]
#[timeout(15000)]
fn a_file_prints_as_a_file_of_std_does() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), "").unwrap();

    let file = block_on(File::open(dir.join("file"))).unwrap();
    let missing = block_on(File::open(dir.join("missing")));

    assert!(format!("{file:?}").starts_with("File {"));
    assert_eq!(missing.unwrap_err().kind(), io::ErrorKind::NotFound);
}

/// A file is not created in a directory that is not there.
#[test]
#[timeout(15000)]
fn create_fails_without_the_directory_of_the_file() {
    let dir = TempDir::new();

    let error = block_on(File::create(dir.join("missing").join("file"))).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[cfg(feature = "runtime")]
in_both_modes! {
    /// A task that reads a byte at a time from a file gives the next task its turn once its budget
    /// is spent, not once the last byte is read.
    ///
    /// A read from a regular file is always ready, so without the budget the reader would never
    /// return `Pending`, and the task spawned right after it would first run when the reader was
    /// done. The look is exact: the reader is stopped by the budget and by nothing else, as it has
    /// more bytes to read than that.
    fn a_task_reading_a_file_yields_to_the_next<M>() {
        let dir = TempDir::new();
        std::fs::write(dir.join("file"), bytes(READS)).unwrap();
        let mut file = File::from(std::fs::File::open(dir.join("file")).unwrap());
        let runtime = Runtime::<M>::new().unwrap();
        let (reads, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let reader = {
            let reads = reads.clone();
            runtime.spawn("a reader", async move {
                for _ in 0..READS {
                    file.read_exact(&mut [0]).await.unwrap();
                    reads.fetch_add(1, Ordering::SeqCst);
                }
            })
        };
        // Spawned right after the reader, so that it is next in the queue.
        let looker = runtime.spawn("a looker", note_progress(reads.clone(), seen.clone()));

        runtime.block_on(async {
            reader.await.unwrap();
            looker.await.unwrap();
        });

        assert_eq!(seen.load(Ordering::SeqCst), BUDGET);
        assert_eq!(reads.load(Ordering::SeqCst), READS);
    }
}

#[cfg(feature = "runtime")]
in_both_modes! {
    /// A task that calls a function of the module over and over gives the next task its turn once
    /// its budget is spent, as one that reads from a file does.
    fn a_task_looping_over_a_function_yields_to_the_next<M>() {
        let dir = TempDir::new();
        let path = dir.join("file");
        std::fs::write(&path, "").unwrap();
        let runtime = Runtime::<M>::new().unwrap();
        let (calls, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let caller = {
            let calls = calls.clone();
            runtime.spawn("a caller", async move {
                for _ in 0..READS {
                    fs::metadata(&path).await.unwrap();
                    calls.fetch_add(1, Ordering::SeqCst);
                }
            })
        };
        let looker = runtime.spawn("a looker", note_progress(calls.clone(), seen.clone()));

        runtime.block_on(async {
            caller.await.unwrap();
            looker.await.unwrap();
        });

        assert_eq!(seen.load(Ordering::SeqCst), BUDGET);
        assert_eq!(calls.load(Ordering::SeqCst), READS);
    }
}

#[cfg(feature = "runtime")]
in_both_modes! {
    /// A task that lists a directory gives the next task its turn once its budget is spent: the
    /// stream of a directory spends one for each entry it yields.
    fn a_task_listing_a_directory_yields_to_the_next<M>() {
        let dir = TempDir::new();
        for index in 0..3 * BUDGET {
            std::fs::write(dir.join(format!("file-{index}")), "").unwrap();
        }
        let runtime = Runtime::<M>::new().unwrap();
        let (listed, seen) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let lister = {
            let (listed, path) = (listed.clone(), dir.path().to_owned());
            runtime.spawn("a lister", async move {
                let mut entries = fs::read_dir(path).await.unwrap();
                while let Some(entry) = entries.next().await {
                    entry.unwrap();
                    listed.fetch_add(1, Ordering::SeqCst);
                }
            })
        };
        let looker = runtime.spawn("a looker", note_progress(listed.clone(), seen.clone()));

        runtime.block_on(async {
            lister.await.unwrap();
            looker.await.unwrap();
        });

        // The first operation of the lister opens the directory, which leaves one fewer for the
        // entries.
        assert_eq!(seen.load(Ordering::SeqCst), BUDGET - 1);
        assert_eq!(listed.load(Ordering::SeqCst), 3 * BUDGET);
    }
}

#[cfg(feature = "runtime")]
in_both_modes! {
    /// A blocking executor of another crate that a task runs gets its `fs` operations done when it
    /// runs them inside `unconstrained`.
    ///
    /// The task makes ten calls and then has `futures`'s `block_on` make the rest. That executor
    /// shares the budget of the task's poll and polls again as soon as it is woken, inside that
    /// poll, so once the budget was spent every call would turn it away for good; `unconstrained`
    /// lifts the budget for the polls of the future it runs.
    fn a_blocking_executor_that_a_task_runs_gets_its_fs_operations_done<M>() {
        let dir = TempDir::new();
        let path = dir.join("file");
        std::fs::write(&path, "").unwrap();
        let runtime = Runtime::<M>::new().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let task = {
            let calls = calls.clone();
            runtime.spawn("a task", async move {
                stat(&path, 10, &calls).await;
                futures::executor::block_on(unconstrained(stat(&path, READS - 10, &calls)));
            })
        };

        runtime.block_on(task).unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), READS);
    }
}

/// A future that is polled outside any runtime has no budget: it makes all its operations in the
/// one poll, whichever kind they are.
///
/// The poll here is made by the test, with a waker that does nothing, so that a future that
/// yielded would be left without anything to wake it.
#[cfg(feature = "runtime")]
#[test]
#[timeout(15000)]
fn a_future_polled_outside_a_runtime_is_never_held_to_the_budget() {
    let dir = TempDir::new();
    std::fs::write(dir.join("file"), bytes(3 * BUDGET)).unwrap();
    let mut file = File::from(std::fs::File::open(dir.join("file")).unwrap());
    let mut calling = pin!(async {
        for _ in 0..3 * BUDGET {
            fs::metadata(dir.path()).await.unwrap();
        }
        for _ in 0..3 * BUDGET {
            file.read_exact(&mut [0]).await.unwrap();
        }
    });

    let polled = calling
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));

    assert!(polled.is_ready());
}

/// A scratch directory under the temporary directory of the OS, which goes with everything in it
/// when the value is dropped.
struct TempDir(PathBuf);

impl TempDir {
    /// A new directory, named by the process and by how many there have been before it.
    fn new() -> Self {
        static COUNT: AtomicUsize = AtomicUsize::new(0);

        let count = COUNT.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("zruntime-fs-test-{}-{count}", process::id()));
        // A directory left by a run that crashed, with the same process id, is cleared away.
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    /// The directory.
    fn path(&self) -> &Path {
        &self.0
    }

    /// The path of `name` in the directory.
    fn join<P>(&self, name: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A file opened to be read and written, without being created or emptied.
async fn read_write<P>(path: P) -> File
where
    P: AsRef<Path>,
{
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .await
        .unwrap()
}

/// Where the reads and the writes of `file` have got to, as a seek from the current position tells.
async fn position(file: &mut File) -> u64 {
    file.seek(SeekFrom::Current(0)).await.unwrap()
}

/// Calls `fs::metadata` for `path` `calls` times, and counts each call that is over in `done`.
#[cfg(feature = "runtime")]
async fn stat(path: &Path, calls: usize, done: &AtomicUsize) {
    for _ in 0..calls {
        fs::metadata(path).await.unwrap();
        done.fetch_add(1, Ordering::SeqCst);
    }
}

/// `len` bytes that are not all alike, so that bytes in the wrong place are noticed.
fn bytes(len: usize) -> Vec<u8> {
    (0..len).map(|index| (index * 31 % 251) as u8).collect()
}

/// The permission bits of the file or directory at `path`, those of the access modes alone.
#[cfg(unix)]
fn mode_of<P>(path: P) -> u32
where
    P: AsRef<Path>,
{
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

/// The flag that has an open fail for a symbolic link, whose value differs among platforms, if it
/// is one that this knows.
#[cfg(unix)]
const O_NOFOLLOW: Option<i32> = if cfg!(all(
    any(target_os = "linux", target_os = "android"),
    any(target_arch = "x86", target_arch = "x86_64")
)) {
    Some(0o400000)
} else if cfg!(all(
    any(target_os = "linux", target_os = "android"),
    any(target_arch = "arm", target_arch = "aarch64")
)) {
    Some(0o100000)
} else if cfg!(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)) {
    Some(0x100)
} else {
    None
};
