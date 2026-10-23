//! Synthetic sealed assertions only; no original owner, credentials or permit.
use hydracache_long_run_supervisor_074::diagnostic_builder::bootstrap_build::entry_format::AssertedEntry;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::{
    fd::{AsRawFd, FromRawFd},
    unix::fs::OpenOptionsExt,
};
pub const SEALS: i32 =
    libc::F_SEAL_SEAL | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_WRITE;
pub fn raw() -> [u8; 1024] {
    AssertedEntry {
        owner: [74, 75, 1000, 1000, 1000, 1000],
        origins: [1, 2, 3, 4],
        bounds: [300, 60, 10, 10],
        attempt: [1; 32],
        original_context: [2; 32],
        root: [1, 2, 10],
        serial: 1,
        names: [[3; 32]; 2],
        documents: [[4; 32]; 7],
        identities: std::array::from_fn(|i| [1, i as u64 + 2, if i == 5 { 256 } else { 64 }]),
        streams: [[1, 2]; 4],
        guardians: [[76, 1, 3], [77, 1, 4]],
        image_hashes: [[5; 32]; 5],
        legacy_hash: [6; 32],
        census_hash: [7; 32],
        state_hash: [8; 32],
    }
    .encode()
    .unwrap()
}
pub fn sealed(raw: &[u8], seals: i32) -> File {
    let fd = unsafe {
        libc::memfd_create(
            c"bootstrap-child-assertion".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    assert!(fd >= 0);
    let mut writer = unsafe { File::from_raw_fd(fd) };
    writer.write_all(raw).unwrap();
    assert_eq!(unsafe { libc::fchmod(fd, 0o400) }, 0);
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, seals) }, 0);
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(format!("/proc/thread-self/fd/{fd}"))
        .unwrap();
    drop(writer);
    assert_eq!(
        unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, 0) },
        0
    );
    file
}
