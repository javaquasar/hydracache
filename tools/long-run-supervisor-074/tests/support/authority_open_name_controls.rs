//! Original syscall helper versus the actual helper; no alternate authority path.
use super::*;

pub(crate) fn reference(
    parent: &File,
    name: &str,
    directory: bool,
) -> Result<File, AuthorityError> {
    let name = CString::new(name).map_err(|_| AuthorityError::Invalid)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    // SAFETY: live parent descriptor and NUL-terminated bounded/fixed name, read-only.
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful openat returns a new owned descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn compare(parent: &File, name: &str, directory: bool) -> Option<i32> {
    // Both owned descriptors remain alive through identity and flags comparisons.
    match (
        reference(parent, name, directory),
        open_at(parent, name, directory),
    ) {
        (Ok(left), Ok(right)) => {
            assert_ne!(left.as_raw_fd(), right.as_raw_fd());
            assert!(Stamp::of(&left.metadata().unwrap()) == Stamp::of(&right.metadata().unwrap()));
            for query in [libc::F_GETFL, libc::F_GETFD] {
                // SAFETY: scalar, non-mutating fcntl queries of two live descriptors.
                let a = unsafe { libc::fcntl(left.as_raw_fd(), query) };
                let b = unsafe { libc::fcntl(right.as_raw_fd(), query) };
                assert!(a >= 0 && b >= 0);
                assert_eq!(a, b);
                if query == libc::F_GETFD {
                    assert_ne!(a & libc::FD_CLOEXEC, 0);
                } else {
                    assert_ne!(a & libc::O_NONBLOCK, 0);
                    assert_eq!(a & libc::O_ACCMODE, libc::O_RDONLY);
                    assert_ne!(a & libc::O_NOFOLLOW, 0);
                    assert_eq!(a & libc::O_DIRECTORY != 0, directory);
                }
            }
            None
        }
        (Err(AuthorityError::Invalid), Err(AuthorityError::Invalid)) => Some(0),
        (Err(AuthorityError::Io(left)), Err(AuthorityError::Io(right))) => {
            assert_eq!(left.raw_os_error(), right.raw_os_error());
            Some(left.raw_os_error().expect("kernel errno"))
        }
        _ => panic!("open helper semantics differ"),
    }
}

fn root() -> tempfile::TempDir {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    tempfile::Builder::new()
        .prefix("hydracache-open-name-")
        .tempdir_in("/tmp")
        .unwrap()
}

#[test]
fn authority_open_name_differential_boundaries_utf8_seeded_and_nul() {
    let root = root();
    let parent = File::open(root.path()).unwrap();
    let mut names = vec![
        "a".to_owned(),
        "é".repeat(11),
        format!("{}a", "é".repeat(11)),
        "é".repeat(12),
    ];
    for length in [21, 22, 23, 24, 25, 254, 255] {
        names.push("x".repeat(length));
    }
    let mut seed = 0x0740_0a23_u64;
    for index in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        names.push(format!("{index:03}-{seed:016x}"));
    }
    for name in names {
        fs::write(root.path().join(&name), b"same payload").unwrap();
        assert_eq!(compare(&parent, &name, false), None);
        assert_eq!(compare(&parent, &name, true), Some(libc::ENOTDIR));
    }
    assert_eq!(compare(&parent, "", false), Some(libc::ENOENT));
    assert_eq!(compare(&parent, "", true), Some(libc::ENOENT));
    assert_eq!(compare(&parent, "missing", false), Some(libc::ENOENT));
    assert_eq!(
        compare(&parent, &"x".repeat(256), false),
        Some(libc::ENAMETOOLONG)
    );
    // Validate every position, including terminators, in both encoding branches.
    for length in [0, 1, 22, 23, 24, 255, 256] {
        for position in 0..=length {
            let mut name = "a".repeat(length);
            name.insert(position, '\0');
            for directory in [false, true] {
                assert_eq!(compare(&parent, &name, directory), Some(0));
            }
        }
    }
    eprintln!("open-name seed=0x07400a23 seeded=128 utf8-byte-boundaries=23/24");
}

#[test]
fn authority_open_name_differential_symlink_fifo_and_retained_parent() {
    let root = root();
    let path = root.path().join("parent");
    fs::create_dir(&path).unwrap();
    let parent = File::open(&path).unwrap();
    fs::write(path.join("file"), b"first").unwrap();
    fs::create_dir(path.join("directory")).unwrap();
    std::os::unix::fs::symlink("file", path.join("link")).unwrap();
    std::os::unix::fs::symlink("directory", path.join("dir-link")).unwrap();
    let fifo = CString::new(path.join("fifo").as_os_str().as_encoded_bytes()).unwrap();
    // SAFETY: a private NUL-terminated pathname and a scalar mode.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    assert_eq!(compare(&parent, "fifo", false), None);
    assert_eq!(compare(&parent, "directory", true), None);
    assert_eq!(compare(&parent, "directory", false), None);
    assert_eq!(compare(&parent, "link", false), Some(libc::ELOOP));
    assert_eq!(compare(&parent, "dir-link", true), Some(libc::ENOTDIR));
    assert_eq!(compare(&parent, "fifo", true), Some(libc::ENOTDIR));
    // Path replacement must not redirect a retained parent descriptor.
    let moved = root.path().join("moved");
    fs::rename(&path, &moved).unwrap();
    fs::create_dir(&path).unwrap();
    fs::write(path.join("file"), b"other").unwrap();
    assert_eq!(compare(&parent, "file", false), None);
    let mut file = open_at(&parent, "file", false).unwrap();
    let mut raw = Vec::new();
    file.read_to_end(&mut raw).unwrap();
    assert_eq!(raw, b"first");
    fs::remove_file(moved.join("file")).unwrap();
    assert_eq!(compare(&parent, "file", false), Some(libc::ENOENT));
}

#[test]
fn authority_short_directory_differential_valid_utf8_boundaries_and_fixed_components() {
    let root = root();
    let parent = File::open(root.path()).unwrap();
    let mut names: Vec<String> = [
        "var",
        "lib",
        "hydracache-performance",
        "worker-authority-074",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    names.extend([
        "a".repeat(23),
        "a".repeat(24),
        "a".repeat(25),
        "a".repeat(255),
        format!("{}a", "é".repeat(11)),
        "é".repeat(12),
    ]);
    let mut seed = 0x074d_1a24_u64;
    for index in 0..64 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        names.push(format!("d-{index:02}-{seed:016x}"));
    }
    for name in names {
        fs::create_dir(root.path().join(&name)).unwrap();
        assert_eq!(compare(&parent, &name, true), None);
        assert_eq!(compare(&parent, &name, false), None);
    }
    // These are children of the private /tmp fixture, not real fixed host paths.
    eprintln!("short-directory seed=0x074d1a24 seeded=64 bytes=23/24 fixed-components=4");
}

#[cfg(feature = "fixed-leaf-alloc-probe")]
pub(crate) struct Probe {
    root: tempfile::TempDir,
    parent: File,
    name: String,
    directory: bool,
    held: Option<Leaf>,
}
#[cfg(feature = "fixed-leaf-alloc-probe")]
impl Probe {
    pub(crate) fn new(case: usize) -> Self {
        let root = root();
        let name = match case {
            0 => name(1, "json"),
            1 => name(1, "commit"),
            2 => "x".repeat(24),
            3 => format!("{}a", "é".repeat(11)),
            4 => "x".repeat(255),
            5 => "sub".to_owned(),
            _ => panic!("unknown helper case"),
        };
        let directory = case == 5;
        if directory {
            fs::create_dir(root.path().join(&name)).unwrap();
        } else {
            fs::write(root.path().join(&name), [7_u8; 65]).unwrap();
            fs::set_permissions(root.path().join(&name), fs::Permissions::from_mode(0o400))
                .unwrap();
        }
        let parent = File::open(root.path()).unwrap();
        assert_eq!(compare(&parent, &name, directory), None);
        let held = if directory {
            None
        } else {
            Some(
                Leaf::open(
                    &parent,
                    name.clone(),
                    unsafe { libc::geteuid() },
                    unsafe { libc::getegid() },
                    MAX_RECORD_BYTES,
                )
                .unwrap(),
            )
        };
        Self {
            root,
            parent,
            name,
            directory,
            held,
        }
    }
    pub(crate) fn identity(&self) -> serde_json::Value {
        serde_json::json!({
            "name":self.name,"name_bytes":self.name.len(),"name_sha256":sha256_hex(self.name.as_bytes()),
            "directory":self.directory,"payload_bytes":if self.directory {0}else{65},
            "payload_sha256":if self.directory {None}else{Some(sha256_hex(&[7_u8;65]))},
            "temporary":self.root.path().starts_with("/tmp"),
        })
    }
    pub(crate) fn run(&self, reference_arm: bool) {
        let file = if reference_arm {
            reference(
                &self.parent,
                std::hint::black_box(&self.name),
                self.directory,
            )
        } else {
            open_at(
                &self.parent,
                std::hint::black_box(&self.name),
                self.directory,
            )
        }
        .unwrap();
        drop(std::hint::black_box(file));
    }
    pub(crate) fn leaf(&self, cold: bool) {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        if cold {
            // Include name ownership in cold open; it is not hidden in setup.
            drop(std::hint::black_box(
                Leaf::open(&self.parent, self.name.clone(), uid, gid, MAX_RECORD_BYTES).unwrap(),
            ));
        } else {
            self.held
                .as_ref()
                .unwrap()
                .revalidate(&self.parent, uid, gid, MAX_RECORD_BYTES)
                .unwrap();
        }
    }
}
