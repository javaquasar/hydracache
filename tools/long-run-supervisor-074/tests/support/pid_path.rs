//! One uncached signed decimal key; all actual opens remain fresh.
use super::*;

// BEGIN STACK PID NAME
pub(super) struct PidName {
    bytes: [u8; 12],
    start: usize,
}
impl PidName {
    pub(super) fn new(pid: i32) -> Self {
        let mut bytes = [0; 12];
        let mut start = 11;
        let mut remaining = pid.unsigned_abs();
        loop {
            start -= 1;
            bytes[start] = b'0' + (remaining % 10) as u8;
            remaining /= 10;
            if remaining == 0 {
                break;
            }
        }
        if pid < 0 {
            start -= 1;
            bytes[start] = b'-';
        }
        Self { bytes, start }
    }
    pub(super) fn as_c_str(&self) -> &std::ffi::CStr {
        std::ffi::CStr::from_bytes_with_nul(&self.bytes[self.start..])
            .expect("signed i32 digits and one terminal NUL fit twelve bytes")
    }
}
// END STACK PID NAME

#[test]
fn pid_path_signed_edges_and_seeded_values_match_complete_decimal() {
    let check = |pid: i32| {
        let name = PidName::new(pid);
        assert_eq!(name.as_c_str().to_bytes(), pid.to_string().as_bytes());
        assert_eq!(name.as_c_str().to_bytes_with_nul().last(), Some(&0));
        assert!(!name.as_c_str().to_bytes().contains(&0));
    };
    for pid in [
        i32::MIN,
        i32::MIN + 1,
        -1_000_000_000,
        -100,
        -10,
        -1,
        0,
        1,
        9,
        10,
        99,
        100,
        999_999_999,
        1_000_000_000,
        i32::MAX,
    ] {
        check(pid);
    }
    let mut seed = 0x0740_1d32_u64;
    for _ in 0..16384 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        check(seed as i32);
    }
    println!("HC074-PID-FORMAT seed=0x07401d32 mutations=16384 complete_signed_i32=true");
}

fn outcome<T>(value: Result<T, KernelError>) -> String {
    match value {
        Ok(_) => "ok".into(),
        Err(KernelError::Io(e)) => format!("io:{:?}:{:?}", e.kind(), e.raw_os_error()),
        Err(e) => e.to_string(),
    }
}

#[test]
fn pid_path_open_differential_preserves_identity_flags_and_refusals() {
    let root = Origin::open().unwrap();
    for pid in [1, std::process::id() as i32] {
        let old = child(&root.root, &pid.to_string(), true).unwrap();
        let new = fixed_child(&root.root, PidName::new(pid).as_c_str(), true).unwrap();
        let identity = |f: &File| {
            let m = f.metadata().unwrap();
            (m.dev(), m.ino(), m.mode())
        };
        assert_eq!(identity(&old), identity(&new));
        for op in [libc::F_GETFL, libc::F_GETFD] {
            assert_eq!(unsafe { libc::fcntl(old.as_raw_fd(), op) }, unsafe {
                libc::fcntl(new.as_raw_fd(), op)
            });
        }
    }
    let scratch = tempfile::tempdir().unwrap();
    let directory = File::open(scratch.path()).unwrap();
    let file = tempfile::tempfile().unwrap();
    for parent in [&root.root, &directory, &file] {
        for pid in [i32::MIN, -1, 0, i32::MAX] {
            assert_eq!(
                outcome(child(parent, &pid.to_string(), true)),
                outcome(fixed_child(parent, PidName::new(pid).as_c_str(), true))
            );
        }
    }
    std::os::unix::fs::symlink("/proc/1", scratch.path().join("1")).unwrap();
    assert_eq!(
        outcome(child(&directory, "1", true)),
        outcome(fixed_child(&directory, PidName::new(1).as_c_str(), true))
    );
    std::fs::remove_file(scratch.path().join("1")).unwrap();
    std::fs::create_dir(scratch.path().join("1")).unwrap();
    assert_eq!(
        outcome(child(&directory, "1", true)),
        outcome(fixed_child(&directory, PidName::new(1).as_c_str(), true))
    );
    root.revalidate().unwrap();
}

#[test]
fn pid_path_runtime_sites_use_fresh_numeric_key_without_string_allocation() {
    let raw = include_str!("../../src/diagnostic_kernel_admission.rs");
    for exact in [
        "let directory = origin.child(credentials.pid, true)?;",
        "let directory = origin.child(self.credentials.pid, true)?;",
    ] {
        assert_eq!(raw.matches(exact).count(), 1, "unimplemented PID candidate");
    }
    let raw = include_str!("../../src/diagnostic_kernel_origin.rs");
    assert!(raw.contains("fixed_child(&self.root, PidName::new(pid).as_c_str(), directory)"));
}
