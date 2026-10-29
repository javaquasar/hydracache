//! Same decimal formatter, bounded stack ownership; no cached filesystem view.
use super::*;

pub(crate) fn reference(fd: i32) -> String {
    format!("/proc/self/fd/{fd}")
}
pub(crate) fn proposed(fd: i32) -> std::io::Cursor<[u8; 25]> {
    let mut path = std::io::Cursor::new([0_u8; 25]);
    write!(&mut path, "/proc/self/fd/{fd}").expect("i32 fd path fits 25 bytes");
    path
}
pub(crate) fn borrowed(path: &std::io::Cursor<[u8; 25]>) -> &str {
    let length = path.position() as usize;
    std::str::from_utf8(&path.get_ref()[..length]).expect("fd path is ASCII")
}

#[test]
fn authority_fd_path_decimal_boundaries_seeded_bytes_and_capacity() {
    let mut descriptors = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let mut boundary = 10_i32;
    while boundary <= 1_000_000_000 {
        for value in [boundary - 1, boundary, boundary + 1] {
            descriptors.extend([value, -value]);
        }
        if boundary == 1_000_000_000 {
            break;
        }
        boundary *= 10;
    }
    let mut seed = 0x074f_d025_u64;
    for _ in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        descriptors.push(seed as i32);
    }
    for fd in descriptors {
        let expected = reference(fd);
        let path = proposed(fd);
        assert_eq!(borrowed(&path).as_bytes(), expected.as_bytes());
        assert!(path.position() <= 25);
        assert!(borrowed(&path).is_ascii());
        assert_eq!(expected.len(), 14 + fd.to_string().len());
    }
    assert_eq!(proposed(i32::MIN).position(), 25);
    eprintln!("fd-path seed=0x074fd025 seeded=4096 decimal-boundaries=i32");
}

#[test]
fn authority_fd_path_retains_descriptor_and_observes_fresh_inventory_mutations() {
    assert_ne!(unsafe { libc::geteuid() }, 0);
    let root = tempfile::Builder::new()
        .prefix("hydracache-authority-")
        .tempdir_in("/tmp")
        .unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let directory = Directory::open(root.path(), unsafe { libc::geteuid() }, unsafe {
        libc::getegid()
    })
    .unwrap();
    let fd = directory.leaf.as_raw_fd();
    assert_eq!(directory.inventory().unwrap(), 0);
    for suffix in ["commit", "json"] {
        fs::write(root.path().join(name(1, suffix)), []).unwrap();
    }
    for path in [reference(fd), borrowed(&proposed(fd)).to_owned()] {
        assert_eq!(fs::read_dir(path).unwrap().count(), 2);
    }
    assert_eq!(directory.inventory().unwrap(), 1);
    fs::remove_file(root.path().join(name(1, "commit"))).unwrap();
    assert!(matches!(
        directory.inventory(),
        Err(AuthorityError::Invalid)
    ));
    fs::write(root.path().join(name(1, "commit")), []).unwrap();
    assert_eq!(directory.inventory().unwrap(), 1);
    assert_eq!(directory.leaf.as_raw_fd(), fd);
    // Inventory alone does not latch. Full-inspect sticky refusal is a separate control.
}
