use super::super::super::tests::Fixture;
use super::*;
use crate::diagnostic_builder::bootstrap_build::tests::sealed_image;
#[test]
fn bootstrap_installed_census_retains_all_old_slots_and_rejects_fd_and_inode_aliases() {
    let f = Fixture::new();
    let images = collect(
        &f.prior(),
        f.original(),
        f.images.each_ref(),
        f.identities(),
    )
    .unwrap();
    let entry = sealed_image(
        &[9; 1024],
        libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE,
    );
    let prior = images.append_entry(&entry).unwrap();
    let dirs: Vec<_> = (0..4).map(|_| tempfile::tempdir().unwrap()).collect();
    let files: Vec<_> = dirs.iter().map(|d| File::open(d.path()).unwrap()).collect();
    let directories = [&files[0], &files[1], &files[2], &files[3]];
    let installed = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(installed.path(), [1; 120]).unwrap();
    let image = File::open(installed.path()).unwrap();
    let observed = prior.append_installed_origin(directories, &image).unwrap();
    assert_eq!(observed.fds[..27], prior.fds);
    assert_eq!(observed.roles[..15], prior.roles);
    assert_eq!(observed.roles[15], image.as_raw_fd());
    assert_eq!(observed.fds[31], image.as_raw_fd());
    assert!(prior
        .append_installed_origin([&files[0]; 4], &image)
        .is_err());
    let alias = f.images[2].try_clone().unwrap();
    assert!(prior.append_installed_origin(directories, &alias).is_err());
    let mut changed = prior;
    changed.fds[0] = image.as_raw_fd();
    assert!(changed
        .append_installed_origin(directories, &image)
        .is_err());
}
