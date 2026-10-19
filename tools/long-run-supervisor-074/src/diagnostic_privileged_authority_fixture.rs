//! Actual root-owned journal engine under a private test origin, not enrollment.
use super::*;
pub(crate) fn initialize(path: &Path, policy: &CheckedWorkerPolicy) -> FixedAuthorityRegistry {
    crate::diagnostic_two_phase_launch::root_enrollment::privileged_fixture::require(path);
    let mut leaf = path.to_owned();
    for name in ANCESTORS.into_iter().chain([FIXED_LEAF]) {
        leaf.push(name);
        fs::create_dir(&leaf).unwrap();
        fs::set_permissions(&leaf, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let root = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .unwrap();
    FixedAuthorityRegistry {
        journal: initialize_directory(walk(root, 0, 0, path).unwrap(), policy).unwrap(),
    }
}
