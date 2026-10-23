use super::*;
use std::os::fd::AsRawFd;
#[path = "../tests/support/bootstrap_census_material.rs"]
mod material;
fn observe(f: &material::Fixture) -> io::Result<ObservedRoles> {
    let entry = super::super::read(f.fds()[14])?;
    // The fixture creator is this test process. Child identity is modeled here;
    // the process control separately uses the actual child's PID and slots.
    capture(f.fds(), &entry, 1)
}
#[test]
fn bootstrap_roles_all_fifteen_actual_resources_match_only_asserted_context() {
    let f = material::Fixture::new();
    let first = observe(&f).unwrap();
    assert!(observe(&f).unwrap() == first);
    assert_eq!(first.stamps.len(), 15);
    assert_eq!(f.retained.len(), 3);
    let entry = super::super::read(f.fds()[14]).unwrap();
    assert!(capture(f.fds(), &entry, std::process::id()).is_err());
}
#[test]
fn bootstrap_roles_collision_swap_flags_and_each_image_identity_digest_refuse() {
    let mut f = material::Fixture::new();
    let entry = super::super::read(f.fds()[14]).unwrap();
    for at in 0..14 {
        let mut fds = f.fds();
        fds[at] = fds[14];
        assert!(capture(fds, &entry, 1).is_err());
    }
    let mut swapped = f.fds();
    swapped.swap(1, 2);
    assert!(capture(swapped, &entry, 1).is_err());
    for at in 0..15 {
        let fd = f.fds()[at];
        assert_eq!(
            unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
            0
        );
        assert!(observe(&f).is_err());
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0);
    }
    let original = f.entry.clone();
    for at in 0..6 {
        f.entry = original.clone();
        f.entry.identities[at][1] += 1;
        f.replace_entry();
        assert!(observe(&f).is_err());
    }
    for at in 0..5 {
        f.entry = original.clone();
        f.entry.image_hashes[at][0] ^= 1;
        f.replace_entry();
        assert!(observe(&f).is_err());
    }
}
#[test]
fn bootstrap_roles_wrong_pidfd_peer_pending_input_and_wrong_alarm_refuse() {
    let mut f = material::Fixture::new();
    let original = f.entry.clone();
    f.entry.owner[4] += 1;
    f.replace_entry();
    assert!(observe(&f).is_err());
    f.entry = original.clone();
    f.replace_entry();
    assert_eq!(
        unsafe {
            libc::send(
                f.retained[0].as_raw_fd(),
                b"x".as_ptr().cast(),
                1,
                libc::MSG_NOSIGNAL,
            )
        },
        1
    );
    assert!(observe(&f).is_err());
    let mut f = material::Fixture::new();
    f.entry.guardians[0][0] += 100000;
    f.replace_entry();
    assert!(observe(&f).is_err());
    let f = material::Fixture::new();
    material::arm(&f.files[9], material::now() + 1_000_000_000);
    assert!(observe(&f).is_err());
}
#[test]
fn bootstrap_roles_dead_guardian_and_same_bytes_replacement_cannot_revalidate() {
    let mut f = material::Fixture::new();
    let first = observe(&f).unwrap();
    let raw = f.entry.encode().unwrap();
    f.files[14] = material::sealed(&raw);
    assert!(observe(&f).unwrap() != first);
    f.children[0].kill().unwrap();
    f.children[0].wait().unwrap();
    assert!(observe(&f).is_err());
}
