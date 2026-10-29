//! Mount only a uniquely owned local cgroup subtree; never host system.slice.
use super::*;
pub(super) struct Private {
    _nonce: tempfile::TempDir,
    original: PathBuf,
}
impl Private {
    pub(super) fn new() -> Self {
        let nonce = crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
        let original = Path::new("/sys/fs/cgroup").join(nonce.path().file_name().unwrap());
        assert_eq!(original.parent(), Some(Path::new("/sys/fs/cgroup")));
        fs::create_dir(&original).unwrap();
        fs::create_dir(original.join("system.slice")).unwrap();
        assert_eq!(unsafe { libc::unshare(libc::CLONE_NEWNS) }, 0);
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
        let source = CString::new(original.as_os_str().as_encoded_bytes()).unwrap();
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
        Self {
            _nonce: nonce,
            original,
        }
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        assert_eq!(
            unsafe { libc::umount2(c"/sys/fs/cgroup".as_ptr(), libc::MNT_DETACH) },
            0
        );
        // Only exact empty fixture directories, no recursive removal or cgroup.kill.
        fs::remove_dir(self.original.join("system.slice")).unwrap();
        fs::remove_dir(&self.original).unwrap();
    }
}
pub(super) fn attempt(attempt: [u8; 32]) -> Cgroup {
    let temporary = crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::new();
    let name = CString::new(format!("system-hc074{}.slice", crate::hex(&attempt))).unwrap();
    let parent = File::open("/sys/fs/cgroup/system.slice").unwrap();
    let path = Path::new("/sys/fs/cgroup/system.slice").join(name.to_str().unwrap());
    fs::create_dir(&path).unwrap();
    fs::create_dir(path.join("nested")).unwrap();
    Cgroup {
        _temporary: temporary,
        name,
        parent,
        path,
        writers: vec![],
        guardians: None,
    }
}
