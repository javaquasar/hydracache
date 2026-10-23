use super::*;
use crate::diagnostic_artifacts::linux::held_role_census::bootstrap_images::entry::installed::entry_material::tests::Fixture;
fn inputs() -> (Fixture, EntryResource) {
    let mut f = Fixture::new();
    let file = f.original.take().unwrap();
    let original = EntryResource {
        retained: stamp(file.as_raw_fd()).unwrap(),
        file,
    };
    (f, original)
}
fn create(f: &Fixture, original: &EntryResource) -> InstalledEntryResource {
    InstalledEntryResource::create(original, &f.base, &f.prior, &f.installed).unwrap()
}
fn inspect(
    r: &mut InstalledEntryResource,
    f: &Fixture,
    original: &EntryResource,
) -> io::Result<InstalledEntryBindings> {
    r.inspect(original, &f.base, &f.prior, &f.installed)
}
#[test]
fn bootstrap_installed_material_resource_exact_readonly_sealed_repeat_and_offsets() {
    let (f, original) = inputs();
    let mut r = create(&f, &original);
    let first = inspect(&mut r, &f, &original).unwrap();
    assert!(first == inspect(&mut r, &f, &original).unwrap());
    let raw = expected(&original, &f.base, &f.prior, &f.installed).unwrap();
    assert_eq!(read(&r.file, 1280).unwrap(), raw);
    assert_eq!(&raw[16..1040], f.raw);
    assert_eq!(r.retained.position, Some(0));
    assert_eq!(stamp(original.file.as_raw_fd()).unwrap(), original.retained);
    assert_eq!(
        unsafe { libc::pwrite(r.file.as_raw_fd(), raw.as_ptr().cast(), 1, 0) },
        -1
    );
    assert_eq!(unsafe { libc::ftruncate(r.file.as_raw_fd(), 1) }, -1);
    readonly(&r.file).unwrap();
    assert!(inspect(&mut r, &f, &original).is_ok());
}
#[test]
fn bootstrap_installed_material_resource_byte_mismatch_is_sticky() {
    let (f, original) = inputs();
    let raw = expected(&original, &f.base, &f.prior, &f.installed).unwrap();
    for at in [0, 8, 16, 128, 1040, 1064, 1128, 1279] {
        let mut r = create(&f, &original);
        let mut bad = raw;
        bad[at] ^= 1;
        let old = std::mem::replace(&mut r.file, sealed(&bad).unwrap());
        // Private fault injection matches the replacement stamp to reach content checks.
        let retained = std::mem::replace(&mut r.retained, stamp(r.file.as_raw_fd()).unwrap());
        assert!(inspect(&mut r, &f, &original).is_err());
        r.file = old;
        r.retained = retained;
        assert!(inspect(&mut r, &f, &original).is_err());
    }
}
#[test]
fn bootstrap_installed_material_resource_same_bytes_replacement_and_offset_refuse_after_restore() {
    let (f, original) = inputs();
    let raw = expected(&original, &f.base, &f.prior, &f.installed).unwrap();
    let mut r = create(&f, &original);
    let old = std::mem::replace(&mut r.file, sealed(&raw).unwrap());
    assert!(inspect(&mut r, &f, &original).is_err());
    r.file = old;
    assert!(inspect(&mut r, &f, &original).is_err());
    let mut r = create(&f, &original);
    assert_eq!(
        unsafe { libc::lseek(r.file.as_raw_fd(), 1, libc::SEEK_SET) },
        1
    );
    assert!(inspect(&mut r, &f, &original).is_err());
    assert_eq!(
        unsafe { libc::lseek(r.file.as_raw_fd(), 0, libc::SEEK_SET) },
        0
    );
    assert!(inspect(&mut r, &f, &original).is_err());
}
#[test]
fn bootstrap_installed_material_resource_original_substitution_malformed_and_wrong_prefix_refuse() {
    let (f, mut original) = inputs();
    let mut r = create(&f, &original);
    let old = std::mem::replace(&mut original.file, sealed(&f.raw).unwrap());
    assert!(inspect(&mut r, &f, &original).is_err());
    original.file = old;
    assert!(inspect(&mut r, &f, &original).is_err());
    let invalid = EntryResource::create(&[74; 1024]).unwrap();
    let wrong = f.base.append_entry(&invalid.file).unwrap();
    assert!(InstalledEntryResource::create(&invalid, &f.base, &wrong, &f.installed).is_err());
    assert!(InstalledEntryResource::create(&original, &f.base, &wrong, &f.installed).is_err());
    assert_eq!(
        unsafe { libc::lseek(original.file.as_raw_fd(), 1, libc::SEEK_SET) },
        1
    );
    assert!(InstalledEntryResource::create(&original, &f.base, &f.prior, &f.installed).is_err());
}
#[test]
fn bootstrap_installed_material_resource_flags_and_metadata_drift_latch() {
    let (f, original) = inputs();
    for descriptor in [true, false] {
        let mut r = create(&f, &original);
        let fd = r.file.as_raw_fd();
        let get = if descriptor {
            libc::F_GETFD
        } else {
            libc::F_GETFL
        };
        let set = if descriptor {
            libc::F_SETFD
        } else {
            libc::F_SETFL
        };
        let before = unsafe { libc::fcntl(fd, get) };
        let changed = if descriptor {
            0
        } else {
            before ^ libc::O_NONBLOCK
        };
        assert_eq!(unsafe { libc::fcntl(fd, set, changed) }, 0);
        assert!(inspect(&mut r, &f, &original).is_err());
        assert_eq!(unsafe { libc::fcntl(fd, set, before) }, 0);
        assert!(inspect(&mut r, &f, &original).is_err());
    }
    let mut r = create(&f, &original);
    assert_eq!(unsafe { libc::fchmod(r.file.as_raw_fd(), 0o500) }, 0);
    assert!(inspect(&mut r, &f, &original).is_err());
}
