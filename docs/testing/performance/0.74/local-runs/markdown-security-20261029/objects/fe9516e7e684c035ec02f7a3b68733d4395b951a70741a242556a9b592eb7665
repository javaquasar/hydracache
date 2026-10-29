//! Fixed SHA-256 oracle and refusal coverage for fresh complete role reads.
use super::*;
use sha2::{Digest, Sha256};
use std::os::fd::AsRawFd;

fn material(raw: &[u8], seals: i32) -> std::fs::File {
    crate::bootstrap_child_material::sealed(raw, seals)
}

#[test]
fn fresh_full_role_hash_matches_fixed_sha256_and_old_oracle_at_chunk_boundaries() {
    let abc = material(b"abc", SEALS);
    let stamp = snapshot(abc.as_raw_fd()).unwrap();
    let known = [
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22,
        0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00,
        0x15, 0xad,
    ];
    regular(&stamp, id(&stamp), known, 3, false).unwrap();
    for length in [1usize, 63, 64, 65, 65_535, 65_536, 65_537, 131_091] {
        let raw: Vec<_> = (0..length)
            .map(|i| (i.wrapping_mul(29).wrapping_add(17) & 255) as u8)
            .collect();
        let file = material(&raw, SEALS);
        let stamp = snapshot(file.as_raw_fd()).unwrap();
        let old_oracle: [u8; 32] = Sha256::digest(&raw).into();
        for _ in 0..2 {
            regular(&stamp, id(&stamp), old_oracle, length as u64, false).unwrap();
            assert!(snapshot(file.as_raw_fd()).unwrap() == stamp);
        }
    }
}

#[test]
fn fresh_full_role_hash_keeps_digest_identity_limit_offset_flags_and_seal_refusals() {
    let raw = vec![0x74; 65_537];
    let file = material(&raw, SEALS);
    let fd = file.as_raw_fd();
    let stamp = snapshot(fd).unwrap();
    let digest: [u8; 32] = Sha256::digest(&raw).into();
    let mut wrong = digest;
    wrong[0] ^= 1;
    assert!(regular(&stamp, id(&stamp), wrong, raw.len() as u64, false).is_err());
    assert!(regular(&stamp, id(&stamp), digest, raw.len() as u64 - 1, false).is_err());
    let other = material(&raw, SEALS);
    let replacement = snapshot(other.as_raw_fd()).unwrap();
    assert!(regular(&replacement, id(&stamp), digest, raw.len() as u64, false).is_err());
    assert_eq!(unsafe { libc::lseek(fd, 1, libc::SEEK_SET) }, 1);
    assert!(regular(&stamp, id(&stamp), digest, raw.len() as u64, false).is_err());
    assert_eq!(
        unsafe { libc::lseek(fd, 0, libc::SEEK_CUR) },
        1,
        "no offset repair"
    );
    assert_eq!(unsafe { libc::lseek(fd, 0, libc::SEEK_SET) }, 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) },
        0
    );
    assert!(regular(&stamp, id(&stamp), digest, raw.len() as u64, false).is_err());
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_GETFD) },
        libc::FD_CLOEXEC,
        "no flag repair"
    );
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFD, 0) }, 0);
    let unsealed = material(&raw, 0);
    let unsealed = snapshot(unsealed.as_raw_fd()).unwrap();
    assert!(regular(&unsealed, id(&unsealed), digest, raw.len() as u64, false).is_err());
    let empty = material(&[], SEALS);
    let empty = snapshot(empty.as_raw_fd()).unwrap();
    assert!(regular(&empty, id(&empty), Sha256::digest([]).into(), 1, false).is_err());
    assert!(snapshot(fd).unwrap() == stamp);
}
