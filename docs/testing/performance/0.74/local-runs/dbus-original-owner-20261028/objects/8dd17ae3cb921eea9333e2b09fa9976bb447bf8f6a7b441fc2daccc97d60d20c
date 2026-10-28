//! Pure actual-helper requests and complete-body differential; no live calls.
use super::*;
use crate::diagnostic_worker_policy::trusted_input::{
    component_stack_controls as candidate, TrustInputError,
};
use std::cell::RefCell;
use std::ffi::{CStr, CString};

fn frozen_child(
    parent: i32,
    name: &str,
    directory: bool,
    open: impl FnOnce(i32, &CStr, i32) -> i32,
    last_error: impl FnOnce() -> std::io::Error,
) -> Result<i32, TrustInputError> {
    let name = CString::new(name).map_err(|_| TrustInputError::Invalid)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | libc::O_NONBLOCK
        | if directory { libc::O_DIRECTORY } else { 0 };
    let fd = open(parent, &name, flags);
    if fd < 0 {
        return Err(last_error().into());
    }
    Ok(fd)
}
fn valid_cases() -> Vec<(String, String, bool)> {
    let mut cases = [
        "etc",
        "hydracache-perf",
        "worker-trust-074.json",
        "controller-trust-074.json",
        "held-runtime-trust-074.json",
        "oidc-trust-074.json",
    ]
    .into_iter()
    .map(|name| (name.into(), name.into(), true))
    .collect::<Vec<_>>();
    for length in [0, 1, 30, 31, 32, 63, 64, 255, 256, 4096] {
        cases.push((format!("length-{length}"), "a".repeat(length), false));
    }
    for (label, name) in [
        ("unicode-cyrillic", "ключ"),
        ("unicode-path", "é/名"),
        ("parent-component", "../x"),
        ("dot", "."),
        ("absolute-name", "/x"),
    ] {
        cases.push((label.into(), name.into(), false));
    }
    cases
}

#[test]
fn trust_component_stack_valid_requests() {
    for (label, name, production) in valid_cases() {
        let (_, counts) = capture(|| {
            for _ in 0..256 {
                candidate::with_name(std::hint::black_box(name.as_str()), |value| {
                    let bytes = std::hint::black_box(value.to_bytes_with_nul());
                    assert_eq!(&bytes[..name.len()], name.as_bytes());
                    assert_eq!(bytes.len(), name.len() + 1);
                    assert_eq!(bytes[name.len()], 0);
                })
                .unwrap();
            }
        });
        let allocations = if name.len() < 32 { 0 } else { 256 };
        assert_eq!(counts.allocations, allocations);
        assert_eq!(counts.reallocations, 0);
        assert_eq!(
            counts.requested_bytes,
            allocations * (name.len() as u64 + 1)
        );
        println!(
            "HC074-TRUST-COMPONENT-CANDIDATE {}",
            serde_json::json!({
                "label":label,"length":name.len(),"production":production,"iterations":256,
                "counts":counts,"rust_requests_only":true,"timing_claim":false,"profile":false,"server":false,"product":false
            })
        );
    }
}

#[test]
fn trust_component_stack_nul_requests() {
    for (label, name) in [
        ("leading-nul", "\0abc"),
        ("interior-nul", "ab\0cd"),
        ("trailing-nul", "abc\0"),
        ("unicode-nul", "é\0名"),
    ] {
        let position = name.as_bytes().iter().position(|b| *b == 0).unwrap();
        let (_, counts) = capture(|| {
            for _ in 0..256 {
                assert!(candidate::with_name(std::hint::black_box(name), |_| panic!(
                    "callback after NUL"
                ))
                .is_err());
            }
        });
        assert_eq!(counts, Counts::default());
        println!(
            "HC074-TRUST-COMPONENT-NUL-CANDIDATE {}",
            serde_json::json!({
                "label":label,"length":name.len(),"position":position,"iterations":256,
                "counts":counts,"rust_requests_only":true,"timing_claim":false,"profile":false,"server":false,"product":false
            })
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Open {
        parent: i32,
        bytes: Vec<u8>,
        flags: i32,
    },
    LastError,
}
fn run(
    new: bool,
    name: &str,
    parent: i32,
    directory: bool,
    fd: i32,
    errno: i32,
) -> (String, Vec<Event>) {
    let events = RefCell::new(Vec::new());
    let open = |parent, value: &CStr, flags| {
        events.borrow_mut().push(Event::Open {
            parent,
            bytes: value.to_bytes_with_nul().to_vec(),
            flags,
        });
        fd
    };
    let last_error = || {
        events.borrow_mut().push(Event::LastError);
        std::io::Error::from_raw_os_error(errno)
    };
    let result = if new {
        candidate::child_model(parent, name, directory, open, last_error)
    } else {
        frozen_child(parent, name, directory, open, last_error)
    };
    let class = match result {
        Ok(fd) => format!("fd:{fd}"),
        Err(TrustInputError::Invalid) => "invalid".into(),
        Err(TrustInputError::Io(error)) => format!("io:{:?}", error.raw_os_error()),
        Err(error) => panic!("unexpected pure error: {error}"),
    };
    (class, events.into_inner())
}

#[test]
fn trust_component_stack_seeded_complete_child_differential() {
    let mut seed = 0x074c_57a1_u64;
    for sample in 0..512 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let length = [0, 1, 30, 31, 32, 63, 64, 255, 256][sample % 9];
        let mut name = "a".repeat(length);
        if seed & 1 != 0 {
            name.push('名');
        }
        if seed & 2 != 0 {
            name.insert((seed as usize) % (length + 1), '\0');
        }
        let parent = [-1, 0, 17, i32::MAX][(seed >> 8) as usize % 4];
        let fd = [-2, -1, 0, 37][(seed >> 16) as usize % 4];
        let errno = [libc::EINTR, libc::EACCES, libc::ENOENT, libc::EIO][(seed >> 24) as usize % 4];
        assert_eq!(
            run(false, &name, parent, seed & 4 != 0, fd, errno),
            run(true, &name, parent, seed & 4 != 0, fd, errno),
            "seed=0x074c57a1 sample={sample}"
        );
    }
    println!("HC074-TRUST-COMPONENT-DIFFERENTIAL seed=0x074c57a1 samples=512 model_only=true");
}

#[test]
fn trust_component_stack_explicit_first_errors_and_long_nul_fallback() {
    for (_, name, _) in valid_cases() {
        for directory in [false, true] {
            for (fd, errno) in [
                (0, libc::EIO),
                (37, libc::EIO),
                (-1, libc::EINTR),
                (-2, libc::EACCES),
            ] {
                assert_eq!(
                    run(false, &name, -1, directory, fd, errno),
                    run(true, &name, -1, directory, fd, errno)
                );
            }
        }
    }
    for length in [1, 31, 32, 33, 63, 64, 256, 4096] {
        for position in [0, length / 2, length - 1] {
            let mut name = "a".repeat(length);
            name.replace_range(position..position + 1, "\0");
            assert_eq!(
                run(false, &name, -1, true, -1, libc::EBADF),
                run(true, &name, -1, true, -1, libc::EBADF)
            );
            assert!(matches!(
                candidate::child_model(
                    -1,
                    &name,
                    false,
                    |_, _, _| panic!("open after NUL"),
                    || panic!("last error after NUL")
                ),
                Err(TrustInputError::Invalid)
            ));
            let (_, counts) = capture(|| {
                assert!(
                    candidate::with_name(std::hint::black_box(&name), |_| panic!(
                        "callback after NUL"
                    ))
                    .is_err()
                );
            });
            let alloc = if length < 32 { 0 } else { 1 };
            assert_eq!(counts.allocations, alloc);
            assert_eq!(counts.reallocations, 0);
            assert_eq!(counts.requested_bytes, alloc * (length as u64 + 1));
        }
    }
}

#[test]
fn trust_component_stack_owned_values_pointer_and_boundary_bytes() {
    for length in [0, 1, 27, 30, 31, 32, 33, 63, 64, 4096] {
        let original = "a".repeat(length);
        let mut expected = original.as_bytes().to_vec();
        expected.push(0);
        assert_eq!(candidate::owned_after_input_mutation(&original), expected);
        candidate::with_name(&original, |outer| {
            candidate::with_name("different", |inner| {
                assert_ne!(outer.as_ptr(), inner.as_ptr());
                assert_eq!(outer.to_bytes_with_nul(), expected);
                assert_eq!(inner.to_bytes(), b"different");
            })
            .unwrap();
            assert_eq!(outer.to_bytes_with_nul(), expected);
        })
        .unwrap();
    }
}
