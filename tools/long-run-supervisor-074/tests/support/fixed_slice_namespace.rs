//! Manual kernel observer fixture, not original admission or a systemd slice.
use super::*;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const PREFIX: &str = "hc074-fixed-slice-";
const TEST: &str = "diagnostic_two_phase_launch::root_enrollment::kernel::fixed_slice::tests::namespace::privileged_fixed_slice_namespace_matrix";
const MOUNT: &str = "/sys/fs/cgroup";
const CASES: [&str; 9] = [
    "healthy",
    "different-attempt",
    "named-root-replaced",
    "system-parent-replaced",
    "root-mode-changed",
    "parent-mode-changed",
    "mount-anchor-replaced",
    "original-refused",
    "same-name-foreign-parent",
];

fn attempt(path: &Path) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(path.file_name().unwrap().as_bytes()).into()
}
fn checked_path(path: &Path) {
    assert_eq!(path.parent(), Some(Path::new(MOUNT)));
    let basename = path.file_name().unwrap().as_bytes();
    assert!(basename.starts_with(PREFIX.as_bytes()) && basename.len() < 128);
    assert!(basename
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-'));
    assert_eq!(std::fs::canonicalize(path).unwrap(), path);
    let file = File::open(path).unwrap();
    stamp(&file).unwrap();
    assert!(file.metadata().unwrap().is_dir());
}
struct Fixture {
    path: PathBuf,
    // Keep the unique ordinary-filesystem nonce reserved until cgroup cleanup.
    _nonce: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let nonce = tempfile::Builder::new().prefix(PREFIX).tempdir().unwrap();
        let path = Path::new(MOUNT).join(nonce.path().file_name().unwrap());
        std::fs::create_dir(&path).unwrap();
        let result = Self {
            path,
            _nonce: nonce,
        };
        checked_path(&result.path);
        let name = name(attempt(&result.path));
        for parent in ["system.slice", "other.slice"] {
            std::fs::create_dir(result.path.join(parent)).unwrap();
            std::fs::create_dir(result.path.join(parent).join(name.to_str().unwrap())).unwrap();
        }
        result
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        checked_path(&self.path);
        let name = name(attempt(&self.path)).into_string().unwrap();
        // Never recursively delete. Only these exact, empty, owned fixture dirs.
        for parent in ["system.slice", "system.old", "other.slice"] {
            let directory = self.path.join(parent);
            if !directory.exists() {
                continue;
            }
            for child in [&name, &format!("{name}.old")] {
                let path = directory.join(child);
                match std::fs::symlink_metadata(&path) {
                    Ok(metadata) => {
                        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
                        assert_eq!((metadata.uid(), metadata.gid()), (0, 0));
                        std::fs::remove_dir(path).unwrap();
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => panic!("fixture cleanup refused: {error}"),
                }
            }
            std::fs::remove_dir(directory).unwrap();
        }
        std::fs::remove_dir(&self.path).unwrap();
        assert!(!self.path.exists());
    }
}
fn private_mount(source: &Path) {
    let original = std::fs::read_link("/proc/thread-self/ns/mnt").unwrap();
    assert_eq!(unsafe { libc::unshare(libc::CLONE_NEWNS) }, 0);
    assert_ne!(
        std::fs::read_link("/proc/thread-self/ns/mnt").unwrap(),
        original
    );
    assert_eq!(
        unsafe {
            libc::mount(
                std::ptr::null(),
                c"/".as_ptr(),
                std::ptr::null(),
                libc::MS_REC | libc::MS_PRIVATE,
                std::ptr::null(),
            )
        },
        0
    );
    bind_mount(source);
}
fn bind_mount(source: &Path) {
    let source = CString::new(source.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe {
            libc::mount(
                source.as_ptr(),
                c"/sys/fs/cgroup".as_ptr(),
                std::ptr::null(),
                libc::MS_BIND,
                std::ptr::null(),
            )
        },
        0
    );
}
fn observer_readonly(slice: &FixedSlice) {
    for file in slice.anchors.iter().chain([&slice.system]) {
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0);
        assert_eq!(flags & libc::O_ACCMODE, libc::O_RDONLY);
        let descriptor_flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
        assert!(descriptor_flags >= 0 && descriptor_flags & libc::FD_CLOEXEC != 0);
    }
}
fn child_case(path: &Path, case: usize) {
    checked_path(path);
    let attempt = attempt(path);
    private_mount(path);
    let expected = name(attempt);
    let system = File::open(format!("{MOUNT}/system.slice")).unwrap();
    let mut root = PinnedRoot::pin_empty(system.as_fd(), &expected).unwrap();
    assert_eq!(
        root.observe().unwrap(),
        Observation {
            populated: false,
            frozen: false
        }
    );
    let slice = FixedSlice::pin(&root, attempt).unwrap();
    slice.revalidate(&root, attempt).unwrap();
    observer_readonly(&slice);
    let named = Path::new(MOUNT)
        .join("system.slice")
        .join(expected.to_str().unwrap());
    match case {
        0 => slice.revalidate(&root, attempt).unwrap(),
        1 => {
            let mut different = attempt;
            different[0] ^= 1;
            assert!(slice.revalidate(&root, different).is_err());
            slice.revalidate(&root, attempt).unwrap();
        }
        2 => {
            // cgroup2 forbids rename. Remove only this empty fixture cgroup;
            // the original descriptors stay retained across name recreation.
            std::fs::remove_dir(&named).unwrap();
            std::fs::create_dir(&named).unwrap();
            assert_ne!(stamp(&File::open(&named).unwrap()).unwrap(), root.identity);
            assert!(slice.revalidate(&root, attempt).is_err());
        }
        3 => {
            std::fs::remove_dir(&named).unwrap();
            std::fs::remove_dir(format!("{MOUNT}/system.slice")).unwrap();
            std::fs::create_dir(format!("{MOUNT}/system.slice")).unwrap();
            std::fs::create_dir(&named).unwrap();
            assert_ne!(
                stamp(&File::open(format!("{MOUNT}/system.slice")).unwrap()).unwrap(),
                root.parent_identity
            );
            assert!(slice.revalidate(&root, attempt).is_err());
        }
        4 => {
            std::fs::set_permissions(&named, std::fs::Permissions::from_mode(0o775)).unwrap();
            assert!(slice.revalidate(&root, attempt).is_err());
        }
        5 => {
            std::fs::set_permissions(
                format!("{MOUNT}/system.slice"),
                std::fs::Permissions::from_mode(0o775),
            )
            .unwrap();
            assert!(slice.revalidate(&root, attempt).is_err());
        }
        6 => {
            bind_mount(Path::new(&format!("{MOUNT}/other.slice")));
            assert!(slice.revalidate(&root, attempt).is_err());
        }
        7 => {
            root.refused = true;
            assert!(slice.revalidate(&root, attempt).is_err());
            assert!(FixedSlice::pin(&root, attempt).is_err());
        }
        8 => {
            let parent = File::open(format!("{MOUNT}/other.slice")).unwrap();
            let foreign = PinnedRoot::pin_empty(parent.as_fd(), &expected).unwrap();
            assert!(FixedSlice::pin(&foreign, attempt).is_err());
            assert!(slice.revalidate(&foreign, attempt).is_err());
            slice.revalidate(&root, attempt).unwrap();
        }
        _ => unreachable!(),
    }
    println!("HC074-FIXED-NAMESPACE case={} positive_pin=true checked=true observer_readonly=true original_join=false manager_ownership=false send=false", CASES[case]);
}
#[test]
#[ignore = "manual root local private-mount cgroup2 fixture; no systemd or original admission"]
fn privileged_fixed_slice_namespace_matrix() {
    assert_eq!(unsafe { libc::geteuid() }, 0, "local root fixture only");
    assert_eq!(
        std::env::var("HC074_FIXED_SLICE_NAMESPACE").as_deref(),
        Ok("1")
    );
    if let Ok(case) = std::env::var("HC074_FIXED_SLICE_NAMESPACE_CHILD") {
        let case: usize = case.parse().unwrap();
        assert!(case < CASES.len());
        child_case(
            Path::new(&std::env::var("HC074_FIXED_SLICE_NAMESPACE_PATH").unwrap()),
            case,
        );
        return;
    }
    let original_namespace = std::fs::read_link("/proc/thread-self/ns/mnt").unwrap();
    let original_anchors = identities(&anchors().unwrap()).unwrap();
    let original_system = stamp(&File::open(format!("{MOUNT}/system.slice")).unwrap()).unwrap();
    for (case, label) in CASES.iter().enumerate() {
        let fixture = Fixture::new();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                TEST,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("HC074_FIXED_SLICE_NAMESPACE_CHILD", case.to_string())
            .env("HC074_FIXED_SLICE_NAMESPACE_PATH", &fixture.path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let bound = Instant::now() + Duration::from_secs(20); // harness only
        let mut timed_out = false;
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= bound {
                timed_out = true;
                child.kill().unwrap(); // still unreaped, exactly owned child
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let output = child.wait_with_output().unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        print!("{stdout}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        // Drop removes exact empty fixture directories even if a child refused.
        drop(fixture);
        assert!(!timed_out && output.status.success(), "{label}: {stderr}");
        assert!(stdout.contains(&format!(
            "HC074-FIXED-NAMESPACE case={label} positive_pin=true checked=true"
        )));
        assert_eq!(
            std::fs::read_link("/proc/thread-self/ns/mnt").unwrap(),
            original_namespace
        );
        assert_eq!(identities(&anchors().unwrap()).unwrap(), original_anchors);
        assert_eq!(
            stamp(&File::open(format!("{MOUNT}/system.slice")).unwrap()).unwrap(),
            original_system
        );
    }
    println!("HC074-FIXED-NAMESPACE cases=9 host_anchors_unchanged=true cleanup=true original_join=false server=false supervisor=false product=false");
}
