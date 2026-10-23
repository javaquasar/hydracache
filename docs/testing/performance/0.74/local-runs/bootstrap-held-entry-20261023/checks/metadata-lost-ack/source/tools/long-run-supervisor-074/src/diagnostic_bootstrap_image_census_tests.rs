use super::*;
use crate::diagnostic_builder::bootstrap_build::tests::sealed_image;
const ALL: i32 = libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
pub(crate) struct Fixture {
    pub(crate) old: Vec<File>,
    pub(crate) images: [File; 3],
}
impl Fixture {
    pub(crate) fn new() -> Self {
        Self {
            old: (0..23).map(|_| tempfile::tempfile().unwrap()).collect(),
            images: std::array::from_fn(|i| sealed_image(&[i as u8 + 1; 128], ALL)),
        }
    }
    pub(crate) fn original(&self) -> OriginalFdSet {
        OriginalFdSet::from_material(std::array::from_fn(|i| self.old[i].as_raw_fd())).unwrap()
    }
    pub(crate) fn prior(&self) -> CompleteBindings {
        let old: [i32; 23] = std::array::from_fn(|i| self.old[i].as_raw_fd());
        let mut roles = [-1; 11];
        for (slot, at) in [
            (0, 14),
            (1, 6),
            (2, 7),
            (3, 16),
            (4, 20),
            (5, 17),
            (6, 19),
            (7, 21),
            (8, 22),
            (9, 1),
            (10, 18),
        ] {
            roles[slot] = old[at];
        }
        CompleteBindings {
            roles,
            extra: std::array::from_fn(|i| stamp(&self.old[20 + i]).unwrap()),
        }
    }
    pub(crate) fn identities(&self) -> [[u64; 3]; 3] {
        self.images.each_ref().map(|f| {
            let m = f.metadata().unwrap();
            [m.dev(), m.ino(), m.len()]
        })
    }
    fn collect(&self) -> BootstrapImageBindings {
        collect(
            &self.prior(),
            self.original(),
            self.images.each_ref(),
            self.identities(),
        )
        .unwrap()
    }
}
#[test]
fn bootstrap_image_census_closed_mapping_keeps_old_prefix_and_all_offsets() {
    let f = Fixture::new();
    let first = f.collect();
    let second = f.collect();
    assert!(first == second);
    assert_eq!(first.roles[..11], f.prior().roles);
    assert_eq!(
        first.roles[11..],
        f.images.each_ref().map(|f| f.as_raw_fd())
    );
    assert_eq!(first.fds.len(), 26);
    assert!(first.images.iter().all(|s| s.offset == Some(0)));
}
#[test]
fn bootstrap_image_census_all_descriptor_collisions_and_original_role_mutations_refuse() {
    for i in 0..26 {
        for j in 0..i {
            let mut values = std::array::from_fn::<_, 26, _>(|n| n as i32 + 74);
            values[i] = values[j];
            assert!(!distinct(&values));
        }
    }
    assert!(!distinct(&[-1]));
    let f = Fixture::new();
    for at in 0..11 {
        let mut prior = f.prior();
        prior.roles[at] = -1;
        assert!(collect(&prior, f.original(), f.images.each_ref(), f.identities()).is_err());
    }
    for at in 0..3 {
        let mut prior = f.prior();
        prior.extra[at].fd = f.old[0].as_raw_fd();
        assert!(collect(&prior, f.original(), f.images.each_ref(), f.identities()).is_err());
    }
    let mut original = f.original();
    original.0[0] = f.images[0].as_raw_fd();
    assert!(collect(&f.prior(), original, f.images.each_ref(), f.identities()).is_err());
}
#[test]
fn bootstrap_image_census_swapped_wrong_zero_and_over_budget_identity_refuses() {
    let f = Fixture::new();
    for at in 0..3 {
        for word in 0..3 {
            let mut expected = f.identities();
            expected[at][word] += 1;
            assert!(collect(&f.prior(), f.original(), f.images.each_ref(), expected).is_err());
            expected = f.identities();
            expected[at][word] = 0;
            assert!(collect(&f.prior(), f.original(), f.images.each_ref(), expected).is_err());
        }
    }
    let mut expected = f.identities();
    expected[2][2] = crate::diagnostic_artifacts::MAX_BINARY_BYTES + 1;
    assert!(collect(&f.prior(), f.original(), f.images.each_ref(), expected).is_err());
    let mut expected = f.identities();
    expected.swap(0, 1);
    assert!(collect(&f.prior(), f.original(), f.images.each_ref(), expected).is_err());
}
#[test]
fn bootstrap_image_census_each_missing_or_extra_seal_and_bad_access_refuses() {
    let f = Fixture::new();
    for seals in [
        0,
        ALL & !libc::F_SEAL_SEAL,
        ALL & !libc::F_SEAL_GROW,
        ALL & !libc::F_SEAL_SHRINK,
        ALL & !libc::F_SEAL_WRITE,
        ALL | libc::F_SEAL_FUTURE_WRITE,
    ] {
        let image = sealed_image(&[7; 128], seals);
        let m = image.metadata().unwrap();
        let mut expected = f.identities();
        expected[2] = [m.dev(), m.ino(), m.len()];
        assert!(collect(
            &f.prior(),
            f.original(),
            [&f.images[0], &f.images[1], &image],
            expected
        )
        .is_err());
    }
    let file = &f.images[2];
    let fd = file.as_raw_fd();
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0);
    assert!(collect(
        &f.prior(),
        f.original(),
        f.images.each_ref(),
        f.identities()
    )
    .is_err());
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    for flags in [libc::O_RDWR, libc::O_PATH] {
        use std::os::{fd::FromRawFd, unix::fs::PermissionsExt};
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .unwrap();
        let path = std::ffi::CString::new(format!("/proc/self/fd/{fd}")).unwrap();
        let bad = unsafe { libc::open(path.as_ptr(), flags | libc::O_CLOEXEC) };
        assert!(bad >= 0);
        let bad = unsafe { File::from_raw_fd(bad) };
        assert!(image(&bad, f.identities()[2]).is_err());
    }
}
#[test]
fn bootstrap_image_census_same_inode_alias_and_reused_descriptor_refuse() {
    let f = Fixture::new();
    let alias = f.images[0].try_clone().unwrap();
    let mut expected = f.identities();
    expected[1] = expected[0];
    assert!(collect(
        &f.prior(),
        f.original(),
        [&f.images[0], &alias, &f.images[2]],
        expected
    )
    .is_err());
    assert!(collect(
        &f.prior(),
        f.original(),
        [&f.images[0], &f.images[0], &f.images[2]],
        expected
    )
    .is_err());
    let mut prior = f.prior();
    prior.extra[0].stat[..2].copy_from_slice(&expected[0][..2]);
    assert!(collect(&prior, f.original(), f.images.each_ref(), f.identities()).is_err());
}
#[test]
fn bootstrap_image_census_offset_mode_and_same_bytes_inode_drift_change_observation() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let before = f.collect();
    assert_eq!(
        unsafe { libc::lseek(f.images[2].as_raw_fd(), 17, libc::SEEK_SET) },
        17
    );
    assert!(f.collect() != before);
    assert_eq!(
        unsafe { libc::lseek(f.images[2].as_raw_fd(), 0, libc::SEEK_SET) },
        0
    );
    let next = sealed_image(&[3; 128], ALL);
    let m = next.metadata().unwrap();
    let mut expected = f.identities();
    expected[2] = [m.dev(), m.ino(), m.len()];
    assert!(
        collect(
            &f.prior(),
            f.original(),
            [&f.images[0], &f.images[1], &next],
            expected
        )
        .unwrap()
            != before
    );
    f.images[2]
        .set_permissions(std::fs::Permissions::from_mode(0o400))
        .unwrap();
    assert!(f.collect() != before);
}
