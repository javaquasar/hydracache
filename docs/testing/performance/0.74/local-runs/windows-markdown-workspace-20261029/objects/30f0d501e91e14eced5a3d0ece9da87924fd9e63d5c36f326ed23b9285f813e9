use super::super::super::super::tests::Fixture as Images;
use super::*;
use crate::diagnostic_builder::bootstrap_build::tests::sealed_image;
const ALL: i32 = libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
pub(crate) struct Fixture {
    _images: Images,
    _directories: Vec<tempfile::TempDir>,
    _handles: Vec<File>,
    _installed_path: tempfile::NamedTempFile,
    _installed_file: File,
    pub(crate) original: Option<File>,
    pub(crate) raw: [u8; 1024],
    pub(crate) base: BootstrapImageBindings,
    pub(crate) prior: EntryBindings,
    pub(crate) installed: InstalledBindings,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let images = Images::new();
        let base = collect(
            &images.prior(),
            images.original(),
            images.images.each_ref(),
            images.identities(),
        )
        .unwrap();
        let ids = images.identities();
        let raw = AssertedEntry {
            owner: [74, 75, 0, 0, 0, 0],
            origins: [1, 2, 3, 4],
            bounds: [300, 60, 10, 10],
            attempt: [1; 32],
            original_context: [2; 32],
            root: [1, 2, 10],
            serial: 1,
            names: [[3; 32]; 2],
            documents: [[4; 32]; 7],
            identities: [
                ids[0],
                ids[1],
                ids[2],
                [u64::MAX, 1, 128],
                [u64::MAX, 2, 128],
                [u64::MAX, 3, 256],
            ],
            streams: [[1, 2]; 4],
            guardians: [[76, 1, 3], [77, 1, 4]],
            image_hashes: [[5; 32]; 5],
            legacy_hash: [6; 32],
            census_hash: base.owner_fingerprint(),
            state_hash: [8; 32],
        }
        .encode()
        .unwrap();
        let original = sealed_image(&raw, ALL);
        let prior = base.append_entry(&original).unwrap();
        let directories: Vec<_> = (0..4).map(|_| tempfile::tempdir().unwrap()).collect();
        let handles: Vec<_> = directories
            .iter()
            .map(|d| File::open(d.path()).unwrap())
            .collect();
        let path = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(path.path(), [3; 128]).unwrap();
        let file = File::open(path.path()).unwrap();
        let installed = prior
            .append_installed_origin([&handles[0], &handles[1], &handles[2], &handles[3]], &file)
            .unwrap();
        Self {
            _images: images,
            _directories: directories,
            _handles: handles,
            _installed_path: path,
            _installed_file: file,
            original: Some(original),
            raw,
            base,
            prior,
            installed,
        }
    }
}
#[test]
fn bootstrap_installed_material_census_exact_prefix_and_context_fingerprint() {
    let mut f = Fixture::new();
    let raw = f.installed.entry_assertions(&f.raw, &f.prior).unwrap();
    let parsed = AssertedInstalledEntry::decode(&raw).unwrap();
    for at in [0, 12, 220] {
        let mut bad = f.raw;
        bad[at] ^= 1;
        assert!(f.installed.entry_assertions(&bad, &f.prior).is_err());
    }
    let size = f.installed.installed.stat[6];
    for bad_size in [0, size + 1] {
        f.installed.installed.stat[6] = bad_size;
        assert!(f.installed.entry_assertions(&f.raw, &f.prior).is_err());
    }
    f.installed.installed.stat[6] = size;
    assert_eq!(parsed.original.encode().unwrap(), f.raw);
    assert_eq!(parsed.installed_identity[2], 128);
    for at in 0..15 {
        f.installed.roles[at] += 1;
        assert!(f.installed.entry_assertions(&f.raw, &f.prior).is_err());
        f.installed.roles[at] -= 1;
    }
    for at in 0..27 {
        f.installed.fds[at] += 1;
        assert!(f.installed.entry_assertions(&f.raw, &f.prior).is_err());
        f.installed.fds[at] -= 1;
    }
    for at in 0..4 {
        f.installed.ancestry[at].stat[1] += 1;
        assert_ne!(f.installed.entry_assertions(&f.raw, &f.prior).unwrap(), raw);
        f.installed.ancestry[at].stat[1] -= 1;
    }
    f.prior.entry.stat[9] += 1;
    assert_ne!(f.installed.entry_assertions(&f.raw, &f.prior).unwrap(), raw);
}
#[test]
fn bootstrap_installed_material_census_one_record_rejects_size_alias_flags_offset() {
    let mut f = Fixture::new();
    let record = sealed_image(
        &f.installed.entry_assertions(&f.raw, &f.prior).unwrap(),
        ALL,
    );
    let observed = f.installed.append_installed_entry(&record).unwrap();
    assert_eq!(observed.fds[..32], f.installed.fds);
    assert_eq!(observed.roles[..16], f.installed.roles);
    assert_eq!(observed.roles[16], record.as_raw_fd());
    assert!(f
        .installed
        .append_installed_entry(f.original.as_ref().unwrap())
        .is_err());
    let alias = record.try_clone().unwrap();
    let old = f.installed.fds[0];
    f.installed.fds[0] = alias.as_raw_fd();
    assert!(f.installed.append_installed_entry(&record).is_err());
    f.installed.fds[0] = old;
    f.installed.fds[0] = record.as_raw_fd();
    assert!(f.installed.append_installed_entry(&record).is_err());
    f.installed.fds[0] = old;
    assert_eq!(
        unsafe { libc::lseek(record.as_raw_fd(), 1, libc::SEEK_SET) },
        1
    );
    assert!(f.installed.append_installed_entry(&record).is_err());
    for seals in [
        0,
        ALL & !libc::F_SEAL_WRITE,
        ALL | libc::F_SEAL_FUTURE_WRITE,
    ] {
        assert!(f
            .installed
            .append_installed_entry(&sealed_image(&[7; 1280], seals))
            .is_err());
    }
}
